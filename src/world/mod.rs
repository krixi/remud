#![allow(clippy::type_complexity)]

pub mod action;
mod scripting;
pub mod types;

use std::{
    collections::{HashMap, VecDeque},
    convert::TryFrom,
    ops::DerefMut,
    sync::{Arc, RwLock},
};

use bevy_app::Events;
use bevy_ecs::prelude::*;
use itertools::Itertools;
use lazy_static::lazy_static;
use rhai::{plugin::*, Scope, AST};

use crate::{
    engine::persist::{self, DynUpdate, Updates},
    world::{
        action::{
            communicate::{emote_system, say_system, send_system},
            immortal::{
                object::{
                    object_clear_flags_system, object_create_system, object_info_system,
                    object_remove_system, object_set_flags_system,
                    object_update_description_system, object_update_keywords_system,
                    object_update_name_system,
                },
                player::player_info_system,
                room::{
                    room_create_system, room_info_system, room_link_system, room_remove_system,
                    room_unlink_system, room_update_description_system,
                },
            },
            movement::{move_system, teleport_system},
            object::{drop_system, get_system, inventory_system},
            observe::{exits_system, look_at_system, look_system, who_system},
            queue_message,
            system::{login_system, logout_system, shutdown_system, Logout},
            ActionEvent, DynAction,
        },
        scripting::{
            post_script_system, pre_script_system, trigger_api, world_api, Script, ScriptExecutions,
        },
        types::{
            player::{Messages, Player, Players},
            room::{Room, RoomBundle, RoomId, Rooms},
            Configuration, Contents, Description, Id, Location, Named,
        },
    },
};

lazy_static! {
    pub static ref VOID_ROOM_ID: RoomId = RoomId::try_from(0).unwrap();
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
        world.insert_resource(Events::<ActionEvent>::default());

        // Add resources
        world.insert_resource(Updates::default());
        world.insert_resource(Players::default());

        // Add void room
        GameWorld::add_void_room(&mut world);

        // Create schedule
        let mut schedule = Schedule::default();

        let mut first = SystemStage::parallel();
        first.add_system(Events::<ActionEvent>::update_system.system());

        let mut update = SystemStage::parallel();
        update.add_system(pre_script_system.exclusive_system());

        update.add_system(drop_system.system());
        update.add_system(emote_system.system());
        update.add_system(exits_system.system());
        update.add_system(get_system.system());
        update.add_system(inventory_system.system());
        update.add_system(login_system.system());
        update.add_system(logout_system.system());
        update.add_system(look_system.system());
        update.add_system(look_at_system.system());
        update.add_system(move_system.system());
        update.add_system(object_clear_flags_system.system());
        update.add_system(object_create_system.system());
        update.add_system(object_info_system.system());
        update.add_system(object_update_description_system.system());
        update.add_system(object_update_keywords_system.system());
        update.add_system(object_update_name_system.system());
        update.add_system(object_remove_system.system());
        update.add_system(object_set_flags_system.system());
        update.add_system(player_info_system.system());
        update.add_system(room_create_system.system());
        update.add_system(room_info_system.system());
        update.add_system(room_link_system.system());
        update.add_system(room_update_description_system.system());
        update.add_system(room_unlink_system.system());
        update.add_system(room_remove_system.system());
        update.add_system(say_system.system());
        update.add_system(send_system.system());
        update.add_system(shutdown_system.system());
        update.add_system(teleport_system.system());
        update.add_system(who_system.system());

        update.add_system(post_script_system.exclusive_system().at_end());

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
            .query::<(&Named, &Location)>()
            .get(&*world, player)
            .map(|(named, location)| (named.name.clone(), location.room))
            .ok()
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
        world
            .get_resource_mut::<Players>()
            .unwrap()
            .remove(name.as_str());
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

    pub async fn spawn_room(&self) -> RoomId {
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
            let mut messages = world.get_mut::<Messages>(player).unwrap();

            if messages.queue.is_empty() {
                continue;
            }

            let mut queue = VecDeque::new();
            std::mem::swap(&mut queue, &mut messages.queue);

            if !messages.received_input {
                queue.push_front("\r\n".to_string());
            }

            messages.received_input = false;

            outgoing.push((player, queue));
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

    fn add_void_room(world: &mut World) {
        if world
            .get_resource::<Rooms>()
            .unwrap()
            .by_id(*VOID_ROOM_ID)
            .is_none()
        {
            let description = "A dark void extends infinitely in all directions.".to_string();
            let bundle = RoomBundle {
                id: Id::Room(*VOID_ROOM_ID),
                room: Room::new(*VOID_ROOM_ID),
                description: Description {
                    text: description.clone(),
                },
                contents: Contents::default(),
            };
            let void_room = world.spawn().insert_bundle(bundle).id();
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
    }
}
