use std::{borrow::Cow, collections::HashMap};

use bevy_ecs::prelude::Entity;
use either::Either;
use tokio::sync::mpsc;

use crate::{
    engine::{
        db::Db,
        fsm::{
            negotiate_login::{ClientLoginFsm, Transition},
            Params, StackFsm, UpdateResult,
        },
        EngineResponse,
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

pub struct Client {
    id: ClientId,
    sender: ClientSender,
    root: ClientLoginFsm,
    fsms: Vec<Box<dyn StackFsm + Send + Sync>>,
}

impl Client {
    #[tracing::instrument(
        name = "update current fsm", 
        fields(
            input = if self.expecting_sensitive_input() { "****" } else { input.unwrap_or("") },
        ),
        skip(self, world, db, )
    )]
    pub async fn update(&mut self, input: Option<&str>, world: &mut GameWorld, db: &Db) {
        let mut update_count: i32 = 0;
        while update_count < 5 {
            update_count += 1;

            let mut params = Params::new(self.id, &mut self.sender, world, db);
            params.with_input(input);

            let current_fsm = if let Some(stack_fsm) = self.fsms.last_mut() {
                Either::Left(stack_fsm)
            } else {
                Either::Right(&mut self.root)
            };

            let result = match current_fsm {
                Either::Left(fsm) => fsm.on_update(&mut params).await,
                Either::Right(fsm) => fsm.on_update(None, &mut params).await,
            };

            match result {
                UpdateResult::PushFsm(fsm) => {
                    self.fsms.push(fsm);
                    // don't break, allow the new FSM to start and run at least one cycle
                }
                UpdateResult::PopFsm => {
                    self.fsms.pop();
                    break;
                }
                UpdateResult::Continue => (),
                UpdateResult::Stop => break,
            }
        }
    }

    #[tracing::instrument(name = "transition client fsm", skip(self, world, db))]
    pub async fn transition(&mut self, tx: Transition, world: &mut GameWorld, db: &Db) {
        self.root
            .on_update(
                Some(tx),
                &mut Params::new(self.id, &mut self.sender, world, db),
            )
            .await;
    }

    pub fn player(&self) -> Option<Entity> {
        self.root.player()
    }

    pub fn expecting_sensitive_input(&self) -> bool {
        self.sender.expecting_sensitive_input.load(Ordering::SeqCst)
    }

    pub async fn send<'a, M: Into<Cow<'a, str>>>(
        &self,
        prompt: SendPrompt,
        messages: impl IntoIterator<Item = M>,
    ) {
        self.sender.send(prompt, messages).await;
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
    pub fn add(&mut self, client_id: ClientId, tx: mpsc::Sender<EngineResponse>) {
        self.clients.insert(
            client_id,
            Client {
                id: client_id,
                sender: ClientSender {
                    tx,
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
