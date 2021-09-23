use std::{
    collections::{HashMap, VecDeque},
    convert::TryFrom,
    error, fmt,
};

use bevy_ecs::prelude::*;
use bitflags::bitflags;
use thiserror::Error;

use crate::world::{
    scripting::ScriptHooks,
    types::{Attributes, Contents, Description, Health, Id, Location, Named},
};

#[derive(Bundle)]
pub struct PlayerBundle {
    pub id: Id,
    pub player: Player,
    pub flags: PlayerFlags,
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

#[derive(Debug, Clone)]
pub struct PlayerFlags {
    flags: Flags,
}

impl PlayerFlags {
    pub fn contains(&self, flags: Flags) -> bool {
        self.flags.contains(flags)
    }

    pub fn insert(&mut self, flags: Flags) {
        self.flags.insert(flags);
    }

    pub fn remove(&mut self, flags: Flags) {
        self.flags.remove(flags);
    }

    pub fn get_flags(&self) -> Flags {
        self.flags
    }
}

impl Default for PlayerFlags {
    fn default() -> Self {
        Self {
            flags: Flags::empty(),
        }
    }
}

impl From<i64> for PlayerFlags {
    fn from(value: i64) -> Self {
        PlayerFlags {
            flags: Flags::from_bits_truncate(value),
        }
    }
}

bitflags! {
    pub struct Flags: i64 {
        const IMMORTAL = 0b0001;
    }
}

impl TryFrom<&[String]> for Flags {
    type Error = FlagsParseError;

    fn try_from(strs: &[String]) -> Result<Self, Self::Error> {
        let mut flags = Flags::empty();

        for flag in strs {
            match flag.to_lowercase().as_str() {
                "immortal" => flags.insert(Flags::IMMORTAL),
                _ => {
                    return Err(FlagsParseError {
                        invalid_flag: flag.to_string(),
                    });
                }
            }
        }

        Ok(flags)
    }
}

#[derive(Debug, Error)]
#[error("Invalid player flag: {invalid_flag}. Valid flags: immortal.")]
pub struct FlagsParseError {
    invalid_flag: String,
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
