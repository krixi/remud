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
    Connect(usize, mpsc::UnboundedSender<EngineMessage>),
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

pub struct Player {
    name: String,
    location: Entity,
    sender: mpsc::UnboundedSender<EngineMessage>,
}

impl Player {
    fn new(name: String, location: Entity, sender: mpsc::UnboundedSender<EngineMessage>) -> Self {
        Player {
            name,
            location,
            sender,
        }
    }
}

pub struct Room {
    description: String,
    smell: String,
}

pub enum Action {
    Look,
    Smell,
    Say(String),
    Shutdown,
}

pub struct WantsToSay {
    message: String,
}

fn say_system(
    mut commands: Commands,
    players_saying: Query<(Entity, &Player, &WantsToSay)>,
    players: Query<(Entity, &Player)>,
) {
    for (player_saying_entity, player_saying, wants_to_say) in players_saying.iter() {
        let location = player_saying.location;

        for (player_entity, player) in players.iter() {
            if player_entity == player_saying_entity {
                continue;
            }

            if player.location == location {
                let message = format!(
                    "\r\n{} says \"{}\"\r\n> ",
                    player_saying.name, wants_to_say.message
                );
                player.sender.send(EngineMessage::Output(message)).ok();
            }
        }

        commands.entity(player_saying_entity).remove::<WantsToSay>();
    }
}

#[derive(Default)]
pub struct WorldStatus {
    shutdown: bool,
}

pub struct Engine {
    engine_rx: mpsc::Receiver<ClientMessage>,
    control_tx: mpsc::Sender<ControlMessage>,
    client_txs: HashMap<usize, mpsc::UnboundedSender<EngineMessage>>,
    client_states: HashMap<usize, ClientState>,
    ticker: Interval,
    world: World,
    schedule: Schedule,
    spawn_room: Entity,
}

impl Engine {
    pub fn new(
        engine_rx: mpsc::Receiver<ClientMessage>,
        control_tx: mpsc::Sender<ControlMessage>,
    ) -> Self {
        let mut world = World::new();

        world.insert_resource(WorldStatus::default());

        let room = Room {
            description: String::from("A dull white light permeates this shapeless space."),
            smell: String::from("You smell a vast nothingness."),
        };

        let spawn_room = world.spawn().insert(room).id();

        let mut schedule = Schedule::default();

        let mut update = SystemStage::parallel();
        update.add_system(say_system.system());
        schedule.add_stage("update", update);

        Engine {
            engine_rx,
            control_tx,
            client_txs: HashMap::new(),
            client_states: HashMap::new(),
            ticker: interval(Duration::from_millis(15)),
            world,
            schedule,
            spawn_room,
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                _ = self.ticker.tick() => {
                    self.schedule.run_once(&mut self.world);

                    if let Some(status) = self.world.get_resource::<WorldStatus>() {
                        if status.shutdown {
                            break;
                        }
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
                            if let Some(tx) = self.client_txs.get(&client) {
                                let player = Player::new(name.clone(), self.spawn_room, tx.clone());
                                let player_entity = self.world.spawn().insert(player).id();

                                self.perform(client, player_entity, Action::Look).await;
                                self.send(client, String::from("> ")).await;

                                self.client_states.insert(
                                    client,
                                    ClientState::InGame {
                                        player: player_entity,
                                    },
                                );
                            }
                        }
                        ClientState::InGame { player } => {
                            let player = *player;

                            if input == "look" {
                                self.perform(client, player, Action::Look).await;
                            } else if input == "smell" {
                                self.perform(client, player, Action::Smell).await;
                            } else if input.starts_with("say ") {
                                let message = input.split_off(4);
                                self.perform(client, player, Action::Say(message)).await;
                            } else if input == "shutdown" {
                                self.perform(client, player, Action::Shutdown).await;
                            } else {
                                self.send(
                                    client,
                                    String::from("I don't know what that means.\r\n"),
                                )
                                .await;
                            }
                            self.send(client, String::from("> ")).await
                        }
                    }
                }
            }
        }
    }

    async fn send(&self, client: usize, string: String) {
        if let Some(tx) = self.client_txs.get(&client) {
            if tx.send(EngineMessage::Output(string)).is_err() {
                return;
            }
        }
    }

    async fn perform(&mut self, client: usize, player: Entity, action: Action) {
        match action {
            Action::Look => {
                let room = if let Some(player) = self.world.get::<Player>(player) {
                    player.location
                } else {
                    return;
                };

                if let Some(room) = self.world.get::<Room>(room) {
                    self.send(client, format!("{}\r\n", room.description)).await
                }
            }
            Action::Smell => {
                let room = if let Some(player) = self.world.get::<Player>(player) {
                    player.location
                } else {
                    return;
                };

                if let Some(room) = self.world.get::<Room>(room) {
                    self.send(client, format!("{}\r\n", room.smell)).await
                }
            }
            Action::Say(message) => {
                self.world.entity_mut(player).insert(WantsToSay { message });
            }
            Action::Shutdown => {
                if let Some(mut status) = self.world.get_resource_mut::<WorldStatus>() {
                    status.shutdown = true;
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
