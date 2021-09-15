use std::{convert::TryFrom, str::FromStr};

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::{word_list, Tokenizer},
    world::{
        action::{
            self, queue_message, Action, DynAction, DEFAULT_OBJECT_KEYWORD, DEFAULT_OBJECT_LONG,
            DEFAULT_OBJECT_SHORT,
        },
        types::{
            self,
            object::{self, Flags, Object, Objects},
            player::Player,
            room::Room,
            Contents,
        },
        Script, ScriptTriggers, Trigger,
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

                                Ok(Box::new(UpdateKeywords {
                                    id,
                                    keywords,
                                }))
                            }
                        }
                        "long" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a long description.".to_string())
                            } else {
                                Ok(Box::new(UpdateLongDescription {
                                    id,
                                    long: tokenizer.rest().to_string(),
                                }))
                            }
                        }
                        "remove" => Ok(Box::new(Remove { id })),
                        "set" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a space separated list of flags. Valid flags: fixed, subtle.".to_string())
                            } else {
                                Ok(Box::new(SetFlags {id, flags: tokenizer.rest().to_string().split_whitespace().map(|flag|flag.to_string()).collect_vec()}))
                            }
                        }
                        "short" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a short description.".to_string())
                            } else {
                                Ok(Box::new(UpdateShortDescription {
                                    id,
                                    short: tokenizer.rest().to_string(),
                                }))
                            }
                        }
                        "unset" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a space separated list of flags. Valid flags: fixed, subtle.".to_string())
                            } else {
                                Ok(Box::new(ClearFlags {id, flags: tokenizer.rest().to_string().split_whitespace().map(|flag|flag.to_string()).collect_vec()}))
                            }
                        }
                        _ => Err("Enter a valid object subcommand: info, keywords, long, set, short, remove, or unset."
                            .to_string()),
                    }
                } else {
                    Err("Enter an object subcommand: info, keywords, long, set, short, remove, or unset.".to_string())
                }
            }
        }
    } else {
        Err("Enter an object ID or subcommand: new.".to_string())
    }
}

struct ClearFlags {
    id: object::Id,
    flags: Vec<String>,
}

impl Action for ClearFlags {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let object_entity =
            if let Some(entity) = world.get_resource::<Objects>().unwrap().by_id(self.id) {
                entity
            } else {
                let message = format!("Object {} not found.", self.id);
                queue_message(world, player, message);
                return Ok(());
            };

        let remove_flags = match Flags::try_from(self.flags.as_slice()) {
            Ok(flags) => flags,
            Err(e) => {
                queue_message(world, player, e.to_string());
                return Ok(());
            }
        };

        let (id, flags) = {
            let mut object = world
                .get_mut::<Object>(object_entity)
                .ok_or(action::Error::MissingComponent(object_entity, "Object"))?;

            object.flags.remove(remove_flags);

            (object.id, object.flags)
        };

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::object::Flags::new(id, flags));

        let message = format!("Updated object {} flags.", self.id);
        queue_message(world, player, message);

        Ok(())
    }
}
struct Create {}

impl Action for Create {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let room_entity = world
            .get::<Player>(player)
            .map(|player| player.room)
            .ok_or(action::Error::MissingComponent(player, "Player"))?;
        let id = world.get_resource_mut::<Objects>().unwrap().next_id();
        let triggers = ScriptTriggers {
            list: vec![(Trigger::Say, Script("say_hi".to_string()))],
        };
        let object_entity = world
            .spawn()
            .insert(Object {
                id,
                flags: object::Flags::empty(),
                container: room_entity,
                keywords: vec![DEFAULT_OBJECT_KEYWORD.to_string()],
                short: DEFAULT_OBJECT_SHORT.to_string(),
                long: DEFAULT_OBJECT_LONG.to_string(),
            })
            .insert(triggers)
            .id();

        world
            .get_mut::<Contents>(room_entity)
            .ok_or(action::Error::MissingComponent(room_entity, "Contents"))?
            .objects
            .push(object_entity);

        world
            .get_resource_mut::<Objects>()
            .unwrap()
            .insert(id, object_entity);

        let room_id = world
            .get::<Room>(room_entity)
            .map(|room| room.id)
            .ok_or(action::Error::MissingComponent(room_entity, "Room"))?;

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

struct Info {
    id: object::Id,
}

impl Action for Info {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let object_entity = match world.get_resource::<Objects>().unwrap().by_id(self.id) {
            Some(entity) => entity,
            None => {
                let message = format!("Object {} does not exist.", self.id);
                queue_message(world, player, message);
                return Ok(());
            }
        };

        let object = world
            .get::<Object>(object_entity)
            .ok_or(action::Error::MissingComponent(object_entity, "Object"))?;

        let mut message = format!("Object {}", self.id);
        message.push_str("\r\n  flags: ");
        message.push_str(format!("{:?}", object.flags).as_str());
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

struct UpdateKeywords {
    id: object::Id,
    keywords: Vec<String>,
}

impl Action for UpdateKeywords {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let object_entity =
            if let Some(entity) = world.get_resource::<Objects>().unwrap().by_id(self.id) {
                entity
            } else {
                let message = format!("Object {} not found.", self.id);
                queue_message(world, player, message);
                return Ok(());
            };

        let (id, keywords) = {
            let mut object = world
                .get_mut::<Object>(object_entity)
                .ok_or(action::Error::MissingComponent(object_entity, "Object"))?;

            object.keywords = self.keywords.clone();

            (object.id, object.keywords.clone())
        };

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::object::Keywords::new(id, keywords));

        let message = format!("Updated object {} keywords.", self.id);
        queue_message(world, player, message);

        Ok(())
    }
}

struct UpdateLongDescription {
    id: object::Id,
    long: String,
}

impl Action for UpdateLongDescription {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let object_entity =
            if let Some(entity) = world.get_resource::<Objects>().unwrap().by_id(self.id) {
                entity
            } else {
                let message = format!("Object {} not found.", self.id);
                queue_message(world, player, message);
                return Ok(());
            };

        let (id, long) = {
            let mut object = world
                .get_mut::<Object>(object_entity)
                .ok_or(action::Error::MissingComponent(object_entity, "Object"))?;

            object.long = self.long.clone();

            (object.id, object.long.clone())
        };

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::object::Long::new(id, long));

        let message = format!("Updated object {} long description.", self.id);
        queue_message(world, player, message);

        Ok(())
    }
}

struct UpdateShortDescription {
    id: object::Id,
    short: String,
}

impl Action for UpdateShortDescription {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let object_entity =
            if let Some(entity) = world.get_resource::<Objects>().unwrap().by_id(self.id) {
                entity
            } else {
                let message = format!("Object {} not found.", self.id);
                queue_message(world, player, message);
                return Ok(());
            };

        let (id, short) = {
            let mut object = world
                .get_mut::<Object>(object_entity)
                .ok_or(action::Error::MissingComponent(object_entity, "Object"))?;

            object.short = self.short.clone();

            (object.id, object.short.clone())
        };

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::object::Short::new(id, short));

        let message = format!("Updated object {} short description.", self.id);
        queue_message(world, player, message);

        Ok(())
    }
}

struct Remove {
    id: object::Id,
}

impl Action for Remove {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let object_entity =
            if let Some(entity) = world.get_resource::<Objects>().unwrap().by_id(self.id) {
                entity
            } else {
                let message = format!("Object {} not found.", self.id);
                queue_message(world, player, message);
                return Ok(());
            };

        let container = world
            .get::<Object>(object_entity)
            .map(|object| object.container)
            .ok_or(action::Error::MissingComponent(object_entity, "Object"))?;

        world.despawn(object_entity);
        world
            .get_mut::<Contents>(container)
            .ok_or(action::Error::MissingComponent(container, "Contents"))?
            .remove(object_entity);

        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(persist::object::Remove::new(self.id));

        let message = format!("Object {} removed.", self.id);
        queue_message(world, player, message);

        Ok(())
    }
}

struct SetFlags {
    id: object::Id,
    flags: Vec<String>,
}

impl Action for SetFlags {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let object_entity =
            if let Some(entity) = world.get_resource::<Objects>().unwrap().by_id(self.id) {
                entity
            } else {
                let message = format!("Object {} not found.", self.id);
                queue_message(world, player, message);
                return Ok(());
            };

        let new_flags = match Flags::try_from(self.flags.as_slice()) {
            Ok(flags) => flags,
            Err(e) => {
                queue_message(world, player, e.to_string());
                return Ok(());
            }
        };

        let (id, flags) = {
            let mut object = world
                .get_mut::<Object>(object_entity)
                .ok_or(action::Error::MissingComponent(object_entity, "Object"))?;

            object.flags.insert(new_flags);

            (object.id, object.flags)
        };

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::object::Flags::new(id, flags));

        let message = format!("Updated object {} flags.", self.id);
        queue_message(world, player, message);

        Ok(())
    }
}
