use std::{borrow::Cow, collections::HashMap, iter};

use bevy_ecs::prelude::Entity;
use tokio::sync::mpsc;

use crate::{
    engine::{
        db::Db,
        fsm::{negotiate_login::ClientLoginFsm, Params, StackFsm, UpdateResult},
        ClientMessage, EngineResponse,
    },
    world::GameWorld,
    ClientId,
};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub enum SendPrompt {
    None,
    Prompt,
    Sensitive,
}

#[derive(Debug, Clone)]
pub enum ClientEvent<'a> {
    Advance,
    Disconnect,
    Input(&'a str),
    PasswordHash(Option<String>),
    PasswordVerification(Option<bool>),
    Ready,
}

pub struct Client {
    engine_sender: EngineSender,
    client_sender: ClientSender,
    root: ClientLoginFsm,
    fsms: Vec<Box<dyn StackFsm + Send + Sync>>,
}

impl Client {
    #[tracing::instrument(
        name = "process client event", 
        fields(
            event = match &event {
                ClientEvent::Input(input) => if self.expecting_sensitive_input() { "******" } else { input },
                ClientEvent::Ready => "ready",
                ClientEvent::Disconnect => "disconnect",
                ClientEvent::Advance => "advance",
                ClientEvent::PasswordHash(_) => "pw-hash",
                ClientEvent::PasswordVerification(verified) => 
                    match verified {
                        Some(true) => "verified",
                        Some(false) => "not verified",
                        None => "failed verification"
                    }
            }
        ),
        skip(self, world, db)
    )]
    pub async fn process(&mut self, event: ClientEvent<'_>, world: &mut GameWorld, db: &Db) {
        let mut event = iter::once(event).chain(iter::repeat(ClientEvent::Advance));
        let mut update_count: i32 = 0;
        while update_count < 5 {
            update_count += 1;

            let mut params = Params::new(
                self.engine_sender.clone(),
                &self.client_sender,
                world,
                db,
            );

            let event = event.next().unwrap();
            let mut result = None;
            for fsm in self.fsms.iter_mut().rev() {
                if let Some(r) = fsm.on_update(event.clone(), &mut params).await {
                    result = Some(r);
                    break;
                }
            }
            if result.is_none() {
                result = self.root.on_update(event, &mut params).await
            }

            if let Some(result) = result {
                match result {
                    UpdateResult::PushFsm(fsm) => {
                        self.fsms.push(fsm);
                        // don't break, allow the new FSM's on_enter to run at minimum
                    }
                    UpdateResult::PopFsm => {
                        self.fsms.pop();
                        break;
                    }
                    UpdateResult::Continue => (),
                    UpdateResult::Stop => break,
                }
            } else {
                tracing::warn!("unhandled client FSM stack message");
            }
        }
    }

    pub fn player(&self) -> Option<Entity> {
        self.root.player()
    }

    pub fn expecting_sensitive_input(&self) -> bool {
        self.client_sender
            .expecting_sensitive_input
            .load(Ordering::SeqCst)
    }

    pub async fn send<'a, M: Into<Cow<'a, str>>>(
        &self,
        prompt: SendPrompt,
        messages: impl IntoIterator<Item = M>,
    ) {
        self.client_sender.send(prompt, messages).await;
    }
}

#[derive(Clone)]
pub struct EngineSender {
    id: ClientId,
    tx: mpsc::Sender<ClientMessage>,
}

impl EngineSender {
    pub fn password_hash(&self, hash: Option<String>) {
        self.tx.blocking_send(ClientMessage::PasswordHash(self.id, hash)).ok();
    }

    pub fn password_verification(&self, verified: Option<bool>) {
        self.tx.blocking_send(ClientMessage::PasswordVerification(self.id, verified)).ok();
    }
}

pub struct ClientSender {
    tx: mpsc::Sender<EngineResponse>,
    expecting_sensitive_input: AtomicBool,
}

impl ClientSender {
    #[tracing::instrument(name = "client send", skip(self, messages))]
    pub async fn send<'a, M: Into<Cow<'a, str>>>(
        &self,
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
        tracing::debug!("{:?}", message);
        if let Err(e) = self.tx.send(message).await {
            tracing::error!("failed to send message to client: {}", e);
        }
    }
}

#[derive(Default)]
pub(crate) struct Clients {
    clients: HashMap<ClientId, Client>,
    by_player: HashMap<Entity, ClientId>,
}

impl Clients {
    pub fn add(
        &mut self,
        client_id: ClientId,
        client_tx: mpsc::Sender<ClientMessage>,
        engine_tx: mpsc::Sender<EngineResponse>,
    ) {
        self.clients.insert(
            client_id,
            Client {
                engine_sender: EngineSender {
                    id: client_id,
                    tx: client_tx,
                },
                client_sender: ClientSender {
                    tx: engine_tx,
                    expecting_sensitive_input: AtomicBool::new(false),
                },
                root: ClientLoginFsm::default(),
                fsms: Vec::new(),
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
