use std::str::FromStr;

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::Tokenizer,
    world::{
        action::{
            self, movement::Teleport, queue_message, Action, DynAction, DEFAULT_ROOM_DESCRIPTION,
        },
        types::{
            object::Object,
            player::Player,
            room::{self, Direction, Room, RoomBundle, Rooms},
            Contents, Description,
        },
        VOID_ROOM_ID,
    },
};

// Valid shapes:
// room info - displays information about the room
// room new - creates a new unlinked room
// room new [direction] - creates a room to the [Direction] of this one with a two way link
// room desc [description] - sets the description of a room
// room link [direction] [room ID] - links the current room to another in a given direction (one way)
// room remove - removes the current room and moves everything in it to the void room
pub fn parse(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(subcommand) = tokenizer.next() {
        match subcommand.to_lowercase().as_str() {
            "info" => Ok(Box::new(Info {})),
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

                Ok(Box::new(Create { direction }))
            }
            "desc" => {
                if tokenizer.rest().is_empty() {
                    Err("Enter a description.".to_string())
                } else {
                    Ok(Box::new(Update {
                        description: Some(tokenizer.rest().to_string()),
                    }))
                }
            }
            "link" => {
                if let Some(direction) = tokenizer.next() {
                    let direction = match Direction::from_str(direction) {
                        Ok(direction) => direction,
                        Err(_) => {
                            return Err(
                                "Enter a valid direction: up, down, north, east, south, west."
                                    .to_string(),
                            )
                        }
                    };

                    if let Some(destination) = tokenizer.next() {
                        let destination = match destination.parse::<room::Id>() {
                            Ok(destination) => destination,
                            Err(e) => return Err(e.to_string()),
                        };

                        Ok(Box::new(Link {
                            direction,
                            destination,
                        }))
                    } else {
                        Err("Enter a destination room ID.".to_string())
                    }
                } else {
                    Err("Enter a direction.".to_string())
                }
            }
            "remove" => Ok(Box::new(Remove {})),
            "unlink" => {
                if let Some(direction) = tokenizer.next() {
                    let direction = match Direction::from_str(direction) {
                        Ok(direction) => direction,
                        Err(_) => {
                            return Err(
                                "Enter a valid direction: up, down, north, east, south, west."
                                    .to_string(),
                            )
                        }
                    };

                    Ok(Box::new(Unlink { direction }))
                } else {
                    Err("Enter a direction.".to_string())
                }
            }
            _ => Err(
                "Enter a valid room subcommand: info, desc, link, new, remove or unlink."
                    .to_string(),
            ),
        }
    } else {
        Err("Enter a room subcommand: info, desc, link, new, remove or unlink.".to_string())
    }
}

struct Create {
    direction: Option<Direction>,
}

impl Action for Create {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let current_room = world
            .get::<Player>(player)
            .map(|player| player.room)
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        // Confirm a room does not already exist in this direction
        if let Some(direction) = self.direction {
            let room = world
                .get::<Room>(current_room)
                .ok_or(action::Error::MissingComponent(current_room, "Room"))?;

            if room.exits.contains_key(&direction) {
                let message = format!("A room already exists {}.", direction.as_to_str());
                queue_message(world, player, message);
                return Ok(());
            }
        };

        // Create new room
        let new_room_id = world.get_resource_mut::<Rooms>().unwrap().next_id();
        let room = RoomBundle {
            room: Room::new(new_room_id, DEFAULT_ROOM_DESCRIPTION.to_string()),
            description: Description {
                text: DEFAULT_ROOM_DESCRIPTION.to_string(),
            },
            contents: Contents::default(),
        };
        let new_room_entity = world.spawn().insert_bundle(room).id();

        // Add reverse lookup
        world
            .get_resource_mut::<Rooms>()
            .unwrap()
            .insert(new_room_id, new_room_entity);

        // Create links
        if let Some(direction) = self.direction {
            world
                .get_mut::<Room>(new_room_entity)
                .unwrap()
                .exits
                .insert(direction.opposite(), current_room);

            world
                .get_mut::<Room>(current_room)
                .unwrap()
                .exits
                .insert(direction, new_room_entity);
        }

        let current_room_id = world
            .get::<Room>(current_room)
            .map(|room| room.id)
            .ok_or(action::Error::MissingComponent(current_room, "Room"))?;

        // Queue update
        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(persist::room::New::new(
            new_room_id,
            DEFAULT_ROOM_DESCRIPTION.to_string(),
        ));
        if let Some(direction) = self.direction {
            updates.queue(persist::room::AddExit::new(
                current_room_id,
                new_room_id,
                direction,
            ));
            updates.queue(persist::room::AddExit::new(
                new_room_id,
                current_room_id,
                direction.opposite(),
            ));
        }

        let mut message = format!("Created room {}", new_room_id);
        if let Some(direction) = self.direction {
            message.push(' ');
            message.push_str(direction.as_to_str());
        }
        message.push('.');
        queue_message(world, player, message);

        Ok(())
    }
}

struct Info {}

impl Action for Info {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let room_entity = world
            .get::<Player>(player)
            .map(|player| player.room)
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let room = world
            .get::<Room>(room_entity)
            .ok_or(action::Error::MissingComponent(room_entity, "Room"))?;

        let mut message = format!("Room {}", room.id);

        message.push_str("\r\n  description: ");
        message.push_str(room.description.as_str());

        message.push_str("\r\n  exits:");
        room.exits
            .iter()
            .filter_map(|(direction, room)| {
                world.get::<Room>(*room).map(|room| (direction, room.id))
            })
            .for_each(|(direction, room_id)| {
                message.push_str(format!("\r\n    {}: room {}", direction, room_id).as_str())
            });

        message.push_str("\r\n  players:");
        room.players
            .iter()
            .filter_map(|player| {
                world
                    .get::<Player>(*player)
                    .map(|player| player.name.as_str())
            })
            .for_each(|name| message.push_str(format!("\r\n    {}", name).as_str()));

        message.push_str("\r\n  objects:");
        world
            .get::<Contents>(room_entity)
            .ok_or(action::Error::MissingComponent(room_entity, "Contents"))?
            .objects
            .iter()
            .filter_map(|object| world.get::<Object>(*object))
            .map(|object| (object.id, object.short.as_str()))
            .for_each(|(id, name)| {
                message.push_str(format!("\r\n    object {}: {}", id, name).as_str());
            });

        queue_message(world, player, message);

        Ok(())
    }
}

struct Link {
    direction: Direction,
    destination: room::Id,
}

impl Action for Link {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let destination = if let Some(room) = world
            .get_resource::<Rooms>()
            .unwrap()
            .by_id(self.destination)
        {
            room
        } else {
            let message = format!("Room {} does not exist.", self.destination);
            queue_message(world, player, message);
            return Ok(());
        };

        let from_room = world
            .get::<Player>(player)
            .map(|player| player.room)
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let from_id = {
            let mut room = world
                .get_mut::<Room>(from_room)
                .ok_or(action::Error::MissingComponent(from_room, "Room"))?;
            room.exits.insert(self.direction, destination);
            room.id
        };

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::room::AddExit::new(
                from_id,
                self.destination,
                self.direction,
            ));

        let message = format!(
            "Linked {} exit to room {}.",
            self.direction, self.destination
        );
        queue_message(world, player, message);

        Ok(())
    }
}

struct Update {
    description: Option<String>,
}

impl Action for Update {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let room_entity = world
            .get::<Player>(player)
            .map(|player| player.room)
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let (room_id, description) = {
            let mut room = world
                .get_mut::<Room>(room_entity)
                .ok_or(action::Error::MissingComponent(room_entity, "Room"))?;

            if self.description.is_some() {
                room.description = self.description.take().unwrap();
            }

            (room.id, room.description.clone())
        };

        // Queue update
        if self.description.is_some() {
            world
                .get_resource_mut::<Updates>()
                .unwrap()
                .queue(persist::room::Update::new(room_id, description));
        }

        let message = format!("Updated room {}.", room_id);
        queue_message(world, player, message);

        Ok(())
    }
}

struct Remove {}

impl Action for Remove {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let (player_id, room_entity) = world
            .get::<Player>(player)
            .map(|player| (player.id, player.room))
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let (room_id, present_players, present_objects) = {
            let room = world
                .get::<Room>(room_entity)
                .ok_or(action::Error::MissingComponent(room_entity, "Room"))?;

            if room.id == *VOID_ROOM_ID {
                let message = "You cannot delete the void room.".to_string();
                queue_message(world, player, message);
                return Ok(());
            }

            let players = room.players.iter().copied().collect_vec();
            let objects = world
                .get::<Contents>(room_entity)
                .ok_or(action::Error::MissingComponent(room_entity, "Contents"))?
                .objects
                .iter()
                .copied()
                .collect_vec();

            (room.id, players, objects)
        };

        // Move all players and objects from this room to the void room.
        let mut emergency_teleport = Teleport::new(*VOID_ROOM_ID);
        for present_player in present_players.iter() {
            emergency_teleport.enact(*present_player, world)?;
        }

        let void_room_entity = world
            .get_resource::<Rooms>()
            .unwrap()
            .by_id(*VOID_ROOM_ID)
            .unwrap();
        {
            let mut void_contents = world.get_mut::<Contents>(void_room_entity).unwrap();
            for object in &present_objects {
                void_contents.objects.push(*object);
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

        let present_player_ids = present_players
            .iter()
            .filter_map(|entity| world.get::<Player>(*entity).map(|player| player.id))
            .collect_vec();

        let present_object_ids = present_objects
            .iter()
            .filter_map(|entity| world.get::<Object>(*entity).map(|object| object.id))
            .collect_vec();

        // Persist the changes
        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(persist::room::Remove::new(room_id));

        updates.queue(persist::player::Room::new(player_id, *VOID_ROOM_ID));
        for id in present_player_ids {
            updates.queue(persist::player::Room::new(id, *VOID_ROOM_ID));
        }

        for id in present_object_ids {
            updates.queue(persist::room::AddObject::new(*VOID_ROOM_ID, id));
        }

        let message = format!("Room {} removed.", room_id);
        queue_message(world, player, message);

        Ok(())
    }
}

struct Unlink {
    direction: Direction,
}

impl Action for Unlink {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let room_entity = world
            .get::<Player>(player)
            .map(|player| player.room)
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let (room_id, removed) = {
            let mut room = world
                .get_mut::<Room>(room_entity)
                .ok_or(action::Error::MissingComponent(room_entity, "Room"))?;
            let removed = room.exits.remove(&self.direction).is_some();
            (room.id, removed)
        };

        let message = if removed {
            format!("Removed exit {}.", self.direction.as_to_str())
        } else {
            format!("There is no exit {}.", self.direction.as_to_str())
        };

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::room::RemoveExit::new(room_id, self.direction));

        queue_message(world, player, message);

        Ok(())
    }
}
