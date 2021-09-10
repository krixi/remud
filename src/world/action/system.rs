use anyhow::bail;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::world::{
    action::{queue_message, Action},
    types::{
        player::{Player, Players},
        Configuration,
    },
};

pub struct Login {}

impl Action for Login {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let (name, room) = match world.get::<Player>(player) {
            Some(player) => (player.name.as_str(), player.room),
            None => bail!("Player {:?} has no Player."),
        };

        let present_players = world
            .get_resource::<Players>()
            .unwrap()
            .by_room(room)
            .filter(|present_player| *present_player != player)
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
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let (name, room) = match world.get::<Player>(player) {
            Some(player) => (player.name.clone(), player.room),
            None => bail!("Player {:?} has no Player.", player),
        };

        let present_players = world
            .get_resource::<Players>()
            .unwrap()
            .by_room(room)
            .filter(|present_player| *present_player != player)
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
    fn enact(&mut self, _player: Entity, world: &mut World) -> anyhow::Result<()> {
        let mut configuration = world.get_resource_mut::<Configuration>().unwrap();
        configuration.shutdown = true;
        Ok(())
    }
}