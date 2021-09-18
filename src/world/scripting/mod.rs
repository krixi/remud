pub mod actions;
mod modules;

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
};

use bevy_app::{EventReader, EventWriter, Events};
use bevy_ecs::prelude::*;
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rhai::{plugin::*, ParseError, AST};
use strum::EnumString;

use crate::world::{
    action::ActionEvent,
    scripting::{
        actions::{run_pre_script, run_script},
        modules::{event_api, world_api},
    },
    types::{room::Room, Container, Contents, Location},
};

pub struct ScriptEngine {
    engine: Arc<RwLock<rhai::Engine>>,
}

impl ScriptEngine {
    pub fn compile(&self, script: &str) -> Result<rhai::AST, rhai::ParseError> {
        self.engine.read().unwrap().compile(script)
    }

    pub fn get(&self) -> Arc<RwLock<rhai::Engine>> {
        self.engine.clone()
    }
}

#[derive(Default)]
pub struct Scripts {
    by_name: HashMap<ScriptName, Entity>,
}

impl Scripts {
    pub fn insert(&mut self, name: ScriptName, script: Entity) {
        self.by_name.insert(name, script);
    }

    pub fn remove(&mut self, name: &ScriptName) {
        self.by_name.remove(name);
    }

    pub fn by_name(&self, name: &ScriptName) -> Option<Entity> {
        self.by_name.get(name).copied()
    }
}

#[derive(Default, Debug)]
pub struct ScriptRuns {
    pub runs: Vec<(ActionEvent, Vec<ScriptRun>)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScriptName(String);

impl From<&str> for ScriptName {
    fn from(value: &str) -> Self {
        ScriptName(value.to_string())
    }
}

impl fmt::Display for ScriptName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Script {
    pub name: ScriptName,
    pub trigger: Trigger,
    pub code: String,
}

#[derive(Bundle)]
pub struct CompiledScript {
    pub script: Script,
    pub ast: ScriptAst,
}

#[derive(Bundle)]
pub struct FailedScript {
    pub script: Script,
    pub error: ScriptError,
}

#[derive(Clone)]
pub struct ScriptAst {
    pub ast: AST,
}

#[derive(Debug)]
pub struct ScriptError {
    pub error: ParseError,
}

#[derive(Debug)]
pub struct ScriptRun {
    pub entity: Entity,
    pub script: ScriptName,
}

#[derive(Debug)]
pub struct PostEventScriptHooks {
    pub list: Vec<ScriptHook>,
}

#[derive(Debug)]
pub struct PreEventScriptHooks {
    pub list: Vec<ScriptHook>,
}

trait ScriptHooks: 'static + Send + Sync {
    fn triggered_by(&self, action_trigger: Trigger) -> Vec<ScriptName>;
}

impl ScriptHooks for PreEventScriptHooks {
    fn triggered_by(&self, action_trigger: Trigger) -> Vec<ScriptName> {
        self.list
            .iter()
            .filter(|hook| hook.trigger == action_trigger)
            .map(|hook| &hook.script)
            .cloned()
            .collect_vec()
    }
}

impl ScriptHooks for PostEventScriptHooks {
    fn triggered_by(&self, action_trigger: Trigger) -> Vec<ScriptName> {
        self.list
            .iter()
            .filter(|hook| hook.trigger == action_trigger)
            .map(|hook| &hook.script)
            .cloned()
            .collect_vec()
    }
}

#[derive(Debug)]
pub struct ScriptHook {
    pub trigger: Trigger,
    pub script: ScriptName,
}

impl ScriptHook {
    pub fn new(trigger: Trigger, script: &str) -> Self {
        ScriptHook {
            trigger,
            script: ScriptName::from(script),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
pub enum Trigger {
    Drop,
    Emote,
    Exits,
    Get,
    Inventory,
    Look,
    LookAt,
    Move,
    Say,
    Send,
}

impl Trigger {
    fn from_action(value: &ActionEvent) -> Option<Self> {
        match value {
            ActionEvent::Drop(_) => Some(Trigger::Drop),
            ActionEvent::Emote(_) => Some(Trigger::Emote),
            ActionEvent::Exits(_) => Some(Trigger::Exits),
            ActionEvent::Get(_) => Some(Trigger::Get),
            ActionEvent::Inventory(_) => Some(Trigger::Inventory),
            ActionEvent::Login(_) => None,
            ActionEvent::Logout(_) => None,
            ActionEvent::Look(_) => Some(Trigger::Look),
            ActionEvent::LookAt(_) => Some(Trigger::LookAt),
            ActionEvent::Move(_) => Some(Trigger::Move),
            ActionEvent::ObjectUnsetFlags(_) => None,
            ActionEvent::ObjectCreate(_) => None,
            ActionEvent::ObjectInfo(_) => None,
            ActionEvent::ObjectRemove(_) => None,
            ActionEvent::ObjectSetFlags(_) => None,
            ActionEvent::ObjectUpdateDescription(_) => None,
            ActionEvent::ObjectUpdateKeywords(_) => None,
            ActionEvent::ObjectUpdateName(_) => None,
            ActionEvent::PlayerInfo(_) => None,
            ActionEvent::RoomCreate(_) => None,
            ActionEvent::RoomInfo(_) => None,
            ActionEvent::RoomLink(_) => None,
            ActionEvent::RoomUpdateDescription(_) => None,
            ActionEvent::RoomRemove(_) => None,
            ActionEvent::RoomUnlink(_) => None,
            ActionEvent::Say(_) => Some(Trigger::Say),
            ActionEvent::Send(_) => Some(Trigger::Send),
            ActionEvent::Shutdown(_) => None,
            ActionEvent::Teleport(_) => None,
            ActionEvent::Who(_) => None,
        }
    }
}

impl fmt::Display for Trigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Trigger::Drop => write!(f, "Drop"),
            Trigger::Emote => write!(f, "Emote"),
            Trigger::Exits => write!(f, "Exits"),
            Trigger::Get => write!(f, "Get"),
            Trigger::Inventory => write!(f, "Inventory"),
            Trigger::Look => write!(f, "Look"),
            Trigger::LookAt => write!(f, "LookAt"),
            Trigger::Move => write!(f, "Move"),
            Trigger::Say => write!(f, "Say"),
            Trigger::Send => write!(f, "Send"),
        }
    }
}

pub struct PreAction {
    pub action: ActionEvent,
}

impl PreAction {
    fn new(action: ActionEvent) -> Self {
        PreAction { action }
    }
}

pub fn script_compiler_system(
    mut commands: Commands,
    engine: Res<ScriptEngine>,
    uncompiled_scripts: Query<(Entity, &Script), (Without<ScriptAst>, Without<ScriptError>)>,
) {
    for (entity, script) in uncompiled_scripts.iter() {
        match engine.compile(script.code.as_str()) {
            Ok(ast) => {
                tracing::info!("Compiled {:?}.", script.name);
                commands.entity(entity).insert(ScriptAst { ast });
            }
            Err(error) => {
                tracing::warn!("Failed to compile {:?}: {}.", script.name, error);
                commands.entity(entity).insert(ScriptError { error });
            }
        }
    }
}

pub fn pre_action_script_system(
    mut pre_action_reader: EventReader<PreAction>,
    mut action_writer: EventWriter<ActionEvent>,
    mut script_runs: ResMut<ScriptRuns>,
    room_query: Query<&Room>,
    location_query: Query<&Location>,
    container_query: Query<&Container>,
    contents_query: Query<&Contents>,
    hooks_query: Query<&PreEventScriptHooks>,
) {
    for PreAction { action } in pre_action_reader.iter() {
        let enactor = action.enactor();

        // Determine the location the action took place. If we can't, we give the action a pass.
        let room = if let Some(room) =
            action_room(enactor, &room_query, &location_query, &container_query)
        {
            room
        } else {
            tracing::warn!("Unable to determine location of action {:?}", action);
            action_writer.send(action.clone());
            continue;
        };

        // Check if any scripts need to run for this action
        let runs = get_script_runs(room, action, &hooks_query, &contents_query, &room_query);
        if runs.is_empty() {
            action_writer.send(action.clone());
        } else {
            script_runs.runs.push((action.clone(), runs));
        }
    }
}

pub fn post_action_script_system(
    mut pre_action_reader: EventReader<PreAction>,
    mut script_runs: ResMut<ScriptRuns>,
    room_query: Query<&Room>,
    location_query: Query<&Location>,
    container_query: Query<&Container>,
    contents_query: Query<&Contents>,
    hooks_query: Query<&PostEventScriptHooks>,
) {
    for PreAction { action } in pre_action_reader.iter() {
        let enactor = action.enactor();

        let room = if let Some(room) =
            action_room(enactor, &room_query, &location_query, &container_query)
        {
            room
        } else {
            tracing::warn!("Unable to determine location of action {:?}", action);
            continue;
        };

        let runs = get_script_runs(room, action, &hooks_query, &contents_query, &room_query);

        if !runs.is_empty() {
            script_runs.runs.push((action.clone(), runs));
        }
    }
}

pub fn create_script_engine() -> ScriptEngine {
    let mut engine = rhai::Engine::default();

    engine.register_type_with_name::<Arc<RwLock<World>>>("World");
    engine.register_global_module(exported_module!(world_api).into());
    engine.register_global_module(exported_module!(event_api).into());

    ScriptEngine {
        engine: Arc::new(RwLock::new(engine)),
    }
}

pub fn run_pre_event_scripts(world: Arc<RwLock<World>>) {
    let mut runs = Vec::new();
    std::mem::swap(
        &mut runs,
        &mut world
            .write()
            .unwrap()
            .get_resource_mut::<ScriptRuns>()
            .unwrap()
            .runs,
    );

    runs.into_par_iter().for_each(|(event, runs)| {
        let allowed: Vec<bool> = runs
            .into_par_iter()
            .map(|ScriptRun { entity, script }| {
                run_pre_script(world.clone(), &event, entity, script)
            })
            .collect();

        if allowed.into_iter().all(|b| b) {
            world
                .write()
                .unwrap()
                .get_resource_mut::<Events<ActionEvent>>()
                .unwrap()
                .send(event);
        }
    });
}

pub fn run_event_scripts(world: Arc<RwLock<World>>) {
    let mut runs = Vec::new();
    std::mem::swap(
        &mut runs,
        &mut world
            .write()
            .unwrap()
            .get_resource_mut::<ScriptRuns>()
            .unwrap()
            .runs,
    );

    runs.into_par_iter().for_each(|(event, runs)| {
        runs.into_par_iter()
            .for_each(|ScriptRun { entity, script }| {
                run_script(world.clone(), &event, entity, script)
            });
    });
}

fn action_room(
    enactor: Entity,
    room_query: &Query<&Room>,
    location_query: &Query<&Location>,
    container_query: &Query<&Container>,
) -> Option<Entity> {
    if let Ok(location) = location_query.get(enactor) {
        Some(location.room)
    } else if room_query.get(enactor).is_ok() {
        Some(enactor)
    } else {
        let mut containing_entity = enactor;

        while let Ok(container) = container_query.get(containing_entity) {
            containing_entity = container.entity;
        }

        location_query
            .get(containing_entity)
            .map(|location| location.room)
            .ok()
    }
}

fn get_script_runs<Hooks: ScriptHooks>(
    room: Entity,
    action: &ActionEvent,
    hooks_query: &Query<&Hooks>,
    contents_query: &Query<&Contents>,
    room_query: &Query<&Room>,
) -> Vec<ScriptRun> {
    let mut runs = Vec::new();

    if let Some(action_trigger) = Trigger::from_action(action) {
        if let Ok(triggers) = hooks_query.get(room) {
            for script in triggers.triggered_by(action_trigger) {
                runs.push(ScriptRun {
                    entity: room,
                    script,
                });
            }
        }

        let contents = contents_query.get(room).unwrap();
        for object in &contents.objects {
            if let Ok(triggers) = hooks_query.get(*object) {
                for script in triggers.triggered_by(action_trigger) {
                    runs.push(ScriptRun {
                        entity: *object,
                        script,
                    });
                }
            }
        }

        let room = room_query.get(room).unwrap();
        for player in &room.players {
            if let Ok(triggers) = hooks_query.get(*player) {
                for script in triggers.triggered_by(action_trigger) {
                    runs.push(ScriptRun {
                        entity: *player,
                        script,
                    });
                }
            }

            let contents = contents_query.get(*player).unwrap();
            for object in contents.objects.iter() {
                if let Ok(triggers) = hooks_query.get(*object) {
                    for script in triggers.triggered_by(action_trigger) {
                        runs.push(ScriptRun {
                            entity: *object,
                            script,
                        })
                    }
                }
            }
        }
    }

    runs
}
