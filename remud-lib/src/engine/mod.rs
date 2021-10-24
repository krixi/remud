mod client;
pub mod db;
pub mod dialog;
pub mod fsm;
pub mod persist;

use std::{borrow::Cow, collections::VecDeque, time::Instant};

use futures::future::join_all;
use itertools::Itertools;
use thiserror::Error;
use tokio::{
    sync::mpsc,
    time::{interval, Duration, Interval},
};

use crate::{
    ecs::{CorePlugin, Ecs},
    engine::{
        client::{Client, ClientEvent, Clients, SendPrompt},
        db::{Db, GameDb},
        persist::PersistPlugin,
    },
    macros::regex,
    metrics::stats_time,
    web::{
        scripts::{JsonScript, JsonScriptInfo, JsonScriptName, JsonScriptResponse},
        ScriptsRequest, ScriptsResponse, WebMessage,
    },
    world::{
        action::ActionsPlugin, fsm::FsmPlugin, scripting::ScriptPlugin, types::TypesPlugin,
        GameWorld,
    },
    ClientId,
};

pub(crate) enum EngineMessage {
    Disconnect(ClientId),
    Restart,
    Shutdown,
}

#[derive(Debug)]
pub enum ClientMessage {
    Connect(
        ClientId,
        mpsc::Sender<ClientMessage>,
        mpsc::Sender<EngineResponse>,
    ),
    Disconnect(ClientId),
    Input(ClientId, String),
    PasswordHash(ClientId, Option<String>),
    PasswordVerification(ClientId, Option<bool>),
    Ready(ClientId),
}

impl ClientMessage {
    fn client_id(&self) -> ClientId {
        match self {
            ClientMessage::Connect(id, _, _) => *id,
            ClientMessage::Disconnect(id) => *id,
            ClientMessage::Input(id, _) => *id,
            ClientMessage::Ready(id) => *id,
            ClientMessage::PasswordHash(id, _) => *id,
            ClientMessage::PasswordVerification(id, _) => *id,
        }
    }
}

// creators of engine response produce one or more messages terminated
// by a prompt type

// consumers of engine response colorize the messages and prompt (?)

#[derive(Debug)]
pub enum EngineResponse {
    Output(VecDeque<Output>),
}

impl EngineResponse {
    pub fn from_messages_prompt<'a, M: Into<Cow<'a, str>>>(
        messages: impl IntoIterator<Item = M>,
        sensitive: bool,
    ) -> Self {
        EngineResponse::Output(
            messages
                .into_iter()
                .map(Into::into)
                .map(|m| m.to_string())
                .map(Output::Message)
                .chain(std::iter::once(Output::Prompt {
                    format: "> ".to_string(),
                    sensitive,
                }))
                .collect(),
        )
    }

    pub fn from_messages<'a, M: Into<Cow<'a, str>>>(messages: impl IntoIterator<Item = M>) -> Self {
        EngineResponse::Output(
            messages
                .into_iter()
                .map(Into::into)
                .map(|m| m.to_string())
                .map(Output::Message)
                .collect(),
        )
    }
}

impl From<Output> for EngineResponse {
    fn from(value: Output) -> Self {
        let is_prompt = matches!(value, Output::Prompt { .. });
        let mut vec = VecDeque::new();
        vec.push_back(value);

        if !is_prompt {
            vec.push_back(Output::Prompt {
                format: "> ".to_string(),
                sensitive: false,
            });
        }

        EngineResponse::Output(vec)
    }
}

#[derive(Debug)]
pub enum Output {
    Message(String),
    Prompt { format: String, sensitive: bool },
}

pub struct Engine {
    client_rx: mpsc::Receiver<ClientMessage>,
    engine_tx: mpsc::Sender<EngineMessage>,
    web_rx: mpsc::Receiver<WebMessage>,
    clients: Clients,
    ticker: Interval,
    game_world: GameWorld,
    db: Db,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("error with database")]
    DbError(#[from] db::Error),
}

impl Engine {
    #[tracing::instrument(name = "creating engine", skip_all)]
    pub(crate) async fn new(
        db: Db,
        client_rx: mpsc::Receiver<ClientMessage>,
        engine_tx: mpsc::Sender<EngineMessage>,
        web_rx: mpsc::Receiver<WebMessage>,
    ) -> Result<Self, Error> {
        let mut ecs = Ecs::new();

        ecs.register(CorePlugin::default()).await;
        ecs.register(TypesPlugin::default()).await;
        ecs.register(ActionsPlugin::default()).await;
        ecs.register(ScriptPlugin::default()).await;
        ecs.register(FsmPlugin::default()).await;
        ecs.register(PersistPlugin::default()).await;

        {
            db.load_world(ecs.world_mut()).await?;
        }

        let mut game_world = GameWorld::new(ecs);

        // Run a tick to perform initialization of loaded objects.
        game_world.run_pre_init();

        Ok(Engine {
            client_rx,
            engine_tx,
            web_rx,
            clients: Clients::default(),
            ticker: interval(Duration::from_millis(15)),
            game_world,
            db,
        })
    }

    #[tracing::instrument(name = "run engine", skip_all)]
    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                _ = self.ticker.tick() => {
                    let start = Instant::now();
                    self.game_world.run_pre_init();
                    self.dispatch_engine_messages().await;

                    self.game_world.run_main();
                    self.dispatch_engine_messages().await;

                    self.game_world.run_post_timed();
                    self.dispatch_engine_messages().await;

                    self.persist_updates().await;

                    self.reload_prototypes().await;

                    stats_time("run-loop", start);

                    // Shutdown if requested
                    if self.game_world.should_shutdown(){
                        self.engine_tx.send(EngineMessage::Shutdown).await.ok();
                        break
                    }

                    // Restart if requested
                    if self.game_world.should_restart(){
                        self.engine_tx.send(EngineMessage::Restart).await.ok();
                        break
                    }
                }
                maybe_message = self.client_rx.recv() => {
                    if let Some(message) = maybe_message {
                        self.process(message).await;
                    }
                }
                maybe_message = self.web_rx.recv() => {
                    if let Some(message) = maybe_message {
                        self.process_web(message).await;
                    }
                }
            }
        }
    }

    #[tracing::instrument(name = "dispatch engine messages", skip_all)]
    pub async fn dispatch_engine_messages(&mut self) {
        // Dispatch all queued messages to players
        for (player, messages) in self.game_world.messages() {
            if let Some(client) = self.clients.by_player(player) {
                client.send(SendPrompt::Prompt, messages).await;
            } else {
                tracing::error!(
                    "attempting to send messages to player without client: {:?}",
                    player
                );
            }
        }
    }

    #[tracing::instrument(name = "persist updates", skip_all)]
    pub async fn persist_updates(&mut self) {
        // Dispatch all persistance requests
        let mut handles = Vec::new();
        for update in self.game_world.updates() {
            let pool = self.db.get_pool();
            handles.push(tokio::spawn(async move {
                match update.enact(&pool).await {
                    Ok(_) => (),
                    Err(e) => tracing::error!("failed to execute update: {}", e),
                };
            }));
        }
        join_all(handles).await;
    }

    #[tracing::instrument(name = "reload prototypes", skip_all)]
    pub async fn reload_prototypes(&mut self) {
        // Reload changed prototypes
        let prototype_reloads = self.game_world.prototype_reloads();
        for prototype in prototype_reloads {
            match self
                .db
                .reload_prototype(self.game_world.world_mut(), prototype)
                .await
            {
                Ok(_) => (),
                Err(e) => tracing::error!("failed to reload prototype {}: {}", prototype, e),
            };
        }
    }

    #[tracing::instrument(name = "process client message", skip_all, fields(client_id = message.client_id().id()))]
    async fn process(&mut self, message: ClientMessage) {
        match message {
            ClientMessage::Connect(client_id, client_tx, engine_tx) => {
                tracing::info!("{} connected", client_id);

                self.clients.add(client_id, client_tx, engine_tx);
            }
            ClientMessage::Disconnect(client_id) => {
                tracing::info!("{} disconnected", client_id);

                if let Some(player) = self.clients.get(client_id).and_then(Client::player) {
                    if let Err(e) = self.game_world.despawn_player(player) {
                        tracing::error!("failed to despawn player: {}", e);
                    }
                }

                if let Some(client) = self.clients.get_mut(client_id) {
                    client
                        .process(ClientEvent::Disconnect, &mut self.game_world, &self.db)
                        .await;
                }

                self.clients.remove(client_id);
                self.engine_tx
                    .send(EngineMessage::Disconnect(client_id))
                    .await
                    .ok();
            }
            ClientMessage::Input(client_id, input) => {
                if let Some(client) = self.clients.get_mut(client_id) {
                    if client.expecting_sensitive_input() {
                        tracing::debug!("{} -> ****** (redacted)", client_id);
                    } else {
                        tracing::debug!("{} -> {}", client_id, input.as_str());
                    }

                    client
                        .process(
                            ClientEvent::Input(input.as_str()),
                            &mut self.game_world,
                            &self.db,
                        )
                        .await;

                    if let Some(player) = client.player() {
                        self.clients.init_player(client_id, player);
                    }
                } else {
                    tracing::error!("received input from unknown client");
                }
            }
            ClientMessage::Ready(client_id) => {
                tracing::info!("{} ready", client_id);

                // this is where we invoke the character login fsm
                if let Some(client) = self.clients.get_mut(client_id) {
                    client
                        .process(ClientEvent::Ready, &mut self.game_world, &self.db)
                        .await;
                } else {
                    tracing::error!("received message from unknown client: {:?}", message);
                }
            }
            ClientMessage::PasswordHash(client_id, hash) => {
                if let Some(client) = self.clients.get_mut(client_id) {
                    client
                        .process(
                            ClientEvent::PasswordHash(hash),
                            &mut self.game_world,
                            &self.db,
                        )
                        .await;
                } else {
                    tracing::error!("received password hash from unknown client");
                }
            }
            ClientMessage::PasswordVerification(client_id, verified) => {
                if let Some(client) = self.clients.get_mut(client_id) {
                    client
                        .process(
                            ClientEvent::PasswordVerification(verified),
                            &mut self.game_world,
                            &self.db,
                        )
                        .await;

                    if let Some(player) = client.player() {
                        self.clients.init_player(client_id, player);
                    }
                } else {
                    tracing::error!("received password verification from unknown client");
                }
            }
        }
    }

    #[tracing::instrument(name = "process web message", skip_all)]
    async fn process_web(&mut self, message: WebMessage) {
        match message.request {
            ScriptsRequest::CreateScript(JsonScript {
                name,
                trigger,
                code,
            }) => match self.game_world.create_script(name, trigger, code) {
                Ok(e) => {
                    message
                        .response
                        .send(ScriptsResponse::ScriptCompiled(e.map(Into::into)))
                        .ok();
                }
                Err(e) => {
                    tracing::error!("failed CreateScript request: {}", e);
                    message.response.send(ScriptsResponse::Error(e)).ok();
                }
            },
            ScriptsRequest::ReadScript(JsonScriptName { name }) => {
                match self.game_world.read_script(name) {
                    Ok((script, err)) => {
                        message
                            .response
                            .send(ScriptsResponse::Script(JsonScriptResponse::new(
                                script, err,
                            )))
                            .ok();
                    }
                    Err(e) => {
                        tracing::error!("failed ReadScript request: {}", e);
                        message.response.send(ScriptsResponse::Error(e)).ok();
                    }
                }
            }
            ScriptsRequest::ReadAllScripts => {
                let scripts = self.game_world.read_all_scripts();
                message
                    .response
                    .send(ScriptsResponse::ScriptList(
                        scripts
                            .into_iter()
                            .map(|(script, error)| JsonScriptInfo::new(script, error))
                            .collect_vec(),
                    ))
                    .ok();
            }
            ScriptsRequest::UpdateScript(JsonScript {
                name,
                trigger,
                code,
            }) => match self.game_world.update_script(name, trigger, code) {
                Ok(e) => {
                    message
                        .response
                        .send(ScriptsResponse::ScriptCompiled(e.map(Into::into)))
                        .ok();
                }
                Err(e) => {
                    tracing::error!("failed UpdateScript request: {}", e);
                    message.response.send(ScriptsResponse::Error(e)).ok();
                }
            },
            ScriptsRequest::DeleteScript(JsonScriptName { name }) => {
                match self.game_world.delete_script(name) {
                    Ok(_) => {
                        message.response.send(ScriptsResponse::Done).ok();
                    }
                    Err(e) => {
                        tracing::error!("failed DeleteScript request: {}", e);
                        message.response.send(ScriptsResponse::Error(e)).ok();
                    }
                }
            }
        };
    }
}

pub fn name_valid(name: &str) -> bool {
    // Match names with between 2 and 32 characters which are alphanumeric and possibly include
    // the following characters: ' ', ''', '-', and '_'
    let re = regex!(r#"^[[:alnum:] '\-_]{2,32}$"#);
    re.is_match(name)
}
