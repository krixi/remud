pub mod communicate;
pub mod immortal;
pub mod movement;
pub mod object;
pub mod observe;
pub mod system;

use bevy_ecs::prelude::*;
use thiserror::Error;

use crate::{
    text::Tokenizer,
    world::{
        action::{
            communicate::{parse_me, parse_say, parse_send, Emote, Say},
            movement::{parse_teleport, Move},
            object::{parse_drop, parse_get, Inventory},
            observe::{parse_look, Exits, Who},
            system::Shutdown,
        },
        types::{
            player::Messages,
            room::{self, Direction},
        },
    },
};

pub const DEFAULT_ROOM_DESCRIPTION: &str = "An empty room.";
pub const DEFAULT_OBJECT_KEYWORD: &str = "object";
pub const DEFAULT_OBJECT_SHORT: &str = "an object";
pub const DEFAULT_OBJECT_LONG: &str = "A nondescript object. Completely uninteresting.";

pub type DynAction = Box<dyn Action + Send>;

pub enum ActionEvent {
    Drop {
        entity: Entity,
        keywords: Vec<String>,
    },
    Emote {
        entity: Entity,
        message: String,
    },
    Exits {
        entity: Entity,
    },
    Get {
        entity: Entity,
        keywords: Vec<String>,
    },
    Inventory {
        entity: Entity,
    },
    Look {
        entity: Entity,
        direction: Option<Direction>,
    },
    LookAt {
        entity: Entity,
        keywords: Vec<String>,
    },
    Move {
        entity: Entity,
        direction: Direction,
    },
    Say {
        entity: Entity,
        message: String,
    },
    Send {
        entity: Entity,
        recipient: String,
        message: String,
    },
    Teleport {
        entity: Entity,
        room_id: room::Id,
    },
    Who {
        entity: Entity,
    },
}

pub trait Action {
    fn enact(&mut self, entity: Entity, world: &mut World) -> Result<(), Error>;
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?} has no {1}.")]
    MissingComponent(Entity, &'static str),
}

pub fn parse(input: &str) -> Result<DynAction, String> {
    if let Some(message) = input.strip_prefix('\'').map(ToString::to_string) {
        if message.is_empty() {
            return Err("Say what?".to_string());
        }

        return Ok(Say::new(message));
    } else if let Some(emote) = input.strip_prefix(';').map(ToString::to_string) {
        if emote.is_empty() {
            return Err("Do what?".to_string());
        }

        return Ok(Emote::new(emote));
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
            "me" => parse_me(tokenizer),
            "north" => Ok(Move::new(Direction::North)),
            "object" => immortal::object::parse(tokenizer),
            "player" => immortal::player::parse(tokenizer),
            "room" => immortal::room::parse(tokenizer),
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
