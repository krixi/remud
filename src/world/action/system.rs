use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::world::{
    action::{self, queue_message, Action},
    types::{player::Player, room::Room, Configuration},
};

pub struct Login {}

impl Action for Login {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let (name, room) = world
            .get::<Player>(player)
            .map(|player| (player.name.as_str(), player.room))
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let present_players = world
            .get::<Room>(room)
            .ok_or(action::Error::MissingComponent(room, "Room"))?
            .players
            .iter()
            .filter(|present_player| **present_player != player)
            .copied()
            .collect_vec();

        let message = { format!("{} arrives.", name) };
        for present_player in present_players {
            queue_message(world, present_player, message.clone());
        }

        Ok(())
    }
}

pub struct Logout {}

impl Action for Logout {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let (name, room) = world
            .get::<Player>(player)
            .map(|player| (player.name.clone(), player.room))
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let present_players = world
            .get::<Room>(room)
            .ok_or(action::Error::MissingComponent(room, "Room"))?
            .players
            .iter()
            .filter(|present_player| **present_player != player)
            .copied()
            .collect_vec();

        let message = format!("{} leaves.", name);
        for present_player in present_players {
            queue_message(world, present_player, message.clone());
        }

        Ok(())
    }
}

pub struct Shutdown {}

impl Action for Shutdown {
    fn enact(&mut self, _player: Entity, world: &mut World) -> Result<(), action::Error> {
        let mut configuration = world.get_resource_mut::<Configuration>().unwrap();
        configuration.shutdown = true;
        Ok(())
    }
}
