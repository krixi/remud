pub mod players;
pub mod room;

use bevy_ecs::prelude::*;

use crate::world::types::room::RoomId;

// Components
pub struct Location {
    pub room: Entity,
}

// Resources
pub struct Configuration {
    pub shutdown: bool,
    pub spawn_room: RoomId,
}
