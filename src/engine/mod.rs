pub mod db;
pub mod macros;
pub mod world;

mod clients;

use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;
use tokio::{
    sync::mpsc,
    time::{interval, Duration, Interval},
};

use crate::{engine::clients::ClientState, ClientId};

use self::{
    clients::Clients,
    world::{Action, GameWorld},
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
}

pub struct Engine {
    engine_rx: mpsc::Receiver<ClientMessage>,
    control_tx: mpsc::Sender<ControlMessage>,
    clients: Clients,
    ticker: Interval,
    game_world: GameWorld,
}

impl Engine {
    pub fn new(
        engine_rx: mpsc::Receiver<ClientMessage>,
        control_tx: mpsc::Sender<ControlMessage>,
        world: World,
    ) -> Self {
        let game_world = GameWorld::new(world);

        Engine {
            engine_rx,
            control_tx,
            clients: Clients::default(),
            ticker: interval(Duration::from_millis(15)),
            game_world,
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                _ = self.ticker.tick() => {
                    tokio::task::block_in_place(|| {
                        self.game_world.run()
                    });

                    for (player, mut messages) in self.game_world.messages() {
                        if let Some(client) = self.clients.by_player(player) {
                            messages.push("> ".to_string());
                            client.send_batch(messages).await;
                        } else {
                            tracing::error!("Attempting to send messages to player without client: {:?}", player);
                        }
                    }

                    if self.game_world.should_shutdown() {
                        break
                    }
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
                tracing::info!("{:?} connected", client_id);

                self.clients.add(client_id, tx);
            }
            ClientMessage::Disconnect(client_id) => {
                tracing::info!("{:?} disconnected", client_id);

                if let Some(player) = self
                    .clients
                    .get(client_id)
                    .and_then(|client| client.get_player())
                {
                    self.game_world.despawn_player(player);
                }

                self.clients.remove(client_id);
            }
            ClientMessage::Ready(client_id) => {
                tracing::info!("{:?} ready", client_id);

                let message = String::from("Welcome to the world.\r\n\r\nName?\r\n> ");
                if let Some(client) = self.clients.get(client_id) {
                    client.send(message).await;
                } else {
                    tracing::error!("Received message from unknown client: {:?}", message);
                }
            }
            ClientMessage::Input(client_id, mut input) => {
                let mut new_player = None;

                if let Some(client) = self.clients.get_mut(client_id) {
                    tracing::info!("{:?} sent {:?}", client_id, input);
                    match client.get_state() {
                        ClientState::LoginName => {
                            let name = input.trim();
                            if !name_valid(name) {
                                client
                                    .send(String::from("That name is invalid.\r\n\r\nName?\r\n> "))
                                    .await;
                            } else {
                                client.send(String::from("Password?\r\n> ")).await;
                                client.set_state(ClientState::LoginPassword {
                                    name: name.to_string(),
                                });
                            }
                        }
                        ClientState::LoginPassword { name } => {
                            let player = self.game_world.spawn_player(name.clone());
                            new_player = Some(player);
                            client.set_state(ClientState::InGame { player });
                        }
                        ClientState::InGame { player } => {
                            if input == "look" {
                                self.game_world.player_action(*player, Action::Look);
                            } else if input.starts_with("say ") {
                                let message = input.split_off(4);
                                self.game_world.player_action(*player, Action::Say(message));
                            } else if input == "shutdown" {
                                self.game_world.player_action(*player, Action::Shutdown);
                            } else {
                                client
                                    .send(String::from("I don't know what that means.\r\n> "))
                                    .await;
                            }
                        }
                    }
                } else {
                    tracing::error!("Received message from unknown client ({:?})", client_id);
                }

                if let Some(player) = new_player {
                    self.clients.set_player(client_id, player);
                }
            }
        }
    }
}

fn name_valid(name: &str) -> bool {
    lazy_static! {
        // Match names with between 2 and 32 characters which are alphanumeric and possibly include
        // the following characters: ' ', ''', '-', and '_'
        static ref NAME_FILTER: Regex = Regex::new(r"^[[:alnum:] '\-_]{2,32}$").unwrap();
    }

    NAME_FILTER.is_match(name)
}
