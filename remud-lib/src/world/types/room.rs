use std::{collections::HashMap, convert::TryFrom, error, fmt, str::FromStr};

use bevy_ecs::prelude::*;

use crate::world::types::{Contents, Description, Id, Named};

#[derive(Bundle)]
pub struct RoomBundle {
    pub id: Id,
    pub room: Room,
    pub name: Named,
    pub description: Description,
    pub regions: Regions,
    pub contents: Contents,
}

pub struct Room {
    id: RoomId,
    exits: HashMap<Direction, Entity>,
    players: Vec<Entity>,
}

impl Room {
    pub fn new(id: RoomId, exits: HashMap<Direction, Entity>, players: Vec<Entity>) -> Self {
        Room { id, exits, players }
    }

    pub fn id(&self) -> RoomId {
        self.id
    }

    pub fn exit(&self, direction: &Direction) -> Option<Entity> {
        self.exits.get(direction).copied()
    }

    pub fn exits(&self) -> &HashMap<Direction, Entity> {
        &self.exits
    }

    pub fn insert_exit(&mut self, direction: Direction, destination: Entity) {
        self.exits.insert(direction, destination);
    }

    pub fn remove_exit(&mut self, direction: &Direction) -> Option<Entity> {
        self.exits.remove(direction)
    }

    pub fn players(&self) -> &[Entity] {
        self.players.as_slice()
    }

    pub fn get_players(&self) -> Vec<Entity> {
        self.players.clone()
    }

    pub fn insert_player(&mut self, player: Entity) {
        self.players.push(player);
    }

    pub fn remove_player(&mut self, player: Entity) {
        if let Some(index) = self.players.iter().position(|p| *p == player) {
            self.players.remove(index);
        }
    }
}

impl From<RoomId> for Room {
    fn from(id: RoomId) -> Self {
        Room {
            id,
            exits: HashMap::new(),
            players: Vec::new(),
        }
    }
}

#[derive(Default)]
pub struct Regions {
    list: Vec<String>,
}

impl Regions {
    pub fn new(list: Vec<String>) -> Self {
        Regions { list }
    }

    pub fn add(&mut self, region: String) {
        self.list.push(region);
    }

    #[allow(clippy::ptr_arg)]
    pub fn contains(&self, region: &String) -> bool {
        self.list.contains(region)
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn get_list(&self) -> Vec<String> {
        self.list.clone()
    }

    pub fn set_list(&mut self, list: Vec<String>) {
        self.list = list;
    }

    pub fn remove(&mut self, region: &str) -> Option<String> {
        if let Some(position) = self.list.iter().position(|r| r.as_str() == region) {
            return Some(self.list.remove(position));
        }

        None
    }
}

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

impl From<RoomId> for Id {
    fn from(id: RoomId) -> Self {
        Id::Room(id)
    }
}

#[derive(Debug)]
pub struct RoomIdParseError {}
impl fmt::Display for RoomIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Room IDs must be a non-negative integers.")
    }
}
impl error::Error for RoomIdParseError {}

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
    pub fn as_from_str(&self) -> &str {
        match self {
            Direction::North => "from the north",
            Direction::East => "from the east",
            Direction::South => "from the south",
            Direction::West => "from the west",
            Direction::Up => "from above",
            Direction::Down => "from below",
        }
    }

    pub fn as_to_str(&self) -> &str {
        match self {
            Direction::North => "to the north",
            Direction::East => "to the east",
            Direction::South => "to the south",
            Direction::West => "to the west",
            Direction::Up => "up",
            Direction::Down => "down",
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Direction::North => "north",
            Direction::East => "east",
            Direction::South => "south",
            Direction::West => "west",
            Direction::Up => "up",
            Direction::Down => "down",
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

// Resource used as index for room/player lookups
pub struct Rooms {
    by_id: HashMap<RoomId, Entity>,
    highest_id: i64,
}

impl Rooms {
    pub fn new(by_id: HashMap<RoomId, Entity>, highest_id: i64) -> Self {
        Rooms { by_id, highest_id }
    }

    pub fn insert(&mut self, id: RoomId, room: Entity) {
        self.by_id.insert(id, room);
    }

    pub fn by_id(&self, id: RoomId) -> Option<Entity> {
        self.by_id.get(&id).copied()
    }

    pub fn remove(&mut self, id: RoomId) {
        self.by_id.remove(&id);
    }

    pub fn next_id(&mut self) -> RoomId {
        self.highest_id += 1;
        RoomId(self.highest_id)
    }
}
