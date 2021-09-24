use std::str::FromStr;

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    text::{word_list, Tokenizer},
    world::{
        action::{into_action, Action},
        types::{
            object::{Flags, Keywords, ObjectFlags},
            player::{Messages, Player},
            room::{Direction, Room},
            Contents, Description, Location, Named,
        },
    },
};

pub fn parse_look(player: Entity, mut tokenizer: Tokenizer) -> Result<Action, String> {
    match tokenizer.next() {
        Some(token) => {
            if token == "at" {
                if tokenizer.rest().is_empty() {
                    return Err("Look at what?".to_string());
                }

                let keywords = tokenizer
                    .rest()
                    .split_whitespace()
                    .map(ToString::to_string)
                    .collect_vec();

                Ok(Action::from(LookAt {
                    actor: player,
                    keywords,
                }))
            } else if let Ok(direction) = Direction::from_str(token) {
                Ok(Action::from(Look {
                    actor: player,
                    direction: Some(direction),
                }))
            } else {
                Err(format!("I don't know how to look {}.", token))
            }
        }
        None => Ok(Action::from(Look {
            actor: player,
            direction: None,
        })),
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Look {
    pub actor: Entity,
    pub direction: Option<Direction>,
}

into_action!(Look);

pub fn look_system(
    mut action_reader: EventReader<Action>,
    looker_query: Query<&Location, With<Player>>,
    room_query: Query<(&Room, &Named, &Description, &Contents)>,
    player_query: Query<&Named>,
    object_query: Query<(&Named, &ObjectFlags)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Look(Look { actor, direction }) = action {
            let current_room = looker_query
                .get(*actor)
                .map(|location| location.room())
                .unwrap();

            let target_room = if let Some(direction) = direction {
                if let Some(room) = room_query
                    .get(current_room)
                    .map(|(room, _, _, _)| room.exit(direction))
                    .expect("Location has a valid room.")
                {
                    room
                } else {
                    if let Ok(mut messages) = messages_query.get_mut(*actor) {
                        messages.queue(format!("There is no room {}.", direction.as_to_str()));
                    }
                    continue;
                }
            } else {
                current_room
            };

            let (room, named, description, contents) = room_query.get(target_room).unwrap();

            let mut message = format!("|white|{}|-|\r\n", named.as_str());

            message.push_str(description.as_str());

            let present_names = room
                .players()
                .iter()
                .filter(|present_player| **present_player != *actor)
                .filter_map(|player| player_query.get(*player).ok())
                .map(|named| named.to_string())
                .sorted()
                .collect_vec();

            if !present_names.is_empty() {
                message.push_str("\r\n");

                let singular = present_names.len() == 1;

                let mut player_list = word_list(present_names);
                if singular {
                    player_list.push_str(" is here.");
                } else {
                    player_list.push_str(" are here.");
                };
                message.push_str(player_list.as_str());
            }

            let objects = contents
                .objects()
                .iter()
                .filter_map(|object| object_query.get(*object).ok())
                .filter(|(_, flags)| !flags.contains(Flags::SUBTLE))
                .map(|(named, _)| named.to_string())
                .collect_vec();

            if !objects.is_empty() {
                message.push_str("\r\nYou see ");
                message.push_str(word_list(objects).as_str());
                message.push('.');
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LookAt {
    pub actor: Entity,
    pub keywords: Vec<String>,
}

into_action!(LookAt);

pub fn look_at_system(
    mut action_reader: EventReader<Action>,
    looker_query: Query<&Location, With<Player>>,
    room_query: Query<&Room>,
    contents_query: Query<&Contents>,
    player_query: Query<(&Named, &Description)>,
    object_query: Query<(&Description, &Keywords)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::LookAt(LookAt { actor, keywords }) = action {
            let description = looker_query
                .get(*actor)
                .ok()
                .map(|location| location.room())
                .and_then(|room| room_query.get(room).ok())
                .and_then(|room| {
                    if keywords.len() == 1 {
                        room.players()
                            .iter()
                            .filter_map(|player| player_query.get(*player).ok())
                            .find(|(name, _)| keywords[0].as_str() == name.as_str())
                            .map(|(_, description)| description.as_str())
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    looker_query
                        .get(*actor)
                        .ok()
                        .map(|location| location.room())
                        .and_then(|room| contents_query.get(room).ok())
                        .and_then(|contents| {
                            contents
                                .objects()
                                .iter()
                                .filter_map(|object| object_query.get(*object).ok())
                                .find(|(_, object_keywords)| {
                                    object_keywords.contains_all(keywords.as_slice())
                                })
                                .map(|(description, _)| description.as_str())
                        })
                })
                .or_else(|| {
                    contents_query.get(*actor).ok().and_then(|contents| {
                        contents
                            .objects()
                            .iter()
                            .filter_map(|object| object_query.get(*object).ok())
                            .find(|(_, object_keywords)| {
                                object_keywords.contains_all(keywords.as_slice())
                            })
                            .map(|(description, _)| description.as_str())
                    })
                });

            let message = if let Some(description) = description {
                description.to_string()
            } else {
                format!(
                    "You find nothing called \"{}\" to look at.",
                    word_list(keywords.clone())
                )
            };

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Exits {
    pub actor: Entity,
}

into_action!(Exits);

pub fn exits_system(
    mut action_reader: EventReader<Action>,
    exiter_query: Query<&Location, With<Player>>,
    room_query: Query<&Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Exits(Exits { actor }) = action {
            let current_room = exiter_query
                .get(*actor)
                .map(|location| location.room())
                .unwrap();

            let exits = room_query
                .get(current_room)
                .unwrap()
                .exits()
                .keys()
                .map(Direction::as_str)
                .map(ToString::to_string)
                .sorted()
                .collect_vec();

            let message = if exits.is_empty() {
                "This room has no obvious exits.".to_string()
            } else if exits.len() == 1 {
                format!("There is an exit {}.", word_list(exits))
            } else {
                format!("There are exits {}.", word_list(exits))
            };

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Who {
    pub actor: Entity,
}

into_action!(Who);

pub fn who_system(
    mut action_reader: EventReader<Action>,
    player_query: Query<&Named, With<Player>>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Who(Who { actor }) = action {
            let players = player_query
                .iter()
                .map(|named| format!("  {}", named.as_str()))
                .sorted()
                .join("\r\n");

            let message = format!("Online players:\r\n{}", players);

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}
