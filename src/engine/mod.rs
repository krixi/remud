mod client;
mod db;
mod macros;
pub mod persist;

use argon2::{
    password_hash::{self, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use lazy_static::lazy_static;
use rand::rngs::OsRng;
use regex::Regex;
use tokio::{
    sync::mpsc,
    time::{interval, Duration, Interval},
};

use crate::{
    engine::{
        client::{Client, Clients, State},
        db::Db,
    },
    world::{
        action::{parse, Login, Look},
        types::Configuration,
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
    ) -> anyhow::Result<Self> {
        let db = Db::new("world.db").await?;
        let world = db.load_world().await?;

        let game_world = GameWorld::new(world);

        Ok(Engine {
            engine_rx,
            control_tx,
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
                    tokio::task::block_in_place(|| {
                        self.game_world.run();
                    });

                    for (player, mut messages) in self.game_world.messages() {
                        if let Some(client) = self.clients.by_player(player) {
                            messages.push_back("> ".to_string());
                            client.send_batch(self.tick, messages).await;
                        } else {
                            tracing::error!("Attempting to send messages to player without client: {:?}", player);
                        }
                    }

                    for update in self.game_world.updates() {
                        match update.enact(self.db.get_pool(), self.game_world.get_world()).await {
                            Ok(_) => (),
                            Err(e) => tracing::error!("Failed to execute update: {}", e),
                        };
                    }

                    if self.game_world.should_shutdown() {
                        break
                    }

                    self.tick += 1;
                }
                maybe_message = self.engine_rx.recv() => {
                    if let Some(message) = maybe_message {
                        self.process(message).await;
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
                    if let Err(e) = self.game_world.despawn_player(player) {
                        tracing::error!("Failed to despawn player: {}", e);
                    }
                }

                self.clients.remove(client_id);
            }
            ClientMessage::Ready(client_id) => {
                tracing::info!("{}> {:?} ready", self.tick, client_id);

                let message = String::from("User?\r\n> ");
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
                                tracing::error!("User presence check error: {}", e);
                                client
                                    .send("Error retrieving user.\r\nName?\r\n> ".into())
                                    .await;
                                return;
                            }
                        };

                        if has_user {
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
                                tracing::info!("Create verify password failure: {}", e);
                                client.verification_failed_creation(name.as_str()).await;
                                return;
                            }
                        }
                    }

                    let spawn_room = self
                        .game_world
                        .get_world()
                        .get_resource::<Configuration>()
                        .unwrap()
                        .spawn_room;

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
                        .load_player(self.game_world.get_world_mut(), name.as_str())
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

                    self.game_world.player_action(player, Box::new(Login {}));
                    self.game_world.player_action(player, Look::here());

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
                                tracing::info!("Login verify password failure: {}", e);
                                client.verification_failed_creation(name.as_str()).await;
                                return;
                            }
                        }
                    }

                    let player = match self
                        .db
                        .load_player(self.game_world.get_world_mut(), name.as_str())
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

                    self.game_world.player_action(player, Box::new(Login {}));
                    self.game_world.player_action(player, Look::here());

                    spawned_player = Some(player);
                }
                State::InGame { player } => {
                    tracing::info!("{}> {:?} sent {:?}", self.tick, client_id, input);
                    match parse(&input) {
                        Ok(action) => self.game_world.player_action(*player, action),
                        Err(message) => client.send(format!("{}\r\n> ", message).into()).await,
                    }
                }
            }
        } else {
            tracing::error!("Received message from unknown client ({:?})", client_id);
        }

        if let Some(player) = spawned_player {
            self.clients.set_player(client_id, player);
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
