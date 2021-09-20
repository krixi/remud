use std::{
    collections::{HashMap, VecDeque},
    convert::TryFrom,
    error, fmt,
};

use bevy_ecs::prelude::*;

use crate::world::types::{Attributes, Contents, Health, Id, Location, Named};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct PlayerId(i64);

impl TryFrom<i64> for PlayerId {
    type Error = PlayerIdParseError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(PlayerId(value))
        } else {
            Err(PlayerIdParseError {})
        }
    }
}

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct PlayerIdParseError {}
impl fmt::Display for PlayerIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Player IDs must be a non-negative integers.")
    }
}
impl error::Error for PlayerIdParseError {}

#[derive(Bundle)]
pub struct PlayerBundle {
    pub id: Id,
    pub player: Player,
    pub name: Named,
    pub location: Location,
    pub contents: Contents,
    pub messages: Messages,
    pub attributes: Attributes,
    pub health: Health,
}

pub struct Player {
    pub id: PlayerId,
}

#[derive(Default)]
pub struct Messages {
    pub received_input: bool,
    pub queue: VecDeque<String>,
}

impl Messages {
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
