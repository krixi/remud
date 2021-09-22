use std::fmt;

use bevy_ecs::prelude::Entity;

use crate::world::types::{
    object::{ObjectFlags, ObjectId, PrototypeId},
    player::PlayerId,
    room::RoomId,
};

pub mod object;
pub mod player;
pub mod room;

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone)]
pub enum ActionTarget {
    PlayerSelf,
    Prototype(PrototypeId),
    Object(ObjectId),
    CurrentRoom,
}

// Components
#[derive(Debug, Default)]
pub struct Contents {
    pub objects: Vec<Entity>,
}

impl Contents {
    pub fn remove(&mut self, object: Entity) {
        if let Some(index) = self.objects.iter().position(|o| *o == object) {
            self.objects.remove(index);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Named {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Description {
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Keywords {
    pub list: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Flags {
    pub flags: ObjectFlags,
}

#[derive(Debug)]
pub struct Location {
    pub room: Entity,
}

#[derive(Debug)]
pub struct Container {
    pub entity: Entity,
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
