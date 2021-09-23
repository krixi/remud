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
            object::{Container, Flags, Keywords, Object, ObjectFlags},
            player::Messages,
            room::Room,
            Contents, Id, Location, Named,
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
    object_query: Query<(&Object, &Named, &Keywords)>,
    mut room_query: Query<(&Room, &mut Contents), With<Room>>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Drop(Drop { actor, keywords }) = action {
            // Find entity to drop in contents of dropping entity, if it exists. Grab some other data as well.
            let (entity_id, room_entity, target) =
                if let Ok((id, location, contents)) = dropping_query.get_mut(*actor) {
                    let target = contents.find(|object| {
                        object_query
                            .get(object)
                            .map(|(_, _, object_keywords)| {
                                {
                                    object_keywords.contains_all(keywords.as_slice())
                                }
                            })
                            .unwrap_or(false)
                    });
                    (*id, location.room(), target)
                } else {
                    tracing::warn!("Entity {:?} cannot drop an item without Contents.", actor);
                    continue;
                };

            let message = if let Some(entity) = target {
                // Move the object from the entity to the room
                dropping_query
                    .get_mut(*actor)
                    .map(|(_, _, mut contents)| contents.remove(entity))
                    .unwrap();

                let room_id = {
                    let (room, mut contents) = room_query
                        .get_mut(room_entity)
                        .expect("Location has valid Room");

                    contents.insert(entity);
                    room.id()
                };

                let (object_id, name) = {
                    let (object, named, _) = object_query.get(entity).unwrap();
                    commands
                        .entity(entity)
                        .insert(Location::from(room_entity))
                        .remove::<Container>();

                    (object.id(), named.as_str())
                };

                // Persist the changes for the object's position
                match entity_id {
                    Id::Player(player_id) => {
                        updates.persist(persist::player::RemoveObject::new(player_id, object_id))
                    }
                    Id::Object(_) => todo!(),
                    Id::Room(_) => todo!(),
                    Id::Prototype(_) => todo!(),
                }
                updates.persist(persist::room::AddObject::new(room_id, object_id));

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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
    object_query: Query<(&Object, &Named, &Keywords, &ObjectFlags)>,
    mut room_query: Query<(&Room, &mut Contents), With<Room>>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Get(Get { actor, keywords }) = action {
            // Get the room that entity is in.
            let (entity_id, room_entity) =
                if let Ok((id, location, _)) = getting_query.get_mut(*actor) {
                    (*id, location.room())
                } else {
                    tracing::warn!("Entity {:?} without Contents cannot get an item.", actor);
                    continue;
                };

            // Find a matching object in the room.
            let target = room_query
                .get_mut(room_entity)
                .map(|(_, contents)| {
                    contents.find(|object| {
                        object_query
                            .get(object)
                            .map(|(_, _, object_keywords, _)| {
                                {
                                    object_keywords.contains_all(keywords.as_slice())
                                }
                            })
                            .unwrap_or(false)
                    })
                })
                .expect("Location has a valid room.");

            let message = if let Some(entity) = target {
                // Move the object from the room to the entity
                let (room_id, object_entity) = {
                    let (room, mut contents) = room_query.get_mut(room_entity).unwrap();

                    let (_, named, _, flags) = object_query.get(entity).unwrap();
                    if flags.contains(Flags::FIXED) {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            let name = named.as_str();
                            messages
                                .queue(format!("Try as you might, you cannot pick up {}.", name));
                        }
                        continue;
                    }

                    contents.remove(entity);

                    (room.id(), entity)
                };

                getting_query
                    .get_mut(*actor)
                    .map(|(_, _, mut contents)| contents.insert(object_entity))
                    .expect("Location has valid Room");

                let (object_id, name) = {
                    let (object, named, _, _) = object_query.get(object_entity).unwrap();

                    commands
                        .entity(object_entity)
                        .insert(Container::from(*actor))
                        .remove::<Location>();

                    (object.id(), named.as_str())
                };

                // Persist the changes for the object's position
                match entity_id {
                    Id::Player(player_id) => {
                        updates.persist(persist::player::AddObject::new(player_id, object_id))
                    }
                    Id::Object(_) => todo!(),
                    Id::Room(_) => todo!(),
                    Id::Prototype(_) => todo!(),
                }
                updates.persist(persist::room::RemoveObject::new(room_id, object_id));

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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Inventory {
    pub actor: Entity,
}

into_action!(Inventory);

pub fn inventory_system(
    mut action_reader: EventReader<Action>,
    inventory_query: Query<&Contents>,
    object_query: Query<&Named>,
    mut messages_query: Query<&mut Messages>,
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

            if contents.is_empty() {
                message.push_str(" nothing.|-|");
            } else {
                message.push_str(":|-|");
                for object_entity in contents.objects().iter() {
                    let named = object_query.get(*object_entity).unwrap();
                    message.push_str("\r\n  ");
                    message.push_str(named.as_str());
                }
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}
