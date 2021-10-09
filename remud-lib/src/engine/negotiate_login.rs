use crate::{
    engine::client::{ClientSender, SendPrompt},
    engine::db::{verify_password, AuthDb, Db, GameDb, VerifyError},
    engine::name_valid,
    world::action::commands::Commands,
    world::action::observe::Look,
    world::action::system::Login,
    world::action::Action,
    world::types::player,
    world::types::player::PlayerFlags,
    world::GameWorld,
    ClientId,
};
use anyhow::bail;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use bevy_ecs::prelude::Entity;
use bevy_ecs::schedule::IntoRunCriteria;
use rand::rngs::OsRng;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

static DEFAULT_LOGIN_ERROR: &'static str = "|Red1|Error retrieving user.|-|";
static DEFAULT_PASSWORD_ERROR: &'static str = "|Red1|Verification failed.|-|";

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, Ord, PartialOrd)]
pub enum StateId {
    NotConnected,
    ConnectionReady,
    LoginName,
    LoginFailed,
    LoginPassword,
    CreatePassword,
    VerifyPassword,
    CreateNewPlayer,
    SpawnPlayer,
    InGame,
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub enum Transition {
    Disconnect,
    Ready,
    Then,
    FailLogin { msg: String },
    ExistsOffline,
    PlayerDoesNotExist,
    CreatedPassword { hash: String },
    FailPassword,
    VerifiedPassword,
    PlayerCreated,
    PlayerLoaded { player: Entity },
}

impl Debug for Transition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Transition::Disconnect => write!(f, "Disconnect"),
            Transition::Ready => write!(f, "Ready"),
            Transition::Then => write!(f, "Then"),
            Transition::FailLogin { .. } => write!(f, "FailLogin"),
            Transition::ExistsOffline => write!(f, "ExistsOffline"),
            Transition::PlayerDoesNotExist => write!(f, "PlayerDoesNotExist"),
            Transition::CreatedPassword { .. } => write!(f, "CreatedPassword"),
            Transition::VerifiedPassword => write!(f, "VerifiedPassword"),
            Transition::FailPassword => write!(f, "FailPassword"),
            Transition::PlayerCreated => write!(f, "PlayerCreated"),
            Transition::PlayerLoaded { .. } => write!(f, "PlayerLoaded"),
        }
    }
}

pub struct Params<'a> {
    pub id: ClientId,
    pub input: Option<&'a str>,
    pub sender: &'a ClientSender,
    pub game_world: &'a mut GameWorld,
    pub db: &'a Db,
    pub commands: &'a Commands,
}
impl<'a> Params<'a> {
    pub fn new(
        id: ClientId,
        sender: &'a ClientSender,
        world: &'a mut GameWorld,
        db: &'a Db,
        commands: &'a Commands,
    ) -> Self {
        Params {
            id,
            input: None,
            sender,
            game_world: world,
            db,
            commands,
        }
    }
    pub fn with_input(&mut self, input: Option<&'a str>) -> &mut Self {
        self.input = input;
        self
    }

    pub async fn send<M: Into<Cow<'a, str>>>(&self, messages: impl IntoIterator<Item = M>) {
        self.sender
            .send(
                0,
                self.id,
                SendPrompt::None,
                messages.into_iter().map(Into::into),
            )
            .await;
    }

    pub async fn send_prompt<M: Into<Cow<'a, str>>>(&self, messages: impl IntoIterator<Item = M>) {
        self.sender
            .send(
                0,
                self.id,
                SendPrompt::Prompt,
                messages.into_iter().map(Into::into),
            )
            .await;
    }
    pub async fn send_sensitive_prompt<M: Into<Cow<'a, str>>>(
        &self,
        messages: impl IntoIterator<Item = M>,
    ) {
        self.sender
            .send(
                0,
                self.id,
                SendPrompt::SensitivePrompt,
                messages.into_iter().map(Into::into),
            )
            .await;
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
}

#[async_trait::async_trait]
pub trait ClientState: Send + Sync {
    fn id(&self) -> StateId;

    // TODO: add wrapper that logs None as a warning with state information
    fn output_state(&self, next: &Transition) -> Option<StateId>;
    fn next_state(&self, tx: &Transition) -> Option<StateId> {
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
        self.next_state(&Transition::Then).is_some()
    }

    #[allow(unused_variables)]
    async fn on_enter<'a>(&mut self, data: &mut ClientData, params: &'a mut Params) {}

    #[allow(unused_variables)]
    async fn decide<'a>(
        &mut self,
        data: &mut ClientData,
        params: &'a mut Params,
    ) -> Option<Transition> {
        None
    }
    #[allow(unused_variables)]
    async fn act<'a>(&mut self, data: &mut ClientData, params: &'a mut Params) {}
    #[allow(unused_variables)]
    async fn on_exit<'a>(&mut self, data: &mut ClientData, params: &'a mut Params) {}
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
            .with_state(Box::new(ConnectionReady::default()))
            .with_state(Box::new(LoginNameState::default()))
            .with_state(Box::new(LoginPasswordState::default()))
            .with_state(Box::new(LoginFailedState::default()))
            .with_state(Box::new(CreatePasswordState::default()))
            .with_state(Box::new(VerifyPasswordState::default()))
            .with_state(Box::new(SpawnPlayerState::default()))
            .with_state(Box::new(CreateNewPlayerState::default()))
            .with_state(Box::new(InGameState::default()))
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
    ) -> bool {
        // delegate to current state -
        let current_state = self.states.get_mut(&self.current).unwrap();

        // check if called with a direct transition or not, if not - decide
        // gets the new current state after any transitions occur
        let current_state = if let Some((next, tx)) = match tx {
            Some(tx) => Some((current_state.next_state(&tx), tx)),
            None => current_state
                .decide(data, params)
                .await
                .map(|tx| (current_state.next_state(&tx), tx)),
        } {
            let tx_copy = tx.clone();
            // store any transition data
            match tx {
                Transition::FailLogin { msg } => data.reason = Some(msg),
                Transition::PlayerLoaded { player } => data.player = Some(player),
                Transition::CreatedPassword { hash } => data.pw_hash = Some(hash),
                Transition::Disconnect => {
                    // TODO: data.clear() or data.reset()?
                    data.reason = None;
                    data.username = None;
                    data.pw_hash = None;
                    data.player = None;
                }
                _ => (),
            };

            // update states if needed
            match next {
                Some(state) => {
                    tracing::info!("{:?} - {:?} -> {:?}", current_state.id(), tx_copy, state); // TODO shane make this better
                    drop(tx_copy);
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

        // the new current state knows best if it should be called again right away.
        current_state.keep_going()
    }
}

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
            .send(
                0,
                params.id,
                SendPrompt::None,
                vec![
                    Cow::from("|SteelBlue3|Connected to|-| |white|ucs://uplink.six.city|-|"),
                    Cow::from(""),
                ],
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
pub struct LoginNameState {}

#[async_trait::async_trait]
impl ClientState for LoginNameState {
    fn id(&self) -> StateId {
        StateId::LoginName
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::ExistsOffline => Some(StateId::LoginPassword),
            Transition::PlayerDoesNotExist => Some(StateId::CreatePassword),
            Transition::FailLogin { .. } => Some(StateId::LoginFailed),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut ClientData, params: &'a mut Params) {
        data.username = None;
        params
            .sender
            .send(
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
                    msg: "Invalid username.".to_string(),
                });
            }
            name
        } else {
            return None; // shouldn't happen? :shrug:
        };

        let (has_user, user_online) = match params.db.has_player(name).await {
            Ok(has_user) => (has_user, params.game_world.player_online(name)),
            Err(e) => {
                tracing::error!("player presence check error: {}", e);
                return Some(Transition::FailLogin {
                    msg: DEFAULT_LOGIN_ERROR.to_string(),
                });
            }
        };

        data.username = Some(name.to_string());

        // was there a user and what is their connection status?
        if has_user {
            if user_online {
                return Some(Transition::FailLogin {
                    msg: DEFAULT_LOGIN_ERROR.to_string(), //TODO?
                });
            }

            // they were offline
            params
                .sender
                .send(
                    0,
                    params.id,
                    SendPrompt::None,
                    vec![Cow::from("|SteelBlue3|User located.|-|")],
                )
                .await;
            return Some(Transition::ExistsOffline);
        } else {
            if user_online {
                // how??
                tracing::error!("player is online but not found in db: {}", name);
                return Some(Transition::FailLogin {
                    msg: DEFAULT_LOGIN_ERROR.to_string(),
                });
            }
        }

        // they didn't exist
        params
            .sender
            .send(
                0,
                params.id,
                SendPrompt::None,
                vec![Cow::from("|SteelBlue3|New user detected.|-|")],
            )
            .await;
        return Some(Transition::PlayerDoesNotExist);
    }
}

#[derive(Default)]
pub struct LoginPasswordState {}

#[async_trait::async_trait]
impl ClientState for LoginPasswordState {
    fn id(&self) -> StateId {
        StateId::LoginPassword
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::VerifiedPassword => Some(StateId::SpawnPlayer),
            Transition::FailPassword => Some(StateId::LoginFailed),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut ClientData, params: &'a mut Params) {
        data.pw_hash = None;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Password?|-|"])
            .await;
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientData,
        params: &'a mut Params,
    ) -> Option<Transition> {
        if params.input.is_none() {
            params.send(vec![DEFAULT_PASSWORD_ERROR]).await;
            return Some(Transition::FailPassword);
        }
        let input = params.input.unwrap();

        // these will be error transitions.
        if let Some(tx) = verify_len(input) {
            if let Transition::FailLogin { msg } = tx {
                params.send(vec![msg]).await;
            }
            return Some(Transition::FailPassword);
        }

        if let Some(name) = data.username.as_ref().map(|s| s.as_str()) {
            return match params.db.verify_player(name, input).await {
                Ok(verified) => {
                    if verified {
                        params
                            .send(vec!["|SteelBlue3|Password verified.|-|", ""])
                            .await;
                        Some(Transition::VerifiedPassword)
                    } else {
                        tracing::info!("verification failed for user {}", name);
                        Some(Transition::FailLogin {
                            msg: DEFAULT_LOGIN_ERROR.to_string(),
                        })
                    }
                }
                Err(e) => {
                    tracing::error!("get user hash error: {:?}", e);
                    Some(Transition::FailLogin {
                        msg: DEFAULT_LOGIN_ERROR.to_string(),
                    })
                }
            };
        }
        None
    }
}

fn verify_len(input: &str) -> Option<Transition> {
    if input.len() < 5 {
        return Some(Transition::FailLogin {
            msg: "|Red1|Weak password detected.|-|".to_string(),
        });
    } else if input.len() > 1024 {
        return Some(Transition::FailLogin {
            msg: "|Red1|Password too strong :(|-|".to_string(),
        });
    }
    None
}

fn hash_input(input: &str) -> Result<String, anyhow::Error> {
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
    Ok(hash.to_string())
}

#[derive(Default)]
pub struct CreatePasswordState {}

#[async_trait::async_trait]
impl ClientState for CreatePasswordState {
    fn id(&self) -> StateId {
        StateId::CreatePassword
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::CreatedPassword { .. } => Some(StateId::VerifyPassword),
            Transition::FailPassword => Some(StateId::CreatePassword),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut ClientData, params: &'a mut Params) {
        data.pw_hash = None;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Password?|-|"])
            .await;
    }

    async fn decide<'a>(
        &mut self,
        _: &mut ClientData,
        params: &'a mut Params,
    ) -> Option<Transition> {
        if params.input.is_none() {
            params.send(vec![DEFAULT_PASSWORD_ERROR]).await;
            return Some(Transition::FailPassword);
        }

        let input = params.input.unwrap();
        if let Some(tx) = verify_len(input) {
            if let Transition::FailLogin { msg } = tx {
                params
                    .sender
                    .send(0, params.id, SendPrompt::None, vec![Cow::from(msg)])
                    .await;
            }
            return Some(Transition::FailPassword); // these will be error transitions.
        }
        // do something with input
        let hash = match hash_input(input) {
            Ok(hash) => hash,
            Err(_) => {
                params.send(vec![DEFAULT_PASSWORD_ERROR]).await;
                return Some(Transition::FailPassword);
            }
        };
        params
            .sender
            .send(
                0,
                params.id,
                SendPrompt::None,
                vec![Cow::from("|SteelBlue3|Password accepted.|-|")],
            )
            .await;
        return Some(Transition::CreatedPassword { hash });
    }
}

#[derive(Default)]
pub struct VerifyPasswordState {}

#[async_trait::async_trait]
impl ClientState for VerifyPasswordState {
    fn id(&self) -> StateId {
        StateId::VerifyPassword
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::FailPassword => Some(StateId::CreatePassword),
            Transition::VerifiedPassword => Some(StateId::CreateNewPlayer),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, _: &mut ClientData, params: &'a mut Params) {
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Verify?|-|"])
            .await;
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientData,
        params: &'a mut Params,
    ) -> Option<Transition> {
        if params.input.is_none() || data.pw_hash.is_none() {
            params.send(vec![DEFAULT_PASSWORD_ERROR]).await;
            return Some(Transition::FailPassword);
        }
        let input = params.input.unwrap();
        let hash = data.pw_hash.as_ref().unwrap();
        return match verify_password(hash.as_str(), input) {
            Ok(_) => {
                params.send(vec!["|SteelBlue3|Password verified.|-|"]).await;
                Some(Transition::VerifiedPassword)
            }
            Err(e) => {
                if let VerifyError::Unknown(e) = e {
                    tracing::error!("create verify password failure: {}", e);
                }
                params.send(vec![DEFAULT_PASSWORD_ERROR]).await;
                Some(Transition::FailPassword)
            }
        };
    }
}

#[derive(Default)]
pub struct LoginFailedState {}

#[async_trait::async_trait]
impl ClientState for LoginFailedState {
    fn id(&self) -> StateId {
        StateId::LoginFailed
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::Then => Some(StateId::LoginName), // this results in :magic:
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut ClientData, params: &'a mut Params) {
        if let Some(reason) = data.reason.as_ref() {
            params.send(vec![format!("|Red1|{}|-|", reason)]).await;
        }
        data.reason = None;
    }

    async fn decide<'a>(&mut self, _: &mut ClientData, _: &'a mut Params) -> Option<Transition> {
        Some(Transition::Then)
    }
}

#[derive(Default)]
pub struct SpawnPlayerState {}

#[async_trait::async_trait]
impl ClientState for SpawnPlayerState {
    fn id(&self) -> StateId {
        StateId::SpawnPlayer
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::PlayerLoaded { .. } => Some(StateId::InGame),
            Transition::FailLogin { .. } => Some(StateId::LoginFailed),
            _ => None,
        }
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientData,
        params: &'a mut Params,
    ) -> Option<Transition> {
        assert!(data.player.is_none());

        if let Some(name) = data.username.as_ref() {
            let player = match params
                .db
                .load_player(params.game_world.world_mut(), name.as_str())
                .await
            {
                Ok(player) => (player),
                Err(e) => {
                    tracing::error!("failed to load player: {}", e);
                    return Some(Transition::FailLogin {
                        msg: DEFAULT_LOGIN_ERROR.to_string(),
                    });
                }
            };

            params
                .sender
                .send(
                    0,
                    params.id,
                    SendPrompt::None,
                    vec![Cow::from("|white|Welcome to City Six."), Cow::from("")],
                )
                .await;
            params
                .game_world
                .player_action(Action::from(Login { actor: player }));
            params.game_world.player_action(Action::from(Look {
                actor: player,
                direction: None,
                // TODO: one-shot in the FSM to let the engine know the player is ready / in game?
            }));

            return Some(Transition::PlayerLoaded { player });
        }

        Some(Transition::FailLogin {
            msg: DEFAULT_LOGIN_ERROR.to_string(),
        })
    }
}

#[derive(Default)]
pub struct CreateNewPlayerState {}

#[async_trait::async_trait]
impl ClientState for CreateNewPlayerState {
    fn id(&self) -> StateId {
        StateId::CreateNewPlayer
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::PlayerCreated => Some(StateId::SpawnPlayer),
            Transition::FailLogin { .. } => Some(StateId::LoginFailed),
            _ => None,
        }
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientData,
        params: &'a mut Params,
    ) -> Option<Transition> {
        if let Some(name) = data.username.as_ref() {
            if let Some(hash) = data.pw_hash.as_ref() {
                let spawn_room = params.game_world.spawn_room();
                return match params
                    .db
                    .create_player(name.as_str(), hash.as_str(), spawn_room)
                    .await
                {
                    Ok(_) => Some(Transition::PlayerCreated),
                    Err(e) => {
                        tracing::error!("user creation error: {}", e);
                        Some(Transition::FailLogin {
                            msg: DEFAULT_LOGIN_ERROR.to_string(),
                        })
                    }
                };
            }
        }
        Some(Transition::FailLogin {
            msg: DEFAULT_LOGIN_ERROR.to_string(),
        })
    }

    async fn on_exit<'a>(&mut self, data: &mut ClientData, _: &'a mut Params) {
        data.pw_hash = None;
    }
}

#[derive(Default)]
pub struct InGameState {}

#[async_trait::async_trait]
impl ClientState for InGameState {
    fn id(&self) -> StateId {
        StateId::InGame
    }
    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            _ => None,
        }
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientData,
        params: &'a mut Params,
    ) -> Option<Transition> {
        if let Some(input) = params.input {
            tracing::debug!("{:?} sent {:?}", params.id, input);
            if let Some(player) = data.player {
                let immortal = params
                    .game_world
                    .world()
                    .get::<PlayerFlags>(player)
                    .unwrap()
                    .contains(player::Flags::IMMORTAL);

                match params.commands.parse(player, &input, !immortal) {
                    Ok(action) => params.game_world.player_action(action),
                    Err(message) => {
                        params
                            .sender
                            .send(0, params.id, SendPrompt::Prompt, vec![message.into()])
                            .await;
                    }
                }
            }
        }
        None
    }
}
