use std::{collections::HashMap, convert::TryFrom, error, fmt, str::FromStr};

use bevy_ecs::prelude::*;
use bitflags::bitflags;
use thiserror::Error;

use crate::world::types::{self, Container, Description, Keywords, Named};

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
        write!(f, "Object IDs must be a non-negative integers.")
    }
}
impl error::Error for IdParseError {}

bitflags! {
    pub struct Flags: i64{
        const FIXED = 0b0001;
        const SUBTLE = 0b0010;
    }
}

impl TryFrom<&[String]> for Flags {
    type Error = FlagsParseError;

    fn try_from(strs: &[String]) -> Result<Self, Self::Error> {
        let mut flags = Flags::empty();

        for flag in strs {
            match flag.to_lowercase().as_str() {
                "fixed" => flags.insert(Flags::FIXED),
                "subtle" => flags.insert(Flags::SUBTLE),
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
#[error("Invalid object flag: {invalid_flag}. Valid flags: fixed, subtle.")]
pub struct FlagsParseError {
    invalid_flag: String,
}

#[derive(Debug, Bundle)]
pub struct ObjectBundle {
    pub object: Object,
    pub id: types::Id,
    pub flags: types::Flags,
    pub container: Container,
    pub name: Named,
    pub description: Description,
    pub keywords: Keywords,
}

#[derive(Debug)]
pub struct Object {
    pub id: Id,
    pub flags: Flags,
    pub container: Entity,
    pub keywords: Vec<String>,
    pub short: String,
    pub long: String,
}

impl Object {
    pub fn new(
        id: Id,
        flags: Flags,
        container: Entity,
        keywords: Vec<String>,
        short: String,
        long: String,
    ) -> Self {
        Object {
            id,
            flags,
            container,
            keywords,
            short,
            long,
        }
    }
}

pub struct Objects {
    by_id: HashMap<Id, Entity>,
    highest_id: i64,
}

impl Objects {
    pub fn new(highest_id: i64, by_id: HashMap<Id, Entity>) -> Self {
        Objects { by_id, highest_id }
    }

    pub fn insert(&mut self, id: Id, entity: Entity) {
        self.by_id.insert(id, entity);
    }

    pub fn by_id(&self, id: Id) -> Option<Entity> {
        self.by_id.get(&id).copied()
    }

    pub fn next_id(&mut self) -> Id {
        self.highest_id += 1;
        Id(self.highest_id)
    }
}
