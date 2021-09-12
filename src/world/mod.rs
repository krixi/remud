#![allow(clippy::type_complexity)]

pub mod action;
pub mod types;

use std::{collections::VecDeque, convert::TryFrom, ops::DerefMut, sync::Arc};

use bevy_ecs::prelude::*;
use itertools::Itertools;
use lazy_static::lazy_static;
use tokio::sync::RwLock;

use crate::{
    engine::persist::{self, DynUpdate, Updates},
    world::{
        action::{queue_message, DynAction, Logout, MissingComponent},
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
    world: Arc<RwLock<World>>,
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

        let world = Arc::new(RwLock::new(world));

        GameWorld { world, schedule }
    }

    pub async fn run(&mut self) {
        self.schedule.run_once(self.world.write().await.deref_mut());
    }

    pub async fn should_shutdown(&self) -> bool {
        self.world
            .read()
            .await
            .get_resource::<Configuration>()
            .map_or(true, |configuration| configuration.shutdown)
    }

    pub async fn despawn_player(&mut self, player: Entity) -> anyhow::Result<()> {
        self.player_action(player, Box::new(Logout {})).await;

        let mut world = self.world.write().await;

        let (name, room) = world
            .get::<Player>(player)
            .map(|player| (player.name.clone(), player.room))
            .ok_or_else(|| MissingComponent::new(player, "Player"))?;

        if let Some(objects) = world
            .get::<Contents>(player)
            .map(|contents| contents.objects.clone())
        {
            for object in objects {
                world.despawn(object);
            }
        }
        world.despawn(player);
        world.get_resource_mut::<Players>().unwrap().remove(&name);
        world
            .get_mut::<Room>(room)
            .ok_or_else(|| MissingComponent::new(room, "Room"))?
            .remove_player(player);

        Ok(())
    }

    pub async fn player_action(&mut self, player: Entity, mut action: DynAction) {
        let mut world = self.world.write().await;

        match world.get_mut::<Messages>(player) {
            Some(mut messages) => messages.received_input = true,
            None => {
                world.entity_mut(player).insert(Messages {
                    received_input: true,
                    queue: VecDeque::new(),
                });
            }
        }

        if let Err(e) = action.enact(player, &mut world) {
            queue_message(&mut world, player, "Command failed.".to_string());
            tracing::error!("Action error: {}", e);
        };
    }

    pub async fn player_online(&self, name: &str) -> bool {
        self.world
            .read()
            .await
            .get_resource::<Players>()
            .unwrap()
            .by_name(name)
            .is_some()
    }

    pub async fn spawn_room(&self) -> room::Id {
        self.world
            .read()
            .await
            .get_resource::<Configuration>()
            .unwrap()
            .spawn_room
    }

    pub async fn messages(&mut self) -> Vec<(Entity, VecDeque<String>)> {
        let mut world = self.world.write().await;

        let players_with_messages = world
            .query_filtered::<Entity, (With<Player>, With<Messages>)>()
            .iter(&world)
            .collect_vec();

        let mut outgoing = Vec::new();

        for player in players_with_messages {
            if let Some(messages) = world.get::<Messages>(player) {
                if messages.queue.is_empty() {
                    continue;
                }
            }
            if let Some(mut messages) = world.entity_mut(player).remove::<Messages>() {
                if !messages.received_input {
                    messages.queue.push_front("\r\n".to_string());
                }
                outgoing.push((player, messages.queue));
            }
        }

        outgoing
    }

    pub async fn updates(&mut self) -> Vec<DynUpdate> {
        self.world
            .write()
            .await
            .get_resource_mut::<Updates>()
            .unwrap()
            .take()
    }

    pub fn get_world(&self) -> Arc<RwLock<World>> {
        self.world.clone()
    }
}
