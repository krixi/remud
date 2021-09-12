use anyhow::bail;
use bevy_ecs::prelude::{Entity, World};

use crate::{
    text::Tokenizer,
    world::{
        action::{queue_message, Action, DynAction},
        types::{
            object::Object,
            player::{Player, Players},
            room::Room,
            Contents,
        },
    },
};

// Valid shapes:
// player <name> info - displays information about the player
pub fn parse(mut tokenizer: Tokenizer) -> Result<DynAction, String> {
    if let Some(token) = tokenizer.next() {
        match token {
            name => {
                if let Some(token) = tokenizer.next() {
                    match token {
                        "info" => Ok(Box::new(Info {
                            name: name.to_string(),
                        })),
                        _ => Err("Enter a valid player subcommand: info.".to_string()),
                    }
                } else {
                    Err("Enter a player subcommand: info.".to_string())
                }
            }
        }
    } else {
        Err("Enter a player name.".to_string())
    }
}

struct Info {
    name: String,
}

impl Action for Info {
    fn enact(&mut self, asking_player: Entity, world: &mut World) -> anyhow::Result<()> {
        let player_entity = match world
            .get_resource::<Players>()
            .unwrap()
            .by_name(self.name.as_str())
        {
            Some(entity) => entity,
            None => {
                let message = format!("Player '{}' not found.", self.name);
                queue_message(world, asking_player, message);
                return Ok(());
            }
        };

        let player = match world.get::<Player>(player_entity) {
            Some(player) => player,
            None => bail!("{:?} has no Player.", player_entity),
        };

        let room = match world.get::<Room>(player.room) {
            Some(room) => room,
            None => bail!("{:?} has no Room.", player.room),
        };

        let mut message = format!("Player {}", player.name);

        message.push_str("\r\n  room: ");
        message.push_str(room.id.to_string().as_str());

        message.push_str("\r\n  objects:");
        match world.get::<Contents>(player_entity) {
            Some(contents) => {
                contents
                    .objects
                    .iter()
                    .filter_map(|object| world.get::<Object>(*object))
                    .map(|object| (object.id, object.short.as_str()))
                    .for_each(|(id, name)| {
                        message.push_str(format!("\r\n    object {}: {}", id, name).as_str());
                    });
            }
            None => bail!("{:?} has no Contents.", player_entity),
        }

        queue_message(world, asking_player, message);

        Ok(())
    }
}
