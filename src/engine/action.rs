use std::{collections::HashMap, str::FromStr};

use bevy_ecs::prelude::*;

use crate::{
    engine::world::{
        Configuration, Direction, Location, Messages, Room, RoomMetadata, WantsToLook, WantsToMove,
        WantsToSay, WantsToTeleport,
    },
    text::Tokenizer,
};

pub type DynAction = Box<dyn Action + Send>;

pub trait Action {
    fn enact(&self, player: Entity, world: &mut World);
}

struct CreateRoom {
    direction: Option<Direction>,
}

fn queue_message(world: &mut World, player: Entity, message: String) {
    match world.entity_mut(player).get_mut::<Messages>() {
        Some(mut messages) => messages.queue.push(message),
        None => {
            world.entity_mut(player).insert(Messages::new_with(message));
        }
    }
}

impl Action for CreateRoom {
    fn enact(&self, player: Entity, world: &mut World) {
        let current_room_entity = world
            .entity(player)
            .get::<Location>()
            .map(|location| location.room);

        // Confirm a room does not already exist in this direction
        if let Some(direction) = self.direction {
            if let Some(current_room) =
                current_room_entity.and_then(|room| world.entity_mut(room).get_mut::<Room>())
            {
                if current_room.exits.contains_key(&direction) {
                    let message = format!("A room already exists {}.\r\n", direction.pretty_to());
                    queue_message(world, player, message);
                    return;
                }
            }
        }

        // Create new room
        let id = world.get_resource_mut::<RoomMetadata>().unwrap().next_id();
        let room = Room {
            id,
            description: "An empty room.".to_string(),
            exits: HashMap::new(),
        };
        let new_room_entity = world.spawn().insert(room).id();

        // Create links
        if let Some(direction) = self.direction {
            if let Some(current_room_entity) = current_room_entity {
                if let Some(mut new_room) = world.entity_mut(new_room_entity).get_mut::<Room>() {
                    new_room
                        .exits
                        .insert(direction.opposite(), current_room_entity);
                }

                if let Some(mut current_room) =
                    world.entity_mut(current_room_entity).get_mut::<Room>()
                {
                    current_room.exits.insert(direction, new_room_entity);
                }
            }
        }

        // Add reverse lookup
        world
            .get_resource_mut::<RoomMetadata>()
            .unwrap()
            .rooms_by_id
            .insert(id, new_room_entity);

        // Teleport player to new room
        world.entity_mut(player).insert(WantsToTeleport {
            room: new_room_entity,
        });
    }
}

struct Look {}

impl Action for Look {
    fn enact(&self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsToLook {});
    }
}

struct Move {
    direction: Direction,
}

impl Action for Move {
    fn enact(&self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsToMove {
            direction: self.direction,
        });
    }
}

struct Say {
    message: String,
}

impl Action for Say {
    fn enact(&self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsToSay {
            message: self.message.clone(),
        });
    }
}

struct Shutdown {}

impl Action for Shutdown {
    fn enact(&self, _player: Entity, world: &mut World) {
        let mut configuration = world.get_resource_mut::<Configuration>().unwrap();
        configuration.shutdown = true;
    }
}

struct Teleport {
    room_id: i64,
}

impl Action for Teleport {
    fn enact(&self, player: Entity, world: &mut World) {
        let room = if let Some(room) = world
            .get_resource::<RoomMetadata>()
            .unwrap()
            .rooms_by_id
            .get(&self.room_id)
        {
            *room
        } else {
            let message = format!("Room {} doesn't exist.\r\n", self.room_id);
            queue_message(world, player, message);
            return;
        };

        world.entity_mut(player).insert(WantsToTeleport { room });
    }
}

pub fn parse_action(input: &str) -> Result<DynAction, String> {
    let mut tokenizer = Tokenizer::new(input);
    if let Some(token) = tokenizer.next() {
        match token.to_lowercase().as_str() {
            "down" => Ok(Box::new(Move {
                direction: Direction::Down,
            })),
            "east" => Ok(Box::new(Move {
                direction: Direction::East,
            })),
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
                    if let Ok(room_id) = destination.parse::<i64>() {
                        if room_id > 0 {
                            Ok(Box::new(Teleport { room_id }))
                        } else {
                            Err("Room IDs must be positive.".to_string())
                        }
                    } else {
                        Err("Room IDs must be positive integers.".to_string())
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
// room new [Direction] - creates a room to the [Direction] of this one with a two way link
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
            s => Err(format!("'{}' is not a valid room subcommand.", s)),
        }
    } else {
        Err("'room' requires a subcommand.".to_string())
    }
}
