use bevy_ecs::prelude::Entity;

pub mod object;
pub mod player;
pub mod room;

#[derive(Debug, Clone, Copy)]
pub enum Id {
    Player(player::Id),
    Object(object::Id),
    Room(room::Id),
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
    pub flags: object::Flags,
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
    pub spawn_room: room::Id,
}
