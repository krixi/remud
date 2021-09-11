use std::str::FromStr;

use anyhow::bail;
use bevy_ecs::prelude::{Entity, World};
use itertools::Itertools;

use crate::{
    text::{word_list, Tokenizer},
    world::{
        action::{queue_message, Action, DynAction},
        types::{
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
    fn look_room(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let current_room = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("Player {:?} does not have a Player.", player),
        };

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

        match world.get::<Room>(look_target) {
            Some(room) => {
                let mut message = room.description.clone();

                if let Some(contents) = world.get::<Contents>(look_target) {
                    contents
                        .objects
                        .iter()
                        .filter_map(|object| world.get::<Object>(*object))
                        .for_each(|object| {
                            message.push_str("\r\n");
                            message.push_str(object.short.as_str());
                        });
                }

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

                queue_message(world, player, message);
            }
            None => bail!("Room {:?} has no Room.", look_target),
        };

        Ok(())
    }

    fn look_object(&mut self, player: Entity, world: &mut World) {
        let description = world
            .get::<Player>(player)
            .map(|player| player.room)
            .and_then(|room| world.get::<Contents>(room))
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
                "I didn't find anything {}.",
                word_list(self.at.as_ref().unwrap().clone())
            )
        };
        queue_message(world, player, message);
    }
}

impl Action for Look {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
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
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let room = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("Player {:?} does not have a Location."),
        };

        match world.get::<Room>(room) {
            Some(room) => {
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
            }
            None => bail!("Room {:?} does not have a Room", room),
        }

        Ok(())
    }
}

pub struct Who {}

impl Action for Who {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
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
