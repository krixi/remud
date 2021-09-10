use bevy_ecs::prelude::*;

use crate::world::{
    action::Action,
    types::{player::Player, Configuration, Location},
    LoggedIn, LoggedOut,
};

pub struct Login {}

impl Action for Login {
    fn enact(&mut self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(LoggedIn {});
    }
}

pub struct Logout {}

impl Action for Logout {
    fn enact(&mut self, player: Entity, world: &mut World) {
        if let Some(room) = world.get::<Location>(player).map(|location| location.room) {
            if let Some(name) = world
                .get::<Player>(player)
                .map(|player| player.name.clone())
            {
                world.entity_mut(room).insert(LoggedOut { name });
            }
        }
    }
}

pub struct Shutdown {}

impl Action for Shutdown {
    fn enact(&mut self, _player: Entity, world: &mut World) {
        let mut configuration = world.get_resource_mut::<Configuration>().unwrap();
        configuration.shutdown = true;
    }
}
