use bevy_ecs::prelude::Entity;
use rhai::plugin::*;

#[derive(Clone)]
pub struct Me {
    entity: Entity,
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

    use bevy_app::Events;
    use bevy_ecs::prelude::{Entity, World};
    use rhai::Dynamic;

    use crate::world::{action::communicate::Say, scripting::QueuedAction, types::Named};

    #[rhai_fn(pure)]
    pub fn get_name(world: &mut Arc<RwLock<World>>, entity: Entity) -> Dynamic {
        if let Some(named) = world.read().unwrap().get::<Named>(entity) {
            Dynamic::from(named.name.clone())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn say(world: &mut Arc<RwLock<World>>, who: Entity, message: String) {
        world
            .write()
            .unwrap()
            .get_resource_mut::<Events<QueuedAction>>()
            .unwrap()
            .send(QueuedAction::new(
                Say {
                    entity: who,
                    message,
                }
                .into(),
            ));
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
