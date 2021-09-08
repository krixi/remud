use std::{
    collections::{hash_set, HashMap, HashSet},
    convert::TryFrom,
    error, fmt,
    str::FromStr,
};

use bevy_ecs::prelude::*;

pub struct Room {
    pub id: RoomId,
    pub description: String,
    pub exits: HashMap<Direction, Entity>,
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
            Direction::Up => "above",
            Direction::Down => "below",
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
    rooms_by_id: HashMap<RoomId, Entity>,
    players_by_room: HashMap<Entity, HashSet<Entity>>,
    highest_id: i64,
}

impl Rooms {
    pub fn new(rooms_by_id: HashMap<RoomId, Entity>, highest_id: i64) -> Self {
        Rooms {
            rooms_by_id,
            players_by_room: HashMap::new(),
            highest_id,
        }
    }

    pub fn players_in(&self, room: Entity) -> PlayersByRoom {
        if let Some(players) = self.players_by_room.get(&room) {
            PlayersByRoom {
                iter: Some(players.iter()),
            }
        } else {
            PlayersByRoom { iter: None }
        }
    }

    pub fn add_player(&mut self, player: Entity, room: Entity) {
        self.players_by_room.entry(room).or_default().insert(player);
    }

    pub fn remove_player(&mut self, player: Entity, room: Entity) {
        self.players_by_room.entry(room).and_modify(|players| {
            players.remove(&player);
        });
    }

    pub fn move_player(&mut self, player: Entity, from: Entity, to: Entity) {
        if let Some(list) = self.players_by_room.get_mut(&from) {
            list.remove(&player);
        }

        self.players_by_room.entry(to).or_default().insert(player);
    }

    pub fn add_room(&mut self, id: RoomId, room: Entity) {
        self.rooms_by_id.insert(id, room);
    }

    pub fn get_room(&self, id: RoomId) -> Option<Entity> {
        self.rooms_by_id.get(&id).copied()
    }

    pub fn next_id(&mut self) -> RoomId {
        self.highest_id += 1;
        RoomId(self.highest_id)
    }
}

pub struct PlayersByRoom<'a> {
    iter: Option<hash_set::Iter<'a, Entity>>,
}

impl<'a> Iterator for PlayersByRoom<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = &mut self.iter {
            iter.next().copied()
        } else {
            None
        }
    }
}
