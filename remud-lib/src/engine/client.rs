use std::{borrow::Cow, collections::HashMap};

use bevy_ecs::prelude::Entity;
use tokio::sync::mpsc;

use crate::engine::db::Db;
use crate::engine::negotiate_login::{ClientData, ClientFSM, Params, Transition};
use crate::world::action::commands::Commands;
use crate::world::GameWorld;
use crate::{engine::EngineResponse, ClientId};

pub enum SendPrompt {
    None,
    Prompt,
    SensitivePrompt,
}

pub struct Client {
    id: ClientId,
    sender: ClientSender,
    fsm: ClientFSM,
    data: ClientData,
}

impl Client {
    pub fn player(&self) -> Option<Entity> {
        self.data.player()
    }

    pub async fn update(
        &mut self,
        input: Option<&str>,
        world: &mut GameWorld,
        db: &Db,
        commands: &Commands,
    ) {
        let data = &mut self.data;
        let sender = &self.sender;

        let mut update_count = 0;
        while self
            .fsm
            .on_update(
                None,
                data,
                &mut Params::new(self.id, sender, world, db, commands).with_input(input),
            )
            .await
        {
            // just go again
            update_count += 1;

            if update_count > 5 {
                tracing::warn!("HOLY **** there were FIVE updates what even");
                break;
            }
        }
    }

    pub async fn transition(
        &mut self,
        tx: Transition,
        world: &mut GameWorld,
        db: &Db,
        commands: &Commands,
    ) {
        let data = &mut self.data;
        let sender = &self.sender;
        self.fsm
            .on_update(
                Some(tx),
                data,
                &mut Params::new(self.id, sender, world, db, commands),
            )
            .await;
    }

    pub async fn send<'a>(
        &self,
        tick: u64,
        prompt: SendPrompt,
        messages: impl IntoIterator<Item = Cow<'a, str>>,
    ) {
        self.sender.send(tick, self.id, prompt, messages).await;
    }
}

#[derive(Clone)]
pub struct ClientSender {
    tx: mpsc::Sender<EngineResponse>,
}

impl ClientSender {
    pub async fn send<'a>(
        &self,
        tick: u64,
        id: ClientId,
        prompt: SendPrompt,
        messages: impl IntoIterator<Item = Cow<'a, str>>,
    ) {
        let message = match prompt {
            SendPrompt::None => EngineResponse::from_messages_noprompt(messages),
            SendPrompt::Prompt => EngineResponse::from_messages(messages, false),
            SendPrompt::SensitivePrompt => EngineResponse::from_messages(messages, true),
        };
        tracing::debug!("[{}] {:?} <- {:?}.", tick, id, message);
        if let Err(e) = self.tx.send(message).await {
            tracing::error!("failed to send message to client {:?}: {}", id, e);
        }
    }
}

#[derive(Default)]
pub(crate) struct Clients {
    clients: HashMap<ClientId, Client>,
    by_player: HashMap<Entity, ClientId>,
}

impl Clients {
    pub fn add(&mut self, client_id: ClientId, tx: mpsc::Sender<EngineResponse>) {
        self.clients.insert(
            client_id,
            Client {
                id: client_id,
                sender: ClientSender { tx },
                fsm: ClientFSM::default(),
                data: ClientData::default(),
            },
        );
    }

    pub fn init_player(&mut self, client: ClientId, player: Entity) {
        self.by_player.entry(player).or_insert(client);
    }

    pub fn get(&self, client: ClientId) -> Option<&Client> {
        self.clients.get(&client)
    }

    pub fn get_mut(&mut self, client: ClientId) -> Option<&mut Client> {
        self.clients.get_mut(&client)
    }

    pub fn remove(&mut self, client: ClientId) {
        let player = self.clients.get(&client).and_then(Client::player);

        if let Some(player) = player {
            self.by_player.remove(&player);
        }

        self.clients.remove(&client);
    }

    pub fn by_player(&self, player: Entity) -> Option<&Client> {
        self.by_player
            .get(&player)
            .and_then(|player| self.clients.get(player))
    }
}
