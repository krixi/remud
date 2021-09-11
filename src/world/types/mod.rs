pub mod object;
pub mod player;
pub mod room;

// Resources
pub struct Configuration {
    pub shutdown: bool,
    pub spawn_room: room::Id,
}
