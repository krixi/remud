#![allow(clippy::type_complexity)]

pub mod action;
pub mod types;

use std::{collections::VecDeque, convert::TryFrom};

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persistence::{DynUpdate, Updates},
    world::{
        action::{DynAction, Login, Logout, Look},
        types::{
            player::{Messages, Player, Players},
            room::{Room, RoomId, Rooms},
            Configuration, Location,
        },
    },
};

pub struct GameWorld {
    world: World,
    schedule: Schedule,
    void_room: Entity,
}

impl GameWorld {
    pub fn new(mut world: World) -> Self {
        // Create emergency room
        let room = Room::new(
            RoomId::try_from(0).unwrap(),
            "A dark void extends infinitely in all directions.".to_string(),
        );
        let void_room = world.spawn().insert(room).id();

        // Add resources
        world.insert_resource(Updates::default());
        world.insert_resource(Players::default());

        // Create schedule
        let mut schedule = Schedule::default();
        let update = SystemStage::parallel();
        // Add fun systems
        schedule.add_stage("update", update);

        GameWorld {
            world,
            schedule,
            void_room,
        }
    }

    pub fn run(&mut self) {
        self.schedule.run_once(&mut self.world);
    }

    pub fn should_shutdown(&self) -> bool {
        self.world
            .get_resource::<Configuration>()
            .map_or(true, |configuration| configuration.shutdown)
    }

    pub fn spawn_player(&mut self, name: String) -> Entity {
        let (player, room) = {
            let room = {
                let configuration = self.world.get_resource::<Configuration>().unwrap();
                let rooms = self.world.get_resource::<Rooms>().unwrap();

                rooms
                    .get_room(configuration.spawn_room)
                    .unwrap_or(self.void_room)
            };

            let player = self
                .world
                .spawn()
                .insert(Player { name: name.clone() })
                .insert(Location { room })
                .id();

            (player, room)
        };

        let mut players = self.world.get_resource_mut::<Players>().unwrap();

        players.spawn(player, name, room);

        self.player_action(player, Box::new(Login {}));
        self.player_action(player, Look::here());

        player
    }

    pub fn despawn_player(&mut self, player: Entity) {
        self.player_action(player, Box::new(Logout {}));

        let location = self
            .world
            .get::<Location>(player)
            .map(|location| location.room);

        if let Some(location) = location {
            let name = if let Some(name) = self
                .world
                .get::<Player>(player)
                .map(|player| player.name.clone())
            {
                name
            } else {
                tracing::error!("Unable to despawn player {:?} at {:?}", player, location);
                return;
            };

            self.world.entity_mut(player).despawn();

            let mut players = self.world.get_resource_mut::<Players>().unwrap();
            players.despawn(player, &name, location);
        } else {
            tracing::error!("Unable to despawn player {:?}", player);
        };
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
}
