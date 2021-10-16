use crate::{
    engine::{
        db::{verify_password, AuthDb, GameDb, VerifyError},
        fsm::{
            hash_input, update_password::UpdatePasswordFsm, verify_len, Fsm, FsmBuilder, Params,
            State, TransitionAction, UpdateResult,
        },
        name_valid,
    },
    world::action::{observe::Look, system::Login, Action},
};

use bevy_ecs::prelude::Entity;
use std::fmt::Debug;

static DEFAULT_LOGIN_ERROR: &str = "|Red1|Error retrieving user.|-|";
static DEFAULT_PASSWORD_ERROR: &str = "|Red1|Verification failed.|-|";

pub struct ClientLoginFsm {
    fsm: Fsm<Transition, StateId, ClientState>,
    data: ClientState,
}

impl ClientLoginFsm {
    pub async fn on_update(
        &mut self,
        tx: Option<Transition>,
        params: &mut Params<'_>,
    ) -> UpdateResult {
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

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
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

impl From<Transition> for TransitionAction<Transition> {
    fn from(tx: Transition) -> Self {
        TransitionAction::Transition(tx)
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

#[derive(Default)]
pub struct ClientState {
    pub username: Option<String>,
    pub pw_hash: Option<String>,
    pub player: Option<Entity>,
    pub reason: Option<String>,
}

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
            Transition::Then => Some(StateId::LoginName),
            Transition::Disconnect => Some(StateId::NotConnected),
            _ => None,
        }
    }
    async fn on_enter<'a>(&mut self, _data: &mut ClientState, params: &'a mut Params<'_>) {
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
        _params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        Some(Transition::Then.into())
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
            Transition::ExistsOffline => Some(StateId::LoginPassword),
            Transition::PlayerDoesNotExist => Some(StateId::CreatePassword),
            Transition::FailLogin => Some(StateId::LoginName),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut ClientState, params: &'a mut Params<'_>) {
        data.username = None;
        params.send_prompt(vec!["|SteelBlue3|Name?|-|"]).await;
    }

    // handle input
    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let name = if let Some(input) = params.input {
            let name = input.trim();
            if !name_valid(name) {
                params.send(vec!["Invalid username."]).await;
                return Some(Transition::FailLogin.into());
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
                return Some(Transition::FailLogin.into());
            }
        };

        data.username = Some(name.to_string());

        // was there a user and what is their connection status?
        if has_user {
            if user_online {
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                return Some(Transition::FailLogin.into());
            }

            // they were offline
            params.send(vec!["|SteelBlue3|User located.|-|"]).await;
            return Some(Transition::ExistsOffline.into());
        } else if user_online {
            // how??
            tracing::error!("player is online but not found in db: {}", name);
            params.send(vec![DEFAULT_LOGIN_ERROR]).await;
            return Some(Transition::FailLogin.into());
        }

        // they didn't exist
        params.send(vec!["|SteelBlue3|New user detected.|-|"]).await;
        return Some(Transition::PlayerDoesNotExist.into());
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
            Transition::FailPassword => Some(StateId::LoginName),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut ClientState, params: &'a mut Params<'_>) {
        data.pw_hash = None;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Password?|-|"])
            .await;
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = params.input?;

        if data.username.is_none() {
            params.send(vec![DEFAULT_LOGIN_ERROR]).await;
            return Some(Transition::Disconnect.into());
        }

        let name = data.username.as_deref().unwrap();

        match params.db.verify_player(name, input).await {
            Ok(verified) => {
                if verified {
                    params
                        .send(vec!["|SteelBlue3|Password verified.|-|", ""])
                        .await;
                    Some(Transition::VerifiedPassword.into())
                } else {
                    tracing::info!("verification failed for user {}", name);
                    params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                    Some(Transition::FailLogin.into())
                }
            }
            Err(e) => {
                tracing::error!("get user hash error: {:?}", e);
                params.send(vec![DEFAULT_LOGIN_ERROR]).await;
                Some(Transition::FailLogin.into())
            }
        }
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
            Transition::CreatedPassword => Some(StateId::VerifyPassword),
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

    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = params.input?;

        if let Some(msg) = verify_len(input) {
            params.send(vec![msg]).await;
            return Some(Transition::FailPassword.into()); // these will be error transitions.
        }
        // do something with input
        let hash = match hash_input(input) {
            Ok(hash) => hash,
            Err(_) => {
                params.send(vec![DEFAULT_PASSWORD_ERROR]).await;
                return Some(Transition::FailPassword.into());
            }
        };
        params.send(vec!["|SteelBlue3|Password accepted.|-|"]).await;

        data.pw_hash = Some(hash);
        return Some(Transition::CreatedPassword.into());
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
            Transition::FailPassword => Some(StateId::CreatePassword),
            Transition::VerifiedPassword => Some(StateId::CreateNewPlayer),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, _: &mut ClientState, params: &'a mut Params<'_>) {
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Verify?|-|"])
            .await;
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        if params.input.is_none() {
            return None;
        } else if data.pw_hash.is_none() {
            params.send(vec![DEFAULT_PASSWORD_ERROR]).await;
            return Some(Transition::FailPassword.into());
        }

        let input = params.input.unwrap();
        let hash = data.pw_hash.as_ref().unwrap();
        match verify_password(hash.as_str(), input) {
            Ok(_) => {
                params.send(vec!["|SteelBlue3|Password verified.|-|"]).await;
                Some(Transition::VerifiedPassword.into())
            }
            Err(e) => {
                if let VerifyError::Unknown(e) = e {
                    tracing::error!("create verify password failure: {}", e);
                }
                params.send(vec![DEFAULT_PASSWORD_ERROR]).await;
                Some(Transition::FailPassword.into())
            }
        }
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
            Transition::PlayerLoaded => Some(StateId::InGame),
            Transition::FailLogin => Some(StateId::LoginName),
            _ => None,
        }
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn on_enter<'a>(&mut self, data: &mut ClientState, _params: &'a mut Params<'_>) {
        data.player = None
    }

    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
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

            data.player = Some(player);
            return Some(Transition::PlayerLoaded.into());
        }

        params.send(vec![DEFAULT_LOGIN_ERROR]).await;
        Some(Transition::FailLogin.into())
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

    async fn decide<'a>(
        &mut self,
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

    async fn decide<'a>(
        &mut self,
        data: &mut ClientState,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = params.input?;

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

        tracing::debug!("{:?} sent {:?}", params.id, input);
        if let Err(message) = params.game_world.player_input(player, input) {
            params.send_prompt(vec![message]).await;
        }

        None
    }
}
