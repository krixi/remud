use std::collections::HashMap;

use bevy_ecs::prelude::*;
use tokio::sync::mpsc;

use crate::{engine::EngineMessage, ClientId};

pub enum ClientState {
    LoginName,
    LoginPassword { name: String },
    InGame { player: Entity },
}

pub struct Client {
    id: ClientId,
    tx: mpsc::Sender<EngineMessage>,
    state: ClientState,
}

impl Client {
    pub fn get_player(&self) -> Option<Entity> {
        match self.state {
            ClientState::InGame { player } => Some(player),
            _ => None,
        }
    }

    pub async fn send(&self, message: String) {
        if self.tx.send(EngineMessage::Output(message)).await.is_err() {
            tracing::error!("Failed to send message to client {:?}", self.id);
        }
    }

    pub async fn send_batch(&self, messages: Vec<String>) {
        for message in messages {
            if self.tx.send(EngineMessage::Output(message)).await.is_err() {
                tracing::error!("Failed to send message to client {:?}", self.id);
                break;
            }
        }
    }

    pub fn get_state(&self) -> &ClientState {
        &self.state
    }

    pub fn set_state(&mut self, new_state: ClientState) {
        self.state = new_state
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
                state: ClientState::LoginName,
            },
        );
    }

    pub fn remove(&mut self, client: ClientId) {
        let player = self
            .clients
            .get(&client)
            .map(|client| client.get_state())
            .and_then(|state| match state {
                ClientState::InGame { player } => Some(*player),
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
