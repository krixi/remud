use bevy_ecs::prelude::*;

use crate::{
    text::Tokenizer,
    world::{
        action::{queue_message, Action, DynAction},
        types::room::{Direction, RoomId, Rooms},
        WantsToMove, WantsToTeleport,
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
    fn enact(&mut self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsToMove {
            direction: self.direction,
        });
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

struct Teleport {
    room_id: RoomId,
}

impl Action for Teleport {
    fn enact(&mut self, player: Entity, world: &mut World) {
        let room = if let Some(room) = world
            .get_resource::<Rooms>()
            .unwrap()
            .get_room(self.room_id)
        {
            room
        } else {
            let message = format!("Room {} doesn't exist.\r\n", self.room_id);
            queue_message(world, player, message);
            return;
        };

        world.entity_mut(player).insert(WantsToTeleport { room });
    }
}
