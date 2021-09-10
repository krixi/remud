use std::str::FromStr;

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persistence::{PersistNewObject, PersistObjectRoom, PersistObjectUpdate, Updates},
    text::Tokenizer,
    world::{
        action::{queue_message, Action, DynAction},
        types::{
            object::{Object, ObjectId, Objects},
            room::Room,
            Location,
        },
    },
};

pub fn parse(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(token) = tokenizer.next() {
        match token {
            "new" => Ok(Box::new(CreateObject {})),
            maybe_id => {
                if let Ok(id) = ObjectId::from_str(maybe_id) {
                    if let Some(token) = tokenizer.next() {
                        match token {
                            "keywords" => {
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
                            "short" => Ok(Box::new(UpdateObject {
                                id,
                                keywords: None,
                                short: Some(tokenizer.rest().to_string()),
                                long: None,
                            })),
                            "long" => Ok(Box::new(UpdateObject {
                                id,
                                keywords: None,
                                short: None,
                                long: Some(tokenizer.rest().to_string()),
                            })),
                            _ => Err(format!("I don't know how to {} object {}.", token, id)),
                        }
                    } else {
                        Err("Provide a valid object subcommand or ID.".to_string())
                    }
                } else {
                    Err(format!("I don't know how to {} an object.", token))
                }
            }
        }
    } else {
        Err("What's all this about an object?".to_string())
    }
}

struct CreateObject {}

impl Action for CreateObject {
    fn enact(&mut self, player: Entity, world: &mut World) {
        let id = world.get_resource_mut::<Objects>().unwrap().next_id();

        let object_entity = world
            .spawn()
            .insert(Object {
                id,
                keywords: vec!["object".to_string()],
                short: "An object.".to_string(),
                long: "A nondescript object. Completely uninteresting.".to_string(),
            })
            .id();

        // place the object in the room
        let room_entity = if let Some(room_entity) =
            world.get::<Location>(player).map(|location| location.room)
        {
            if let Some(mut room) = world.get_mut::<Room>(room_entity) {
                room.objects.push(object_entity);
                Some(room_entity)
            } else {
                None
            }
        } else {
            None
        };

        world
            .get_resource_mut::<Objects>()
            .unwrap()
            .add_object(id, object_entity);

        // notify the player that the object was created
        let message = format!("Created object {}\r\n", id);
        queue_message(world, player, message);

        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(PersistNewObject::new(object_entity));
        if let Some(room_entity) = room_entity {
            updates.queue(PersistObjectRoom::new(object_entity, room_entity));
        }
    }
}

struct UpdateObject {
    id: ObjectId,
    keywords: Option<Vec<String>>,
    short: Option<String>,
    long: Option<String>,
}

impl Action for UpdateObject {
    fn enact(&mut self, player: Entity, world: &mut World) {
        let object_entity =
            if let Some(entity) = world.get_resource::<Objects>().unwrap().get_object(self.id) {
                entity
            } else {
                let message = format!("Object {} not found.", self.id);
                queue_message(world, player, message);
                return;
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

        let message = format!("Updated object {}\r\n", self.id);
        queue_message(world, player, message);

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(PersistObjectUpdate::new(object_entity));
    }
}
