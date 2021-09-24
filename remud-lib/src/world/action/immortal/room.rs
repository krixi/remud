use std::{collections::HashMap, convert::TryFrom, str::FromStr};

use bevy_app::{EventReader, EventWriter};
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persist::{self, UpdateGroup, Updates},
    text::{word_list, Tokenizer},
    world::{
        action::{
            immortal::{Initialize, ShowError, UpdateDescription, UpdateName},
            into_action,
            observe::Look,
            Action,
        },
        scripting::{
            time::Timers, ExecutionErrors, QueuedAction, ScriptData, ScriptHook, ScriptHooks,
            ScriptName,
        },
        types::{
            object::{Container, Object},
            player::{Messages, Player},
            room::{Direction, Regions, Room, RoomBundle, RoomId, Rooms},
            ActionTarget, Contents, Description, Id, Location, Named,
        },
        VOID_ROOM_ID,
    },
};

pub const DEFAULT_ROOM_NAME: &str = "Room";
pub const DEFAULT_ROOM_DESCRIPTION: &str = "An empty room.";

// Valid shapes:
// room info - displays information about the room
// room new - creates a new unlinked room
// room new [direction] - creates a room to the [Direction] of this one with a two way link
// room desc [description] - sets the description of a room
// room link [direction] [room ID] - links the current room to another in a given direction (one way)
// room unlink [direction] - removes an exit from this room
// room region - sets the list of regions for the current room
// room remove - removes the current room and moves everything in it to the void room
pub fn parse_room(player: Entity, mut tokenizer: Tokenizer) -> Result<Action, String> {
    if let Some(subcommand) = tokenizer.next() {
        match subcommand.to_lowercase().as_str() {
            "error" => {
                if tokenizer.rest().is_empty() {
                    Err("Enter a script to look for its errors.".to_string())
                } else {
                    let script = ScriptName::try_from(tokenizer.next().unwrap().to_string())
                        .map_err(|e| e.to_string())?;
                    Ok(Action::from(ShowError {
                        actor: player,
                        target: ActionTarget::CurrentRoom,
                        script,
                    }))
                }
            }
            "desc" => {
                if tokenizer.rest().is_empty() {
                    Err("Enter a description.".to_string())
                } else {
                    Ok(Action::from(UpdateDescription {
                        actor: player,
                        target: ActionTarget::CurrentRoom,
                        description: tokenizer.rest().to_string(),
                    }))
                }
            }
            "info" => Ok(Action::from(RoomInfo { actor: player })),
            "init" => Ok(Action::from(Initialize {
                actor: player,
                target: ActionTarget::CurrentRoom,
            })),
            "link" => {
                if let Some(direction) = tokenizer.next() {
                    let direction = match Direction::from_str(direction) {
                        Ok(direction) => direction,
                        Err(_) => {
                            return Err("Enter a valid direction: up, down, north, east, south, \
                                        west."
                                .to_string())
                        }
                    };

                    if let Some(destination) = tokenizer.next() {
                        let destination = match destination.parse::<RoomId>() {
                            Ok(destination) => destination,
                            Err(e) => return Err(e.to_string()),
                        };

                        Ok(Action::from(RoomLink {
                            actor: player,
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
            "name" => {
                if tokenizer.rest().is_empty() {
                    Err("Enter a name.".to_string())
                } else {
                    Ok(Action::from(UpdateName {
                        actor: player,
                        target: ActionTarget::CurrentRoom,
                        name: tokenizer.rest().to_string(),
                    }))
                }
            }
            "new" => {
                let direction = if let Some(direction) = tokenizer.next() {
                    match Direction::from_str(direction) {
                        Ok(direction) => Some(direction),
                        Err(_) => {
                            return Err("Enter a valid direction: up, down, north, east, south, \
                                        west."
                                .to_string())
                        }
                    }
                } else {
                    None
                };

                Ok(Action::from(RoomCreate {
                    actor: player,
                    direction,
                }))
            }
            "regions" => {
                if let Some(operation) = tokenizer.next() {
                    if tokenizer.rest().is_empty() {
                        Err("Enter one or more space separated regions.".to_string())
                    } else {
                        let regions = tokenizer
                            .rest()
                            .split(' ')
                            .map(|s| s.to_string())
                            .collect_vec();

                        match operation {
                            "add" => Ok(Action::from(RoomUpdateRegions {
                                actor: player,
                                remove: false,
                                regions,
                            })),
                            "remove" => Ok(Action::from(RoomUpdateRegions {
                                actor: player,
                                remove: true,
                                regions,
                            })),
                            _ => Err("Enter a valid region operation: add or remove.".to_string()),
                        }
                    }
                } else {
                    Err("Enter a region operation: add or remove.".to_string())
                }
            }
            "remove" => Ok(Action::from(RoomRemove { actor: player })),
            "unlink" => {
                if let Some(direction) = tokenizer.next() {
                    let direction = match Direction::from_str(direction) {
                        Ok(direction) => direction,
                        Err(_) => {
                            return Err("Enter a valid direction: up, down, north, east, south, \
                                        west."
                                .to_string())
                        }
                    };

                    Ok(Action::from(RoomUnlink {
                        actor: player,
                        direction,
                    }))
                } else {
                    Err("Enter a direction.".to_string())
                }
            }
            _ => Err(
                "Enter a valid room subcommand: info, desc, link, new, regions, remove, or unlink."
                    .to_string(),
            ),
        }
    } else {
        Err(
            "Enter a room subcommand: info, desc, link, new, regions, remove, or unlink."
                .to_string(),
        )
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RoomCreate {
    pub actor: Entity,
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
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomCreate(RoomCreate { actor, direction }) = action {
            let current_room_entity = if let Ok(location) = player_query.get(*actor) {
                location.room()
            } else {
                tracing::info!("Player {:?} cannot create a room from nowhere.", actor);
                continue;
            };

            if let Some(direction) = direction {
                if room_query
                    .get_mut(current_room_entity)
                    .unwrap()
                    .exit(direction)
                    .is_some()
                {
                    if let Ok(mut messages) = messages_query.get_mut(*actor) {
                        messages.queue(format!("A room already exists {}.", direction.as_to_str()));
                    }
                    continue;
                }
            }

            let new_room_id = rooms.next_id();
            let mut exits = HashMap::new();
            if let Some(direction) = direction {
                exits.insert(direction.opposite(), current_room_entity);
            }

            let new_room_entity = commands
                .spawn_bundle(RoomBundle {
                    id: Id::Room(new_room_id),
                    room: Room::new(new_room_id, exits, Vec::new()),
                    regions: Regions::default(),
                    name: Named::from(DEFAULT_ROOM_NAME.to_string()),
                    description: Description::from(DEFAULT_ROOM_DESCRIPTION.to_string()),
                    contents: Contents::default(),
                })
                .id();

            rooms.insert(new_room_id, new_room_entity);

            if let Some(direction) = direction {
                room_query
                    .get_mut(current_room_entity)
                    .unwrap()
                    .insert_exit(*direction, new_room_entity);
            }

            let current_room_id = room_query.get_mut(current_room_entity).unwrap().id();
            let mut update = UpdateGroup::new(vec![persist::room::Create::new(
                new_room_id,
                DEFAULT_ROOM_NAME.to_string(),
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
            updates.persist(update);

            let mut message = format!("Created room {}", new_room_id);
            if let Some(direction) = direction {
                message.push(' ');
                message.push_str(direction.as_to_str());
            }
            message.push('.');
            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RoomInfo {
    pub actor: Entity,
}

into_action!(RoomInfo);

pub fn room_info_system(
    mut action_reader: EventReader<Action>,
    player_query: Query<&Location, With<Player>>,
    room_query: Query<(
        &Room,
        &Named,
        &Description,
        &Regions,
        &Contents,
        Option<&ScriptHooks>,
        Option<&Timers>,
        Option<&ScriptData>,
        Option<&ExecutionErrors>,
    )>,
    named_query: Query<&Named>,
    object_query: Query<(&Object, &Named)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomInfo(RoomInfo { actor }) = action {
            let room_entity = if let Ok(location) = player_query.get(*actor) {
                location.room()
            } else {
                tracing::info!("Player {:?} cannot create a room from nowhere.", actor);
                continue;
            };

            let (room, named, description, regions, contents, hooks, timers, data, errors) =
                room_query.get(room_entity).unwrap();

            let mut message = format!("|white|Room {}|-|", room.id());

            message.push_str("\r\n  |white|name|-|: ");
            message.push_str(named.escaped().as_str());

            message.push_str("\r\n  |white|description|-|: ");
            message.push_str(description.escaped().as_str());

            message.push_str("\r\n  |white|exits|-|:");
            room.exits()
                .iter()
                .filter_map(|(direction, room)| {
                    room_query
                        .get(*room)
                        .map(|(room, named, _, _, _, _, _, _, _)| {
                            (direction, named.as_str(), room.id())
                        })
                        .ok()
                })
                .for_each(|(direction, name, room_id)| {
                    message.push_str(
                        format!("\r\n    {}: {} (room {})", direction, name, room_id).as_str(),
                    )
                });

            message.push_str("\r\n  |white|regions|-|: ");
            if regions.is_empty() {
                message.push_str("none");
            } else {
                message.push_str(word_list(regions.get_list()).as_str());
            }

            message.push_str("\r\n  |white|players|-|:");
            room.players()
                .iter()
                .filter_map(|player| named_query.get(*player).ok())
                .map(|named| named.as_str())
                .for_each(|name| message.push_str(format!("\r\n    {}", name).as_str()));

            message.push_str("\r\n  |white|objects|-|:");
            contents
                .objects()
                .iter()
                .filter_map(|object| object_query.get(*object).ok())
                .map(|(object, named)| (object.id(), named.as_str()))
                .for_each(|(id, name)| {
                    message.push_str(
                        format!(
                            "\r\n    object {}: {}",
                            id,
                            name.replace("|", "||").as_str()
                        )
                        .as_str(),
                    )
                });

            message.push_str("\r\n  |white|script hooks|-|:");
            if let Some(hooks) = hooks {
                if hooks.is_empty() {
                    message.push_str(" none");
                }
                for ScriptHook { trigger, script } in hooks.hooks().iter() {
                    message.push_str(format!("\r\n    {:?} -> {}", trigger, script).as_str());

                    if errors.map(|e| e.has_error(script)).unwrap_or(false) {
                        message.push_str(" |red|(error)|-|");
                    }
                }
            } else {
                message.push_str(" none");
            }

            message.push_str("\r\n  |white|script data|-|:");
            if let Some(data) = data {
                if data.is_empty() {
                    message.push_str(" none");
                } else {
                    for (k, v) in data.map() {
                        message.push_str(format!("\r\n    {} -> {:?}", k, v).as_str());
                    }
                }
            } else {
                message.push_str(" none");
            }

            message.push_str("\r\n  |white|timers|-|:");
            if let Some(timers) = timers {
                if timers.timers().is_empty() {
                    message.push_str(" none");
                }
                for (name, timer) in timers.timers().iter() {
                    message.push_str(
                        format!(
                            "\r\n    {}: {}/{}ms",
                            name,
                            timer.elapsed().as_millis(),
                            timer.duration().as_millis()
                        )
                        .as_str(),
                    )
                }
            } else {
                message.push_str(" none");
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RoomLink {
    pub actor: Entity,
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
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomLink(RoomLink {
            actor,
            direction,
            destination,
        }) = action
        {
            let to_room_entity = if let Some(room) = rooms.by_id(*destination) {
                room
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!("Room {} does not exist.", destination));
                }
                continue;
            };

            let from_room_entity = player_query
                .get(*actor)
                .map(|location| location.room())
                .unwrap();

            let from_room_id = {
                let mut from_room = room_query.get_mut(from_room_entity).unwrap();
                from_room.insert_exit(*direction, to_room_entity);
                from_room.id()
            };

            updates.persist(persist::room::AddExit::new(
                from_room_id,
                *destination,
                *direction,
            ));

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!(
                    "Linked {} exit to room {}.",
                    direction, destination
                ));
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RoomRemove {
    pub actor: Entity,
}

into_action!(RoomRemove);

pub fn room_remove_system(
    mut commands: Commands,
    mut action_reader: EventReader<Action>,
    mut queued_action_writer: EventWriter<QueuedAction>,
    mut rooms: ResMut<Rooms>,
    mut updates: ResMut<Updates>,
    mut player_query: Query<(&Player, &mut Location)>,
    mut room_query: Query<(&mut Room, &mut Contents)>,
    mut object_query: Query<(&Object, &mut Container)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomRemove(RoomRemove { actor }) = action {
            let room_entity = player_query
                .get_mut(*actor)
                .map(|(_, location)| location.room())
                .unwrap();

            // Retrieve information about the current room.
            let (room, contents) = room_query.get_mut(room_entity).unwrap();

            if room.id() == *VOID_ROOM_ID {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue("You cannot remove the void room.".to_string())
                }
                continue;
            }

            let room_id = room.id();
            let players = room.get_players();
            let objects = contents.get_objects();

            // Move all objects and players to the void room.
            let void_room_entity = rooms.by_id(*VOID_ROOM_ID).unwrap();
            {
                let (mut room, mut contents) = room_query.get_mut(void_room_entity).unwrap();
                for player in players.iter() {
                    room.insert_player(*player);
                }
                for object in objects.iter() {
                    contents.insert(*object);
                }
            }

            for player in players.iter() {
                if let Ok(mut messages) = messages_query.get_mut(*player) {
                    messages.queue("The world begins to disintigrate around you.".to_string());
                }

                player_query
                    .get_mut(*player)
                    .map(|(_, location)| location)
                    .unwrap()
                    .set_room(void_room_entity);

                queued_action_writer.send(
                    Action::from(Look {
                        actor: *player,
                        direction: None,
                    })
                    .into(),
                );
            }

            for object in objects.iter() {
                object_query
                    .get_mut(*object)
                    .map(|(_, container)| container)
                    .unwrap()
                    .set_entity(void_room_entity);
            }

            // Remove the room
            rooms.remove(room_id);
            commands.entity(room_entity).despawn();

            // Find and remove all exits to the room
            for (mut room, _) in room_query.iter_mut() {
                let to_remove = room
                    .exits()
                    .iter()
                    .filter(|(_, entity)| **entity == room_entity)
                    .map(|(direction, _)| *direction)
                    .collect_vec();

                for direction in to_remove {
                    room.remove_exit(&direction);
                }
            }

            // Gather all IDs for persistence
            let present_player_ids = players
                .iter()
                .filter_map(|player| {
                    player_query
                        .get_mut(*player)
                        .map(|(player, _)| player.id())
                        .ok()
                })
                .collect_vec();

            let present_object_ids = objects
                .iter()
                .filter_map(|object| {
                    object_query
                        .get_mut(*object)
                        .map(|(object, _)| object.id())
                        .ok()
                })
                .collect_vec();

            updates.persist(persist::room::Delete::new(room_id));

            for id in present_player_ids {
                updates.persist(persist::player::Room::new(id, *VOID_ROOM_ID));
            }

            for id in present_object_ids {
                updates.persist(persist::room::AddObject::new(*VOID_ROOM_ID, id));
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Room {} removed.", room_id));
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RoomUnlink {
    pub actor: Entity,
    pub direction: Direction,
}

into_action!(RoomUnlink);

pub fn room_unlink_system(
    mut action_reader: EventReader<Action>,
    mut updates: ResMut<Updates>,
    player_query: Query<&Location, With<Player>>,
    mut room_query: Query<&mut Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomUnlink(RoomUnlink { actor, direction }) = action {
            let room_entity = player_query
                .get(*actor)
                .map(|location| location.room())
                .unwrap();

            let mut room = room_query.get_mut(room_entity).unwrap();

            let removed = room.remove_exit(direction).is_some();

            updates.persist(persist::room::RemoveExit::new(room.id(), *direction));

            let message = if removed {
                format!("Removed exit {}.", direction.as_to_str())
            } else {
                format!("There is no exit {}.", direction.as_to_str())
            };

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RoomUpdateRegions {
    pub actor: Entity,
    pub remove: bool,
    pub regions: Vec<String>,
}

into_action!(RoomUpdateRegions);

pub fn room_update_regions_system(
    mut action_reader: EventReader<Action>,
    mut updates: ResMut<Updates>,
    player_query: Query<&Location, With<Player>>,
    mut room_query: Query<(&Room, &mut Regions)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::RoomUpdateRegions(RoomUpdateRegions {
            actor,
            remove,
            regions,
        }) = action
        {
            let room_entity = player_query
                .get(*actor)
                .map(|location| location.room())
                .unwrap();

            let (room, mut room_regions) = room_query.get_mut(room_entity).unwrap();
            if *remove {
                for region in regions {
                    room_regions.remove(region.as_str())
                }
            } else {
                room_regions.extend(regions.iter().cloned())
            }

            if *remove {
                updates.persist(persist::room::RemoveRegions::new(
                    room.id(),
                    room_regions.get_list(),
                ));
            } else {
                updates.persist(persist::room::AddRegions::new(
                    room.id(),
                    room_regions.get_list(),
                ));
            }

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue(format!("Updated room {} regions.", room.id()));
            }
        }
    }
}
