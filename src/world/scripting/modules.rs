use std::sync::{Arc, RwLock};

use bevy_ecs::prelude::{Entity, World};
use rhai::plugin::*;

#[derive(Clone)]
pub struct Me {
    pub entity: Entity,
    pub world: Arc<RwLock<World>>,
}

#[export_module]
pub mod event_api {
    use rhai::Dynamic;

    use crate::world::action::Action;

    #[rhai_fn(get = "actor", pure)]
    pub fn get_actor(action_event: &mut Action) -> Dynamic {
        Dynamic::from(action_event.enactor())
    }
}

#[export_module]
pub mod world_api {
    use std::sync::{Arc, RwLock};

    use bevy_ecs::prelude::{Entity, World};
    use rhai::Dynamic;

    use crate::world::types::{
        object::Object, player::Player, room::Room, Container, Contents, Description, Keywords,
        Location, Named,
    };

    #[rhai_fn(pure)]
    pub fn is_player(world: &mut Arc<RwLock<World>>, entity: Entity) -> bool {
        world.read().unwrap().entity(entity).contains::<Player>()
    }

    #[rhai_fn(pure)]
    pub fn is_room(world: &mut Arc<RwLock<World>>, entity: Entity) -> bool {
        world.read().unwrap().entity(entity).contains::<Room>()
    }

    #[rhai_fn(pure)]
    pub fn is_object(world: &mut Arc<RwLock<World>>, entity: Entity) -> bool {
        world.read().unwrap().entity(entity).contains::<Object>()
    }

    #[rhai_fn(pure)]
    pub fn get_name(world: &mut Arc<RwLock<World>>, entity: Entity) -> Dynamic {
        if let Some(named) = world.read().unwrap().get::<Named>(entity) {
            Dynamic::from(named.name.clone())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_description(world: &mut Arc<RwLock<World>>, entity: Entity) -> Dynamic {
        if let Some(description) = world.read().unwrap().get::<Description>(entity) {
            Dynamic::from(description.text.clone())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_keywords(world: &mut Arc<RwLock<World>>, entity: Entity) -> Dynamic {
        if let Some(keywords) = world.read().unwrap().get::<Keywords>(entity) {
            Dynamic::from(keywords.list.clone())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_location(world: &mut Arc<RwLock<World>>, entity: Entity) -> Dynamic {
        if let Some(location) = world.read().unwrap().get::<Location>(entity) {
            Dynamic::from(location.room)
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_container(world: &mut Arc<RwLock<World>>, entity: Entity) -> Dynamic {
        if let Some(container) = world.read().unwrap().get::<Container>(entity) {
            Dynamic::from(container.entity)
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_contents(world: &mut Arc<RwLock<World>>, entity: Entity) -> Dynamic {
        if let Some(contents) = world.read().unwrap().get::<Contents>(entity) {
            Dynamic::from(contents.objects.clone())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_players(world: &mut Arc<RwLock<World>>, entity: Entity) -> Dynamic {
        if let Some(room) = world.read().unwrap().get::<Room>(entity) {
            Dynamic::from(room.players.clone())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure, name = "!=")]
    pub fn entity_ne(a: &mut Entity, b: Entity) -> bool {
        *a != b
    }

    #[rhai_fn(pure, name = "==")]
    pub fn entity_eq(a: &mut Entity, b: Entity) -> bool {
        *a == b
    }
}

#[export_module]
pub mod self_api {
    use bevy_app::Events;

    use crate::world::{
        action::communicate::{Emote, Message, Say, SendMessage},
        scripting::{modules::Me, QueuedAction},
    };

    #[rhai_fn(pure, get = "entity")]
    pub fn get_entity(me: &mut Me) -> Entity {
        me.entity
    }

    #[rhai_fn(pure)]
    pub fn emote(me: &mut Me, emote: String) {
        me.world
            .write()
            .unwrap()
            .get_resource_mut::<Events<QueuedAction>>()
            .unwrap()
            .send(QueuedAction::new(
                Emote {
                    entity: me.entity,
                    emote,
                }
                .into(),
            ));
    }

    #[rhai_fn(pure)]
    pub fn message(me: &mut Me, message: String) {
        me.world
            .write()
            .unwrap()
            .get_resource_mut::<Events<QueuedAction>>()
            .unwrap()
            .send(QueuedAction::new(
                Message {
                    entity: me.entity,
                    message,
                }
                .into(),
            ));
    }

    #[rhai_fn(pure)]
    pub fn say(me: &mut Me, message: String) {
        me.world
            .write()
            .unwrap()
            .get_resource_mut::<Events<QueuedAction>>()
            .unwrap()
            .send(QueuedAction::new(
                Say {
                    entity: me.entity,
                    message,
                }
                .into(),
            ));
    }

    #[rhai_fn(pure)]
    pub fn send(me: &mut Me, recipient: String, message: String) {
        me.world
            .write()
            .unwrap()
            .get_resource_mut::<Events<QueuedAction>>()
            .unwrap()
            .send(QueuedAction::new(
                SendMessage {
                    entity: me.entity,
                    recipient,
                    message,
                }
                .into(),
            ));
    }
}
