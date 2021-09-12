use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::Tokenizer,
    world::{
        action::{self, queue_message, Action, DynAction, Look},
        types::{
            player::Player,
            room::{self, Direction, Room, Rooms},
        },
    },
};

pub struct Move {
    direction: Direction,
}

impl Move {
    pub fn new(direction: Direction) -> Box<Self> {
        Box::new(Move { direction })
    }
}

impl Action for Move {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let (player_id, name, current_room) = world
            .get::<Player>(player)
            .map(|player| (player.id, player.name.clone(), player.room))
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let (destination, present_players) = {
            let room = world
                .get::<Room>(current_room)
                .ok_or(action::Error::MissingComponent(current_room, "Room"))?;

            let destination = if let Some(destination) = room.exits.get(&self.direction) {
                *destination
            } else {
                let message = format!("There is no exit {}.", self.direction.as_to_str());
                queue_message(world, player, message);
                return Ok(());
            };

            let present_players = room
                .players
                .iter()
                .filter(|present_player| **present_player != player)
                .copied()
                .collect_vec();

            (destination, present_players)
        };

        let leave_message = format!("{} leaves {}.", name, self.direction.as_to_str());
        for present_player in present_players {
            queue_message(world, present_player, leave_message.clone());
        }

        world
            .get_mut::<Room>(current_room)
            .unwrap()
            .remove_player(player);
        world.get_mut::<Player>(player).unwrap().room = destination;
        world
            .get_mut::<Room>(destination)
            .ok_or(action::Error::MissingComponent(destination, "Room"))?
            .players
            .push(player);

        let (destination_id, from_direction, present_players) = {
            let room = world.get::<Room>(destination).unwrap();

            let direction = room
                .exits
                .iter()
                .find(|(_, room)| **room == current_room)
                .map(|(direction, _)| direction)
                .copied();

            let present_players = room
                .players
                .iter()
                .filter(|present_player| **present_player != player)
                .copied()
                .collect_vec();

            (room.id, direction, present_players)
        };

        let message = from_direction.map_or_else(
            || format!("{} appears.", name),
            |from| format!("{} arrives {}.", name, from.as_from_str()),
        );
        for present_player in present_players {
            queue_message(world, present_player, message.clone());
        }

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::player::Room::new(player_id, destination_id));

        Look::here().enact(player, world)
    }
}

pub fn parse_teleport(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(destination) = tokenizer.next() {
        match destination.parse::<room::Id>() {
            Ok(room_id) => Ok(Box::new(Teleport { room_id })),
            Err(e) => Err(e.to_string()),
        }
    } else {
        Err("Teleport to where?".to_string())
    }
}

pub struct Teleport {
    room_id: room::Id,
}

impl Teleport {
    pub fn new(room_id: room::Id) -> Box<Self> {
        Box::new(Teleport { room_id })
    }
}

impl Action for Teleport {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let destination =
            if let Some(room) = world.get_resource::<Rooms>().unwrap().by_id(self.room_id) {
                room
            } else {
                let message = format!("Room {} doesn't exist.", self.room_id);
                queue_message(world, player, message);
                return Ok(());
            };

        let (player_id, name, current_room) = world
            .get::<Player>(player)
            .map(|player| (player.id, player.name.clone(), player.room))
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let present_players = world
            .get::<Room>(current_room)
            .ok_or(action::Error::MissingComponent(current_room, "Room"))?
            .players
            .iter()
            .filter(|present_player| **present_player != player)
            .copied()
            .collect_vec();

        let message = format!("{} disappears in the blink of an eye.", name);
        for present_player in present_players {
            queue_message(world, present_player, message.clone());
        }

        world
            .get_mut::<Room>(current_room)
            .unwrap()
            .remove_player(player);
        world.get_mut::<Player>(player).unwrap().room = destination;
        let destination_id = {
            let mut room = world
                .get_mut::<Room>(destination)
                .ok_or(action::Error::MissingComponent(destination, "Room"))?;
            room.players.push(player);
            room.id
        };

        let present_players = world
            .get::<Room>(destination)
            .unwrap()
            .players
            .iter()
            .filter(|present_player| **present_player != player)
            .copied()
            .collect_vec();

        let message = format!("{} appears in a flash of light.", name);
        for present_player in present_players {
            queue_message(world, present_player, message.clone());
        }

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::player::Room::new(player_id, destination_id));

        Look::here().enact(player, world)
    }
}
