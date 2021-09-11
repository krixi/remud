use bevy_ecs::prelude::Entity;

pub mod object;
pub mod player;
pub mod room;

// Components
#[derive(Default)]
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

// Resources
pub struct Configuration {
    pub shutdown: bool,
    pub spawn_room: room::Id,
}
