use bevy_app::EventReader;
use bevy_ecs::prelude::*;

use crate::{
    text::Tokenizer,
    world::{
        action::{get_room_std, into_action, Action},
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
            actor: player,
            emote: tokenizer.rest().to_string(),
        }))
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Emote {
    pub actor: Entity,
    pub emote: String,
}

into_action!(Emote);

#[tracing::instrument(name = "emote system", skip_all)]
pub fn emote_system(
    mut action_reader: EventReader<Action>,
    location_query: Query<(Option<&Location>, Option<&Room>)>,
    named_query: Query<&Named>,
    room_query: Query<&Room>,
    mut present_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Emote(Emote { actor, emote }) = action {
            let room_entity = get_room_std(*actor, &location_query);
            let name = if let Ok(named) = named_query.get(*actor) {
                named.as_str()
            } else {
                tracing::warn!("Entity {:?} cannot emote without Named.", actor);
                continue;
            };

            let message = format!("{} {}", name, emote);

            let room = room_query
                .get(room_entity)
                .expect("Location contains a valid room.");

            for player in room.players() {
                if let Ok(mut messages) = present_query.get_mut(*player) {
                    messages.queue(message.clone());
                }
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Message {
    pub actor: Entity,
    pub message: String,
}

into_action!(Message);

#[tracing::instrument(name = "message system", skip_all)]
pub fn message_system(
    mut action_reader: EventReader<Action>,
    location_query: Query<(Option<&Location>, Option<&Room>)>,
    mut present_query: Query<&mut Messages>,
    room_query: Query<&Room>,
) {
    for action in action_reader.iter() {
        if let Action::Message(Message { actor, message }) = action {
            let room_entity = get_room_std(*actor, &location_query);

            let room = room_query
                .get(room_entity)
                .expect("Location contains a valid room.");

            for player in room.players() {
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
            actor: player,
            message: tokenizer.rest().to_string(),
        }))
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Say {
    pub actor: Entity,
    pub message: String,
}

into_action!(Say);

#[tracing::instrument(name = "say system", skip_all)]
pub fn say_system(
    mut action_reader: EventReader<Action>,
    location_query: Query<(Option<&Location>, Option<&Room>)>,
    saying_query: Query<&Named>,
    mut present_query: Query<&mut Messages>,
    room_query: Query<&Room>,
) {
    for action in action_reader.iter() {
        if let Action::Say(Say { actor, message }) = action {
            let room_entity = get_room_std(*actor, &location_query);
            let name = if let Ok(named) = saying_query.get(*actor) {
                named.as_str()
            } else {
                tracing::warn!("entity {:?} cannot say without Named.", actor);
                continue;
            };

            let other_message = format!("{} says \"{}\"", name, message);

            let room = room_query
                .get(room_entity)
                .expect("Location contains a valid room.");

            for player in room.players() {
                if *player == *actor {
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
                actor: player,
                recipient: target.to_string(),
                message: tokenizer.rest().to_string(),
            }))
        }
    } else {
        Err("Send to whom?".to_string())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SendMessage {
    pub actor: Entity,
    pub recipient: String,
    pub message: String,
}

impl From<SendMessage> for Action {
    fn from(value: SendMessage) -> Self {
        Action::Send(value)
    }
}

#[tracing::instrument(name = "send message system", skip_all)]
pub fn send_message_system(
    mut action_reader: EventReader<Action>,
    players: Res<Players>,
    saying_query: Query<&Named>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Send(SendMessage {
            actor,
            recipient,
            message,
        }) = action
        {
            let name = if let Ok(named) = saying_query.get(*actor) {
                named.as_str()
            } else {
                tracing::warn!("nameless entity {:?} cannot send a message.", actor);
                continue;
            };

            let recipient = if let Some(recipient) = players.by_name(recipient.as_str()) {
                recipient
            } else {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue(format!(
                        "Your term beeps in irritation: \"User '{}' not found.\"",
                        recipient
                    ));
                };

                continue;
            };

            if recipient == *actor {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
                    messages.queue("Your term trills: \"Invalid recipient: Self.\"".to_string());
                };

                continue;
            }

            messages_query
                .get_mut(recipient)
                .expect("Recipient player has Messages.")
                .queue(format!("{} sends \"{}\"", name, message));

            if let Ok(mut messages) = messages_query.get_mut(*actor) {
                messages.queue("Your term chirps happily: \"Message sent.\"".to_string());
            };
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Whisper {
    pub actor: Entity,
    pub target: Entity,
    pub message: String,
}

into_action!(Whisper);

#[tracing::instrument(name = "whisper system", skip_all)]
pub fn whisper_system(
    mut action_reader: EventReader<Action>,
    mut target_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Whisper(Whisper {
            actor,
            target,
            message,
        }) = action
        {
            tracing::debug!(r#"{:?} => {:?}: "{}""#, actor, target, message);
            if let Ok(mut messages) = target_query.get_mut(*target) {
                messages.queue(message.clone());
            }
        }
    }
}
