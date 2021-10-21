pub mod negotiate_login;
mod update_password;

use anyhow::bail;
use argon2::{
    password_hash::{self, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use rand::rngs::OsRng;
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
};

use crate::{
    engine::{
        client::{ClientEvent, ClientSender, EngineSender, SendPrompt},
        db::Db,
    },
    world::GameWorld,
};

#[async_trait::async_trait]
pub trait StackFsm {
    async fn on_update(
        &mut self,
        event: ClientEvent<'_>,
        params: &mut Params,
    ) -> Option<UpdateResult>;
}

pub trait Transition: Debug + Clone {}
impl<T> Transition for T where T: Debug + Clone {}

pub trait StateId: Debug + Copy + Clone + Eq + PartialEq + Hash {}
impl<SID> StateId for SID where SID: Debug + Copy + Clone + Eq + PartialEq + Hash {}

pub trait FsmState<T>: Send + Sync {
    fn update(&mut self, tx: &T);
}

pub struct Params<'p> {
    pub engine_sender: EngineSender,
    pub sender: &'p ClientSender,
    pub game_world: &'p mut GameWorld,
    pub db: &'p Db,
}

impl<'p> Params<'p> {
    pub fn new(
        engine_sender: EngineSender,
        sender: &'p ClientSender,
        world: &'p mut GameWorld,
        db: &'p Db,
    ) -> Self {
        Params {
            engine_sender,
            sender,
            game_world: world,
            db,
        }
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

pub enum FsmEvent<'a, T> {
    Transition(T),
    Advance,
    Input(&'a str),
}

impl<'a, T> TryFrom<ClientEvent<'a>> for FsmEvent<'a, T>
where
    T: TryFrom<ClientEvent<'a>, Error = ()>,
{
    type Error = ();

    fn try_from(value: ClientEvent<'a>) -> Result<Self, Self::Error> {
        match value {
            ClientEvent::Advance => Ok(FsmEvent::Advance),
            ClientEvent::Input(input) => Ok(FsmEvent::Input(input)),
            event => Ok(FsmEvent::Transition(event.try_into()?)),
        }
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
    S: FsmState<T>,
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
    async fn process<'p>(
        &mut self,
        input: Option<&str>,
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
    S: FsmState<T>,
{
    states: Vec<(SID, Box<dyn State<T, SID, S>>)>,
}

impl<T, SID, S> FsmBuilder<T, SID, S>
where
    T: Transition,
    SID: StateId,
    S: FsmState<T>,
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
    S: FsmState<T>,
{
    states: HashMap<SID, Box<dyn State<T, SID, S>>>,
    current: SID,
}

impl<T, SID, S> Fsm<T, SID, S>
where
    T: Transition,
    SID: StateId,
    S: FsmState<T>,
{
    pub async fn on_update(
        &mut self,
        event: FsmEvent<'_, T>,
        data: &mut S,
        params: &mut Params<'_>,
    ) -> UpdateResult {
        // delegate to current state -
        let current_state = self.states.get_mut(&self.current).unwrap();

        // determine the next state by applying a transition or processing input
        let next = match event {
            FsmEvent::Transition(tx) => {
                data.update(&tx);
                let next = current_state.next_state(&tx);
                tracing::info!("{:?} * {:?} -> {:?}", current_state.id(), tx, next);
                Some(next)
            }
            FsmEvent::Input(input) => {
                match current_state.process(Some(input), data, params).await {
                    Some(action) => match action {
                        TransitionAction::Transition(tx) => {
                            data.update(&tx);
                            let next = current_state.next_state(&tx);
                            tracing::info!("{:?} * {:?} -> {:?}", current_state.id(), tx, next);
                            Some(next)
                        }
                        TransitionAction::PushFsm(fsm) => return UpdateResult::PushFsm(fsm),
                        TransitionAction::PopFsm => return UpdateResult::PopFsm,
                    },
                    None => None,
                }
            }
            FsmEvent::Advance => match current_state.process(None, data, params).await {
                Some(action) => match action {
                    TransitionAction::Transition(tx) => {
                        data.update(&tx);
                        let next = current_state.next_state(&tx);
                        tracing::info!("{:?} * {:?} -> {:?}", current_state.id(), tx, next);
                        Some(next)
                    }
                    TransitionAction::PushFsm(fsm) => return UpdateResult::PushFsm(fsm),
                    TransitionAction::PopFsm => return UpdateResult::PopFsm,
                },
                None => None,
            },
        };

        // gets the new current state after any transitions occur
        let current_state = if let Some(next) = next {
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

pub enum VerifyError {
    BadPassword,
    Unknown(String),
}

pub fn verify_password(hash: &str, password: &str) -> Result<(), VerifyError> {
    let password_hash = PasswordHash::new(hash)
        .map_err(|e| VerifyError::Unknown(format!("Hash parsing error: {}", e)))?;
    let hasher = Argon2::default();
    hasher
        .verify_password(password.as_bytes(), &password_hash)
        .map_err(|e| match e {
            password_hash::Error::Password => VerifyError::BadPassword,
            e => VerifyError::Unknown(format!("Verify password error: {}", e)),
        })?;
    Ok(())
}
