use anyhow::{self, bail};
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    text::Tokenizer,
    world::{
        action::{queue_message, Action, DynAction, Look},
        types::{
            player::{Player, Players},
            room::{Direction, Room, RoomId, Rooms},
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
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let (name, current_room) = match world.get::<Player>(player) {
            Some(player) => (player.name.clone(), player.room),
            None => bail!("Player {:?} has no Player.", player),
        };

        let destination = match world
            .get::<Room>(current_room)
            .and_then(|room| room.exits.get(&self.direction))
        {
            Some(destination) => *destination,
            None => {
                let message = format!("There is no exit {}.", self.direction.as_to_str());
                queue_message(world, player, message);
                return Ok(());
            }
        };

        let present_players = world
            .get_resource::<Players>()
            .unwrap()
            .by_room(current_room)
            .filter(|present_player| *present_player != player)
            .collect_vec();

        let leave_message = format!("{} leaves {}.", name, self.direction.as_to_str());
        for present_player in present_players {
            queue_message(world, present_player, leave_message.clone())
        }

        world
            .get_resource_mut::<Players>()
            .unwrap()
            .change_room(player, current_room, destination);
        world.get_mut::<Player>(player).unwrap().room = destination;

        let from_direction = match world.get::<Room>(destination).map(|room| {
            room.exits
                .iter()
                .find(|(_, room)| **room == current_room)
                .map(|(direction, _)| direction)
                .copied()
        }) {
            Some(direction) => direction,
            None => bail!("Destination room {:?} does not have a Room.", destination),
        };

        let present_players = world
            .get_resource::<Players>()
            .unwrap()
            .by_room(destination)
            .filter(|present_player| *present_player != player)
            .collect_vec();

        let message = from_direction
            .map(|from| format!("{} arrives {}.", name, from.as_from_str()))
            .unwrap_or_else(|| format!("{} appears.", name));
        for present_player in present_players {
            queue_message(world, present_player, message.clone());
        }

        Look::here().enact(player, world)
    }
}

pub fn parse_teleport(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(destination) = tokenizer.next() {
        match destination.parse::<RoomId>() {
            Ok(room_id) => Ok(Box::new(Teleport { room_id })),
            Err(e) => Err(e.to_string()),
        }
    } else {
        Err("Teleport to where?".to_string())
    }
}

pub struct Teleport {
    room_id: RoomId,
}

impl Teleport {
    pub fn new(room_id: RoomId) -> Box<Self> {
        Box::new(Teleport { room_id })
    }
}

impl Action for Teleport {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let destination =
            if let Some(room) = world.get_resource::<Rooms>().unwrap().by_id(self.room_id) {
                room
            } else {
                let message = format!("Room {} doesn't exist.", self.room_id);
                queue_message(world, player, message);
                return Ok(());
            };

        let (name, current_room) = match world.get::<Player>(player) {
            Some(player) => (player.name.clone(), player.room),
            None => bail!("Player {:?} does not have a Player."),
        };

        let present_players = world
            .get_resource::<Players>()
            .unwrap()
            .by_room(current_room)
            .filter(|present_player| *present_player != player)
            .collect_vec();

        let message = format!("{} disappears in the blink of an eye.", name);
        for present_player in present_players {
            queue_message(world, present_player, message.clone());
        }

        world
            .get_resource_mut::<Players>()
            .unwrap()
            .change_room(player, current_room, destination);
        world.get_mut::<Player>(player).unwrap().room = destination;

        let present_players = world
            .get_resource::<Players>()
            .unwrap()
            .by_room(destination)
            .filter(|present_player| *present_player != player)
            .collect_vec();

        let message = format!("{} appears in a flash of light.", name);
        for present_player in present_players {
            queue_message(world, present_player, message.clone());
        }

        Look::here().enact(player, world)
    }
}
