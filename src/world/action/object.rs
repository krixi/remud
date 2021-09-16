use bevy_app::{EventReader, Events};
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::Tokenizer,
    world::{
        action::{self, queue_message, Action, ActionEvent, DynAction},
        types::{
            self,
            object::Object,
            player::{Messages, Player},
            room::Room,
            Container, Contents, Id, Keywords, Location, Named,
        },
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
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::Drop {
                entity,
                keywords: self.keywords.clone(),
            });
        Ok(())
    }
}

pub fn drop_system(
    mut events: EventReader<ActionEvent>,
    mut updates: ResMut<Updates>,
    mut dropping_query: Query<(&Id, &Location, &mut Contents)>,
    mut object_query: Query<(&Id, &Named, &Keywords, &mut Container)>,
    mut room_query: Query<(&Room, &mut Contents)>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Drop { entity, keywords } = event {
            // Find entity to drop in contents of dropping entity, if it exists. Grab some other data as well.
            let (entity_id, room_entity, pos) =
                if let Ok((id, location, contents)) = dropping_query.get_mut(*entity) {
                    let pos = contents.objects.iter().position(|object| {
                        object_query
                            .get_mut(*object)
                            .map(|(_, _, object_keywords, _)| {
                                {
                                    keywords
                                        .iter()
                                        .all(|keyword| object_keywords.list.contains(keyword))
                                }
                            })
                            .unwrap_or(false)
                    });
                    (*id, location.room, pos)
                } else {
                    tracing::warn!("Entity {:?} cannot drop an item without Contents.", entity);
                    continue;
                };

            let message = if let Some(pos) = pos {
                let object_entity = dropping_query
                    .get_mut(*entity)
                    .map(|(_, _, mut contents)| contents.objects.remove(pos))
                    .unwrap();

                let room_id = {
                    let (room, mut contents) = room_query
                        .get_mut(room_entity)
                        .expect("Location has valid Room");

                    contents.objects.push(object_entity);
                    room.id
                };

                let (object_id, name) = {
                    let (id, named, _, mut container) =
                        object_query.get_mut(object_entity).unwrap();
                    container.entity = room_entity;

                    let id = if let Id::Object(id) = id {
                        *id
                    } else {
                        tracing::warn!("Object {:?} does not have an object ID.", object_entity);
                        continue;
                    };

                    (id, named.name.as_str())
                };

                match entity_id {
                    Id::Player(player_id) => {
                        updates.queue(persist::player::RemoveObject::new(player_id, object_id))
                    }
                    Id::Object(_) => todo!(),
                    Id::Room(_) => todo!(),
                }
                updates.queue(persist::room::AddObject::new(room_id, object_id));

                format!("You drop {}.", name)
            } else {
                format!("You don't have \"{}\".", keywords.join(" "))
            };

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue(message);
            }
        };
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
