use crate::{
    engine::{
        client::ClientEvent,
        db::{AuthDb, GameDb},
        fsm::{
            hash_input, update_password::UpdatePasswordFsm, verify_len, verify_password, Fsm,
            FsmBuilder, FsmState, Params, State, TransitionAction, UpdateResult, VerifyError,
        },
        name_valid,
    },
    world::action::{observe::Look, system::Login, Action},
};

use bevy_ecs::prelude::Entity;
use std::{
    fmt::Debug,
};

static DEFAULT_LOGIN_ERROR: &str = "|Red1|Error retrieving user.|-|";
static DEFAULT_PASSWORD_ERROR: &str = "|Red1|Verification failed.|-|";

pub struct ClientLoginFsm {
    fsm: Fsm<Transition, StateId, ClientState>,
    data: ClientState,
}

impl ClientLoginFsm {
    pub async fn on_update(
        &mut self,
        event: ClientEvent<'_>,
        params: &mut Params<'_>,
    ) -> Option<UpdateResult> {
        if let Ok(event) = event.try_into() {
            Some(self.fsm.on_update(event, &mut self.data, params).await)
        } else {
            None
        }
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
            .with_state(Box::new(FailLoginState::default()))
            .with_state(Box::new(CreatePasswordState::default()))
            .with_state(Box::new(VerifyPasswordState::default()))
            .with_state(Box::new(FailPasswordState::default()))
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

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum Transition {
    Disconnect,
    Ready,
    BeginLogin,
    FailLogin,
    ExistsOffline(String),
    PlayerDoesNotExist(String),
    CreatedPassword(String),
    VerifiedPassword,
    FailPassword,
    BeginPassword,
    PlayerCreated,
    PlayerLoaded(Entity),
}

impl From<Transition> for TransitionAction<Transition> {
    fn from(tx: Transition) -> Self {
        TransitionAction::Transition(tx)
    }
}

impl<'a> TryFrom<ClientEvent<'a>> for Transition {
    type Error = ();

    fn try_from(value: ClientEvent<'a>) -> Result<Self, Self::Error> {
        let event = match value {
            ClientEvent::Disconnect => Transition::Disconnect,
            ClientEvent::PasswordHash(hash) => match hash {
                Some(hash) => Transition::CreatedPassword(hash),
                None => Transition::FailPassword,
            },
            ClientEvent::PasswordVerification(verified) => match verified {
                Some(true) => Transition::VerifiedPassword,
                None | Some(false) => Transition::FailPassword,
            },
            ClientEvent::Ready => Transition::Ready,
            _ => return Err(()),
        };

        Ok(event)
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, Ord, PartialOrd)]
pub enum StateId {
    NotConnected,
    ConnectionReady,
    LoginName,
    LoginPassword,
    FailLogin,
    CreatePassword,
    VerifyPassword,
    FailPassword,
    CreateNewPlayer,
    SpawnPlayer,
    InGame,
}

#[derive(Default)]
pub struct ClientState {
    pub username: Option<String>,
    pub pw_hash: Option<String>,
    pub player: Option<Entity>,
}

impl ClientState {
    pub fn player(&self) -> Option<Entity> {
        self.player
    }

    pub fn clear(&mut self) {
        *self = ClientState::default();
    }
}

impl FsmState<Transition> for ClientState {
    fn update(&mut self, tx: &Transition) {
        match tx {
            Transition::CreatedPassword(hash) => self.pw_hash = Some(hash.to_owned()),
            Transition::ExistsOffline(name) => self.username = Some(name.to_owned()),
            Transition::PlayerDoesNotExist(name) => self.username = Some(name.to_owned()),
            Transition::PlayerLoaded(player) => {
                tracing::info!("setting player");
                self.player = Some(*player)
            }
            _ => (),
        }
    }
}

// Not connected (default) state.
#[derive(Default)]
pub struct NotConnectedState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState> for NotConnectedState {
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

    async fn on_enter<'a>(&mut self, data: &mut ClientState, _params: &'a mut Params<'_>) {
        data.clear();
    }
}

#[derive(Default)]
pub struct ConnectionReady {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState> for ConnectionReady {
    fn id(&self) -> StateId {
        StateId::ConnectionReady
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::BeginLogin => Some(StateId::LoginName),
            Transition::Disconnect => Some(StateId::NotConnected),
            _ => None,
        }
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn on_enter<'a>(&mut self, _data: &mut ClientState, params: &'a mut Params<'_>) {
        params
            .send(vec![
                "|SteelBlue3|Connected to|-| |white|ucs://uplink.six.city|-|",
                "",
            ])
            .await;
    }

    async fn process<'a>(
        &mut self,
        _input: Option<&str>,
        _data: &mut ClientState,
        _params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        Some(Transition::BeginLogin.into())
    }
}

// in this state we introduce ourselves
#[derive(Default)]
pub struct LoginNameState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState> for LoginNameState {
    fn id(&self) -> StateId {
        StateId::LoginName
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::ExistsOffline { .. } => Some(StateId::LoginPassword),
            Transition::PlayerDoesNotExist { .. } => Some(StateId::CreatePassword),
            Transition::FailLogin => Some(StateId::LoginName),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut ClientState, params: &'a mut Params<'_>) {
        data.username = None;
        params.send_prompt(vec!["|SteelBlue3|Name?|-|"]).await;
    }

    // handle input
    async fn process<'a>(
        &mut self,
        input: Option<&str>,
        _data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let name = input?.trim();
        if !name_valid(name) {
            params.send(vec!["Invalid username."]).await;
            return Some(Transition::FailLogin.into());
        }

        let (has_user, user_online) = match params.db.has_player(name).await {
            Ok(has_user) => (has_user, params.game_world.player_online(name)),
            Err(e) => {
                tracing::error!("player presence check error: {}", e);
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                return Some(Transition::FailLogin.into());
            }
        };

        // was there a user and what is their connection status?
        if has_user {
            if user_online {
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                return Some(Transition::FailLogin.into());
            }

            // they were offline
            params.send(vec!["|SteelBlue3|User located.|-|"]).await;
            return Some(Transition::ExistsOffline(name.to_string()).into());
        } else if user_online {
            // how??
            tracing::error!("player is online but not found in db: {}", name);
            params.send(vec![DEFAULT_LOGIN_ERROR]).await;
            return Some(Transition::FailLogin.into());
        }

        // they didn't exist
        params.send(vec!["|SteelBlue3|New user detected.|-|"]).await;
        return Some(Transition::PlayerDoesNotExist(name.to_string()).into());
    }
}

#[derive(Default)]
pub struct LoginPasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState> for LoginPasswordState {
    fn id(&self) -> StateId {
        StateId::LoginPassword
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::VerifiedPassword => Some(StateId::SpawnPlayer),
            Transition::FailPassword => Some(StateId::FailLogin),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut ClientState, params: &'a mut Params<'_>) {
        data.pw_hash = None;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Password?|-|"])
            .await;
    }

    async fn process<'a>(
        &mut self,
        input: Option<&str>,
        data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = input?;

        if data.username.is_none() {
            params.send(vec![DEFAULT_LOGIN_ERROR]).await;
            return Some(Transition::Disconnect.into());
        }

        let name = data.username.as_deref().unwrap();

        let hash = match params.db.player_hash(name).await {
            Ok(Some(hash)) => hash,
            Ok(None) => {
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                return Some(Transition::FailLogin.into());
            }
            Err(e) => {
                tracing::error!("get user hash error: {:?}", e);
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                return Some(Transition::FailLogin.into());
            }
        };

        let input = input.to_string();
        let sender = params.engine_sender.clone();
        tokio::task::spawn_blocking(
            move || match verify_password(hash.as_str(), input.as_str()) {
                Ok(_) => sender.password_verification(Some(true)),
                Err(e) => match e {
                    VerifyError::Unknown(e) => {
                        tracing::error!("failed to verify password: {}", e);
                        sender.password_verification(None)
                    }
                    VerifyError::BadPassword => sender.password_verification(Some(false)),
                },
            },
        );

        None
    }
}

#[derive(Default)]
pub struct FailLoginState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState> for FailLoginState {
    fn id(&self) -> StateId {
        StateId::FailLogin
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::BeginLogin => Some(StateId::LoginName),
            Transition::Disconnect => Some(StateId::NotConnected),
            _ => None,
        }
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn on_enter<'a>(&mut self, _data: &mut ClientState, params: &'a mut Params<'_>) {
        params.send(vec![DEFAULT_LOGIN_ERROR]).await;
    }

    async fn process<'a>(
        &mut self,
        _input: Option<&str>,
        _data: &mut ClientState,
        _params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        Some(TransitionAction::Transition(Transition::BeginLogin))
    }
}

#[derive(Default)]
pub struct CreatePasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState> for CreatePasswordState {
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

    async fn on_enter<'a>(&mut self, data: &mut ClientState, params: &'a mut Params<'_>) {
        data.pw_hash = None;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Password?|-|"])
            .await;
    }

    async fn process<'a>(
        &mut self,
        input: Option<&str>,
        _data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = input?;

        if let Some(msg) = verify_len(input) {
            params.send(vec![msg]).await;
            return Some(Transition::FailPassword.into()); // these will be error transitions.
        }

        let password = input.to_owned();
        let sender = params.engine_sender.clone();

        tokio::task::spawn_blocking(move || match hash_input(password.as_str()) {
            Ok(hash) => sender.password_hash(Some(hash)),
            Err(e) => {
                tracing::error!("failed to hash password: {}", e);
                sender.password_hash(None)
            }
        });

        None
    }
}

#[derive(Default)]
pub struct VerifyPasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState> for VerifyPasswordState {
    fn id(&self) -> StateId {
        StateId::VerifyPassword
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::FailPassword => Some(StateId::FailPassword),
            Transition::VerifiedPassword => Some(StateId::CreateNewPlayer),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, _: &mut ClientState, params: &'a mut Params<'_>) {
        params.send(vec!["|SteelBlue3|Password accepted.|-|"]).await;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Verify?|-|"])
            .await;
    }

    async fn process<'a>(
        &mut self,
        input: Option<&str>,
        data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = input?;

        if data.pw_hash.is_none() {
            params.send(vec![DEFAULT_PASSWORD_ERROR]).await;
            return Some(Transition::FailPassword.into());
        }

        let hash = data.pw_hash.as_ref().unwrap().to_owned();
        let password = input.to_owned();
        let sender = params.engine_sender.clone();

        tokio::task::spawn_blocking(move || {
            match verify_password(hash.as_str(), password.as_str()) {
                Ok(_) => sender.password_verification(Some(true)),
                Err(e) => match e {
                    VerifyError::Unknown(e) => {
                        tracing::error!("failed to verify password: {}", e);
                        sender.password_verification(None)
                    }
                    VerifyError::BadPassword => sender.password_verification(Some(false)),
                },
            }
        });

        None
    }
}

#[derive(Default)]
pub struct FailPasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState> for FailPasswordState {
    fn id(&self) -> StateId {
        StateId::FailPassword
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::BeginPassword => Some(StateId::CreatePassword),
            Transition::Disconnect => Some(StateId::NotConnected),
            _ => None,
        }
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn on_enter<'a>(&mut self, _data: &mut ClientState, params: &'a mut Params<'_>) {
        params.send(vec![DEFAULT_PASSWORD_ERROR]).await;
    }

    async fn process<'a>(
        &mut self,
        _input: Option<&str>,
        _data: &mut ClientState,
        _params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        Some(TransitionAction::Transition(Transition::BeginPassword))
    }
}

#[derive(Default)]
pub struct SpawnPlayerState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState> for SpawnPlayerState {
    fn id(&self) -> StateId {
        StateId::SpawnPlayer
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::PlayerLoaded { .. } => Some(StateId::InGame),
            Transition::FailLogin => Some(StateId::LoginName),
            _ => None,
        }
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn on_enter<'a>(&mut self, data: &mut ClientState, params: &'a mut Params<'_>) {
        params
            .send(vec!["|SteelBlue3|Password verified.|-|", ""])
            .await;
        data.player = None
    }

    async fn process<'a>(
        &mut self,
        _input: Option<&str>,
        data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        if data.username.is_none() {
            params.send(vec![DEFAULT_LOGIN_ERROR]).await;
            return Some(Transition::FailLogin.into());
        }

        let name = data.username.as_ref().unwrap();

        let player = match params
            .db
            .load_player(params.game_world.world_mut(), name.as_str())
            .await
        {
            Ok(player) => (player),
            Err(e) => {
                tracing::error!("failed to load player: {}", e);
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                return Some(Transition::FailLogin.into());
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

        Some(Transition::PlayerLoaded(player).into())
    }
}

#[derive(Default)]
pub struct CreateNewPlayerState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState> for CreateNewPlayerState {
    fn id(&self) -> StateId {
        StateId::CreateNewPlayer
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            Transition::PlayerCreated => Some(StateId::SpawnPlayer),
            Transition::FailLogin => Some(StateId::LoginName),
            _ => None,
        }
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn on_enter<'a>(&mut self, _data: &mut ClientState, params: &'a mut Params<'_>) {
        params.send(vec!["|SteelBlue3|Password verified.|-|"]).await;
    }

    async fn process<'a>(
        &mut self,
        _input: Option<&str>,
        data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        if data.username.is_none() || data.pw_hash.is_none() {
            params.send(vec![DEFAULT_LOGIN_ERROR]).await;
            return Some(Transition::FailLogin.into());
        }

        let name = data.username.as_ref().unwrap();
        let hash = data.pw_hash.as_ref().unwrap();
        let spawn_room = params.game_world.spawn_room();

        return match params
            .db
            .create_player(name.as_str(), hash.as_str(), spawn_room)
            .await
        {
            Ok(_) => Some(Transition::PlayerCreated.into()),
            Err(e) => {
                tracing::error!("user creation error: {}", e);
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                Some(Transition::FailLogin.into())
            }
        };
    }

    async fn on_exit<'a>(&mut self, data: &mut ClientState, _: &'a mut Params<'_>) {
        data.pw_hash = None;
    }
}

#[derive(Default)]
pub struct InGameState {}

#[async_trait::async_trait]
impl State<Transition, StateId, ClientState> for InGameState {
    fn id(&self) -> StateId {
        StateId::InGame
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Disconnect => Some(StateId::NotConnected),
            _ => None,
        }
    }

    async fn process<'a>(
        &mut self,
        input: Option<&str>,
        data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = input?;

        if data.player.is_none() {
            params.send(vec!["|Red1|Disconnected.|-|"]).await;
            return Some(Transition::Disconnect.into());
        }

        if input == "password" {
            return Some(TransitionAction::PushFsm(Box::new(UpdatePasswordFsm::new(
                data.username.as_ref().unwrap().clone(),
            ))));
        }

        let player = data.player.unwrap();

        if let Err(message) = params.game_world.player_input(player, input) {
            params.send_prompt(vec![message]).await;
        }

        None
    }
}
