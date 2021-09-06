use std::str::FromStr;

use bevy_ecs::prelude::*;

use crate::{
    engine::world::{Configuration, Direction, WantsToLook, WantsToSay},
    text::Tokenizer,
};

pub type DynAction = Box<dyn Action + Send>;

pub trait Action {
    fn enact(&self, player: Entity, world: &mut World);
}

struct Look {}

impl Action for Look {
    fn enact(&self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsToLook {});
    }
}

struct CreateRoom {
    direction: Option<Direction>,
}

impl Action for CreateRoom {
    fn enact(&self, player: Entity, world: &mut World) {
        // create new room

        if let Some(direction) = self.direction {
            // link room
        }

        // teleport player to new room
    }
}

struct Say {
    message: String,
}

impl Action for Say {
    fn enact(&self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsToSay {
            message: self.message.clone(),
        });
    }
}

struct Shutdown {}

impl Action for Shutdown {
    fn enact(&self, _player: Entity, world: &mut World) {
        let mut configuration = world.get_resource_mut::<Configuration>().unwrap();
        configuration.shutdown = true;
    }
}

pub fn parse_action(input: &str) -> Result<DynAction, String> {
    let mut tokenizer = Tokenizer::new(input);
    if let Some(token) = tokenizer.next() {
        match token.to_lowercase().as_str() {
            "look" => Ok(Box::new(Look {})),
            "room" => parse_room(tokenizer),
            "say" => Ok(Box::new(Say {
                message: tokenizer.rest().to_string(),
            })),
            "shutdown" => Ok(Box::new(Shutdown {})),
            _ => Err("I don't know what that means.".to_string()),
        }
    } else {
        Err("Go on, then.".to_string())
    }
}

// Valid shapes:
// room new - creates a new unlinked room
// room new [Direction] - creates a room to the [Direction] of this one with a two way link
fn parse_room(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(subcommand) = tokenizer.next() {
        match subcommand.to_lowercase().as_str() {
            "new" => {
                let direction = if let Some(direction) = tokenizer.next() {
                    match Direction::from_str(direction) {
                        Ok(direction) => Some(direction),
                        Err(_) => return Err(format!("'{}' is not a valid direction.", direction)),
                    }
                } else {
                    None
                };

                Ok(Box::new(CreateRoom { direction }))
            }
            s => Err(format!("'{}' is not a valid room subcommand.", s)),
        }
    } else {
        Err("'room' requires a subcommand.".to_string())
    }
}
