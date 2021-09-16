#![allow(clippy::type_complexity)]

pub mod action;
pub mod types;

use std::{
    collections::{HashMap, VecDeque},
    convert::TryFrom,
    ops::DerefMut,
    sync::{Arc, RwLock},
};

use bevy_app::{EventReader, Events};
use bevy_ecs::prelude::*;
use itertools::Itertools;
use lazy_static::lazy_static;
use rhai::{plugin::*, Scope, AST};

use crate::{
    engine::persist::{self, DynUpdate, Updates},
    world::{
        action::{
            communicate::{emote_system, say_system, send_system},
            movement::{move_system, teleport_system},
            object::drop_system,
            queue_message,
            system::Logout,
            DynAction,
        },
        types::{
            object::Object,
            player::{Messages, Player, Players},
            room::{self, Room, Rooms},
            Configuration, Contents,
        },
    },
};

lazy_static! {
    pub static ref VOID_ROOM_ID: room::Id = room::Id::try_from(0).unwrap();
}

#[derive(Debug)]
pub struct PlayerAction {
    player: Entity,
    event: PlayerEvent,
}

impl PlayerAction {
    fn trigger(&self) -> Trigger {
        match self.event {
            PlayerEvent::Say { .. } => Trigger::Say,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Say { room: Entity, message: String },
}

#[derive(Debug, Clone)]
pub enum TriggerData {
    Player(Entity, PlayerEvent),
}

#[export_module]
mod trigger_api {
    use crate::world::TriggerData;

    #[rhai_fn(get = "entity", pure)]
    pub fn get_entity(trigger_data: &mut TriggerData) -> Dynamic {
        match trigger_data {
            TriggerData::Player(entity, _) => Dynamic::from(*entity),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Script(pub String);

pub struct ScriptExecutions {
    runs: Vec<(TriggerData, Script)>,
}

pub struct ScriptTriggers {
    list: Vec<(Trigger, Script)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Say,
}

#[export_module]
pub mod world_api {
    use std::sync::RwLock;

    use rhai::Dynamic;

    #[rhai_fn(pure)]
    pub fn player_name(world: &mut Arc<RwLock<World>>, player: Entity) -> Dynamic {
        if let Some(player) = world.read().unwrap().get::<Player>(player) {
            Dynamic::from(player.name.clone())
        } else {
            Dynamic::UNIT
        }
    }
}

fn player_action_events(
    mut commands: Commands,
    mut actions: EventReader<PlayerAction>,
    objects_query: Query<(Entity, &Object, &ScriptTriggers)>,
    mut executions_query: Query<&mut ScriptExecutions>,
) {
    for action in actions.iter() {
        let room = match action.event {
            PlayerEvent::Say { room, .. } => Some(room),
        };

        for (object_entity, object, script_triggers) in objects_query.iter() {
            if let Some(room) = room {
                if object.container != room {
                    continue;
                }
            }

            let trigger = action.trigger();

            let scripts = script_triggers
                .list
                .iter()
                .filter(|(script_trigger, _)| trigger == *script_trigger)
                .map(|(_, script)| script)
                .collect_vec();

            if let Ok(mut executions) = executions_query.get_mut(object_entity) {
                for script in scripts {
                    executions.runs.push((
                        TriggerData::Player(action.player, action.event.clone()),
                        script.clone(),
                    ));
                }
            } else {
                let executions = {
                    let runs = scripts
                        .into_iter()
                        .map(|script| {
                            (
                                TriggerData::Player(action.player, action.event.clone()),
                                script.clone(),
                            )
                        })
                        .collect_vec();
                    ScriptExecutions { runs }
                };
                commands.entity(object_entity).insert(executions);
            };
        }
    }
}

pub struct GameWorld {
    world: Arc<RwLock<World>>,
    schedule: Schedule,
    engine: rhai::Engine,
    scripts: HashMap<Script, AST>,
}

impl GameWorld {
    pub fn new(mut world: World) -> Self {
        // Add events
        world.insert_resource(Events::<PlayerAction>::default());

        // Add resources
        world.insert_resource(Updates::default());
        world.insert_resource(Players::default());

        if world
            .get_resource::<Rooms>()
            .unwrap()
            .by_id(*VOID_ROOM_ID)
            .is_none()
        {
            let description = "A dark void extends infinitely in all directions.".to_string();
            let room = Room::new(*VOID_ROOM_ID, description.clone());
            let void_room = world.spawn().insert(room).id();
            world
                .get_resource_mut::<Rooms>()
                .unwrap()
                .insert(*VOID_ROOM_ID, void_room);

            world
                .get_resource_mut::<Updates>()
                .unwrap()
                .queue(persist::room::New::new(*VOID_ROOM_ID, description));

            tracing::warn!("Void room was deleted and has been recreated.");
        }

        // Create schedule
        let mut schedule = Schedule::default();
        let mut first = SystemStage::parallel();
        first.add_system(Events::<PlayerAction>::update_system.system());

        let mut update = SystemStage::parallel();
        update.add_system(player_action_events.system());

        update.add_system(drop_system.system());
        update.add_system(emote_system.system());
        update.add_system(move_system.system());
        update.add_system(say_system.system());
        update.add_system(send_system.system());
        update.add_system(teleport_system.system());

        // Add fun systems
        schedule.add_stage("first", first);
        schedule.add_stage_after("first", "update", update);

        let world = Arc::new(RwLock::new(world));

        let mut engine = rhai::Engine::default();
        engine.register_type_with_name::<Arc<RwLock<World>>>("World");
        engine.register_global_module(exported_module!(world_api).into());
        engine.register_global_module(exported_module!(trigger_api).into());

        let mut scripts = HashMap::new();

        let script = r#"
        let player = TRIGGER.entity;
        let name = WORLD.player_name(player);
        let output = `Hello there, ${name}.`;
        "#;
        let compiled = engine.compile(script).unwrap();

        scripts.insert(Script("say_hi".to_string()), compiled);

        GameWorld {
            world,
            schedule,
            engine,
            scripts,
        }
    }

    pub async fn run(&mut self) {
        self.schedule
            .run_once(self.world.write().unwrap().deref_mut());

        let executions = {
            let mut world = self.world.write().unwrap();
            let entities = world
                .query_filtered::<Entity, With<ScriptExecutions>>()
                .iter(&world)
                .collect_vec();

            entities
                .into_iter()
                .map(|entity| {
                    (
                        entity,
                        world
                            .entity_mut(entity)
                            .remove::<ScriptExecutions>()
                            .unwrap(),
                    )
                })
                .collect_vec()
        };

        for (entity, execution) in executions {
            for (trigger, script) in execution.runs {
                if let Some(ast) = self.scripts.get(&script) {
                    let mut scope = Scope::new();
                    scope.push_constant("SELF", entity);
                    scope.push_constant("WORLD", self.world.clone());
                    scope.push_constant("TRIGGER", trigger);
                    self.engine.consume_ast_with_scope(&mut scope, ast).unwrap();
                    let output = scope.get_value::<String>("output");
                    tracing::info!("script output: {:?}", output);
                }
            }
        }
    }

    pub async fn should_shutdown(&self) -> bool {
        self.world
            .read()
            .unwrap()
            .get_resource::<Configuration>()
            .map_or(true, |configuration| configuration.shutdown)
    }

    pub async fn despawn_player(&mut self, player: Entity) -> anyhow::Result<()> {
        self.player_action(player, Box::new(Logout {})).await;

        let mut world = self.world.write().unwrap();

        let (name, room) = world
            .get::<Player>(player)
            .map(|player| (player.name.clone(), player.room))
            .ok_or(action::Error::MissingComponent(player, "Player"))?;

        if let Some(objects) = world
            .get::<Contents>(player)
            .map(|contents| contents.objects.clone())
        {
            for object in objects {
                world.despawn(object);
            }
        }
        world.despawn(player);
        world.get_resource_mut::<Players>().unwrap().remove(&name);
        world
            .get_mut::<Room>(room)
            .ok_or(action::Error::MissingComponent(room, "Room"))?
            .remove_player(player);

        Ok(())
    }

    pub async fn player_action(&mut self, player: Entity, mut action: DynAction) {
        let mut world = self.world.write().unwrap();

        match world.get_mut::<Messages>(player) {
            Some(mut messages) => messages.received_input = true,
            None => {
                world.entity_mut(player).insert(Messages {
                    received_input: true,
                    queue: VecDeque::new(),
                });
            }
        }

        if let Err(e) = action.enact(player, &mut world) {
            queue_message(&mut world, player, "Command failed.".to_string());
            tracing::error!("Action error: {}", e);
        };
    }

    pub async fn player_online(&self, name: &str) -> bool {
        self.world
            .read()
            .unwrap()
            .get_resource::<Players>()
            .unwrap()
            .by_name(name)
            .is_some()
    }

    pub async fn spawn_room(&self) -> room::Id {
        self.world
            .read()
            .unwrap()
            .get_resource::<Configuration>()
            .unwrap()
            .spawn_room
    }

    pub async fn messages(&mut self) -> Vec<(Entity, VecDeque<String>)> {
        let mut world = self.world.write().unwrap();

        let players_with_messages = world
            .query_filtered::<Entity, (With<Player>, With<Messages>)>()
            .iter(&world)
            .collect_vec();

        let mut outgoing = Vec::new();

        for player in players_with_messages {
            if let Some(messages) = world.get::<Messages>(player) {
                if messages.queue.is_empty() {
                    continue;
                }
            }
            if let Some(mut messages) = world.entity_mut(player).remove::<Messages>() {
                if !messages.received_input {
                    messages.queue.push_front("\r\n".to_string());
                }
                outgoing.push((player, messages.queue));
            }
        }

        outgoing
    }

    pub async fn updates(&mut self) -> Vec<DynUpdate> {
        self.world
            .write()
            .unwrap()
            .get_resource_mut::<Updates>()
            .unwrap()
            .take()
    }

    pub fn get_world(&self) -> Arc<RwLock<World>> {
        self.world.clone()
    }
}
