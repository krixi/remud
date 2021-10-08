use crate::engine::client::{ClientSender, SendPrompt};
use crate::engine::db::{Db, GameDb};
use crate::engine::name_valid;
use crate::world::GameWorld;
use crate::{engine::EngineResponse, ClientId};
use anyhow::bail;
use async_trait::async_trait;
use bevy_ecs::prelude::*;
use futures::TryFutureExt;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, Ord, PartialOrd)]
pub enum StateId {
    NotConnected,
    ConnectionReady,
    LoginName,
    LoginFailed,
    LoginPassword,
    CreatePassword,
    VerifyPassword,
    AlreadyOnline,
    InGame,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum Transition {
    Disconnect,
    Ready,
    Then,
    FailLogin { msg: String },
    ExistsOffline,
    DoesNotExist,
    PasswordEntered { hash: String },
    PlayerLoaded { player: Entity },
}

pub struct Params<'a> {
    pub id: ClientId,
    pub input: Option<&'a str>,
    pub sender: &'a ClientSender,
    pub game_world: &'a mut GameWorld,
    pub db: &'a Db,
}
impl<'a> Params<'a> {
    pub fn new(
        id: ClientId,
        sender: &'a ClientSender,
        world: &'a mut GameWorld,
        db: &'a Db,
    ) -> Self {
        Params {
            id,
            input: None,
            sender,
            game_world: world,
            db,
        }
    }
    pub fn with_input(&mut self, input: Option<&'a str>) -> &mut Self {
        self.input = input;
        self
    }
}

#[derive(Default)]
pub struct ClientData {
    username: Option<String>,
    pw_hash: Option<String>,
    player: Option<Entity>,
    reason: Option<String>,
}

impl ClientData {
    pub fn player(&self) -> Option<Entity> {
        self.player
    }

    pub fn username(&self) -> Option<String> {
        self.username.as_ref().map(|s| s.clone())
    }

    pub fn pw_hash(&self) -> Option<String> {
        self.pw_hash.as_ref().map(|s| s.clone())
    }

    pub fn reason(&self) -> Option<String> {
        self.reason.as_ref().map(|s| s.clone())
    }
}

#[async_trait::async_trait]
pub trait ClientState: Send + Sync {
    fn id(&self) -> StateId;
    fn output_state(&self, next: &Transition) -> Option<StateId>;

    async fn on_enter<'a>(&mut self, _data: &mut ClientData, _params: &'a mut Params) {}
    async fn decide<'a>(
        &mut self,
        _data: &mut ClientData,
        _params: &'a mut Params,
    ) -> Option<Transition> {
        None
    }
    async fn act<'a>(&mut self, _data: &mut ClientData, _params: &'a mut Params) {}
    async fn on_exit<'a>(&mut self, _data: &mut ClientData, _params: &'a mut Params) {}
}

#[derive(Default)]
pub struct ClientFSMBuilder {
    states: Vec<(StateId, Box<dyn ClientState>)>,
}

impl ClientFSMBuilder {
    pub fn build(self) -> anyhow::Result<ClientFSM> {
        let mut states = HashMap::new();
        let mut first = None;
        for (id, state) in self.states {
            if first == None {
                first = Some(id)
            }
            states.insert(id, state);
        }

        if let Some(current) = first {
            Ok(ClientFSM { states, current })
        } else {
            bail!("No states found for client fsm")
        }
    }
    pub fn with_state(mut self, state: Box<dyn ClientState>) -> Self {
        self.states.push((state.id(), state));
        self
    }
}

pub struct ClientFSM {
    states: HashMap<StateId, Box<dyn ClientState>>,
    current: StateId,
}

// this state machine will always have the same shape, and there's only one of them,
// so implement default to construct it.
impl Default for ClientFSM {
    fn default() -> Self {
        let fsm = ClientFSMBuilder::default()
            .with_state(Box::new(NotConnectedState::default()))
            .build();

        fsm.unwrap()
    }
}

impl ClientFSM {
    pub async fn on_update<'a>(
        &mut self,
        tx: Option<Transition>,
        data: &mut ClientData,
        params: &'a mut Params<'a>,
    ) {
        // delegate to current state -
        let current_state = self.states.get_mut(&self.current).unwrap();

        // check if called with a direct transition or not, if not - decide
        // gets the new current state after any transitions occur
        let current_state = if let Some((next, tx)) = match tx {
            Some(tx) => Some((current_state.output_state(&tx), tx)),
            None => current_state
                .decide(data, params)
                .await
                .map(|tx| (current_state.output_state(&tx), tx)),
        } {
            // store any transition data
            match tx {
                Transition::FailLogin { msg } => data.reason = Some(msg),
                _ => (),
            };

            // update states if needed
            match next {
                Some(state) => {
                    // exit outgoing state.
                    current_state.on_exit(data, params).await;
                    // assert that the new state exists
                    let next_state = self
                        .states
                        .get_mut(&state)
                        .expect("missing state implementation for");
                    // update the current state reference
                    self.current = state;
                    // enter the new state
                    next_state.on_enter(data, params).await;
                    next_state
                }
                None => current_state,
            }
        } else {
            current_state
        };

        // finally: let the new current state act.
        current_state.act(data, params).await;
    }
}

/**

,
,
LoginName,
LoginPassword,
CreatePassword,
VerifyPassword,
PasswordRejected,
AlreadyOnline,
InGame,
**/

// Not connected (default) state.
#[derive(Default)]
pub struct NotConnectedState {}
impl ClientState for NotConnectedState {
    fn id(&self) -> StateId {
        StateId::NotConnected
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Ready => Some(StateId::ConnectionReady),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct ConnectionReady {}

#[async_trait::async_trait]
impl ClientState for ConnectionReady {
    fn id(&self) -> StateId {
        StateId::ConnectionReady
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Then => Some(StateId::LoginName),
            Transition::Disconnect => Some(StateId::NotConnected),
            _ => None,
        }
    }
    async fn on_enter<'a>(&mut self, _data: &mut ClientData, params: &'a mut Params) {
        params
            .sender
            .send_batch(
                0,
                params.id,
                SendPrompt::None,
                vec![Cow::from(
                    "|SteelBlue3|Connected to|-| |white|ucs://uplink.six.city|-|\r\n",
                )],
            )
            .await;
    }
    async fn decide<'a>(
        &mut self,
        _data: &mut ClientData,
        _params: &'a mut Params,
    ) -> Option<Transition> {
        Some(Transition::Then)
    }
}

// in this state we introduce ourselves
#[derive(Default)]
pub struct LoginNameState {
    name: Option<String>,
}

#[async_trait::async_trait]
impl ClientState for LoginNameState {
    fn id(&self) -> StateId {
        StateId::LoginName
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::ExistsOffline => Some(StateId::LoginPassword),
            Transition::DoesNotExist => Some(StateId::CreatePassword),
            Transition::FailLogin { .. } => Some(StateId::LoginFailed),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut ClientData, params: &'a mut Params) {
        data.username = None;
        params
            .sender
            .send_batch(
                0,
                params.id,
                SendPrompt::Prompt,
                vec![Cow::from("|SteelBlue3|Name?|-|")],
            )
            .await;
    }

    // handle input
    async fn decide<'a>(
        &mut self,
        data: &mut ClientData,
        params: &'a mut Params,
    ) -> Option<Transition> {
        let name = if let Some(input) = params.input {
            let name = input.trim();
            if !name_valid(name) {
                return Some(Transition::FailLogin {
                    msg: "".to_string(),
                });
            }
            name
        } else {
            return None; // shouldn't happen? :shrug:
        };

        data.username = Some(name.to_string());

        let has_user = match params.db.has_player(name).await {
            Ok(has_user) => has_user,
            Err(e) => {
                tracing::error!("player presence check error: {}", e);
                return Some(Transition::FailLogin {
                    msg: "|Red1|Error retrieving user.|-|".to_string(),
                });
            }
        };

        // do tests needed to figure out transition out of this state.
        None
    }
}

pub struct AlreadyOnlineState {}

impl ClientState for AlreadyOnlineState {
    fn id(&self) -> StateId {
        todo!()
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        todo!()
    }
}

pub struct LoginPasswordState {}

impl ClientState for LoginPasswordState {
    fn id(&self) -> StateId {
        todo!()
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        todo!()
    }
}

pub struct CreatePasswordState {}

impl ClientState for CreatePasswordState {
    fn id(&self) -> StateId {
        todo!()
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        todo!()
    }
}

pub struct VerifyPasswordState {}

impl ClientState for VerifyPasswordState {
    fn id(&self) -> StateId {
        todo!()
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        todo!()
    }
}

pub struct PasswordRejectedState {}

impl ClientState for PasswordRejectedState {
    fn id(&self) -> StateId {
        todo!()
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        todo!()
    }
}

pub struct InGameState {}

impl ClientState for InGameState {
    fn id(&self) -> StateId {
        todo!()
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        todo!()
    }
}
