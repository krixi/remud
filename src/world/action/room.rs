use std::str::FromStr;

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
                        Err(_) => return Err(format!("'{}' is not a valid direction.", direction)),
                    }
                } else {
                    None
                };

                Ok(Box::new(CreateRoom { direction }))
            }
            "desc" => {
                let description = tokenizer.rest();
                Ok(Box::new(UpdateRoom {
                    description: Some(description.to_string()),
                }))
            }
            "link" => {
                if let Some(direction) = tokenizer.next() {
                    if let Some(destination) = tokenizer.next() {
                        let direction = match Direction::from_str(direction) {
                            Ok(direction) => direction,
                            Err(_) => {
                                return Err(format!("'{}' is not a valid direction.", direction))
                            }
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
            s => Err(format!("'{}' is not a valid room subcommand.", s)),
        }
    } else {
        Err("'room' requires a subcommand.".to_string())
    }
}

struct CreateRoom {
    direction: Option<Direction>,
}

impl Action for CreateRoom {
    fn enact(&mut self, player: Entity, world: &mut World) {
        let current_room_entity = if let Some(room) = world
            .entity(player)
            .get::<Location>()
            .map(|location| location.room)
        {
            room
        } else {
            tracing::error!("Unable to create room, player's current room cannot be found");
            return;
        };

        // Confirm a room does not already exist in this direction
        if let Some(direction) = self.direction {
            if let Some(current_room) = world.entity_mut(current_room_entity).get_mut::<Room>() {
                if current_room.exits.contains_key(&direction) {
                    let message = format!("A room already exists {}.\r\n", direction.as_to_str());
                    queue_message(world, player, message);
                    return;
                }
            }
        }

        // Create new room
        let id = world.get_resource_mut::<Rooms>().unwrap().next_id();
        let room = Room::new(id, "An empty room.".to_string());
        let new_room_entity = world.spawn().insert(room).id();

        // Create links
        if let Some(direction) = self.direction {
            if let Some(mut new_room) = world.entity_mut(new_room_entity).get_mut::<Room>() {
                new_room
                    .exits
                    .insert(direction.opposite(), current_room_entity);
            }

            if let Some(mut current_room) = world.entity_mut(current_room_entity).get_mut::<Room>()
            {
                current_room.exits.insert(direction, new_room_entity);
            }
        }

        let mut message = format!("Created room {}", id);
        if let Some(direction) = self.direction {
            message.push(' ');
            message.push_str(direction.as_to_str());
        }
        message.push_str(".\r\n");
        queue_message(world, player, message);

        // Add reverse lookup
        world
            .get_resource_mut::<Rooms>()
            .unwrap()
            .add_room(id, new_room_entity);

        // Queue update
        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(PersistNewRoom::new(new_room_entity));
        if self.direction.is_some() {
            updates.queue(PersistRoomExits::new(new_room_entity));
            updates.queue(PersistRoomExits::new(current_room_entity));
        }
    }
}

struct UpdateExit {
    direction: Direction,
    destination: RoomId,
}

impl Action for UpdateExit {
    fn enact(&mut self, player: Entity, world: &mut World) {
        let from_room = match world
            .entity(player)
            .get::<Location>()
            .map(|location| location.room)
        {
            Some(room) => room,
            None => {
                return;
            }
        };

        let destination = if let Some(destination) = world
            .get_resource::<Rooms>()
            .unwrap()
            .get_room(self.destination)
        {
            destination
        } else {
            return;
        };

        if let Some(mut room) = world.entity_mut(from_room).get_mut::<Room>() {
            room.exits.insert(self.direction, destination);
        }

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(PersistRoomExits::new(from_room));

        if let Some(room) = world.entity(from_room).get::<Room>() {
            let message = format!(
                "Linked room {} {} to room {}.\r\n",
                room.id, self.direction, self.destination
            );
            queue_message(world, player, message);
        }
    }
}

struct UpdateRoom {
    description: Option<String>,
}

impl Action for UpdateRoom {
    fn enact(&mut self, player: Entity, world: &mut World) {
        let room_entity = match world
            .entity(player)
            .get::<Location>()
            .map(|location| location.room)
        {
            Some(room) => room,
            None => {
                return;
            }
        };

        if let Some(mut room) = world.entity_mut(room_entity).get_mut::<Room>() {
            if let Some(description) = self.description.take() {
                room.description = description;
            }

            let message = format!("Updated room {} description.\r\n", room.id);
            queue_message(world, player, message);
        }

        // Queue update
        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(PersistRoomUpdates::new(room_entity));
    }
}
