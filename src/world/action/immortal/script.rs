use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use either::Either;

use crate::{
    engine::persist::{self, Updates},
    event_from_action,
    text::Tokenizer,
    world::{
        action::ActionEvent,
        scripting::{
            PostEventScriptHooks, PreEventScriptHooks, Script, ScriptHook, ScriptName, Scripts,
        },
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
// script <name> detach-pre [object|player|room] <id/name>
// script <name> detach [object|player|room] <id/name>
pub fn parse_script(player: Entity, mut tokenizer: Tokenizer) -> Result<ActionEvent, String> {
    if let Some(script_name) = tokenizer.next() {
        if let Some(command) = tokenizer.next() {
            match command {
                "attach" => parse_params(player, tokenizer, Command::Attach, script_name),
                "attach-pre" => parse_params(player, tokenizer, Command::AttachPre, script_name),
                "detach" => parse_params(player, tokenizer, Command::Detach, script_name),
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
    mut tokenizer: Tokenizer,
    command: Command,
    script_name: &str,
) -> Result<ActionEvent, String> {
    if let Some(target_type) = tokenizer.next() {
        if let Some(id) = tokenizer.next() {
            match target_type {
                "object" => Ok(command.into_action(
                    player,
                    script_name,
                    Either::Left(id.parse::<ObjectId>().map_err(|e| e.to_string())?.into()),
                )),
                "player" => {
                    Ok(command.into_action(player, script_name, Either::Right(id.to_string())))
                }
                "room" => Ok(command.into_action(
                    player,
                    script_name,
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
    fn into_action(self, entity: Entity, script_name: &str, id: Either<Id, String>) -> ActionEvent {
        let script = ScriptName::from(script_name);
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

event_from_action!(ScriptAttach);

pub fn script_attach_system(
    mut commands: Commands,
    mut events: EventReader<ActionEvent>,
    scripts: Res<Scripts>,
    objects: Res<Objects>,
    rooms: Res<Rooms>,
    players: Res<Players>,
    mut updates: ResMut<Updates>,
    script_query: Query<&Script>,
    player_query: Query<&Player>,
    mut pre_hook_query: Query<&mut PreEventScriptHooks>,
    mut post_hook_query: Query<&mut PostEventScriptHooks>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ScriptAttach(ScriptAttach {
            entity,
            script,
            pre,
            target,
        }) = event
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

            let trigger = script_query.get(script_entity).unwrap().trigger;

            let hook = ScriptHook {
                trigger,
                script: script.clone(),
            };

            if *pre {
                if let Ok(mut hooks) = pre_hook_query.get_mut(target_entity) {
                    if hooks.list.contains(&hook) {
                        if let Ok(mut messages) = messages_query.get_mut(*entity) {
                            messages
                                .queue(format!("Script {} already attached to entity.", script));
                        }
                        continue;
                    }
                    hooks.list.push(hook);
                } else {
                    commands
                        .entity(target_entity)
                        .insert(PreEventScriptHooks { list: vec![hook] });
                }
            } else if let Ok(mut hooks) = post_hook_query.get_mut(target_entity) {
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
                    .insert(PostEventScriptHooks { list: vec![hook] });
            }

            let id = match target {
                Either::Left(id) => *id,
                Either::Right(_) => Id::Player(player_query.get(target_entity).unwrap().id),
            };

            updates.queue(persist::script::Attach::new(
                id,
                *pre,
                script.clone(),
                trigger,
            ));

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

event_from_action!(ScriptDetach);

pub fn script_detach_system(
    mut events: EventReader<ActionEvent>,
    scripts: Res<Scripts>,
    objects: Res<Objects>,
    rooms: Res<Rooms>,
    players: Res<Players>,
    mut updates: ResMut<Updates>,
    script_query: Query<&Script>,
    player_query: Query<&Player>,
    mut pre_hook_query: Query<&mut PreEventScriptHooks>,
    mut post_hook_query: Query<&mut PostEventScriptHooks>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ScriptDetach(ScriptDetach {
            entity,
            script,
            target,
        }) = event
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

            let trigger = script_query.get(script_entity).unwrap().trigger;

            let mut pre = false;
            let mut removed = false;

            if let Ok(mut hooks) = pre_hook_query.get_mut(target_entity) {
                if hooks.remove(&trigger, script) {
                    pre = true;
                    removed = true;
                }
            }

            if !removed {
                if let Ok(mut hooks) = post_hook_query.get_mut(target_entity) {
                    if hooks.remove(&trigger, script) {
                        removed = true;
                    }
                }
            }

            if !removed {
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
                pre,
                script.clone(),
                trigger,
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
