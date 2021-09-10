use std::str::FromStr;

use anyhow::bail;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persistence::{
        PersistNewRoom, PersistObjectLocation, PersistRemoveRoom, PersistRoomExits,
        PersistRoomUpdates, Updates,
    },
    text::Tokenizer,
    world::{
        action::{movement::Teleport, queue_message, Action, DynAction},
        types::{
            player::Player,
            room::{Direction, Room, RoomId, Rooms},
        },
        VOID_ROOM_ID,
    },
};

// Valid shapes:
// room new - creates a new unlinked room
// room new [direction] - creates a room to the [Direction] of this one with a two way link
// room desc [description] - sets the description of a room
// room link [direction] [room ID] - links the current room to another in a given direction (one way)
pub fn parse(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(subcommand) = tokenizer.next() {
        match subcommand.to_lowercase().as_str() {
            "new" => {
                let direction = if let Some(direction) = tokenizer.next() {
                    match Direction::from_str(direction) {
                        Ok(direction) => Some(direction),
                        Err(_) => {
                            return Err(
                                "Enter a valid direction: up, down, north, east, south, west."
                                    .to_string(),
                            )
                        }
                    }
                } else {
                    None
                };

                Ok(Box::new(CreateRoom { direction }))
            }
            "desc" => {
                if tokenizer.rest().is_empty() {
                    Err("Enter a description.".to_string())
                } else {
                    Ok(Box::new(UpdateRoom {
                        description: Some(tokenizer.rest().to_string()),
                    }))
                }
            }
            "link" => {
                if let Some(direction) = tokenizer.next() {
                    if let Some(destination) = tokenizer.next() {
                        let direction =
                            match Direction::from_str(direction) {
                                Ok(direction) => direction,
                                Err(_) => return Err(
                                    "Enter a valid direction: up, down, north, east, south, west."
                                        .to_string(),
                                ),
                            };

                        let destination = match destination.parse::<RoomId>() {
                            Ok(destination) => destination,
                            Err(e) => return Err(e.to_string()),
                        };

                        Ok(Box::new(UpdateExit {
                            direction,
                            destination,
                        }))
                    } else {
                        Err("A destination room ID is required.".to_string())
                    }
                } else {
                    Err("A direction and destination room ID are required.".to_string())
                }
            }
            "remove" => Ok(Box::new(RemoveRoom {})),
            _ => Err("Enter a valid room subcommand: new, desc, link, remove".to_string()),
        }
    } else {
        Err("Enter a valid room subcommand: new, desc, link".to_string())
    }
}

struct CreateRoom {
    direction: Option<Direction>,
}

impl Action for CreateRoom {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let current_room = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("Player {:?} does not have a Location."),
        };

        // Confirm a room does not already exist in this direction
        if let Some(direction) = self.direction {
            match world.get::<Room>(current_room) {
                Some(room) => {
                    if room.exits.contains_key(&direction) {
                        let message = format!("A room already exists {}.", direction.as_to_str());
                        queue_message(world, player, message);
                        return Ok(());
                    }
                }
                None => bail!("Room {:?} has no Room.", current_room),
            }
        }

        // Create new room
        let id = world.get_resource_mut::<Rooms>().unwrap().next_id();
        let room = Room::new(id, "An empty room.".to_string());
        let new_room = world.spawn().insert(room).id();

        // Add reverse lookup
        world
            .get_resource_mut::<Rooms>()
            .unwrap()
            .insert(id, new_room);

        // Create links
        if let Some(direction) = self.direction {
            world
                .get_mut::<Room>(new_room)
                .unwrap()
                .exits
                .insert(direction.opposite(), current_room);

            world
                .get_mut::<Room>(current_room)
                .unwrap()
                .exits
                .insert(direction, new_room);
        }

        let mut message = format!("Created room {}", id);
        if let Some(direction) = self.direction {
            message.push(' ');
            message.push_str(direction.as_to_str());
        }
        message.push('.');
        queue_message(world, player, message);

        // Queue update
        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(PersistNewRoom::new(new_room));
        if self.direction.is_some() {
            updates.queue(PersistRoomExits::new(new_room));
            updates.queue(PersistRoomExits::new(current_room));
        }

        Ok(())
    }
}

struct UpdateExit {
    direction: Direction,
    destination: RoomId,
}

impl Action for UpdateExit {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let destination = match world
            .get_resource::<Rooms>()
            .unwrap()
            .by_id(self.destination)
        {
            Some(room) => room,
            None => {
                let message = format!("Room {} does not exist.", self.destination);
                queue_message(world, player, message);
                return Ok(());
            }
        };

        let from_room = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("Player {:?} does not have a Location.", player),
        };

        match world.get_mut::<Room>(from_room) {
            Some(mut room) => room.exits.insert(self.direction, destination),
            None => bail!("Room {:?} does not have a Room"),
        };

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(PersistRoomExits::new(from_room));

        let from_room = world.get::<Room>(from_room).unwrap();
        let message = format!(
            "Linked room {} {} to room {}.",
            from_room.id, self.direction, self.destination
        );
        queue_message(world, player, message);

        Ok(())
    }
}

struct UpdateRoom {
    description: Option<String>,
}

impl Action for UpdateRoom {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let room_entity = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("Player {:?} does not have a Location.", player),
        };

        match world.get_mut::<Room>(room_entity) {
            Some(mut room) => {
                if self.description.is_some() {
                    room.description = self.description.take().unwrap();

                    let message = format!("Updated room {} description.", room.id);
                    queue_message(world, player, message);
                }
            }
            None => bail!("Room {:?} has no Room.", room_entity),
        }

        // Queue update
        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(PersistRoomUpdates::new(room_entity));

        Ok(())
    }
}

struct RemoveRoom {}

impl Action for RemoveRoom {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let room_entity = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("Player {:?} has no Location."),
        };

        let (room_id, present_players, present_objects) = match world.get::<Room>(room_entity) {
            Some(room) => {
                if room.id == *VOID_ROOM_ID {
                    let message = "You cannot delete the void room.".to_string();
                    queue_message(world, player, message);
                    return Ok(());
                }

                let players = room.players.iter().cloned().collect_vec();
                let objects = room.objects.iter().cloned().collect_vec();

                (room.id, players, objects)
            }
            None => bail!("Room {:?} does not have a Room", room_entity),
        };

        // Move all players and objects from this room to the void room.
        let mut emergency_teleport = Teleport::new(*VOID_ROOM_ID);
        for present_player in present_players {
            emergency_teleport.enact(present_player, world)?;
        }

        let void_room_entity = world
            .get_resource::<Rooms>()
            .unwrap()
            .by_id(*VOID_ROOM_ID)
            .unwrap();
        {
            let mut void_room = world.get_mut::<Room>(void_room_entity).unwrap();
            for object in present_objects.iter() {
                void_room.objects.push(*object);
            }
        }

        // Remove the room
        world.get_resource_mut::<Rooms>().unwrap().remove(room_id);
        world.despawn(room_entity);

        // Find and remove all exits to the room (FKs handle this in the DB)
        world
            .query::<(Entity, &mut Room)>()
            .for_each_mut(world, |(room_entity, mut room)| {
                let to_remove = room
                    .exits
                    .iter()
                    .filter(|(_, entity)| **entity == room_entity)
                    .map(|(direction, _)| *direction)
                    .collect_vec();

                for direction in to_remove {
                    room.exits.remove(&direction);
                }
            });

        // Persist the changes
        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(PersistRemoveRoom::new(room_id));

        for object in present_objects {
            updates.queue(PersistObjectLocation::new(object))
        }

        let message = format!("Room {} removed.", room_id);
        queue_message(world, player, message);

        Ok(())
    }
}
