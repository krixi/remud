use std::str::FromStr;

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::world::action::targeting::{Params, Target, TargetFinder};
use crate::{
    text::{sorted_word_list, Tokenizer},
    world::{
        action::{get_room_std, into_action, Action},
        types::{
            object::{Flags, ObjectFlags},
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

#[tracing::instrument(name = "look system", skip_all)]
pub fn look_system(
    mut action_reader: EventReader<Action>,
    looker_query: Query<(Option<&Location>, Option<&Room>)>,
    room_query: Query<(&Room, &Named, &Description, &Contents)>,
    player_query: Query<&Named>,
    object_query: Query<(&Named, &ObjectFlags)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Look(Look { actor, direction }) = action {
            let current_room = get_room_std(*actor, &looker_query);

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

                let mut player_list = sorted_word_list(present_names);
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
                message.push_str(sorted_word_list(objects).as_str());
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

#[tracing::instrument(name = "look at system", skip_all)]
pub fn look_at_system(
    mut action_reader: EventReader<Action>,
    looker_query: Query<(Option<&Location>, Option<&Room>)>,
    target_finder: TargetFinder,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::LookAt(LookAt { actor, keywords }) = action {
            let current_room = get_room_std(*actor, &looker_query);

            let target_info =
                target_finder.resolve(Params::new(*actor, current_room, Some(keywords.clone())));

            let resp: Vec<String> = if let Some(Target { name, desc, .. }) = target_info {
                vec![format!("|white|{}|-|", name), desc.to_string()]
            } else {
                vec![format!(
                    "You find nothing called \"{}\" to look at.",
                    sorted_word_list(keywords.clone())
                )]
            };

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                for message in resp {
                    messages.queue(message);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Exits {
    pub actor: Entity,
}

into_action!(Exits);

#[tracing::instrument(name = "exits system", skip_all)]
pub fn exits_system(
    mut action_reader: EventReader<Action>,
    exiter_query: Query<(Option<&Location>, Option<&Room>)>,
    room_query: Query<&Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Exits(Exits { actor }) = action {
            let current_room = get_room_std(*actor, &exiter_query);

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
                format!("There is an exit {}.", sorted_word_list(exits))
            } else {
                format!("There are exits {}.", sorted_word_list(exits))
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

#[tracing::instrument(name = "who system", skip_all)]
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
