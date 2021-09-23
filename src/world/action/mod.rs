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
        initialize_system,
        object::{
            object_create_system, object_info_system, object_inherit_fields_system,
            object_remove_system, update_keywords_system, update_object_flags, ObjectCreate,
            ObjectInfo, ObjectInheritFields, ObjectRemove, UpdateKeywords, UpdateObjectFlags,
        },
        player::{player_info_system, player_update_flags_system, PlayerInfo, PlayerUpdateFlags},
        prototype::{
            prototype_create_system, prototype_info_system, prototype_remove_system,
            PrototypeCreate, PrototypeInfo, PrototypeRemove,
        },
        room::{
            room_create_system, room_info_system, room_link_system, room_remove_system,
            room_unlink_system, room_update_regions_system, RoomCreate, RoomInfo, RoomLink,
            RoomRemove, RoomUnlink, RoomUpdateRegions,
        },
        script::{script_attach_system, script_detach_system, ScriptAttach, ScriptDetach},
        update_description_system, update_name_system, Initialize, UpdateDescription, UpdateName,
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Action {
    Drop(Drop),
    Emote(Emote),
    Exits(Exits),
    Get(Get),
    Initialize(Initialize),
    Inventory(Inventory),
    Login(Login),
    Look(Look),
    LookAt(LookAt),
    Message(Message),
    Move(Move),
    ObjectCreate(ObjectCreate),
    ObjectInfo(ObjectInfo),
    ObjectInheritFields(ObjectInheritFields),
    ObjectRemove(ObjectRemove),
    PlayerInfo(PlayerInfo),
    PlayerUpdateFlags(PlayerUpdateFlags),
    PrototypeCreate(PrototypeCreate),
    PrototypeInfo(PrototypeInfo),
    PrototypeRemove(PrototypeRemove),
    RoomCreate(RoomCreate),
    RoomInfo(RoomInfo),
    RoomLink(RoomLink),
    RoomRemove(RoomRemove),
    RoomUnlink(RoomUnlink),
    RoomUpdateRegions(RoomUpdateRegions),
    Say(Say),
    ScriptAttach(ScriptAttach),
    ScriptDetach(ScriptDetach),
    Send(SendMessage),
    Shutdown(Shutdown),
    Stats(Stats),
    Teleport(Teleport),
    UpdateDescription(UpdateDescription),
    UpdateKeywords(UpdateKeywords),
    UpdateName(UpdateName),
    UpdateObjectFlags(UpdateObjectFlags),
    Who(Who),
}

impl Action {
    pub fn actor(&self) -> Entity {
        match self {
            Action::Drop(action) => action.actor,
            Action::Emote(action) => action.actor,
            Action::Exits(action) => action.actor,
            Action::Get(action) => action.actor,
            Action::Initialize(action) => action.actor,
            Action::Inventory(action) => action.actor,
            Action::Login(action) => action.actor,
            Action::Look(action) => action.actor,
            Action::LookAt(action) => action.actor,
            Action::Message(action) => action.actor,
            Action::Move(action) => action.actor,
            Action::ObjectCreate(action) => action.actor,
            Action::ObjectInfo(action) => action.actor,
            Action::ObjectInheritFields(action) => action.actor,
            Action::ObjectRemove(action) => action.actor,
            Action::PlayerInfo(action) => action.actor,
            Action::PlayerUpdateFlags(action) => action.actor,
            Action::PrototypeCreate(action) => action.actor,
            Action::PrototypeInfo(action) => action.actor,
            Action::PrototypeRemove(action) => action.actor,
            Action::RoomCreate(action) => action.actor,
            Action::RoomInfo(action) => action.actor,
            Action::RoomLink(action) => action.actor,
            Action::RoomRemove(action) => action.actor,
            Action::RoomUnlink(action) => action.actor,
            Action::RoomUpdateRegions(action) => action.actor,
            Action::Say(action) => action.actor,
            Action::ScriptAttach(action) => action.actor,
            Action::ScriptDetach(action) => action.actor,
            Action::Send(action) => action.actor,
            Action::Shutdown(action) => action.actor,
            Action::Stats(action) => action.actor,
            Action::Teleport(action) => action.actor,
            Action::UpdateDescription(action) => action.actor,
            Action::UpdateKeywords(action) => action.actor,
            Action::UpdateName(action) => action.actor,
            Action::UpdateObjectFlags(action) => action.actor,
            Action::Who(action) => action.actor,
        }
    }
}

pub fn register_action_systems(stage: &mut SystemStage) {
    stage.add_system(drop_system.system());
    stage.add_system(emote_system.system().after("look"));
    stage.add_system(exits_system.system());
    stage.add_system(get_system.system());
    stage.add_system(initialize_system.system());
    stage.add_system(inventory_system.system());
    stage.add_system(login_system.system());
    stage.add_system(look_at_system.system());
    stage.add_system(look_system.system().label("look"));
    stage.add_system(message_system.system());
    stage.add_system(move_system.system());
    stage.add_system(object_create_system.system());
    stage.add_system(object_info_system.system());
    stage.add_system(object_inherit_fields_system.system());
    stage.add_system(object_remove_system.system());
    stage.add_system(player_info_system.system());
    stage.add_system(player_update_flags_system.system());
    stage.add_system(prototype_create_system.system());
    stage.add_system(prototype_info_system.system());
    stage.add_system(prototype_remove_system.system());
    stage.add_system(room_create_system.system());
    stage.add_system(room_info_system.system());
    stage.add_system(room_link_system.system());
    stage.add_system(room_remove_system.system());
    stage.add_system(room_unlink_system.system());
    stage.add_system(room_update_regions_system.system());
    stage.add_system(say_system.system().after("look"));
    stage.add_system(script_attach_system.system());
    stage.add_system(script_detach_system.system());
    stage.add_system(send_system.system().after("look"));
    stage.add_system(shutdown_system.system());
    stage.add_system(stats_system.system());
    stage.add_system(teleport_system.system());
    stage.add_system(update_description_system.system());
    stage.add_system(update_keywords_system.system());
    stage.add_system(update_name_system.system());
    stage.add_system(update_object_flags.system());
    stage.add_system(who_system.system());
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?} has no {1}.")]
    MissingComponent(Entity, &'static str),
}
