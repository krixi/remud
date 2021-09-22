use std::{collections::HashMap, convert::TryFrom, error, fmt, str::FromStr};

use bevy_ecs::prelude::*;
use bitflags::bitflags;
use strum::EnumString;
use thiserror::Error;

use crate::world::{
    scripting::ScriptHooks,
    types::{self, Description, Id, Keywords, Named},
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct PrototypeId(i64);

impl TryFrom<i64> for PrototypeId {
    type Error = PrototypeIdParseError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(PrototypeId(value))
        } else {
            Err(PrototypeIdParseError {})
        }
    }
}

impl FromStr for PrototypeId {
    type Err = PrototypeIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let int = s.parse::<i64>().map_err(|_| PrototypeIdParseError {})?;
        PrototypeId::try_from(int)
    }
}

impl From<PrototypeId> for Id {
    fn from(prototype: PrototypeId) -> Self {
        Id::Prototype(prototype)
    }
}

impl fmt::Display for PrototypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug)]
pub struct PrototypeIdParseError {}
impl fmt::Display for PrototypeIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Prototype IDs must be a non-negative integers.")
    }
}
impl error::Error for PrototypeIdParseError {}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct ObjectId(i64);

impl TryFrom<i64> for ObjectId {
    type Error = ObjectIdParseError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(ObjectId(value))
        } else {
            Err(ObjectIdParseError {})
        }
    }
}

impl FromStr for ObjectId {
    type Err = ObjectIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let int = s.parse::<i64>().map_err(|_| ObjectIdParseError {})?;
        ObjectId::try_from(int)
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl From<ObjectId> for Id {
    fn from(id: ObjectId) -> Self {
        Id::Object(id)
    }
}

#[derive(Debug)]
pub struct ObjectIdParseError {}
impl fmt::Display for ObjectIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Object IDs must be a non-negative integers.")
    }
}
impl error::Error for ObjectIdParseError {}

bitflags! {
    pub struct ObjectFlags: i64{
        const FIXED = 0b0001;
        const SUBTLE = 0b0010;
    }
}

impl TryFrom<&[String]> for ObjectFlags {
    type Error = FlagsParseError;

    fn try_from(strs: &[String]) -> Result<Self, Self::Error> {
        let mut flags = ObjectFlags::empty();

        for flag in strs {
            match flag.to_lowercase().as_str() {
                "fixed" => flags.insert(ObjectFlags::FIXED),
                "subtle" => flags.insert(ObjectFlags::SUBTLE),
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
pub struct PrototypeBundle {
    pub prototype: Prototype,
    pub name: Named,
    pub description: Description,
    pub flags: types::Flags,
    pub keywords: Keywords,
    pub hooks: ScriptHooks,
}

#[derive(Debug, Bundle)]
pub struct ObjectBundle {
    pub id: Id,
    pub object: Object,
    pub name: Named,
    pub description: Description,
    pub flags: types::Flags,
    pub keywords: Keywords,
    pub hooks: ScriptHooks,
}

#[derive(Debug)]
pub struct Prototype {
    pub id: PrototypeId,
}

#[derive(Debug)]
pub struct Object {
    pub id: ObjectId,
    pub prototype: Entity,
    pub inherit_scripts: bool,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, EnumString)]
pub enum InheritableFields {
    #[strum(serialize = "flags")]
    Flags,
    #[strum(serialize = "name")]
    Name,
    #[strum(serialize = "desc")]
    Description,
    #[strum(serialize = "keywords")]
    Keywords,
    #[strum(serialize = "hooks")]
    Hooks,
}

pub struct Objects {
    by_id: HashMap<ObjectId, Entity>,
    highest_id: i64,
}

impl Objects {
    pub fn new(highest_id: i64, by_id: HashMap<ObjectId, Entity>) -> Self {
        Objects { by_id, highest_id }
    }

    pub fn insert(&mut self, id: ObjectId, entity: Entity) {
        self.by_id.insert(id, entity);
    }

    pub fn remove(&mut self, id: ObjectId) {
        self.by_id.remove(&id);
    }

    pub fn by_id(&self, id: ObjectId) -> Option<Entity> {
        self.by_id.get(&id).copied()
    }

    pub fn next_id(&mut self) -> ObjectId {
        self.highest_id += 1;
        ObjectId(self.highest_id)
    }
}

pub struct Prototypes {
    by_id: HashMap<PrototypeId, Entity>,
    highest_id: i64,
}

impl Prototypes {
    pub fn new(highest_id: i64, by_id: HashMap<PrototypeId, Entity>) -> Self {
        Prototypes { by_id, highest_id }
    }

    pub fn insert(&mut self, id: PrototypeId, entity: Entity) {
        self.by_id.insert(id, entity);
    }

    pub fn remove(&mut self, id: PrototypeId) {
        self.by_id.remove(&id);
    }

    pub fn by_id(&self, id: PrototypeId) -> Option<Entity> {
        self.by_id.get(&id).copied()
    }

    pub fn next_id(&mut self) -> PrototypeId {
        self.highest_id += 1;
        PrototypeId(self.highest_id)
    }
}
