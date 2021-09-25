#![allow(clippy::type_complexity)]

pub mod action;
pub mod ecs;
pub mod fsm;
pub mod scripting;
pub mod types;

use std::{collections::VecDeque, convert::TryFrom, str::FromStr};

use bevy_app::Events;
use bevy_ecs::prelude::*;
use itertools::Itertools;
use once_cell::sync::Lazy;
use rhai::ParseError;

use crate::{
    engine::persist::{self, DynPersist, PersistPlugin, Updates},
    web,
    world::{
        action::{Action, ActionsPlugin},
        ecs::{CorePlugin, Ecs, SharedWorld, Step},
        fsm::FsmPlugin,
        scripting::{
            run_init_scripts, run_post_action_scripts, run_pre_action_scripts, run_timed_scripts,
            QueuedAction, Script, ScriptName, ScriptPlugin, TriggerEvent,
        },
        types::{
            object::PrototypeId,
            player::{Messages, Player, Players},
            room::{Regions, Room, RoomBundle, RoomId, Rooms},
            Configuration, Contents, Description, Id, Location, Named, TypesPlugin,
        },
    },
};

pub static VOID_ROOM_ID: Lazy<RoomId> = Lazy::new(|| RoomId::try_from(0).unwrap());

pub struct GameWorld {
    ecs: Ecs,
}

impl GameWorld {
    pub fn new(world: World) -> Self {
        let mut ecs = Ecs::new(world);

        ecs.register(CorePlugin::default());
        ecs.register(TypesPlugin::default());
        ecs.register(ActionsPlugin::default());
        ecs.register(ScriptPlugin::default());
        ecs.register(FsmPlugin::default());
        ecs.register(PersistPlugin::default());

        // Create emergency room
        add_void_room(&mut *ecs.world().write().unwrap());

        GameWorld { ecs }
    }

    pub fn run(&mut self) {
        let world = self.ecs.world();

        self.ecs.run(Step::PreEvent);

        run_init_scripts(world.clone());

        run_pre_action_scripts(world.clone());

        self.ecs.run(Step::Main);
        self.ecs.run(Step::PostEvent);

        run_timed_scripts(world.clone());

        run_post_action_scripts(world);
    }

    pub fn should_shutdown(&self) -> bool {
        self.ecs
            .world()
            .read()
            .unwrap()
            .get_resource::<Configuration>()
            .map_or(true, |configuration| configuration.shutdown)
    }

    pub fn despawn_player(&mut self, player: Entity) -> anyhow::Result<()> {
        let world = self.ecs.world();
        let mut world = world.write().unwrap();

        let (name, room) = world
            .query::<(&Named, &Location)>()
            .get(&*world, player)
            .map(|(named, location)| (named.to_string(), location.room()))
            .unwrap();

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
        world.get_mut::<Room>(room).unwrap().remove_player(player);

        Ok(())
    }

    pub fn player_action(&mut self, action: Action) {
        let world = self.ecs.world();
        let mut world = world.write().unwrap();

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
        self.ecs
            .world()
            .read()
            .unwrap()
            .get_resource::<Players>()
            .unwrap()
            .by_name(name)
            .is_some()
    }

    pub fn spawn_room(&self) -> RoomId {
        self.ecs
            .world()
            .read()
            .unwrap()
            .get_resource::<Configuration>()
            .unwrap()
            .spawn_room
    }

    pub fn messages(&mut self) -> Vec<(Entity, VecDeque<String>)> {
        let world = self.ecs.world();
        let mut world = world.write().unwrap();

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

            outgoing.push((player, messages.take_queue()));
        }

        outgoing
    }

    pub fn updates(&mut self) -> Vec<DynPersist> {
        self.ecs
            .world()
            .write()
            .unwrap()
            .get_resource_mut::<Updates>()
            .unwrap()
            .take_updates()
    }

    pub fn prototype_reloads(&mut self) -> Vec<PrototypeId> {
        self.ecs
            .world()
            .write()
            .unwrap()
            .get_resource_mut::<Updates>()
            .unwrap()
            .take_reloads()
    }

    pub fn get_world(&self) -> SharedWorld {
        self.ecs.world()
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

        let script = Script::new(name, trigger, code);

        scripting::actions::create_script(&mut *self.ecs.world().write().unwrap(), script)
    }

    pub fn read_script(
        &mut self,
        name: String,
    ) -> Result<(Script, Option<ParseError>), web::Error> {
        let name = ScriptName::try_from(name).map_err(|_| web::Error::BadScriptName)?;

        scripting::actions::read_script(&*self.ecs.world().read().unwrap(), name)
    }

    pub fn read_all_scripts(&mut self) -> Vec<(Script, Option<ParseError>)> {
        scripting::actions::read_all_scripts(&mut *self.ecs.world().write().unwrap())
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

        let script = Script::new(name, trigger, code);

        scripting::actions::update_script(&mut *self.ecs.world().write().unwrap(), script)
    }

    pub fn delete_script(&mut self, name: String) -> Result<(), web::Error> {
        let name = ScriptName::try_from(name).map_err(|_| web::Error::BadScriptName)?;

        scripting::actions::delete_script(&mut *self.ecs.world().write().unwrap(), name)
    }
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

        tracing::warn!("Void room was created.");
    }
}
