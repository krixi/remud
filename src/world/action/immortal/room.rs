use std::{collections::HashMap, str::FromStr};

use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, UpdateGroup, Updates},
    into_action,
    text::Tokenizer,
    world::{
        action::{Action, DEFAULT_ROOM_DESCRIPTION},
        scripting::{ScriptHook, ScriptHooks},
        types::{
            object::Object,
            player::{Messages, Player},
            room::{Direction, Room, RoomBundle, RoomId, Rooms},
            Container, Contents, Description, Id, Location, Named,
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
pub fn parse_room(player: Entity, mut tokenizer: Tokenizer) -> Result<Action, String> {
    if let Some(subcommand) = tokenizer.next() {
        match subcommand.to_lowercase().as_str() {
            "info" => Ok(Action::from(RoomInfo { entity: player })),
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

                Ok(Action::from(RoomCreate {
                    entity: player,
                    direction,
                }))
            }
            "desc" => {
                if tokenizer.rest().is_empty() {
                    Err("Enter a description.".to_string())
                } else {
                    Ok(Action::from(RoomUpdateDescription {
                        entity: player,
                        description: tokenizer.rest().to_string(),
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
                        let destination = match destination.parse::<RoomId>() {
                            Ok(destination) => destination,
                            Err(e) => return Err(e.to_string()),
                        };

                        Ok(Action::from(RoomLink {
                            entity: player,
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
            "remove" => Ok(Action::from(RoomRemove { entity: player })),
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

                    Ok(Action::from(RoomUnlink {
                        entity: player,
                        direction,
                    }))
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

#[derive(Debug, Clone)]
pub struct RoomCreate {
    pub entity: Entity,
    pub direction: Option<Direction>,
}

into_action!(RoomCreate);

pub fn room_create_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    mut rooms: ResMut<Rooms>,
    mut updates: ResMut<Updates>,
    player_query: Query<&Location, With<Player>>,
    mut room_query: Query<&mut Room>,
    mut message_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomCreate(RoomCreate { entity, direction }) = action {
            let current_room_entity =
                if let Ok(room) = player_query.get(*entity).map(|location| location.room) {
                    room
                } else {
                    tracing::info!("Player {:?} cannot create a room from nowhere.", entity);
                    continue;
                };

            if let Some(direction) = direction {
                if room_query
                    .get_mut(current_room_entity)
                    .unwrap()
                    .exits
                    .contains_key(direction)
                {
                    if let Ok(mut messages) = message_query.get_mut(*entity) {
                        messages.queue(format!("A room already exists {}.", direction.as_to_str()));
                    }
                    continue;
                }
            }

            let mut exits = HashMap::new();
            if let Some(direction) = direction {
                exits.insert(direction.opposite(), current_room_entity);
            }

            let new_room_id = rooms.next_id();
            let new_room_entity = commands
                .spawn_bundle(RoomBundle {
                    id: Id::Room(new_room_id),
                    room: Room {
                        id: new_room_id,
                        exits,
                        players: Vec::new(),
                    },
                    description: Description {
                        text: DEFAULT_ROOM_DESCRIPTION.to_string(),
                    },
                    contents: Contents::default(),
                })
                .id();

            rooms.insert(new_room_id, new_room_entity);

            if let Some(direction) = direction {
                room_query
                    .get_mut(current_room_entity)
                    .unwrap()
                    .exits
                    .insert(*direction, new_room_entity);
            }

            let current_room_id = room_query.get_mut(current_room_entity).unwrap().id;
            let mut update = UpdateGroup::new(vec![persist::room::Create::new(
                new_room_id,
                DEFAULT_ROOM_DESCRIPTION.to_string(),
            )]);
            if let Some(direction) = direction {
                update.append(persist::room::AddExit::new(
                    current_room_id,
                    new_room_id,
                    *direction,
                ));
                update.append(persist::room::AddExit::new(
                    new_room_id,
                    current_room_id,
                    direction.opposite(),
                ));
            }
            updates.queue(update);

            let mut message = format!("Created room {}", new_room_id);
            if let Some(direction) = direction {
                message.push(' ');
                message.push_str(direction.as_to_str());
            }
            message.push('.');
            if let Ok(mut messages) = message_query.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoomInfo {
    pub entity: Entity,
}

into_action!(RoomInfo);

pub fn room_info_system(
    mut action_reader: EventReader<Action>,
    player_query: Query<&Location, With<Player>>,
    room_query: Query<(&Room, &Description, &Contents, Option<&ScriptHooks>)>,
    named_query: Query<&Named>,
    object_query: Query<(&Object, &Named)>,
    mut message_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomInfo(RoomInfo { entity }) = action {
            let room_entity =
                if let Ok(room) = player_query.get(*entity).map(|location| location.room) {
                    room
                } else {
                    tracing::info!("Player {:?} cannot create a room from nowhere.", entity);
                    continue;
                };

            let (room, description, contents, hooks) = room_query.get(room_entity).unwrap();

            let mut message = format!("Room {}", room.id);

            message.push_str("\r\n  description: ");
            message.push_str(description.text.as_str());

            message.push_str("\r\n  exits:");
            room.exits
                .iter()
                .filter_map(|(direction, room)| {
                    room_query
                        .get(*room)
                        .map(|(room, _, _, _)| (direction, room.id))
                        .ok()
                })
                .for_each(|(direction, room_id)| {
                    message.push_str(format!("\r\n    {}: room {}", direction, room_id).as_str())
                });

            message.push_str("\r\n  players:");
            room.players
                .iter()
                .filter_map(|player| named_query.get(*player).ok())
                .map(|named| named.name.as_str())
                .for_each(|name| message.push_str(format!("\r\n    {}", name).as_str()));

            message.push_str("\r\n  objects:");
            contents
                .objects
                .iter()
                .filter_map(|object| object_query.get(*object).ok())
                .map(|(object, named)| (object.id, named.name.as_str()))
                .for_each(|(id, name)| {
                    message.push_str(format!("\r\n    object {}: {}", id, name).as_str())
                });
            message.push_str("\r\n  script hooks:");
            if let Some(ScriptHooks { list }) = hooks {
                if list.is_empty() {
                    message.push_str(" none");
                }
                for ScriptHook { trigger, script } in list.iter() {
                    message.push_str(format!("\r\n    {:?} -> {}", trigger, script).as_str());
                }
            } else {
                message.push_str(" none");
            }

            if let Ok(mut messages) = message_query.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoomLink {
    pub entity: Entity,
    pub direction: Direction,
    pub destination: RoomId,
}

into_action!(RoomLink);

pub fn room_link_system(
    mut action_reader: EventReader<Action>,
    rooms: Res<Rooms>,
    mut updates: ResMut<Updates>,
    player_query: Query<&Location, With<Player>>,
    mut room_query: Query<&mut Room>,
    mut message_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomLink(RoomLink {
            entity,
            direction,
            destination,
        }) = action
        {
            let to_room_entity = if let Some(room) = rooms.by_id(*destination) {
                room
            } else {
                if let Ok(mut messages) = message_query.get_mut(*entity) {
                    messages.queue(format!("Room {} does not exist.", destination));
                }
                continue;
            };

            let from_room_entity = player_query
                .get(*entity)
                .map(|location| location.room)
                .unwrap();

            let from_room_id = {
                let mut from_room = room_query.get_mut(from_room_entity).unwrap();
                from_room.exits.insert(*direction, to_room_entity);
                from_room.id
            };

            updates.queue(persist::room::AddExit::new(
                from_room_id,
                *destination,
                *direction,
            ));

            if let Ok(mut messages) = message_query.get_mut(*entity) {
                messages.queue(format!(
                    "Linked {} exit to room {}.",
                    direction, destination
                ));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoomUpdateDescription {
    pub entity: Entity,
    pub description: String,
}

into_action!(RoomUpdateDescription);

pub fn room_update_description_system(
    mut action_reader: EventReader<Action>,
    mut updates: ResMut<Updates>,
    player_query: Query<&Location, With<Player>>,
    mut room_query: Query<(&Room, &mut Description)>,
    mut message_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomUpdateDescription(RoomUpdateDescription {
            entity,
            description,
        }) = action
        {
            let room_entity = player_query
                .get(*entity)
                .map(|location| location.room)
                .unwrap();

            let room_id = {
                let (room, mut room_description) = room_query.get_mut(room_entity).unwrap();
                room_description.text = description.clone();
                room.id
            };

            updates.queue(persist::room::Update::new(room_id, description.clone()));

            if let Ok(mut messages) = message_query.get_mut(*entity) {
                messages.queue(format!("Updated room {} description.", room_id));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoomRemove {
    pub entity: Entity,
}

into_action!(RoomRemove);

pub fn room_remove_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    mut rooms: ResMut<Rooms>,
    mut updates: ResMut<Updates>,
    mut player_queries: QuerySet<(
        Query<&Location, With<Player>>,
        Query<(&Player, &mut Location)>,
    )>,
    mut room_query: Query<(&mut Room, &mut Contents)>,
    mut object_query: Query<(&Object, &mut Container)>,
    mut message_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomRemove(RoomRemove { entity }) = action {
            let room_entity = player_queries
                .q0()
                .get(*entity)
                .map(|location| location.room)
                .unwrap();

            // Retrieve information about the current room.
            let (room_id, present_players, present_objects) = {
                let (mut room, mut contents) = room_query.get_mut(room_entity).unwrap();

                if room.id == *VOID_ROOM_ID {
                    if let Ok(mut messages) = message_query.get_mut(*entity) {
                        messages.queue("You cannot remove the void room.".to_string())
                    }
                    continue;
                }

                let players = room.players.drain(..).collect_vec();
                let objects = contents.objects.drain(..).collect_vec();

                (room.id, players, objects)
            };

            // Move all objects and players to the void room.
            let void_room_entity = rooms.by_id(*VOID_ROOM_ID).unwrap();
            {
                let (mut room, mut contents) = room_query.get_mut(void_room_entity).unwrap();
                for player in present_players.iter() {
                    room.players.push(*player);
                }
                for object in present_objects.iter() {
                    contents.objects.push(*object);
                }
            }

            for player in present_players.iter() {
                player_queries
                    .q1_mut()
                    .get_mut(*player)
                    .map(|(_, location)| location)
                    .unwrap()
                    .room = void_room_entity;
            }

            for object in present_objects.iter() {
                object_query
                    .get_mut(*object)
                    .map(|(_, container)| container)
                    .unwrap()
                    .entity = void_room_entity;
            }

            // Remove the room
            rooms.remove(room_id);
            commands.entity(room_entity).despawn();

            // Find and remove all exits to the room
            for (mut room, _) in room_query.iter_mut() {
                let to_remove = room
                    .exits
                    .iter()
                    .filter(|(_, entity)| **entity == room_entity)
                    .map(|(direction, _)| *direction)
                    .collect_vec();

                for direction in to_remove {
                    room.exits.remove(&direction);
                }
            }

            let present_player_ids = present_players
                .iter()
                .filter_map(|player| {
                    player_queries
                        .q1_mut()
                        .get_mut(*player)
                        .map(|(player, _)| player.id)
                        .ok()
                })
                .collect_vec();

            let present_object_ids = present_objects
                .iter()
                .filter_map(|object| {
                    object_query
                        .get_mut(*object)
                        .map(|(object, _)| object.id)
                        .ok()
                })
                .collect_vec();

            updates.queue(persist::room::Delete::new(room_id));

            for id in present_player_ids {
                updates.queue(persist::player::Room::new(id, *VOID_ROOM_ID));
            }

            for id in present_object_ids {
                updates.queue(persist::room::AddObject::new(*VOID_ROOM_ID, id));
            }

            if let Ok(mut messages) = message_query.get_mut(*entity) {
                messages.queue(format!("Room {} removed.", room_id));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoomUnlink {
    pub entity: Entity,
    pub direction: Direction,
}

into_action!(RoomUnlink);

pub fn room_unlink_system(
    mut action_reader: EventReader<Action>,
    mut updates: ResMut<Updates>,
    player_query: Query<&Location, With<Player>>,
    mut room_query: Query<&mut Room>,
    mut message_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomUnlink(RoomUnlink { entity, direction }) = action {
            let room_entity = player_query
                .get(*entity)
                .map(|location| location.room)
                .unwrap();

            let (room_id, removed) = {
                let mut room = room_query.get_mut(room_entity).unwrap();
                let removed = room.exits.remove(direction).is_some();
                (room.id, removed)
            };

            updates.queue(persist::room::RemoveExit::new(room_id, *direction));

            let message = if removed {
                format!("Removed exit {}.", direction.as_to_str())
            } else {
                format!("There is no exit {}.", direction.as_to_str())
            };

            if let Ok(mut messages) = message_query.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}
