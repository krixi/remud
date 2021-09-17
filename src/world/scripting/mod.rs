#![allow(dead_code)]

use std::sync::Arc;

use bevy_app::{EventReader, Events};
use bevy_ecs::prelude::*;
use itertools::Itertools;
use rhai::plugin::*;

use crate::world::{action::ActionEvent, types::Container};

pub struct PreAction {
    pub action: ActionEvent,
}

pub struct PostAction {
    pub action: ActionEvent,
}

pub fn pre_script_system(world: &mut World) {
    let world = world.cell();

    let pre_action_events = world.get_resource_mut::<Events<PreAction>>().unwrap();
    let mut action_events = world.get_resource_mut::<Events<ActionEvent>>().unwrap();

    for PreAction { action } in pre_action_events.get_reader().iter(&*pre_action_events) {
        action_events.send(action.clone());
    }
}

pub fn post_script_system(world: &mut World) {
    let world = world.cell();

    let action_events = world.get_resource_mut::<Events<ActionEvent>>().unwrap();
    let mut post_action_events = world.get_resource_mut::<Events<PostAction>>().unwrap();

    for action in action_events.get_reader().iter(&*action_events) {
        post_action_events.send(PostAction {
            action: action.clone(),
        });
    }
}

#[derive(Debug)]
pub struct PlayerAction {
    player: Entity,
    event: PlayerEvent,
}

impl PlayerAction {
    fn trigger(&self) -> Trigger {
        match self.event {
            PlayerEvent::Say { .. } => Trigger::Say,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Say { room: Entity, message: String },
}

#[derive(Debug, Clone)]
pub enum TriggerData {
    Player(Entity, PlayerEvent),
}

#[export_module]
pub mod trigger_api {
    use crate::world::scripting::TriggerData;

    #[rhai_fn(get = "entity", pure)]
    pub fn get_entity(trigger_data: &mut TriggerData) -> Dynamic {
        match trigger_data {
            TriggerData::Player(entity, _) => Dynamic::from(*entity),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Script(pub String);

pub struct ScriptExecutions {
    pub runs: Vec<(TriggerData, Script)>,
}

pub struct ScriptTriggers {
    list: Vec<(Trigger, Script)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Say,
}

#[export_module]
pub mod world_api {
    use std::sync::RwLock;

    use rhai::Dynamic;

    use crate::world::types::Named;

    #[rhai_fn(pure)]
    pub fn get_name(world: &mut Arc<RwLock<World>>, entity: Entity) -> Dynamic {
        if let Some(named) = world.read().unwrap().get::<Named>(entity) {
            Dynamic::from(named.name.clone())
        } else {
            Dynamic::UNIT
        }
    }
}

fn player_action_events(
    mut commands: Commands,
    mut actions: EventReader<PlayerAction>,
    objects_query: Query<(Entity, &ScriptTriggers, &Container)>,
    mut executions_query: Query<&mut ScriptExecutions>,
) {
    for action in actions.iter() {
        let room = match action.event {
            PlayerEvent::Say { room, .. } => Some(room),
        };

        for (object_entity, script_triggers, container) in objects_query.iter() {
            if let Some(room) = room {
                if container.entity != room {
                    continue;
                }
            }

            let trigger = action.trigger();

            let scripts = script_triggers
                .list
                .iter()
                .filter(|(script_trigger, _)| trigger == *script_trigger)
                .map(|(_, script)| script)
                .collect_vec();

            if let Ok(mut executions) = executions_query.get_mut(object_entity) {
                for script in scripts {
                    executions.runs.push((
                        TriggerData::Player(action.player, action.event.clone()),
                        script.clone(),
                    ));
                }
            } else {
                let executions = {
                    let runs = scripts
                        .into_iter()
                        .map(|script| {
                            (
                                TriggerData::Player(action.player, action.event.clone()),
                                script.clone(),
                            )
                        })
                        .collect_vec();
                    ScriptExecutions { runs }
                };
                commands.entity(object_entity).insert(executions);
            };
        }
    }
}
