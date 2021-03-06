use bevy_app::{EventReader, EventWriter};
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::engine::persist::UpdateGroup;
use crate::world::action::into_action;
use crate::world::types::Contents;
use crate::{
    engine::persist::{self, Updates},
    text::Tokenizer,
    world::{
        action::{observe::Look, Action},
        scripting::QueuedAction,
        types::{
            player::Messages,
            room::{Direction, Room, RoomId, Rooms},
            Id, Location, Named,
        },
    },
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Move {
    pub actor: Entity,
    pub direction: Direction,
}

into_action!(Move);

#[tracing::instrument(name = "move system", skip_all)]
pub fn move_system(
    mut action_reader: EventReader<Action>,
    mut pre_events: EventWriter<QueuedAction>,
    mut updates: ResMut<Updates>,
    mut moving_query: Query<(&Id, &Named, &mut Location)>,
    mut room_query: Query<&mut Room>,
    mut contents_query: Query<&mut Contents>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Move(Move { actor, direction }) = action {
            // Retrieve information about the moving entity.
            let (id, name, mut location) =
                if let Ok((id, named, location)) = moving_query.get_mut(*actor) {
                    (id, named.as_str(), location)
                } else {
                    tracing::warn!("cannot move {:?} without Named and Location.", actor);
                    continue;
                };

            // Retrieve information about the origin/current room.
            let current_room = if let Ok(room) = room_query.get_mut(location.entity()) {
                room
            } else {
                tracing::warn!("cannot move {:?} without being in a room.", actor);
                continue;
            };

            let (destination, origin_players, room_id) =
                if let Some(destination) = current_room.exit(direction) {
                    (
                        destination,
                        current_room
                            .players()
                            .iter()
                            .filter(|present_player| **present_player != *actor)
                            .copied()
                            .collect_vec(),
                        current_room.id(),
                    )
                } else {
                    if let Ok(mut messages) = messages_query.get_mut(*actor) {
                        messages.queue(format!("There is no exit {}.", direction.as_to_str()));
                    }
                    continue;
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
            let destination_room = room_query
                .get_mut(destination)
                .expect("Destinations are valid rooms.");

            let destination_id = destination_room.id();

            let from_direction = destination_room
                .exits()
                .iter()
                .find(|(_, room)| **room == location.entity())
                .map(|(direction, _)| direction)
                .copied();

            let destination_players = destination_room
                .players()
                .iter()
                .filter(|present_player| **present_player != *actor)
                .copied()
                .collect_vec();

            // Move the entity.
            match id {
                Id::Player(_) => {
                    room_query
                        .get_mut(location.entity())
                        .unwrap()
                        .remove_player(*actor);
                    room_query
                        .get_mut(destination)
                        .unwrap()
                        .insert_player(*actor);
                }
                Id::Object(_) => {
                    contents_query
                        .get_mut(location.entity())
                        .unwrap()
                        .remove(*actor);
                    contents_query.get_mut(destination).unwrap().insert(*actor);
                }
                Id::Room(_) => todo!(),
                Id::Prototype(_) => todo!(),
            }

            location.set_entity(destination);

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
                    // TODO: why is there a * below?
                    updates.persist(persist::player::Room::new(*id, destination_id));
                    pre_events.send(
                        Action::Look(Look {
                            actor: *actor,
                            direction: None,
                        })
                        .into(),
                    );
                }
                Id::Object(id) => {
                    let group = UpdateGroup::new(vec![
                        persist::room::RemoveObject::new(room_id, *id),
                        persist::room::AddObject::new(destination_id, *id),
                    ]);
                    updates.persist(group);
                }
                Id::Room(_) => todo!(),
                Id::Prototype(_) => todo!(),
            }
        }
    }
}

pub fn parse_teleport(player: Entity, mut tokenizer: Tokenizer) -> Result<Action, String> {
    if let Some(destination) = tokenizer.next() {
        match destination.parse::<RoomId>() {
            Ok(room_id) => Ok(Action::from(Teleport {
                actor: player,
                room_id,
            })),
            Err(e) => Err(e.to_string()),
        }
    } else {
        Err("Teleport to where?".to_string())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Teleport {
    pub actor: Entity,
    pub room_id: RoomId,
}

into_action!(Teleport);

#[tracing::instrument(name = "teleport system", skip_all)]
pub fn teleport_system(
    mut action_reader: EventReader<Action>,
    mut pre_events: EventWriter<QueuedAction>,
    rooms: Res<Rooms>,
    mut updates: ResMut<Updates>,
    mut moving_query: Query<(&Id, &Named, &mut Location)>,
    mut room_query: Query<&mut Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Teleport(Teleport { actor, room_id }) = action {
            let destination = if let Some(entity) = rooms.by_id(*room_id) {
                entity
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Room {} doesn't exist.", room_id));
                }
                continue;
            };

            // Retrieve information about the moving entity.
            let (id, name, mut location) =
                if let Ok((id, named, location)) = moving_query.get_mut(*actor) {
                    (id, named.as_str(), location)
                } else {
                    tracing::warn!("cannot teleport {:?} without Named and Location.", actor);
                    continue;
                };

            let current_room = if let Ok(room) = room_query.get_mut(location.entity()) {
                room
            } else {
                tracing::warn!("cannot teleport {:?} without being in a room.", actor);
                continue;
            };

            // Retrieve information about the origin/current room.
            let origin_players = current_room
                .players()
                .iter()
                .filter(|present_player| **present_player != *actor)
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
            let destination_players = room_query
                .get_mut(destination)
                .expect("Destinations are valid rooms.")
                .players()
                .iter()
                .filter(|present_player| **present_player != *actor)
                .copied()
                .collect_vec();

            // Move the entity.
            match id {
                Id::Player(_) => {
                    room_query
                        .get_mut(location.entity())
                        .unwrap()
                        .remove_player(*actor);
                    room_query
                        .get_mut(destination)
                        .unwrap()
                        .insert_player(*actor);
                }
                Id::Object(_) => todo!(),
                Id::Room(_) => todo!(),
                Id::Prototype(_) => todo!(),
            }

            location.set_entity(destination);

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
                    updates.persist(persist::player::Room::new(*id, *room_id));
                    pre_events.send(QueuedAction {
                        action: Action::Look(Look {
                            actor: *actor,
                            direction: None,
                        }),
                    });
                }
                Id::Object(_) => todo!(),
                Id::Room(_) => todo!(),
                Id::Prototype(_) => todo!(),
            }
        }
    }
}
