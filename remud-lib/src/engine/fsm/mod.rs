pub mod negotiate_login;
mod update_password;

use anyhow::bail;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use rand::rngs::OsRng;
use std::{borrow::Cow, collections::HashMap, fmt::Debug, hash::Hash};

use crate::{
    engine::{
        client::{ClientSender, SendPrompt},
        db::Db,
    },
    world::GameWorld,
    ClientId,
};

#[async_trait::async_trait]
pub trait StackFsm {
    async fn on_update(&mut self, params: &mut Params) -> UpdateResult;
}

pub trait Transition: Debug {}
impl<T> Transition for T where T: Debug {}

pub trait StateId: Debug + Copy + Clone + Eq + PartialEq + Hash {}
impl<SID> StateId for SID where SID: Debug + Copy + Clone + Eq + PartialEq + Hash {}

pub trait FsmState: Send + Sync {}
impl<S> FsmState for S where S: Send + Sync {}

pub struct Params<'p> {
    pub id: ClientId,
    pub input: Option<&'p str>,
    pub sender: &'p ClientSender,
    pub game_world: &'p mut GameWorld,
    pub db: &'p Db,
}

impl<'p> Params<'p> {
    pub fn new(
        id: ClientId,
        sender: &'p ClientSender,
        world: &'p mut GameWorld,
        db: &'p Db,
    ) -> Self {
        Params {
            id,
            input: None,
            sender,
            game_world: world,
            db,
        }
    }

    pub fn with_input(&mut self, input: Option<&'p str>) -> &mut Self {
        self.input = input;
        self
    }

    pub async fn send<M: Into<Cow<'p, str>>>(&self, messages: impl IntoIterator<Item = M>) {
        self.sender
            .send(SendPrompt::None, messages.into_iter().map(Into::into))
            .await;
    }

    pub async fn send_prompt<M: Into<Cow<'p, str>>>(&self, messages: impl IntoIterator<Item = M>) {
        self.sender
            .send(SendPrompt::Prompt, messages.into_iter().map(Into::into))
            .await;
    }

    pub async fn send_sensitive_prompt<M: Into<Cow<'p, str>>>(
        &self,
        messages: impl IntoIterator<Item = M>,
    ) {
        self.sender
            .send(SendPrompt::Sensitive, messages.into_iter().map(Into::into))
            .await;
    }
}

pub enum TransitionAction<T>
where
    T: Transition,
{
    Transition(T),
    PushFsm(Box<dyn StackFsm + Send + Sync>),
    PopFsm,
}

pub enum UpdateResult {
    PushFsm(Box<dyn StackFsm + Send + Sync>),
    PopFsm,
    Continue,
    Stop,
}

#[async_trait::async_trait]
pub trait State<T, SID, S>: Send + Sync
where
    T: Transition,
    SID: StateId,
    S: FsmState,
{
    fn id(&self) -> SID;

    fn output_state(&self, next: &T) -> Option<SID>;

    fn next_state(&self, tx: &T) -> Option<SID> {
        return match self.output_state(tx) {
            Some(state_id) => Some(state_id),
            None => {
                tracing::warn!(
                    "{:?} -> {:?} -> ? (unable to find output state)",
                    self.id(),
                    tx
                );
                None
            }
        };
    }

    // whether or not to call the state update again.
    fn keep_going(&self) -> bool {
        false
    }

    #[allow(unused_variables)]
    async fn on_enter<'p>(&mut self, state: &mut S, params: &'p mut Params<'_>) {}

    #[allow(unused_variables)]
    async fn decide<'p>(
        &mut self,
        state: &mut S,
        params: &'p mut Params<'_>,
    ) -> Option<TransitionAction<T>> {
        None
    }

    #[allow(unused_variables)]
    async fn on_exit<'p>(&mut self, state: &mut S, params: &'p mut Params<'_>) {}
}

pub struct FsmBuilder<T, SID, S>
where
    T: Transition,
    SID: StateId,
    S: FsmState,
{
    states: Vec<(SID, Box<dyn State<T, SID, S>>)>,
}

impl<T, SID, S> FsmBuilder<T, SID, S>
where
    T: Transition,
    SID: StateId,
    S: FsmState,
{
    pub fn new() -> Self {
        FsmBuilder { states: Vec::new() }
    }

    pub fn build(self) -> anyhow::Result<Fsm<T, SID, S>> {
        let mut states = HashMap::new();
        let mut first = None;
        for (id, state) in self.states {
            if first == None {
                first = Some(id)
            }
            states.insert(id, state);
        }

        if let Some(current) = first {
            Ok(Fsm { states, current })
        } else {
            bail!("No states found for client fsm")
        }
    }

    pub fn with_state(mut self, state: Box<dyn State<T, SID, S>>) -> Self {
        self.states.push((state.id(), state));
        self
    }
}

pub struct Fsm<T, SID, S>
where
    T: Transition,
    SID: StateId,
    S: FsmState,
{
    states: HashMap<SID, Box<dyn State<T, SID, S>>>,
    current: SID,
}

impl<T, SID, S> Fsm<T, SID, S>
where
    T: Transition,
    SID: StateId,
    S: FsmState,
{
    pub async fn on_update(
        &mut self,
        tx: Option<T>,
        data: &mut S,
        params: &mut Params<'_>,
    ) -> UpdateResult {
        // delegate to current state -
        let current_state = self.states.get_mut(&self.current).unwrap();

        // check if called with a direct transition or not, if not - decide
        // gets the new current state after any transitions occur
        let current_state = if let Some(next) = match tx {
            Some(tx) => {
                let next = current_state.next_state(&tx);
                tracing::info!("{:?} - {:?} -> {:?}", current_state.id(), tx, next);
                Some(next)
            }
            None => match current_state.decide(data, params).await {
                Some(action) => match action {
                    TransitionAction::Transition(tx) => Some(current_state.next_state(&tx)),
                    TransitionAction::PushFsm(fsm) => return UpdateResult::PushFsm(fsm),
                    TransitionAction::PopFsm => return UpdateResult::PopFsm,
                },
                None => None,
            },
        } {
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

        // the new current state knows best if it should be called again right away.
        match current_state.keep_going() {
            true => UpdateResult::Continue,
            false => UpdateResult::Stop,
        }
    }
}

pub fn verify_len(input: &str) -> Option<String> {
    if input.len() < 5 {
        return Some("|Red1|Weak password detected.|-|".to_string());
    } else if input.len() > 1024 {
        return Some("|Red1|Password too strong :(|-|".to_string());
    }
    None
}

pub fn hash_input(input: &str) -> Result<String, anyhow::Error> {
    // TODO: Add zeroizing library to clear password input from memory
    let hasher = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let hash = match hasher
        .hash_password(input.as_bytes(), &salt)
        .map(|hash| hash.to_string())
    {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!("create password hash error: {}", e);
            bail!(e.to_string())
        }
    };
    Ok(hash)
}
