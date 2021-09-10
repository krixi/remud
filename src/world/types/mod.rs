pub mod object;
pub mod player;
pub mod room;

use crate::world::types::room::RoomId;

// Resources
pub struct Configuration {
    pub shutdown: bool,
    pub spawn_room: RoomId,
}
