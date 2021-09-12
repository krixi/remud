use std::str::FromStr;

use bevy_ecs::prelude::{Entity, World};
use itertools::Itertools;

use crate::{
    text::{word_list, Tokenizer},
    world::{
        action::{self, queue_message, Action, DynAction},
        types::{
            self,
            object::Object,
            player::Player,
            room::{Direction, Room},
            Contents,
        },
    },
};

pub fn parse_look(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
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

                Ok(Box::new(Look {
                    at: Some(keywords),
                    direction: None,
                }))
            } else if let Ok(direction) = Direction::from_str(token) {
                Ok(Box::new(Look {
                    at: None,
                    direction: Some(direction),
                }))
            } else {
                Err(format!("I don't know how to look {}.", token))
            }
        }
        None => Ok(Box::new(Look {
            at: None,
            direction: None,
        })),
    }
}

#[derive(Default)]
pub struct Look {
    at: Option<Vec<String>>,
    direction: Option<Direction>,
}

impl Look {
    pub fn here() -> Box<Self> {
        Box::new(Look::default())
    }
}

impl Look {
    fn look_room(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let current_room = world
            .get::<Player>(player)
            .map(|player| player.room)
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let look_target = if let Some(direction) = &self.direction {
            if let Some(room) = world
                .get::<Room>(current_room)
                .and_then(|room| room.exits.get(direction))
            {
                *room
            } else {
                let message = format!("There is no room {}.", direction.as_to_str());
                queue_message(world, player, message);
                return Ok(());
            }
        } else {
            current_room
        };

        let room = world
            .get::<Room>(look_target)
            .ok_or(action::Error::MissingComponent(look_target, "Room"))?;
        {
            let mut message = room.description.clone();

            let present_names = room
                .players
                .iter()
                .filter(|present_player| **present_player != player)
                .filter_map(|player| world.get::<Player>(*player))
                .map(|player| player.name.clone())
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

            if let Some(contents) = world.get::<Contents>(look_target) {
                let objects = contents
                    .objects
                    .iter()
                    .filter_map(|object| world.get::<Object>(*object))
                    .filter(|object| !object.flags.contains(types::object::Flags::SUBTLE))
                    .map(|object| object.short.clone())
                    .collect_vec();

                if !objects.is_empty() {
                    message.push_str("\r\nYou see ");
                    message.push_str(word_list(objects).as_str());
                    message.push('.');
                }
            }

            queue_message(world, player, message);
        }

        Ok(())
    }

    fn look_object(&mut self, player: Entity, world: &mut World) {
        let description = world
            .get::<Player>(player)
            .map(|player| player.room)
            .and_then(|room| world.get::<Contents>(room))
            .or_else(|| world.get::<Contents>(player))
            .and_then(|contents| {
                contents
                    .objects
                    .iter()
                    .filter_map(|object| world.get::<Object>(*object))
                    .find(|object| {
                        self.at
                            .as_ref()
                            .unwrap()
                            .iter()
                            .all(|keyword| object.keywords.contains(keyword))
                    })
                    .map(|object| object.long.as_str())
            });

        let message = if let Some(description) = description {
            description.to_string()
        } else {
            format!(
                "You find nothing called \"{}\" to look at.",
                word_list(self.at.as_ref().unwrap().clone())
            )
        };
        queue_message(world, player, message);
    }
}

impl Action for Look {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        if self.at.is_some() {
            self.look_object(player, world);
        } else {
            self.look_room(player, world)?;
        }

        Ok(())
    }
}

pub struct Exits {}

impl Action for Exits {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let room = world
            .get::<Player>(player)
            .map(|player| player.room)
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let room = world
            .get::<Room>(room)
            .ok_or(action::Error::MissingComponent(room, "Room"))?;

        let exits = room
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

        queue_message(world, player, message);

        Ok(())
    }
}

pub struct Who {}

impl Action for Who {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let players = world
            .query::<&Player>()
            .iter(world)
            .map(|player| format!("  {}", player.name))
            .sorted()
            .join("\r\n");

        let message = format!("Online players:\r\n{}", players);
        queue_message(world, player, message);
        Ok(())
    }
}
