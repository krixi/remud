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
            communicate::{
                emote_system, parse_me, parse_say, parse_send, say_system, send_system, Emote, Say,
                SendMessage,
            },
            immortal::{
                object::{
                    object_clear_flags_system, object_create_system, object_info_system,
                    object_remove_system, object_set_flags_system,
                    object_update_description_system, object_update_keywords_system,
                    object_update_name_system, ObjectCreate, ObjectInfo, ObjectRemove,
                    ObjectSetFlags, ObjectUnsetFlags, ObjectUpdateDescription,
                    ObjectUpdateKeywords, ObjectUpdateName,
                },
                player::{player_info_system, PlayerInfo},
                room::{
                    room_create_system, room_info_system, room_link_system, room_remove_system,
                    room_unlink_system, room_update_description_system, RoomCreate, RoomInfo,
                    RoomLink, RoomRemove, RoomUnlink, RoomUpdateDescription,
                },
                script::{
                    parse_script, script_attach_system, script_detach_system, ScriptAttach,
                    ScriptDetach,
                },
            },
            movement::{move_system, parse_teleport, teleport_system, Move, Teleport},
            object::{
                drop_system, get_system, inventory_system, parse_drop, parse_get, Drop, Get,
                Inventory,
            },
            observe::{
                exits_system, look_at_system, look_system, parse_look, who_system, Exits, Look,
                LookAt, Who,
            },
            system::{login_system, logout_system, shutdown_system, Login, Logout, Shutdown},
        },
        types::room::Direction,
    },
};

#[macro_export]
macro_rules! event_from_action {
    ($action:tt) => {
        impl From<$action> for ActionEvent {
            fn from(value: $action) -> Self {
                ActionEvent::$action(value)
            }
        }
    };
}

pub const DEFAULT_ROOM_DESCRIPTION: &str = "An empty room.";
pub const DEFAULT_OBJECT_KEYWORD: &str = "object";
pub const DEFAULT_OBJECT_NAME: &str = "an object";
pub const DEFAULT_OBJECT_DESCRIPTION: &str = "A nondescript object. Completely uninteresting.";

#[derive(Debug, Clone)]
pub enum ActionEvent {
    Drop(Drop),
    Emote(Emote),
    Exits(Exits),
    Get(Get),
    Inventory(Inventory),
    Login(Login),
    Logout(Logout),
    Look(Look),
    LookAt(LookAt),
    Move(Move),
    ObjectUnsetFlags(ObjectUnsetFlags),
    ObjectCreate(ObjectCreate),
    ObjectInfo(ObjectInfo),
    ObjectRemove(ObjectRemove),
    ObjectSetFlags(ObjectSetFlags),
    ObjectUpdateDescription(ObjectUpdateDescription),
    ObjectUpdateKeywords(ObjectUpdateKeywords),
    ObjectUpdateName(ObjectUpdateName),
    PlayerInfo(PlayerInfo),
    RoomCreate(RoomCreate),
    RoomInfo(RoomInfo),
    RoomLink(RoomLink),
    RoomUpdateDescription(RoomUpdateDescription),
    RoomRemove(RoomRemove),
    RoomUnlink(RoomUnlink),
    Say(Say),
    ScriptAttach(ScriptAttach),
    ScriptDetach(ScriptDetach),
    Send(SendMessage),
    Shutdown(Shutdown),
    Teleport(Teleport),
    Who(Who),
}

impl ActionEvent {
    pub fn enactor(&self) -> Entity {
        match self {
            ActionEvent::Drop(action) => action.entity,
            ActionEvent::Emote(action) => action.entity,
            ActionEvent::Exits(action) => action.entity,
            ActionEvent::Get(action) => action.entity,
            ActionEvent::Inventory(action) => action.entity,
            ActionEvent::Login(action) => action.entity,
            ActionEvent::Logout(action) => action.entity,
            ActionEvent::Look(action) => action.entity,
            ActionEvent::LookAt(action) => action.entity,
            ActionEvent::Move(action) => action.entity,
            ActionEvent::ObjectCreate(action) => action.entity,
            ActionEvent::ObjectInfo(action) => action.entity,
            ActionEvent::ObjectRemove(action) => action.entity,
            ActionEvent::ObjectSetFlags(action) => action.entity,
            ActionEvent::ObjectUnsetFlags(action) => action.entity,
            ActionEvent::ObjectUpdateDescription(action) => action.entity,
            ActionEvent::ObjectUpdateKeywords(action) => action.entity,
            ActionEvent::ObjectUpdateName(action) => action.entity,
            ActionEvent::PlayerInfo(action) => action.entity,
            ActionEvent::RoomCreate(action) => action.entity,
            ActionEvent::RoomInfo(action) => action.entity,
            ActionEvent::RoomLink(action) => action.entity,
            ActionEvent::RoomRemove(action) => action.entity,
            ActionEvent::RoomUnlink(action) => action.entity,
            ActionEvent::RoomUpdateDescription(action) => action.entity,
            ActionEvent::Say(action) => action.entity,
            ActionEvent::ScriptAttach(action) => action.entity,
            ActionEvent::ScriptDetach(action) => action.entity,
            ActionEvent::Send(action) => action.entity,
            ActionEvent::Shutdown(action) => action.entity,
            ActionEvent::Teleport(action) => action.entity,
            ActionEvent::Who(action) => action.entity,
        }
    }
}

pub fn register_action_systems(stage: &mut SystemStage) {
    stage.add_system(drop_system.system());
    stage.add_system(emote_system.system().after("look"));
    stage.add_system(exits_system.system());
    stage.add_system(get_system.system());
    stage.add_system(inventory_system.system());
    stage.add_system(login_system.system());
    stage.add_system(logout_system.system());
    stage.add_system(look_at_system.system());
    stage.add_system(look_system.system().label("look"));
    stage.add_system(move_system.system());
    stage.add_system(object_clear_flags_system.system());
    stage.add_system(object_create_system.system());
    stage.add_system(object_info_system.system());
    stage.add_system(object_remove_system.system());
    stage.add_system(object_set_flags_system.system());
    stage.add_system(object_update_description_system.system());
    stage.add_system(object_update_keywords_system.system());
    stage.add_system(object_update_name_system.system());
    stage.add_system(player_info_system.system());
    stage.add_system(room_create_system.system());
    stage.add_system(room_info_system.system());
    stage.add_system(room_link_system.system());
    stage.add_system(room_remove_system.system());
    stage.add_system(room_unlink_system.system());
    stage.add_system(room_update_description_system.system());
    stage.add_system(say_system.system().after("look"));
    stage.add_system(script_attach_system.system());
    stage.add_system(script_detach_system.system());
    stage.add_system(send_system.system().after("look"));
    stage.add_system(shutdown_system.system());
    stage.add_system(teleport_system.system());
    stage.add_system(who_system.system());
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?} has no {1}.")]
    MissingComponent(Entity, &'static str),
}

pub fn parse(player: Entity, input: &str) -> Result<ActionEvent, String> {
    if let Some(message) = input.strip_prefix('\'').map(ToString::to_string) {
        if message.is_empty() {
            return Err("Say what?".to_string());
        }

        return Ok(ActionEvent::from(Say {
            entity: player,
            message,
        }));
    } else if let Some(emote) = input.strip_prefix(';').map(ToString::to_string) {
        if emote.is_empty() {
            return Err("Do what?".to_string());
        }

        return Ok(ActionEvent::from(Emote {
            entity: player,
            emote,
        }));
    }

    let mut tokenizer = Tokenizer::new(input);
    if let Some(token) = tokenizer.next() {
        match token.to_lowercase().as_str() {
            "down" => Ok(ActionEvent::from(Move {
                entity: player,
                direction: Direction::Down,
            })),
            "drop" => parse_drop(player, tokenizer),
            "east" => Ok(ActionEvent::from(Move {
                entity: player,
                direction: Direction::East,
            })),
            "exits" => Ok(ActionEvent::from(Exits { entity: player })),
            "get" => parse_get(player, tokenizer),
            "inventory" => Ok(ActionEvent::from(Inventory { entity: player })),
            "look" => parse_look(player, tokenizer),
            "me" => parse_me(player, tokenizer),
            "north" => Ok(ActionEvent::from(Move {
                entity: player,
                direction: Direction::North,
            })),
            "object" => immortal::object::parse(player, tokenizer),
            "player" => immortal::player::parse(player, tokenizer),
            "room" => immortal::room::parse(player, tokenizer),
            "say" => parse_say(player, tokenizer),
            "script" => parse_script(player, tokenizer),
            "send" => parse_send(player, tokenizer),
            "shutdown" => Ok(ActionEvent::from(Shutdown { entity: player })),
            "south" => Ok(ActionEvent::from(Move {
                entity: player,
                direction: Direction::South,
            })),
            "teleport" => parse_teleport(player, tokenizer),
            "up" => Ok(ActionEvent::from(Move {
                entity: player,
                direction: Direction::Up,
            })),
            "west" => Ok(ActionEvent::from(Move {
                entity: player,
                direction: Direction::West,
            })),
            "who" => Ok(ActionEvent::from(Who { entity: player })),
            _ => Err("I don't know what that means.".to_string()),
        }
    } else {
        Err("Go on, then.".to_string())
    }
}
