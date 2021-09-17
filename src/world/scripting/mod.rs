use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use bevy_app::{EventReader, EventWriter};
use bevy_ecs::prelude::*;
use itertools::Itertools;
use rhai::{plugin::*, AST};
use strum::EnumString;

use crate::world::{
    action::ActionEvent,
    types::{room::Room, Container, Contents, Location},
};

pub struct ScriptEngine {
    engine: Arc<RwLock<rhai::Engine>>,
}

impl ScriptEngine {
    pub fn compile(&self, script: &str) -> anyhow::Result<rhai::AST> {
        Ok(self.engine.read().unwrap().compile(script)?)
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

impl ToString for ScriptName {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Script {
    pub name: ScriptName,
    pub trigger: Trigger,
    pub code: String,
}

#[derive(Bundle)]
pub struct ScriptArtifacts {
    compiled: CompiledScript,
    error: CompilationError,
}

pub struct CompiledScript {
    pub ast: AST,
}

#[derive(Debug)]
pub struct CompilationError {
    pub error: String,
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
    trigger: Trigger,
    script: ScriptName,
}

impl ScriptHook {
    pub fn new(trigger: Trigger, script: &str) -> Self {
        ScriptHook {
            trigger,
            script: ScriptName::from(script),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::ToString, EnumString)]
pub enum Trigger {
    Say,
}

impl Trigger {
    fn from_action(value: &ActionEvent) -> Option<Self> {
        match value {
            ActionEvent::Drop(_) => None,
            ActionEvent::Emote(_) => None,
            ActionEvent::Exits(_) => None,
            ActionEvent::Get(_) => None,
            ActionEvent::Inventory(_) => None,
            ActionEvent::Login(_) => None,
            ActionEvent::Logout(_) => None,
            ActionEvent::Look(_) => None,
            ActionEvent::LookAt(_) => None,
            ActionEvent::Move(_) => None,
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
            ActionEvent::Send(_) => None,
            ActionEvent::Shutdown(_) => None,
            ActionEvent::Teleport(_) => None,
            ActionEvent::Who(_) => None,
        }
    }
}

pub struct PreAction {
    pub action: ActionEvent,
}

pub fn script_compiler_system(
    mut commands: Commands,
    engine: Res<ScriptEngine>,
    uncompiled_scripts: Query<
        (Entity, &Script),
        (Without<CompiledScript>, Without<CompilationError>),
    >,
) {
    for (entity, script) in uncompiled_scripts.iter() {
        match engine.compile(script.code.as_str()) {
            Ok(ast) => {
                tracing::info!("Compiled {:?}.", script.name);
                commands.entity(entity).insert(CompiledScript { ast });
            }
            Err(e) => {
                let error = e.to_string();
                tracing::warn!("Failed to compile {:?}: {}.", script.name, error);
                commands.entity(entity).insert(CompilationError { error });
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
        }

        // TODO: add checks for all objects in present player's inventories
    }

    runs
}

#[export_module]
pub mod event_api {
    use crate::world::action::ActionEvent;

    #[rhai_fn(get = "entity", pure)]
    pub fn get_entity(action_event: &mut ActionEvent) -> Dynamic {
        Dynamic::from(action_event.enactor())
    }
}

#[export_module]
pub mod world_api {
    use std::sync::RwLock;

    use rhai::Dynamic;

    use crate::world::types::Named;

    #[rhai_fn(pure)]
    pub fn get_name(world: &mut Arc<RwLock<World>>, entity: Entity) -> Dynamic {
        if let Some(named) = world.read().unwrap().get::<Named>(entity) {
            Dynamic::from(named.name.clone())
        } else {
            Dynamic::UNIT
        }
    }
}
