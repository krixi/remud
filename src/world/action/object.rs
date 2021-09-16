use bevy_app::{EventReader, Events};
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::Tokenizer,
    world::{
        action::{self, Action, ActionEvent, DynAction},
        types::{
            object, player::Messages, room::Room, Container, Contents, Flags, Id, Keywords,
            Location, Named,
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
    mut dropping_query: Query<(&Id, &Location, &mut Contents), Without<Room>>,
    mut object_query: Query<(&Id, &Named, &Keywords, &mut Container)>,
    mut room_query: Query<(&Room, &mut Contents), With<Room>>,
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
                // Move the object from the entity to the room
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

                // Persist the changes for the object's position
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
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::Get {
                entity,
                keywords: self.keywords.clone(),
            });
        Ok(())
    }
}

pub fn get_system(
    mut events: EventReader<ActionEvent>,
    mut updates: ResMut<Updates>,
    mut getting_query: Query<(&Id, &Location, &mut Contents), Without<Room>>,
    mut object_query: Query<(&Id, &Named, &Keywords, &mut Container)>,
    flags_query: Query<&Flags>,
    mut room_query: Query<(&Room, &mut Contents), With<Room>>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Get { entity, keywords } = event {
            // Get the room that entity is in.
            let (entity_id, room_entity) =
                if let Ok((id, location, _)) = getting_query.get_mut(*entity) {
                    (*id, location.room)
                } else {
                    tracing::warn!("Entity {:?} without Contents cannot get an item.", entity);
                    continue;
                };

            // Find a matching object in the room.
            let pos = room_query
                .get_mut(room_entity)
                .map(|(_, contents)| {
                    contents.objects.iter().position(|object| {
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
                    })
                })
                .expect("Location has a valid room.");

            let message = if let Some(pos) = pos {
                // Move the object from the room to the entity
                let (room_id, object_entity) = {
                    let (room, mut contents) = room_query.get_mut(room_entity).unwrap();

                    let object_entity = contents.objects[pos];

                    if flags_query
                        .get(object_entity)
                        .unwrap()
                        .flags
                        .contains(object::Flags::FIXED)
                    {
                        if let Ok(mut messages) = messages_query.get_mut(*entity) {
                            let (_, named, _, _) = object_query.get_mut(object_entity).unwrap();
                            messages.queue(format!(
                                "Try as you might, you cannot pick up {}.",
                                named.name
                            ));
                        }
                        continue;
                    }

                    contents.objects.remove(pos);

                    (room.id, object_entity)
                };

                getting_query
                    .get_mut(*entity)
                    .map(|(_, _, mut contents)| contents.objects.push(object_entity))
                    .expect("Location has valid Room");

                let (object_id, name) = {
                    let (id, named, _, mut container) =
                        object_query.get_mut(object_entity).unwrap();
                    container.entity = *entity;

                    let id = if let Id::Object(id) = id {
                        *id
                    } else {
                        tracing::warn!("Object {:?} does not have an object ID.", object_entity);
                        continue;
                    };

                    (id, named.name.as_str())
                };

                // Persist the changes for the object's position
                match entity_id {
                    Id::Player(player_id) => {
                        updates.queue(persist::player::AddObject::new(player_id, object_id))
                    }
                    Id::Object(_) => todo!(),
                    Id::Room(_) => todo!(),
                }
                updates.queue(persist::room::RemoveObject::new(room_id, object_id));

                format!("You pick up {}.", name)
            } else {
                format!(
                    "You find no object called \"{}\" to pick up.",
                    keywords.join(" ")
                )
            };

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue(message);
            }
        };
    }
}

#[derive(Default)]
pub struct Inventory {}

impl Action for Inventory {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::Inventory { entity });
        Ok(())
    }
}

pub fn inventory_system(
    mut events: EventReader<ActionEvent>,
    inventory_query: Query<&Contents>,
    object_query: Query<&Named>,
    mut messages: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Inventory { entity } = event {
            let mut message = "You have".to_string();

            let contents = if let Ok(contents) = inventory_query.get(*entity) {
                contents
            } else {
                tracing::warn!(
                    "Cannot request inventory of entity {:?} without Contents",
                    entity
                );
                continue;
            };

            if contents.objects.is_empty() {
                message.push_str(" nothing.");
            } else {
                message.push(':');
                contents
                    .objects
                    .iter()
                    .filter_map(|object| object_query.get(*object).ok())
                    .map(|named| named.name.as_str())
                    .for_each(|desc| {
                        message.push_str("\r\n  ");
                        message.push_str(desc)
                    });
            }

            if let Ok(mut messages) = messages.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}
