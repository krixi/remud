use bevy_app::EventReader;
use bevy_ecs::prelude::*;

use crate::{
    event_from_action,
    text::Tokenizer,
    world::{
        action::ActionEvent,
        scripting::{PostEventScriptHooks, PreEventScriptHooks, ScriptHook},
        types::{
            object::Object,
            player::{Messages, Players},
            room::Room,
            Contents, Location, Named,
        },
    },
};

// Valid shapes:
// player <name> info - displays information about the player
pub fn parse(player: Entity, mut tokenizer: Tokenizer) -> Result<ActionEvent, String> {
    if let Some(name) = tokenizer.next() {
        if let Some(token) = tokenizer.next() {
            match token {
                "info" => Ok(ActionEvent::from(PlayerInfo {
                    entity: player,
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

#[derive(Debug, Clone)]
pub struct PlayerInfo {
    pub entity: Entity,
    pub name: String,
}

event_from_action!(PlayerInfo);

pub fn player_info_system(
    mut events: EventReader<ActionEvent>,
    players: Res<Players>,
    player_query: Query<(
        &Contents,
        &Location,
        Option<&PreEventScriptHooks>,
        Option<&PostEventScriptHooks>,
    )>,
    room_query: Query<&Room>,
    object_query: Query<(&Object, &Named)>,
    mut message_query: Query<&mut Messages>,
) {
    for event in events.iter() {
        if let ActionEvent::PlayerInfo(PlayerInfo { entity, name }) = event {
            let player = if let Some(entity) = players.by_name(name) {
                entity
            } else {
                if let Ok(mut messages) = message_query.get_mut(*entity) {
                    messages.queue(format!("Player '{}' not found.", name))
                }
                continue;
            };

            let (contents, location, pre_hooks, post_hooks) = player_query.get(player).unwrap();
            let room = room_query.get(location.room).unwrap();

            let mut message = format!("Player {}", name);

            message.push_str("\r\n  room: ");
            message.push_str(room.id.to_string().as_str());

            message.push_str("\r\n  inventory:");
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
            if let Some(PreEventScriptHooks { list }) = pre_hooks {
                message.push_str("\r\n  pre-event hooks:");
                if list.is_empty() {
                    message.push_str(" none");
                }
                for ScriptHook { trigger, script } in list.iter() {
                    message.push_str(format!("\r\n    {} -> {}", trigger, script).as_str());
                }
            } else {
                message.push_str("\r\n  pre-event hooks: none");
            }
            if let Some(PostEventScriptHooks { list }) = post_hooks {
                message.push_str("\r\n  post-event hooks:");
                if list.is_empty() {
                    message.push_str(" none");
                }
                for ScriptHook { trigger, script } in list.iter() {
                    message.push_str(format!("\r\n    {} -> {}", trigger, script).as_str());
                }
            } else {
                message.push_str("\r\n  post-event hooks: none");
            }

            if let Ok(mut messages) = message_query.get_mut(*entity) {
                messages.queue(message);
            }
        }
    }
}
