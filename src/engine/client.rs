use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
};

use bevy_ecs::prelude::*;
use tokio::sync::mpsc;

use crate::{engine::EngineMessage, ClientId};

pub enum State {
    LoginName,
    CreatePassword { name: String },
    VerifyPassword { name: String, hash: String },
    LoginPassword { name: String },
    InGame { player: Entity },
}

pub struct Client {
    id: ClientId,
    tx: mpsc::Sender<EngineMessage>,
    state: State,
}

impl Client {
    pub fn get_player(&self) -> Option<Entity> {
        match self.state {
            State::InGame { player } => Some(player),
            _ => None,
        }
    }

    pub async fn send(&self, message: Cow<'_, str>) {
        if self
            .tx
            .send(EngineMessage::Output(message.to_string()))
            .await
            .is_err()
        {
            tracing::error!("Failed to send message to client {:?}", self.id);
        }
    }

    pub async fn send_batch(&self, tick: u64, messages: VecDeque<String>) {
        for message in messages {
            tracing::info!("{}> {:?} received {:?}.", tick, self.id, message);
            if self.tx.send(EngineMessage::Output(message)).await.is_err() {
                tracing::error!("Failed to send message to client {:?}", self.id);
                break;
            }
        }
        if self.tx.send(EngineMessage::EndOutput).await.is_err() {
            tracing::error!("Failed to send message to client {:?}", self.id);
        }
    }

    pub fn get_state(&self) -> &State {
        &self.state
    }

    pub fn set_state(&mut self, new_state: State) {
        self.state = new_state;
    }

    pub async fn verification_failed_creation(&mut self, name: &str) {
        self.send("Verification failed.\r\nPassword?\r\n> ".into())
            .await;
        self.set_state(State::CreatePassword {
            name: name.to_string(),
        });
    }

    pub async fn verification_failed_login(&mut self) {
        self.send("Verification failed.\r\nName?\r\n> ".into())
            .await;
        self.set_state(State::LoginName {});
    }

    pub async fn spawn_failed(&mut self) {
        self.send("User instantiation failed.\r\nName?\r\n> ".into())
            .await;
        self.set_state(State::LoginName {});
    }

    pub async fn verified(&mut self) {
        self.send("Password verified.\r\n\r\nWelcome to City Six.\r\n\r\n".into())
            .await;
    }
}

#[derive(Default)]
pub struct Clients {
    clients: HashMap<ClientId, Client>,
    by_player: HashMap<Entity, ClientId>,
}

impl Clients {
    pub fn add(&mut self, client_id: ClientId, tx: mpsc::Sender<EngineMessage>) {
        self.clients.insert(
            client_id,
            Client {
                id: client_id,
                tx,
                state: State::LoginName,
            },
        );
    }

    pub fn remove(&mut self, client: ClientId) {
        let player =
            self.clients
                .get(&client)
                .map(Client::get_state)
                .and_then(|state| match state {
                    State::InGame { player } => Some(*player),
                    _ => None,
                });

        if let Some(player) = player {
            self.by_player.remove(&player);
        }

        self.clients.remove(&client);
    }

    pub fn get(&self, client: ClientId) -> Option<&Client> {
        self.clients.get(&client)
    }

    pub fn get_mut(&mut self, client: ClientId) -> Option<&mut Client> {
        self.clients.get_mut(&client)
    }

    pub fn set_player(&mut self, client: ClientId, player: Entity) {
        self.by_player.insert(player, client);
    }

    pub fn by_player(&self, player: Entity) -> Option<&Client> {
        self.by_player
            .get(&player)
            .and_then(|player| self.clients.get(player))
    }
}
