use bevy_app::{EventReader, EventWriter};
use bevy_ecs::prelude::*;

use crate::world::{
    action::Action,
    scripting::{
        time::Timers, CompilationError, QueuedAction, RunInitScript, Script, ScriptAst,
        ScriptEngine, ScriptHooks, ScriptRun, ScriptRuns, ScriptTrigger, TriggerEvent,
    },
    types::{object::Container, room::Room, Contents, Location},
};

pub fn script_compiler_system(
    mut commands: Commands,
    engine: Res<ScriptEngine>,
    uncompiled_scripts: Query<(Entity, &Script), (Without<ScriptAst>, Without<CompilationError>)>,
) {
    for (entity, script) in uncompiled_scripts.iter() {
        match engine.compile(script.code.as_str()) {
            Ok(ast) => {
                commands.entity(entity).insert(ScriptAst { ast });
            }
            Err(error) => {
                commands.entity(entity).insert(CompilationError { error });
            }
        }
    }
}

pub fn init_script_runs_system(
    mut init_reader: EventReader<RunInitScript>,
    mut script_runs: ResMut<ScriptRuns>,
) {
    for RunInitScript { entity, script } in init_reader.iter() {
        script_runs.init_runs.push(ScriptRun {
            entity: *entity,
            script: script.clone(),
        })
    }
}

pub fn timed_script_runs_system(
    mut timers_query: Query<(Entity, &mut Timers, &ScriptHooks)>,
    mut script_runs: ResMut<ScriptRuns>,
) {
    for (entity, mut timers, hooks) in timers_query.iter_mut() {
        for name in timers.finished() {
            for script in hooks.by_trigger(ScriptTrigger::Timer(name)) {
                script_runs.timed_runs.push(ScriptRun { entity, script })
            }
        }
    }
}

pub fn pre_event_script_runs_system(
    mut queued_action_reader: EventReader<QueuedAction>,
    mut action_writer: EventWriter<Action>,
    mut script_runs: ResMut<ScriptRuns>,
    room_query: Query<&Room>,
    location_query: Query<&Location>,
    container_query: Query<&Container>,
    contents_query: Query<&Contents>,
    hooks_query: Query<&ScriptHooks>,
) {
    for QueuedAction { action } in queued_action_reader.iter() {
        let trigger_event = match TriggerEvent::from_action(action) {
            Some(trigger) => trigger,
            None => {
                action_writer.send(action.clone());
                continue;
            }
        };

        let enactor = action.actor();

        // Determine the location the action took place. If we can't, we give the action a pass.
        let room = if let Some(room) =
            action_room(enactor, &room_query, &location_query, &container_query)
        {
            room
        } else {
            tracing::warn!("Unable to determine location of action {:?}", action);
            action_writer.send(action.clone());
            continue;
        };

        // Check if any scripts need to run for this action
        let runs = get_script_runs(
            ScriptTrigger::PreEvent(trigger_event),
            room,
            &hooks_query,
            &contents_query,
            &room_query,
        );

        if runs.is_empty() {
            action_writer.send(action.clone());
        } else {
            script_runs.runs.push((action.clone(), runs));
        }
    }
}

pub fn post_action_script_runs_system(
    mut queued_action_reader: EventReader<QueuedAction>,
    mut script_runs: ResMut<ScriptRuns>,
    room_query: Query<&Room>,
    location_query: Query<&Location>,
    container_query: Query<&Container>,
    contents_query: Query<&Contents>,
    hooks_query: Query<&ScriptHooks>,
) {
    for QueuedAction { action } in queued_action_reader.iter() {
        let trigger_event = match TriggerEvent::from_action(action) {
            Some(trigger) => trigger,
            None => continue,
        };

        let enactor = action.actor();

        let room = if let Some(room) =
            action_room(enactor, &room_query, &location_query, &container_query)
        {
            room
        } else {
            tracing::warn!("Unable to determine location of action {:?}", action);
            continue;
        };

        let runs = get_script_runs(
            ScriptTrigger::PostEvent(trigger_event),
            room,
            &hooks_query,
            &contents_query,
            &room_query,
        );

        if !runs.is_empty() {
            script_runs.runs.push((action.clone(), runs));
        }
    }
}

fn action_room(
    enactor: Entity,
    room_query: &Query<&Room>,
    location_query: &Query<&Location>,
    container_query: &Query<&Container>,
) -> Option<Entity> {
    if let Ok(location) = location_query.get(enactor) {
        Some(location.room())
    } else if room_query.get(enactor).is_ok() {
        Some(enactor)
    } else {
        let mut containing_entity = enactor;

        while let Ok(container) = container_query.get(containing_entity) {
            containing_entity = container.entity();
        }

        location_query
            .get(containing_entity)
            .map(|location| location.room())
            .ok()
    }
}

fn get_script_runs(
    trigger: ScriptTrigger,
    room: Entity,
    hooks_query: &Query<&ScriptHooks>,
    contents_query: &Query<&Contents>,
    room_query: &Query<&Room>,
) -> Vec<ScriptRun> {
    let mut runs = Vec::new();

    if let Ok(hooks) = hooks_query.get(room) {
        for script in hooks.by_trigger(trigger.clone()) {
            runs.push(ScriptRun {
                entity: room,
                script,
            });
        }
    }

    let contents = contents_query.get(room).unwrap();
    for object in contents.objects() {
        if let Ok(hooks) = hooks_query.get(*object) {
            for script in hooks.by_trigger(trigger.clone()) {
                runs.push(ScriptRun {
                    entity: *object,
                    script,
                });
            }
        }
    }

    let room = room_query.get(room).unwrap();
    for player in room.players() {
        if let Ok(hooks) = hooks_query.get(*player) {
            for script in hooks.by_trigger(trigger.clone()) {
                runs.push(ScriptRun {
                    entity: *player,
                    script,
                });
            }
        }

        let contents = contents_query.get(*player).unwrap();
        for object in contents.objects() {
            if let Ok(hooks) = hooks_query.get(*object) {
                for script in hooks.by_trigger(trigger.clone()) {
                    runs.push(ScriptRun {
                        entity: *object,
                        script,
                    })
                }
            }
        }
    }

    runs
}
