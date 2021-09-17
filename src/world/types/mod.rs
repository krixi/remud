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

// Components
#[derive(Debug, Default)]
pub struct Contents {
    pub objects: Vec<Entity>,
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

impl Contents {
    pub fn remove(&mut self, object: Entity) {
        if let Some(index) = self.objects.iter().position(|o| *o == object) {
            self.objects.remove(index);
        }
    }
}

// Resources
pub struct Configuration {
    pub shutdown: bool,
    pub spawn_room: RoomId,
}
