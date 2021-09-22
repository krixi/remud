use std::{convert::TryFrom, str::FromStr};

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, UpdateGroup, Updates},
    into_action,
    text::{word_list, Tokenizer},
    world::{
        action::{
            immortal::{UpdateDescription, UpdateName},
            Action,
        },
        fsm::StateMachine,
        scripting::{ScriptHook, ScriptHooks},
        types::{
            self,
            object::{
                InheritableFields, Object, ObjectBundle, ObjectFlags, ObjectId, Objects, Prototype,
                PrototypeId, Prototypes,
            },
            player::{Messages, Player},
            room::Room,
            ActionTarget, Container, Contents, Description, Flags, Id, Keywords, Location, Named,
        },
    },
};

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
            "new" => {
                if let Some(id) = tokenizer.next() {
                    let prototype_id = PrototypeId::from_str(id).map_err(|e| e.to_string())?;
                    Ok(Action::from(ObjectCreate {
                        actor: player,
                        prototype_id,
                    }))
                } else {
                    Err("Enter a prototype ID.".to_string())
                }
            }
            maybe_id => {
                let id = ObjectId::from_str(maybe_id).map_err(|e| e.to_string())?;

                if let Some(token) = tokenizer.next() {
                    match token {
                        "info" => Ok(Action::from(ObjectInfo {actor: player, id })),
                        "inherit" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a space separated list of fields to inherit.".to_string())
                            } else {
                                match tokenizer.rest().split_whitespace().map(|s| InheritableFields::from_str(s)).try_collect() {
                                    Ok(fields) => Ok(Action::from(ObjectInheritFields {
                                    actor: player,
                                    id,
                                    fields
                                })),
                                    Err(_) => Err("Enter valid inheritable fields: desc, flags, hooks, keywords, and name".to_string()),
                                }
                            }
                        }
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
                                    actor: player,
                                    id,
                                    keywords,
                                }))
                            }
                        }
                        "desc" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a long description.".to_string())
                            } else {
                                Ok(Action::from(UpdateDescription {
                                    actor: player,
                                    target: ActionTarget::Object(id),
                                    description: tokenizer.rest().to_string(),
                                }))
                            }
                        }
                        "remove" => Ok(Action::from(ObjectRemove { actor: player, id })),
                        "set" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a space separated list of flags. Valid flags: fixed, subtle.".to_string())
                            } else {
                                Ok(Action::from(ObjectUpdateFlags {actor: player, id, flags: tokenizer.rest().to_string().split_whitespace().map(|flag|flag.to_string()).collect_vec(), clear: false}))
                            }
                        }
                        "name" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a short description.".to_string())
                            } else {
                                Ok(Action::from(UpdateName {
                                    actor: player,
                                    target: ActionTarget::Object(id),
                                    name: tokenizer.rest().to_string(),
                                }))
                            }
                        }
                        "unset" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a space separated list of flags. Valid flags: fixed, subtle.".to_string())
                            } else {
                                Ok(Action::from(ObjectUpdateFlags {actor: player, id, flags: tokenizer.rest().to_string().split_whitespace().map(|flag|flag.to_string()).collect_vec(), clear: true}))
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
    pub actor: Entity,
    pub prototype_id: PrototypeId,
}

into_action!(ObjectCreate);

pub fn object_create_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    prototypes: Res<Prototypes>,
    mut objects: ResMut<Objects>,
    mut updates: ResMut<Updates>,
    prototypes_query: Query<(&Named, &Description, &Flags, &Keywords)>,
    player_query: Query<&Location, With<Player>>,
    mut room_query: Query<(&Room, &mut Contents)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectCreate(ObjectCreate {
            actor,
            prototype_id,
        }) = action
        {
            let prototype = match prototypes.by_id(*prototype_id) {
                Some(entity) => entity,
                None => {
                    if let Ok(mut messages) = messages_query.get_mut(*actor) {
                        messages.queue(format!("Prototype {} does not exist.", prototype_id))
                    }
                    continue;
                }
            };

            let (named, description, flags, keywords) = prototypes_query.get(prototype).unwrap();

            let room_entity = player_query
                .get(*actor)
                .map(|location| location.room)
                .unwrap();

            let id = objects.next_id();

            let object_entity = commands
                .spawn_bundle(ObjectBundle {
                    object: Object {
                        id,
                        prototype,
                        inherit_scripts: true,
                    },
                    id: Id::Object(id),
                    flags: flags.clone(),
                    name: named.clone(),
                    description: description.clone(),
                    keywords: keywords.clone(),
                    hooks: ScriptHooks::default(),
                })
                .insert(Location { room: room_entity })
                .id();

            let room_id = {
                let (room, mut contents) = room_query.get_mut(room_entity).unwrap();
                contents.objects.push(object_entity);
                room.id
            };

            updates.persist(UpdateGroup::new(vec![
                persist::object::Create::new(id, *prototype_id),
                persist::room::AddObject::new(room_id, id),
            ]));

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Created object {}.", id));
            }

            objects.insert(id, object_entity);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectInfo {
    pub actor: Entity,
    pub id: ObjectId,
}

into_action!(ObjectInfo);

pub fn object_info_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    object_query: Query<(
        &Object,
        &Named,
        &Description,
        &types::Flags,
        &Keywords,
        &ScriptHooks,
        Option<&Container>,
        Option<&Location>,
        Option<&StateMachine>,
    )>,
    prototype_query: Query<&Prototype>,
    room_query: Query<&Room>,
    player_query: Query<&Named, With<Player>>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectInfo(ObjectInfo { actor, id }) = action {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let (object, named, description, flags, keywords, hooks, container, location, fsm) =
                object_query.get(object_entity).unwrap();

            let prototype_id = prototype_query.get(object.prototype).unwrap().id;

            let mut message = format!("|white|Object {}|-|", object.id);

            message.push_str("\r\n  |white|prototype|-|: ");
            message.push_str(prototype_id.to_string().as_str());

            message.push_str("\r\n  |white|inherit scripts|-|: ");
            message.push_str(&object.inherit_scripts.to_string());

            message.push_str("\r\n  |white|name|-|: ");
            message.push_str(named.name.replace("|", "||").as_str());

            message.push_str("\r\n  |white|description|-|: ");
            message.push_str(description.text.replace("|", "||").as_str());

            message.push_str("\r\n  |white|flags|-|: ");
            message.push_str(format!("{:?}", flags.flags).as_str());

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
            if hooks.list.is_empty() {
                message.push_str(" none");
            } else {
                for ScriptHook { trigger, script } in hooks.list.iter() {
                    message.push_str(format!("\r\n    {:?} -> {}", trigger, script).as_str());
                }
            }

            message.push_str("\r\n  |white|fsm|-|:");
            if let Some(StateMachine { states, current }) = fsm {
                for state in states.keys().sorted() {
                    let mut current_indicator = "";
                    if current == state {
                        current_indicator = "<-";
                    }
                    message.push_str(format!("\r\n    {:?} {}", state, current_indicator).as_str());
                }
            } else {
                message.push_str(" none");
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectInheritFields {
    pub actor: Entity,
    pub id: ObjectId,
    pub fields: Vec<InheritableFields>,
}

into_action!(ObjectInheritFields);

pub fn object_inherit_fields_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    mut object_query: Query<&mut Object>,
    prototype_query: Query<(&Named, &Description, &Flags, &Keywords, &ScriptHooks)>,
    mut updates: ResMut<Updates>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectInheritFields(ObjectInheritFields { actor, id, fields }) = action {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let mut object = object_query.get_mut(object_entity).unwrap();

            let (named, description, flags, keywords, hooks) =
                prototype_query.get(object.prototype).unwrap();

            for field in fields {
                match field {
                    InheritableFields::Name => {
                        commands.entity(object_entity).insert(named.clone());
                    }
                    InheritableFields::Description => {
                        commands.entity(object_entity).insert(description.clone());
                    }
                    InheritableFields::Flags => {
                        commands.entity(object_entity).insert(flags.clone());
                    }
                    InheritableFields::Keywords => {
                        commands.entity(object_entity).insert(keywords.clone());
                    }
                    InheritableFields::Hooks => {
                        object.inherit_scripts = true;
                        commands.entity(object_entity).insert(hooks.clone());
                    }
                }
            }

            updates.persist(persist::object::Inherit::new(*id, fields.clone()));

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Object {} fields set to inherit.", id));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectRemove {
    pub actor: Entity,
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
        if let Action::ObjectRemove(ObjectRemove { actor, id }) = action {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let container = if let Ok(container) = container_query.get(object_entity) {
                container.entity
            } else if let Ok(location) = location_query.get(object_entity) {
                location.room
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
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

            updates.persist(persist::object::Remove::new(*id));

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Object {} removed.", id));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectUpdateKeywords {
    pub actor: Entity,
    pub id: ObjectId,
    pub keywords: Vec<String>,
}

into_action!(ObjectUpdateKeywords);

pub fn object_update_keywords_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<&mut Keywords>,
    mut messages: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectUpdateKeywords(ObjectUpdateKeywords {
            actor,
            id,
            keywords,
        }) = action
        {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages.get_mut(*actor) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            object_query.get_mut(object_entity).unwrap().list = keywords.clone();

            updates.persist(persist::object::Keywords::new(*id, keywords.clone()));

            if let Ok(mut messages) = messages.get_mut(*actor) {
                messages.queue(format!("Updated object {} keywords.", id));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectUpdateFlags {
    pub actor: Entity,
    pub id: ObjectId,
    pub flags: Vec<String>,
    pub clear: bool,
}

into_action!(ObjectUpdateFlags);

pub fn object_update_flags_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<&mut types::Flags>,
    mut messages: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::ObjectUpdateFlags(ObjectUpdateFlags {
            actor,
            id,
            flags,
            clear,
        }) = action
        {
            let object_entity = if let Some(object) = objects.by_id(*id) {
                object
            } else {
                if let Ok(mut messages) = messages.get_mut(*actor) {
                    messages.queue(format!("Object {} not found.", id));
                }
                continue;
            };

            let changed_flags = match ObjectFlags::try_from(flags.as_slice()) {
                Ok(flags) => flags,
                Err(e) => {
                    if let Ok(mut messages) = messages.get_mut(*actor) {
                        messages.queue(e.to_string());
                    }
                    continue;
                }
            };

            let flags = {
                let mut flags = object_query.get_mut(object_entity).unwrap();

                if *clear {
                    flags.flags.remove(changed_flags);
                } else {
                    flags.flags.insert(changed_flags);
                }

                flags.flags
            };

            updates.persist(persist::object::Flags::new(*id, flags));

            if let Ok(mut messages) = messages.get_mut(*actor) {
                messages.queue(format!("Updated object {} flags.", id));
            }
        }
    }
}
