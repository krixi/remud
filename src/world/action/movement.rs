use bevy_app::Events;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, Updates},
    text::Tokenizer,
    world::{
        action::{self, Action, ActionEvent, DynAction},
        types::{
            player::Messages,
            room::{self, Direction, Room, Rooms},
            Id, Location, Named,
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
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::Move {
                entity,
                direction: self.direction,
            });

        Ok(())
    }
}

pub fn move_system(
    mut events: ResMut<Events<ActionEvent>>,
    mut updates: ResMut<Updates>,
    mut moving_query: Query<(&Id, &Named, &mut Location)>,
    mut room_query: Query<&mut Room>,
    mut messages_query: Query<&mut Messages>,
) {
    let mut events_to_send = Vec::new();

    for event in events.get_reader().iter(&*events) {
        if let ActionEvent::Move { entity, direction } = event {
            // Retrieve information about the moving entity.
            let (id, name, mut location) =
                if let Ok((id, named, location)) = moving_query.get_mut(*entity) {
                    (id, named.name.as_str(), location)
                } else {
                    tracing::warn!("Cannot move {:?} without Named and Location.", entity);
                    continue;
                };

            // Retrieve information about the origin/current room.
            let (destination, origin_players) = {
                let room = room_query
                    .get_mut(location.room)
                    .expect("Location contains a valid room.");

                if let Some(destination) = room.exits.get(&direction) {
                    (
                        *destination,
                        room.players
                            .iter()
                            .filter(|present_player| **present_player != *entity)
                            .copied()
                            .collect_vec(),
                    )
                } else {
                    if let Ok(mut messages) = messages_query.get_mut(*entity) {
                        messages.queue(format!("There is no exit {}.", direction.as_to_str()));
                    }
                    continue;
                }
            };

            // Notify players in the origin room that something is leaving.
            let leave_message = format!("{} leaves {}.", name, direction.as_to_str());
            for player in origin_players {
                messages_query
                    .get_mut(player)
                    .unwrap_or_else(|_| panic!("Player {:?} has Messages.", player))
                    .queue(leave_message.clone());
            }

            // Retrieve information about the destination room.
            let (destination_id, from_direction, destination_players) = {
                let room = room_query
                    .get_mut(destination)
                    .expect("Destinations are valid rooms.");

                let direction = room
                    .exits
                    .iter()
                    .find(|(_, room)| **room == location.room)
                    .map(|(direction, _)| direction)
                    .copied();

                let present_players = room
                    .players
                    .iter()
                    .filter(|present_player| **present_player != *entity)
                    .copied()
                    .collect_vec();

                (room.id, direction, present_players)
            };

            // Move the entity.
            match id {
                Id::Player(_) => {
                    room_query
                        .get_mut(location.room)
                        .unwrap()
                        .remove_player(*entity);
                    room_query
                        .get_mut(destination)
                        .unwrap()
                        .players
                        .push(*entity);
                }
                Id::Object(_) => todo!(),
                Id::Room(_) => todo!(),
            }

            location.room = destination;

            // Notify players in the destination room that something has arrived.
            let arrive_message = from_direction.map_or_else(
                || format!("{} appears.", name),
                |from| format!("{} arrives {}.", name, from.as_from_str()),
            );
            for player in destination_players {
                messages_query
                    .get_mut(player)
                    .unwrap_or_else(|_| panic!("Player {:?} has Messages.", player))
                    .queue(arrive_message.clone());
            }

            // Dispatch a storage update to the new location.
            match id {
                Id::Player(id) => {
                    updates.queue(persist::player::Room::new(*id, destination_id));
                    events_to_send.push(ActionEvent::Look {
                        entity: *entity,
                        direction: None,
                    });
                }
                Id::Object(_) => todo!(),
                Id::Room(_) => todo!(),
            }
        }
    }

    for event in events_to_send {
        events.send(event);
    }
}

pub fn parse_teleport(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(destination) = tokenizer.next() {
        match destination.parse::<room::Id>() {
            Ok(room_id) => Ok(Box::new(Teleport { room_id })),
            Err(e) => Err(e.to_string()),
        }
    } else {
        Err("Teleport to where?".to_string())
    }
}

pub struct Teleport {
    room_id: room::Id,
}

impl Action for Teleport {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::Teleport {
                entity,
                room_id: self.room_id,
            });

        Ok(())
    }
}

pub fn teleport_system(
    mut events: ResMut<Events<ActionEvent>>,
    rooms: Res<Rooms>,
    mut updates: ResMut<Updates>,
    mut moving_query: Query<(&Id, &Named, &mut Location)>,
    mut room_query: Query<&mut Room>,
    mut messages_query: Query<&mut Messages>,
) {
    let mut events_to_send = Vec::new();
    for event in events.get_reader().iter(&*events) {
        if let ActionEvent::Teleport {
            entity,
            room_id: destination_id,
        } = event
        {
            let destination = if let Some(entity) = rooms.by_id(*destination_id) {
                entity
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!("Room {} doesn't exist.", destination_id));
                }
                continue;
            };

            // Retrieve information about the moving entity.
            let (id, name, mut location) =
                if let Ok((id, named, location)) = moving_query.get_mut(*entity) {
                    (id, named.name.as_str(), location)
                } else {
                    tracing::warn!("Cannot teleport {:?} without Named and Location.", entity);
                    continue;
                };

            // Retrieve information about the origin/current room.
            let origin_players = room_query
                .get_mut(location.room)
                .expect("Location contains a valid room.")
                .players
                .iter()
                .filter(|present_player| **present_player != *entity)
                .copied()
                .collect_vec();

            // Notify players in the origin room that something is leaving.
            let leave_message = format!("{} disappears in the blink of an eye.", name);
            for player in origin_players {
                messages_query
                    .get_mut(player)
                    .unwrap_or_else(|_| panic!("Player {:?} has Messages.", player))
                    .queue(leave_message.clone());
            }

            // Retrieve information about the destination room.
            let (destination_id, destination_players) = {
                let room = room_query
                    .get_mut(destination)
                    .expect("Destinations are valid rooms.");

                let present_players = room
                    .players
                    .iter()
                    .filter(|present_player| **present_player != *entity)
                    .copied()
                    .collect_vec();

                (room.id, present_players)
            };

            // Move the entity.
            match id {
                Id::Player(_) => {
                    room_query
                        .get_mut(location.room)
                        .unwrap()
                        .remove_player(*entity);
                    room_query
                        .get_mut(destination)
                        .unwrap()
                        .players
                        .push(*entity);
                }
                Id::Object(_) => todo!(),
                Id::Room(_) => todo!(),
            }

            location.room = destination;

            // Notify players in the destination room that something has arrived.
            let arrive_message = format!("{} appears in a flash of light.", name);
            for player in destination_players {
                messages_query
                    .get_mut(player)
                    .unwrap_or_else(|_| panic!("Player {:?} has Messages.", player))
                    .queue(arrive_message.clone());
            }

            // Dispatch a storage update to the new location.
            match id {
                Id::Player(id) => {
                    updates.queue(persist::player::Room::new(*id, destination_id));
                    events_to_send.push(ActionEvent::Look {
                        entity: *entity,
                        direction: None,
                    })
                }
                Id::Object(_) => todo!(),
                Id::Room(_) => todo!(),
            }
        }
    }
    for event in events_to_send {
        events.send(event);
    }
}
