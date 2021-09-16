use bevy_app::{EventReader, Events};
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::world::{
    action::{self, Action, ActionEvent},
    types::{
        player::{Messages, Player},
        room::Room,
        Configuration, Location, Named,
    },
};

pub struct Login {}

impl Action for Login {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::Login { entity });

        Ok(())
    }
}

pub fn login_system(
    mut events: EventReader<ActionEvent>,
    player_query: Query<(&Named, &Location), With<Player>>,
    room_query: Query<&Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Login { entity } = event {
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

pub struct Logout {}

impl Action for Logout {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::Logout { entity });
        Ok(())
    }
}

pub fn logout_system(
    mut events: EventReader<ActionEvent>,
    player_query: Query<(&Named, &Location), With<Player>>,
    room_query: Query<&Room>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Login { entity } = event {
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

pub struct Shutdown {}

impl Action for Shutdown {
    fn enact(&mut self, _player: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::Shutdown {});
        Ok(())
    }
}

pub fn shutdown_system(mut events: EventReader<ActionEvent>, mut config: ResMut<Configuration>) {
    for event in events.iter() {
        if let ActionEvent::Shutdown {} = event {
            config.shutdown = true
        }
    }
}
