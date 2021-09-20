mod client;
mod db;
mod macros;
pub mod persist;

use argon2::{
    password_hash::{self, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use futures::future::join_all;
use itertools::Itertools;
use lazy_static::lazy_static;
use rand::rngs::OsRng;
use regex::Regex;
use tokio::{
    sync::mpsc,
    time::{interval, Duration, Interval},
};

use crate::web::JsonScriptResponse;
use crate::{
    engine::{
        client::{Client, Clients, State},
        db::Db,
    },
    web::{JsonScript, JsonScriptInfo, JsonScriptName, WebMessage, WebRequest, WebResponse},
    world::{
        action::{observe::Look, parse, system::Login, Action},
        GameWorld,
    },
    ClientId,
};

pub enum ControlMessage {
    Shutdown,
}

#[derive(Debug)]
pub enum ClientMessage {
    Connect(ClientId, mpsc::Sender<EngineMessage>),
    Disconnect(ClientId),
    Ready(ClientId),
    Input(ClientId, String),
}

#[derive(Debug)]
pub enum EngineMessage {
    Output(String),
    EndOutput,
}

pub struct Engine {
    engine_rx: mpsc::Receiver<ClientMessage>,
    control_tx: mpsc::Sender<ControlMessage>,
    web_message_rx: mpsc::Receiver<WebMessage>,
    clients: Clients,
    ticker: Interval,
    game_world: GameWorld,
    db: Db,
    tick: u64,
}

impl Engine {
    pub async fn new(
        engine_rx: mpsc::Receiver<ClientMessage>,
        control_tx: mpsc::Sender<ControlMessage>,
        web_message_rx: mpsc::Receiver<WebMessage>,
    ) -> anyhow::Result<Self> {
        let db = Db::new("world.db").await?;
        let world = db.load_world().await?;

        let game_world = GameWorld::new(world);

        Ok(Engine {
            engine_rx,
            control_tx,
            web_message_rx,
            clients: Clients::default(),
            ticker: interval(Duration::from_millis(15)),
            game_world,
            db,
            tick: 0,
        })
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                _ = self.ticker.tick() => {
                    self.game_world.run().await;

                    for (player, mut messages) in self.game_world.messages().await {
                        if let Some(client) = self.clients.by_player(player) {
                            messages.push_back("> ".to_string());
                            client.send_batch(self.tick, messages).await;
                        } else {
                            tracing::error!("Attempting to send messages to player without client: {:?}", player);
                        }
                    }

                    let mut handles = Vec::new();
                    for update in self.game_world.updates().await {
                        let pool = self.db.get_pool().clone();
                        handles.push(tokio::spawn(async move {
                            match update.enact(&pool).await {
                                Ok(_) => (),
                                Err(e) => tracing::error!("Failed to execute update: {}", e),
                            };
                        }));
                    }
                    join_all(handles).await;

                    if self.game_world.should_shutdown().await {
                        break
                    }

                    self.tick += 1;
                }
                maybe_message = self.engine_rx.recv() => {
                    if let Some(message) = maybe_message {
                        self.process(message).await;
                    }
                }
                maybe_message = self.web_message_rx.recv() => {
                    if let Some(message) = maybe_message {
                        self.process_web(message).await;
                    }
                }
            }
        }

        self.control_tx.send(ControlMessage::Shutdown).await.ok();
    }

    async fn process(&mut self, message: ClientMessage) {
        match message {
            ClientMessage::Connect(client_id, tx) => {
                tracing::info!("{}> {:?} connected", self.tick, client_id);

                self.clients.add(client_id, tx);
            }
            ClientMessage::Disconnect(client_id) => {
                tracing::info!("{}> {:?} disconnected", self.tick, client_id);

                if let Some(player) = self.clients.get(client_id).and_then(Client::get_player) {
                    if let Err(e) = self.game_world.despawn_player(player).await {
                        tracing::error!("Failed to despawn player: {}", e);
                    }
                }

                self.clients.remove(client_id);
            }
            ClientMessage::Ready(client_id) => {
                tracing::info!("{}> {:?} ready", self.tick, client_id);

                let message = String::from(
                    "\r\n|#7822a3|Connected|-| to |2|ucs://uplink.six.city|-|\r\n\r\nName?\r\n> ",
                );
                if let Some(client) = self.clients.get(client_id) {
                    client.send(message.into()).await;
                } else {
                    tracing::error!("Received message from unknown client: {:?}", message);
                }
            }
            ClientMessage::Input(client_id, input) => {
                self.process_input(client_id, input).await;
            }
        }
    }

    async fn process_web(&mut self, message: WebMessage) {
        match message.request {
            WebRequest::CreateScript(JsonScript {
                name,
                trigger,
                code,
            }) => match self.game_world.create_script(name, trigger, code) {
                Ok(e) => {
                    message
                        .response
                        .send(WebResponse::ScriptCompiled(e.map(Into::into)))
                        .ok();
                }
                Err(e) => {
                    tracing::error!("Failed CreateScript request: {}", e);
                    message.response.send(WebResponse::Error(e)).ok();
                }
            },
            WebRequest::ReadScript(JsonScriptName { name }) => {
                match self.game_world.read_script(name) {
                    Ok((script, err)) => {
                        message
                            .response
                            .send(WebResponse::Script(JsonScriptResponse::new(script, err)))
                            .ok();
                    }
                    Err(e) => {
                        tracing::error!("Failed ReadScript request: {}", e);
                        message.response.send(WebResponse::Error(e)).ok();
                    }
                }
            }
            WebRequest::ReadAllScripts => {
                let scripts = self.game_world.read_all_scripts();
                message
                    .response
                    .send(WebResponse::ScriptList(
                        scripts
                            .into_iter()
                            .map(|(script, error)| JsonScriptInfo::new(script, error))
                            .collect_vec(),
                    ))
                    .ok();
            }
            WebRequest::UpdateScript(JsonScript {
                name,
                trigger,
                code,
            }) => match self.game_world.update_script(name, trigger, code) {
                Ok(e) => {
                    message
                        .response
                        .send(WebResponse::ScriptCompiled(e.map(Into::into)))
                        .ok();
                }
                Err(e) => {
                    tracing::error!("Failed UpdateScript request: {}", e);
                    message.response.send(WebResponse::Error(e)).ok();
                }
            },
            WebRequest::DeleteScript(JsonScriptName { name }) => {
                match self.game_world.delete_script(name) {
                    Ok(_) => {
                        message.response.send(WebResponse::Done).ok();
                    }
                    Err(e) => {
                        tracing::error!("Failed DeleteScript request: {}", e);
                        message.response.send(WebResponse::Error(e)).ok();
                    }
                }
            }
        };
    }

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
                                tracing::error!("Player presence check error: {}", e);
                                client
                                    .send("Error retrieving user.\r\nName?\r\n> ".into())
                                    .await;
                                return;
                            }
                        };

                        if has_user {
                            if self.game_world.player_online(name).await {
                                client
                                    .send("User currently online.\r\nName?\r\n> ".into())
                                    .await;
                                return;
                            }
                            client.send("User located.\r\nPassword?\r\n> ".into()).await;
                            client.set_state(State::LoginPassword {
                                name: name.to_string(),
                            });
                        } else {
                            client
                                .send("New user detected.\r\nPassword?\r\n>".into())
                                .await;
                            client.set_state(State::CreatePassword {
                                name: name.to_string(),
                            });
                        }
                    } else {
                        client.send("Invalid username.\r\nName?\r\n> ".into()).await;
                    }
                }
                State::CreatePassword { name } => {
                    let name = name.clone();

                    if input.len() < 5 {
                        client
                            .send("Weak password detected.\r\nPassword?\r\n> ".into())
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
                            tracing::error!("Create password hash error: {}", e);
                            client
                                .send("Error computing password hash.\r\nPassword?\r\n> ".into())
                                .await;
                            return;
                        }
                    };

                    client
                        .send("Password accepted.\r\nVerify?\r\n> ".into())
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
                                tracing::error!("Create verify password failure: {}", e);
                                client.verification_failed_creation(name.as_str()).await;
                                return;
                            }
                        }
                    }

                    let spawn_room = self.game_world.spawn_room().await;
                    match self.db.create_player(name.as_str(), hash, spawn_room).await {
                        Ok(_) => (),
                        Err(e) => {
                            tracing::error!("User creation error: {}", e);
                            client.verification_failed_creation(name.as_str()).await;
                            return;
                        }
                    };

                    let player = match self
                        .db
                        .load_player(self.game_world.get_world(), name.as_str())
                        .await
                    {
                        Ok(player) => (player),
                        Err(e) => {
                            tracing::error!("Failed to load player: {}", e);
                            client.spawn_failed().await;
                            return;
                        }
                    };

                    client.verified().await;
                    client.set_state(State::InGame { player });

                    self.game_world
                        .player_action(Action::from(Login { entity: player }))
                        .await;
                    self.game_world
                        .player_action(Action::from(Look {
                            entity: player,
                            direction: None,
                        }))
                        .await;

                    spawned_player = Some(player);
                }
                State::LoginPassword { name } => {
                    let name = name.clone();

                    let hash = match self.db.get_user_hash(name.as_str()).await {
                        Ok(hash) => hash,
                        Err(e) => {
                            tracing::error!("Get user hash error: {}", e);
                            client.verification_failed_login().await;
                            return;
                        }
                    };

                    match verify_password(hash.as_str(), input.as_str()) {
                        Ok(_) => (),
                        Err(e) => {
                            if let VerifyError::Unknown(e) = e {
                                tracing::error!("Login verify password failure: {}", e);
                                client.verification_failed_creation(name.as_str()).await;
                                return;
                            }
                        }
                    }

                    let player = match self
                        .db
                        .load_player(self.game_world.get_world(), name.as_str())
                        .await
                    {
                        Ok(player) => (player),
                        Err(e) => {
                            tracing::error!("Failed to load player: {}", e);
                            client.spawn_failed().await;
                            return;
                        }
                    };

                    client.verified().await;
                    client.set_state(State::InGame { player });

                    self.game_world
                        .player_action(Action::from(Login { entity: player }))
                        .await;
                    self.game_world
                        .player_action(Action::from(Look {
                            entity: player,
                            direction: None,
                        }))
                        .await;

                    spawned_player = Some(player);
                }
                State::InGame { player } => {
                    tracing::debug!("{}> {:?} sent {:?}", self.tick, client_id, input);
                    match parse(*player, &input) {
                        Ok(action) => self.game_world.player_action(action).await,
                        Err(message) => client.send(format!("{}\r\n> ", message).into()).await,
                    }
                }
            }
        } else {
            tracing::error!("Received message from unknown client ({:?})", client_id);
        }

        if let Some(player) = spawned_player {
            self.clients.insert(client_id, player);
        }
    }
}

enum VerifyError {
    BadPassword,
    Unknown(String),
}

fn verify_password(hash: &str, password: &str) -> Result<(), VerifyError> {
    let password_hash = PasswordHash::new(hash)
        .map_err(|e| VerifyError::Unknown(format!("Hash parsing error: {}", e)))?;
    let hasher = Argon2::default();
    hasher
        .verify_password(password.as_bytes(), &password_hash)
        .map_err(|e| match e {
            password_hash::Error::Password => VerifyError::BadPassword,
            e => VerifyError::Unknown(format!("Verify password error: {}", e)),
        })?;
    Ok(())
}

fn name_valid(name: &str) -> bool {
    lazy_static! {
        // Match names with between 2 and 32 characters which are alphanumeric and possibly include
        // the following characters: ' ', ''', '-', and '_'
        static ref NAME_FILTER: Regex = Regex::new(r"^[[:alnum:] '\-_]{2,32}$").unwrap();
    }

    NAME_FILTER.is_match(name)
}
