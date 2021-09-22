use bevy_app::EventReader;
use bevy_ecs::prelude::*;

use crate::{
    into_action,
    text::Tokenizer,
    world::{
        action::Action,
        scripting::{ScriptHook, ScriptHooks},
        types::{
            object::Object,
            player::{Messages, Player, Players},
            room::Room,
            Contents, Description, Location, Named,
        },
    },
};

// Valid shapes:
// player <name> info - displays information about the player
pub fn parse_player(player: Entity, mut tokenizer: Tokenizer) -> Result<Action, String> {
    if let Some(name) = tokenizer.next() {
        if let Some(token) = tokenizer.next() {
            match token {
                "info" => Ok(Action::from(PlayerInfo {
                    actor: player,
                    name: name.to_string(),
                })),
                _ => Err("Enter a valid player subcommand: info.".to_string()),
            }
        } else {
            Err("Enter a player subcommand: info.".to_string())
        }
    } else {
        Err("Enter a player name.".to_string())
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PlayerInfo {
    pub actor: Entity,
    pub name: String,
}

into_action!(PlayerInfo);

pub fn player_info_system(
    mut action_reader: EventReader<Action>,
    players: Res<Players>,
    player_query: Query<(
        &Player,
        &Description,
        &Contents,
        &Location,
        Option<&ScriptHooks>,
    )>,
    room_query: Query<&Room>,
    object_query: Query<(&Object, &Named)>,
    mut message_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::PlayerInfo(PlayerInfo { actor, name }) = action {
            let player = if let Some(entity) = players.by_name(name) {
                entity
            } else {
                if let Ok(mut messages) = message_query.get_mut(*actor) {
                    messages.queue(format!("Player '{}' not found.", name))
                }
                continue;
            };

            let (player, description, contents, location, hooks) =
                player_query.get(player).unwrap();
            let room = room_query.get(location.room).unwrap();

            let mut message = format!("|white|Player {}|-|", name);

            message.push_str("\r\n  |white|id|-|: ");
            message.push_str(player.id.to_string().as_str());

            message.push_str("\r\n  |white|description|-|: ");
            message.push_str(description.text.as_str());

            message.push_str("\r\n  |white|room|-|: ");
            message.push_str(room.id.to_string().as_str());

            message.push_str("\r\n  |white|inventory|-|:");
            contents
                .objects
                .iter()
                .filter_map(|object| {
                    object_query
                        .get(*object)
                        .map(|(object, named)| (object.id, named.name.as_str()))
                        .ok()
                })
                .for_each(|(id, name)| {
                    message.push_str(format!("\r\n    object {}: {}", id, name).as_str())
                });
            message.push_str("\r\n  |white|script hooks|-|:");
            if let Some(ScriptHooks { list }) = hooks {
                if list.is_empty() {
                    message.push_str(" none");
                }
                for ScriptHook { trigger, script } in list.iter() {
                    message.push_str(format!("\r\n    {:?} -> {}", trigger, script).as_str());
                }
            } else {
                message.push_str(" none");
            }

            if let Ok(mut messages) = message_query.get_mut(*actor) {
                messages.queue(message);
            }
        }
    }
}
