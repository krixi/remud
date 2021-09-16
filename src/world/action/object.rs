use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::Tokenizer,
    world::{
        action::{self, queue_message, Action, DynAction},
        types::{self, object::Object, player::Player, room::Room, Contents},
    },
};

pub fn parse_drop(tokenizer: Tokenizer) -> Result<DynAction, String> {
    if tokenizer.rest().is_empty() {
        return Err("Drop what?".to_string());
    }

    let keywords = tokenizer
        .rest()
        .split_whitespace()
        .map(ToString::to_string)
        .collect_vec();

    Ok(Box::new(Drop { keywords }))
}

#[derive(Default)]
pub struct Drop {
    keywords: Vec<String>,
}

impl Action for Drop {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let pos = world
            .get::<Contents>(player)
            .ok_or(action::Error::MissingComponent(player, "Contents"))?
            .objects
            .iter()
            .position(|object| {
                world
                    .get::<Object>(*object)
                    .map(|object| {
                        {
                            self.keywords
                                .iter()
                                .all(|keyword| object.keywords.contains(keyword))
                        }
                    })
                    .unwrap_or(false)
            });

        let message = if let Some(pos) = pos {
            let (player_id, room_entity) = world
                .get::<Player>(player)
                .map(|player| (player.id, player.room))
                .ok_or(action::Error::MissingComponent(player, "Player"))?;

            let object_entity = world
                .get_mut::<Contents>(player)
                .unwrap()
                .objects
                .remove(pos);

            world
                .get_mut::<Contents>(room_entity)
                .ok_or(action::Error::MissingComponent(room_entity, "Contents"))?
                .objects
                .push(object_entity);

            let (object_id, short) = {
                let mut object = world.get_mut::<Object>(object_entity).unwrap();
                object.container = room_entity;
                (object.id, object.short.clone())
            };

            let room_id = world
                .get::<Room>(room_entity)
                .map(|room| room.id)
                .ok_or(action::Error::MissingComponent(room_entity, "Room"))?;

            let mut updates = world.get_resource_mut::<Updates>().unwrap();
            updates.queue(persist::player::RemoveObject::new(player_id, object_id));
            updates.queue(persist::room::AddObject::new(room_id, object_id));

            format!("You drop {}.", short)
        } else {
            format!("You don't have \"{}\".", self.keywords.join(" "))
        };

        queue_message(world, player, message);

        Ok(())
    }
}

pub fn parse_get(tokenizer: Tokenizer) -> Result<DynAction, String> {
    if tokenizer.rest().is_empty() {
        return Err("Get what?".to_string());
    }

    let keywords = tokenizer
        .rest()
        .split_whitespace()
        .map(ToString::to_string)
        .collect_vec();

    Ok(Box::new(Get { keywords }))
}

#[derive(Default)]
pub struct Get {
    keywords: Vec<String>,
}

impl Action for Get {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let (player_id, room_entity) = world
            .get::<Player>(player)
            .map(|player| (player.id, player.room))
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let pos = world
            .get::<Contents>(room_entity)
            .ok_or(action::Error::MissingComponent(room_entity, "Contents"))?
            .objects
            .iter()
            .position(|object| {
                world
                    .get::<Object>(*object)
                    .map(|object| {
                        {
                            self.keywords
                                .iter()
                                .all(|keyword| object.keywords.contains(keyword))
                        }
                    })
                    .unwrap_or(false)
            });

        let message = if let Some(pos) = pos {
            let object_entity = world.get::<Contents>(room_entity).unwrap().objects[pos];

            let (short, fixed) = world
                .get::<Object>(object_entity)
                .map(|object| {
                    (
                        object.short.clone(),
                        object.flags.contains(types::object::Flags::FIXED),
                    )
                })
                .unwrap();

            if fixed {
                let message = format!("Try as you might, you cannot pick up {}.", short);
                queue_message(world, player, message);
                return Ok(());
            }

            world
                .get_mut::<Contents>(room_entity)
                .unwrap()
                .objects
                .remove(pos);

            world
                .get_mut::<Contents>(player)
                .ok_or(action::Error::MissingComponent(player, "Contents"))?
                .objects
                .push(object_entity);

            let object_id = {
                let mut object = world.get_mut::<Object>(object_entity).unwrap();
                object.container = player;
                object.id
            };

            let room_id = world
                .get::<Room>(room_entity)
                .map(|room| room.id)
                .ok_or(action::Error::MissingComponent(room_entity, "Room"))?;

            let mut updates = world.get_resource_mut::<Updates>().unwrap();
            updates.queue(persist::room::RemoveObject::new(room_id, object_id));
            updates.queue(persist::player::AddObject::new(player_id, object_id));

            format!("You pick up {}.", short)
        } else {
            format!(
                "You find no object called \"{}\" to pick up.",
                self.keywords.join(" ")
            )
        };

        queue_message(world, player, message);

        Ok(())
    }
}

#[derive(Default)]
pub struct Inventory {}

impl Action for Inventory {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let mut message = "You have".to_string();

        let contents = world
            .get::<Contents>(player)
            .ok_or(action::Error::MissingComponent(player, "Contents"))?;

        if contents.objects.is_empty() {
            message.push_str(" nothing.");
        } else {
            message.push(':');
            contents
                .objects
                .iter()
                .filter_map(|object| world.get::<Object>(*object))
                .map(|object| object.short.as_str())
                .for_each(|desc| {
                    message.push_str("\r\n  ");
                    message.push_str(desc)
                });
        }

        queue_message(world, player, message);

        Ok(())
    }
}
