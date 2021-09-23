#![allow(clippy::type_complexity)]

pub mod action;
pub mod fsm;
pub mod scripting;
pub mod types;

use std::{
    collections::VecDeque,
    convert::TryFrom,
    ops::DerefMut,
    str::FromStr,
    sync::{Arc, RwLock},
};

use bevy_app::Events;
use bevy_core::Time;
use bevy_ecs::prelude::*;
use itertools::Itertools;
use lazy_static::lazy_static;
use rhai::ParseError;

use crate::{
    engine::persist::{self, DynPersist, Updates},
    web,
    world::{
        action::{register_action_systems, Action},
        fsm::system::state_machine_system,
        scripting::{
            create_script_engine, init_script_system, post_action_script_system,
            queued_action_script_system, run_init_scripts, run_post_action_scripts,
            run_pre_action_scripts, script_compiler_system,
            timed_actions::{timed_actions_system, TimedActions},
            QueuedAction, Script, ScriptHooks, ScriptInit, ScriptName, ScriptRuns, TriggerEvent,
        },
        types::{
            object::PrototypeId,
            player::{Messages, Player, Players},
            room::{Regions, Room, RoomBundle, RoomId, Rooms},
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
        world.insert_resource(Time::default());
        world.insert_resource(Updates::default());
        world.insert_resource(Players::default());
        world.insert_resource(ScriptRuns::default());
        world.insert_resource(TimedActions::default());
        world.insert_resource(create_script_engine());

        // Add events
        // The ScriptInit resource is added by the DB.
        pre_event_schedule
            .add_system_to_stage(STAGE_FIRST, Events::<ScriptInit>::update_system.system());
        add_events::<QueuedAction>(&mut world, &mut pre_event_schedule);
        add_events::<Action>(&mut world, &mut pre_event_schedule);

        // Create emergency room
        add_void_room(&mut world);

        // Configure schedule systems
        let update = update_schedule
            .get_stage_mut::<SystemStage>(&STAGE_UPDATE)
            .unwrap();
        register_action_systems(update);

        pre_event_schedule.add_system_to_stage(STAGE_FIRST, time_system.exclusive_system());
        pre_event_schedule
            .add_system_to_stage(STAGE_UPDATE, timed_actions_system.system().before("queued"));
        pre_event_schedule.add_system_to_stage(STAGE_UPDATE, init_script_system.system());
        pre_event_schedule.add_system_to_stage(
            STAGE_UPDATE,
            queued_action_script_system.system().label("queued"),
        );
        pre_event_schedule.add_system_to_stage(STAGE_UPDATE, script_compiler_system.system());
        update_schedule.add_system_to_stage(
            STAGE_UPDATE,
            state_machine_system.exclusive_system().at_end(),
        );
        post_event_schedule.add_system_to_stage(STAGE_UPDATE, post_action_script_system.system());

        let world = Arc::new(RwLock::new(world));

        GameWorld {
            world,
            pre_event_schedule,
            update_schedule,
            post_event_schedule,
        }
    }

    pub fn run(&mut self) {
        let world = self.world.clone();

        self.pre_event_schedule
            .run(world.write().unwrap().deref_mut());

        run_init_scripts(world.clone());

        run_pre_action_scripts(world.clone());

        self.update_schedule
            .run_once(world.write().unwrap().deref_mut());

        self.post_event_schedule
            .run(world.write().unwrap().deref_mut());

        run_post_action_scripts(world);
    }

    pub fn should_shutdown(&self) -> bool {
        self.world
            .read()
            .unwrap()
            .get_resource::<Configuration>()
            .map_or(true, |configuration| configuration.shutdown)
    }

    pub fn despawn_player(&mut self, player: Entity) -> anyhow::Result<()> {
        let mut world = self.world.write().unwrap();

        let (name, room) = world
            .query::<(&Named, &Location)>()
            .get(&*world, player)
            .map(|(named, location)| (named.to_string(), location.room()))
            .ok()
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let players = world
            .get::<Room>(room)
            .unwrap()
            .players()
            .iter()
            .filter(|p| **p != player)
            .copied()
            .collect_vec();

        let message = format!("{} leaves.", name);

        for player in players {
            if let Some(mut messages) = world.get_mut::<Messages>(player) {
                messages.queue(message.clone());
            }
        }

        if let Some(objects) = world
            .get::<Contents>(player)
            .map(|contents| contents.get_objects())
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

    pub fn player_action(&mut self, action: Action) {
        let mut world = self.world.write().unwrap();

        world
            .get_mut::<Messages>(action.actor())
            .unwrap()
            .set_received_input();

        world
            .get_resource_mut::<Events<QueuedAction>>()
            .unwrap()
            .send(QueuedAction { action });
    }

    pub fn player_online(&self, name: &str) -> bool {
        self.world
            .read()
            .unwrap()
            .get_resource::<Players>()
            .unwrap()
            .by_name(name)
            .is_some()
    }

    pub fn spawn_room(&self) -> RoomId {
        self.world
            .read()
            .unwrap()
            .get_resource::<Configuration>()
            .unwrap()
            .spawn_room
    }

    pub fn messages(&mut self) -> Vec<(Entity, VecDeque<String>)> {
        let mut world = self.world.write().unwrap();

        let players_with_messages = world
            .query_filtered::<Entity, (With<Player>, With<Messages>)>()
            .iter(&world)
            .collect_vec();

        let mut outgoing = Vec::new();

        for player in players_with_messages {
            let mut messages = world.get_mut::<Messages>(player).unwrap();

            if messages.is_empty() {
                continue;
            }

            outgoing.push((player, messages.get_queue()));
        }

        outgoing
    }

    pub fn updates(&mut self) -> Vec<DynPersist> {
        self.world
            .write()
            .unwrap()
            .get_resource_mut::<Updates>()
            .unwrap()
            .take_updates()
    }

    pub fn prototype_reloads(&mut self) -> Vec<PrototypeId> {
        self.world
            .write()
            .unwrap()
            .get_resource_mut::<Updates>()
            .unwrap()
            .take_reloads()
    }

    pub fn get_world(&self) -> Arc<RwLock<World>> {
        self.world.clone()
    }

    pub fn create_script(
        &mut self,
        name: String,
        trigger: String,
        code: String,
    ) -> Result<Option<ParseError>, web::Error> {
        let name = ScriptName::try_from(name).map_err(|_| web::Error::BadScriptName)?;
        let trigger =
            TriggerEvent::from_str(trigger.as_str()).map_err(|_| web::Error::BadTrigger)?;

        let script = Script {
            name,
            trigger,
            code,
        };

        scripting::actions::create_script(self.world.write().unwrap().deref_mut(), script)
    }

    pub fn read_script(
        &mut self,
        name: String,
    ) -> Result<(Script, Option<ParseError>), web::Error> {
        let name = ScriptName::try_from(name).map_err(|_| web::Error::BadScriptName)?;

        scripting::actions::read_script(&*self.world.read().unwrap(), name)
    }

    pub fn read_all_scripts(&mut self) -> Vec<(Script, Option<ParseError>)> {
        scripting::actions::read_all_scripts(self.world.write().unwrap().deref_mut())
    }

    pub fn update_script(
        &mut self,
        name: String,
        trigger: String,
        code: String,
    ) -> Result<Option<ParseError>, web::Error> {
        let name = ScriptName::try_from(name).map_err(|_| web::Error::BadScriptName)?;
        let trigger =
            TriggerEvent::from_str(trigger.as_str()).map_err(|_| web::Error::BadTrigger)?;

        let script = Script {
            name,
            trigger,
            code,
        };

        scripting::actions::update_script(self.world.write().unwrap().deref_mut(), script)
    }

    pub fn delete_script(&mut self, name: String) -> Result<(), web::Error> {
        let name = ScriptName::try_from(name).map_err(|_| web::Error::BadScriptName)?;

        scripting::actions::delete_script(self.world.write().unwrap().deref_mut(), name)
    }
}

fn time_system(mut time: ResMut<Time>) {
    time.update()
}

fn add_void_room(world: &mut World) {
    if world
        .get_resource::<Rooms>()
        .unwrap()
        .by_id(*VOID_ROOM_ID)
        .is_none()
    {
        let name = "The Void".to_string();
        let description = "A dark void extends infinitely in all directions.".to_string();
        let bundle = RoomBundle {
            id: Id::Room(*VOID_ROOM_ID),
            room: Room::from(*VOID_ROOM_ID),
            name: Named::from(name.clone()),
            description: Description::from(description.clone()),
            regions: Regions::default(),
            contents: Contents::default(),
            hooks: ScriptHooks::default(),
        };
        let void_room = world.spawn().insert_bundle(bundle).id();
        world
            .get_resource_mut::<Rooms>()
            .unwrap()
            .insert(*VOID_ROOM_ID, void_room);

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .persist(persist::room::Create::new(*VOID_ROOM_ID, name, description));

        tracing::warn!("Void room was deleted and has been recreated.");
    }
}

fn build_schedules() -> (Schedule, Schedule, Schedule) {
    let mut pre_event_schedule = Schedule::default();
    pre_event_schedule.add_stage(STAGE_FIRST, SystemStage::parallel());
    pre_event_schedule.add_stage_after(STAGE_FIRST, STAGE_UPDATE, SystemStage::parallel());

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
