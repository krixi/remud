pub mod states;
pub mod system;

use anyhow::{self, bail};
use bevy_ecs::prelude::*;
use std::{collections::HashMap, fmt::Debug};

use crate::{
    ecs::{Ecs, Phase, Plugin, Step},
    world::fsm::{
        states::{FollowState, WanderState},
        system::run_state_machines,
    },
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, SystemLabel)]
pub enum FsmSystem {
    RunStateMachines,
}

#[derive(Default)]
pub struct FsmPlugin {}

impl Plugin for FsmPlugin {
    fn build(&self, ecs: &mut Ecs) {
        ecs.add_system(
            Step::Main,
            Phase::Update,
            run_state_machines
                .exclusive_system()
                .label(FsmSystem::RunStateMachines)
                .at_end(),
        );
    }
}

#[derive(Default, Debug)]
pub struct StateMachines {
    stack: Vec<StateMachine>,
}

impl StateMachines {
    pub fn new(fsm: StateMachine) -> Self {
        StateMachines { stack: vec![fsm] }
    }

    pub fn push(&mut self, fsm: StateMachine) {
        self.stack.push(fsm)
    }

    pub fn pop(&mut self) -> Option<StateMachine> {
        self.stack.pop()
    }
}

pub trait State: Debug + Send + Sync {
    fn on_enter(&mut self, _entity: Entity, _world: &mut World) {}
    fn decide(&mut self, _entity: Entity, _world: &mut World) -> Option<Transition> {
        None
    }
    fn act(&mut self, _entity: Entity, _world: &mut World) {}
    fn on_exit(&mut self, _entity: Entity, _world: &mut World) {}
    fn output_state(&self, next: Transition) -> Option<StateId>;
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, Ord, PartialOrd)]
pub enum StateId {
    Chase,
    Wander,
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum Transition {
    SawTarget,
    Done,
}

#[derive(Debug)]
pub struct StateMachine {
    pub states: HashMap<StateId, Box<dyn State>>,
    pub current: StateId,
}
impl StateMachine {
    fn on_update(&mut self, entity: Entity, world: &mut World) {
        // delegate to current state -
        // Step 1: see if it requested a transition by calling decide.
        let current_state = self.states.get_mut(&self.current).unwrap();
        let next = current_state
            .decide(entity, world)
            .and_then(|tx| current_state.output_state(tx));

        // Step 2: transition states if required
        let current_state = match next {
            Some(state) => {
                // exit outgoing state.
                current_state.on_exit(entity, world);
                // assert that the new state exists
                let next_state = self.states.get_mut(&state).unwrap();
                // update the current state reference
                self.current = state;
                // enter the new state
                next_state.on_enter(entity, world);
                next_state
            }
            None => current_state,
        };

        // Step 3: let the new current state act.
        current_state.act(entity, world)
    }
}

#[derive(Debug, Default, Clone)]
pub struct StateMachineBuilder {
    states: Vec<(StateId, rhai::Map, rhai::Array)>,
}

impl StateMachineBuilder {
    pub fn build(self) -> anyhow::Result<StateMachine> {
        let mut states = HashMap::new();
        let mut first = None;
        for (id, params, tx) in self.states {
            if first == None {
                first = Some(id)
            }
            states.insert(id, to_state(id, params, tx));
        }

        if let Some(current) = first {
            Ok(StateMachine { states, current })
        } else {
            bail!("You must configure states for all state machines. No states were found.");
        }
    }

    pub fn add_state(&mut self, id: &StateId, params: rhai::Map, tx: rhai::Array) {
        self.states.push((*id, params, tx));
    }
}

pub fn to_state(id: StateId, params: rhai::Map, tx: rhai::Array) -> Box<dyn State> {
    match id {
        StateId::Wander => Box::new(WanderState::new(params, tx)),
        StateId::Chase => Box::new(FollowState::new(params, tx)),
    }
}
