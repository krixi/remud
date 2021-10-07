mod client;
pub mod db;
pub mod persist;

use std::{borrow::Cow, collections::VecDeque};

use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use futures::future::join_all;
use itertools::Itertools;
use rand::rngs::OsRng;
use thiserror::Error;
use tokio::{
    sync::mpsc,
    time::{interval, Duration, Interval},
};

use crate::{
    ecs::{CorePlugin, Ecs},
    engine::{
        client::{Client, Clients, SendPrompt, State},
        db::{verify_password, AuthDb, Db, GameDb, VerifyError},
        persist::PersistPlugin,
    },
    macros::regex,
    web::{
        scripts::{JsonScript, JsonScriptInfo, JsonScriptName, JsonScriptResponse},
        ScriptsRequest, ScriptsResponse, WebMessage,
    },
    world::{
        action::{commands::Commands, observe::Look, system::Login, Action, ActionsPlugin},
        fsm::FsmPlugin,
        scripting::ScriptPlugin,
        types::{
            player::{self, PlayerFlags},
            TypesPlugin,
        },
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
pub(crate) enum ClientMessage {
    Connect(ClientId, mpsc::Sender<EngineResponse>),
    Disconnect(ClientId),
    Ready(ClientId),
    Input(ClientId, String),
}

impl ClientMessage {
    fn client_id(&self) -> ClientId {
        match self {
            ClientMessage::Connect(id, _) => *id,
            ClientMessage::Disconnect(id) => *id,
            ClientMessage::Ready(id) => *id,
            ClientMessage::Input(id, _) => *id,
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
    pub fn with_message(message: Cow<str>) -> Self {
        Output::Message(message.to_owned().to_string()).into()
    }

    pub fn from_messages<'a>(
        messages: impl IntoIterator<Item = Cow<'a, str>>,
        sensitive: bool,
    ) -> Self {
        EngineResponse::Output(
            messages
                .into_iter()
                .map(|message| Output::Message(message.to_owned().to_string()))
                .chain(std::iter::once(Output::Prompt {
                    format: "> ".to_string(),
                    sensitive,
                }))
                .collect(),
        )
    }

    pub fn from_messages_noprompt<'a>(messages: impl IntoIterator<Item = Cow<'a, str>>) -> Self {
        EngineResponse::Output(
            messages
                .into_iter()
                .map(|message| Output::Message(message.to_owned().to_string()))
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
    commands: Commands,
    db: Db,
    tick: u64,
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
        game_world.run();

        let commands = Commands::default();

        Ok(Engine {
            client_rx,
            engine_tx,
            web_rx,
            clients: Clients::default(),
            ticker: interval(Duration::from_millis(15)),
            game_world,
            commands,
            db,
            tick: 0,
        })
    }

    #[tracing::instrument(name = "run engine", skip_all)]
    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                _ = self.ticker.tick() => {
                    self.game_world.run();

                    self.dispatch_engine_messages().await;

                    self.persist_updates().await;

                    self.reload_prototypes().await;

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

                    self.tick += 1;
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
                client
                    .send_batch(
                        self.tick,
                        SendPrompt::Prompt,
                        messages.into_iter().map(Into::into),
                    )
                    .await;
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
            ClientMessage::Connect(client_id, tx) => {
                tracing::info!("[{}] {} connected", self.tick, client_id);

                self.clients.add(client_id, tx);
            }
            ClientMessage::Disconnect(client_id) => {
                tracing::info!("[{}] {} disconnected", self.tick, client_id);

                if let Some(player) = self.clients.get(client_id).and_then(Client::get_player) {
                    if let Err(e) = self.game_world.despawn_player(player) {
                        tracing::error!("failed to despawn player: {}", e);
                    }
                }

                self.clients.remove(client_id);
                self.engine_tx
                    .send(EngineMessage::Disconnect(client_id))
                    .await
                    .ok();
            }
            ClientMessage::Ready(client_id) => {
                tracing::info!("[{}] {} ready", self.tick, client_id);

                if let Some(client) = self.clients.get(client_id) {
                    client
                        .send_batch(
                            self.tick,
                            SendPrompt::Prompt,
                            vec![
                                Cow::from(
                                    "|SteelBlue3|Connected to|-| \
                                     |white|ucs://uplink.six.city|-|\r\n",
                                ),
                                Cow::from("|SteelBlue3|Name?|-|"),
                            ],
                        )
                        .await;
                } else {
                    tracing::error!("received message from unknown client: {:?}", message);
                }
            }
            ClientMessage::Input(client_id, input) => {
                tracing::debug!("[{}] {} -> {}", self.tick, client_id, input.as_str());
                self.process_input(client_id, input).await;
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

    #[tracing::instrument(name = "process input", skip_all)]
    async fn process_input(&mut self, client_id: ClientId, input: String) {
        let mut spawned_player = None;

        if let Some(client) = self.clients.get_mut(client_id) {
            match client.get_state() {
                State::LoginName => {
                    let name = input.trim();
                    if name_valid(name) {
                        let has_user = match self.db.has_player(name).await {
                            Ok(has_user) => has_user,
                            Err(e) => {
                                tracing::error!("player presence check error: {}", e);
                                client
                                    .send_batch(
                                        self.tick,
                                        SendPrompt::Prompt,
                                        vec![
                                            Cow::from("|Red1|Error retrieving user.|-|"),
                                            Cow::from("|SteelBlue3|Name?|-|"),
                                        ],
                                    )
                                    .await;
                                return;
                            }
                        };

                        if has_user {
                            if self.game_world.player_online(name) {
                                client
                                    .send_batch(
                                        self.tick,
                                        SendPrompt::Prompt,
                                        vec![
                                            Cow::from("|Red1|User currently online.|-|"),
                                            Cow::from("|SteelBlue3|Name?|-|"),
                                        ],
                                    )
                                    .await;
                                return;
                            }
                            client
                                .send_batch(
                                    self.tick,
                                    SendPrompt::SensitivePrompt,
                                    vec![
                                        Cow::from("|SteelBlue3|User located.|-|"),
                                        Cow::from("|SteelBlue3|Password?|-|"),
                                    ],
                                )
                                .await;
                            client.set_state(State::LoginPassword {
                                name: name.to_string(),
                            });
                        } else {
                            client
                                .send_batch(
                                    self.tick,
                                    SendPrompt::SensitivePrompt,
                                    vec![
                                        Cow::from("|SteelBlue3|New user detected.|-|"),
                                        Cow::from("|SteelBlue3|Password?|-|"),
                                    ],
                                )
                                .await;
                            client.set_state(State::CreatePassword {
                                name: name.to_string(),
                            });
                        }
                    } else {
                        client
                            .send_batch(
                                self.tick,
                                SendPrompt::Prompt,
                                vec![
                                    Cow::from("|SteelBlue3|Invalid username.|-|"),
                                    Cow::from("|SteelBlue3|Name?|-|"),
                                ],
                            )
                            .await;
                    }
                }
                State::CreatePassword { name } => {
                    let name = name.clone();

                    if input.len() < 5 {
                        client
                            .send_batch(
                                self.tick,
                                SendPrompt::SensitivePrompt,
                                vec![
                                    Cow::from("|Red1|Weak password detected.|-|"),
                                    Cow::from("|SteelBlue3|Password?|-|"),
                                ],
                            )
                            .await;
                        return;
                    }

                    let hasher = Argon2::default();
                    let salt = SaltString::generate(&mut OsRng);
                    let hash = match hasher
                        .hash_password(input.as_bytes(), &salt)
                        .map(|hash| hash.to_string())
                    {
                        Ok(hash) => hash,
                        Err(e) => {
                            tracing::error!("create password hash error: {}", e);
                            client
                                .send_batch(
                                    self.tick,
                                    SendPrompt::SensitivePrompt,
                                    vec![
                                        Cow::from("|Red1|Error computing password hash.|-|"),
                                        Cow::from("|SteelBlue3|Password?|-|"),
                                    ],
                                )
                                .await;
                            return;
                        }
                    };

                    client
                        .send_batch(
                            self.tick,
                            SendPrompt::SensitivePrompt,
                            vec![
                                Cow::from("|SteelBlue3|Password accepted.|-|"),
                                Cow::from("|SteelBlue3|Verify?|-|"),
                            ],
                        )
                        .await;
                    client.set_state(State::VerifyPassword {
                        name: name.clone(),
                        hash,
                    });
                }
                State::VerifyPassword { name, hash } => {
                    let name = name.clone();

                    match verify_password(hash, input.as_str()) {
                        Ok(_) => (),
                        Err(e) => {
                            if let VerifyError::Unknown(e) = e {
                                tracing::error!("create verify password failure: {}", e);
                            }
                            client
                                .verification_failed_creation(self.tick, name.as_str())
                                .await;
                            return;
                        }
                    }

                    let spawn_room = self.game_world.spawn_room();
                    match self.db.create_player(name.as_str(), hash, spawn_room).await {
                        Ok(_) => (),
                        Err(e) => {
                            tracing::error!("user creation error: {}", e);
                            client
                                .verification_failed_creation(self.tick, name.as_str())
                                .await;
                            return;
                        }
                    };

                    let player = match self
                        .db
                        .load_player(self.game_world.world_mut(), name.as_str())
                        .await
                    {
                        Ok(player) => (player),
                        Err(e) => {
                            tracing::error!("failed to load player: {}", e);
                            client.spawn_failed(self.tick).await;
                            return;
                        }
                    };

                    client.verified(self.tick).await;
                    client.set_state(State::InGame { player });

                    self.game_world
                        .player_action(Action::from(Login { actor: player }));
                    self.game_world.player_action(Action::from(Look {
                        actor: player,
                        direction: None,
                    }));

                    spawned_player = Some(player);
                }
                State::LoginPassword { name } => {
                    let name = name.clone();

                    match self.db.verify_player(name.as_str(), input.as_str()).await {
                        Ok(verified) => {
                            if !verified {
                                client.verification_failed_login(self.tick).await;
                                return;
                            }
                        }
                        Err(e) => {
                            tracing::error!("get user hash error: {}", e);
                            client.verification_failed_login(self.tick).await;
                            return;
                        }
                    };

                    let player = match self
                        .db
                        .load_player(self.game_world.world_mut(), name.as_str())
                        .await
                    {
                        Ok(player) => (player),
                        Err(e) => {
                            tracing::error!("failed to load player: {}", e);
                            client.spawn_failed(self.tick).await;
                            return;
                        }
                    };

                    client.verified(self.tick).await;
                    client.set_state(State::InGame { player });

                    self.game_world
                        .player_action(Action::from(Login { actor: player }));
                    self.game_world.player_action(Action::from(Look {
                        actor: player,
                        direction: None,
                    }));

                    spawned_player = Some(player);
                }
                State::InGame { player } => {
                    tracing::debug!("{}> {:?} sent {:?}", self.tick, client_id, input);
                    let immortal = self
                        .game_world
                        .world()
                        .get::<PlayerFlags>(*player)
                        .unwrap()
                        .contains(player::Flags::IMMORTAL);

                    match self.commands.parse(*player, &input, !immortal) {
                        Ok(action) => self.game_world.player_action(action),
                        Err(message) => {
                            client.send_prompted(self.tick, message.into()).await;
                        }
                    }
                }
            }
        } else {
            tracing::error!("received message from unknown client ({:?})", client_id);
        }

        if let Some(player) = spawned_player {
            self.clients.insert(client_id, player);
        }
    }
}

fn name_valid(name: &str) -> bool {
    // Match names with between 2 and 32 characters which are alphanumeric and possibly include
    // the following characters: ' ', ''', '-', and '_'
    let re = regex!(r#"^[[:alnum:] '\-_]{2,32}$"#);
    re.is_match(name)
}
