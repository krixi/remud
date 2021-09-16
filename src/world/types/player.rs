use std::{
    collections::{HashMap, VecDeque},
    convert::TryFrom,
    error, fmt,
};

use bevy_ecs::prelude::*;

use crate::world::types::{self, Contents, Location, Named};

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

#[derive(Debug)]
pub struct IdParseError {}
impl fmt::Display for IdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Player IDs must be a non-negative integers.")
    }
}
impl error::Error for IdParseError {}

#[derive(Bundle)]
pub struct PlayerBundle {
    pub id: types::Id,
    pub player: Player,
    pub name: Named,
    pub location: Location,
    pub contents: Contents,
    pub messages: Messages,
}

pub struct Player {
    pub id: Id,
    pub name: String,
    pub room: Entity,
}

#[derive(Default)]
pub struct Messages {
    pub received_input: bool,
    pub queue: VecDeque<String>,
}

impl Messages {
    pub fn new_with(message: String) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back(message);

        Messages {
            received_input: false,
            queue,
        }
    }

    pub fn queue(&mut self, mut message: String) {
        message.push_str("\r\n");
        self.queue.push_back(message);
    }
}

#[derive(Default)]
pub struct Players {
    by_name: HashMap<String, Entity>,
}

impl Players {
    pub fn by_name(&self, name: &str) -> Option<Entity> {
        self.by_name.get(name).copied()
    }

    pub fn insert(&mut self, player: Entity, name: String) {
        self.by_name.insert(name, player);
    }

    pub fn remove(&mut self, name: &str) {
        self.by_name.remove(name);
    }
}
