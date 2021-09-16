use bevy_app::{EventReader, Events};
use bevy_ecs::prelude::*;

use crate::{
    text::Tokenizer,
    world::{
        action::{self, Action, ActionEvent, DynAction},
        types::{
            object::Object,
            player::{Messages, Players},
            room::Room,
            Contents, Location, Named,
        },
    },
};

// Valid shapes:
// player <name> info - displays information about the player
pub fn parse(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(name) = tokenizer.next() {
        if let Some(token) = tokenizer.next() {
            match token {
                "info" => Ok(Box::new(Info {
                    name: name.to_string(),
                })),
                _ => Err("Enter a valid player subcommand: info.".to_string()),
            }
        } else {
            Err("Enter a player subcommand: info.".to_string())
        }
    } else {
        Err("Enter a player name.".to_string())
    }
}

struct Info {
    name: String,
}

impl Action for Info {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::PlayerInfo {
                entity,
                name: self.name.clone(),
            });

        Ok(())
    }
}

pub fn player_info_system(
    mut events: EventReader<ActionEvent>,
    players: Res<Players>,
    player_query: Query<(&Contents, &Location)>,
    room_query: Query<&Room>,
    object_query: Query<(&Object, &Named)>,
    mut message_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::PlayerInfo { entity, name } = event {
            let player = if let Some(entity) = players.by_name(name) {
                entity
            } else {
                if let Ok(mut messages) = message_query.get_mut(*entity) {
                    messages.queue(format!("Player '{}' not found.", name))
                }
                continue;
            };

            let (contents, location) = player_query.get(player).unwrap();
            let room = room_query.get(location.room).unwrap();

            let mut message = format!("Player {}", name);

            message.push_str("\r\n  room: ");
            message.push_str(room.id.to_string().as_str());

            message.push_str("\r\n  inventory:");
            contents
                .objects
                .iter()
                .filter_map(|object| {
                    object_query
                        .get(*object)
                        .map(|(object, named)| (object.id, named.name.as_str()))
                        .ok()
                })
                .for_each(|(id, name)| {
                    message.push_str(format!("\r\n    object {}: {}", id, name).as_str())
                });

            if let Ok(mut messages) = message_query.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}
