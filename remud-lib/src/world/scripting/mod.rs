pub mod actions;
pub mod execution;
mod modules;
mod systems;
pub mod time;

use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt,
    sync::{Arc, RwLock},
};

use bevy_app::Events;
use bevy_ecs::prelude::*;
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rhai::{plugin::*, ParseError, AST};
use strum::EnumString;
use thiserror::Error;

use crate::{
    ecs::{CoreSystem, Ecs, Phase, Plugin, SharedWorld, Step},
    world::{
        action::Action,
        fsm::{StateId, StateMachineBuilder, Transition},
        scripting::{
            execution::{
                run_init_script, run_post_event_script, run_pre_event_script, run_timed_script,
                SharedEngine,
            },
            modules::{
                event_api, rand_api, self_api, states_api, time_api, transitions_api, world_api,
            },
            systems::{
                init_script_runs_system, post_action_script_runs_system,
                pre_event_script_runs_system, timed_script_runs_system,
            },
            time::{tick_timers_system, timed_actions_system, timer_cleanup_system, TimedActions},
        },
    },
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, SystemLabel)]
pub enum ScriptSystem {
    InitScriptRuns,
    PostActionScriptRuns,
    PreEventScriptRuns,
    TickTimers,
    TimedActions,
    TimerCleanup,
    TimedScriptRuns,
}

#[derive(Default)]
pub struct ScriptPlugin {}

impl Plugin for ScriptPlugin {
    fn build(&self, ecs: &mut Ecs) {
        ecs.init_resource::<ScriptRuns>()
            .init_resource::<TimedActions>()
            .init_resource::<ScriptEngine>()
            .add_event::<RunInitScript>()
            .add_system(
                Step::PreEvent,
                Phase::First,
                timer_cleanup_system
                    .system()
                    .label(ScriptSystem::TimerCleanup)
                    .before(ScriptSystem::TickTimers),
            )
            .add_system(
                Step::PreEvent,
                Phase::First,
                tick_timers_system
                    .system()
                    .label(ScriptSystem::TickTimers)
                    .after(CoreSystem::Time),
            )
            .add_system(
                Step::PreEvent,
                Phase::Update,
                timed_actions_system
                    .system()
                    .label(ScriptSystem::TimedActions)
                    .before(ScriptSystem::PreEventScriptRuns),
            )
            .add_system(
                Step::PreEvent,
                Phase::Update,
                timed_script_runs_system
                    .system()
                    .label(ScriptSystem::TimedScriptRuns)
                    .before(ScriptSystem::PreEventScriptRuns),
            )
            .add_system(
                Step::PreEvent,
                Phase::Update,
                init_script_runs_system
                    .system()
                    .label(ScriptSystem::InitScriptRuns),
            )
            .add_system(
                Step::PreEvent,
                Phase::Update,
                pre_event_script_runs_system
                    .system()
                    .label(ScriptSystem::PreEventScriptRuns),
            )
            .add_system(
                Step::PostEvent,
                Phase::Update,
                post_action_script_runs_system
                    .system()
                    .label(ScriptSystem::PostActionScriptRuns),
            );
    }
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

#[derive(Debug, Clone)]
pub struct Script {
    name: ScriptName,
    trigger: TriggerEvent,
    code: String,
}

impl Script {
    pub fn new(name: ScriptName, trigger: TriggerEvent, code: String) -> Self {
        Script {
            name,
            trigger,
            code,
        }
    }

    pub fn name(&self) -> &ScriptName {
        &self.name
    }

    pub fn trigger(&self) -> TriggerEvent {
        self.trigger
    }

    pub fn code(&self) -> String {
        self.code.clone()
    }

    pub fn as_str(&self) -> &str {
        self.code.as_str()
    }

    pub fn into_parts(self) -> (ScriptName, TriggerEvent, String) {
        (self.name, self.trigger, self.code)
    }
}

#[derive(Clone)]
pub struct ScriptAst {
    ast: AST,
}

impl From<AST> for ScriptAst {
    fn from(ast: AST) -> Self {
        ScriptAst { ast }
    }
}

#[derive(Debug)]
pub struct CompilationError {
    error: ParseError,
}

impl From<ParseError> for CompilationError {
    fn from(error: ParseError) -> Self {
        CompilationError { error }
    }
}

pub struct ScriptEngine {
    engine: SharedEngine,
}

impl ScriptEngine {
    pub fn get(&self) -> SharedEngine {
        self.engine.clone()
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        let mut engine = rhai::Engine::default();

        engine.register_type_with_name::<SharedWorld>("World");

        engine.register_type_with_name::<StateMachineBuilder>("StateMachineBuilder");
        engine.register_fn("fsm_builder", StateMachineBuilder::default);
        engine.register_fn("add_state", StateMachineBuilder::add_state);

        engine.register_type_with_name::<StateId>("StateId");
        engine.register_static_module("StateId", exported_module!(states_api).into());
        engine.register_type_with_name::<Transition>("Transition");
        engine.register_static_module("Transition", exported_module!(transitions_api).into());

        engine.register_global_module(exported_module!(world_api).into());
        engine.register_global_module(exported_module!(event_api).into());
        engine.register_global_module(exported_module!(self_api).into());
        engine.register_global_module(exported_module!(time_api).into());
        engine.register_global_module(exported_module!(rand_api).into());

        ScriptEngine {
            engine: Arc::new(RwLock::new(engine)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScriptName(String);

impl ScriptName {
    pub fn into_string(self) -> String {
        self.0
    }
}

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
    init_runs: Vec<ScriptRun>,
    runs: Vec<(Action, Vec<ScriptRun>)>,
    timed_runs: Vec<ScriptRun>,
}

impl ScriptRuns {
    pub fn queue_init(&mut self, run: ScriptRun) {
        self.init_runs.push(run);
    }
}

#[derive(Debug)]
pub struct ScriptRun {
    pub entity: Entity,
    pub script: ScriptName,
}

impl ScriptRun {
    pub fn new(entity: Entity, script: ScriptName) -> Self {
        ScriptRun { entity, script }
    }
}

#[derive(Default)]
pub struct ExecutionErrors {
    errors: HashMap<ScriptName, Box<EvalAltResult>>,
}

impl ExecutionErrors {
    pub fn new_with_error(script: ScriptName, error: Box<EvalAltResult>) -> Self {
        let mut errors = ExecutionErrors::default();
        errors.insert(script, error);
        errors
    }

    pub fn has_error(&self, script: &ScriptName) -> bool {
        self.errors.contains_key(script)
    }

    pub fn get(&self, script: &ScriptName) -> Option<&EvalAltResult> {
        self.errors.get(script).map(|e| e.as_ref())
    }

    pub fn insert(&mut self, script: ScriptName, error: Box<EvalAltResult>) {
        self.errors.insert(script, error);
    }
}

#[derive(Debug, Default, Clone)]
pub struct ScriptHooks {
    list: Vec<ScriptHook>,
}

impl ScriptHooks {
    pub fn new(hook: ScriptHook) -> Self {
        ScriptHooks { list: vec![hook] }
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn hooks(&self) -> &[ScriptHook] {
        self.list.as_slice()
    }

    pub fn contains(&self, hook: &ScriptHook) -> bool {
        self.list.contains(hook)
    }

    pub fn insert(&mut self, hook: ScriptHook) {
        self.list.push(hook)
    }

    pub fn remove(&mut self, script: &ScriptName) -> Option<ScriptHook> {
        if let Some(pos) = self.list.iter().position(|h| &h.script == script) {
            Some(self.list.remove(pos))
        } else {
            None
        }
    }

    pub fn by_trigger(&self, trigger: ScriptTrigger) -> Vec<ScriptName> {
        self.list
            .iter()
            .filter(|hook| hook.trigger == trigger)
            .map(|hook| hook.script.clone())
            .collect_vec()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ScriptHook {
    pub trigger: ScriptTrigger,
    pub script: ScriptName,
}

#[derive(Default, Clone)]
pub struct ScriptData {
    map: HashMap<ImmutableString, Dynamic>,
}

impl ScriptData {
    pub fn new_with_entry(key: ImmutableString, value: Dynamic) -> Self {
        let mut map = HashMap::new();
        map.insert(key, value);
        ScriptData { map }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn get(&self, key: ImmutableString) -> Dynamic {
        self.map.get(&key).cloned().unwrap_or(Dynamic::UNIT)
    }

    pub fn map(&self) -> &HashMap<ImmutableString, Dynamic> {
        &self.map
    }

    pub fn insert(&mut self, key: ImmutableString, value: Dynamic) {
        self.map.insert(key, value);
    }

    pub fn remove(&mut self, key: ImmutableString) -> Dynamic {
        self.map.remove(&key).unwrap_or(Dynamic::UNIT)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptTrigger {
    Init,
    PostEvent(TriggerEvent),
    PreEvent(TriggerEvent),
    Timer(String),
}

impl ScriptTrigger {
    pub fn kind(&self) -> TriggerKind {
        match self {
            ScriptTrigger::Init => TriggerKind::Init,
            ScriptTrigger::PostEvent(_) => TriggerKind::PostEvent,
            ScriptTrigger::PreEvent(_) => TriggerKind::PreEvent,
            ScriptTrigger::Timer(_) => TriggerKind::Timer,
        }
    }
}

impl ToString for ScriptTrigger {
    fn to_string(&self) -> String {
        match self {
            ScriptTrigger::Init => String::new(),
            ScriptTrigger::PostEvent(event) => event.to_string(),
            ScriptTrigger::PreEvent(event) => event.to_string(),
            ScriptTrigger::Timer(name) => name.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString)]
pub enum TriggerKind {
    PreEvent,
    PostEvent,
    Init,
    Timer,
}

impl fmt::Display for TriggerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriggerKind::Init => write!(f, "Init"),
            TriggerKind::PostEvent => write!(f, "PostEvent"),
            TriggerKind::PreEvent => write!(f, "PreEvent"),
            TriggerKind::Timer => write!(f, "Timer"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
pub enum TriggerEvent {
    Drop,
    Emote,
    Exits,
    Get,
    Init,
    Inventory,
    Look,
    LookAt,
    Move,
    Say,
    Send,
    Timer,
}

impl TriggerEvent {
    fn from_action(value: &Action) -> Option<Self> {
        match value {
            Action::Drop(_) => Some(TriggerEvent::Drop),
            Action::Emote(_) => Some(TriggerEvent::Emote),
            Action::Exits(_) => Some(TriggerEvent::Exits),
            Action::Get(_) => Some(TriggerEvent::Get),
            Action::Initialize(_) => None,
            Action::Inventory(_) => Some(TriggerEvent::Inventory),
            Action::Login(_) => None,
            Action::Look(_) => Some(TriggerEvent::Look),
            Action::LookAt(_) => Some(TriggerEvent::LookAt),
            Action::Message(_) => None,
            Action::Move(_) => Some(TriggerEvent::Move),
            Action::ObjectCreate(_) => None,
            Action::ObjectInfo(_) => None,
            Action::ObjectInheritFields(_) => None,
            Action::ObjectRemove(_) => None,
            Action::PlayerInfo(_) => None,
            Action::PlayerUpdateFlags(_) => None,
            Action::PrototypeCreate(_) => None,
            Action::PrototypeInfo(_) => None,
            Action::Restart(_) => None,
            Action::RoomCreate(_) => None,
            Action::RoomInfo(_) => None,
            Action::RoomLink(_) => None,
            Action::RoomRemove(_) => None,
            Action::RoomUnlink(_) => None,
            Action::RoomUpdateRegions(_) => None,
            Action::Say(_) => Some(TriggerEvent::Say),
            Action::ScriptAttach(_) => None,
            Action::ScriptDetach(_) => None,
            Action::Send(_) => Some(TriggerEvent::Send),
            Action::ShowError(_) => None,
            Action::Shutdown(_) => None,
            Action::Stats(_) => None,
            Action::Teleport(_) => None,
            Action::UpdateDescription(_) => None,
            Action::UpdateKeywords(_) => None,
            Action::UpdateName(_) => None,
            Action::UpdateObjectFlags(_) => None,
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
            TriggerEvent::Init => write!(f, "Init"),
            TriggerEvent::Inventory => write!(f, "Inventory"),
            TriggerEvent::Look => write!(f, "Look"),
            TriggerEvent::LookAt => write!(f, "LookAt"),
            TriggerEvent::Move => write!(f, "Move"),
            TriggerEvent::Say => write!(f, "Say"),
            TriggerEvent::Send => write!(f, "Send"),
            TriggerEvent::Timer => write!(f, "Timer"),
        }
    }
}

pub struct QueuedAction {
    pub action: Action,
}

impl From<Action> for QueuedAction {
    fn from(action: Action) -> Self {
        QueuedAction { action }
    }
}

pub struct RunInitScript {
    pub entity: Entity,
    pub script: ScriptName,
}

impl RunInitScript {
    pub fn new(entity: Entity, script: ScriptName) -> Self {
        RunInitScript { entity, script }
    }
}

pub fn run_init_scripts(world: SharedWorld) {
    let mut runs = Vec::new();
    std::mem::swap(
        &mut runs,
        &mut world
            .write()
            .unwrap()
            .get_resource_mut::<ScriptRuns>()
            .unwrap()
            .init_runs,
    );

    runs.into_par_iter()
        .for_each(|ScriptRun { entity, script }| run_init_script(world.clone(), entity, script))
}

pub fn run_pre_action_scripts(world: SharedWorld) {
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

    runs.into_par_iter().for_each(|(action, runs)| {
        let allowed: Vec<bool> = runs
            .into_par_iter()
            .map(|ScriptRun { entity, script }| {
                run_pre_event_script(world.clone(), &action, entity, script)
            })
            .collect();

        if allowed.into_iter().all(|allowed| allowed) {
            world
                .write()
                .unwrap()
                .get_resource_mut::<Events<Action>>()
                .unwrap()
                .send(action);
        }
    });
}

pub fn run_post_action_scripts(world: SharedWorld) {
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

    runs.into_par_iter().for_each(|(action, runs)| {
        runs.into_par_iter()
            .for_each(|ScriptRun { entity, script }| {
                run_post_event_script(world.clone(), &action, entity, script);
            });
    });
}

pub fn run_timed_scripts(world: SharedWorld) {
    let mut runs = Vec::new();
    std::mem::swap(
        &mut runs,
        &mut world
            .write()
            .unwrap()
            .get_resource_mut::<ScriptRuns>()
            .unwrap()
            .timed_runs,
    );

    runs.into_par_iter()
        .for_each(|ScriptRun { entity, script }| run_timed_script(world.clone(), entity, script))
}
