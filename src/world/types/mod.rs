use std::{fmt, ops::Index};

use bevy_ecs::prelude::Entity;

use crate::world::types::{
    object::{ObjectId, PrototypeId},
    player::PlayerId,
    room::RoomId,
};

pub mod object;
pub mod player;
pub mod room;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Id {
    Player(PlayerId),
    Prototype(PrototypeId),
    Object(ObjectId),
    Room(RoomId),
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Id::Player(id) => write!(f, "{}", id),
            Id::Prototype(id) => write!(f, "{}", id),
            Id::Object(id) => write!(f, "{}", id),
            Id::Room(id) => write!(f, "{}", id),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ActionTarget {
    CurrentRoom,
    Object(ObjectId),
    PlayerSelf,
    Player(String),
    Prototype(PrototypeId),
}

#[derive(Debug, Clone)]
pub struct Named {
    name: String,
}

impl Named {
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn as_str(&self) -> &str {
        self.name.as_str()
    }

    pub fn escaped(&self) -> String {
        self.name.replace("|", "||")
    }
}

impl From<String> for Named {
    fn from(name: String) -> Self {
        Named { name }
    }
}

impl fmt::Display for Named {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone)]
pub struct Description {
    text: String,
}

impl Description {
    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }

    pub fn escaped(&self) -> String {
        self.text.replace("|", "||")
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl From<String> for Description {
    fn from(text: String) -> Self {
        Description { text }
    }
}

#[derive(Debug)]
pub struct Location {
    room: Entity,
}

impl Location {
    pub fn set_room(&mut self, room: Entity) {
        self.room = room;
    }

    pub fn room(&self) -> Entity {
        self.room
    }
}

impl From<Entity> for Location {
    fn from(room: Entity) -> Self {
        Location { room }
    }
}

// Components
#[derive(Debug, Default)]
pub struct Contents {
    objects: Vec<Entity>,
}

impl Contents {
    pub fn insert(&mut self, object: Entity) {
        self.objects.push(object);
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    pub fn find(&self, mut predicate: impl FnMut(Entity) -> bool) -> Option<Entity> {
        self.objects
            .iter()
            .find(|entity| predicate(**entity))
            .copied()
    }

    pub fn remove(&mut self, object: Entity) -> bool {
        if let Some(index) = self.objects.iter().position(|o| *o == object) {
            self.objects.remove(index);
            true
        } else {
            false
        }
    }

    pub fn objects(&self) -> &[Entity] {
        self.objects.as_slice()
    }

    pub fn get_objects(&self) -> Vec<Entity> {
        self.objects.clone()
    }
}

impl Index<usize> for Contents {
    type Output = Entity;

    fn index(&self, index: usize) -> &Self::Output {
        &self.objects[index]
    }
}

// Resources
pub struct Configuration {
    pub shutdown: bool,
    pub spawn_room: RoomId,
}

#[derive(Debug, Clone, Copy)]
pub struct Attributes {
    pub constitution: f32,
    pub dexterity: f32,
    pub intellect: f32,
    pub strength: f32,
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes {
            constitution: 10.0,
            dexterity: 10.0,
            intellect: 10.0,
            strength: 10.0,
        }
    }
}

#[derive(Debug)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(attr: &Attributes) -> Self {
        Health {
            current: attr.constitution * 5.0,
            max: attr.constitution * 5.0,
        }
    }
}
