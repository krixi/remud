use std::str::FromStr;

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::{word_list, Tokenizer},
    world::{
        action::{
            immortal::{
                object::{UpdateKeywords, UpdateObjectFlags},
                UpdateDescription, UpdateName,
            },
            into_action, Action,
        },
        scripting::{ScriptHook, ScriptHooks},
        types::{
            object::{
                Container, Keywords, Object, ObjectFlags, ObjectOrPrototype, Prototype,
                PrototypeBundle, PrototypeId, Prototypes,
            },
            player::Messages,
            ActionTarget, Contents, Description, Location, Named,
        },
    },
};

pub const DEFAULT_PROTOTYPE_KEYWORD: &str = "object";
pub const DEFAULT_PROTOTYPE_NAME: &str = "an object";
pub const DEFAULT_PROTOTYPE_DESCRIPTION: &str = "A nondescript object. Completely uninteresting.";

pub fn parse_prototype(player: Entity, mut tokenizer: Tokenizer) -> Result<Action, String> {
    if let Some(token) = tokenizer.next() {
        match token {
            "new" => Ok(Action::from(PrototypeCreate { actor: player })),
            maybe_id => {
                let id = match PrototypeId::from_str(maybe_id) {
                    Ok(id) => id,
                    Err(e) => return Err(e.to_string()),
                };

                if let Some(token) = tokenizer.next() {
                    match token {
                        "info" => Ok(Action::from(PrototypeInfo { actor: player, id })),
                        "keywords" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a space separated list of keywords.".to_string())
                            } else {
                                let keywords = tokenizer
                                    .rest()
                                    .split(' ')
                                    .map(|keyword| keyword.trim().to_string())
                                    .collect_vec();

                                Ok(Action::from(UpdateKeywords {
                                    actor: player,
                                    id: ObjectOrPrototype::Prototype(id),
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
                                    target: ActionTarget::Prototype(id),
                                    description: tokenizer.rest().to_string(),
                                }))
                            }
                        }
                        "remove" => Ok(Action::from(PrototypeRemove { actor: player, id })),
                        "set" => {
                            if tokenizer.rest().is_empty() {
                                Err(
                                    "Enter a space separated list of flags. Valid flags: fixed, \
                                     subtle."
                                        .to_string(),
                                )
                            } else {
                                Ok(Action::from(UpdateObjectFlags {
                                    actor: player,
                                    id: ObjectOrPrototype::Prototype(id),
                                    flags: tokenizer
                                        .rest()
                                        .to_string()
                                        .split_whitespace()
                                        .map(|flag| flag.to_string())
                                        .collect_vec(),
                                    clear: false,
                                }))
                            }
                        }
                        "name" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a short description.".to_string())
                            } else {
                                Ok(Action::from(UpdateName {
                                    actor: player,
                                    target: ActionTarget::Prototype(id),
                                    name: tokenizer.rest().to_string(),
                                }))
                            }
                        }
                        "unset" => {
                            if tokenizer.rest().is_empty() {
                                Err(
                                    "Enter a space separated list of flags. Valid flags: fixed, \
                                     subtle."
                                        .to_string(),
                                )
                            } else {
                                Ok(Action::from(UpdateObjectFlags {
                                    actor: player,
                                    id: ObjectOrPrototype::Prototype(id),
                                    flags: tokenizer
                                        .rest()
                                        .to_string()
                                        .split_whitespace()
                                        .map(|flag| flag.to_string())
                                        .collect_vec(),
                                    clear: true,
                                }))
                            }
                        }
                        _ => Err("Enter a valid prototype subcommand: desc, info, keywords, \
                                  name, remove, set, or unset."
                            .to_string()),
                    }
                } else {
                    Err(
                        "Enter a prototype subcommand: desc, info, keywords, name, remove, set, \
                         or unset."
                            .to_string(),
                    )
                }
            }
        }
    } else {
        Err("Enter a prototype ID or subcommand: new.".to_string())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PrototypeCreate {
    pub actor: Entity,
}

into_action!(PrototypeCreate);

pub fn prototype_create_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    mut prototypes: ResMut<Prototypes>,
    mut updates: ResMut<Updates>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::PrototypeCreate(PrototypeCreate { actor }) = action {
            let id = prototypes.next_id();

            let bundle = PrototypeBundle {
                prototype: Prototype::from(id),
                name: Named::from(DEFAULT_PROTOTYPE_NAME.to_string()),
                description: Description::from(DEFAULT_PROTOTYPE_DESCRIPTION.to_string()),
                flags: ObjectFlags::default(),
                keywords: Keywords::from(vec![DEFAULT_PROTOTYPE_KEYWORD.to_string()]),
            };

            updates.persist(persist::prototype::Create::new(
                id,
                bundle.name.to_string(),
                bundle.description.to_string(),
                bundle.flags.get_flags(),
                bundle.keywords.get_list(),
            ));

            let prototype_entity = commands.spawn_bundle(bundle).id();

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Created prototype {}.", id));
            }

            prototypes.insert(id, prototype_entity);
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PrototypeInfo {
    pub actor: Entity,
    pub id: PrototypeId,
}

into_action!(PrototypeInfo);

pub fn prototype_info_system(
    mut action_reader: EventReader<Action>,
    prototypes: Res<Prototypes>,
    prototype_query: Query<(
        &Prototype,
        &ObjectFlags,
        &Keywords,
        &Named,
        &Description,
        Option<&ScriptHooks>,
    )>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::PrototypeInfo(PrototypeInfo { actor, id }) = action {
            let prototype_entity = if let Some(prototype) = prototypes.by_id(*id) {
                prototype
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Prototype {} not found.", id));
                }
                continue;
            };

            let (prototype, flags, keywords, named, description, hooks) =
                prototype_query.get(prototype_entity).unwrap();

            let mut message = format!("|white|Prototype {}|-|", prototype.id());

            message.push_str("\r\n  |white|name|-|: ");
            message.push_str(named.escaped().as_str());

            message.push_str("\r\n  |white|description|-|: ");
            message.push_str(description.escaped().as_str());
            message.push_str(format!("\r\n  |white|flags|-|: {:?}", flags.get_flags()).as_str());

            message.push_str("\r\n  |white|keywords|-|: ");
            message.push_str(word_list(keywords.get_list()).as_str());

            message.push_str("\r\n  |white|script hooks|-|:");
            if let Some(hooks) = hooks {
                if hooks.is_empty() {
                    message.push_str(" none");
                }
                for ScriptHook { trigger, script } in hooks.hooks().iter() {
                    message.push_str(format!("\r\n    {:?} -> {}", trigger, script).as_str());
                }

                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(message);
                }
            } else {
                message.push_str(" none");
            }
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PrototypeRemove {
    pub actor: Entity,
    pub id: PrototypeId,
}

into_action!(PrototypeRemove);

pub fn prototype_remove_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    mut prototypes: ResMut<Prototypes>,
    mut updates: ResMut<Updates>,
    container_query: Query<&Container>,
    location_query: Query<&Location>,
    object_query: Query<(Entity, &Object)>,
    mut contents_query: Query<&mut Contents>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::PrototypeRemove(PrototypeRemove { actor, id }) = action {
            let prototype_entity = if let Some(prototype) = prototypes.by_id(*id) {
                prototype
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Prototype {} not found.", id));
                }
                continue;
            };

            let container = if let Ok(container) = container_query.get(prototype_entity) {
                container.entity()
            } else if let Ok(location) = location_query.get(prototype_entity) {
                location.room()
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Prototype {} not in a location or container.", id));
                }
                continue;
            };

            prototypes.remove(*id);
            commands.entity(prototype_entity).despawn();
            contents_query
                .get_mut(container)
                .unwrap()
                .remove(prototype_entity);

            let objects = object_query
                .iter()
                .filter(|(_, object)| object.prototype() == prototype_entity)
                .map(|(object_entity, object)| (object_entity, object.id()))
                .collect_vec();

            for (entity, id) in objects {
                commands.entity(entity).despawn();
                updates.persist(persist::object::Remove::new(id));
            }

            updates.persist(persist::prototype::Remove::new(*id));

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Prototype {} removed.", id));
            }
        }
    }
}
