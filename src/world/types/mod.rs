use std::fmt;

use bevy_ecs::prelude::Entity;

use crate::world::types::{
    object::{ObjectFlags, ObjectId},
    player::PlayerId,
    room::RoomId,
};

pub mod object;
pub mod player;
pub mod room;

#[derive(Debug, Clone, Copy)]
pub enum Id {
    Player(PlayerId),
    Object(ObjectId),
    Room(RoomId),
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Id::Player(id) => write!(f, "{}", id),
            Id::Object(id) => write!(f, "{}", id),
            Id::Room(id) => write!(f, "{}", id),
        }
    }
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

#[derive(Debug)]
pub struct Named {
    pub name: String,
}

#[derive(Debug)]
pub struct Location {
    pub room: Entity,
}

#[derive(Debug)]
pub struct Container {
    pub entity: Entity,
}

#[derive(Debug)]
pub struct Description {
    pub text: String,
}

#[derive(Debug)]
pub struct Keywords {
    pub list: Vec<String>,
}

#[derive(Debug)]
pub struct Flags {
    pub flags: ObjectFlags,
}

// Resources
pub struct Configuration {
    pub shutdown: bool,
    pub spawn_room: RoomId,
}
