use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    into_action,
    world::{
        action::Action,
        types::{
            player::{Messages, Player},
            room::Room,
            Configuration, Location, Named,
        },
    },
};

#[derive(Debug, Clone)]
pub struct Login {
    pub entity: Entity,
}

into_action!(Login);

pub fn login_system(
    mut action_reader: EventReader<Action>,
    player_query: Query<(&Named, &Location), With<Player>>,
    room_query: Query<&Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Login(Login { entity }) = action {
            let (name, room) = player_query
                .get(*entity)
                .map(|(named, location)| (named.name.as_str(), location.room))
                .unwrap();

            let players = room_query
                .get(room)
                .unwrap()
                .players
                .iter()
                .filter(|player| **player != *entity)
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

#[derive(Debug, Clone)]
pub struct Logout {
    pub entity: Entity,
}

into_action!(Logout);

pub fn logout_system(
    mut action_reader: EventReader<Action>,
    player_query: Query<(&Named, &Location), With<Player>>,
    room_query: Query<&Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Logout(Logout { entity }) = action {
            let (name, room) = player_query
                .get(*entity)
                .map(|(named, location)| (named.name.as_str(), location.room))
                .unwrap();

            let players = room_query
                .get(room)
                .unwrap()
                .players
                .iter()
                .filter(|player| **player != *entity)
                .copied()
                .collect_vec();

            let message = format!("{} leaves.", name);

            for player in players {
                if let Ok(mut messages) = messages_query.get_mut(player) {
                    messages.queue(message.clone());
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Shutdown {
    pub entity: Entity,
}

into_action!(Shutdown);

pub fn shutdown_system(mut action_reader: EventReader<Action>, mut config: ResMut<Configuration>) {
    for action in action_reader.iter() {
        if let Action::Shutdown(Shutdown { .. }) = action {
            config.shutdown = true
        }
    }
}
