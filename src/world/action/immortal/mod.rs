pub mod object;
pub mod player;
pub mod prototype;
pub mod room;
pub mod script;

use bevy_app::EventReader;
use bevy_ecs::prelude::*;

use crate::{
    engine::persist::{self, Updates},
    into_action,
    world::{
        action::Action,
        types::{
            object::{Objects, Prototypes},
            player::{Messages, Player},
            room::Room,
            ActionTarget, Description, Id, Location, Named,
        },
    },
};

#[derive(Debug, Clone)]
pub struct UpdateDescription {
    pub actor: Entity,
    pub target: ActionTarget,
    pub description: String,
}

into_action!(UpdateDescription);

pub fn update_description_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    prototypes: Res<Prototypes>,
    mut updates: ResMut<Updates>,
    player_query: Query<&Player>,
    location_query: Query<&Location>,
    room_query: Query<&Room>,
    mut description_query: Query<&mut Description>,
    mut messages: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::UpdateDescription(UpdateDescription {
            actor,
            target,
            description,
        }) = action
        {
            let (id, entity) = match target {
                ActionTarget::Object(id) => {
                    if let Some(entity) = objects.by_id(*id) {
                        (Id::Object(*id), entity)
                    } else {
                        if let Ok(mut messages) = messages.get_mut(*actor) {
                            messages.queue(format!("Object {} not found.", id));
                        }
                        continue;
                    }
                }
                ActionTarget::Prototype(id) => {
                    if let Some(entity) = prototypes.by_id(*id) {
                        (Id::Prototype(*id), entity)
                    } else {
                        if let Ok(mut messages) = messages.get_mut(*actor) {
                            messages.queue(format!("Prototype {} not found.", id));
                        }
                        continue;
                    }
                }
                ActionTarget::PlayerSelf => {
                    let id = player_query.get(*actor).unwrap().id;
                    (Id::Player(id), *actor)
                }
                ActionTarget::CurrentRoom => {
                    let room = location_query.get(*actor).unwrap().room;
                    let id = room_query.get(room).unwrap().id;
                    (Id::Room(id), room)
                }
            };

            description_query.get_mut(entity).unwrap().text = description.clone();

            if let Ok(mut desc) = description_query.get_mut(entity) {
                desc.text = description.clone();
            } else {
                commands.entity(entity).insert(Description {
                    text: description.clone(),
                });
            }

            match id {
                Id::Player(id) => {
                    updates.persist(persist::player::Description::new(id, description.clone()))
                }
                Id::Prototype(id) => {
                    updates.persist(persist::prototype::Description::new(
                        id,
                        description.clone(),
                    ));
                    updates.reload(id);
                }
                Id::Object(id) => {
                    updates.persist(persist::object::Description::new(id, description.clone()))
                }
                Id::Room(id) => {
                    updates.persist(persist::room::Description::new(id, description.clone()))
                }
            }

            if let Ok(mut messages) = messages.get_mut(*actor) {
                messages.queue(format!("Updated description for {:?}.", target));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateName {
    pub actor: Entity,
    pub target: ActionTarget,
    pub name: String,
}

into_action!(UpdateName);

pub fn update_name_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    objects: Res<Objects>,
    prototypes: Res<Prototypes>,
    mut updates: ResMut<Updates>,
    location_query: Query<&Location>,
    room_query: Query<&Room>,
    mut name_query: Query<&mut Named>,
    mut messages: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::UpdateName(UpdateName {
            actor,
            target,
            name,
        }) = action
        {
            let (id, entity) = match target {
                ActionTarget::PlayerSelf => todo!(),
                ActionTarget::Prototype(id) => {
                    if let Some(entity) = prototypes.by_id(*id) {
                        (Id::Prototype(*id), entity)
                    } else {
                        if let Ok(mut messages) = messages.get_mut(*actor) {
                            messages.queue(format!("Prototype {} not found.", id));
                        }
                        continue;
                    }
                }
                ActionTarget::Object(id) => {
                    if let Some(entity) = objects.by_id(*id) {
                        (Id::Object(*id), entity)
                    } else {
                        if let Ok(mut messages) = messages.get_mut(*actor) {
                            messages.queue(format!("Object {} not found.", id));
                        }
                        continue;
                    }
                }
                ActionTarget::CurrentRoom => {
                    let room = location_query.get(*actor).unwrap().room;
                    let id = room_query.get(room).unwrap().id;
                    (Id::Room(id), room)
                }
            };

            if let Ok(mut named) = name_query.get_mut(entity) {
                named.name = name.clone();
            } else {
                commands.entity(entity).insert(Named { name: name.clone() });
            }

            match id {
                Id::Prototype(id) => {
                    updates.persist(persist::prototype::Name::new(id, name.clone()));
                    updates.reload(id);
                }
                Id::Object(id) => {
                    updates.persist(persist::object::Name::new(id, name.clone()));
                }
                Id::Player(_) => todo!(),
                Id::Room(id) => {
                    updates.persist(persist::room::Name::new(id, name.clone()));
                }
            }

            if let Ok(mut messages) = messages.get_mut(*actor) {
                messages.queue(format!("Updated {:?} name.", target));
            }
        }
    }
}
