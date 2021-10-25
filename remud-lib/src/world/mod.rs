#![allow(clippy::type_complexity)]

pub mod action;
pub mod fsm;
pub mod scripting;
pub mod types;

use std::{collections::VecDeque, convert::TryFrom, str::FromStr};

use bevy_app::Events;
use bevy_ecs::prelude::{Entity, With, World};
use itertools::Itertools;
use once_cell::sync::Lazy;
use rhai::ParseError;

use crate::{
    ecs::{Ecs, Step},
    engine::persist::{self, DynPersist, Updates},
    web::scripts::ScriptError,
    world::{
        action::{commands::Commands, Action},
        scripting::{
            actions::compile_scripts, run_init_scripts, run_post_action_scripts,
            run_pre_action_scripts, run_timed_scripts, QueuedAction, Script, ScriptName,
            TriggerEvent,
        },
        types::{
            object::{Objects, PrototypeId},
            player::{self, Messages, Player, PlayerFlags, Players},
            room::{Regions, Room, RoomBundle, RoomId, Rooms},
            Configuration, Contents, Description, Id, Location, Named,
        },
    },
};

pub static VOID_ROOM_ID: Lazy<RoomId> = Lazy::new(|| RoomId::try_from(0).unwrap());

pub struct GameWorld {
    ecs: Ecs,
    commands: Commands,
}

impl GameWorld {
    #[tracing::instrument(name = "creating world", skip_all)]
    pub fn new(mut ecs: Ecs) -> Self {
        let world = ecs.world_mut();

        // Perform initial compilation of all scripts
        compile_scripts(world);

        // Create emergency room
        add_void_room(world);

        GameWorld {
            ecs,
            commands: Commands::default(),
        }
    }

    #[tracing::instrument(name = "run pre/init", skip_all)]
    pub fn run_pre_init(&mut self) {
        self.ecs.run(Step::PreEvent);

        self.ecs.with_shared_world(|world| {
            run_init_scripts(world.clone());
            run_pre_action_scripts(world);
        });
    }

    #[tracing::instrument(name = "run main", skip_all)]
    pub fn run_main(&mut self) {
        self.ecs.run(Step::Main);
        self.ecs.run(Step::PostEvent);
    }

    #[tracing::instrument(name = "run post/timed", skip_all)]
    pub fn run_post_timed(&mut self) {
        self.ecs.with_shared_world(|world| {
            run_timed_scripts(world.clone());
            run_post_action_scripts(world);
        })
    }

    pub fn should_shutdown(&self) -> bool {
        self.ecs
            .world()
            .get_resource::<Configuration>()
            .map_or(true, |configuration| configuration.shutdown)
    }

    pub fn should_restart(&self) -> bool {
        self.ecs
            .world()
            .get_resource::<Configuration>()
            .map_or(false, |configuration| configuration.restart)
    }

    #[tracing::instrument(name = "despawning player", skip_all, fields(player = player.to_bits()))]
    pub fn despawn_player(&mut self, player: Entity) -> anyhow::Result<()> {
        let world = self.ecs.world_mut();

        let (name, room) = world
            .query::<(&Named, &Location)>()
            .get(&*world, player)
            .map(|(named, location)| (named.to_string(), location.entity()))
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
                world
                    .get_resource_mut::<Objects>()
                    .unwrap()
                    .remove_entity(object);
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

    #[tracing::instrument(name = "player input", skip_all)]
    pub fn player_input(&mut self, player: Entity, input: &str) -> Result<(), String> {
        let immortal = self
            .ecs
            .world()
            .get::<PlayerFlags>(player)
            .unwrap()
            .contains(player::Flags::IMMORTAL);

        match self.commands.parse(player, input, !immortal) {
            Ok(action) => self.player_action(action),
            Err(message) => return Err(message),
        }

        Ok(())
    }

    #[tracing::instrument(name = "player action", skip_all)]
    pub fn player_action(&mut self, action: Action) {
        self.ecs
            .world_mut()
            .get_resource_mut::<Events<QueuedAction>>()
            .unwrap()
            .send(QueuedAction { action });
    }

    pub fn player_online(&self, name: &str) -> bool {
        self.ecs
            .world()
            .get_resource::<Players>()
            .unwrap()
            .by_name(name)
            .is_some()
    }

    pub fn spawn_room(&self) -> RoomId {
        self.ecs
            .world()
            .get_resource::<Configuration>()
            .unwrap()
            .spawn_room
    }

    #[tracing::instrument(name = "dispatching messages", skip_all)]
    pub fn messages(&mut self) -> Vec<(Entity, VecDeque<String>)> {
        let world = self.ecs.world_mut();

        let players_with_messages = world
            .query_filtered::<Entity, (With<Player>, With<Messages>)>()
            .iter(world)
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
            .world_mut()
            .get_resource_mut::<Updates>()
            .unwrap()
            .take_updates()
    }

    pub fn prototype_reloads(&mut self) -> Vec<PrototypeId> {
        self.ecs
            .world_mut()
            .get_resource_mut::<Updates>()
            .unwrap()
            .take_reloads()
    }

    pub fn world_mut(&mut self) -> &mut World {
        self.ecs.world_mut()
    }

    pub fn create_script(
        &mut self,
        name: String,
        trigger: String,
        code: String,
    ) -> Result<Option<ParseError>, ScriptError> {
        let name = ScriptName::try_from(name).map_err(|_| ScriptError::BadScriptName)?;
        let trigger =
            TriggerEvent::from_str(trigger.as_str()).map_err(|_| ScriptError::BadTrigger)?;

        let script = Script::new(name, trigger, code);

        scripting::actions::create_script(&mut *self.ecs.world_mut(), script)
    }

    pub fn read_script(
        &mut self,
        name: String,
    ) -> Result<(Script, Option<ParseError>), ScriptError> {
        let name = ScriptName::try_from(name).map_err(|_| ScriptError::BadScriptName)?;

        scripting::actions::read_script(&*self.ecs.world(), name)
    }

    pub fn read_all_scripts(&mut self) -> Vec<(Script, Option<ParseError>)> {
        scripting::actions::read_all_scripts(&mut *self.ecs.world_mut())
    }

    pub fn update_script(
        &mut self,
        name: String,
        trigger: String,
        code: String,
    ) -> Result<Option<ParseError>, ScriptError> {
        let name = ScriptName::try_from(name).map_err(|_| ScriptError::BadScriptName)?;
        let trigger =
            TriggerEvent::from_str(trigger.as_str()).map_err(|_| ScriptError::BadTrigger)?;

        let script = Script::new(name, trigger, code);

        scripting::actions::update_script(&mut *self.ecs.world_mut(), script)
    }

    pub fn delete_script(&mut self, name: String) -> Result<(), ScriptError> {
        let name = ScriptName::try_from(name).map_err(|_| ScriptError::BadScriptName)?;

        scripting::actions::delete_script(&mut *self.ecs.world_mut(), name)
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

        tracing::warn!("void room was created.");
    }
}
