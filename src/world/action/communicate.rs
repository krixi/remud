use bevy_app::{EventReader, Events};
use bevy_ecs::prelude::*;

use crate::{
    text::Tokenizer,
    world::{
        action::{self, Action, ActionEvent, DynAction},
        types::{
            player::{Messages, Players},
            room::Room,
            Location, Named,
        },
    },
};

pub fn parse_me(tokenizer: Tokenizer) -> Result<DynAction, String> {
    if tokenizer.rest().is_empty() {
        Err("Do what?".to_string())
    } else {
        Ok(Emote::new(tokenizer.rest().to_string()))
    }
}

pub struct Emote {
    emote: String,
}

impl Emote {
    pub fn new(emote: String) -> Box<Self> {
        Box::new(Emote { emote })
    }
}

impl Action for Emote {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::Emote {
                entity,
                message: self.emote.clone(),
            });

        Ok(())
    }
}

pub fn emote_system(
    mut events: EventReader<ActionEvent>,
    emoting_query: Query<(&Named, &Location)>,
    mut present_query: Query<&mut Messages>,
    room_query: Query<&Room>,
) {
    for event in events.iter() {
        if let ActionEvent::Emote { entity, message } = event {
            let (name, room_entity) = if let Ok((named, location)) = emoting_query.get(*entity) {
                (named.name.as_str(), location.room)
            } else {
                tracing::warn!(
                    "Entity {:?} cannot emote without Named and Location.",
                    entity
                );
                continue;
            };

            let message = format!("{} {}", name, message);

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

pub fn parse_say(tokenizer: Tokenizer) -> Result<DynAction, String> {
    if tokenizer.rest().is_empty() {
        Err("Say what?".to_string())
    } else {
        Ok(Say::new(tokenizer.rest().to_string()))
    }
}

pub struct Say {
    message: String,
}

impl Say {
    pub fn new(message: String) -> Box<Self> {
        Box::new(Say { message })
    }
}

pub fn say_system(
    mut events: EventReader<ActionEvent>,
    saying_query: Query<(&Named, &Location)>,
    mut present_query: Query<&mut Messages>,
    room_query: Query<&Room>,
) {
    for event in events.iter() {
        if let ActionEvent::Say { entity, message } = event {
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

impl Action for Say {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::Say {
                entity,
                message: self.message.clone(),
            });

        Ok(())
    }
}

pub fn parse_send(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(player) = tokenizer.next() {
        if tokenizer.rest().is_empty() {
            Err(format!("Send what to {}?", player))
        } else {
            Ok(Box::new(SendMessage {
                recipient: player.to_string(),
                message: tokenizer.rest().to_string(),
            }))
        }
    } else {
        Err("Send to whom?".to_string())
    }
}

struct SendMessage {
    recipient: String,
    message: String,
}

impl Action for SendMessage {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), action::Error> {
        world
            .get_resource_mut::<Events<ActionEvent>>()
            .unwrap()
            .send(ActionEvent::Send {
                entity,
                recipient: self.recipient.clone(),
                message: self.message.clone(),
            });

        Ok(())
    }
}

pub fn send_system(
    mut events: EventReader<ActionEvent>,
    players: Res<Players>,
    saying_query: Query<&Named>,
    mut messages_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::Send {
            entity,
            recipient,
            message,
        } = event
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
