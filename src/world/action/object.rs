use std::str::FromStr;

use anyhow::bail;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persistence::{
        PersistNewObject, PersistObjectContainer, PersistRemoveObject, PersistUpdateObject, Updates,
    },
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
        updates.queue(PersistNewObject::new(object_entity));
        updates.queue(PersistObjectContainer::new(object_entity));

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
            .queue(PersistUpdateObject::new(object_entity));

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
        updates.queue(PersistRemoveObject::new(self.id));

        let message = format!("Object {} removed.", self.id);
        queue_message(world, player, message);

        Ok(())
    }
}
