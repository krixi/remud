pub mod object;
pub mod player;
pub mod prototype;
pub mod room;
pub mod script;

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    world::{
        action::{get_room_std, into_action, Action},
        fsm::StateMachines,
        scripting::{
            time::Timers, ExecutionErrors, Script, ScriptData, ScriptHooks, ScriptName, ScriptRun,
            ScriptRuns, ScriptTrigger, Scripts,
        },
        types::{
            object::{Objects, Prototypes},
            player::{Messages, Player, Players},
            room::Room,
            ActionTarget, Description, Id, Location, Named,
        },
    },
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Initialize {
    pub actor: Entity,
    pub target: ActionTarget,
}

into_action!(Initialize);

#[tracing::instrument(name = "initialize system", skip_all)]
pub fn initialize_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    players: Res<Players>,
    mut runs: ResMut<ScriptRuns>,
    hooks_query: Query<&ScriptHooks>,
    location_query: Query<(Option<&Location>, Option<&Room>)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Initialize(Initialize { actor, target }) = action {
            let entity = match target {
                ActionTarget::PlayerSelf => *actor,
                ActionTarget::Player(name) => {
                    if let Some(entity) = players.by_name(name.as_str()) {
                        entity
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Player {} not found", name));
                        }
                        continue;
                    }
                }
                ActionTarget::Prototype(_) => {
                    if let Ok(mut messages) = messages_query.get_mut(*actor) {
                        messages
                            .queue("Prototypes cannot have their init scripts run.".to_string());
                    }
                    continue;
                }
                ActionTarget::Object(id) => {
                    if let Some(entity) = objects.by_id(*id) {
                        entity
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Object {} not found.", id));
                        }
                        continue;
                    }
                }
                ActionTarget::CurrentRoom => get_room_std(*actor, &location_query),
            };

            let mut queued = 0;
            if let Ok(hooks) = hooks_query.get(entity) {
                for script in hooks.by_trigger(ScriptTrigger::Init) {
                    runs.queue_init(ScriptRun::new(entity, script));
                    queued += 1;
                }
            }

            commands
                .entity(entity)
                .remove::<Timers>()
                .remove::<ScriptData>()
                .remove::<StateMachines>();

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                if queued > 0 {
                    messages.queue(format!(
                        "Queued {} init scripts for execution on {}.",
                        queued, target
                    ));
                } else {
                    messages.queue(format!("Found no init scripts for {}.", target));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ShowError {
    pub actor: Entity,
    pub target: ActionTarget,
    pub script: ScriptName,
}

into_action!(ShowError);

#[tracing::instrument(name = "show error", skip_all)]
pub fn show_error_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    players: Res<Players>,
    scripts: Res<Scripts>,
    errors_query: Query<&ExecutionErrors>,
    script_query: Query<&Script>,
    location_query: Query<(Option<&Location>, Option<&Room>)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ShowError(ShowError {
            actor,
            target,
            script,
        }) = action
        {
            let entity = match target {
                ActionTarget::PlayerSelf => *actor,
                ActionTarget::Player(name) => {
                    if let Some(entity) = players.by_name(name.as_str()) {
                        entity
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Player {} not found", name));
                        }
                        continue;
                    }
                }
                ActionTarget::Prototype(_) => {
                    if let Ok(mut messages) = messages_query.get_mut(*actor) {
                        messages
                            .queue("Prototypes cannot have their init scripts run.".to_string());
                    }
                    continue;
                }
                ActionTarget::Object(id) => {
                    if let Some(entity) = objects.by_id(*id) {
                        entity
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Object {} not found.", id));
                        }
                        continue;
                    }
                }
                ActionTarget::CurrentRoom => get_room_std(*actor, &location_query),
            };

            let script_entity = if let Some(entity) = scripts.by_name(script) {
                entity
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Script {} not found.", script));
                }
                continue;
            };

            let error = if let Ok(errors) = errors_query.get(entity) {
                if let Some(error) = errors.get(script) {
                    error
                } else {
                    if let Ok(mut messages) = messages_query.get_mut(*actor) {
                        messages.queue(format!("No errors found for script {}.", script));
                    }
                    continue;
                }
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("No errors found for {}.", target));
                }
                continue;
            };

            let mut code = script_query.get(script_entity).unwrap().code();
            let message = if let Some(line) = error.position().line() {
                let mut lines = code.lines().map(ToString::to_string).collect_vec();
                if let Some(position) = error.position().position() {
                    lines[line - 1].insert_str(position - 1, "|red|");
                    lines[line - 1].push_str("|-|");
                } else {
                    lines[line - 1] = format!("|red|{}|-|", lines[line - 1]);
                }
                let mut joined = lines.join("\r\n");
                joined.push_str(format!("\r\n\r\n|red|{}|-|", error.to_string()).as_ref());
                joined
            } else {
                code.push_str(format!("\r\n\r\n|red|{}|-|", error.to_string()).as_ref());
                code
            };

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UpdateDescription {
    pub actor: Entity,
    pub target: ActionTarget,
    pub description: String,
}

into_action!(UpdateDescription);

#[tracing::instrument(name = "update description system", skip_all)]
pub fn update_description_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    players: Res<Players>,
    prototypes: Res<Prototypes>,
    mut updates: ResMut<Updates>,
    player_query: Query<&Player>,
    location_query: Query<(Option<&Location>, Option<&Room>)>,
    room_query: Query<&Room>,
    mut description_query: Query<&mut Description>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::UpdateDescription(UpdateDescription {
            actor,
            target,
            description,
        }) = action
        {
            let (id, entity) = match target {
                ActionTarget::CurrentRoom => {
                    let room = get_room_std(*actor, &location_query);
                    let id = room_query.get(room).unwrap().id();
                    (Id::Room(id), room)
                }
                ActionTarget::Object(id) => {
                    if let Some(entity) = objects.by_id(*id) {
                        (Id::Object(*id), entity)
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Object {} not found.", id));
                        }
                        continue;
                    }
                }
                ActionTarget::Prototype(id) => {
                    if let Some(entity) = prototypes.by_id(*id) {
                        (Id::Prototype(*id), entity)
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Prototype {} not found.", id));
                        }
                        continue;
                    }
                }
                ActionTarget::PlayerSelf => {
                    let id = player_query.get(*actor).unwrap().id();
                    (Id::Player(id), *actor)
                }
                ActionTarget::Player(name) => {
                    if let Some(entity) = players.by_name(name.as_str()) {
                        let id = player_query.get(*actor).unwrap().id();
                        (Id::Player(id), entity)
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Player {} not found", name));
                        }
                        continue;
                    }
                }
            };

            description_query
                .get_mut(entity)
                .unwrap()
                .set_text(description.clone());

            match id {
                Id::Player(id) => {
                    updates.persist(persist::player::Description::new(id, description.clone()))
                }
                Id::Prototype(id) => {
                    updates.persist(persist::prototype::Description::new(
                        id,
                        description.clone(),
                    ));
                    updates.reload(id);
                }
                Id::Object(id) => {
                    updates.persist(persist::object::Description::new(id, description.clone()))
                }
                Id::Room(id) => {
                    updates.persist(persist::room::Description::new(id, description.clone()))
                }
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Updated {} description.", target));
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UpdateName {
    pub actor: Entity,
    pub target: ActionTarget,
    pub name: String,
}

into_action!(UpdateName);

#[tracing::instrument(name = "update name system", skip_all)]
pub fn update_name_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    prototypes: Res<Prototypes>,
    mut updates: ResMut<Updates>,
    location_query: Query<(Option<&Location>, Option<&Room>)>,
    room_query: Query<&Room>,
    mut name_query: Query<&mut Named>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::UpdateName(UpdateName {
            actor,
            target,
            name,
        }) = action
        {
            let (id, entity) = match target {
                ActionTarget::CurrentRoom => {
                    let room = get_room_std(*actor, &location_query);
                    let id = room_query.get(room).unwrap().id();
                    (Id::Room(id), room)
                }
                ActionTarget::Object(id) => {
                    if let Some(entity) = objects.by_id(*id) {
                        (Id::Object(*id), entity)
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Object {} not found.", id));
                        }
                        continue;
                    }
                }
                ActionTarget::PlayerSelf => todo!(),
                ActionTarget::Player(_) => todo!(),
                ActionTarget::Prototype(id) => {
                    if let Some(entity) = prototypes.by_id(*id) {
                        (Id::Prototype(*id), entity)
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Prototype {} not found.", id));
                        }
                        continue;
                    }
                }
            };

            name_query.get_mut(entity).unwrap().set_name(name.clone());

            match id {
                Id::Prototype(id) => {
                    updates.persist(persist::prototype::Name::new(id, name.clone()));
                    updates.reload(id);
                }
                Id::Object(id) => {
                    updates.persist(persist::object::Name::new(id, name.clone()));
                }
                Id::Player(_) => todo!(),
                Id::Room(id) => {
                    updates.persist(persist::room::Name::new(id, name.clone()));
                }
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Updated {} name.", target));
            }
        }
    }
}
