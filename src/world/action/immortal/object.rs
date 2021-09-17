use std::{convert::TryFrom, str::FromStr};

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    event_from_action,
    text::{word_list, Tokenizer},
    world::{
        action::{ActionEvent, DEFAULT_OBJECT_KEYWORD, DEFAULT_OBJECT_LONG, DEFAULT_OBJECT_SHORT},
        types::{
            self,
            object::{Object, ObjectBundle, ObjectFlags, ObjectId, Objects},
            player::{Messages, Player},
            room::Room,
            Container, Contents, Description, Id, Keywords, Location, Named,
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
pub fn parse(player: Entity, mut tokenizer: Tokenizer) -> Result<ActionEvent, String> {
    if let Some(token) = tokenizer.next() {
        match token {
            "new" => Ok(ActionEvent::from(ObjectCreate { entity: player })),
            maybe_id => {
                let id = match ObjectId::from_str(maybe_id) {
                    Ok(id) => id,
                    Err(e) => return Err(e.to_string()),
                };

                if let Some(token) = tokenizer.next() {
                    match token {
                        "info" => Ok(ActionEvent::from(ObjectInfo {entity: player, id })),
                        "keywords" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a comma separated list of keywords.".to_string())
                            } else {
                                let keywords = tokenizer
                                    .rest()
                                    .split(',')
                                    .map(|keyword| keyword.trim().to_string())
                                    .collect_vec();

                                Ok(ActionEvent::from(ObjectUpdateKeywords {
                                    entity: player,
                                    id,
                                    keywords,
                                }))
                            }
                        }
                        "desc" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a long description.".to_string())
                            } else {
                                Ok(ActionEvent::from(ObjectUpdateDescription {
                                    entity: player,
                                    id,
                                    description: tokenizer.rest().to_string(),
                                }))
                            }
                        }
                        "remove" => Ok(ActionEvent::from(ObjectRemove { entity: player, id })),
                        "set" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a space separated list of flags. Valid flags: fixed, subtle.".to_string())
                            } else {
                                Ok(ActionEvent::from(ObjectSetFlags {entity: player, id, flags: tokenizer.rest().to_string().split_whitespace().map(|flag|flag.to_string()).collect_vec()}))
                            }
                        }
                        "name" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a short description.".to_string())
                            } else {
                                Ok(ActionEvent::from(ObjectUpdateName {
                                    entity: player,
                                    id,
                                    name: tokenizer.rest().to_string(),
                                }))
                            }
                        }
                        "unset" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a space separated list of flags. Valid flags: fixed, subtle.".to_string())
                            } else {
                                Ok(ActionEvent::from(ObjectUnsetFlags {entity: player, id, flags: tokenizer.rest().to_string().split_whitespace().map(|flag|flag.to_string()).collect_vec()}))
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

#[derive(Debug, Clone)]
pub struct ObjectUnsetFlags {
    pub entity: Entity,
    pub id: ObjectId,
    pub flags: Vec<String>,
}

event_from_action!(ObjectUnsetFlags);

pub fn object_clear_flags_system(
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut types::Flags)>,
    mut messages: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectUnsetFlags(ObjectUnsetFlags { entity, id, flags }) = event {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let remove_flags = match ObjectFlags::try_from(flags.as_slice()) {
                Ok(flags) => flags,
                Err(e) => {
                    if let Ok(mut messages) = messages.get_mut(*entity) {
                        messages.queue(e.to_string());
                    }
                    continue;
                }
            };

            let (id, flags) = {
                let (object, mut flags) = object_query.get_mut(object_entity).unwrap();

                flags.flags.remove(remove_flags);

                (object.id, flags.flags)
            };

            updates.queue(persist::object::Flags::new(id, flags));

            if let Ok(mut messages) = messages.get_mut(*entity) {
                messages.queue(format!("Updated object {} flags.", id));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectCreate {
    pub entity: Entity,
}

event_from_action!(ObjectCreate);

pub fn object_create_system(
    mut commands: Commands,
    mut events: EventReader<ActionEvent>,
    mut objects: ResMut<Objects>,
    mut updates: ResMut<Updates>,
    player_query: Query<&Location, With<Player>>,
    mut room_query: Query<(&Room, &mut Contents)>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectCreate(ObjectCreate { entity }) = event {
            let room_entity = player_query
                .get(*entity)
                .map(|location| location.room)
                .unwrap();

            let id = objects.next_id();

            let object_entity = commands
                .spawn_bundle(ObjectBundle {
                    object: Object { id },
                    id: Id::Object(id),
                    flags: types::Flags {
                        flags: ObjectFlags::empty(),
                    },
                    container: Container {
                        entity: room_entity,
                    },
                    keywords: Keywords {
                        list: vec![DEFAULT_OBJECT_KEYWORD.to_string()],
                    },
                    name: Named {
                        name: DEFAULT_OBJECT_SHORT.to_string(),
                    },
                    description: Description {
                        text: DEFAULT_OBJECT_LONG.to_string(),
                    },
                })
                .id();

            let room_id = {
                let (room, mut contents) = room_query.get_mut(room_entity).unwrap();
                contents.objects.push(object_entity);
                room.id
            };

            updates.queue(persist::object::New::new(id));
            updates.queue(persist::room::AddObject::new(room_id, id));

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue(format!("Created object {}.", id));
            }

            objects.insert(id, object_entity);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectInfo {
    pub entity: Entity,
    pub id: ObjectId,
}

event_from_action!(ObjectInfo);

pub fn object_info_system(
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    object_query: Query<(
        &Object,
        &types::Flags,
        &Keywords,
        &Named,
        &Description,
        &Container,
    )>,
    room_query: Query<&Room>,
    player_query: Query<&Named, With<Player>>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectInfo(ObjectInfo { entity, id }) = event {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let (object, flags, keywords, named, description, container) =
                object_query.get(object_entity).unwrap();

            let mut message = format!("Object {}", object.id);
            message.push_str("\r\n  name: ");
            message.push_str(named.name.as_str());
            message.push_str("\r\n  description: ");
            message.push_str(description.text.as_str());
            message.push_str("\r\n  flags: ");
            message.push_str(format!("{:?}", flags).as_str());
            message.push_str("\r\n  keywords: ");
            message.push_str(word_list(keywords.list.clone()).as_str());
            message.push_str("\r\n  container: ");
            if let Ok(room) = room_query.get(container.entity) {
                message.push_str("room ");
                message.push_str(room.id.to_string().as_str());
            } else if let Ok(named) = player_query.get(container.entity) {
                message.push_str("player ");
                message.push_str(named.name.as_str());
            } else {
                message.push_str(format!("{:?}", container.entity).as_str());
            }

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectUpdateKeywords {
    pub entity: Entity,
    pub id: ObjectId,
    pub keywords: Vec<String>,
}

event_from_action!(ObjectUpdateKeywords);

pub fn object_update_keywords_system(
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut Keywords)>,
    mut messages: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectUpdateKeywords(ObjectUpdateKeywords {
            entity,
            id,
            keywords,
        }) = event
        {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let id = {
                let (object, mut current_keywords) = object_query.get_mut(object_entity).unwrap();

                current_keywords.list = keywords.clone();

                object.id
            };

            updates.queue(persist::object::Keywords::new(id, keywords.clone()));

            if let Ok(mut messages) = messages.get_mut(*entity) {
                messages.queue(format!("Updated object {} keywords.", id));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectUpdateDescription {
    pub entity: Entity,
    pub id: ObjectId,
    pub description: String,
}

event_from_action!(ObjectUpdateDescription);

pub fn object_update_description_system(
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut Description)>,
    mut messages: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectUpdateDescription(ObjectUpdateDescription {
            entity,
            id,
            description,
        }) = event
        {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let id = {
                let (object, mut current_description) =
                    object_query.get_mut(object_entity).unwrap();

                current_description.text = description.clone();

                object.id
            };

            updates.queue(persist::object::Long::new(id, description.clone()));

            if let Ok(mut messages) = messages.get_mut(*entity) {
                messages.queue(format!("Updated object {} description.", id));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectUpdateName {
    pub entity: Entity,
    pub id: ObjectId,
    pub name: String,
}

event_from_action!(ObjectUpdateName);

pub fn object_update_name_system(
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut Named)>,
    mut messages: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectUpdateName(ObjectUpdateName { entity, id, name }) = event {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let id = {
                let (object, mut named) = object_query.get_mut(object_entity).unwrap();

                named.name = name.clone();

                object.id
            };

            updates.queue(persist::object::Short::new(id, name.clone()));

            if let Ok(mut messages) = messages.get_mut(*entity) {
                messages.queue(format!("Updated object {} name.", id));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectRemove {
    pub entity: Entity,
    pub id: ObjectId,
}

event_from_action!(ObjectRemove);

pub fn object_remove_system(
    mut commands: Commands,
    mut events: EventReader<ActionEvent>,
    mut objects: ResMut<Objects>,
    mut updates: ResMut<Updates>,
    container_query: Query<&Container>,
    mut contents_query: Query<&mut Contents>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectRemove(ObjectRemove { entity, id }) = event {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let container = container_query.get(object_entity).unwrap().entity;

            objects.remove(*id);
            commands.entity(object_entity).despawn();
            contents_query
                .get_mut(container)
                .unwrap()
                .remove(object_entity);

            updates.queue(persist::object::Remove::new(*id));

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue(format!("Object {} removed.", id));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectSetFlags {
    pub entity: Entity,
    pub id: ObjectId,
    pub flags: Vec<String>,
}

event_from_action!(ObjectSetFlags);

pub fn object_set_flags_system(
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut types::Flags)>,
    mut messages: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectSetFlags(ObjectSetFlags { entity, id, flags }) = event {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let set_flags = match ObjectFlags::try_from(flags.as_slice()) {
                Ok(flags) => flags,
                Err(e) => {
                    if let Ok(mut messages) = messages.get_mut(*entity) {
                        messages.queue(e.to_string());
                    }
                    continue;
                }
            };

            let (id, flags) = {
                let (object, mut flags) = object_query.get_mut(object_entity).unwrap();

                flags.flags.insert(set_flags);

                (object.id, flags.flags)
            };

            updates.queue(persist::object::Flags::new(id, flags));

            if let Ok(mut messages) = messages.get_mut(*entity) {
                messages.queue(format!("Updated object {} flags.", id));
            }
        }
    }
}
