use std::{
    collections::{HashMap, VecDeque},
    convert::TryFrom,
    error, fmt,
};

use bevy_ecs::prelude::*;

use crate::world::{
    scripting::ScriptHooks,
    types::{Attributes, Contents, Description, Health, Id, Location, Named},
};

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
    pub description: Description,
    pub location: Location,
    pub contents: Contents,
    pub messages: Messages,
    pub attributes: Attributes,
    pub health: Health,
    pub hooks: ScriptHooks,
}

pub struct Player {
    id: PlayerId,
}

impl Player {
    pub fn id(&self) -> PlayerId {
        self.id
    }
}

impl From<PlayerId> for Player {
    fn from(id: PlayerId) -> Self {
        Player { id }
    }
}

#[derive(Default)]
pub struct Messages {
    received_input: bool,
    queue: VecDeque<String>,
}

impl Messages {
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn queue(&mut self, mut message: String) {
        message.push_str("\r\n");
        self.queue.push_back(message);
    }

    pub fn get_queue(&mut self) -> VecDeque<String> {
        let mut queue = VecDeque::new();
        std::mem::swap(&mut queue, &mut self.queue);

        if !self.received_input {
            queue.push_front("\r\n".to_string());
        }

        self.received_input = false;

        queue
    }

    pub fn set_received_input(&mut self) {
        self.received_input = true;
    }
}

#[derive(Default)]
pub struct Players {
    by_name: HashMap<String, Entity>,
    id_by_name: HashMap<String, PlayerId>,
}

impl Players {
    pub fn by_name(&self, name: &str) -> Option<Entity> {
        self.by_name.get(name).copied()
    }

    pub fn insert(&mut self, player: Entity, name: String, id: PlayerId) {
        self.by_name.insert(name.clone(), player);
        self.id_by_name.insert(name, id);
    }

    pub fn remove(&mut self, name: &str) {
        self.by_name.remove(name);
        self.id_by_name.remove(name);
    }
}
