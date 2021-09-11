mod communicate;
mod movement;
mod object;
mod observe;
mod room;
mod system;

use bevy_ecs::prelude::*;

use crate::{
    text::Tokenizer,
    world::{
        action::{
            communicate::{parse_say, parse_send, Say},
            movement::{parse_teleport, Move},
            object::{parse_drop, parse_get, Inventory},
            observe::{parse_look, Exits, Who},
            system::Shutdown,
        },
        types::{player::Messages, room::Direction},
    },
};

pub use observe::Look;
pub use system::{Login, Logout};

pub type DynAction = Box<dyn Action + Send>;

pub trait Action {
    fn enact(&mut self, player: Entity, world: &mut World) -> anyhow::Result<()>;
}

pub fn parse(input: &str) -> Result<DynAction, String> {
    if let Some(message) = input.strip_prefix('\'').map(ToString::to_string) {
        if message.is_empty() {
            return Err("Say what?".to_string());
        }

        return Ok(Say::new(message));
    }

    let mut tokenizer = Tokenizer::new(input);
    if let Some(token) = tokenizer.next() {
        match token.to_lowercase().as_str() {
            "down" => Ok(Move::new(Direction::Down)),
            "drop" => parse_drop(tokenizer),
            "east" => Ok(Move::new(Direction::East)),
            "exits" => Ok(Box::new(Exits {})),
            "get" => parse_get(tokenizer),
            "inventory" => Ok(Box::new(Inventory {})),
            "look" => parse_look(tokenizer),
            "north" => Ok(Move::new(Direction::North)),
            "object" => object::parse(tokenizer),
            "room" => room::parse(tokenizer),
            "say" => parse_say(tokenizer),
            "send" => parse_send(tokenizer),
            "shutdown" => Ok(Box::new(Shutdown {})),
            "south" => Ok(Move::new(Direction::South)),
            "teleport" => parse_teleport(tokenizer),
            "up" => Ok(Move::new(Direction::Up)),
            "west" => Ok(Move::new(Direction::West)),
            "who" => Ok(Box::new(Who {})),
            _ => Err("I don't know what that means.".to_string()),
        }
    } else {
        Err("Go on, then.".to_string())
    }
}

pub fn queue_message(world: &mut World, player: Entity, mut message: String) {
    message.push_str("\r\n");

    match world.get_mut::<Messages>(player) {
        Some(mut messages) => messages.queue(message),
        None => {
            world.entity_mut(player).insert(Messages::new_with(message));
        }
    }
}
