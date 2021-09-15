use bevy_app::Events;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    text::Tokenizer,
    world::{
        action::{self, queue_message, Action, DynAction},
        types::{
            player::{Player, Players},
            room::Room,
        },
        PlayerAction, PlayerEvent,
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
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let (name, room_entity) = world
            .get::<Player>(player)
            .map(|player| (player.name.as_str(), player.room))
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let present_players = world
            .get::<Room>(room_entity)
            .ok_or(action::Error::MissingComponent(room_entity, "Room"))?
            .players
            .iter()
            .copied()
            .collect_vec();

        let message = format!("{} {}", name, self.emote);
        for present_player in present_players {
            queue_message(world, present_player, message.clone());
        }

        Ok(())
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

impl Action for Say {
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let (name, room_entity) = world
            .get::<Player>(player)
            .map(|player| (player.name.as_str(), player.room))
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let present_players = world
            .get::<Room>(room_entity)
            .ok_or(action::Error::MissingComponent(room_entity, "Room"))?
            .players
            .iter()
            .filter(|present_player| **present_player != player)
            .copied()
            .collect_vec();

        let message = format!("{} says \"{}\"", name, self.message);
        for present_player in present_players {
            queue_message(world, present_player, message.clone());
        }

        let message = format!("You say \"{}\"", self.message);
        queue_message(world, player, message);

        world
            .get_resource_mut::<Events<PlayerAction>>()
            .unwrap()
            .send(PlayerAction {
                player,
                event: PlayerEvent::Say {
                    room: room_entity,
                    message: self.message.clone(),
                },
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
    fn enact(&mut self, player: Entity, world: &mut World) -> Result<(), action::Error> {
        let recipient = if let Some(recipient) = world
            .get_resource::<Players>()
            .unwrap()
            .by_name(self.recipient.as_str())
        {
            recipient
        } else {
            let message = format!(
                "Your term beeps in irritation: \"User '{}' not found.\"",
                self.recipient
            );
            queue_message(world, player, message);
            return Ok(());
        };

        if recipient == player {
            let message = "Your term trills: \"Invalid recipient: Self.\"".to_string();
            queue_message(world, player, message);
            return Ok(());
        }

        let sender = world
            .get::<Player>(player)
            .map(|player| player.name.as_str())
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        let message = format!("{} sends \"{}\".", sender, self.message);
        queue_message(world, recipient, message);

        let recipient_name = world
            .get::<Player>(recipient)
            .map(|player| player.name.as_str())
            .ok_or(action::Error::MissingComponent(recipient, "Player"))?;

        let sent_message = format!(
            "Your term chirps happily: \"Message sent to '{}'.\"",
            recipient_name
        );
        queue_message(world, player, sent_message);

        Ok(())
    }
}
