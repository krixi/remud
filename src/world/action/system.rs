use bevy_app::EventReader;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    event_from_action,
    world::{
        action::ActionEvent,
        types::{
            player::{Messages, Player},
            room::Room,
            Configuration, Location, Named,
        },
    },
};

pub struct Login {
    pub entity: Entity,
}

event_from_action!(Login);

pub fn login_system(
    mut events: EventReader<ActionEvent>,
    player_query: Query<(&Named, &Location), With<Player>>,
    room_query: Query<&Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Login(Login { entity }) = event {
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

pub struct Logout {
    pub entity: Entity,
}

event_from_action!(Logout);

pub fn logout_system(
    mut events: EventReader<ActionEvent>,
    player_query: Query<(&Named, &Location), With<Player>>,
    room_query: Query<&Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Logout(Logout { entity }) = event {
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

pub struct Shutdown {
    pub entity: Entity,
}

event_from_action!(Shutdown);

pub fn shutdown_system(mut events: EventReader<ActionEvent>, mut config: ResMut<Configuration>) {
    for event in events.iter() {
        if let ActionEvent::Shutdown(Shutdown { .. }) = event {
            config.shutdown = true
        }
    }
}
