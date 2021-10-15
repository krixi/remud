#[allow(dead_code)]
use crate::engine::fsm::Fsm;

pub struct UpdatePasswordFsm {
    fsm: Fsm<Transition, StateId, UpdatePasswordState>,
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
