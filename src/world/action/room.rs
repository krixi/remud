use std::str::FromStr;

use anyhow::bail;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::Tokenizer,
    world::{
        action::{movement::Teleport, queue_message, Action, DynAction},
        types::{
            object::Object,
            player::Player,
            room::{self, Direction, Room, Rooms},
            Contents,
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

struct Info {}

impl Action for Info {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let room_entity = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("{:?} has no Player.", player),
        };

        let room = match world.get::<Room>(room_entity) {
            Some(room) => room,
            None => bail!("{:?} has no Room.", room_entity),
        };

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
        match world.get::<Contents>(room_entity) {
            Some(contents) => {
                contents
                    .objects
                    .iter()
                    .filter_map(|object| world.get::<Object>(*object))
                    .map(|object| (object.id, object.short.as_str()))
                    .for_each(|(id, name)| {
                        message.push_str(format!("\r\n    object {}: {}", id, name).as_str());
                    });
            }
            None => bail!("{:?} has no Contents.", room_entity),
        }

        queue_message(world, player, message);

        Ok(())
    }
}

struct Create {
    direction: Option<Direction>,
}

impl Action for Create {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let current_room = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("{:?} has no Player.", player),
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
                None => bail!("{:?} has no Room.", current_room),
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
        updates.queue(persist::room::New::new(new_room));
        if self.direction.is_some() {
            updates.queue(persist::room::Exits::new(new_room));
            updates.queue(persist::room::Exits::new(current_room));
        }

        Ok(())
    }
}

struct Link {
    direction: Direction,
    destination: room::Id,
}

impl Action for Link {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
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

        let from_room = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("{:?} has no Player.", player),
        };

        match world.get_mut::<Room>(from_room) {
            Some(mut room) => room.exits.insert(self.direction, destination),
            None => bail!("{:?} has no Room", from_room),
        };

        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::room::Exits::new(from_room));

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
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let room_entity = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("{:?} has no Player.", player),
        };

        match world.get_mut::<Room>(room_entity) {
            Some(mut room) => {
                if self.description.is_some() {
                    room.description = self.description.take().unwrap();

                    let message = format!("Updated room {} description.", room.id);
                    queue_message(world, player, message);
                }
            }
            None => bail!("{:?} has no Room.", room_entity),
        }

        // Queue update
        world
            .get_resource_mut::<Updates>()
            .unwrap()
            .queue(persist::room::Update::new(room_entity));

        Ok(())
    }
}

struct Remove {}

impl Action for Remove {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let room_entity = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("{:?} has no Player.", player),
        };

        let (room_id, present_players, present_objects) = match world.get::<Room>(room_entity) {
            Some(room) => {
                if room.id == *VOID_ROOM_ID {
                    let message = "You cannot delete the void room.".to_string();
                    queue_message(world, player, message);
                    return Ok(());
                }

                let players = room.players.iter().copied().collect_vec();
                let objects = if let Some(contents) = world.get::<Contents>(room_entity) {
                    contents.objects.iter().copied().collect_vec()
                } else {
                    bail!("{:?} has no Contents.", room_entity);
                };

                (room.id, players, objects)
            }
            None => bail!("{:?} has no Room", room_entity),
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

        // Persist the changes
        let mut updates = world.get_resource_mut::<Updates>().unwrap();
        updates.queue(persist::room::Remove::new(room_id));

        for object in present_objects {
            updates.queue(persist::room::AddObject::new(room_entity, object));
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
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let room_entity = match world.get::<Player>(player).map(|player| player.room) {
            Some(room) => room,
            None => bail!("{:?} has no Player.", player),
        };

        let mut room = match world.get_mut::<Room>(room_entity) {
            Some(room) => room,
            None => bail!("{:?} has no Room.", room_entity),
        };

        let removed = room.exits.remove(&self.direction).is_some();
        let message = if removed {
            format!("Removed exit {}.", self.direction.as_to_str())
        } else {
            format!("There is no exit {}.", self.direction.as_to_str())
        };

        queue_message(world, player, message);

        Ok(())
    }
}
