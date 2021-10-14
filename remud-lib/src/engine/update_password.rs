use crate::{
    engine::{
        client::ClientSender,
        db::Db,
        fsm::{Fsm, Params, ParamsInfo},
    },
    ClientId,
};

pub struct UpdatePasswordFsm {
    fsm: Fsm<Transition, StateId, UpdatePasswordState, UpdatePasswordParamsInfo>,
    data: UpdatePasswordState,
}

#[derive(Debug)]
pub enum Transition {}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum StateId {}

pub struct UpdatePasswordState {
    pub pw_hash: Option<String>,
}

pub struct UpdatePasswordParamsInfo {}

impl ParamsInfo for UpdatePasswordParamsInfo {
    type Params<'p> = UpdatePasswordParams<'p>;
}

pub struct UpdatePasswordParams<'p> {
    pub id: ClientId,
    pub sender: &'p ClientSender,
    pub input: Option<&'p str>,
    pub db: &'p Db,
}

impl<'a> Params<'a> for UpdatePasswordParams<'a> {}
