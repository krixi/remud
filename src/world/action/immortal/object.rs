use std::{convert::TryFrom, str::FromStr};

use bevy_app::{EventReader, Events};
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::{word_list, Tokenizer},
    world::{
        action::{
            self, Action, ActionEvent, DynAction, DEFAULT_OBJECT_KEYWORD, DEFAULT_OBJECT_LONG,
            DEFAULT_OBJECT_SHORT,
        },
        types::{
            self,
            object::{self, Flags, Object, ObjectBundle, Objects},
            player::{Messages, Player},
            room::Room,
            Container, Contents, Description, Keywords, Location, Named,
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
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::ObjectClearFlags {
                entity,
                id: self.id,
                flags: self.flags.clone(),
            });

        Ok(())
    }
}

pub fn object_clear_flags_system(
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut types::Flags)>,
    mut messages: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectClearFlags { entity, id, flags } = event {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let remove_flags = match Flags::try_from(flags.as_slice()) {
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

struct Create {}

impl Action for Create {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::ObjectCreate { entity });

        Ok(())
    }
}

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
        if let ActionEvent::ObjectCreate { entity } = event {
            let room_entity = player_query
                .get(*entity)
                .map(|location| location.room)
                .unwrap();

            let id = objects.next_id();

            let object_entity = commands
                .spawn_bundle(ObjectBundle {
                    object: Object {
                        id,
                        flags: object::Flags::empty(),
                        container: room_entity,
                        keywords: vec![DEFAULT_OBJECT_KEYWORD.to_string()],
                        short: DEFAULT_OBJECT_SHORT.to_string(),
                        long: DEFAULT_OBJECT_LONG.to_string(),
                    },
                    id: types::Id::Object(id),
                    flags: types::Flags {
                        flags: object::Flags::empty(),
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

struct Info {
    id: object::Id,
}

impl Action for Info {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::ObjectInfo {
                entity,
                id: self.id,
            });

        Ok(())
    }
}

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
        if let ActionEvent::ObjectInfo { entity, id } = event {
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
                message.push_str(format!("{:?}", object.container).as_str());
            }

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}

struct UpdateKeywords {
    id: object::Id,
    keywords: Vec<String>,
}

impl Action for UpdateKeywords {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::ObjectUpdateKeywords {
                entity,
                id: self.id,
                keywords: self.keywords.clone(),
            });

        Ok(())
    }
}

pub fn object_update_keywords_system(
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut Keywords)>,
    mut messages: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectUpdateKeywords {
            entity,
            id,
            keywords,
        } = event
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

struct UpdateLongDescription {
    id: object::Id,
    long: String,
}

impl Action for UpdateLongDescription {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::ObjectUpdateDescription {
                entity,
                id: self.id,
                description: self.long.clone(),
            });

        Ok(())
    }
}

pub fn object_update_description_system(
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut Description)>,
    mut messages: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectUpdateDescription {
            entity,
            id,
            description,
        } = event
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

struct UpdateShortDescription {
    id: object::Id,
    short: String,
}

impl Action for UpdateShortDescription {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::ObjectUpdateName {
                entity,
                id: self.id,
                name: self.short.clone(),
            });

        Ok(())
    }
}

pub fn object_update_name_system(
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut Named)>,
    mut messages: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectUpdateName { entity, id, name } = event {
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

struct Remove {
    id: object::Id,
}

impl Action for Remove {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::ObjectRemove {
                entity,
                id: self.id,
            });

        Ok(())
    }
}

pub fn object_remove_system(
    mut commands: Commands,
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    container_query: Query<&Container>,
    mut contents_query: Query<&mut Contents>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectRemove { entity, id } = event {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let container = container_query.get(object_entity).unwrap().entity;

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

struct SetFlags {
    id: object::Id,
    flags: Vec<String>,
}

impl Action for SetFlags {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::ObjectSetFlags {
                entity,
                id: self.id,
                flags: self.flags.clone(),
            });
        Ok(())
    }
}

pub fn object_set_flags_system(
    mut events: EventReader<ActionEvent>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut types::Flags)>,
    mut messages: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::ObjectSetFlags { entity, id, flags } = event {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let set_flags = match Flags::try_from(flags.as_slice()) {
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
