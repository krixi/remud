use std::{collections::HashMap, str::FromStr};

use bevy_ecs::prelude::*;

use crate::{
    engine::persistence::{PersistNewRoom, PersistRoomExits, PersistRoomUpdates, Updates},
    text::Tokenizer,
    world::{
        types::room::{Direction, Room, RoomId, Rooms},
        Configuration, Location, LoggedIn, LoggedOut, Messages, Player, WantsExits, WantsToLook,
        WantsToMove, WantsToSay, WantsToTeleport,
    },
};

pub type DynAction = Box<dyn Action + Send>;

pub trait Action {
    fn enact(&mut self, player: Entity, world: &mut World);
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
        let room = Room {
            id,
            description: "An empty room.".to_string(),
            exits: HashMap::new(),
        };
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

struct Exits {}

impl Action for Exits {
    fn enact(&mut self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsExits {});
    }
}

pub struct Login {}

impl Action for Login {
    fn enact(&mut self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(LoggedIn {});
    }
}

pub struct Logout {}

impl Action for Logout {
    fn enact(&mut self, player: Entity, world: &mut World) {
        if let Some(room) = world.get::<Location>(player).map(|location| location.room) {
            if let Some(name) = world
                .get::<Player>(player)
                .map(|player| player.name.clone())
            {
                world.entity_mut(room).insert(LoggedOut { name });
            }
        }
    }
}

struct Look {}

impl Action for Look {
    fn enact(&mut self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsToLook {});
    }
}

struct Move {
    direction: Direction,
}

impl Action for Move {
    fn enact(&mut self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsToMove {
            direction: self.direction,
        });
    }
}

struct Say {
    message: String,
}

impl Action for Say {
    fn enact(&mut self, player: Entity, world: &mut World) {
        let mut message = String::new();
        std::mem::swap(&mut self.message, &mut message);
        world.entity_mut(player).insert(WantsToSay { message });
    }
}

struct Shutdown {}

impl Action for Shutdown {
    fn enact(&mut self, _player: Entity, world: &mut World) {
        let mut configuration = world.get_resource_mut::<Configuration>().unwrap();
        configuration.shutdown = true;
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

pub fn parse(input: &str) -> Result<DynAction, String> {
    if input.starts_with('\'') {
        return Ok(Box::new(Say {
            message: input[1..].to_string(),
        }));
    }

    let mut tokenizer = Tokenizer::new(input);
    if let Some(token) = tokenizer.next() {
        match token.to_lowercase().as_str() {
            "down" => Ok(Box::new(Move {
                direction: Direction::Down,
            })),
            "east" => Ok(Box::new(Move {
                direction: Direction::East,
            })),
            "exits" => Ok(Box::new(Exits {})),
            "look" => Ok(Box::new(Look {})),
            "north" => Ok(Box::new(Move {
                direction: Direction::North,
            })),
            "room" => parse_room(tokenizer),
            "say" => Ok(Box::new(Say {
                message: tokenizer.rest().to_string(),
            })),
            "shutdown" => Ok(Box::new(Shutdown {})),
            "south" => Ok(Box::new(Move {
                direction: Direction::South,
            })),
            "teleport" => {
                if let Some(destination) = tokenizer.next() {
                    match destination.parse::<RoomId>() {
                        Ok(room_id) => Ok(Box::new(Teleport { room_id })),
                        Err(e) => Err(e.to_string()),
                    }
                } else {
                    Err("Teleport to where?".to_string())
                }
            }
            "up" => Ok(Box::new(Move {
                direction: Direction::Up,
            })),
            "west" => Ok(Box::new(Move {
                direction: Direction::West,
            })),
            _ => Err("I don't know what that means.".to_string()),
        }
    } else {
        Err("Go on, then.".to_string())
    }
}

// Valid shapes:
// room new - creates a new unlinked room
// room new [direction] - creates a room to the [Direction] of this one with a two way link
// room desc [description] - sets the description of a room
// room link [direction] [room ID] - links the current room to another in a given direction (one way)
fn parse_room(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
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

fn queue_message(world: &mut World, player: Entity, message: String) {
    match world.entity_mut(player).get_mut::<Messages>() {
        Some(mut messages) => messages.queue(message),
        None => {
            world.entity_mut(player).insert(Messages::new_with(message));
        }
    }
}
