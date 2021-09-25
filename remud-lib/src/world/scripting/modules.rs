use bevy_ecs::prelude::Entity;
use rhai::plugin::*;

use crate::ecs::SharedWorld;

#[derive(Clone)]
pub struct Me {
    pub entity: Entity,
    pub world: SharedWorld,
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

    use bevy_ecs::prelude::Entity;
    use rhai::Dynamic;

    use crate::{
        ecs::SharedWorld,
        world::types::{
            object::{Container, Keywords, Object},
            player::Player,
            room::Room,
            Contents, Description, Location, Named,
        },
    };

    #[rhai_fn(pure)]
    pub fn is_player(world: &mut SharedWorld, entity: Entity) -> bool {
        world
            .try_read()
            .unwrap()
            .entity(entity)
            .contains::<Player>()
    }

    #[rhai_fn(pure)]
    pub fn is_room(world: &mut SharedWorld, entity: Entity) -> bool {
        world.try_read().unwrap().entity(entity).contains::<Room>()
    }

    #[rhai_fn(pure)]
    pub fn is_object(world: &mut SharedWorld, entity: Entity) -> bool {
        world
            .try_read()
            .unwrap()
            .entity(entity)
            .contains::<Object>()
    }

    #[rhai_fn(pure)]
    pub fn get_name(world: &mut SharedWorld, entity: Entity) -> Dynamic {
        if let Some(named) = world.try_read().unwrap().get::<Named>(entity) {
            Dynamic::from(named.to_string())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_description(world: &mut SharedWorld, entity: Entity) -> Dynamic {
        if let Some(description) = world.try_read().unwrap().get::<Description>(entity) {
            Dynamic::from(description.to_string())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_keywords(world: &mut SharedWorld, entity: Entity) -> Dynamic {
        if let Some(keywords) = world.try_read().unwrap().get::<Keywords>(entity) {
            Dynamic::from(keywords.get_list())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_location(world: &mut SharedWorld, entity: Entity) -> Dynamic {
        if let Some(location) = world.try_read().unwrap().get::<Location>(entity) {
            Dynamic::from(location.room())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_container(world: &mut SharedWorld, entity: Entity) -> Dynamic {
        if let Some(container) = world.try_read().unwrap().get::<Container>(entity) {
            Dynamic::from(container.entity())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_contents(world: &mut SharedWorld, entity: Entity) -> Dynamic {
        if let Some(contents) = world.try_read().unwrap().get::<Contents>(entity) {
            Dynamic::from(contents.get_objects())
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn get_players(world: &mut SharedWorld, entity: Entity) -> Dynamic {
        if let Some(room) = world.try_read().unwrap().get::<Room>(entity) {
            Dynamic::from(room.get_players())
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
    use rhai::ImmutableString;

    use crate::world::{
        action::{
            communicate::{Emote, Message, Say, SendMessage},
            Action,
        },
        fsm::{StateMachineBuilder, StateMachines},
        scripting::{
            modules::Me,
            time::{TimedActions, Timers},
            QueuedAction, ScriptData,
        },
    };

    #[rhai_fn(pure, get = "entity")]
    pub fn get_entity(me: &mut Me) -> Entity {
        me.entity
    }

    #[rhai_fn(pure)]
    pub fn emote(me: &mut Me, emote: String) {
        me.world
            .try_write()
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
            .try_write()
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
    pub fn get(me: &mut Me, key: ImmutableString) -> Dynamic {
        if let Some(data) = me.world.try_read().unwrap().get::<ScriptData>(me.entity) {
            data.get(key)
        } else {
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn message(me: &mut Me, message: String) {
        me.world
            .try_write()
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
            .try_write()
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
    pub fn pop_fsm(me: &mut Me) {
        if let Some(mut fsms) = me
            .world
            .try_write()
            .unwrap()
            .get_mut::<StateMachines>(me.entity)
        {
            fsms.pop();
        }
    }

    #[rhai_fn(pure)]
    pub fn push_fsm(me: &mut Me, builder: StateMachineBuilder) {
        let mut world = me.world.try_write().unwrap();
        match builder.build() {
            Ok(fsm) => {
                if let Some(mut fsms) = world.get_mut::<StateMachines>(me.entity) {
                    fsms.push(fsm);
                } else {
                    world.entity_mut(me.entity).insert(StateMachines::new(fsm));
                }
            }
            Err(e) => {
                tracing::warn!("Failed to build state machine: {}", e);
            }
        }
    }

    #[rhai_fn(pure)]
    pub fn remove(me: &mut Me, key: ImmutableString) -> Dynamic {
        if let Some(mut data) = me
            .world
            .try_write()
            .unwrap()
            .get_mut::<ScriptData>(me.entity)
        {
            data.remove(key)
        } else {
            tracing::info!("script data not found, returning unit");
            Dynamic::UNIT
        }
    }

    #[rhai_fn(pure)]
    pub fn say(me: &mut Me, message: String) {
        me.world
            .try_write()
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
            .try_write()
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
            .try_write()
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
            .try_write()
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

    #[rhai_fn(pure)]
    pub fn set(me: &mut Me, key: ImmutableString, value: Dynamic) {
        let mut world = me.world.try_write().unwrap();
        if let Some(mut data) = world.get_mut::<ScriptData>(me.entity) {
            data.insert(key, value)
        } else {
            world
                .entity_mut(me.entity)
                .insert(ScriptData::new_with_entry(key, value));
        }
    }

    #[rhai_fn(pure)]
    pub fn timer(me: &mut Me, name: ImmutableString, duration: Duration) {
        let mut world = me.world.try_write().unwrap();
        if let Some(mut timers) = world.get_mut::<Timers>(me.entity) {
            timers.add(name.to_string(), duration);
        } else {
            let mut timers = Timers::default();
            timers.add(name.to_string(), duration);
            world.entity_mut(me.entity).insert(timers);
        }
    }

    #[rhai_fn(pure)]
    pub fn timer_repeating(me: &mut Me, name: ImmutableString, duration: Duration) {
        let mut world = me.world.try_write().unwrap();
        if let Some(mut timers) = world.get_mut::<Timers>(me.entity) {
            timers.add_repeating(name.to_string(), duration);
        } else {
            let mut timers = Timers::default();
            timers.add_repeating(name.to_string(), duration);
            world.entity_mut(me.entity).insert(timers);
        }
    }
}

#[export_module]
pub mod rand_api {
    use rand::{prelude::SliceRandom, thread_rng, Rng};

    pub fn chance(probability: f64) -> bool {
        thread_rng().gen_bool(probability)
    }

    pub fn choose(choices: rhai::Array) -> Dynamic {
        if let Some(choice) = choices.choose(&mut thread_rng()) {
            choice.clone()
        } else {
            Dynamic::UNIT
        }
    }

    pub fn range(start: i64, end: i64) -> i64 {
        thread_rng().gen_range(start..=end)
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
pub mod transitions_api {
    use crate::world::fsm::Transition;

    pub const SAW_PLAYER: &Transition = &Transition::SawPlayer;
    pub const LOST_PLAYER: &Transition = &Transition::LostPlayer;

    #[rhai_fn(pure, name = "!=")]
    pub fn tx_ne(a: &mut Transition, b: Transition) -> bool {
        *a != b
    }

    #[rhai_fn(pure, name = "==")]
    pub fn tx_eq(a: &mut Transition, b: Transition) -> bool {
        *a == b
    }
}
