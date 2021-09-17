use bevy_app::EventReader;
use bevy_ecs::prelude::*;

use crate::{
    event_from_action,
    text::Tokenizer,
    world::{
        action::ActionEvent,
        types::{
            player::{Messages, Players},
            room::Room,
            Location, Named,
        },
    },
};

pub fn parse_me(player: Entity, tokenizer: Tokenizer) -> Result<ActionEvent, String> {
    if tokenizer.rest().is_empty() {
        Err("Do what?".to_string())
    } else {
        Ok(ActionEvent::from(Emote {
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

event_from_action!(Emote);

pub fn emote_system(
    mut events: EventReader<ActionEvent>,
    emoting_query: Query<(&Named, &Location)>,
    mut present_query: Query<&mut Messages>,
    room_query: Query<&Room>,
) {
    for event in events.iter() {
        if let ActionEvent::Emote(Emote { entity, emote }) = event {
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

pub fn parse_say(player: Entity, tokenizer: Tokenizer) -> Result<ActionEvent, String> {
    if tokenizer.rest().is_empty() {
        Err("Say what?".to_string())
    } else {
        Ok(ActionEvent::from(Say {
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

event_from_action!(Say);

pub fn say_system(
    mut events: EventReader<ActionEvent>,
    saying_query: Query<(&Named, &Location)>,
    mut present_query: Query<&mut Messages>,
    room_query: Query<&Room>,
) {
    for event in events.iter() {
        if let ActionEvent::Say(Say { entity, message }) = event {
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

pub fn parse_send(player: Entity, mut tokenizer: Tokenizer) -> Result<ActionEvent, String> {
    if let Some(target) = tokenizer.next() {
        if tokenizer.rest().is_empty() {
            Err(format!("Send what to {}?", target))
        } else {
            Ok(ActionEvent::from(SendMessage {
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

impl From<SendMessage> for ActionEvent {
    fn from(value: SendMessage) -> Self {
        ActionEvent::Send(value)
    }
}

pub fn send_system(
    mut events: EventReader<ActionEvent>,
    players: Res<Players>,
    saying_query: Query<&Named>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Send(SendMessage {
            entity,
            recipient,
            message,
        }) = event
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
