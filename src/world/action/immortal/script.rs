use std::convert::TryFrom;

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use either::Either;

use crate::{
    engine::persist::{self, Updates},
    into_action,
    text::Tokenizer,
    world::{
        action::Action,
        scripting::{Script, ScriptHook, ScriptHooks, ScriptName, ScriptTrigger, Scripts},
        types::{
            object::{ObjectId, Objects},
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
                "attach" => parse_params(player, script, tokenizer, Command::Attach),
                "attach-pre" => parse_params(player, script, tokenizer, Command::AttachPre),
                "detach" => parse_params(player, script, tokenizer, Command::Detach),
                _ => Err("Enter a valid subcommand: attach, attach-pre, detach.".to_string()),
            }
        } else {
            Err("Enter a subcommand: attach, attach-pre, detach.".to_string())
        }
    } else {
        Err("Enter a script name.".to_string())
    }
}

fn parse_params(
    player: Entity,
    script: ScriptName,
    mut tokenizer: Tokenizer,
    command: Command,
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
                "room" => Ok(command.into_action(
                    player,
                    script,
                    Either::Left(id.parse::<RoomId>().map_err(|e| e.to_string())?.into()),
                )),
                _ => Err("Enter a valid target type: object, player, or room.".to_string()),
            }
        } else {
            Err("Enter a room ID, object ID, or player name.".to_string())
        }
    } else {
        Err("Enter a target type: object, player, or room.".to_string())
    }
}

enum Command {
    Attach,
    AttachPre,
    Detach,
}

impl Command {
    fn into_action(self, entity: Entity, script: ScriptName, id: Either<Id, String>) -> Action {
        match self {
            Command::Attach => ScriptAttach {
                entity,
                script,
                pre: false,
                target: id,
            }
            .into(),
            Command::AttachPre => ScriptAttach {
                entity,
                script,
                pre: true,
                target: id,
            }
            .into(),
            Command::Detach => ScriptDetach {
                entity,
                script,
                target: id,
            }
            .into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptAttach {
    pub entity: Entity,
    pub script: ScriptName,
    pub pre: bool,
    pub target: Either<Id, String>,
}

into_action!(ScriptAttach);

pub fn script_attach_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    scripts: Res<Scripts>,
    objects: Res<Objects>,
    rooms: Res<Rooms>,
    players: Res<Players>,
    mut updates: ResMut<Updates>,
    script_query: Query<&Script>,
    player_query: Query<&Player>,
    mut hook_query: Query<&mut ScriptHooks>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ScriptAttach(ScriptAttach {
            entity,
            script,
            pre,
            target,
        }) = action
        {
            let script_entity = if let Some(script) = scripts.by_name(script) {
                script
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!("Script {} not found.", script));
                }
                continue;
            };

            let target_entity = match target {
                Either::Left(id) => match id {
                    Id::Object(id) => {
                        if let Some(object) = objects.by_id(*id) {
                            object
                        } else {
                            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                                messages.queue(format!("Target object {} not found.", id));
                            }
                            continue;
                        }
                    }
                    Id::Room(id) => {
                        if let Some(room) = rooms.by_id(*id) {
                            room
                        } else {
                            if let Ok(mut messages) = messages_query.get_mut(*entity) {
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
                        if let Ok(mut messages) = messages_query.get_mut(*entity) {
                            messages.queue(format!("Target player {} not found.", name));
                        }
                        continue;
                    }
                }
            };

            let trigger = {
                let trigger = script_query.get(script_entity).unwrap().trigger;
                match pre {
                    true => ScriptTrigger::PreEvent(trigger),
                    false => ScriptTrigger::PostEvent(trigger),
                }
            };

            let hook = ScriptHook {
                trigger,
                script: script.clone(),
            };

            if let Ok(mut hooks) = hook_query.get_mut(target_entity) {
                if hooks.list.contains(&hook) {
                    if let Ok(mut messages) = messages_query.get_mut(*entity) {
                        messages.queue(format!("Script {} already attached to entity.", script));
                    }
                    continue;
                }
                hooks.list.push(hook);
            } else {
                commands
                    .entity(target_entity)
                    .insert(ScriptHooks { list: vec![hook] });
            }

            let id = match target {
                Either::Left(id) => *id,
                Either::Right(_) => Id::Player(player_query.get(target_entity).unwrap().id),
            };

            updates.queue(persist::script::Attach::new(id, script.clone(), trigger));

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                match target {
                    Either::Left(id) => {
                        match id {
                            Id::Player(_) => unreachable!("Players are referenced by name."),
                            Id::Object(id) => messages
                                .queue(format!("Script {} attached to object {}.", script, id)),
                            Id::Room(id) => messages
                                .queue(format!("Script {} attached to room {}.", script, id)),
                        }
                    }
                    Either::Right(name) => {
                        messages.queue(format!("Script {} attached to player {}.", script, name))
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptDetach {
    pub entity: Entity,
    pub script: ScriptName,
    pub target: Either<Id, String>,
}

into_action!(ScriptDetach);

pub fn script_detach_system(
    mut action_reader: EventReader<Action>,
    scripts: Res<Scripts>,
    objects: Res<Objects>,
    rooms: Res<Rooms>,
    players: Res<Players>,
    mut updates: ResMut<Updates>,
    script_query: Query<&Script>,
    player_query: Query<&Player>,
    mut hook_query: Query<&mut ScriptHooks>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ScriptDetach(ScriptDetach {
            entity,
            script,
            target,
        }) = action
        {
            let script_entity = if let Some(script) = scripts.by_name(script) {
                script
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!("Script {} not found.", script));
                }
                continue;
            };

            let target_entity = match target {
                Either::Left(id) => match id {
                    Id::Object(id) => {
                        if let Some(object) = objects.by_id(*id) {
                            object
                        } else {
                            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                                messages.queue(format!("Target object {} not found.", id));
                            }
                            continue;
                        }
                    }
                    Id::Room(id) => {
                        if let Some(room) = rooms.by_id(*id) {
                            room
                        } else {
                            if let Ok(mut messages) = messages_query.get_mut(*entity) {
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
                        if let Ok(mut messages) = messages_query.get_mut(*entity) {
                            messages.queue(format!("Target player {} not found.", name));
                        }
                        continue;
                    }
                }
            };

            let trigger_event = script_query.get(script_entity).unwrap().trigger;

            let mut remove_trigger = None;

            if let Ok(mut hooks) = hook_query.get_mut(target_entity) {
                let hook = ScriptHook {
                    trigger: ScriptTrigger::PreEvent(trigger_event),
                    script: script.clone(),
                };

                if hooks.remove(&hook) {
                    remove_trigger = Some(hook.trigger);
                } else {
                    let hook = ScriptHook {
                        trigger: ScriptTrigger::PostEvent(trigger_event),
                        script: script.clone(),
                    };

                    if hooks.remove(&hook) {
                        remove_trigger = Some(hook.trigger);
                    }
                }
            }

            if remove_trigger.is_none() {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!("Script {} not found on target.", script));
                }
                continue;
            }

            let id = match target {
                Either::Left(id) => *id,
                Either::Right(_) => Id::Player(player_query.get(target_entity).unwrap().id),
            };

            updates.queue(persist::script::Detach::new(
                id,
                script.clone(),
                remove_trigger.unwrap(),
            ));

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                match target {
                    Either::Left(id) => match id {
                        Id::Player(_) => unreachable!("Players are referenced by name."),
                        Id::Object(id) => messages
                            .queue(format!("Script {} detached from object {}.", script, id)),
                        Id::Room(id) => {
                            messages.queue(format!("Script {} detached from room {}.", script, id))
                        }
                    },
                    Either::Right(name) => {
                        messages.queue(format!("Script {} detached from player {}.", script, name))
                    }
                }
            }
        }
    }
}
