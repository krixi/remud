use std::{borrow::Cow, collections::HashMap};

use bevy_ecs::prelude::*;
use tokio::sync::mpsc;

use crate::engine::db::Db;
use crate::engine::negotiate_login::{ClientData, ClientFSM, Params, Transition};
use crate::world::GameWorld;
use crate::{engine::EngineResponse, ClientId};

pub enum State {
    LoginName,
    CreatePassword { name: String },
    VerifyPassword { name: String, hash: String },
    LoginPassword { name: String },
    InGame { player: Entity },
}

pub enum SendPrompt {
    None,
    Prompt,
    SensitivePrompt,
}

pub struct Client {
    id: ClientId,
    sender: ClientSender,
    state: State, // TODO
    fsm: ClientFSM,
    data: ClientData,
}

impl Client {
    pub fn player(&self) -> Option<Entity> {
        self.data.player()
    }

    pub async fn update(&mut self, input: Option<&str>, world: &mut GameWorld, db: &Db) {
        let data = &mut self.data;
        let sender = &self.sender;
        self.fsm
            .on_update(
                None,
                data,
                &mut Params::new(self.id, sender, world, db).with_input(input),
            )
            .await;
    }

    pub async fn transition(&mut self, tx: Transition, world: &mut GameWorld, db: &Db) {
        let data = &mut self.data;
        let sender = &self.sender;
        self.fsm
            .on_update(Some(tx), data, &mut Params::new(self.id, sender, world, db))
            .await;
    }

    // TODO : get rid of this probably?
    pub async fn send_prompted(&self, tick: u64, message: Cow<'_, str>) {
        self.sender
            .send_batch(tick, self.id, SendPrompt::Prompt, vec![message])
            .await;
    }

    pub async fn send_batch<'a>(
        &self,
        tick: u64,
        prompt: SendPrompt,
        messages: impl IntoIterator<Item = Cow<'a, str>>,
    ) {
        self.sender
            .send_batch(tick, self.id, prompt, messages)
            .await;
    }

    pub fn get_state(&self) -> &State {
        &self.state
    }

    pub fn set_state(&mut self, new_state: State) {
        self.state = new_state;
    }

    pub async fn verification_failed_creation(&mut self, tick: u64, name: &str) {
        self.send_batch(
            tick,
            SendPrompt::SensitivePrompt,
            vec![
                Cow::from("|Red1|Verification failed.|-|"),
                Cow::from("|SteelBlue3|Password?|-|"),
            ],
        )
        .await;

        self.set_state(State::CreatePassword {
            name: name.to_string(),
        });
    }

    pub async fn verification_failed_login(&mut self, tick: u64) {
        self.send_batch(
            tick,
            SendPrompt::Prompt,
            vec![
                Cow::from("|Red1|Verification failed.|-|"),
                Cow::from("|SteelBlue3|Name?|-|"),
            ],
        )
        .await;
        self.set_state(State::LoginName {});
    }

    pub async fn spawn_failed(&mut self, tick: u64) {
        self.send_batch(
            tick,
            SendPrompt::Prompt,
            vec![
                Cow::from("|Red1|User instantiation failed.|-|"),
                Cow::from("|SteelBlue3|Name?|-|"),
            ],
        )
        .await;
        self.set_state(State::LoginName {});
    }

    pub async fn verified(&mut self, tick: u64) {
        self.send_batch(
            tick,
            SendPrompt::None,
            vec![
                Cow::from("|SteelBlue3|Password verified.|-|"),
                Cow::from(""),
                Cow::from("|white|Welcome to City Six."),
                Cow::from(""),
            ],
        )
        .await;
    }
}

#[derive(Clone)]
pub struct ClientSender {
    tx: mpsc::Sender<EngineResponse>,
}

impl ClientSender {
    pub async fn send_batch<'a>(
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
                state: State::LoginName,
                fsm: ClientFSM::default(),
                data: ClientData::default(),
            },
        );
    }

    pub fn get(&self, client: ClientId) -> Option<&Client> {
        self.clients.get(&client)
    }

    pub fn get_mut(&mut self, client: ClientId) -> Option<&mut Client> {
        self.clients.get_mut(&client)
    }

    pub fn insert(&mut self, client: ClientId, player: Entity) {
        self.by_player.insert(player, client);
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

    pub fn by_player(&self, player: Entity) -> Option<&Client> {
        self.by_player
            .get(&player)
            .and_then(|player| self.clients.get(player))
    }
}
