pub mod states;
pub mod system;

use anyhow::{self, bail};
use bevy_ecs::prelude::*;
use std::{collections::HashMap, fmt::Debug};

pub trait State: Debug + Send + Sync {
    fn on_enter(&mut self, _entity: Entity, _world: &mut World) {}
    fn decide(&mut self, _entity: Entity, _world: &mut World) -> Option<Transition> {
        None
    }
    fn act(&mut self, _entity: Entity, _world: &mut World) {}
    fn on_exit(&mut self, _entity: Entity, _world: &mut World) {}
    fn output_state(&self, next: Transition) -> Option<StateId>;
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum StateId {
    Wander,
    Chase,
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum Transition {
    SawPlayer,
    LostPlayer,
}

#[derive(Debug)]
pub struct StateMachine {
    states: HashMap<StateId, Box<dyn State>>,
    current: StateId,
}
impl StateMachine {
    pub fn new() -> StateMachineBuilder {
        StateMachineBuilder::default()
    }

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

#[derive(Default)]
pub struct StateMachineBuilder {
    states: Vec<(StateId, Box<dyn State>)>,
}

impl StateMachineBuilder {
    pub fn build(self) -> anyhow::Result<StateMachine> {
        let mut states = HashMap::new();
        let mut first = None;
        for (id, state) in self.states {
            if first == None {
                first = Some(id)
            }
            states.insert(id, state);
        }

        if let Some(current) = first {
            Ok(StateMachine { states, current })
        } else {
            bail!("You must configure states for all state machines");
        }
    }

    pub fn with_state<T: 'static + State>(mut self, id: StateId, state: T) -> Self {
        self.states.push((id, Box::new(state)));
        self
    }
}
