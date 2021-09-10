use std::str::FromStr;

use bevy_ecs::prelude::{Entity, World};
use itertools::Itertools;

use crate::{
    text::Tokenizer,
    world::{
        action::{Action, DynAction},
        types::room::Direction,
        WantsExits, WantsToLook, WantsToLookAt, WantsWhoInfo,
    },
};

pub fn parse_look(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    match tokenizer.next() {
        Some(token) => {
            if token == "at" {
                let keywords = tokenizer
                    .rest()
                    .split_whitespace()
                    .map(|keyword| keyword.to_string())
                    .collect_vec();

                Ok(Box::new(Look {
                    at: Some(keywords),
                    direction: None,
                }))
            } else if let Ok(direction) = Direction::from_str(token) {
                Ok(Box::new(Look {
                    at: None,
                    direction: Some(direction),
                }))
            } else {
                Err(format!("I don't know how to look {}.", token))
            }
        }
        None => Ok(Box::new(Look {
            at: None,
            direction: None,
        })),
    }
}

struct Look {
    at: Option<Vec<String>>,
    direction: Option<Direction>,
}

impl Action for Look {
    fn enact(&mut self, player: Entity, world: &mut World) {
        if self.at.is_some() {
            world.entity_mut(player).insert(WantsToLookAt {
                keywords: self.at.take().unwrap(),
            });
        } else {
            world.entity_mut(player).insert(WantsToLook {
                direction: self.direction,
            });
        }
    }
}

pub struct Exits {}

impl Action for Exits {
    fn enact(&mut self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsExits {});
    }
}

pub struct Who {}

impl Action for Who {
    fn enact(&mut self, player: Entity, world: &mut World) {
        world.entity_mut(player).insert(WantsWhoInfo {});
    }
}
