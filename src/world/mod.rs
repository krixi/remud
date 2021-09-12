#![allow(clippy::type_complexity)]

pub mod action;
pub mod types;

use std::{collections::VecDeque, convert::TryFrom};

use anyhow::bail;
use bevy_ecs::prelude::*;
use itertools::Itertools;
use lazy_static::lazy_static;

use crate::{
    engine::persist::{self, DynUpdate, Updates},
    world::{
        action::{queue_message, DynAction, Logout},
        types::{
            player::{Messages, Player, Players},
            room::{self, Room, Rooms},
            Configuration, Contents,
        },
    },
};

lazy_static! {
    pub static ref VOID_ROOM_ID: room::Id = room::Id::try_from(0).unwrap();
}

pub struct GameWorld {
    world: World,
    schedule: Schedule,
}

impl GameWorld {
    pub fn new(mut world: World) -> Self {
        // Add resources
        world.insert_resource(Updates::default());
        world.insert_resource(Players::default());

        if world
            .get_resource::<Rooms>()
            .unwrap()
            .by_id(*VOID_ROOM_ID)
            .is_none()
        {
            let description = "A dark void extends infinitely in all directions.".to_string();
            let room = Room::new(*VOID_ROOM_ID, description.clone());
            let void_room = world.spawn().insert(room).id();
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

        // Create schedule
        let mut schedule = Schedule::default();
        let update = SystemStage::parallel();
        // Add fun systems
        schedule.add_stage("update", update);

        GameWorld { world, schedule }
    }

    pub fn run(&mut self) {
        self.schedule.run_once(&mut self.world);
    }

    pub fn should_shutdown(&self) -> bool {
        self.world
            .get_resource::<Configuration>()
            .map_or(true, |configuration| configuration.shutdown)
    }

    pub fn despawn_player(&mut self, player: Entity) -> anyhow::Result<()> {
        self.player_action(player, Box::new(Logout {}));

        let room = match self.world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("{:?} has no Player", player),
        };

        let name = if let Some(name) = self
            .world
            .get::<Player>(player)
            .map(|player| player.name.clone())
        {
            name
        } else {
            bail!("Unable to despawn {:?} at {:?}", player, room);
        };

        if let Some(objects) = self
            .world
            .get::<Contents>(player)
            .map(|contents| contents.objects.clone())
        {
            for object in objects {
                self.world.despawn(object);
            }
        }
        self.world.despawn(player);
        self.world
            .get_resource_mut::<Players>()
            .unwrap()
            .remove(&name);
        match self.world.get_mut::<Room>(room) {
            Some(mut room) => room.remove_player(player),
            None => bail!("{:?} has no Room.", room),
        }

        Ok(())
    }

    pub fn player_action(&mut self, player: Entity, mut action: DynAction) {
        match self.world.get_mut::<Messages>(player) {
            Some(mut messages) => messages.received_input = true,
            None => {
                self.world.entity_mut(player).insert(Messages {
                    received_input: true,
                    queue: VecDeque::new(),
                });
            }
        }
        if let Err(e) = action.enact(player, &mut self.world) {
            queue_message(&mut self.world, player, "Command failed.".to_string());
            tracing::error!("Action error: {}", e);
        };
    }

    pub fn messages(&mut self) -> Vec<(Entity, VecDeque<String>)> {
        let players_with_messages = self
            .world
            .query_filtered::<Entity, (With<Player>, With<Messages>)>()
            .iter(&self.world)
            .collect_vec();

        let mut outgoing = Vec::new();

        for player in players_with_messages {
            if let Some(messages) = self.world.get::<Messages>(player) {
                if messages.queue.is_empty() {
                    continue;
                }
            }
            if let Some(mut messages) = self.world.entity_mut(player).remove::<Messages>() {
                if !messages.received_input {
                    messages.queue.push_front("\r\n".to_string());
                }
                outgoing.push((player, messages.queue));
            }
        }

        outgoing
    }

    pub fn updates(&mut self) -> Vec<DynUpdate> {
        self.world.get_resource_mut::<Updates>().unwrap().take()
    }

    pub fn get_world(&self) -> &World {
        &self.world
    }

    pub fn get_world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}
