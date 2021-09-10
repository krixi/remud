mod communicate;
mod movement;
mod object;
mod observe;
mod room;
mod system;

pub use system::{Login, Logout};

use bevy_ecs::prelude::*;

use crate::{
    text::Tokenizer,
    world::{
        action::{
            communicate::{parse_send, Say},
            movement::{parse_teleport, Move},
            observe::{parse_look, Exits, Who},
            system::Shutdown,
        },
        types::{player::Messages, room::Direction},
    },
};

pub type DynAction = Box<dyn Action + Send>;

pub trait Action {
    fn enact(&mut self, player: Entity, world: &mut World);
}

pub fn parse(input: &str) -> Result<DynAction, String> {
    if let Some(message) = input.strip_prefix('\'').map(|str| str.to_string()) {
        return Ok(Say::new(message));
    }

    let mut tokenizer = Tokenizer::new(input);
    if let Some(token) = tokenizer.next() {
        match token.to_lowercase().as_str() {
            "down" => Ok(Move::new(Direction::Down)),
            "east" => Ok(Move::new(Direction::East)),
            "exits" => Ok(Box::new(Exits {})),
            "look" => parse_look(tokenizer),
            "north" => Ok(Move::new(Direction::North)),
            "object" => object::parse(tokenizer),
            "room" => room::parse(tokenizer),
            "say" => Ok(Say::new(tokenizer.rest().to_string())),
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

fn queue_message(world: &mut World, player: Entity, message: String) {
    match world.entity_mut(player).get_mut::<Messages>() {
        Some(mut messages) => messages.queue(message),
        None => {
            world.entity_mut(player).insert(Messages::new_with(message));
        }
    }
}
