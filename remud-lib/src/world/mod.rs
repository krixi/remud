#![allow(clippy::type_complexity)]

pub mod action;
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
    ecs::{Ecs, SharedWorld, Step},
    engine::persist::{self, DynPersist, Updates},
    web,
    world::{
        action::Action,
        scripting::{
            actions::compile_scripts, run_init_scripts, run_post_action_scripts,
            run_pre_action_scripts, run_timed_scripts, QueuedAction, Script, ScriptName,
            TriggerEvent,
        },
        types::{
            object::PrototypeId,
            player::{Messages, Player, Players},
            room::{Regions, Room, RoomBundle, RoomId, Rooms},
            Configuration, Contents, Description, Id, Location, Named,
        },
    },
};

pub static VOID_ROOM_ID: Lazy<RoomId> = Lazy::new(|| RoomId::try_from(0).unwrap());

pub struct GameWorld {
    ecs: Ecs,
}

impl GameWorld {
    pub async fn new(ecs: Ecs) -> Self {
        let world = ecs.world();
        let mut world = world.write().await;

        // Perform initial compilation of all scripts
        compile_scripts(&mut *world).await;

        // Create emergency room
        add_void_room(&mut *world);

        GameWorld { ecs }
    }

    pub async fn run(&mut self) {
        let world = self.ecs.world();

        self.ecs.run(Step::PreEvent).await;

        run_init_scripts(world.clone()).await;

        run_pre_action_scripts(world.clone()).await;

        self.ecs.run(Step::Main).await;
        self.ecs.run(Step::PostEvent).await;

        run_timed_scripts(world.clone()).await;

        run_post_action_scripts(world).await;
    }

    pub async fn should_shutdown(&self) -> bool {
        self.ecs
            .world()
            .read()
            .await
            .get_resource::<Configuration>()
            .map_or(true, |configuration| configuration.shutdown)
    }

    pub async fn despawn_player(&mut self, player: Entity) -> anyhow::Result<()> {
        let world = self.ecs.world();
        let mut world = world.write().await;

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

    pub async fn player_action(&mut self, action: Action) {
        let world = self.ecs.world();
        let mut world = world.write().await;

        world
            .get_mut::<Messages>(action.actor())
            .unwrap()
            .set_received_input();

        world
            .get_resource_mut::<Events<QueuedAction>>()
            .unwrap()
            .send(QueuedAction { action });
    }

    pub async fn player_online(&self, name: &str) -> bool {
        self.ecs
            .world()
            .read()
            .await
            .get_resource::<Players>()
            .unwrap()
            .by_name(name)
            .is_some()
    }

    pub async fn spawn_room(&self) -> RoomId {
        self.ecs
            .world()
            .read()
            .await
            .get_resource::<Configuration>()
            .unwrap()
            .spawn_room
    }

    pub async fn messages(&mut self) -> Vec<(Entity, VecDeque<String>)> {
        let world = self.ecs.world();
        let mut world = world.write().await;

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

    pub async fn updates(&mut self) -> Vec<DynPersist> {
        self.ecs
            .world()
            .write()
            .await
            .get_resource_mut::<Updates>()
            .unwrap()
            .take_updates()
    }

    pub async fn prototype_reloads(&mut self) -> Vec<PrototypeId> {
        self.ecs
            .world()
            .write()
            .await
            .get_resource_mut::<Updates>()
            .unwrap()
            .take_reloads()
    }

    pub fn get_world(&self) -> SharedWorld {
        self.ecs.world()
    }

    pub async fn create_script(
        &mut self,
        name: String,
        trigger: String,
        code: String,
    ) -> Result<Option<ParseError>, web::Error> {
        let name = ScriptName::try_from(name).map_err(|_| web::Error::BadScriptName)?;
        let trigger =
            TriggerEvent::from_str(trigger.as_str()).map_err(|_| web::Error::BadTrigger)?;

        let script = Script::new(name, trigger, code);

        scripting::actions::create_script(&mut *self.ecs.world().write().await, script).await
    }

    pub async fn read_script(
        &mut self,
        name: String,
    ) -> Result<(Script, Option<ParseError>), web::Error> {
        let name = ScriptName::try_from(name).map_err(|_| web::Error::BadScriptName)?;

        scripting::actions::read_script(&*self.ecs.world().read().await, name)
    }

    pub async fn read_all_scripts(&mut self) -> Vec<(Script, Option<ParseError>)> {
        scripting::actions::read_all_scripts(&mut *self.ecs.world().write().await)
    }

    pub async fn update_script(
        &mut self,
        name: String,
        trigger: String,
        code: String,
    ) -> Result<Option<ParseError>, web::Error> {
        let name = ScriptName::try_from(name).map_err(|_| web::Error::BadScriptName)?;
        let trigger =
            TriggerEvent::from_str(trigger.as_str()).map_err(|_| web::Error::BadTrigger)?;

        let script = Script::new(name, trigger, code);

        scripting::actions::update_script(&mut *self.ecs.world().write().await, script).await
    }

    pub async fn delete_script(&mut self, name: String) -> Result<(), web::Error> {
        let name = ScriptName::try_from(name).map_err(|_| web::Error::BadScriptName)?;

        scripting::actions::delete_script(&mut *self.ecs.world().write().await, name)
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
