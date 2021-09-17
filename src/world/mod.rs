#![allow(clippy::type_complexity)]

pub mod action;
mod scripting;
pub mod types;

use std::{
    collections::VecDeque,
    convert::TryFrom,
    ops::DerefMut,
    sync::{Arc, RwLock},
};

use bevy_app::Events;
use bevy_ecs::prelude::*;
use itertools::Itertools;
use lazy_static::lazy_static;
use rhai::{Dynamic, Scope};

use crate::{
    engine::persist::{self, DynUpdate, Updates},
    world::{
        action::{register_action_systems, system::Logout, ActionEvent},
        scripting::{
            create_script_engine, post_action_script_system, pre_action_script_system,
            script_compiler_system, CompiledScript, PreAction, Script, ScriptEngine, ScriptName,
            ScriptRun, ScriptRuns, Scripts, Trigger,
        },
        types::{
            player::{Messages, Player, Players},
            room::{Room, RoomBundle, RoomId, Rooms},
            Configuration, Contents, Description, Id, Location, Named,
        },
    },
};

pub const STAGE_FIRST: &str = "first";
pub const STAGE_UPDATE: &str = "update";

lazy_static! {
    pub static ref VOID_ROOM_ID: RoomId = RoomId::try_from(0).unwrap();
}

pub struct GameWorld {
    world: Arc<RwLock<World>>,
    pre_event_schedule: Schedule,
    update_schedule: Schedule,
    post_event_schedule: Schedule,
}

impl GameWorld {
    pub fn new(mut world: World) -> Self {
        let (mut pre_event_schedule, mut update_schedule, mut post_event_schedule) =
            build_schedules();

        // Add resources
        world.insert_resource(Updates::default());
        world.insert_resource(Players::default());
        world.insert_resource(Scripts::default());
        world.insert_resource(ScriptRuns::default());
        world.insert_resource(create_script_engine());

        // Add events
        add_events::<PreAction>(&mut world, &mut update_schedule);
        add_events::<ActionEvent>(&mut world, &mut update_schedule);

        // Create emergency room
        add_void_room(&mut world);

        // Configure schedule systems
        let update = update_schedule
            .get_stage_mut::<SystemStage>(&STAGE_UPDATE)
            .unwrap();
        register_action_systems(update);

        pre_event_schedule.add_system_to_stage(STAGE_UPDATE, pre_action_script_system.system());
        update_schedule.add_system_to_stage(STAGE_UPDATE, script_compiler_system.system());
        post_event_schedule.add_system_to_stage(STAGE_UPDATE, post_action_script_system.system());

        let test_script_name = ScriptName::from("test_script");
        let test_script = Script {
            name: test_script_name.clone(),
            trigger: Trigger::Say,
            code: r#"
            let player = EVENT.entity;
            let name = WORLD.get_name(player);
            let output = `Hello there, ${name}.`;
        "#
            .to_string(),
        };

        let test_script_entity = world.spawn().insert(test_script).id();
        world
            .get_resource_mut::<Scripts>()
            .unwrap()
            .insert(test_script_name, test_script_entity);

        let world = Arc::new(RwLock::new(world));

        GameWorld {
            world,
            pre_event_schedule,
            update_schedule,
            post_event_schedule,
        }
    }

    pub async fn run(&mut self) {
        let world = self.world.clone();

        self.pre_event_schedule
            .run(world.write().unwrap().deref_mut());

        let mut runs = Vec::new();
        std::mem::swap(
            &mut runs,
            &mut world
                .write()
                .unwrap()
                .get_resource_mut::<ScriptRuns>()
                .unwrap()
                .runs,
        );

        for (action, runs) in runs.drain(..) {
            let mut allowed = true;

            for ScriptRun { entity, script } in runs {
                if !self.run_pre_script(&action, entity, script) {
                    allowed = false
                }
            }

            if allowed {
                world
                    .write()
                    .unwrap()
                    .get_resource_mut::<Events<ActionEvent>>()
                    .unwrap()
                    .send(action);
            }
        }

        self.update_schedule
            .run_once(world.write().unwrap().deref_mut());

        self.post_event_schedule
            .run(world.write().unwrap().deref_mut());

        let mut runs = Vec::new();
        std::mem::swap(
            &mut runs,
            &mut world
                .write()
                .unwrap()
                .get_resource_mut::<ScriptRuns>()
                .unwrap()
                .runs,
        );

        for (action, runs) in runs.drain(..) {
            for ScriptRun { entity, script } in runs {
                self.run_script(&action, entity, script);
            }
        }
    }

    pub async fn should_shutdown(&self) -> bool {
        self.world
            .read()
            .unwrap()
            .get_resource::<Configuration>()
            .map_or(true, |configuration| configuration.shutdown)
    }

    pub async fn despawn_player(&mut self, player: Entity) -> anyhow::Result<()> {
        self.player_action(ActionEvent::from(Logout { entity: player }))
            .await;

        let mut world = self.world.write().unwrap();

        let (name, room) = world
            .query::<(&Named, &Location)>()
            .get(&*world, player)
            .map(|(named, location)| (named.name.clone(), location.room))
            .ok()
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        if let Some(objects) = world
            .get::<Contents>(player)
            .map(|contents| contents.objects.clone())
        {
            for object in objects {
                world.despawn(object);
            }
        }
        world.despawn(player);
        world
            .get_resource_mut::<Players>()
            .unwrap()
            .remove(name.as_str());
        world
            .get_mut::<Room>(room)
            .ok_or(action::Error::MissingComponent(room, "Room"))?
            .remove_player(player);

        Ok(())
    }

    pub async fn player_action(&mut self, action: ActionEvent) {
        let mut world = self.world.write().unwrap();

        match world.get_mut::<Messages>(action.enactor()) {
            Some(mut messages) => messages.received_input = true,
            None => {
                world.entity_mut(action.enactor()).insert(Messages {
                    received_input: true,
                    queue: VecDeque::new(),
                });
            }
        }

        world
            .get_resource_mut::<Events<PreAction>>()
            .unwrap()
            .send(PreAction { action });
    }

    pub async fn player_online(&self, name: &str) -> bool {
        self.world
            .read()
            .unwrap()
            .get_resource::<Players>()
            .unwrap()
            .by_name(name)
            .is_some()
    }

    pub async fn spawn_room(&self) -> RoomId {
        self.world
            .read()
            .unwrap()
            .get_resource::<Configuration>()
            .unwrap()
            .spawn_room
    }

    pub async fn messages(&mut self) -> Vec<(Entity, VecDeque<String>)> {
        let mut world = self.world.write().unwrap();

        let players_with_messages = world
            .query_filtered::<Entity, (With<Player>, With<Messages>)>()
            .iter(&world)
            .collect_vec();

        let mut outgoing = Vec::new();

        for player in players_with_messages {
            let mut messages = world.get_mut::<Messages>(player).unwrap();

            if messages.queue.is_empty() {
                continue;
            }

            let mut queue = VecDeque::new();
            std::mem::swap(&mut queue, &mut messages.queue);

            if !messages.received_input {
                queue.push_front("\r\n".to_string());
            }

            messages.received_input = false;

            outgoing.push((player, queue));
        }

        outgoing
    }

    pub async fn updates(&mut self) -> Vec<DynUpdate> {
        self.world
            .write()
            .unwrap()
            .get_resource_mut::<Updates>()
            .unwrap()
            .take()
    }

    pub fn get_world(&self) -> Arc<RwLock<World>> {
        self.world.clone()
    }

    fn run_pre_script(&self, event: &ActionEvent, entity: Entity, script: ScriptName) -> bool {
        let world = self.world.clone();

        if let Some(script) = world
            .read()
            .unwrap()
            .get_resource::<Scripts>()
            .unwrap()
            .by_name(&script)
        {
            if let Some(compiled_script) = world.read().unwrap().get::<CompiledScript>(script) {
                let engine = world
                    .read()
                    .unwrap()
                    .get_resource::<ScriptEngine>()
                    .unwrap()
                    .get();

                let mut scope = Scope::new();
                scope.push_constant("SELF", entity);
                scope.push_constant("WORLD", world.clone());
                scope.push_constant("EVENT", event.clone());
                scope.push_dynamic("allow_action", Dynamic::from(true));

                engine
                    .read()
                    .unwrap()
                    .consume_ast_with_scope(&mut scope, &compiled_script.ast)
                    .unwrap();

                return scope.get_value("allow_action").unwrap();
            } else {
                tracing::warn!(
                    "Skipping execution of {:?}, compiled script not found.",
                    script
                );
            };
        }

        false
    }

    fn run_script(&self, event: &ActionEvent, entity: Entity, script: ScriptName) {
        let world = self.world.clone();

        if let Some(script) = world
            .read()
            .unwrap()
            .get_resource::<Scripts>()
            .unwrap()
            .by_name(&script)
        {
            if let Some(compiled_script) = world.read().unwrap().get::<CompiledScript>(script) {
                let engine = world
                    .read()
                    .unwrap()
                    .get_resource::<ScriptEngine>()
                    .unwrap()
                    .get();

                let mut scope = Scope::new();
                scope.push_constant("SELF", entity);
                scope.push_constant("WORLD", world.clone());
                scope.push_constant("EVENT", event.clone());

                tracing::info!("running script");
                engine
                    .read()
                    .unwrap()
                    .consume_ast_with_scope(&mut scope, &compiled_script.ast)
                    .unwrap();
                tracing::info!("done running script");

                let output = scope.get_value::<String>("output");
                tracing::info!("script output: {:?}", output);
            } else {
                tracing::warn!(
                    "Skipping execution of {:?}, compiled script not found.",
                    script
                );
            };
        };
    }
}

fn add_void_room(world: &mut World) {
    if world
        .get_resource::<Rooms>()
        .unwrap()
        .by_id(*VOID_ROOM_ID)
        .is_none()
    {
        let description = "A dark void extends infinitely in all directions.".to_string();
        let bundle = RoomBundle {
            id: Id::Room(*VOID_ROOM_ID),
            room: Room::new(*VOID_ROOM_ID),
            description: Description {
                text: description.clone(),
            },
            contents: Contents::default(),
        };
        let void_room = world.spawn().insert_bundle(bundle).id();
        world
            .get_resource_mut::<Rooms>()
            .unwrap()
            .insert(*VOID_ROOM_ID, void_room);

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::room::New::new(*VOID_ROOM_ID, description));

        tracing::warn!("Void room was deleted and has been recreated.");
    }
}

fn build_schedules() -> (Schedule, Schedule, Schedule) {
    let mut pre_event_schedule = Schedule::default();
    pre_event_schedule.add_stage(STAGE_UPDATE, SystemStage::parallel());

    let mut update_schedule = Schedule::default();
    update_schedule.add_stage(STAGE_FIRST, SystemStage::parallel());
    update_schedule.add_stage_after(STAGE_FIRST, STAGE_UPDATE, SystemStage::parallel());

    let mut post_event_schedule = Schedule::default();
    post_event_schedule.add_stage(STAGE_UPDATE, SystemStage::parallel());

    (pre_event_schedule, update_schedule, post_event_schedule)
}

fn add_events<T: 'static + Send + Sync>(world: &mut World, schedule: &mut Schedule) {
    world.insert_resource(Events::<T>::default());
    schedule.add_system_to_stage(STAGE_FIRST, Events::<T>::update_system.system());
}
