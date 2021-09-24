use std::convert::TryFrom;

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    into_action,
    text::Tokenizer,
    world::{
        action::{
            immortal::{Initialize, ShowError},
            Action,
        },
        scripting::{
            time::Timers, ExecutionErrors, ScriptData, ScriptHook, ScriptHooks, ScriptName,
        },
        types::{
            object::Object,
            player::{self, Messages, Player, PlayerFlags, Players},
            room::Room,
            ActionTarget, Contents, Description, Location, Named,
        },
    },
};

// Valid shapes:
// player <name> info - displays information about the player
pub fn parse_player(player: Entity, mut tokenizer: Tokenizer) -> Result<Action, String> {
    if let Some(name) = tokenizer.next() {
        if let Some(token) = tokenizer.next() {
            match token {
                "error" => {
                    if tokenizer.rest().is_empty() {
                        Err("Enter a script to look for its errors.".to_string())
                    } else {
                        let script = ScriptName::try_from(tokenizer.next().unwrap().to_string())
                            .map_err(|e| e.to_string())?;
                        Ok(Action::from(ShowError {
                            actor: player,
                            target: ActionTarget::Player(name.to_string()),
                            script,
                        }))
                    }
                }
                "info" => Ok(Action::from(PlayerInfo {
                    actor: player,
                    name: name.to_string(),
                })),
                "init" => Ok(Action::from(Initialize {
                    actor: player,
                    target: ActionTarget::Player(name.to_string()),
                })),
                "set" => {
                    if tokenizer.rest().is_empty() {
                        Err(
                            "Enter a space separated list of flags. Valid flags: immortal."
                                .to_string(),
                        )
                    } else {
                        Ok(Action::from(PlayerUpdateFlags {
                            actor: player,
                            name: name.to_string(),
                            flags: tokenizer
                                .rest()
                                .to_string()
                                .split_whitespace()
                                .map(|flag| flag.to_string())
                                .collect_vec(),
                            clear: false,
                        }))
                    }
                }
                "unset" => {
                    if tokenizer.rest().is_empty() {
                        Err(
                            "Enter a space separated list of flags. Valid flags: immortal."
                                .to_string(),
                        )
                    } else {
                        Ok(Action::from(PlayerUpdateFlags {
                            actor: player,
                            name: name.to_string(),
                            flags: tokenizer
                                .rest()
                                .to_string()
                                .split_whitespace()
                                .map(|flag| flag.to_string())
                                .collect_vec(),
                            clear: true,
                        }))
                    }
                }
                _ => Err("Enter a valid player subcommand: info.".to_string()),
            }
        } else {
            Err("Enter a player subcommand: info.".to_string())
        }
    } else {
        Err("Enter a player name.".to_string())
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PlayerInfo {
    pub actor: Entity,
    pub name: String,
}

into_action!(PlayerInfo);

pub fn player_info_system(
    mut action_reader: EventReader<Action>,
    players: Res<Players>,
    player_query: Query<(
        &Player,
        &PlayerFlags,
        &Description,
        &Contents,
        &Location,
        Option<&ScriptHooks>,
        Option<&Timers>,
        Option<&ScriptData>,
        Option<&ExecutionErrors>,
    )>,
    room_query: Query<(&Room, &Named)>,
    object_query: Query<(&Object, &Named)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::PlayerInfo(PlayerInfo { actor, name }) = action {
            let player = if let Some(entity) = players.by_name(name) {
                entity
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Player '{}' not found.", name))
                }
                continue;
            };

            let (player, flags, description, contents, location, hooks, timers, data, errors) =
                player_query.get(player).unwrap();
            let (room, room_name) = room_query.get(location.room()).unwrap();

            let mut message = format!("|white|Player {}|-|", name);

            message.push_str("\r\n  |white|id|-|: ");
            message.push_str(player.id().to_string().as_str());

            message.push_str("\r\n  |white|description|-|: ");
            message.push_str(description.as_str());

            message.push_str("\r\n  |white|flags|-|: ");
            message.push_str(format!("{:?}", flags.get_flags()).as_str());

            message.push_str("\r\n  |white|room|-|: ");
            message.push_str(format!("{} (room {})", room_name.as_str(), room.id()).as_str());

            message.push_str("\r\n  |white|inventory|-|:");
            contents
                .objects()
                .iter()
                .filter_map(|object| {
                    object_query
                        .get(*object)
                        .map(|(object, named)| (object.id(), named.as_str()))
                        .ok()
                })
                .for_each(|(id, name)| {
                    message.push_str(format!("\r\n    object {}: {}", id, name).as_str())
                });

            message.push_str("\r\n  |white|script hooks|-|:");
            if let Some(hooks) = hooks {
                if hooks.is_empty() {
                    message.push_str(" none");
                }
                for ScriptHook { trigger, script } in hooks.hooks().iter() {
                    message.push_str(format!("\r\n    {:?} -> {}", trigger, script).as_str());

                    if errors.map(|e| e.has_error(script)).unwrap_or(false) {
                        message.push_str(" |red|(error)|-|");
                    }
                }
            } else {
                message.push_str(" none");
            }

            message.push_str("\r\n  |white|script data|-|:");
            if let Some(data) = data {
                if data.is_empty() {
                    message.push_str(" none");
                } else {
                    for (k, v) in data.map() {
                        message.push_str(format!("\r\n    {} -> {:?}", k, v).as_str());
                    }
                }
            } else {
                message.push_str(" none");
            }

            message.push_str("\r\n  |white|timers|-|:");
            if let Some(timers) = timers {
                if timers.timers().is_empty() {
                    message.push_str(" none");
                }
                for (name, timer) in timers.timers().iter() {
                    message.push_str(
                        format!(
                            "\r\n    {}: {}/{}ms",
                            name,
                            timer.elapsed().as_millis(),
                            timer.duration().as_millis()
                        )
                        .as_str(),
                    )
                }
            } else {
                message.push_str(" none");
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PlayerUpdateFlags {
    pub actor: Entity,
    pub name: String,
    pub flags: Vec<String>,
    pub clear: bool,
}

into_action!(PlayerUpdateFlags);

pub fn player_update_flags_system(
    mut action_reader: EventReader<Action>,
    players: Res<Players>,
    mut updates: ResMut<Updates>,
    mut player_query: Query<(&Player, &mut PlayerFlags)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::PlayerUpdateFlags(PlayerUpdateFlags {
            actor,
            name,
            flags,
            clear,
        }) = action
        {
            let player_entity = if let Some(player) = players.by_name(name.as_str()) {
                player
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Player {} not found.", name));
                }
                continue;
            };

            let changed_flags = match player::Flags::try_from(flags.as_slice()) {
                Ok(flags) => flags,
                Err(e) => {
                    if let Ok(mut messages) = messages_query.get_mut(*actor) {
                        messages.queue(e.to_string());
                    }
                    continue;
                }
            };

            let (player, mut flags) = player_query.get_mut(player_entity).unwrap();

            if *clear {
                flags.remove(changed_flags);
            } else {
                flags.insert(changed_flags);
            }

            updates.persist(persist::player::Flags::new(player.id(), flags.get_flags()));

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Updated player {} flags.", name));
            }
        }
    }
}
