use std::str::FromStr;

use anyhow::bail;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::Tokenizer,
    world::{
        action::{queue_message, Action, DynAction},
        types::{
            object::{self, Object, Objects},
            player::Player,
            Contents,
        },
    },
};

pub fn parse(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(token) = tokenizer.next() {
        match token {
            "new" => Ok(Box::new(CreateObject {})),
            maybe_id => {
                if let Ok(id) = object::Id::from_str(maybe_id) {
                    if let Some(token) = tokenizer.next() {
                        match token {
                            "keywords" => {
                                if tokenizer.rest().is_empty() {
                                    Err("Enter a comma separated list of keywords.".to_string())
                                } else {
                                    let keywords = tokenizer
                                        .rest()
                                        .split(',')
                                        .map(|keyword| keyword.trim().to_string())
                                        .collect_vec();

                                    Ok(Box::new(UpdateObject {
                                        id,
                                        keywords: Some(keywords),
                                        short: None,
                                        long: None,
                                    }))
                                }
                            }
                            "short" => {
                                if tokenizer.rest().is_empty() {
                                    Err("Enter a short description.".to_string())
                                } else {
                                    Ok(Box::new(UpdateObject {
                                        id,
                                        keywords: None,
                                        short: Some(tokenizer.rest().to_string()),
                                        long: None,
                                    }))
                                }
                            }
                            "long" => {
                                if tokenizer.rest().is_empty() {
                                    Err("Enter a long description.".to_string())
                                } else {
                                    Ok(Box::new(UpdateObject {
                                        id,
                                        keywords: None,
                                        short: None,
                                        long: Some(tokenizer.rest().to_string()),
                                    }))
                                }
                            }
                            "remove" => Ok(Box::new(RemoveObject { id })),
                            _ => Err("Enter a valid object subcommand: keywords, short, long, or remove."
                                .to_string()),
                        }
                    } else {
                        Err(
                            "Enter a valid object subcommand: keywords, short, long, or remove."
                                .to_string(),
                        )
                    }
                } else {
                    Err("Enter a valid object ID or subcommand: new.".to_string())
                }
            }
        }
    } else {
        Err("Enter a valid object ID or subcommand: new.".to_string())
    }
}

struct CreateObject {}

impl Action for CreateObject {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let room_entity = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("Player {:?} has no Location."),
        };

        let id = world.get_resource_mut::<Objects>().unwrap().next_id();
        let object_entity = world
            .spawn()
            .insert(Object {
                id,
                container: room_entity,
                keywords: vec!["object".to_string()],
                short: "An object.".to_string(),
                long: "A nondescript object. Completely uninteresting.".to_string(),
            })
            .id();

        if let Some(mut contents) = world.get_mut::<Contents>(room_entity) {
            contents.objects.push(object_entity);
        }

        world
            .get_resource_mut::<Objects>()
            .unwrap()
            .insert(id, object_entity);

        let message = format!("Created object {}.", id);
        queue_message(world, player, message);

        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(persist::object::New::new(object_entity));
        updates.queue(persist::room::AddObject::new(room_entity, object_entity));

        Ok(())
    }
}

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
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let pos = match world.get::<Contents>(player) {
            Some(contents) => contents.objects.iter().position(|object| {
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
            }),
            None => bail!("Player {:?} does not have Contents.", player),
        };

        let message = if let Some(pos) = pos {
            let room_entity = match world.get::<Player>(player) {
                Some(player) => player.room,
                None => bail!("Player {:?} does not have a Player.", player),
            };

            let object_entity = world
                .get_mut::<Contents>(player)
                .unwrap()
                .objects
                .remove(pos);
            match world.get_mut::<Contents>(room_entity) {
                Some(mut contents) => contents.objects.push(object_entity),
                None => bail!("Room {:?} does not have Contents.", room_entity),
            }
            world.get_mut::<Object>(object_entity).unwrap().container = room_entity;

            let mut updates = world.get_resource_mut::<Updates>().unwrap();
            updates.queue(persist::room::AddObject::new(room_entity, object_entity));
            updates.queue(persist::player::RemoveObject::new(player, object_entity));

            format!("You drop \"{}\".", self.keywords.join(" "))
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
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let room_entity = match world.get::<Player>(player) {
            Some(player) => player.room,
            None => bail!("Player {:?} does not have a Player.", player),
        };

        let pos = match world.get::<Contents>(room_entity) {
            Some(contents) => contents.objects.iter().position(|object| {
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
            }),
            None => bail!("Player {:?} does not have Contents.", player),
        };

        let message = if let Some(pos) = pos {
            let object_entity = world
                .get_mut::<Contents>(room_entity)
                .unwrap()
                .objects
                .remove(pos);
            match world.get_mut::<Contents>(player) {
                Some(mut contents) => contents.objects.push(object_entity),
                None => bail!("Player {:?} does not have Contents.", object_entity),
            }
            world.get_mut::<Object>(object_entity).unwrap().container = player;

            let mut updates = world.get_resource_mut::<Updates>().unwrap();
            updates.queue(persist::room::RemoveObject::new(room_entity, object_entity));
            updates.queue(persist::player::AddObject::new(player, object_entity));

            format!("You pick up \"{}\".", self.keywords.join(" "))
        } else {
            format!("You find no \"{}\" here.", self.keywords.join(" "))
        };

        queue_message(world, player, message);

        Ok(())
    }
}

#[derive(Default)]
pub struct Inventory {}

impl Action for Inventory {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let mut message = "You have:".to_string();

        match world.get::<Contents>(player) {
            Some(contents) => {
                if contents.objects.is_empty() {
                    message.push_str(" nothing.");
                } else {
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
            }
            None => bail!("Player {:?} does not have Contents.", player),
        }

        queue_message(world, player, message);

        Ok(())
    }
}

struct UpdateObject {
    id: object::Id,
    keywords: Option<Vec<String>>,
    short: Option<String>,
    long: Option<String>,
}

impl Action for UpdateObject {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let object_entity =
            if let Some(entity) = world.get_resource::<Objects>().unwrap().by_id(self.id) {
                entity
            } else {
                let message = format!("Object {} not found.", self.id);
                queue_message(world, player, message);
                return Ok(());
            };

        if let Some(mut object) = world.get_mut::<Object>(object_entity) {
            if self.keywords.is_some() {
                object.keywords = self.keywords.take().unwrap();
            }
            if self.short.is_some() {
                object.short = self.short.take().unwrap();
            }
            if self.long.is_some() {
                object.long = self.long.take().unwrap();
            }
        }

        let message = format!("Updated object {}.", self.id);
        queue_message(world, player, message);

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::object::Update::new(object_entity));

        Ok(())
    }
}

struct RemoveObject {
    id: object::Id,
}

impl Action for RemoveObject {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let object_entity = match world.get_resource::<Objects>().unwrap().by_id(self.id) {
            Some(entity) => entity,
            None => bail!("Unable to find object by ID: {}", self.id),
        };

        let container = match world
            .get::<Object>(object_entity)
            .map(|object| object.container)
        {
            Some(container) => container,
            None => bail!("Object {:?} does not have Object", object_entity),
        };

        world.despawn(object_entity);
        match world.get_mut::<Contents>(container) {
            Some(mut room) => room.remove(object_entity),
            None => bail!("Container {:?} does not have Contents.", container),
        }

        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(persist::object::Remove::new(self.id));

        let message = format!("Object {} removed.", self.id);
        queue_message(world, player, message);

        Ok(())
    }
}
