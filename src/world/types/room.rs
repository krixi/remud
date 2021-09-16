use std::{collections::HashMap, convert::TryFrom, error, fmt, str::FromStr};

use bevy_ecs::prelude::*;

use crate::world::types::{Contents, Description};

#[derive(Bundle)]
pub struct RoomBundle {
    pub room: Room,
    pub description: Description,
    pub contents: Contents,
}

pub struct Room {
    pub id: Id,
    pub description: String,
    pub exits: HashMap<Direction, Entity>,
    pub players: Vec<Entity>,
}

impl Room {
    pub fn new(id: Id, description: String) -> Self {
        Room {
            id,
            description,
            exits: HashMap::new(),
            players: Vec::new(),
        }
    }

    pub fn remove_player(&mut self, player: Entity) {
        if let Some(index) = self.players.iter().position(|p| *p == player) {
            self.players.remove(index);
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

impl TryFrom<i64> for Id {
    type Error = IdParseError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(Id(value))
        } else {
            Err(IdParseError {})
        }
    }
}

impl FromStr for Id {
    type Err = IdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let int = s.parse::<i64>().map_err(|_| IdParseError {})?;
        Id::try_from(int)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug)]
pub struct IdParseError {}
impl fmt::Display for IdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Room IDs must be a non-negative integers.")
    }
}
impl error::Error for IdParseError {}

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
    by_id: HashMap<Id, Entity>,
    highest_id: i64,
}

impl Rooms {
    pub fn new(by_id: HashMap<Id, Entity>, highest_id: i64) -> Self {
        Rooms { by_id, highest_id }
    }

    pub fn insert(&mut self, id: Id, room: Entity) {
        self.by_id.insert(id, room);
    }

    pub fn by_id(&self, id: Id) -> Option<Entity> {
        self.by_id.get(&id).copied()
    }

    pub fn remove(&mut self, id: Id) {
        self.by_id.remove(&id);
    }

    pub fn next_id(&mut self) -> Id {
        self.highest_id += 1;
        Id(self.highest_id)
    }
}
