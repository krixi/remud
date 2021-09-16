use bevy_ecs::prelude::Entity;

pub mod object;
pub mod player;
pub mod room;

// Components
#[derive(Default)]
pub struct Contents {
    pub objects: Vec<Entity>,
}

pub struct Named {
    pub name: String,
}

pub struct Location {
    pub room: Entity,
}

// pub struct Container {
//     pub entity: Entity,
// }

// pub struct Description {
//     pub text: String,
// }

// pub struct Keywords {
//     pub list: Vec<String>,
// }

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
