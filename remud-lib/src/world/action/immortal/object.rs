use std::{convert::TryFrom, str::FromStr};

use bevy_app::{EventReader, EventWriter};
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, UpdateGroup, Updates},
    text::Tokenizer,
    world::{
        action::{
            get_room_std,
            immortal::{Initialize, ShowError, UpdateDescription, UpdateName},
            into_action, Action, Mode,
        },
        fsm::StateMachine,
        scripting::{
            time::Timers, ExecutionErrors, RunInitScript, ScriptData, ScriptHook, ScriptHooks,
            ScriptName, ScriptTrigger,
        },
        types::{
            object::{
                Flags, InheritableFields, Keywords, Object, ObjectBundle, ObjectFlags, ObjectId,
                ObjectOrPrototype, Objects, Prototype, PrototypeId, Prototypes,
            },
            player::{Messages, Player},
            room::Room,
            ActionTarget, Contents, Description, Id, Location, Named,
        },
    },
};

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
                        "errors" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a script to look for its errors.".to_string())
                            } else {
                                let script =
                                    ScriptName::try_from(tokenizer.next().unwrap().to_string())
                                        .map_err(|e| e.to_string())?;
                                Ok(Action::from(ShowError {
                                    actor: player,
                                    target: ActionTarget::Object(id),
                                    script,
                                }))
                            }
                        }
                        "info" => Ok(Action::from(ObjectInfo { actor: player, id })),
                        "inherit" => {
                            if tokenizer.rest().is_empty() {
                                Err("Enter a space separated list of fields to inherit."
                                    .to_string())
                            } else {
                                match tokenizer
                                    .rest()
                                    .split_whitespace()
                                    .map(InheritableFields::from_str)
                                    .try_collect()
                                {
                                    Ok(fields) => Ok(Action::from(ObjectInheritFields {
                                        actor: player,
                                        id,
                                        fields,
                                    })),
                                    Err(_) => Err("Enter valid inheritable fields: desc, flags, \
                                                   keywords, name, and scripts"
                                        .to_string()),
                                }
                            }
                        }
                        "init" => Ok(Action::Initialize(Initialize {
                            actor: player,
                            target: ActionTarget::Object(id),
                        })),
                        "keywords" => {
                            if let Some(mode) = tokenizer.next() {
                                let mode = match Mode::from_str(mode) {
                                    Ok(mode) => mode,
                                    Err(_) => {
                                        return Err("Enter a valid keyword alteration mode: add, \
                                                    remove, or set."
                                            .to_string())
                                    }
                                };

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
                                        id: ObjectOrPrototype::Object(id),
                                        mode,
                                        keywords,
                                    }))
                                }
                            } else {
                                Err("Enter a keyword alteration mode: add, remove, or set."
                                    .to_string())
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
                        "remove" => Ok(Action::from(ObjectRemove { actor: player, id })),
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
                                    id: ObjectOrPrototype::Object(id),
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
                                    id: ObjectOrPrototype::Object(id),
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
                        _ => Err(
                            "Enter a valid object subcommand: desc, info, keywords, name, remove, \
                             set, or unset."
                                .to_string(),
                        ),
                    }
                } else {
                    Err(
                        "Enter an object subcommand: desc, info, keywords, name, remove, set, or \
                         unset."
                            .to_string(),
                    )
                }
            }
        }
    } else {
        Err("Enter an object ID or subcommand: new.".to_string())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ObjectCreate {
    pub actor: Entity,
    pub prototype_id: PrototypeId,
}

into_action!(ObjectCreate);

#[tracing::instrument(name = "object create system", skip_all)]
pub fn object_create_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    mut init_writer: EventWriter<RunInitScript>,
    prototypes: Res<Prototypes>,
    mut objects: ResMut<Objects>,
    mut updates: ResMut<Updates>,
    prototypes_query: Query<(
        &Named,
        &Description,
        &ObjectFlags,
        &Keywords,
        Option<&ScriptHooks>,
    )>,
    player_query: Query<(Option<&Location>, Option<&Room>)>,
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

            let (named, description, flags, keywords, hooks) =
                prototypes_query.get(prototype).unwrap();

            let room_entity = get_room_std(*actor, &player_query);

            let id = objects.next_id();

            let mut e = commands.spawn_bundle(ObjectBundle {
                object: Object::new(id, prototype, true),
                id: Id::Object(id),
                name: named.clone(),
                description: description.clone(),
                flags: flags.clone(),
                keywords: keywords.clone(),
                location: Location::from(room_entity),
            });

            if let Some(hooks) = hooks {
                e.insert(hooks.clone());
            }

            let object_entity = e.id();

            if let Some(hooks) = hooks {
                for script in hooks.by_trigger(ScriptTrigger::Init) {
                    init_writer.send(RunInitScript::new(object_entity, script));
                }
            }

            let room_id = {
                let (room, mut contents) = room_query.get_mut(room_entity).unwrap();
                contents.insert(object_entity);
                room.id()
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ObjectInfo {
    pub actor: Entity,
    pub id: ObjectId,
}

into_action!(ObjectInfo);

#[tracing::instrument(name = "object info system", skip_all)]
pub fn object_info_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    object_query: Query<(
        &Object,
        &Named,
        &Description,
        &ObjectFlags,
        &Keywords,
        Option<&ScriptHooks>,
        Option<&Location>,
        Option<&Timers>,
        Option<&StateMachine>,
        Option<&ScriptData>,
        Option<&ExecutionErrors>,
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

            let (
                object,
                named,
                description,
                flags,
                keywords,
                hooks,
                location,
                timers,
                fsm,
                data,
                errors,
            ) = object_query.get(object_entity).unwrap();

            let prototype_id = prototype_query.get(object.prototype()).unwrap().id();

            let mut message = format!("|white|Object {}|-|", object.id());

            message.push_str("\r\n  |white|prototype|-|: ");
            message.push_str(prototype_id.to_string().as_str());

            message.push_str("\r\n  |white|inherit scripts|-|: ");
            message.push_str(object.inherit_scripts().to_string().as_str());

            message.push_str("\r\n  |white|name|-|: ");
            message.push_str(named.escaped().as_str());

            message.push_str("\r\n  |white|description|-|: ");
            message.push_str(description.escaped().as_str());

            message.push_str("\r\n  |white|flags|-|: ");
            message.push_str(format!("{:?}", flags.get_flags()).as_str());

            message.push_str("\r\n  |white|keywords|-|: ");
            message.push_str(keywords.as_word_list().as_str());

            message.push_str("\r\n  |white|location|-|: ");
            if let Some(location) = location {
                if let Ok(room) = room_query.get(location.location()) {
                    message.push_str("room ");
                    message.push_str(room.id().to_string().as_str());
                } else if let Ok(named) = player_query.get(location.location()) {
                    message.push_str("player ");
                    message.push_str(named.as_str());
                } else {
                    message.push_str("other ");
                    message.push_str(format!("{:?}", location.location()).as_str());
                }
            }

            message.push_str("\r\n  |white|script hooks|-|:");
            if let Some(hooks) = hooks {
                if hooks.is_empty() {
                    message.push_str(" none");
                } else {
                    for ScriptHook { trigger, script } in hooks.hooks().iter() {
                        message.push_str(format!("\r\n    {:?} -> {}", trigger, script).as_str());

                        if errors.map(|e| e.has_error(script)).unwrap_or(false) {
                            message.push_str(" |red|(error)|-|");
                        }
                    }
                }
            } else {
                message.push_str(" none");
            }

            message.push_str("\r\n  |white|script data|-|:");
            if let Some(data) = data {
                if data.is_empty() {
                    message.push_str(" none");
                } else {
                    for (k, v) in data.map() {
                        message.push_str(format!("\r\n    {} -> {:?}", k, v).as_str());
                    }
                }
            } else {
                message.push_str(" none");
            }

            message.push_str("\r\n  |white|timers|-|:");
            if let Some(timers) = timers {
                if timers.timers().is_empty() {
                    message.push_str(" none");
                }
                for (name, timer) in timers.timers().iter() {
                    message.push_str(
                        format!(
                            "\r\n    {}: {}/{}ms",
                            name,
                            timer.elapsed().as_millis(),
                            timer.duration().as_millis()
                        )
                        .as_str(),
                    )
                }
            } else {
                message.push_str(" none");
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ObjectInheritFields {
    pub actor: Entity,
    pub id: ObjectId,
    pub fields: Vec<InheritableFields>,
}

into_action!(ObjectInheritFields);

#[tracing::instrument(name = "object inherit fields system", skip_all)]
pub fn object_inherit_fields_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    mut object_query: Query<&mut Object>,
    prototype_query: Query<(
        &Named,
        &Description,
        &ObjectFlags,
        &Keywords,
        Option<&ScriptHooks>,
    )>,
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
                prototype_query.get(object.prototype()).unwrap();

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
                    InheritableFields::Scripts => {
                        object.set_inherit_scripts(true);
                        if let Some(hooks) = hooks {
                            commands.entity(object_entity).insert(hooks.clone());
                        } else {
                            commands.entity(object_entity).remove::<ScriptHooks>();
                        }
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UpdateObjectFlags {
    pub actor: Entity,
    pub id: ObjectOrPrototype,
    pub flags: Vec<String>,
    pub clear: bool,
}

into_action!(UpdateObjectFlags);

#[tracing::instrument(name = "object flags system", skip_all)]
pub fn update_object_flags(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    prototypes: Res<Prototypes>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<&mut ObjectFlags>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::UpdateObjectFlags(UpdateObjectFlags {
            actor,
            id,
            flags,
            clear,
        }) = action
        {
            let entity = match id {
                ObjectOrPrototype::Object(id) => {
                    if let Some(object) = objects.by_id(*id) {
                        object
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Object {} not found.", id));
                        }
                        continue;
                    }
                }
                ObjectOrPrototype::Prototype(id) => {
                    if let Some(prototype) = prototypes.by_id(*id) {
                        prototype
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Prototype {} not found.", id));
                        }
                        continue;
                    }
                }
            };

            let changed_flags = match Flags::try_from(flags.as_slice()) {
                Ok(flags) => flags,
                Err(e) => {
                    if let Ok(mut messages) = messages_query.get_mut(*actor) {
                        messages.queue(e.to_string());
                    }
                    continue;
                }
            };

            let mut flags = object_query.get_mut(entity).unwrap();

            if *clear {
                flags.remove(changed_flags);
            } else {
                flags.insert(changed_flags);
            }

            match id {
                ObjectOrPrototype::Object(id) => {
                    updates.persist(persist::object::Flags::new(*id, flags.get_flags()));
                }
                ObjectOrPrototype::Prototype(id) => {
                    updates.persist(persist::prototype::Flags::new(*id, flags.get_flags()));
                    updates.reload(*id);
                }
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Updated {} flags.", id));
            }
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct UpdateKeywords {
    pub actor: Entity,
    pub id: ObjectOrPrototype,
    pub mode: Mode,
    pub keywords: Vec<String>,
}

into_action!(UpdateKeywords);

#[tracing::instrument(name = "object keywords system", skip_all)]
pub fn update_keywords_system(
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    prototypes: Res<Prototypes>,
    mut updates: ResMut<Updates>,
    mut object_query: Query<&mut Keywords>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::UpdateKeywords(UpdateKeywords {
            actor,
            id,
            mode,
            keywords,
        }) = action
        {
            let entity = match id {
                ObjectOrPrototype::Object(id) => {
                    if let Some(object) = objects.by_id(*id) {
                        object
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Object {} not found.", id));
                        }
                        continue;
                    }
                }
                ObjectOrPrototype::Prototype(id) => {
                    if let Some(prototype) = prototypes.by_id(*id) {
                        prototype
                    } else {
                        if let Ok(mut messages) = messages_query.get_mut(*actor) {
                            messages.queue(format!("Prototype {} not found.", id));
                        }
                        continue;
                    }
                }
            };

            let keywords = match mode {
                Mode::Add => {
                    let mut component = object_query.get_mut(entity).unwrap();
                    component.add(keywords.clone());
                    component.get_list()
                }
                Mode::Remove => {
                    let mut component = object_query.get_mut(entity).unwrap();
                    component.remove(keywords.as_slice());
                    component.get_list()
                }
                Mode::Set => {
                    object_query
                        .get_mut(entity)
                        .unwrap()
                        .set_list(keywords.clone());
                    keywords.clone()
                }
            };

            match id {
                ObjectOrPrototype::Object(id) => {
                    updates.persist(persist::object::Keywords::new(*id, keywords));
                }
                ObjectOrPrototype::Prototype(id) => {
                    updates.persist(persist::prototype::Keywords::new(*id, keywords));
                    updates.reload(*id);
                }
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Updated {} keywords.", id));
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ObjectRemove {
    pub actor: Entity,
    pub id: ObjectId,
}

into_action!(ObjectRemove);

#[tracing::instrument(name = "remove object system", skip_all)]
pub fn object_remove_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    mut objects: ResMut<Objects>,
    mut updates: ResMut<Updates>,
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

            let location = location_query.get(object_entity).unwrap().location();

            objects.remove(*id);
            commands.entity(object_entity).despawn();
            contents_query
                .get_mut(location)
                .unwrap()
                .remove(object_entity);

            updates.persist(persist::object::Remove::new(*id));

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Removed object {}.", id));
            }
        }
    }
}
