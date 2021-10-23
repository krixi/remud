use crate::engine::{
    client::ClientEvent,
    db::AuthDb,
    fsm::{
        hash_input, verify_len, verify_password, Fsm, FsmBuilder, FsmState, Params, StackFsm,
        State, TransitionAction, UpdateResult, VerifyError,
    },
};

static UPDATE_PASSWORD_ERROR: &str = "|Red1|Failed to update password.|-|";

pub struct UpdatePasswordFsm {
    fsm: Fsm<Transition, StateId, UpdatePasswordData>,
    data: UpdatePasswordData,
}

impl UpdatePasswordFsm {
    pub fn new(username: String) -> Self {
        let fsm = FsmBuilder::new()
            .with_state(Box::new(ChangePasswordState::default()))
            .with_state(Box::new(VerifyPasswordState::default()))
            .with_state(Box::new(EnterPasswordState::default()))
            .with_state(Box::new(ConfirmPasswordState::default()))
            .with_state(Box::new(UpdatePasswordState::default()))
            .with_state(Box::new(FailPasswordState::default()))
            .build()
            .unwrap();

        Self {
            fsm,
            data: UpdatePasswordData {
                username,
                pw_hash: None,
            },
        }
    }
}

#[async_trait::async_trait]
impl StackFsm for UpdatePasswordFsm {
    async fn on_update(
        &mut self,
        event: ClientEvent<'_>,
        params: &mut Params,
    ) -> Option<UpdateResult> {
        if let Ok(event) = event.try_into() {
            Some(self.fsm.on_update(event, &mut self.data, params).await)
        } else {
            None
        }
    }
}

impl<'a> TryFrom<ClientEvent<'a>> for Transition {
    type Error = ();

    fn try_from(value: ClientEvent<'a>) -> Result<Self, Self::Error> {
        let event = match value {
            ClientEvent::PasswordHash(hash) => match hash {
                Some(hash) => Transition::EnteredPassword(hash),
                None => Transition::HashFailed,
            },
            ClientEvent::PasswordVerification(verified) => match verified {
                Some(true) => Transition::VerifiedPassword,
                None | Some(false) => Transition::VerificationFailed,
            },
            _ => return Err(()),
        };

        Ok(event)
    }
}

#[derive(Debug, Clone)]
pub enum Transition {
    Ready,
    VerifiedPassword,
    VerificationFailed,
    EnteredPassword(String),
    HashFailed,
}

impl From<Transition> for TransitionAction<Transition> {
    fn from(tx: Transition) -> Self {
        TransitionAction::Transition(tx)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum StateId {
    ChangePassword,
    VerifyPassword,
    EnterPassword,
    ConfirmPassword,
    UpdatePassword,
    FailPassword,
}

pub struct UpdatePasswordData {
    pub username: String,
    pub pw_hash: Option<String>,
}

impl FsmState<Transition> for UpdatePasswordData {
    fn update(&mut self, tx: &Transition) {
        match tx {
            Transition::EnteredPassword(hash) => self.pw_hash = Some(hash.to_owned()),
            _ => (),
        }
    }
}

#[derive(Default)]
pub struct ChangePasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, UpdatePasswordData> for ChangePasswordState {
    fn id(&self) -> StateId {
        StateId::ChangePassword
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::Ready => Some(StateId::VerifyPassword),
            _ => None,
        }
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn process<'a>(
        &mut self,
        _input: Option<&str>,
        _data: &mut UpdatePasswordData,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        params.send(vec!["", "|White|Password Update|-|", ""]).await;
        Some(Transition::Ready.into())
    }
}

#[derive(Default)]
pub struct VerifyPasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, UpdatePasswordData> for VerifyPasswordState {
    fn id(&self) -> StateId {
        StateId::VerifyPassword
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::VerifiedPassword => Some(StateId::EnterPassword),
            Transition::VerificationFailed => Some(StateId::FailPassword),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut UpdatePasswordData, params: &'a mut Params<'_>) {
        data.pw_hash = None;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Current password?|-|"])
            .await;
    }

    async fn process<'a>(
        &mut self,
        input: Option<&str>,
        data: &mut UpdatePasswordData,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = input?.to_string();

        let name = data.username.as_str();
        let hash = match params.db.player_hash(name).await {
            Ok(Some(hash)) => hash,
            Ok(None) => {
                params.send_prompt(vec![UPDATE_PASSWORD_ERROR]).await;
                return Some(TransitionAction::PopFsm);
            }
            Err(e) => {
                tracing::error!("get user hash error: {:?}", e);
                params.send_prompt(vec![UPDATE_PASSWORD_ERROR]).await;
                return Some(TransitionAction::PopFsm);
            }
        };

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
pub struct EnterPasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, UpdatePasswordData> for EnterPasswordState {
    fn id(&self) -> StateId {
        StateId::EnterPassword
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::EnteredPassword { .. } => Some(StateId::ConfirmPassword),
            Transition::HashFailed => Some(StateId::FailPassword),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut UpdatePasswordData, params: &'a mut Params<'_>) {
        data.pw_hash = None;
        params.send(vec!["|SteelBlue3|Password verified.|-|"]).await;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|New password?|-|"])
            .await;
    }

    async fn process<'a>(
        &mut self,
        input: Option<&str>,
        _data: &mut UpdatePasswordData,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = input?.to_owned();

        if let Some(msg) = verify_len(input.as_str()) {
            params.send_prompt(vec![msg]).await;
            return Some(TransitionAction::PopFsm);
        }

        let sender = params.engine_sender.clone();

        tokio::task::spawn_blocking(move || match hash_input(input.as_str()) {
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
pub struct ConfirmPasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, UpdatePasswordData> for ConfirmPasswordState {
    fn id(&self) -> StateId {
        StateId::ConfirmPassword
    }

    fn output_state(&self, next: &Transition) -> Option<StateId> {
        match next {
            Transition::VerifiedPassword => Some(StateId::UpdatePassword),
            Transition::VerificationFailed => Some(StateId::FailPassword),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, _data: &mut UpdatePasswordData, params: &'a mut Params<'_>) {
        params.send(vec!["|SteelBlue3|Password accepted.|-|"]).await;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Confirm?|-|"])
            .await;
    }

    async fn process<'a>(
        &mut self,
        input: Option<&str>,
        data: &mut UpdatePasswordData,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = input?.to_owned();

        if data.pw_hash.is_none() {
            params.send(vec![UPDATE_PASSWORD_ERROR]).await;
            return Some(TransitionAction::PopFsm);
        }

        let hash = data.pw_hash.as_ref().unwrap().to_owned();
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
pub struct UpdatePasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, UpdatePasswordData> for UpdatePasswordState {
    fn id(&self) -> StateId {
        StateId::UpdatePassword
    }

    fn output_state(&self, _next: &Transition) -> Option<StateId> {
        None
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn process<'a>(
        &mut self,
        _input: Option<&str>,
        data: &mut UpdatePasswordData,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        if data.pw_hash.is_none() {
            params.send(vec![UPDATE_PASSWORD_ERROR]).await;
            return Some(TransitionAction::PopFsm);
        }

        let hash = data.pw_hash.as_ref().unwrap().as_str();

        match params
            .db
            .update_password(data.username.as_str(), hash)
            .await
        {
            Ok(_) => {
                params
                    .send_prompt(vec!["|SteelBlue3|Password updated.|-|"])
                    .await;
            }
            Err(e) => {
                tracing::error!("failed to update password: {}", e);
                params.send_prompt(vec![UPDATE_PASSWORD_ERROR]).await;
            }
        }

        Some(TransitionAction::PopFsm)
    }
}

#[derive(Default)]
pub struct FailPasswordState {}

#[async_trait::async_trait]
impl State<Transition, StateId, UpdatePasswordData> for FailPasswordState {
    fn id(&self) -> StateId {
        StateId::FailPassword
    }

    fn output_state(&self, _next: &Transition) -> Option<StateId> {
        None
    }

    fn keep_going(&self) -> bool {
        true
    }

    async fn on_enter<'a>(&mut self, _data: &mut UpdatePasswordData, params: &'a mut Params<'_>) {
        params.send_prompt(vec![UPDATE_PASSWORD_ERROR]).await;
    }

    async fn process<'a>(
        &mut self,
        _input: Option<&str>,
        _data: &mut UpdatePasswordData,
        _params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        Some(TransitionAction::PopFsm)
    }
}
