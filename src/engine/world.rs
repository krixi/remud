use std::collections::HashMap;

use bevy_ecs::prelude::*;
use itertools::Itertools;

pub struct Player {
    name: String,
    location: Entity,
}

impl Player {
    pub fn new(name: String, location: Entity) -> Self {
        Player { name, location }
    }
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
        let void_room = Room {
            id: 0,
            description: "A dark void extends infinitely in all directions.".to_string(),
        };

        let void_room = world.spawn().insert(void_room).id();

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
        let configuration = self.world.get_resource::<Configuration>().unwrap();
        let room_metadata = self.world.get_resource::<RoomMetadata>().unwrap();

        let spawn_room = room_metadata
            .rooms_by_id
            .get(&configuration.spawn_room)
            .unwrap_or(&self.void_room);

        let player = Player::new(name, *spawn_room);
        let player_entity = self
            .world
            .spawn()
            .insert(player)
            .insert(WantsToLook {})
            .id();

        player_entity
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
    players_saying: Query<(Entity, &Player, &WantsToSay)>,
    players: Query<(Entity, &Player)>,
    mut messages: Query<&mut Messages>,
) {
    for (player_saying_entity, player_saying, wants_to_say) in players_saying.iter() {
        let location = player_saying.location;

        for (player_entity, player) in players.iter() {
            if player_entity == player_saying_entity {
                continue;
            }

            if player.location == location {
                let message = format!(
                    "{} says \"{}\"\r\n",
                    player_saying.name, wants_to_say.message
                );
                match messages.get_mut(player_entity) {
                    Ok(mut messages) => messages.queue.push(message),
                    Err(_) => {
                        commands
                            .entity(player_entity)
                            .insert(Messages::new_with(message));
                    }
                }
            }
        }

        commands.entity(player_saying_entity).remove::<WantsToSay>();
    }
}

fn look_system(
    mut commands: Commands,
    players_looking: Query<(Entity, &Player), With<WantsToLook>>,
    rooms: Query<&Room>,
    mut messages: Query<&mut Messages>,
) {
    for (player_entity, player) in players_looking.iter() {
        if let Ok(room) = rooms.get(player.location) {
            let message = format!("{}\r\n", room.description);
            match messages.get_mut(player_entity) {
                Ok(mut messages) => messages.queue.push(message),
                Err(_) => {
                    commands
                        .entity(player_entity)
                        .insert(Messages::new_with(message));
                }
            }
        }
        commands.entity(player_entity).remove::<WantsToLook>();
    }
}
