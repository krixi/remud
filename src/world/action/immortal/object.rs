use std::{convert::TryFrom, str::FromStr};

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, UpdateGroup, Updates},
    into_action,
    text::{word_list, Tokenizer},
    world::{
        action::Action,
        fsm::{
            states::{ChaseState, WanderState},
            StateId, StateMachine,
        },
        scripting::{ScriptHook, ScriptHooks},
        types::{
            self,
            object::{Object, ObjectBundle, ObjectFlags, ObjectId, Objects},
            player::{Messages, Player},
            room::Room,
            Container, Contents, Description, Id, Keywords, Location, Named,
        },
    },
};

pub const DEFAULT_OBJECT_KEYWORD: &str = "object";
pub const DEFAULT_OBJECT_NAME: &str = "an object";
pub const DEFAULT_OBJECT_DESCRIPTION: &str = "A nondescript object. Completely uninteresting.";

// Valid shapes:
// object new - creates a new object and puts it on the ground
// object <id> info - displays information about the object
// object <id> keywords - sets an object's keywords
// object <id> name - sets an object's short description
// object <id> description - sets an object's long description
// object <id> remove - removes an object
// object <id> set - sets one or more object flags
// object <id> unset - clears one or more object flags
pub fn parse_object(player: Entity, mut tokenizer: Tokenizer) -> Result<Action, String> {
    if let Some(token) = tokenizer.next() {
        match token {
            "new" => Ok(Action::from(ObjectCreate { entity: player })),
            maybe_id => {
                let id = match ObjectId::from_str(maybe_id) {
                    Ok(id) => id,
                    Err(e) => return Err(e.to_string()),
                };

                if let Some(token) = tokenizer.next() {
                    match token {
                        "info" => Ok(Action::from(ObjectInfo {entity: player, id })),
                        "keywords" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a space separated list of keywords.".to_string())
                            } else {
                                let keywords = tokenizer
                                    .rest()
                                    .split(' ')
                                    .map(|keyword| keyword.trim().to_string())
                                    .collect_vec();

                                Ok(Action::from(ObjectUpdateKeywords {
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
                                Ok(Action::from(ObjectUpdateDescription {
                                    entity: player,
                                    id,
                                    description: tokenizer.rest().to_string(),
                                }))
                            }
                        }
                        "remove" => Ok(Action::from(ObjectRemove { entity: player, id })),
                        "set" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a space separated list of flags. Valid flags: fixed, subtle.".to_string())
                            } else {
                                Ok(Action::from(ObjectUpdateFlags {entity: player, id, flags: tokenizer.rest().to_string().split_whitespace().map(|flag|flag.to_string()).collect_vec(), clear: false}))
                            }
                        }
                        "name" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a short description.".to_string())
                            } else {
                                Ok(Action::from(ObjectUpdateName {
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
                                Ok(Action::from(ObjectUpdateFlags {entity: player, id, flags: tokenizer.rest().to_string().split_whitespace().map(|flag|flag.to_string()).collect_vec(), clear: true}))
                            }
                        }
                        _ => Err("Enter a valid object subcommand: desc, info, keywords, name, remove, set, or unset."
                            .to_string()),
                    }
                } else {
                    Err("Enter an object subcommand: desc, info, keywords, name, remove, set, or unset.".to_string())
                }
            }
        }
    } else {
        Err("Enter an object ID or subcommand: new.".to_string())
    }
}

#[derive(Debug, Clone)]
pub struct ObjectCreate {
    pub entity: Entity,
}

into_action!(ObjectCreate);

pub fn object_create_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    mut objects: ResMut<Objects>,
    mut updates: ResMut<Updates>,
    player_query: Query<&Location, With<Player>>,
    mut room_query: Query<(&Room, &mut Contents)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectCreate(ObjectCreate { entity }) = action {
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
                    keywords: Keywords {
                        list: vec![DEFAULT_OBJECT_KEYWORD.to_string()],
                    },
                    name: Named {
                        name: DEFAULT_OBJECT_NAME.to_string(),
                    },
                    description: Description {
                        text: DEFAULT_OBJECT_DESCRIPTION.to_string(),
                    },
                })
                .insert(Location { room: room_entity })
                // TODO: this is how to add a state machine for now, until
                .insert(
                    StateMachine::builder()
                        .with_state(StateId::Wander, WanderState::default())
                        .with_state(StateId::Chase, ChaseState::default())
                        .build()
                        .unwrap(),
                )
                .id();

            let room_id = {
                let (room, mut contents) = room_query.get_mut(room_entity).unwrap();
                contents.objects.push(object_entity);
                room.id
            };

            updates.queue(UpdateGroup::new(vec![
                persist::object::Create::new(id),
                persist::room::AddObject::new(room_id, id),
            ]));

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

into_action!(ObjectInfo);

pub fn object_info_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    object_query: Query<(
        &Object,
        &types::Flags,
        &Keywords,
        &Named,
        &Description,
        Option<&Container>,
        Option<&Location>,
        Option<&ScriptHooks>,
    )>,
    room_query: Query<&Room>,
    player_query: Query<&Named, With<Player>>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectInfo(ObjectInfo { entity, id }) = action {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let (object, flags, keywords, named, description, container, location, hooks) =
                object_query.get(object_entity).unwrap();

            let mut message = format!("|white|Object {}|-|", object.id);
            message.push_str("\r\n  |white|name|-|: ");
            message.push_str(named.name.replace("|", "||").as_str());
            message.push_str("\r\n  |white|description|-|: ");
            message.push_str(description.text.replace("|", "||").as_str());
            message.push_str(format!("\r\n  |white|flags|-|: {:?}", flags.flags).as_str());
            message.push_str("\r\n  |white|keywords|-|: ");
            message.push_str(word_list(keywords.list.clone()).as_str());
            message.push_str("\r\n  |white|container|-|: ");
            if let Some(container) = container {
                if let Ok(named) = player_query.get(container.entity) {
                    message.push_str("player ");
                    message.push_str(named.name.as_str());
                } else {
                    message.push_str("other ");
                    message.push_str(format!("{:?}", container.entity).as_str());
                }
            } else if let Some(location) = location {
                if let Ok(room) = room_query.get(location.room) {
                    message.push_str("room ");
                    message.push_str(room.id.to_string().as_str());
                }
            }
            message.push_str("\r\n  |white|script hooks|-|:");
            if let Some(ScriptHooks { list }) = hooks {
                if list.is_empty() {
                    message.push_str(" none");
                }
                for ScriptHook { trigger, script } in list.iter() {
                    message.push_str(format!("\r\n    {:?} -> {}", trigger, script).as_str());
                }
            } else {
                message.push_str(" none");
            }

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue(message);
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

into_action!(ObjectUpdateDescription);

pub fn object_update_description_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut Description)>,
    mut messages: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectUpdateDescription(ObjectUpdateDescription {
            entity,
            id,
            description,
        }) = action
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

            updates.queue(persist::object::Description::new(id, description.clone()));

            if let Ok(mut messages) = messages.get_mut(*entity) {
                messages.queue(format!("Updated object {} description.", id));
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

into_action!(ObjectUpdateKeywords);

pub fn object_update_keywords_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut Keywords)>,
    mut messages: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectUpdateKeywords(ObjectUpdateKeywords {
            entity,
            id,
            keywords,
        }) = action
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
pub struct ObjectUpdateName {
    pub entity: Entity,
    pub id: ObjectId,
    pub name: String,
}

into_action!(ObjectUpdateName);

pub fn object_update_name_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut Named)>,
    mut messages: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectUpdateName(ObjectUpdateName { entity, id, name }) = action {
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

            updates.queue(persist::object::Name::new(id, name.clone()));

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

into_action!(ObjectRemove);

pub fn object_remove_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    mut objects: ResMut<Objects>,
    mut updates: ResMut<Updates>,
    container_query: Query<&Container>,
    location_query: Query<&Location>,
    mut contents_query: Query<&mut Contents>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectRemove(ObjectRemove { entity, id }) = action {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let container = if let Ok(container) = container_query.get(object_entity) {
                container.entity
            } else if let Ok(location) = location_query.get(object_entity) {
                location.room
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!("Object {} not in a location or container.", id));
                }
                continue;
            };

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
pub struct ObjectUpdateFlags {
    pub entity: Entity,
    pub id: ObjectId,
    pub flags: Vec<String>,
    pub clear: bool,
}

into_action!(ObjectUpdateFlags);

pub fn object_update_flags_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<(&Object, &mut types::Flags)>,
    mut messages: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectUpdateFlags(ObjectUpdateFlags {
            entity,
            id,
            flags,
            clear,
        }) = action
        {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages.get_mut(*entity) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let changed_flags = match ObjectFlags::try_from(flags.as_slice()) {
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

                if *clear {
                    flags.flags.remove(changed_flags);
                } else {
                    flags.flags.insert(changed_flags);
                }

                (object.id, flags.flags)
            };

            updates.queue(persist::object::Flags::new(id, flags));

            if let Ok(mut messages) = messages.get_mut(*entity) {
                messages.queue(format!("Updated object {} flags.", id));
            }
        }
    }
}
