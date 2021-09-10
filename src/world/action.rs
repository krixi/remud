use std::{mem, str::FromStr};

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persistence::{
        PersistNewObject, PersistNewRoom, PersistObjectRoom, PersistObjectUpdate, PersistRoomExits,
        PersistRoomUpdates, Updates,
    },
    text::Tokenizer,
    world::{
        types::{
            object::{Object, ObjectId, Objects},
            players::{Messages, Player, Players},
            room::{Direction, Room, RoomId, Rooms},
            Configuration, Location,
        },
        LoggedIn, LoggedOut, WantsExits, WantsToLook, WantsToLookAt, WantsToMove, WantsToSay,
        WantsToSendMessage, WantsToTeleport, WantsWhoInfo,
    },
};

pub type DynAction = Box<dyn Action + Send>;

pub trait Action {
    fn enact(&mut self, player: Entity, world: &mut World);
}

struct CreateObject {}

impl Action for CreateObject {
    fn enact(&mut self, player: Entity, world: &mut World) {
        let id = world.get_resource_mut::<Objects>().unwrap().next_id();

        let object_entity = world
            .spawn()
            .insert(Object {
                id,
                keywords: vec!["object".to_string()],
                short: "An object.".to_string(),
                long: "A nondescript object. Completely uninteresting.".to_string(),
            })
            .id();

        // place the object in the room
        let room_entity = if let Some(room_entity) =
            world.get::<Location>(player).map(|location| location.room)
        {
            if let Some(mut room) = world.get_mut::<Room>(room_entity) {
                room.objects.push(object_entity);
                Some(room_entity)
            } else {
                None
            }
        } else {
            None
        };

        world
            .get_resource_mut::<Objects>()
            .unwrap()
            .add_object(id, object_entity);

        // notify the player that the object was created
        let message = format!("Created object {}\r\n", id);
        queue_message(world, player, message);

        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(PersistNewObject::new(object_entity));
        if let Some(room_entity) = room_entity {
            updates.queue(PersistObjectRoom::new(object_entity, room_entity));
        }
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

struct Look {
    at: Option<Vec<String>>,
    direction: Option<Direction>,
}

impl Action for Look {
    fn enact(&mut self, player: Entity, world: &mut World) {
        if self.at.is_some() {
            world.entity_mut(player).insert(WantsToLookAt {
                keywords: self.at.take().unwrap(),
            });
        } else {
            world.entity_mut(player).insert(WantsToLook {
                direction: self.direction,
            });
        }
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

struct SendMessage {
    player: String,
    message: String,
}

impl Action for SendMessage {
    fn enact(&mut self, player: Entity, world: &mut World) {
        if let Some(recipient) = world
            .get_resource::<Players>()
            .unwrap()
            .by_name(self.player.as_str())
        {
            let mut message = String::new();
            mem::swap(&mut self.message, &mut message);
            world
                .entity_mut(player)
                .insert(WantsToSendMessage { recipient, message });
        } else {
            let message = format!(
                "Your term beeps in irritation: \"User '{}' not found.\"\r\n",
                self.player
            );
            queue_message(world, player, message)
        }
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

struct UpdateObject {
    id: ObjectId,
    keywords: Option<Vec<String>>,
    short: Option<String>,
    long: Option<String>,
}

impl Action for UpdateObject {
    fn enact(&mut self, player: Entity, world: &mut World) {
        let object_entity =
            if let Some(entity) = world.get_resource::<Objects>().unwrap().get_object(self.id) {
                entity
            } else {
                let message = format!("Object {} not found.", self.id);
                queue_message(world, player, message);
                return;
            };

        if let Some(mut object) = world.get_mut::<Object>(object_entity) {
            if self.keywords.is_some() {
                object.keywords = self.keywords.take().unwrap();
            }
            if self.short.is_some() {
                object.short = self.short.take().unwrap();
            }
            if self.long.is_some() {
                object.long = self.long.take().unwrap();
            }
        }

        let message = format!("Updated object {}\r\n", self.id);
        queue_message(world, player, message);

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(PersistObjectUpdate::new(object_entity));
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

struct Who {}

impl Action for Who {
    fn enact(&mut self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsWhoInfo {});
    }
}

pub fn parse(input: &str) -> Result<DynAction, String> {
    if let Some(message) = input.strip_prefix('\'').map(|str| str.to_string()) {
        return Ok(Box::new(Say { message }));
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
            "look" => parse_look(tokenizer),
            "north" => Ok(Box::new(Move {
                direction: Direction::North,
            })),
            "object" => parse_object(tokenizer),
            "room" => parse_room(tokenizer),
            "say" => Ok(Box::new(Say {
                message: tokenizer.rest().to_string(),
            })),
            "send" => parse_send(tokenizer),
            "shutdown" => Ok(Box::new(Shutdown {})),
            "south" => Ok(Box::new(Move {
                direction: Direction::South,
            })),
            "teleport" => parse_teleport(tokenizer),
            "up" => Ok(Box::new(Move {
                direction: Direction::Up,
            })),
            "west" => Ok(Box::new(Move {
                direction: Direction::West,
            })),
            "who" => Ok(Box::new(Who {})),
            _ => Err("I don't know what that means.".to_string()),
        }
    } else {
        Err("Go on, then.".to_string())
    }
}

fn parse_look(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    match tokenizer.next() {
        Some(token) => {
            if token == "at" {
                let keywords = tokenizer
                    .rest()
                    .split_whitespace()
                    .map(|keyword| keyword.to_string())
                    .collect_vec();

                Ok(Box::new(Look {
                    at: Some(keywords),
                    direction: None,
                }))
            } else if let Ok(direction) = Direction::from_str(token) {
                Ok(Box::new(Look {
                    at: None,
                    direction: Some(direction),
                }))
            } else {
                Err(format!("I don't know how to look {}.", token))
            }
        }
        None => Ok(Box::new(Look {
            at: None,
            direction: None,
        })),
    }
}

fn parse_object(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(token) = tokenizer.next() {
        match token {
            "new" => Ok(Box::new(CreateObject {})),
            maybe_id => {
                if let Ok(id) = ObjectId::from_str(maybe_id) {
                    if let Some(token) = tokenizer.next() {
                        match token {
                            "keywords" => {
                                let keywords = tokenizer
                                    .rest()
                                    .split(',')
                                    .map(|keyword| keyword.trim().to_string())
                                    .collect_vec();

                                Ok(Box::new(UpdateObject {
                                    id,
                                    keywords: Some(keywords),
                                    short: None,
                                    long: None,
                                }))
                            }
                            "short" => Ok(Box::new(UpdateObject {
                                id,
                                keywords: None,
                                short: Some(tokenizer.rest().to_string()),
                                long: None,
                            })),
                            "long" => Ok(Box::new(UpdateObject {
                                id,
                                keywords: None,
                                short: None,
                                long: Some(tokenizer.rest().to_string()),
                            })),
                            _ => Err(format!("I don't know how to {} object {}.", token, id)),
                        }
                    } else {
                        Err("Provide a valid object subcommand or ID.".to_string())
                    }
                } else {
                    Err(format!("I don't know how to {} an object.", token))
                }
            }
        }
    } else {
        Err("What's all this about an object?".to_string())
    }
}

fn parse_send(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(player) = tokenizer.next() {
        Ok(Box::new(SendMessage {
            player: player.to_string(),
            message: tokenizer.rest().to_string(),
        }))
    } else {
        Err("Send to whom?".to_string())
    }
}

fn parse_teleport(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(destination) = tokenizer.next() {
        match destination.parse::<RoomId>() {
            Ok(room_id) => Ok(Box::new(Teleport { room_id })),
            Err(e) => Err(e.to_string()),
        }
    } else {
        Err("Teleport to where?".to_string())
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
