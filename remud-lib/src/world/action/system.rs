use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::world::{
    action::{into_action, Action},
    types::{
        player::{Messages, Player},
        room::Room,
        Configuration, Location, Named,
    },
};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Login {
    pub actor: Entity,
}

into_action!(Login);

pub fn login_system(
    mut action_reader: EventReader<Action>,
    player_query: Query<(&Named, &Location), With<Player>>,
    room_query: Query<&Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Login(Login { actor }) = action {
            let (name, room) = player_query
                .get(*actor)
                .map(|(named, location)| (named.as_str(), location.room()))
                .unwrap();

            let players = room_query
                .get(room)
                .unwrap()
                .players()
                .iter()
                .filter(|player| **player != *actor)
                .copied()
                .collect_vec();

            let message = format!("{} arrives.", name);

            for player in players {
                if let Ok(mut messages) = messages_query.get_mut(player) {
                    messages.queue(message.clone());
                }
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Shutdown {
    pub actor: Entity,
}

into_action!(Shutdown);

pub fn shutdown_system(mut action_reader: EventReader<Action>, mut config: ResMut<Configuration>) {
    for action in action_reader.iter() {
        if let Action::Shutdown(Shutdown { .. }) = action {
            config.shutdown = true
        }
    }
}
