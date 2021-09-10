use anyhow::bail;
use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    text::Tokenizer,
    world::{
        action::{queue_message, Action, DynAction},
        types::{
            player::{Player, Players},
            room::Room,
        },
    },
};

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
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
        let (name, room_entity) = match world.get::<Player>(player) {
            Some(player) => (player.name.as_str(), player.room),
            None => bail!("Player {:?} has no name.", player),
        };

        let present_players = match world.get::<Room>(room_entity) {
            Some(room) => room
                .players
                .iter()
                .filter(|present_player| **present_player != player)
                .cloned()
                .collect_vec(),
            None => bail!("Room {:?} has no Room.", room_entity),
        };

        let message = format!("{} says \"{}\"", name, self.message);
        for present_player in present_players {
            queue_message(world, present_player, message.clone());
        }

        let message = format!("You say \"{}\"", self.message);
        queue_message(world, player, message);

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
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()> {
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

        let sender = match world
            .get::<Player>(player)
            .map(|player| player.name.as_str())
        {
            Some(name) => name,
            None => bail!("Player {:?} has no name.", player),
        };

        let message = format!("{} sends \"{}\".", sender, self.message);
        queue_message(world, player, message);

        let recipient_name = match world
            .get::<Player>(recipient)
            .map(|player| player.name.as_str())
        {
            Some(name) => name,
            None => bail!("Recipient {:?} has no name.", player),
        };

        let sent_message = format!(
            "Your term chirps happily: \"Message sent to '{}'.\"",
            recipient_name
        );
        queue_message(world, player, sent_message);

        Ok(())
    }
}
