use std::convert::TryFrom;

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use either::Either;

use crate::{
    engine::persist::{self, Updates},
    text::Tokenizer,
    world::{
        action::{into_action, Action},
        scripting::{
            Script, ScriptHook, ScriptHooks, ScriptName, ScriptTrigger, Scripts, TriggerKind,
        },
        types::{
            object::{Object, ObjectId, Objects, Prototype, PrototypeId, Prototypes},
            player::{Messages, Player, Players},
            room::{RoomId, Rooms},
            Id,
        },
    },
};

// script <name> attach-pre [object|player|room] <id/name>
// script <name> attach [object|player|room] <id/name>
// script <name> detach [object|player|room] <id/name>
pub fn parse_script(player: Entity, mut tokenizer: Tokenizer) -> Result<Action, String> {
    if let Some(script) = tokenizer.next() {
        let script = ScriptName::try_from(script.to_string()).map_err(|e| e.to_string())?;

        if let Some(command) = tokenizer.next() {
            match command {
                "attach-init" => parse_params(player, script, tokenizer, ScriptCommand::AttachInit),
                "attach-post" => {
                    parse_params(player, script, tokenizer, ScriptCommand::AttachPostAction)
                }
                "attach-pre" => {
                    parse_params(player, script, tokenizer, ScriptCommand::AttachPreAction)
                }
                "attach-timer" => {
                    if let Some(timer_name) = tokenizer.next() {
                        parse_params(
                            player,
                            script,
                            tokenizer,
                            ScriptCommand::AttachTimer(timer_name.to_string()),
                        )
                    } else {
                        Err("Enter a timer name.".to_string())
                    }
                }
                "detach" => parse_params(player, script, tokenizer, ScriptCommand::Detach),
                _ => Err(
                    "Enter a valid subcommand: attach-init, attach-post, attach-pre, \
                     attach-timer, or detach."
                        .to_string(),
                ),
            }
        } else {
            Err(
                "Enter a subcommand: attach-init, attach-post, attach-pre, attach-timer, or \
                 detach."
                    .to_string(),
            )
        }
    } else {
        Err("Enter a script name.".to_string())
    }
}

fn parse_params(
    player: Entity,
    script: ScriptName,
    mut tokenizer: Tokenizer,
    command: ScriptCommand,
) -> Result<Action, String> {
    if let Some(target_type) = tokenizer.next() {
        if let Some(id) = tokenizer.next() {
            match target_type {
                "object" => Ok(command.into_action(
                    player,
                    script,
                    Either::Left(id.parse::<ObjectId>().map_err(|e| e.to_string())?.into()),
                )),
                "player" => Ok(command.into_action(player, script, Either::Right(id.to_string()))),
                "prototype" => Ok(command.into_action(
                    player,
                    script,
                    Either::Left(id.parse::<PrototypeId>().map_err(|e| e.to_string())?.into()),
                )),
                "room" => Ok(command.into_action(
                    player,
                    script,
                    Either::Left(id.parse::<RoomId>().map_err(|e| e.to_string())?.into()),
                )),
                _ => Err(
                    "Enter a valid target type: object, player, prototype, or room.".to_string(),
                ),
            }
        } else {
            Err("Enter a object ID, player name, prototype ID, or room ID.".to_string())
        }
    } else {
        Err("Enter a target type: object, player, prototype, or room.".to_string())
    }
}

enum ScriptCommand {
    AttachInit,
    AttachPostAction,
    AttachPreAction,
    AttachTimer(String),
    Detach,
}

impl ScriptCommand {
    fn into_action(self, actor: Entity, script: ScriptName, id: Either<Id, String>) -> Action {
        match self {
            ScriptCommand::AttachInit => ScriptAttach {
                actor,
                script,
                trigger: TriggerKind::Init,
                target: id,
                timer: None,
            }
            .into(),
            ScriptCommand::AttachPostAction => ScriptAttach {
                actor,
                script,
                trigger: TriggerKind::PostEvent,
                target: id,
                timer: None,
            }
            .into(),
            ScriptCommand::AttachPreAction => ScriptAttach {
                actor,
                script,
                trigger: TriggerKind::PreEvent,
                target: id,
                timer: None,
            }
            .into(),
            ScriptCommand::AttachTimer(name) => ScriptAttach {
                actor,
                script,
                trigger: TriggerKind::Timer,
                target: id,
                timer: Some(name),
            }
            .into(),
            ScriptCommand::Detach => ScriptDetach {
                actor,
                script,
                target: id,
            }
            .into(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ScriptAttach {
    pub actor: Entity,
    pub script: ScriptName,
    pub trigger: TriggerKind,
    pub target: Either<Id, String>,
    pub timer: Option<String>,
}

into_action!(ScriptAttach);

#[tracing::instrument(name = "script attach system", skip_all)]
pub fn script_attach_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    scripts: Res<Scripts>,
    objects: Res<Objects>,
    prototypes: Res<Prototypes>,
    rooms: Res<Rooms>,
    players: Res<Players>,
    mut updates: ResMut<Updates>,
    prototype_query: Query<&Prototype>,
    script_query: Query<&Script>,
    player_query: Query<&Player>,
    mut object_query: Query<&mut Object>,
    mut hook_query: Query<&mut ScriptHooks>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ScriptAttach(ScriptAttach {
            actor,
            script,
            trigger,
            target,
            timer,
        }) = action
        {
            let script_entity = if let Some(script) = scripts.by_name(script) {
                script
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Script {} not found.", script));
                }
                continue;
            };

            let target_entity = match target {
                Either::Left(id) => match id {
                    Id::Prototype(id) => {
                        if let Some(prototype) = prototypes.by_id(*id) {
                            prototype
                        } else {
                            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                                messages.queue(format!("Target prototype {} not found.", id));
                            }
                            continue;
                        }
                    }
                    Id::Object(id) => {
                        if let Some(object) = objects.by_id(*id) {
                            object
                        } else {
                            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                                messages.queue(format!("Target object {} not found.", id));
                            }
                            continue;
                        }
                    }
                    Id::Room(id) => {
                        if let Some(room) = rooms.by_id(*id) {
                            room
                        } else {
                            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                                messages.queue(format!("Target room {} not found.", id));
                            }
                            continue;
                        }
                    }
                    Id::Player(_) => unreachable!("Players are referenced by name."),
                },
                Either::Right(name) => {
                    if let Some(player) = players.by_name(name) {
                        player
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Target player {} not found.", name));
                        }
                        continue;
                    }
                }
            };

            let script_trigger = {
                let trigger_event = &script_query.get(script_entity).unwrap().trigger();
                match trigger {
                    TriggerKind::PreEvent => ScriptTrigger::PreEvent(*trigger_event),
                    TriggerKind::PostEvent => ScriptTrigger::PostEvent(*trigger_event),
                    TriggerKind::Init => ScriptTrigger::Init,
                    TriggerKind::Timer => ScriptTrigger::Timer(timer.clone().unwrap()),
                }
            };

            let hook = ScriptHook {
                trigger: script_trigger.clone(),
                script: script.clone(),
            };

            if let Ok(mut hooks) = hook_query.get_mut(target_entity) {
                if hooks.contains(&hook) {
                    if let Ok(mut messages) = messages_query.get_mut(*actor) {
                        messages.queue(format!("Script {} already attached to entity.", script));
                    }
                    continue;
                }
                hooks.insert(hook);
            } else {
                commands
                    .entity(target_entity)
                    .insert(ScriptHooks::new(hook));
            }

            let id = match target {
                Either::Left(id) => *id,
                Either::Right(_) => Id::Player(player_query.get(target_entity).unwrap().id()),
            };

            let copy = match id {
                Id::Object(_) => {
                    let mut object = object_query.get_mut(target_entity).unwrap();
                    if object.inherit_scripts() {
                        object.set_inherit_scripts(false);
                        Some(prototype_query.get(object.prototype()).unwrap().id())
                    } else {
                        None
                    }
                }
                _ => None,
            };

            updates.persist(persist::script::Attach::new(
                id,
                script.clone(),
                script_trigger,
                copy,
            ));

            if let Id::Prototype(id) = id {
                updates.reload(id);
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                match target {
                    Either::Left(id) => {
                        messages.queue(format!("Script {} attached to {}.", script, id))
                    }
                    Either::Right(name) => {
                        messages.queue(format!("Script {} attached to player {}.", script, name))
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ScriptDetach {
    pub actor: Entity,
    pub script: ScriptName,
    pub target: Either<Id, String>,
}

into_action!(ScriptDetach);

#[tracing::instrument(name = "script detach system", skip_all)]
pub fn script_detach_system(
    mut action_reader: EventReader<Action>,
    prototypes: Res<Prototypes>,
    objects: Res<Objects>,
    rooms: Res<Rooms>,
    players: Res<Players>,
    mut updates: ResMut<Updates>,
    player_query: Query<&Player>,
    prototype_query: Query<&Prototype>,
    mut object_query: Query<&mut Object>,
    mut hook_query: Query<&mut ScriptHooks>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ScriptDetach(ScriptDetach {
            actor,
            script,
            target,
        }) = action
        {
            let target_entity = match target {
                Either::Left(id) => match id {
                    Id::Prototype(id) => {
                        if let Some(prototype) = prototypes.by_id(*id) {
                            prototype
                        } else {
                            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                                messages.queue(format!("Target prototype {} not found.", id));
                            }
                            continue;
                        }
                    }
                    Id::Object(id) => {
                        if let Some(object) = objects.by_id(*id) {
                            object
                        } else {
                            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                                messages.queue(format!("Target object {} not found.", id));
                            }
                            continue;
                        }
                    }
                    Id::Room(id) => {
                        if let Some(room) = rooms.by_id(*id) {
                            room
                        } else {
                            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                                messages.queue(format!("Target room {} not found.", id));
                            }
                            continue;
                        }
                    }
                    Id::Player(_) => unreachable!(),
                },
                Either::Right(name) => {
                    if let Some(player) = players.by_name(name) {
                        player
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Target player {} not found.", name));
                        }
                        continue;
                    }
                }
            };

            let mut remove_trigger = None;
            if let Ok(mut hooks) = hook_query.get_mut(target_entity) {
                if let Some(hook) = hooks.remove(script) {
                    remove_trigger = Some(hook.trigger)
                }
            }

            if remove_trigger.is_none() {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Script {} not found on target.", script));
                }
                continue;
            }

            let id = match target {
                Either::Left(id) => *id,
                Either::Right(_) => Id::Player(player_query.get(target_entity).unwrap().id()),
            };

            let copy = match id {
                Id::Object(_) => {
                    let mut object = object_query.get_mut(target_entity).unwrap();
                    if object.inherit_scripts() {
                        object.set_inherit_scripts(false);
                        Some(prototype_query.get(object.prototype()).unwrap().id())
                    } else {
                        None
                    }
                }
                _ => None,
            };

            updates.persist(persist::script::Detach::new(
                id,
                script.clone(),
                remove_trigger.unwrap(),
                copy,
            ));

            if let Id::Prototype(id) = id {
                updates.reload(id);
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                match target {
                    Either::Left(id) => {
                        messages.queue(format!("Detached script {} from {}.", script, id))
                    }
                    Either::Right(name) => {
                        messages.queue(format!("Detached script {} from player {}.", script, name))
                    }
                }
            }
        }
    }
}
