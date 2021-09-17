use std::str::FromStr;

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    event_from_action,
    text::{word_list, Tokenizer},
    world::{
        action::ActionEvent,
        types::{
            object::ObjectFlags,
            player::{Messages, Player},
            room::{Direction, Room},
            Contents, Description, Flags, Keywords, Location, Named,
        },
    },
};

pub fn parse_look(player: Entity, mut tokenizer: Tokenizer) -> Result<ActionEvent, String> {
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

                Ok(ActionEvent::from(LookAt {
                    entity: player,
                    keywords,
                }))
            } else if let Ok(direction) = Direction::from_str(token) {
                Ok(ActionEvent::from(Look {
                    entity: player,
                    direction: Some(direction),
                }))
            } else {
                Err(format!("I don't know how to look {}.", token))
            }
        }
        None => Ok(ActionEvent::from(Look {
            entity: player,
            direction: None,
        })),
    }
}

pub struct Look {
    pub entity: Entity,
    pub direction: Option<Direction>,
}

event_from_action!(Look);

pub fn look_system(
    mut events: EventReader<ActionEvent>,
    looker_query: Query<&Location, With<Player>>,
    room_query: Query<(&Room, &Description, &Contents)>,
    player_query: Query<&Named>,
    object_query: Query<(&Named, &Flags)>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Look(Look { entity, direction }) = event {
            let current_room = looker_query
                .get(*entity)
                .map(|location| location.room)
                .unwrap();

            let target_room = if let Some(direction) = direction {
                if let Some(room) = room_query
                    .get(current_room)
                    .map(|(room, _, _)| room.exits.get(direction))
                    .expect("Location has a valid room.")
                {
                    *room
                } else {
                    if let Ok(mut messages) = messages_query.get_mut(*entity) {
                        messages.queue(format!("There is no room {}.", direction.as_to_str()));
                    }
                    continue;
                }
            } else {
                current_room
            };

            let (room, description, contents) = room_query.get(target_room).unwrap();

            let mut message = description.text.clone();

            let present_names = room
                .players
                .iter()
                .filter(|present_player| **present_player != *entity)
                .filter_map(|player| player_query.get(*player).ok())
                .map(|named| named.name.clone())
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
                .objects
                .iter()
                .filter_map(|object| object_query.get(*object).ok())
                .filter(|(_, flags)| !flags.flags.contains(ObjectFlags::SUBTLE))
                .map(|(named, _)| named.name.clone())
                .collect_vec();

            if !objects.is_empty() {
                message.push_str("\r\nYou see ");
                message.push_str(word_list(objects).as_str());
                message.push('.');
            }

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}

pub struct LookAt {
    pub entity: Entity,
    pub keywords: Vec<String>,
}

event_from_action!(LookAt);

pub fn look_at_system(
    mut events: EventReader<ActionEvent>,
    looker_query: Query<&Location, With<Player>>,
    contents_query: Query<&Contents>,
    object_query: Query<(&Description, &Keywords)>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::LookAt(LookAt { entity, keywords }) = event {
            let description = looker_query
                .get(*entity)
                .ok()
                .map(|location| location.room)
                .and_then(|room| contents_query.get(room).ok())
                .and_then(|contents| {
                    contents
                        .objects
                        .iter()
                        .filter_map(|object| object_query.get(*object).ok())
                        .find(|(_, object_keywords)| {
                            keywords
                                .iter()
                                .all(|keyword| object_keywords.list.contains(keyword))
                        })
                        .map(|(description, _)| description.text.as_str())
                })
                .or_else(|| {
                    contents_query.get(*entity).ok().and_then(|contents| {
                        contents
                            .objects
                            .iter()
                            .filter_map(|object| object_query.get(*object).ok())
                            .find(|(_, object_keywords)| {
                                keywords
                                    .iter()
                                    .all(|keyword| object_keywords.list.contains(keyword))
                            })
                            .map(|(description, _)| description.text.as_str())
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

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}

pub struct Exits {
    pub entity: Entity,
}

event_from_action!(Exits);

pub fn exits_system(
    mut events: EventReader<ActionEvent>,
    exiter_query: Query<&Location, With<Player>>,
    room_query: Query<&Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Exits(Exits { entity }) = event {
            let current_room = exiter_query
                .get(*entity)
                .map(|location| location.room)
                .unwrap();

            let exits = room_query
                .get(current_room)
                .unwrap()
                .exits
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

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}

pub struct Who {
    pub entity: Entity,
}

event_from_action!(Who);

pub fn who_system(
    mut events: EventReader<ActionEvent>,
    player_query: Query<&Named, With<Player>>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Who(Who { entity }) = event {
            let players = player_query
                .iter()
                .map(|named| format!("  {}", named.name))
                .sorted()
                .join("\r\n");

            let message = format!("Online players:\r\n{}", players);

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}
