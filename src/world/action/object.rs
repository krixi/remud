use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    into_action,
    text::Tokenizer,
    world::{
        action::Action,
        types::{
            object::ObjectFlags, player::Messages, room::Room, Container, Contents, Flags, Id,
            Keywords, Location, Named,
        },
    },
};

pub fn parse_drop(player: Entity, tokenizer: Tokenizer) -> Result<Action, String> {
    if tokenizer.rest().is_empty() {
        return Err("Drop what?".to_string());
    }

    let keywords = tokenizer
        .rest()
        .split_whitespace()
        .map(ToString::to_string)
        .collect_vec();

    Ok(Action::from(Drop {
        actor: player,
        keywords,
    }))
}

#[derive(Debug, Clone)]
pub struct Drop {
    pub actor: Entity,
    pub keywords: Vec<String>,
}

into_action!(Drop);

pub fn drop_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    mut updates: ResMut<Updates>,
    mut dropping_query: Query<(&Id, &Location, &mut Contents), Without<Room>>,
    mut object_query: Query<(&Id, &Named, &Keywords)>,
    mut room_query: Query<(&Room, &mut Contents), With<Room>>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Drop(Drop { actor, keywords }) = action {
            // Find entity to drop in contents of dropping entity, if it exists. Grab some other data as well.
            let (entity_id, room_entity, pos) =
                if let Ok((id, location, contents)) = dropping_query.get_mut(*actor) {
                    let pos = contents.objects.iter().position(|object| {
                        object_query
                            .get_mut(*object)
                            .map(|(_, _, object_keywords)| {
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
                    tracing::warn!("Entity {:?} cannot drop an item without Contents.", actor);
                    continue;
                };

            let message = if let Some(pos) = pos {
                // Move the object from the entity to the room
                let object_entity = dropping_query
                    .get_mut(*actor)
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
                    let (id, named, _) = object_query.get_mut(object_entity).unwrap();
                    commands
                        .entity(object_entity)
                        .insert(Location { room: room_entity })
                        .remove::<Container>();

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

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        };
    }
}

pub fn parse_get(player: Entity, tokenizer: Tokenizer) -> Result<Action, String> {
    if tokenizer.rest().is_empty() {
        return Err("Get what?".to_string());
    }

    let keywords = tokenizer
        .rest()
        .split_whitespace()
        .map(ToString::to_string)
        .collect_vec();

    Ok(Action::from(Get {
        actor: player,
        keywords,
    }))
}

#[derive(Debug, Clone)]
pub struct Get {
    pub actor: Entity,
    pub keywords: Vec<String>,
}

into_action!(Get);

pub fn get_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    mut updates: ResMut<Updates>,
    mut getting_query: Query<(&Id, &Location, &mut Contents), Without<Room>>,
    mut object_query: Query<(&Id, &Named, &Keywords)>,
    flags_query: Query<&Flags>,
    mut room_query: Query<(&Room, &mut Contents), With<Room>>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Get(Get { actor, keywords }) = action {
            // Get the room that entity is in.
            let (entity_id, room_entity) =
                if let Ok((id, location, _)) = getting_query.get_mut(*actor) {
                    (*id, location.room)
                } else {
                    tracing::warn!("Entity {:?} without Contents cannot get an item.", actor);
                    continue;
                };

            // Find a matching object in the room.
            let pos = room_query
                .get_mut(room_entity)
                .map(|(_, contents)| {
                    contents.objects.iter().position(|object| {
                        object_query
                            .get_mut(*object)
                            .map(|(_, _, object_keywords)| {
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
                        .contains(ObjectFlags::FIXED)
                    {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            let (_, named, _) = object_query.get_mut(object_entity).unwrap();
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
                    .get_mut(*actor)
                    .map(|(_, _, mut contents)| contents.objects.push(object_entity))
                    .expect("Location has valid Room");

                let (object_id, name) = {
                    let (id, named, _) = object_query.get_mut(object_entity).unwrap();

                    commands
                        .entity(object_entity)
                        .insert(Container { entity: *actor })
                        .remove::<Location>();

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

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        };
    }
}

#[derive(Debug, Clone)]
pub struct Inventory {
    pub actor: Entity,
}

into_action!(Inventory);

pub fn inventory_system(
    mut action_reader: EventReader<Action>,
    inventory_query: Query<&Contents>,
    object_query: Query<&Named>,
    mut messages: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Inventory(Inventory { actor }) = action {
            let mut message = "|white|You have".to_string();

            let contents = if let Ok(contents) = inventory_query.get(*actor) {
                contents
            } else {
                tracing::warn!(
                    "Cannot request inventory of entity {:?} without Contents",
                    actor
                );
                continue;
            };

            if contents.objects.is_empty() {
                message.push_str(" nothing.|-|");
            } else {
                message.push_str(":|-|");
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

            if let Ok(mut messages) = messages.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}
