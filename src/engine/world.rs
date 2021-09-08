#![allow(clippy::type_complexity)]

use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    fmt,
    str::FromStr,
};

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::{action::DynAction, persistence::DynUpdate},
    queue_message,
    text::word_list,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct RoomId(i64);

impl TryFrom<i64> for RoomId {
    type Error = RoomIdParseError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(RoomId(value))
        } else {
            Err(RoomIdParseError {})
        }
    }
}

impl FromStr for RoomId {
    type Err = RoomIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let int = s.parse::<i64>().map_err(|_| RoomIdParseError {})?;
        RoomId::try_from(int)
    }
}

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug)]
pub struct RoomIdParseError {}
impl fmt::Display for RoomIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Room ID must be a positive integer.")
    }
}
impl std::error::Error for RoomIdParseError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

impl Direction {
    pub fn pretty_from(&self) -> &str {
        match self {
            Direction::North => "from the north",
            Direction::East => "from the east",
            Direction::South => "from the south",
            Direction::West => "from the west",
            Direction::Up => "from above",
            Direction::Down => "from below",
        }
    }

    pub fn pretty_to(&self) -> &str {
        match self {
            Direction::North => "to the north",
            Direction::East => "to the east",
            Direction::South => "to the south",
            Direction::West => "to the west",
            Direction::Up => "above",
            Direction::Down => "below",
        }
    }

    pub fn opposite(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }
}

impl FromStr for Direction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "north" => Ok(Direction::North),
            "east" => Ok(Direction::East),
            "south" => Ok(Direction::South),
            "west" => Ok(Direction::West),
            "up" => Ok(Direction::Up),
            "down" => Ok(Direction::Down),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::North => write!(f, "north"),
            Direction::East => write!(f, "east"),
            Direction::South => write!(f, "south"),
            Direction::West => write!(f, "west"),
            Direction::Up => write!(f, "up"),
            Direction::Down => write!(f, "down"),
        }
    }
}

// Components
pub struct Player {
    pub name: String,
}

pub struct Location {
    pub room: Entity,
}

pub struct Room {
    pub id: RoomId,
    pub description: String,
    pub exits: HashMap<Direction, Entity>,
}

pub struct Messages {
    pub queue: Vec<String>,
}

impl Messages {
    pub fn new_with(message: String) -> Self {
        Messages {
            queue: vec![message],
        }
    }
}

// Resources
pub struct RoomMetadata {
    pub rooms_by_id: HashMap<RoomId, Entity>,
    pub players_by_room: HashMap<Entity, HashSet<Entity>>,
    highest_id: i64,
}

impl RoomMetadata {
    pub fn new(rooms_by_id: HashMap<RoomId, Entity>, highest_id: i64) -> Self {
        RoomMetadata {
            rooms_by_id,
            players_by_room: HashMap::new(),
            highest_id,
        }
    }

    pub fn player_moved(&mut self, player: Entity, from: Entity, to: Entity) {
        if let Some(list) = self.players_by_room.get_mut(&from) {
            list.remove(&player);
        }

        self.players_by_room.entry(to).or_default().insert(player);
    }

    pub fn next_id(&mut self) -> RoomId {
        self.highest_id += 1;
        RoomId(self.highest_id)
    }
}

pub struct Configuration {
    pub shutdown: bool,
    pub spawn_room: RoomId,
}

#[derive(Default)]
pub struct Updates {
    updates: Vec<DynUpdate>,
}

impl Updates {
    pub fn queue(&mut self, update: DynUpdate) {
        self.updates.push(update);
    }
}

pub struct GameWorld {
    world: World,
    schedule: Schedule,
    void_room: Entity,
}

impl GameWorld {
    pub fn new(mut world: World) -> Self {
        // Create emergency room
        let room = Room {
            id: RoomId(0),
            description: "A dark void extends infinitely in all directions.".to_string(),
            exits: HashMap::new(),
        };
        let void_room = world.spawn().insert(room).id();

        // Add resources
        world.insert_resource(Updates::default());

        // Create schedule
        let mut schedule = Schedule::default();

        let mut update = SystemStage::parallel();
        update.add_system(look_system.system());
        update.add_system(move_system.system());
        update.add_system(say_system.system());
        update.add_system(teleport_system.system());
        schedule.add_stage("update", update);

        GameWorld {
            world,
            schedule,
            void_room,
        }
    }

    pub fn run(&mut self) {
        self.schedule.run_once(&mut self.world);
    }

    pub fn should_shutdown(&self) -> bool {
        self.world
            .get_resource::<Configuration>()
            .map_or(true, |configuration| configuration.shutdown)
    }

    pub fn spawn_player(&mut self, name: String) -> Entity {
        let (player, room) = {
            let room = {
                let configuration = self.world.get_resource::<Configuration>().unwrap();
                let room_metadata = self.world.get_resource::<RoomMetadata>().unwrap();

                *room_metadata
                    .rooms_by_id
                    .get(&configuration.spawn_room)
                    .unwrap_or(&self.void_room)
            };

            let player_entity = self
                .world
                .spawn()
                .insert(Player { name })
                .insert(Location { room })
                .insert(WantsToLook {})
                .id();

            (player_entity, room)
        };

        let mut room_metadata = self.world.get_resource_mut::<RoomMetadata>().unwrap();

        room_metadata
            .players_by_room
            .entry(room)
            .or_default()
            .insert(player);

        player
    }

    pub fn despawn_player(&mut self, player_entity: Entity) {
        let location = self
            .world
            .get::<Location>(player_entity)
            .map(|location| location.room);

        self.world.entity_mut(player_entity).despawn();

        if let Some(location) = location {
            let mut room_metadata = self.world.get_resource_mut::<RoomMetadata>().unwrap();
            if let Some(players_by_room) = room_metadata.players_by_room.get_mut(&location) {
                players_by_room.remove(&player_entity);
            }
        }
    }

    pub fn player_action(&mut self, player: Entity, mut action: DynAction) {
        action.enact(player, &mut self.world);
    }

    pub fn messages(&mut self) -> Vec<(Entity, Vec<String>)> {
        let players_with_messages = self
            .world
            .query_filtered::<Entity, (With<Player>, With<Messages>)>()
            .iter(&self.world)
            .collect_vec();

        let mut outgoing = Vec::new();

        for player in players_with_messages {
            if let Some(messages) = self.world.entity_mut(player).remove::<Messages>() {
                outgoing.push((player, messages.queue));
            }
        }

        outgoing
    }

    pub fn updates(&mut self) -> Vec<DynUpdate> {
        let mut updates = self.world.get_resource_mut::<Updates>().unwrap();

        let mut new_updates = Vec::new();
        std::mem::swap(&mut updates.updates, &mut new_updates);

        new_updates
    }

    pub fn get_world(&self) -> &World {
        &self.world
    }
}

pub struct WantsToLook {}

fn look_system(
    mut commands: Commands,
    room_data: Res<RoomMetadata>,
    looking_players: Query<(Entity, &Location), (With<Player>, With<WantsToLook>)>,
    players: Query<&Player>,
    rooms: Query<&Room>,
    mut messages: Query<&mut Messages>,
) {
    for (looking_entity, looking_location) in looking_players.iter() {
        if let Ok(room) = rooms.get(looking_location.room) {
            let mut message = format!("{}\r\n", room.description);

            if let Some(present_players) = room_data.players_by_room.get(&looking_location.room) {
                let mut present_player_names = present_players
                    .iter()
                    .filter(|player| **player != looking_entity)
                    .filter_map(|player| players.get(*player).ok())
                    .map(|player| player.name.clone())
                    .collect_vec();

                if !present_player_names.is_empty() {
                    present_player_names.sort();

                    let singular = present_player_names.len() == 1;

                    let mut player_list = word_list(present_player_names);
                    if singular {
                        player_list.push_str(" is here.");
                    } else {
                        player_list.push_str(" are here.");
                    };
                    message.push_str(player_list.as_str());
                    message.push_str("\r\n");
                }
            }

            queue_message!(commands, messages, looking_entity, message);
        }
        commands.entity(looking_entity).remove::<WantsToLook>();
    }
}

pub struct WantsToMove {
    pub direction: Direction,
}

fn move_system(
    mut commands: Commands,
    mut room_data: ResMut<RoomMetadata>,
    mut moving_players: Query<(Entity, &Player, &WantsToMove, &mut Location)>,
    rooms: Query<&Room>,
    mut messages: Query<&mut Messages>,
) {
    for (moving_player_entity, player, wants_to_move, mut location) in moving_players.iter_mut() {
        let destination = if let Some(destination) = rooms
            .get(location.room)
            .ok()
            .and_then(|room| room.exits.get(&wants_to_move.direction))
        {
            *destination
        } else {
            let message = "There is nothing in that direction.\r\n".to_string();
            queue_message!(commands, messages, moving_player_entity, message);

            commands
                .entity(moving_player_entity)
                .remove::<WantsToMove>();

            continue;
        };

        room_data.player_moved(moving_player_entity, location.room, destination);

        if let Some(present_players) = room_data.players_by_room.get(&location.room) {
            for present_player in present_players {
                if *present_player == moving_player_entity {
                    continue;
                }

                let message = format!(
                    "{} leaves {}.\r\n",
                    player.name,
                    wants_to_move.direction.pretty_to()
                );
                queue_message!(commands, messages, *present_player, message);
            }
        }

        location.room = destination;

        if let Some(present_players) = room_data.players_by_room.get(&destination) {
            for present_player in present_players {
                if *present_player == moving_player_entity {
                    continue;
                }

                let message = format!(
                    "{} enters {}.\r\n",
                    player.name,
                    wants_to_move.direction.opposite().pretty_from()
                );
                queue_message!(commands, messages, *present_player, message);
            }
        }

        commands
            .entity(moving_player_entity)
            .insert(WantsToLook {})
            .remove::<WantsToMove>();
    }
}

pub struct WantsToSay {
    pub message: String,
}

fn say_system(
    mut commands: Commands,
    room_data: Res<RoomMetadata>,
    saying_players: Query<(Entity, &Player, &Location, &WantsToSay)>,
    mut messages: Query<&mut Messages>,
) {
    for (saying_entity, saying_player, saying_location, wants_to_say) in saying_players.iter() {
        if let Some(present_players) = room_data.players_by_room.get(&saying_location.room) {
            for present_player_entity in present_players.iter() {
                if *present_player_entity == saying_entity {
                    continue;
                }

                let message = format!(
                    "{} says \"{}\"\r\n",
                    saying_player.name, wants_to_say.message
                );

                queue_message!(commands, messages, *present_player_entity, message);
            }
        }

        commands.entity(saying_entity).remove::<WantsToSay>();
    }
}

pub struct WantsToTeleport {
    pub room: Entity,
}

fn teleport_system(
    mut commands: Commands,
    mut room_data: ResMut<RoomMetadata>,
    mut teleporting_players: Query<(Entity, &Player, &WantsToTeleport, &mut Location)>,
    mut messages: Query<&mut Messages>,
) {
    for (teleporting_player_entity, teleporting_player, wants_to_teleport, mut location) in
        teleporting_players.iter_mut()
    {
        if let Some(present_players) = room_data.players_by_room.get(&location.room) {
            for present_player in present_players {
                if *present_player == teleporting_player_entity {
                    continue;
                }

                let message = format!(
                    "{} disappears in the blink of an eye.\r\n",
                    teleporting_player.name
                );
                queue_message!(commands, messages, *present_player, message);
            }
        }

        room_data.player_moved(
            teleporting_player_entity,
            location.room,
            wants_to_teleport.room,
        );

        location.room = wants_to_teleport.room;

        if let Some(present_players) = room_data.players_by_room.get(&location.room) {
            for present_player in present_players {
                if *present_player == teleporting_player_entity {
                    continue;
                }

                let message = format!(
                    "{} appears in a puff of smoke.\r\n",
                    teleporting_player.name
                );
                queue_message!(commands, messages, *present_player, message);
            }
        }

        commands
            .entity(teleporting_player_entity)
            .insert(WantsToLook {})
            .remove::<WantsToTeleport>();
    }
}
