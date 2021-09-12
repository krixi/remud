use std::str::FromStr;

use anyhow::bail;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::{word_list, Tokenizer},
    world::{
        action::{
            queue_message, Action, DynAction, DEFAULT_OBJECT_KEYWORD, DEFAULT_OBJECT_LONG,
            DEFAULT_OBJECT_SHORT,
        },
        types::{
            object::{self, Object, Objects},
            player::Player,
            room::Room,
            Contents,
        },
    },
};

// Valid shapes:
// object <id> info - displays information about the object
// object new - creates a new object and puts it on the ground
// object <id> keywords - sets an object's keywords
// object <id> short - sets an object's short description
// object <id> long - sets an object's long description
// object <id> remove - removes an object
pub fn parse(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(token) = tokenizer.next() {
        match token {
            "new" => Ok(Box::new(Create {})),
            maybe_id => {
                let id = match object::Id::from_str(maybe_id) {
                    Ok(id) => id,
                    Err(e) => return Err(e.to_string()),
                };

                if let Some(token) = tokenizer.next() {
                    match token {
                        "info" => Ok(Box::new(Info { id })),
                        "keywords" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a comma separated list of keywords.".to_string())
                            } else {
                                let keywords = tokenizer
                                    .rest()
                                    .split(',')
                                    .map(|keyword| keyword.trim().to_string())
                                    .collect_vec();

                                Ok(Box::new(Update {
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
                                Ok(Box::new(Update {
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
                                Ok(Box::new(Update {
                                    id,
                                    keywords: None,
                                    short: None,
                                    long: Some(tokenizer.rest().to_string()),
                                }))
                            }
                        }
                        "remove" => Ok(Box::new(Remove { id })),
                        _ => Err("Enter a valid object subcommand: info, keywords, short, long, or remove."
                            .to_string()),
                    }
                } else {
                    Err(
                        "Enter an object subcommand: info, keywords, short, long, or remove."
                            .to_string(),
                    )
                }
            }
        }
    } else {
        Err("Enter an object ID or subcommand: new.".to_string())
    }
}

struct Create {}

impl Action for Create {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let room_entity = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("{:?} has no Player.", player),
        };

        let id = world.get_resource_mut::<Objects>().unwrap().next_id();
        let object_entity = world
            .spawn()
            .insert(Object {
                id,
                container: room_entity,
                keywords: vec![DEFAULT_OBJECT_KEYWORD.to_string()],
                short: DEFAULT_OBJECT_SHORT.to_string(),
                long: DEFAULT_OBJECT_LONG.to_string(),
            })
            .id();

        match world.get_mut::<Contents>(room_entity) {
            Some(mut contents) => contents.objects.push(object_entity),
            None => bail!("{:?} has no Contents.", room_entity),
        }

        world
            .get_resource_mut::<Objects>()
            .unwrap()
            .insert(id, object_entity);

        let room_id = match world.get::<Room>(room_entity).map(|room| room.id) {
            Some(id) => id,
            None => bail!("{:?} has no Room", room_entity),
        };

        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(persist::object::New::new(id));
        updates.queue(persist::room::AddObject::new(room_id, id));

        let message = format!("Created object {}.", id);
        queue_message(world, player, message);

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
            None => bail!("{:?} has no Contents.", player),
        };

        let message = if let Some(pos) = pos {
            let (player_id, room_entity) = match world.get::<Player>(player) {
                Some(player) => (player.id, player.room),
                None => bail!("{:?} has no Player.", player),
            };

            let object_entity = world
                .get_mut::<Contents>(player)
                .unwrap()
                .objects
                .remove(pos);
            match world.get_mut::<Contents>(room_entity) {
                Some(mut contents) => contents.objects.push(object_entity),
                None => bail!("{:?} has no Contents.", room_entity),
            }

            let object_id = {
                let mut object = world.get_mut::<Object>(object_entity).unwrap();
                object.container = room_entity;
                object.id
            };

            let room_id = match world.get::<Room>(room_entity).map(|room| room.id) {
                Some(id) => id,
                None => bail!("{:?} has no Room", room_entity),
            };

            let mut updates = world.get_resource_mut::<Updates>().unwrap();
            updates.queue(persist::player::RemoveObject::new(player_id, object_id));
            updates.queue(persist::room::AddObject::new(room_id, object_id));

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
        let (player_id, room_entity) = match world.get::<Player>(player) {
            Some(player) => (player.id, player.room),
            None => bail!("{:?} has no Player.", player),
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
            None => bail!("{:?} has no Contents.", player),
        };

        let message = if let Some(pos) = pos {
            let object_entity = world
                .get_mut::<Contents>(room_entity)
                .unwrap()
                .objects
                .remove(pos);
            match world.get_mut::<Contents>(player) {
                Some(mut contents) => contents.objects.push(object_entity),
                None => bail!("{:?} has no Contents.", object_entity),
            }

            let object_id = {
                let mut object = world.get_mut::<Object>(object_entity).unwrap();
                object.container = player;
                object.id
            };

            let room_id = match world.get::<Room>(room_entity).map(|room| room.id) {
                Some(id) => id,
                None => bail!("{:?} has no Room", room_entity),
            };

            let mut updates = world.get_resource_mut::<Updates>().unwrap();
            updates.queue(persist::room::RemoveObject::new(room_id, object_id));
            updates.queue(persist::player::AddObject::new(player_id, object_id));

            format!("You pick up \"{}\".", self.keywords.join(" "))
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

struct Info {
    id: object::Id,
}

impl Action for Info {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let object_entity = match world.get_resource::<Objects>().unwrap().by_id(self.id) {
            Some(entity) => entity,
            None => {
                let message = format!("Object {} does not exist.", self.id);
                queue_message(world, player, message);
                return Ok(());
            }
        };

        let object = match world.get::<Object>(object_entity) {
            Some(object) => object,
            None => bail!("{:?} has no Object.", object_entity),
        };

        let mut message = format!("Object {}", self.id);
        message.push_str("\r\n  keywords: ");
        message.push_str(word_list(object.keywords.clone()).as_str());
        message.push_str("\r\n  short: ");
        message.push_str(object.short.as_str());
        message.push_str("\r\n  long: ");
        message.push_str(object.long.as_str());
        message.push_str("\r\n  container: ");
        if let Some(room) = world.get::<Room>(object.container) {
            message.push_str("room ");
            message.push_str(room.id.to_string().as_str());
        } else if let Some(player) = world.get::<Player>(object.container) {
            message.push_str("player ");
            message.push_str(player.name.as_str());
        } else {
            message.push_str(format!("{:?}", object.container).as_str());
        }

        queue_message(world, player, message);

        Ok(())
    }
}

#[derive(Default)]
pub struct Inventory {}

impl Action for Inventory {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let mut message = "You have".to_string();

        match world.get::<Contents>(player) {
            Some(contents) => {
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
            }
            None => bail!("{:?} has no Contents.", player),
        }

        queue_message(world, player, message);

        Ok(())
    }
}

struct Update {
    id: object::Id,
    keywords: Option<Vec<String>>,
    short: Option<String>,
    long: Option<String>,
}

impl Action for Update {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let object_entity =
            if let Some(entity) = world.get_resource::<Objects>().unwrap().by_id(self.id) {
                entity
            } else {
                let message = format!("Object {} not found.", self.id);
                queue_message(world, player, message);
                return Ok(());
            };

        let (id, keywords, short, long) = match world.get_mut::<Object>(object_entity) {
            Some(mut object) => {
                if self.keywords.is_some() {
                    object.keywords = self.keywords.take().unwrap();
                }
                if self.short.is_some() {
                    object.short = self.short.take().unwrap();
                }
                if self.long.is_some() {
                    object.long = self.long.take().unwrap();
                }

                (
                    object.id,
                    object.keywords.clone(),
                    object.short.clone(),
                    object.long.clone(),
                )
            }
            None => bail!("{:?} has no Object.", object_entity),
        };

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::object::Update::new(id, keywords, short, long));

        let message = format!("Updated object {}.", self.id);
        queue_message(world, player, message);

        Ok(())
    }
}

struct Remove {
    id: object::Id,
}

impl Action for Remove {
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
            None => bail!("{:?} has no Object", object_entity),
        };

        world.despawn(object_entity);
        match world.get_mut::<Contents>(container) {
            Some(mut room) => room.remove(object_entity),
            None => bail!("{:?} has no Contents.", container),
        }

        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(persist::object::Remove::new(self.id));

        let message = format!("Object {} removed.", self.id);
        queue_message(world, player, message);

        Ok(())
    }
}
