use crate::engine::{
    db::{verify_password, AuthDb, VerifyError},
    fsm::{
        hash_input, verify_len, Fsm, FsmBuilder, Params, StackFsm, State, TransitionAction,
        UpdateResult,
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
    async fn on_update(&mut self, params: &mut Params) -> UpdateResult {
        self.fsm.on_update(None, &mut self.data, params).await
    }
}

#[derive(Debug)]
pub enum Transition {
    Ready,
    VerifiedPassword,
    EnteredPassword,
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
}

pub struct UpdatePasswordData {
    pub username: String,
    pub pw_hash: Option<String>,
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

    async fn decide<'a>(
        &mut self,
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
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut UpdatePasswordData, params: &'a mut Params<'_>) {
        data.pw_hash = None;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Current password?|-|"])
            .await;
    }

    async fn decide<'a>(
        &mut self,
        data: &mut UpdatePasswordData,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = params.input?;
        let name = data.username.as_str();

        let verified = match params.db.verify_player(name, input).await {
            Ok(verified) => verified,
            Err(e) => {
                tracing::error!("get user hash error: {:?}", e);
                params.send_prompt(vec![UPDATE_PASSWORD_ERROR]).await;
                return Some(TransitionAction::PopFsm);
            }
        };

        if verified {
            params
                .send(vec!["|SteelBlue3|Password verified.|-|", ""])
                .await;
            Some(Transition::VerifiedPassword.into())
        } else {
            tracing::info!("verification failed for user {}", name);
            params
                .send_prompt(vec!["|Red1|Verification failed.|-|"])
                .await;
            Some(TransitionAction::PopFsm)
        }
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
            Transition::EnteredPassword => Some(StateId::ConfirmPassword),
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, data: &mut UpdatePasswordData, params: &'a mut Params<'_>) {
        data.pw_hash = None;
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|New password?|-|"])
            .await;
    }

    async fn decide<'a>(
        &mut self,
        data: &mut UpdatePasswordData,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = params.input?;

        if let Some(msg) = verify_len(input) {
            params.send_prompt(vec![msg]).await;
            return Some(TransitionAction::PopFsm);
        }

        if let Ok(hash) = hash_input(input) {
            data.pw_hash = Some(hash);
            params
                .send(vec!["|SteelBlue3|Password accepted.|-|", ""])
                .await;
            Some(Transition::EnteredPassword.into())
        } else {
            params.send_prompt(vec![UPDATE_PASSWORD_ERROR]).await;
            Some(TransitionAction::PopFsm)
        }
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
            _ => None,
        }
    }

    async fn on_enter<'a>(&mut self, _data: &mut UpdatePasswordData, params: &'a mut Params<'_>) {
        params
            .send_sensitive_prompt(vec!["|SteelBlue3|Confirm?|-|"])
            .await;
    }

    async fn decide<'a>(
        &mut self,
        data: &mut UpdatePasswordData,
        params: &'a mut Params<'_>,
    ) -> Option<TransitionAction<Transition>> {
        let input = params.input?;

        if data.pw_hash.is_none() {
            params.send(vec![UPDATE_PASSWORD_ERROR]).await;
            return Some(TransitionAction::PopFsm);
        }

        let hash = data.pw_hash.as_ref().unwrap().as_str();

        match verify_password(hash, input) {
            Ok(_) => {
                match params
                    .db
                    .update_password(data.username.as_str(), hash)
                    .await
                {
                    Ok(_) => {
                        params
                            .send_prompt(vec!["|SteelBlue3|Password updated.|-|"])
                            .await;
                        return Some(TransitionAction::PopFsm);
                    }
                    Err(e) => {
                        tracing::error!("failed to update password: {}", e);
                        params.send_prompt(vec![UPDATE_PASSWORD_ERROR]).await;
                        return Some(TransitionAction::PopFsm);
                    }
                }
            }
            Err(e) => {
                if let VerifyError::Unknown(e) = e {
                    tracing::error!("update password confirm failure: {}", e);
                    params.send_prompt(vec![UPDATE_PASSWORD_ERROR]).await;
                } else {
                    params
                        .send_prompt(vec!["|Red1|Confirmation failed.|-|"])
                        .await;
                }
                return Some(TransitionAction::PopFsm);
            }
        }
    }
}
