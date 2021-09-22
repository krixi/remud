pub mod actions;
mod modules;

use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt,
    sync::{Arc, RwLock},
};

use bevy_app::{EventReader, EventWriter, Events};
use bevy_ecs::prelude::*;
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rhai::{plugin::*, ParseError, AST};
use strum::EnumString;
use thiserror::Error;

use crate::world::{
    action::Action,
    scripting::{
        actions::{run_post_event_script, run_pre_event_script},
        modules::{event_api, self_api, world_api},
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
    pub runs: Vec<(Action, Vec<ScriptRun>)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScriptName(String);

impl TryFrom<String> for ScriptName {
    type Error = ScriptNameParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.chars().all(|c| c.is_ascii() && !c.is_whitespace()) {
            Ok(ScriptName(value))
        } else {
            Err(ScriptNameParseError {})
        }
    }
}

impl fmt::Display for ScriptName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Error)]
#[error("Failed to parse script name: must be ASCII and contain no whitespace.")]
pub struct ScriptNameParseError {}

#[derive(Debug, Clone)]
pub struct Script {
    pub name: ScriptName,
    pub trigger: TriggerEvent,
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
    pub error: CompilationError,
}

#[derive(Clone)]
pub struct ScriptAst {
    pub ast: AST,
}

#[derive(Debug)]
pub struct CompilationError {
    pub error: ParseError,
}

#[derive(Debug)]
pub struct ScriptRun {
    pub entity: Entity,
    pub script: ScriptName,
}

#[derive(Debug)]
pub struct ScriptHooks {
    pub list: Vec<ScriptHook>,
}

impl ScriptHooks {
    pub fn remove(&mut self, hook: &ScriptHook) -> bool {
        if let Some(pos) = self.list.iter().position(|h| hook == h) {
            self.list.remove(pos);
            true
        } else {
            false
        }
    }

    fn by_trigger(&self, trigger: ScriptTrigger) -> Vec<ScriptName> {
        self.list
            .iter()
            .filter(|hook| hook.trigger == trigger)
            .map(|hook| hook.script.clone())
            .collect_vec()
    }
}

#[derive(Debug, PartialEq)]
pub struct ScriptHook {
    pub trigger: ScriptTrigger,
    pub script: ScriptName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptTrigger {
    PreEvent(TriggerEvent),
    PostEvent(TriggerEvent),
}

impl ScriptTrigger {
    pub fn kind(&self) -> TriggerKind {
        match self {
            ScriptTrigger::PreEvent(_) => TriggerKind::PreEvent,
            ScriptTrigger::PostEvent(_) => TriggerKind::PostEvent,
        }
    }

    pub fn trigger(&self) -> TriggerEvent {
        match self {
            ScriptTrigger::PreEvent(trigger) => *trigger,
            ScriptTrigger::PostEvent(trigger) => *trigger,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
pub enum TriggerKind {
    PreEvent,
    PostEvent,
}

impl TriggerKind {
    pub fn with_trigger(self, event: TriggerEvent) -> ScriptTrigger {
        match self {
            TriggerKind::PreEvent => ScriptTrigger::PreEvent(event),
            TriggerKind::PostEvent => ScriptTrigger::PostEvent(event),
        }
    }
}

impl fmt::Display for TriggerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriggerKind::PreEvent => write!(f, "PreEvent"),
            TriggerKind::PostEvent => write!(f, "PostEvent"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
pub enum TriggerEvent {
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

impl TriggerEvent {
    fn from_action(value: &Action) -> Option<Self> {
        match value {
            Action::Drop(_) => Some(TriggerEvent::Drop),
            Action::Emote(_) => Some(TriggerEvent::Emote),
            Action::Exits(_) => Some(TriggerEvent::Exits),
            Action::Get(_) => Some(TriggerEvent::Get),
            Action::Inventory(_) => Some(TriggerEvent::Inventory),
            Action::Login(_) => None,
            Action::Look(_) => Some(TriggerEvent::Look),
            Action::LookAt(_) => Some(TriggerEvent::LookAt),
            Action::Message(_) => None,
            Action::Move(_) => Some(TriggerEvent::Move),
            Action::ObjectCreate(_) => None,
            Action::ObjectInfo(_) => None,
            Action::ObjectRemove(_) => None,
            Action::ObjectUpdateDescription(_) => None,
            Action::ObjectUpdateFlags(_) => None,
            Action::ObjectUpdateKeywords(_) => None,
            Action::ObjectUpdateName(_) => None,
            Action::PlayerInfo(_) => None,
            Action::RoomCreate(_) => None,
            Action::RoomInfo(_) => None,
            Action::RoomLink(_) => None,
            Action::RoomRemove(_) => None,
            Action::RoomUnlink(_) => None,
            Action::RoomUpdateDescription(_) => None,
            Action::RoomUpdateRegions(_) => None,
            Action::Say(_) => Some(TriggerEvent::Say),
            Action::ScriptAttach(_) => None,
            Action::ScriptDetach(_) => None,
            Action::Send(_) => Some(TriggerEvent::Send),
            Action::Shutdown(_) => None,
            Action::Stats(_) => None,
            Action::Teleport(_) => None,
            Action::Who(_) => None,
        }
    }
}

impl fmt::Display for TriggerEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriggerEvent::Drop => write!(f, "Drop"),
            TriggerEvent::Emote => write!(f, "Emote"),
            TriggerEvent::Exits => write!(f, "Exits"),
            TriggerEvent::Get => write!(f, "Get"),
            TriggerEvent::Inventory => write!(f, "Inventory"),
            TriggerEvent::Look => write!(f, "Look"),
            TriggerEvent::LookAt => write!(f, "LookAt"),
            TriggerEvent::Move => write!(f, "Move"),
            TriggerEvent::Say => write!(f, "Say"),
            TriggerEvent::Send => write!(f, "Send"),
        }
    }
}

pub struct QueuedAction {
    pub action: Action,
}

impl QueuedAction {
    fn new(action: Action) -> Self {
        QueuedAction { action }
    }
}

pub fn script_compiler_system(
    mut commands: Commands,
    engine: Res<ScriptEngine>,
    uncompiled_scripts: Query<(Entity, &Script), (Without<ScriptAst>, Without<CompilationError>)>,
) {
    for (entity, script) in uncompiled_scripts.iter() {
        match engine.compile(script.code.as_str()) {
            Ok(ast) => {
                commands.entity(entity).insert(ScriptAst { ast });
            }
            Err(error) => {
                commands.entity(entity).insert(CompilationError { error });
            }
        }
    }
}

pub fn queued_action_script_system(
    mut queued_action_reader: EventReader<QueuedAction>,
    mut action_writer: EventWriter<Action>,
    mut script_runs: ResMut<ScriptRuns>,
    room_query: Query<&Room>,
    location_query: Query<&Location>,
    container_query: Query<&Container>,
    contents_query: Query<&Contents>,
    hooks_query: Query<&ScriptHooks>,
) {
    for QueuedAction { action } in queued_action_reader.iter() {
        let trigger_event = match TriggerEvent::from_action(action) {
            Some(trigger) => trigger,
            None => {
                action_writer.send(action.clone());
                continue;
            }
        };

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
        let runs = get_script_runs(
            ScriptTrigger::PreEvent(trigger_event),
            room,
            &hooks_query,
            &contents_query,
            &room_query,
        );

        if runs.is_empty() {
            action_writer.send(action.clone());
        } else {
            script_runs.runs.push((action.clone(), runs));
        }
    }
}

pub fn post_action_script_system(
    mut queued_action_reader: EventReader<QueuedAction>,
    mut script_runs: ResMut<ScriptRuns>,
    room_query: Query<&Room>,
    location_query: Query<&Location>,
    container_query: Query<&Container>,
    contents_query: Query<&Contents>,
    hooks_query: Query<&ScriptHooks>,
) {
    for QueuedAction { action } in queued_action_reader.iter() {
        let trigger_event = match TriggerEvent::from_action(action) {
            Some(trigger) => trigger,
            None => continue,
        };

        let enactor = action.enactor();

        let room = if let Some(room) =
            action_room(enactor, &room_query, &location_query, &container_query)
        {
            room
        } else {
            tracing::warn!("Unable to determine location of action {:?}", action);
            continue;
        };

        let runs = get_script_runs(
            ScriptTrigger::PostEvent(trigger_event),
            room,
            &hooks_query,
            &contents_query,
            &room_query,
        );

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
    engine.register_global_module(exported_module!(self_api).into());

    ScriptEngine {
        engine: Arc::new(RwLock::new(engine)),
    }
}

pub fn run_pre_action_scripts(world: Arc<RwLock<World>>) {
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
                run_pre_event_script(world.clone(), &event, entity, script)
            })
            .collect();

        if allowed.into_iter().all(|b| b) {
            world
                .write()
                .unwrap()
                .get_resource_mut::<Events<Action>>()
                .unwrap()
                .send(event);
        }
    });
}

pub fn run_post_action_scripts(world: Arc<RwLock<World>>) {
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
                run_post_event_script(world.clone(), &event, entity, script)
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

fn get_script_runs(
    trigger: ScriptTrigger,
    room: Entity,
    hooks_query: &Query<&ScriptHooks>,
    contents_query: &Query<&Contents>,
    room_query: &Query<&Room>,
) -> Vec<ScriptRun> {
    let mut runs = Vec::new();

    if let Ok(hooks) = hooks_query.get(room) {
        for script in hooks.by_trigger(trigger) {
            runs.push(ScriptRun {
                entity: room,
                script,
            });
        }
    }

    let contents = contents_query.get(room).unwrap();
    for object in &contents.objects {
        if let Ok(hooks) = hooks_query.get(*object) {
            for script in hooks.by_trigger(trigger) {
                runs.push(ScriptRun {
                    entity: *object,
                    script,
                });
            }
        }
    }

    let room = room_query.get(room).unwrap();
    for player in &room.players {
        if let Ok(hooks) = hooks_query.get(*player) {
            for script in hooks.by_trigger(trigger) {
                runs.push(ScriptRun {
                    entity: *player,
                    script,
                });
            }
        }

        let contents = contents_query.get(*player).unwrap();
        for object in contents.objects.iter() {
            if let Ok(hooks) = hooks_query.get(*object) {
                for script in hooks.by_trigger(trigger) {
                    runs.push(ScriptRun {
                        entity: *object,
                        script,
                    })
                }
            }
        }
    }

    runs
}
