#![allow(clippy::type_complexity)]

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{queue_message, text::word_list};

pub struct Player {
    name: String,
}

pub struct Location {
    room: Entity,
}

pub enum Action {
    Look,
    Say(String),
    Shutdown,
}

pub struct Messages {
    queue: Vec<String>,
}

impl Messages {
    fn new_with(message: String) -> Self {
        Messages {
            queue: vec![message],
        }
    }
}

pub struct Room {
    pub id: i64,
    pub description: String,
}

pub struct RoomMetadata {
    pub rooms_by_id: HashMap<i64, Entity>,
    pub players_by_room: HashMap<Entity, HashSet<Entity>>,
    pub highest_id: i64,
}

pub struct Configuration {
    pub shutdown: bool,
    pub spawn_room: i64,
}

pub struct WantsToSay {
    message: String,
}

pub struct WantsToLook {}

pub struct GameWorld {
    world: World,
    schedule: Schedule,
    void_room: Entity,
}

impl GameWorld {
    pub fn new(mut world: World) -> Self {
        let room = Room {
            id: 0,
            description: "A dark void extends infinitely in all directions.".to_string(),
        };
        let void_room = world.spawn().insert(room).id();

        let mut schedule = Schedule::default();

        let mut update = SystemStage::parallel();
        update.add_system(say_system.system());
        update.add_system(look_system.system());
        schedule.add_stage("update", update);

        GameWorld {
            world,
            schedule,
            void_room,
        }
    }

    pub fn run(&mut self) {
        self.schedule.run_once(&mut self.world);
    }

    pub fn should_shutdown(&self) -> bool {
        self.world
            .get_resource::<Configuration>()
            .map(|configuration| configuration.shutdown)
            .unwrap_or(true)
    }

    pub fn spawn_player(&mut self, name: String) -> Entity {
        let (player, room) = {
            let room = {
                let configuration = self.world.get_resource::<Configuration>().unwrap();
                let room_metadata = self.world.get_resource::<RoomMetadata>().unwrap();

                *room_metadata
                    .rooms_by_id
                    .get(&configuration.spawn_room)
                    .unwrap_or(&self.void_room)
            };

            let player_entity = self
                .world
                .spawn()
                .insert(Player { name })
                .insert(Location { room })
                .insert(WantsToLook {})
                .id();

            (player_entity, room)
        };

        let mut room_metadata = self.world.get_resource_mut::<RoomMetadata>().unwrap();

        room_metadata
            .players_by_room
            .entry(room)
            .or_default()
            .insert(player);

        player
    }

    pub fn despawn_player(&mut self, player_entity: Entity) {
        let location = self
            .world
            .get::<Location>(player_entity)
            .map(|location| location.room);

        self.world.entity_mut(player_entity).despawn();

        if let Some(location) = location {
            let mut room_metadata = self.world.get_resource_mut::<RoomMetadata>().unwrap();
            if let Some(players_by_room) = room_metadata.players_by_room.get_mut(&location) {
                players_by_room.remove(&player_entity);
            }
        }
    }

    pub fn player_action(&mut self, player: Entity, action: Action) {
        match action {
            Action::Look => {
                self.world.entity_mut(player).insert(WantsToLook {});
            }
            Action::Say(message) => {
                self.world.entity_mut(player).insert(WantsToSay { message });
            }
            Action::Shutdown => {
                let mut configuration = self.world.get_resource_mut::<Configuration>().unwrap();
                configuration.shutdown = true;
            }
        }
    }

    pub fn messages(&mut self) -> Vec<(Entity, Vec<String>)> {
        let players_with_messages = self
            .world
            .query_filtered::<Entity, (With<Player>, With<Messages>)>()
            .iter(&self.world)
            .collect_vec();

        let mut outgoing = Vec::new();

        for player in players_with_messages {
            if let Some(messages) = self.world.entity_mut(player).remove::<Messages>() {
                outgoing.push((player, messages.queue));
            }
        }

        outgoing
    }
}

fn say_system(
    mut commands: Commands,
    room_data: Res<RoomMetadata>,
    saying_players: Query<(Entity, &Player, &Location, &WantsToSay)>,
    mut messages: Query<&mut Messages>,
) {
    for (saying_entity, saying_player, saying_location, wants_to_say) in saying_players.iter() {
        if let Some(present_players) = room_data.players_by_room.get(&saying_location.room) {
            for present_player_entity in present_players.iter() {
                if *present_player_entity == saying_entity {
                    continue;
                }

                let message = format!(
                    "{} says \"{}\"\r\n",
                    saying_player.name, wants_to_say.message
                );

                queue_message!(commands, messages, *present_player_entity, message);
            }
        }

        commands.entity(saying_entity).remove::<WantsToSay>();
    }
}

fn look_system(
    mut commands: Commands,
    room_data: Res<RoomMetadata>,
    looking_players: Query<(Entity, &Location), (With<Player>, With<WantsToLook>)>,
    players: Query<&Player>,
    rooms: Query<&Room>,
    mut messages: Query<&mut Messages>,
) {
    for (looking_entity, looking_location) in looking_players.iter() {
        if let Ok(room) = rooms.get(looking_location.room) {
            let mut message = format!("{}\r\n", room.description);

            if let Some(present_players) = room_data.players_by_room.get(&looking_location.room) {
                let mut present_player_names = present_players
                    .iter()
                    .filter(|player| **player != looking_entity)
                    .filter_map(|player| players.get(*player).ok())
                    .map(|player| player.name.clone())
                    .collect_vec();

                if !present_player_names.is_empty() {
                    present_player_names.sort();

                    let singular = present_player_names.len() == 1;

                    let mut player_list = word_list(present_player_names);
                    if singular {
                        player_list.push_str(" is here.");
                    } else {
                        player_list.push_str(" are here.");
                    };
                    message.push_str(player_list.as_str());
                    message.push_str("\r\n");
                }
            }

            queue_message!(commands, messages, looking_entity, message);
        }
        commands.entity(looking_entity).remove::<WantsToLook>();
    }
}
