use std::{borrow::Cow, collections::HashMap};

use bevy_ecs::prelude::Entity;
use tokio::sync::mpsc;

use crate::{
    engine::{
        db::Db,
        negotiate_login::{ClientLoginFsm, ClientParams, Transition},
        EngineResponse,
    },
    world::{action::commands::Commands, GameWorld},
    ClientId,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub enum SendPrompt {
    None,
    Prompt,
    Sensitive,
}

pub struct Client {
    id: ClientId,
    sender: ClientSender,
    fsm: ClientLoginFsm,
}

impl Client {
    pub fn player(&self) -> Option<Entity> {
        self.fsm.player()
    }

    pub fn expecting_sensitive_input(&self) -> bool {
        self.sender.expecting_sensitive_input.load(Ordering::SeqCst)
    }

    #[tracing::instrument(name = "update client fsm", fields(input = if self.expecting_sensitive_input() { "****" } else { input.unwrap_or("") }), skip(self, world, db, commands))]
    pub async fn update(
        &mut self,
        input: Option<&str>,
        world: &mut GameWorld,
        db: &Db,
        commands: &Commands,
    ) {
        let sender = &self.sender;

        let mut update_count: i32 = 0;
        while self
            .fsm
            .on_update(
                None,
                ClientParams::new(self.id, sender, world, db, commands).with_input(input),
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

    #[tracing::instrument(name = "transition client fsm", skip(self, world, db, commands))]
    pub async fn transition(
        &mut self,
        tx: Transition,
        world: &mut GameWorld,
        db: &Db,
        commands: &Commands,
    ) {
        let sender = &self.sender;
        self.fsm
            .on_update(
                Some(tx),
                &mut ClientParams::new(self.id, sender, world, db, commands),
            )
            .await;
    }

    pub async fn send<'a, M: Into<Cow<'a, str>>>(
        &self,
        prompt: SendPrompt,
        messages: impl IntoIterator<Item = M>,
    ) {
        self.sender.send(self.id, prompt, messages).await;
    }
}

pub struct ClientSender {
    tx: mpsc::Sender<EngineResponse>,
    expecting_sensitive_input: AtomicBool,
}

impl ClientSender {
    pub async fn send<'a, M: Into<Cow<'a, str>>>(
        &self,
        id: ClientId,
        prompt: SendPrompt,
        messages: impl IntoIterator<Item = M>,
    ) {
        self.expecting_sensitive_input
            .store(false, Ordering::SeqCst);
        let message = match prompt {
            SendPrompt::None => EngineResponse::from_messages(messages),
            SendPrompt::Prompt => EngineResponse::from_messages_prompt(messages, false),
            SendPrompt::Sensitive => {
                self.expecting_sensitive_input.store(true, Ordering::SeqCst);
                EngineResponse::from_messages_prompt(messages, true)
            }
        };
        tracing::debug!("{:?} <- {:?}.", id, message);
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
                sender: ClientSender {
                    tx,
                    expecting_sensitive_input: AtomicBool::new(false),
                },
                fsm: ClientLoginFsm::default(),
            },
        );
    }

    pub fn init_player(&mut self, client: ClientId, player: Entity) {
        self.by_player.entry(player).or_insert(client);
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
