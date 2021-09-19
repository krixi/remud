use bevy_app::EventReader;
use bevy_ecs::prelude::*;

use crate::{
    into_action,
    text::Tokenizer,
    world::{
        action::Action,
        types::{
            player::{Messages, Players},
            room::Room,
            Location, Named,
        },
    },
};

pub fn parse_me(player: Entity, tokenizer: Tokenizer) -> Result<Action, String> {
    if tokenizer.rest().is_empty() {
        Err("Do what?".to_string())
    } else {
        Ok(Action::from(Emote {
            entity: player,
            emote: tokenizer.rest().to_string(),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct Emote {
    pub entity: Entity,
    pub emote: String,
}

into_action!(Emote);

pub fn emote_system(
    mut action_reader: EventReader<Action>,
    emoting_query: Query<(&Named, &Location)>,
    mut present_query: Query<&mut Messages>,
    room_query: Query<&Room>,
) {
    for action in action_reader.iter() {
        if let Action::Emote(Emote { entity, emote }) = action {
            let (name, room_entity) = if let Ok((named, location)) = emoting_query.get(*entity) {
                (named.name.as_str(), location.room)
            } else {
                tracing::warn!(
                    "Entity {:?} cannot emote without Named and Location.",
                    entity
                );
                continue;
            };

            let message = format!("{} {}", name, emote);

            let room = room_query
                .get(room_entity)
                .expect("Location contains a valid room.");

            for player in &room.players {
                if let Ok(mut messages) = present_query.get_mut(*player) {
                    messages.queue(message.clone());
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub entity: Entity,
    pub message: String,
}

into_action!(Message);

pub fn message_system(
    mut action_reader: EventReader<Action>,
    messaging_query: Query<&Location>,
    mut present_query: Query<&mut Messages>,
    room_query: Query<&Room>,
) {
    for action in action_reader.iter() {
        if let Action::Say(Say { entity, message }) = action {
            let room_entity = if let Ok(location) = messaging_query.get(*entity) {
                location.room
            } else {
                tracing::warn!("Entity {:?} cannot say without Named and Location.", entity);
                continue;
            };

            let room = room_query
                .get(room_entity)
                .expect("Location contains a valid room.");

            for player in &room.players {
                if let Ok(mut messages) = present_query.get_mut(*player) {
                    messages.queue(message.clone());
                }
            }
        }
    }
}

pub fn parse_say(player: Entity, tokenizer: Tokenizer) -> Result<Action, String> {
    if tokenizer.rest().is_empty() {
        Err("Say what?".to_string())
    } else {
        Ok(Action::from(Say {
            entity: player,
            message: tokenizer.rest().to_string(),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct Say {
    pub entity: Entity,
    pub message: String,
}

into_action!(Say);

pub fn say_system(
    mut action_reader: EventReader<Action>,
    saying_query: Query<(&Named, &Location)>,
    mut present_query: Query<&mut Messages>,
    room_query: Query<&Room>,
) {
    for action in action_reader.iter() {
        if let Action::Say(Say { entity, message }) = action {
            let (name, room_entity) = if let Ok((named, location)) = saying_query.get(*entity) {
                (named.name.as_str(), location.room)
            } else {
                tracing::warn!("Entity {:?} cannot say without Named and Location.", entity);
                continue;
            };

            let other_message = format!("{} says \"{}\"", name, message);

            let room = room_query
                .get(room_entity)
                .expect("Location contains a valid room.");

            for player in &room.players {
                if *player == *entity {
                    if let Ok(mut messages) = present_query.get_mut(*player) {
                        messages.queue(format!("You say \"{}\"", message));
                    }
                } else if let Ok(mut messages) = present_query.get_mut(*player) {
                    messages.queue(other_message.clone());
                }
            }
        }
    }
}

pub fn parse_send(player: Entity, mut tokenizer: Tokenizer) -> Result<Action, String> {
    if let Some(target) = tokenizer.next() {
        if tokenizer.rest().is_empty() {
            Err(format!("Send what to {}?", target))
        } else {
            Ok(Action::from(SendMessage {
                entity: player,
                recipient: target.to_string(),
                message: tokenizer.rest().to_string(),
            }))
        }
    } else {
        Err("Send to whom?".to_string())
    }
}

#[derive(Debug, Clone)]
pub struct SendMessage {
    pub entity: Entity,
    pub recipient: String,
    pub message: String,
}

impl From<SendMessage> for Action {
    fn from(value: SendMessage) -> Self {
        Action::Send(value)
    }
}

pub fn send_system(
    mut action_reader: EventReader<Action>,
    players: Res<Players>,
    saying_query: Query<&Named>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Send(SendMessage {
            entity,
            recipient,
            message,
        }) = action
        {
            let name = if let Ok(named) = saying_query.get(*entity) {
                named.name.as_str()
            } else {
                tracing::warn!("Nameless entity {:?} cannot send a message.", entity);
                continue;
            };

            let recipient = if let Some(recipient) = players.by_name(recipient.as_str()) {
                recipient
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!(
                        "Your term beeps in irritation: \"User '{}' not found.\"",
                        recipient
                    ));
                };

                continue;
            };

            if recipient == *entity {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue("Your term trills: \"Invalid recipient: Self.\"".to_string());
                };

                continue;
            }

            messages_query
                .get_mut(recipient)
                .expect("Recipient player has Messages.")
                .queue(format!("{} sends \"{}\"", name, message));

            if let Ok(mut messages) = messages_query.get_mut(*entity) {
                messages.queue("Your term chirps happily: \"Message sent.\"".to_string());
            };
        }
    }
}
