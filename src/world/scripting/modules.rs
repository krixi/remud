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

    use crate::world::action::{communicate::Emote, Action};

    #[rhai_fn(get = "actor", pure)]
    pub fn get_actor(action_event: &mut Action) -> Dynamic {
        Dynamic::from(action_event.actor())
    }

    #[rhai_fn(get = "emote", pure)]
    pub fn get_emote(action_event: &mut Action) -> Dynamic {
        if let Action::Emote(Emote { emote, .. }) = action_event {
            Dynamic::from(rhai::ImmutableString::from(emote.as_str()))
        } else {
            Dynamic::UNIT
        }
    }
}

#[export_module]
pub mod time_api {
    use std::time::Duration;

    pub fn ms(value: i64) -> Duration {
        Duration::from_millis(if value >= 0 { value as u64 } else { 0 })
    }

    pub fn secs(value: i64) -> Duration {
        Duration::from_secs(if value >= 0 { value as u64 } else { 0 })
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
    use std::time::Duration;

    use bevy_app::Events;

    use crate::world::{
        action::{
            communicate::{Emote, Message, Say, SendMessage},
            Action,
        },
        fsm::{StateMachineBuilder, StateMachines},
        scripting::{modules::Me, timed_actions::TimedActions, QueuedAction},
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
            .send(
                Action::from(Emote {
                    actor: me.entity,
                    emote,
                })
                .into(),
            );
    }

    #[rhai_fn(pure)]
    pub fn emote_after(me: &mut Me, duration: Duration, emote: String) {
        me.world
            .write()
            .unwrap()
            .get_resource_mut::<TimedActions>()
            .unwrap()
            .send_after(
                Action::from(Emote {
                    actor: me.entity,
                    emote,
                }),
                duration,
            );
    }

    #[rhai_fn(pure)]
    pub fn pop_fsm(me: &mut Me) {
        if let Some(mut fsms) = me
            .world
            .write()
            .unwrap()
            .get_mut::<StateMachines>(me.entity)
        {
            fsms.pop();
        }
    }

    #[rhai_fn(pure)]
    pub fn push_fsm(me: &mut Me, builder: StateMachineBuilder) {
        match builder.build() {
            Ok(fsm) => {
                if let Some(mut fsms) = me
                    .world
                    .write()
                    .unwrap()
                    .get_mut::<StateMachines>(me.entity)
                {
                    fsms.push(fsm);
                } else {
                    me.world
                        .write()
                        .unwrap()
                        .entity_mut(me.entity)
                        .insert(StateMachines::new(fsm));
                }
            }
            Err(e) => {
                tracing::warn!("Failed to build state machine: {}", e);
            }
        }
    }

    #[rhai_fn(pure)]
    pub fn message(me: &mut Me, message: String) {
        me.world
            .write()
            .unwrap()
            .get_resource_mut::<Events<QueuedAction>>()
            .unwrap()
            .send(
                Action::from(Message {
                    actor: me.entity,
                    message,
                })
                .into(),
            );
    }

    #[rhai_fn(pure)]
    pub fn message_after(me: &mut Me, duration: Duration, message: String) {
        me.world
            .write()
            .unwrap()
            .get_resource_mut::<TimedActions>()
            .unwrap()
            .send_after(
                Action::from(Message {
                    actor: me.entity,
                    message,
                }),
                duration,
            );
    }

    #[rhai_fn(pure)]
    pub fn say(me: &mut Me, message: String) {
        me.world
            .write()
            .unwrap()
            .get_resource_mut::<Events<QueuedAction>>()
            .unwrap()
            .send(
                Action::from(Say {
                    actor: me.entity,
                    message,
                })
                .into(),
            );
    }

    #[rhai_fn(pure)]
    pub fn say_after(me: &mut Me, duration: Duration, message: String) {
        me.world
            .write()
            .unwrap()
            .get_resource_mut::<TimedActions>()
            .unwrap()
            .send_after(
                Action::from(Say {
                    actor: me.entity,
                    message,
                }),
                duration,
            );
    }

    #[rhai_fn(pure)]
    pub fn send(me: &mut Me, recipient: String, message: String) {
        me.world
            .write()
            .unwrap()
            .get_resource_mut::<Events<QueuedAction>>()
            .unwrap()
            .send(
                Action::from(SendMessage {
                    actor: me.entity,
                    recipient,
                    message,
                })
                .into(),
            );
    }

    #[rhai_fn(pure)]
    pub fn send_after(me: &mut Me, duration: Duration, recipient: String, message: String) {
        me.world
            .write()
            .unwrap()
            .get_resource_mut::<TimedActions>()
            .unwrap()
            .send_after(
                Action::from(SendMessage {
                    actor: me.entity,
                    recipient,
                    message,
                }),
                duration,
            );
    }
}

#[export_module]
pub mod states_api {
    use crate::world::fsm::StateId;

    pub const WANDER: &StateId = &StateId::Wander;
    pub const CHASE: &StateId = &StateId::Chase;

    #[rhai_fn(pure, name = "!=")]
    pub fn state_ne(a: &mut StateId, b: StateId) -> bool {
        *a != b
    }

    #[rhai_fn(pure, name = "==")]
    pub fn state_eq(a: &mut StateId, b: StateId) -> bool {
        *a == b
    }
}

#[export_module]
pub mod rand_api {
    use rand::{thread_rng, Rng};

    pub fn chance(probability: f64) -> bool {
        thread_rng().gen_bool(probability)
    }

    pub fn range(start: i64, end: i64) -> i64 {
        thread_rng().gen_range(start..=end)
    }
}
