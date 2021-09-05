mod world;

use crate::engine::world::{Action, GameWorld};
use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use tokio::{
    sync::mpsc,
    time::{interval, Duration, Interval},
};

pub enum ControlMessage {
    Shutdown,
}

#[derive(Debug)]
pub enum ClientMessage {
    Connect(usize, mpsc::Sender<EngineMessage>),
    Disconnect(usize),
    Ready(usize),
    Input(usize, String),
}

#[derive(Debug)]
pub enum EngineMessage {
    Output(String),
}

pub enum ClientState {
    LoginName,
    LoginPassword { name: String },
    InGame { player: Entity },
}

pub struct Engine {
    engine_rx: mpsc::Receiver<ClientMessage>,
    control_tx: mpsc::Sender<ControlMessage>,
    client_txs: HashMap<usize, mpsc::Sender<EngineMessage>>,
    client_states: HashMap<usize, ClientState>,
    player_clients: HashMap<Entity, usize>,
    ticker: Interval,
    game_world: GameWorld,
}

impl Engine {
    pub fn new(
        engine_rx: mpsc::Receiver<ClientMessage>,
        control_tx: mpsc::Sender<ControlMessage>,
    ) -> Self {
        let game_world = GameWorld::new();

        Engine {
            engine_rx,
            control_tx,
            client_txs: HashMap::new(),
            client_states: HashMap::new(),
            player_clients: HashMap::new(),
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

                    for (player, messages) in self.game_world.messages() {
                        if let Some(client) = self.player_clients.get(&player) {
                            for message in messages {
                                self.send(*client, message).await;
                            }
                            self.send(*client, "> ".to_string()).await;
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
            ClientMessage::Connect(client, client_tx) => {
                self.client_txs.insert(client, client_tx);
                self.client_states.insert(client, ClientState::LoginName);
                tracing::info!("Client {} connected", client);
            }
            ClientMessage::Disconnect(client) => {
                self.client_txs.remove(&client);
                self.client_states.remove(&client);
                tracing::info!("Client {} disconnected", client);
            }
            ClientMessage::Ready(client) => {
                let message = String::from("Welcome to the world.\r\n\r\nName?\r\n> ");
                self.send(client, message).await;
                tracing::info!("Client {} ready", client)
            }
            ClientMessage::Input(client, mut input) => {
                tracing::info!("Client {} sent {:?}", client, input);
                if let Some(client_state) = self.client_states.get(&client) {
                    match client_state {
                        ClientState::LoginName => {
                            if !name_valid(input.trim()) {
                                self.send(
                                    client,
                                    String::from("That name is invalid.\r\n\r\nName?\r\n> "),
                                )
                                .await;
                            } else {
                                self.send(client, String::from("Password?\r\n> ")).await;

                                self.client_states.insert(
                                    client,
                                    ClientState::LoginPassword {
                                        name: input.trim().to_string(),
                                    },
                                );
                            }
                        }
                        ClientState::LoginPassword { name } => {
                            let player = self.game_world.spawn_player(name.clone());

                            self.player_clients.insert(player, client);

                            self.client_states
                                .insert(client, ClientState::InGame { player });
                        }
                        ClientState::InGame { player } => {
                            let player = *player;

                            if input == "look" {
                                self.game_world.player_action(player, Action::Look);
                            } else if input.starts_with("say ") {
                                let message = input.split_off(4);
                                self.game_world.player_action(player, Action::Say(message));
                            } else if input == "shutdown" {
                                self.game_world.player_action(player, Action::Shutdown);
                            } else {
                                self.send(
                                    client,
                                    String::from("I don't know what that means.\r\n> "),
                                )
                                .await;
                            }
                        }
                    }
                }
            }
        }
    }

    async fn send(&self, client: usize, string: String) {
        if let Some(tx) = self.client_txs.get(&client) {
            if tx.send(EngineMessage::Output(string)).await.is_err() {
                return;
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
