#![allow(clippy::type_complexity)]

pub mod action;
pub mod types;

use std::{collections::HashMap, convert::TryFrom};

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persistence::DynUpdate,
    queue_message,
    text::word_list,
    world::{
        action::DynAction,
        types::room::{Direction, Room, RoomId, Rooms},
    },
};

// Components
pub struct Player {
    pub name: String,
}

pub struct Location {
    pub room: Entity,
}

pub struct Messages {
    pub queue: Vec<String>,
}

impl Messages {
    pub fn new_with(message: String) -> Self {
        Messages {
            queue: vec![message],
        }
    }
}

// Resources
pub struct Configuration {
    pub shutdown: bool,
    pub spawn_room: RoomId,
}

#[derive(Default)]
pub struct Updates {
    updates: Vec<DynUpdate>,
}

impl Updates {
    pub fn queue(&mut self, update: DynUpdate) {
        self.updates.push(update);
    }
}

pub struct GameWorld {
    world: World,
    schedule: Schedule,
    void_room: Entity,
}

impl GameWorld {
    pub fn new(mut world: World) -> Self {
        // Create emergency room
        let room = Room {
            id: RoomId::try_from(0).unwrap(),
            description: "A dark void extends infinitely in all directions.".to_string(),
            exits: HashMap::new(),
        };
        let void_room = world.spawn().insert(room).id();

        // Add resources
        world.insert_resource(Updates::default());

        // Create schedule
        let mut schedule = Schedule::default();

        let mut update = SystemStage::parallel();
        update.add_system(look_system.system());
        update.add_system(move_system.system());
        update.add_system(say_system.system());
        update.add_system(teleport_system.system());
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

            let player_entity = self
                .world
                .spawn()
                .insert(Player { name })
                .insert(Location { room })
                .insert(WantsToLook {})
                .id();

            (player_entity, room)
        };

        let mut rooms = self.world.get_resource_mut::<Rooms>().unwrap();

        rooms.add_player(player, room);

        player
    }

    pub fn despawn_player(&mut self, player: Entity) {
        let location = self
            .world
            .get::<Location>(player)
            .map(|location| location.room);

        self.world.entity_mut(player).despawn();

        if let Some(location) = location {
            let mut rooms = self.world.get_resource_mut::<Rooms>().unwrap();
            rooms.remove_player(player, location);
        }
    }

    pub fn player_action(&mut self, player: Entity, mut action: DynAction) {
        action.enact(player, &mut self.world);
    }

    pub fn messages(&mut self) -> Vec<(Entity, Vec<String>)> {
        let players_with_messages = self
            .world
            .query_filtered::<Entity, (With<Player>, With<Messages>)>()
            .iter(&self.world)
            .collect_vec();

        let mut outgoing = Vec::new();

        for player in players_with_messages {
            if let Some(messages) = self.world.entity_mut(player).remove::<Messages>() {
                outgoing.push((player, messages.queue));
            }
        }

        outgoing
    }

    pub fn updates(&mut self) -> Vec<DynUpdate> {
        let mut updates = self.world.get_resource_mut::<Updates>().unwrap();

        let mut new_updates = Vec::new();
        std::mem::swap(&mut updates.updates, &mut new_updates);

        new_updates
    }

    pub fn get_world(&self) -> &World {
        &self.world
    }
}

pub struct WantsToLook {}

fn look_system(
    mut commands: Commands,
    rooms: Res<Rooms>,
    looking_query: Query<(Entity, &Location), (With<Player>, With<WantsToLook>)>,
    players_query: Query<&Player>,
    rooms_query: Query<&Room>,
    mut messages: Query<&mut Messages>,
) {
    for (looking_entity, looking_location) in looking_query.iter() {
        if let Ok(room) = rooms_query.get(looking_location.room) {
            let mut message = format!("{}\r\n", room.description);

            let mut present_names = rooms
                .players_in(looking_location.room)
                .filter(|player| player != &looking_entity)
                .filter_map(|player| players_query.get(player).ok())
                .map(|player| player.name.clone())
                .collect_vec();

            if !present_names.is_empty() {
                present_names.sort();

                let singular = present_names.len() == 1;

                let mut player_list = word_list(present_names);
                if singular {
                    player_list.push_str(" is here.");
                } else {
                    player_list.push_str(" are here.");
                };
                message.push_str(player_list.as_str());
                message.push_str("\r\n");
            }

            queue_message!(commands, messages, looking_entity, message);
        }

        commands.entity(looking_entity).remove::<WantsToLook>();
    }
}

pub struct WantsToMove {
    pub direction: Direction,
}

fn move_system(
    mut commands: Commands,
    mut rooms: ResMut<Rooms>,
    mut moving_query: Query<(Entity, &Player, &WantsToMove, &mut Location)>,
    rooms_query: Query<&Room>,
    mut messages: Query<&mut Messages>,
) {
    for (moving_entity, player, wants_to_move, mut location) in moving_query.iter_mut() {
        let destination = if let Some(destination) = rooms_query
            .get(location.room)
            .ok()
            .and_then(|room| room.exits.get(&wants_to_move.direction))
        {
            *destination
        } else {
            let message = "There is nothing in that direction.\r\n".to_string();
            queue_message!(commands, messages, moving_entity, message);

            commands.entity(moving_entity).remove::<WantsToMove>();

            continue;
        };

        rooms.move_player(moving_entity, location.room, destination);

        rooms
            .players_in(location.room)
            .filter(|player| player != &moving_entity)
            .for_each(|present_player| {
                let message = format!(
                    "{} leaves {}.\r\n",
                    player.name,
                    wants_to_move.direction.as_to_str()
                );
                queue_message!(commands, messages, present_player, message);
            });

        location.room = destination;

        rooms
            .players_in(destination)
            .filter(|player| player != &moving_entity)
            .for_each(|present_player| {
                let message = format!(
                    "{} enters {}.\r\n",
                    player.name,
                    wants_to_move.direction.opposite().as_from_str()
                );
                queue_message!(commands, messages, present_player, message);
            });

        commands
            .entity(moving_entity)
            .insert(WantsToLook {})
            .remove::<WantsToMove>();
    }
}

pub struct WantsToSay {
    pub message: String,
}

fn say_system(
    mut commands: Commands,
    rooms: Res<Rooms>,
    saying_query: Query<(Entity, &Player, &Location, &WantsToSay)>,
    mut messages: Query<&mut Messages>,
) {
    for (saying_entity, saying_player, saying_location, wants_to_say) in saying_query.iter() {
        rooms
            .players_in(saying_location.room)
            .filter(|player| player != &saying_entity)
            .for_each(|present_player| {
                let message = format!(
                    "{} says \"{}\"\r\n",
                    saying_player.name, wants_to_say.message
                );
                queue_message!(commands, messages, present_player, message);
            });

        commands.entity(saying_entity).remove::<WantsToSay>();
    }
}

pub struct WantsToTeleport {
    pub room: Entity,
}

fn teleport_system(
    mut commands: Commands,
    mut rooms: ResMut<Rooms>,
    mut teleporting_query: Query<(Entity, &Player, &WantsToTeleport, &mut Location)>,
    mut messages: Query<&mut Messages>,
) {
    for (teleporting_entity, teleporting_player, wants_to_teleport, mut location) in
        teleporting_query.iter_mut()
    {
        rooms
            .players_in(location.room)
            .filter(|player| player != &teleporting_entity)
            .for_each(|present_player| {
                let message = format!(
                    "{} disappears in the blink of an eye.\r\n",
                    teleporting_player.name
                );
                queue_message!(commands, messages, present_player, message);
            });

        rooms.move_player(teleporting_entity, location.room, wants_to_teleport.room);

        location.room = wants_to_teleport.room;

        rooms
            .players_in(location.room)
            .filter(|player| player != &teleporting_entity)
            .for_each(|present_player| {
                let message = format!(
                    "{} appears in a puff of smoke.\r\n",
                    teleporting_player.name
                );
                queue_message!(commands, messages, present_player, message);
            });

        commands
            .entity(teleporting_entity)
            .insert(WantsToLook {})
            .remove::<WantsToTeleport>();
    }
}
