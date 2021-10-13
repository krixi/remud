use crate::{
    engine::{
        client::{ClientSender, SendPrompt},
        db::{verify_password, AuthDb, Db, GameDb, VerifyError},
        fsm::{self, Fsm, FsmBuilder, FsmState, Params, ParamsInfo, State},
        name_valid,
    },
    world::{
        action::{commands::Commands, observe::Look, system::Login, Action},
        types::player::{self, PlayerFlags},
        GameWorld,
    },
    ClientId,
};

use anyhow::bail;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use bevy_ecs::prelude::Entity;
use rand::rngs::OsRng;
use std::{
    borrow::Cow,
    fmt::{Debug, Formatter},
};

static DEFAULT_LOGIN_ERROR: &str = "|Red1|Error retrieving user.|-|";
static DEFAULT_PASSWORD_ERROR: &str = "|Red1|Verification failed.|-|";

pub struct ClientLoginFsm {
    fsm: Fsm<Transition, StateId, ClientState, ClientParamsInfo>,
    data: ClientState,
}

impl ClientLoginFsm {
    pub async fn on_update(
        &mut self,
        tx: Option<Transition>,
        params: &mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) -> bool {
        self.fsm.on_update(tx, &mut self.data, params).await
    }

    pub fn player(&self) -> Option<Entity> {
        self.data.player()
    }
}

// this state machine will always have the same shape, and there's only one of them,
// so implement default to construct it.
impl Default for ClientLoginFsm {
    fn default() -> Self {
        let fsm = FsmBuilder::new()
            .with_state(Box::new(NotConnectedState::default()))
            .with_state(Box::new(ConnectionReady::default()))
            .with_state(Box::new(LoginNameState::default()))
            .with_state(Box::new(LoginPasswordState::default()))
            .with_state(Box::new(CreatePasswordState::default()))
            .with_state(Box::new(VerifyPasswordState::default()))
            .with_state(Box::new(SpawnPlayerState::default()))
            .with_state(Box::new(CreateNewPlayerState::default()))
            .with_state(Box::new(InGameState::default()))
            .build();

        ClientLoginFsm {
            fsm: fsm.unwrap(),
            data: ClientState::default(),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, Ord, PartialOrd)]
pub enum StateId {
    NotConnected,
    ConnectionReady,
    LoginName,
    LoginPassword,
    CreatePassword,
    VerifyPassword,
    CreateNewPlayer,
    SpawnPlayer,
    InGame,
}

impl fsm::StateId for StateId {}

#[derive(Hash, Eq, PartialEq, Clone)]
pub enum Transition {
    Disconnect,
    Ready,
    Then,
    FailLogin,
    ExistsOffline,
    PlayerDoesNotExist,
    CreatedPassword,
    FailPassword,
    VerifiedPassword,
    PlayerCreated,
    PlayerLoaded,
}

impl fsm::Transition for Transition {}

impl Debug for Transition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Transition::Disconnect => write!(f, "Disconnect"),
            Transition::Ready => write!(f, "Ready"),
            Transition::Then => write!(f, "Then"),
            Transition::FailLogin => write!(f, "FailLogin"),
            Transition::ExistsOffline => write!(f, "ExistsOffline"),
            Transition::PlayerDoesNotExist => write!(f, "PlayerDoesNotExist"),
            Transition::CreatedPassword => write!(f, "CreatedPassword"),
            Transition::VerifiedPassword => write!(f, "VerifiedPassword"),
            Transition::FailPassword => write!(f, "FailPassword"),
            Transition::PlayerCreated => write!(f, "PlayerCreated"),
            Transition::PlayerLoaded => write!(f, "PlayerLoaded"),
        }
    }
}

pub struct ClientParamsInfo {}

impl ParamsInfo for ClientParamsInfo {
    type Params<'p> = ClientParams<'p>;
}

impl<'p> Params<'p> for ClientParams<'p> {}

pub struct ClientParams<'p> {
    pub id: ClientId,
    pub input: Option<&'p str>,
    pub sender: &'p ClientSender,
    pub game_world: &'p mut GameWorld,
    pub db: &'p Db,
    pub commands: &'p Commands,
}

impl<'p> ClientParams<'p> {
    pub fn new(
        id: ClientId,
        sender: &'p ClientSender,
        world: &'p mut GameWorld,
        db: &'p Db,
        commands: &'p Commands,
    ) -> Self {
        ClientParams {
            id,
            input: None,
            sender,
            game_world: world,
            db,
            commands,
        }
    }

    pub fn with_input(&mut self, input: Option<&'p str>) -> &mut Self {
        self.input = input;
        self
    }

    pub async fn send<M: Into<Cow<'p, str>>>(&self, messages: impl IntoIterator<Item = M>) {
        self.sender
            .send(
                self.id,
                SendPrompt::None,
                messages.into_iter().map(Into::into),
            )
            .await;
    }

    pub async fn send_prompt<M: Into<Cow<'p, str>>>(&self, messages: impl IntoIterator<Item = M>) {
        self.sender
            .send(
                self.id,
                SendPrompt::Prompt,
                messages.into_iter().map(Into::into),
            )
            .await;
    }

    pub async fn send_sensitive_prompt<M: Into<Cow<'p, str>>>(
        &self,
        messages: impl IntoIterator<Item = M>,
    ) {
        self.sender
            .send(
                self.id,
                SendPrompt::Sensitive,
                messages.into_iter().map(Into::into),
            )
            .await;
    }
}

#[derive(Default)]
pub struct ClientState {
    pub username: Option<String>,
    pub pw_hash: Option<String>,
    pub player: Option<Entity>,
    pub reason: Option<String>,
}

impl FsmState for ClientState {}

impl ClientState {
    pub fn player(&self) -> Option<Entity> {
        self.player
    }

    pub fn clear(&mut self) {
        *self = ClientState::default();
    }
}

// Not connected (default) state.
#[derive(Default)]
pub struct NotConnectedState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState, ClientParamsInfo> for NotConnectedState {
    fn id(&self) -> StateId {
        StateId::NotConnected
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Ready => Some(StateId::ConnectionReady),
            Transition::Disconnect => Some(StateId::NotConnected),
            _ => None,
        }
    }

    async fn on_enter<'a>(
        &mut self,
        data: &mut ClientState,
        _params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) {
        data.clear();
    }
}

#[derive(Default)]
pub struct ConnectionReady {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState, ClientParamsInfo> for ConnectionReady {
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
    async fn on_enter<'a>(
        &mut self,
        _data: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) {
        params
            .send(vec![
                "|SteelBlue3|Connected to|-| |white|ucs://uplink.six.city|-|",
                "",
            ])
            .await;
    }
    async fn decide<'a>(
        &mut self,
        _data: &mut ClientState,
        _params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) -> Option<Transition> {
        Some(Transition::Then)
    }
}

// in this state we introduce ourselves
#[derive(Default)]
pub struct LoginNameState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState, ClientParamsInfo> for LoginNameState {
    fn id(&self) -> StateId {
        StateId::LoginName
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::ExistsOffline => Some(StateId::LoginPassword),
            Transition::PlayerDoesNotExist => Some(StateId::CreatePassword),
            Transition::FailLogin { .. } => Some(StateId::LoginName),
            _ => None,
        }
    }

    async fn on_enter<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) {
        data.username = None;
        params.send_prompt(vec!["|SteelBlue3|Name?|-|"]).await;
    }

    // handle input
    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) -> Option<Transition> {
        let name = if let Some(input) = params.input {
            let name = input.trim();
            if !name_valid(name) {
                params.send(vec!["Invalid username."]).await;
                return Some(Transition::FailLogin);
            }
            name
        } else {
            return None; // shouldn't happen? :shrug:
        };

        let (has_user, user_online) = match params.db.has_player(name).await {
            Ok(has_user) => (has_user, params.game_world.player_online(name)),
            Err(e) => {
                tracing::error!("player presence check error: {}", e);
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                return Some(Transition::FailLogin);
            }
        };

        data.username = Some(name.to_string());

        // was there a user and what is their connection status?
        if has_user {
            if user_online {
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                return Some(Transition::FailLogin);
            }

            // they were offline
            params.send(vec!["|SteelBlue3|User located.|-|"]).await;
            return Some(Transition::ExistsOffline);
        } else if user_online {
            // how??
            tracing::error!("player is online but not found in db: {}", name);
            params.send(vec![DEFAULT_LOGIN_ERROR]).await;
            return Some(Transition::FailLogin);
        }

        // they didn't exist
        params.send(vec!["|SteelBlue3|New user detected.|-|"]).await;
        return Some(Transition::PlayerDoesNotExist);
    }
}

#[derive(Default)]
pub struct LoginPasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState, ClientParamsInfo> for LoginPasswordState {
    fn id(&self) -> StateId {
        StateId::LoginPassword
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::VerifiedPassword => Some(StateId::SpawnPlayer),
            Transition::FailPassword => Some(StateId::LoginName),
            _ => None,
        }
    }

    async fn on_enter<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) {
        data.pw_hash = None;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Password?|-|"])
            .await;
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) -> Option<Transition> {
        if params.input.is_none() {
            return None;
        } else if data.username.is_none() {
            params.send(vec![DEFAULT_LOGIN_ERROR]).await;
            return Some(Transition::Disconnect);
        }

        let input = params.input.unwrap();
        let name = data.username.as_deref().unwrap();

        // these will be error transitions.
        if let Some(msg) = verify_len(input) {
            params.send(vec![msg]).await;
            return Some(Transition::FailPassword);
        }

        match params.db.verify_player(name, input).await {
            Ok(verified) => {
                if verified {
                    params
                        .send(vec!["|SteelBlue3|Password verified.|-|", ""])
                        .await;
                    Some(Transition::VerifiedPassword)
                } else {
                    tracing::info!("verification failed for user {}", name);
                    params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                    Some(Transition::FailLogin)
                }
            }
            Err(e) => {
                tracing::error!("get user hash error: {:?}", e);
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                Some(Transition::FailLogin)
            }
        }
    }
}

#[derive(Default)]
pub struct CreatePasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState, ClientParamsInfo> for CreatePasswordState {
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

    async fn on_enter<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) {
        data.pw_hash = None;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Password?|-|"])
            .await;
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) -> Option<Transition> {
        let input = params.input?;

        if let Some(msg) = verify_len(input) {
            params.send(vec![msg]).await;
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
        params.send(vec!["|SteelBlue3|Password accepted.|-|"]).await;

        data.pw_hash = Some(hash);
        return Some(Transition::CreatedPassword);
    }
}

#[derive(Default)]
pub struct VerifyPasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState, ClientParamsInfo> for VerifyPasswordState {
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

    async fn on_enter<'a>(
        &mut self,
        _: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) {
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Verify?|-|"])
            .await;
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) -> Option<Transition> {
        if params.input.is_none() {
            return None;
        } else if data.pw_hash.is_none() {
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
pub struct SpawnPlayerState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState, ClientParamsInfo> for SpawnPlayerState {
    fn id(&self) -> StateId {
        StateId::SpawnPlayer
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::PlayerLoaded { .. } => Some(StateId::InGame),
            Transition::FailLogin { .. } => Some(StateId::LoginName),
            _ => None,
        }
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn on_enter<'a>(
        &mut self,
        data: &mut ClientState,
        _params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) {
        data.player = None
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) -> Option<Transition> {
        if let Some(name) = data.username.as_ref() {
            let player = match params
                .db
                .load_player(params.game_world.world_mut(), name.as_str())
                .await
            {
                Ok(player) => (player),
                Err(e) => {
                    tracing::error!("failed to load player: {}", e);
                    params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                    return Some(Transition::FailLogin);
                }
            };

            params.send(vec!["|white|Welcome to City Six.", ""]).await;
            params
                .game_world
                .player_action(Action::from(Login { actor: player }));
            params.game_world.player_action(Action::from(Look {
                actor: player,
                direction: None,
                // TODO: one-shot in the FSM to let the engine know the player is ready / in game?
            }));

            data.player = Some(player);
            return Some(Transition::PlayerLoaded);
        }

        params.send(vec![DEFAULT_LOGIN_ERROR]).await;
        Some(Transition::FailLogin)
    }
}

#[derive(Default)]
pub struct CreateNewPlayerState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState, ClientParamsInfo> for CreateNewPlayerState {
    fn id(&self) -> StateId {
        StateId::CreateNewPlayer
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::PlayerCreated => Some(StateId::SpawnPlayer),
            Transition::FailLogin { .. } => Some(StateId::LoginName),
            _ => None,
        }
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) -> Option<Transition> {
        if data.username.is_none() || data.pw_hash.is_none() {
            params.send(vec![DEFAULT_LOGIN_ERROR]).await;
            return Some(Transition::FailLogin);
        }

        let name = data.username.as_ref().unwrap();
        let hash = data.pw_hash.as_ref().unwrap();
        let spawn_room = params.game_world.spawn_room();

        return match params
            .db
            .create_player(name.as_str(), hash.as_str(), spawn_room)
            .await
        {
            Ok(_) => Some(Transition::PlayerCreated),
            Err(e) => {
                tracing::error!("user creation error: {}", e);
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                Some(Transition::FailLogin)
            }
        };
    }

    async fn on_exit<'a>(
        &mut self,
        data: &mut ClientState,
        _: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) {
        data.pw_hash = None;
    }
}

#[derive(Default)]
pub struct InGameState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState, ClientParamsInfo> for InGameState {
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
        data: &mut ClientState,
        params: &'a mut <ClientParamsInfo as ParamsInfo>::Params<'_>,
    ) -> Option<Transition> {
        let input = params.input?;

        if data.player.is_none() {
            params.send(vec!["|Red1|Disconnected.|-|"]).await;
            return Some(Transition::Disconnect);
        }

        let player = data.player.unwrap();

        tracing::debug!("{:?} sent {:?}", params.id, input);
        let immortal = params
            .game_world
            .world()
            .get::<PlayerFlags>(player)
            .unwrap()
            .contains(player::Flags::IMMORTAL);

        match params.commands.parse(player, input, !immortal) {
            Ok(action) => params.game_world.player_action(action),
            Err(message) => {
                params.send_prompt(vec![message]).await;
            }
        }

        None
    }
}

fn verify_len(input: &str) -> Option<String> {
    if input.len() < 5 {
        return Some("|Red1|Weak password detected.|-|".to_string());
    } else if input.len() > 1024 {
        return Some("|Red1|Password too strong :(|-|".to_string());
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
    Ok(hash)
}
