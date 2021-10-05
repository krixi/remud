pub mod attributes;
pub mod commands;
pub mod communicate;
pub mod immortal;
pub mod movement;
pub mod object;
pub mod observe;
pub mod system;

use bevy_ecs::prelude::*;
use strum::EnumString;

use crate::{
    ecs::{Ecs, Phase, Plugin, Step},
    world::{
        action::{
            attributes::{stats_system, Stats},
            communicate::{
                emote_system, message_system, say_system, send_system, Emote, Message, Say,
                SendMessage,
            },
            immortal::{
                initialize_system,
                object::{
                    object_create_system, object_info_system, object_inherit_fields_system,
                    object_remove_system, update_keywords_system, update_object_flags,
                    ObjectCreate, ObjectInfo, ObjectInheritFields, ObjectRemove, UpdateKeywords,
                    UpdateObjectFlags,
                },
                player::{
                    player_info_system, player_update_flags_system, PlayerInfo, PlayerUpdateFlags,
                },
                prototype::{
                    prototype_create_system, prototype_info_system, PrototypeCreate, PrototypeInfo,
                    PrototypeList,
                },
                room::{
                    room_create_system, room_info_system, room_link_system, room_remove_system,
                    room_unlink_system, room_update_regions_system, RoomCreate, RoomInfo, RoomLink,
                    RoomRemove, RoomUnlink, RoomUpdateRegions,
                },
                script::{script_attach_system, script_detach_system, ScriptAttach, ScriptDetach},
                show_error_system, update_description_system, update_name_system, Initialize,
                ShowError, UpdateDescription, UpdateName,
            },
            movement::{move_system, teleport_system, Move, Teleport},
            object::{
                drop_system, get_system, inventory_system, use_system, Drop, Get, Inventory, Use,
            },
            observe::{
                exits_system, look_at_system, look_system, who_system, Exits, Look, LookAt, Who,
            },
            system::{login_system, restart_system, shutdown_system, Login, Restart, Shutdown},
        },
        scripting::QueuedAction,
    },
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString)]
pub enum Mode {
    #[strum(serialize = "add")]
    Add,
    #[strum(serialize = "remove")]
    Remove,
    #[strum(serialize = "set")]
    Set,
}

macro_rules! into_action {
    ($action:tt) => {
        impl From<$action> for Action {
            fn from(value: $action) -> Self {
                Action::$action(value)
            }
        }
    };
}

use crate::world::action::immortal::prototype::prototype_list_system;
pub(crate) use into_action;

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
    PrototypeList(PrototypeList),
    Restart(Restart),
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
    ShowError(ShowError),
    Shutdown(Shutdown),
    Stats(Stats),
    Teleport(Teleport),
    UpdateDescription(UpdateDescription),
    UpdateKeywords(UpdateKeywords),
    UpdateName(UpdateName),
    UpdateObjectFlags(UpdateObjectFlags),
    Use(Use),
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
            Action::PrototypeList(action) => action.actor,
            Action::Restart(action) => action.actor,
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
            Action::ShowError(action) => action.actor,
            Action::Shutdown(action) => action.actor,
            Action::Stats(action) => action.actor,
            Action::Teleport(action) => action.actor,
            Action::UpdateDescription(action) => action.actor,
            Action::UpdateKeywords(action) => action.actor,
            Action::UpdateName(action) => action.actor,
            Action::UpdateObjectFlags(action) => action.actor,
            Action::Use(action) => action.actor,
            Action::Who(action) => action.actor,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, SystemLabel)]
pub enum ActionSystem {
    Drop,
    Emote,
    Exits,
    Get,
    Initialize,
    Inventory,
    Login,
    Look,
    LookAt,
    Message,
    Move,
    ObjectCreate,
    ObjectInfo,
    ObjectInheritFields,
    ObjectRemove,
    PlayerInfo,
    PlayerUpdateFlags,
    PrototypeCreate,
    PrototypeInfo,
    PrototypeList,
    Restart,
    RoomCreate,
    RoomInfo,
    RoomLink,
    RoomRemove,
    RoomUnlink,
    RoomUpdateRegions,
    Say,
    ScriptAttach,
    ScriptDetach,
    Send,
    ShowError,
    Shutdown,
    Stats,
    Teleport,
    UpdateDescription,
    UpdateKeywords,
    UpdateName,
    UpdateObjectFlags,
    Use,
    Who,
}

#[derive(Default)]
pub struct ActionsPlugin {}

impl Plugin for ActionsPlugin {
    fn build(&self, ecs: &mut Ecs) {
        ecs.add_event::<QueuedAction>()
            .add_event::<Action>()
            .add_system(
                Step::Main,
                Phase::Update,
                drop_system.system().label(ActionSystem::Drop),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                emote_system
                    .system()
                    .label(ActionSystem::Emote)
                    .after(ActionSystem::Look),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                exits_system.system().label(ActionSystem::Exits),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                get_system.system().label(ActionSystem::Get),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                initialize_system.system().label(ActionSystem::Initialize),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                inventory_system.system().label(ActionSystem::Inventory),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                login_system.system().label(ActionSystem::Login),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                look_at_system.system().label(ActionSystem::LookAt),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                look_system.system().label(ActionSystem::Look),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                message_system.system().label(ActionSystem::Message),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                move_system.system().label(ActionSystem::Move),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                object_create_system
                    .system()
                    .label(ActionSystem::ObjectCreate),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                object_info_system.system().label(ActionSystem::ObjectInfo),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                object_inherit_fields_system
                    .system()
                    .label(ActionSystem::ObjectInheritFields),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                object_remove_system
                    .system()
                    .label(ActionSystem::ObjectRemove),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                player_info_system.system().label(ActionSystem::PlayerInfo),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                player_update_flags_system
                    .system()
                    .label(ActionSystem::PlayerUpdateFlags),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                prototype_create_system
                    .system()
                    .label(ActionSystem::PrototypeCreate),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                prototype_info_system
                    .system()
                    .label(ActionSystem::PrototypeInfo),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                prototype_list_system
                    .system()
                    .label(ActionSystem::PrototypeList),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                restart_system.system().label(ActionSystem::Restart),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                room_create_system.system().label(ActionSystem::RoomCreate),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                room_info_system.system().label(ActionSystem::RoomInfo),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                room_link_system.system().label(ActionSystem::RoomLink),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                room_remove_system.system().label(ActionSystem::RoomRemove),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                room_unlink_system.system().label(ActionSystem::RoomUnlink),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                room_update_regions_system
                    .system()
                    .label(ActionSystem::RoomUpdateRegions),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                say_system
                    .system()
                    .label(ActionSystem::Say)
                    .after(ActionSystem::Look),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                script_attach_system
                    .system()
                    .label(ActionSystem::ScriptAttach),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                script_detach_system
                    .system()
                    .label(ActionSystem::ScriptDetach),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                send_system
                    .system()
                    .label(ActionSystem::Send)
                    .after(ActionSystem::Look),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                show_error_system.system().label(ActionSystem::ShowError),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                shutdown_system.system().label(ActionSystem::Shutdown),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                stats_system.system().label(ActionSystem::Stats),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                teleport_system.system().label(ActionSystem::Teleport),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                update_description_system
                    .system()
                    .label(ActionSystem::UpdateDescription),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                update_keywords_system
                    .system()
                    .label(ActionSystem::UpdateKeywords),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                update_name_system.system().label(ActionSystem::UpdateName),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                update_object_flags
                    .system()
                    .label(ActionSystem::UpdateObjectFlags),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                use_system.system().label(ActionSystem::Use),
            )
            .add_system(
                Step::Main,
                Phase::Update,
                who_system.system().label(ActionSystem::Who),
            );
    }
}
