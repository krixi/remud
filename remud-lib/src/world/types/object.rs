use std::{collections::HashMap, convert::TryFrom, error, fmt, str::FromStr};

use bevy_ecs::prelude::*;
use bitflags::bitflags;
use strum::EnumString;
use thiserror::Error;

use crate::{
    text::sorted_word_list,
    world::types::{Description, Id, Named},
};

#[derive(Debug, Bundle)]
pub struct PrototypeBundle {
    pub prototype: Prototype,
    pub name: Named,
    pub description: Description,
    pub flags: ObjectFlags,
    pub keywords: Keywords,
}

#[derive(Debug, Bundle)]
pub struct ObjectBundle {
    pub id: Id,
    pub object: Object,
    pub name: Named,
    pub description: Description,
    pub flags: ObjectFlags,
    pub keywords: Keywords,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ObjectOrPrototype {
    Object(ObjectId),
    Prototype(PrototypeId),
}

impl fmt::Display for ObjectOrPrototype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectOrPrototype::Object(id) => write!(f, "object {}", id),
            ObjectOrPrototype::Prototype(id) => write!(f, "prototype {}", id),
        }
    }
}

#[derive(Debug)]
pub struct Prototype {
    id: PrototypeId,
}

impl Prototype {
    pub fn id(&self) -> PrototypeId {
        self.id
    }
}

impl From<PrototypeId> for Prototype {
    fn from(id: PrototypeId) -> Self {
        Prototype { id }
    }
}

#[derive(Debug)]
pub struct Object {
    id: ObjectId,
    prototype: Entity,
    inherit_scripts: bool,
}

impl Object {
    pub fn new(id: ObjectId, prototype: Entity, inherit_scripts: bool) -> Self {
        Object {
            id,
            prototype,
            inherit_scripts,
        }
    }

    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn prototype(&self) -> Entity {
        self.prototype
    }

    pub fn inherit_scripts(&self) -> bool {
        self.inherit_scripts
    }

    pub fn set_inherit_scripts(&mut self, inherit: bool) {
        self.inherit_scripts = inherit;
    }
}

#[derive(Debug, Clone)]
pub struct Keywords {
    list: Vec<String>,
}

impl Keywords {
    pub fn get_list(&self) -> Vec<String> {
        self.list.clone()
    }

    pub fn remove(&mut self, list: &[String]) {
        self.list.retain(|k| !list.contains(k));
    }

    pub fn add(&mut self, list: Vec<String>) {
        self.list.extend(list.into_iter());
        self.list.sort_unstable();
        self.list.dedup();
    }

    pub fn set_list(&mut self, list: Vec<String>) {
        self.list = list
    }

    pub fn contains_all(&self, words: &[String]) -> bool {
        words.iter().all(|word| self.list.contains(word))
    }

    pub fn as_word_list(&self) -> String {
        sorted_word_list(self.list.clone())
    }
}

impl From<Vec<String>> for Keywords {
    fn from(list: Vec<String>) -> Self {
        Keywords { list }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectFlags {
    flags: Flags,
}

impl ObjectFlags {
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

impl Default for ObjectFlags {
    fn default() -> Self {
        Self {
            flags: Flags::empty(),
        }
    }
}

impl From<i64> for ObjectFlags {
    fn from(value: i64) -> Self {
        ObjectFlags {
            flags: Flags::from_bits_truncate(value),
        }
    }
}

#[derive(Debug)]
pub struct Container {
    entity: Entity,
}

impl Container {
    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn set_entity(&mut self, entity: Entity) {
        self.entity = entity;
    }
}

impl From<Entity> for Container {
    fn from(entity: Entity) -> Self {
        Container { entity }
    }
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
    #[strum(serialize = "scripts")]
    Scripts,
}

bitflags! {
    pub struct Flags: i64 {
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

    pub fn remove_entity(&mut self, entity: Entity) {
        if let Some((id, _)) = self.by_id.iter().find(|(_, object)| **object == entity) {
            let id = *id;
            self.by_id.remove(&id);
        }
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

    pub fn by_id(&self, id: PrototypeId) -> Option<Entity> {
        self.by_id.get(&id).copied()
    }

    pub fn next_id(&mut self) -> PrototypeId {
        self.highest_id += 1;
        PrototypeId(self.highest_id)
    }
}
