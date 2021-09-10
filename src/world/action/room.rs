use std::str::FromStr;

use anyhow::bail;
use bevy_ecs::prelude::*;

use crate::{
    engine::persistence::{PersistNewRoom, PersistRoomExits, PersistRoomUpdates, Updates},
    text::Tokenizer,
    world::{
        action::{queue_message, Action, DynAction},
        types::{
            room::{Direction, Room, RoomId, Rooms},
            Location,
        },
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
            _ => Err("Enter a valid room subcommand: new, desc, link".to_string()),
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
        let current_room = match world.get::<Location>(player).map(|location| location.room) {
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
            .add_room(id, new_room);

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
            .get_room(self.destination)
        {
            Some(room) => room,
            None => {
                let message = format!("Room {} does not exist.", self.destination);
                queue_message(world, player, message);
                return Ok(());
            }
        };

        let from_room = match world.get::<Location>(player).map(|location| location.room) {
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
        let room_entity = match world.get::<Location>(player).map(|location| location.room) {
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
