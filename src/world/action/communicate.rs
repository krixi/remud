use std::mem;

use bevy_ecs::prelude::*;

use crate::{
    text::Tokenizer,
    world::{
        action::{queue_message, Action, DynAction},
        types::player::Players,
        WantsToSay, WantsToSendMessage,
    },
};

pub struct Say {
    message: String,
}

impl Say {
    pub fn new(message: String) -> Box<Self> {
        Box::new(Say { message })
    }
}

impl Action for Say {
    fn enact(&mut self, player: Entity, world: &mut World) {
        let mut message = String::new();
        std::mem::swap(&mut self.message, &mut message);
        world.entity_mut(player).insert(WantsToSay { message });
    }
}

pub fn parse_send(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(player) = tokenizer.next() {
        Ok(Box::new(SendMessage {
            player: player.to_string(),
            message: tokenizer.rest().to_string(),
        }))
    } else {
        Err("Send to whom?".to_string())
    }
}

struct SendMessage {
    player: String,
    message: String,
}

impl Action for SendMessage {
    fn enact(&mut self, player: Entity, world: &mut World) {
        if let Some(recipient) = world
            .get_resource::<Players>()
            .unwrap()
            .by_name(self.player.as_str())
        {
            let mut message = String::new();
            mem::swap(&mut self.message, &mut message);
            world
                .entity_mut(player)
                .insert(WantsToSendMessage { recipient, message });
        } else {
            let message = format!(
                "Your term beeps in irritation: \"User '{}' not found.\"\r\n",
                self.player
            );
            queue_message(world, player, message)
        }
    }
}
