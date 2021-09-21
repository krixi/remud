pub mod attributes;
pub mod commands;
pub mod communicate;
pub mod immortal;
pub mod movement;
pub mod object;
pub mod observe;
pub mod system;

use bevy_ecs::prelude::*;
use thiserror::Error;

use crate::world::action::{
    attributes::{stats_system, Stats},
    communicate::{
        emote_system, message_system, say_system, send_system, Emote, Message, Say, SendMessage,
    },
    immortal::{
        object::{
            object_clear_flags_system, object_create_system, object_info_system,
            object_remove_system, object_set_flags_system, object_update_description_system,
            object_update_keywords_system, object_update_name_system, ObjectCreate, ObjectInfo,
            ObjectRemove, ObjectSetFlags, ObjectUnsetFlags, ObjectUpdateDescription,
            ObjectUpdateKeywords, ObjectUpdateName,
        },
        player::{player_info_system, PlayerInfo},
        room::{
            room_create_system, room_info_system, room_link_system, room_remove_system,
            room_unlink_system, room_update_description_system, room_update_regions_system,
            RoomCreate, RoomInfo, RoomLink, RoomRemove, RoomUnlink, RoomUpdateDescription,
            RoomUpdateRegions,
        },
        script::{script_attach_system, script_detach_system, ScriptAttach, ScriptDetach},
    },
    movement::{move_system, teleport_system, Move, Teleport},
    object::{drop_system, get_system, inventory_system, Drop, Get, Inventory},
    observe::{exits_system, look_at_system, look_system, who_system, Exits, Look, LookAt, Who},
    system::{login_system, shutdown_system, Login, Shutdown},
};

#[macro_export]
macro_rules! into_action {
    ($action:tt) => {
        impl From<$action> for Action {
            fn from(value: $action) -> Self {
                Action::$action(value)
            }
        }
    };
}

pub const DEFAULT_ROOM_DESCRIPTION: &str = "An empty room.";
pub const DEFAULT_OBJECT_KEYWORD: &str = "object";
pub const DEFAULT_OBJECT_NAME: &str = "an object";
pub const DEFAULT_OBJECT_DESCRIPTION: &str = "A nondescript object. Completely uninteresting.";

#[derive(Debug, Clone)]
pub enum Action {
    Drop(Drop),
    Emote(Emote),
    Exits(Exits),
    Get(Get),
    Inventory(Inventory),
    Login(Login),
    Look(Look),
    LookAt(LookAt),
    Message(Message),
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
    RoomRemove(RoomRemove),
    RoomUnlink(RoomUnlink),
    RoomUpdateDescription(RoomUpdateDescription),
    RoomUpdateRegions(RoomUpdateRegions),
    Say(Say),
    ScriptAttach(ScriptAttach),
    ScriptDetach(ScriptDetach),
    Send(SendMessage),
    Shutdown(Shutdown),
    Stats(Stats),
    Teleport(Teleport),
    Who(Who),
}

impl Action {
    pub fn enactor(&self) -> Entity {
        match self {
            Action::Drop(action) => action.entity,
            Action::Emote(action) => action.entity,
            Action::Exits(action) => action.entity,
            Action::Get(action) => action.entity,
            Action::Inventory(action) => action.entity,
            Action::Login(action) => action.entity,
            Action::Look(action) => action.entity,
            Action::LookAt(action) => action.entity,
            Action::Message(action) => action.entity,
            Action::Move(action) => action.entity,
            Action::ObjectCreate(action) => action.entity,
            Action::ObjectInfo(action) => action.entity,
            Action::ObjectRemove(action) => action.entity,
            Action::ObjectSetFlags(action) => action.entity,
            Action::ObjectUnsetFlags(action) => action.entity,
            Action::ObjectUpdateDescription(action) => action.entity,
            Action::ObjectUpdateKeywords(action) => action.entity,
            Action::ObjectUpdateName(action) => action.entity,
            Action::PlayerInfo(action) => action.entity,
            Action::RoomCreate(action) => action.entity,
            Action::RoomInfo(action) => action.entity,
            Action::RoomLink(action) => action.entity,
            Action::RoomRemove(action) => action.entity,
            Action::RoomUnlink(action) => action.entity,
            Action::RoomUpdateRegions(action) => action.entity,
            Action::RoomUpdateDescription(action) => action.entity,
            Action::Say(action) => action.entity,
            Action::ScriptAttach(action) => action.entity,
            Action::ScriptDetach(action) => action.entity,
            Action::Send(action) => action.entity,
            Action::Shutdown(action) => action.entity,
            Action::Stats(action) => action.entity,
            Action::Teleport(action) => action.entity,
            Action::Who(action) => action.entity,
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
    stage.add_system(look_at_system.system());
    stage.add_system(look_system.system().label("look"));
    stage.add_system(message_system.system());
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
    stage.add_system(room_update_regions_system.system());
    stage.add_system(say_system.system().after("look"));
    stage.add_system(script_attach_system.system());
    stage.add_system(script_detach_system.system());
    stage.add_system(send_system.system().after("look"));
    stage.add_system(shutdown_system.system());
    stage.add_system(stats_system.system());
    stage.add_system(teleport_system.system());
    stage.add_system(who_system.system());
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?} has no {1}.")]
    MissingComponent(Entity, &'static str),
}
