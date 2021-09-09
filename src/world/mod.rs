#![allow(clippy::type_complexity)]

pub mod action;
pub mod types;

use std::{
    collections::{HashMap, VecDeque},
    convert::TryFrom,
};

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    engine::persistence::{DynUpdate, Updates},
    queue_message,
    text::word_list,
    world::{
        action::{DynAction, Login, Logout},
        types::{
            players::{Messages, Player, Players},
            room::{Direction, Room, RoomId, Rooms},
            Configuration, Location,
        },
    },
};

pub struct GameWorld {
    world: World,
    schedule: Schedule,
    void_room: Entity,
}

impl GameWorld {
    pub fn new(mut world: World) -> Self {
        // Create emergency room
        let room = Room {
            id: RoomId::try_from(0).unwrap(),
            description: "A dark void extends infinitely in all directions.".to_string(),
            exits: HashMap::new(),
        };
        let void_room = world.spawn().insert(room).id();

        // Add resources
        world.insert_resource(Updates::default());
        world.insert_resource(Players::default());

        // Create schedule
        let mut schedule = Schedule::default();

        let mut update = SystemStage::parallel();
        update.add_system(exits_system.system());
        update.add_system(login_system.system());
        update.add_system(logout_system.system());
        update.add_system(look_system.system());
        update.add_system(move_system.system());
        update.add_system(say_system.system());
        update.add_system(send_message_system.system());
        update.add_system(teleport_system.system());
        update.add_system(who_system.system());
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
            .map_or(true, |configuration| configuration.shutdown)
    }

    pub fn spawn_player(&mut self, name: String) -> Entity {
        let (player, room) = {
            let room = {
                let configuration = self.world.get_resource::<Configuration>().unwrap();
                let rooms = self.world.get_resource::<Rooms>().unwrap();

                rooms
                    .get_room(configuration.spawn_room)
                    .unwrap_or(self.void_room)
            };

            let player = self
                .world
                .spawn()
                .insert(Player { name: name.clone() })
                .insert(Location { room })
                .insert(WantsToLook::default())
                .id();

            (player, room)
        };

        let mut players = self.world.get_resource_mut::<Players>().unwrap();

        players.spawn(player, name, room);

        self.player_action(player, Box::new(Login {}));

        player
    }

    pub fn despawn_player(&mut self, player: Entity) {
        self.player_action(player, Box::new(Logout {}));

        let location = self
            .world
            .get::<Location>(player)
            .map(|location| location.room);

        if let Some(location) = location {
            let name = if let Some(name) = self
                .world
                .get::<Player>(player)
                .map(|player| player.name.clone())
            {
                name
            } else {
                tracing::error!("Unable to despawn player {:?} at {:?}", player, location);
                return;
            };

            self.world.entity_mut(player).despawn();

            let mut players = self.world.get_resource_mut::<Players>().unwrap();
            players.despawn(player, &name, location);
        } else {
            tracing::error!("Unable to despawn player {:?}", player);
        };
    }

    pub fn player_action(&mut self, player: Entity, mut action: DynAction) {
        match self.world.entity_mut(player).get_mut::<Messages>() {
            Some(mut messages) => messages.received_input = true,
            None => {
                self.world.entity_mut(player).insert(Messages {
                    received_input: true,
                    queue: VecDeque::new(),
                });
            }
        }
        action.enact(player, &mut self.world);
    }

    pub fn messages(&mut self) -> Vec<(Entity, VecDeque<String>)> {
        let players_with_messages = self
            .world
            .query_filtered::<Entity, (With<Player>, With<Messages>)>()
            .iter(&self.world)
            .collect_vec();

        let mut outgoing = Vec::new();

        for player in players_with_messages {
            if let Some(mut messages) = self.world.entity_mut(player).remove::<Messages>() {
                if !messages.queue.is_empty() {
                    if !messages.received_input {
                        messages.queue.push_front("\r\n".to_string());
                    }
                    outgoing.push((player, messages.queue));
                }
            }
        }

        outgoing
    }

    pub fn updates(&mut self) -> Vec<DynUpdate> {
        self.world.get_resource_mut::<Updates>().unwrap().take()
    }

    pub fn get_world(&self) -> &World {
        &self.world
    }
}

pub struct LoggedIn {}

fn login_system(
    mut commands: Commands,
    players: Res<Players>,
    login_query: Query<(Entity, &Player, &Location), With<LoggedIn>>,
    mut messages: Query<&mut Messages>,
) {
    for (login_entity, login_player, login_location) in login_query.iter() {
        players
            .by_room(login_location.room)
            .filter(|player| *player != login_entity)
            .for_each(|present_player| {
                let message = format!("{} arrives.\r\n", login_player.name);
                queue_message!(commands, messages, present_player, message);
            });

        commands.entity(login_entity).remove::<LoggedIn>();
    }
}

pub struct LoggedOut {
    name: String,
}
fn logout_system(
    mut commands: Commands,
    players: Res<Players>,
    login_query: Query<(Entity, &LoggedOut), With<Room>>,
    mut messages: Query<&mut Messages>,
) {
    for (logout_entity, logged_out) in login_query.iter() {
        players.by_room(logout_entity).for_each(|present_player| {
            let message = format!("{} leaves.\r\n", logged_out.name);
            queue_message!(commands, messages, present_player, message);
        });

        commands.entity(logout_entity).remove::<LoggedOut>();
    }
}

pub struct WantsExits {}

fn exits_system(
    mut commands: Commands,
    exits_query: Query<(Entity, &Location), (With<Player>, With<WantsExits>)>,
    rooms_query: Query<&Room>,
    mut messages: Query<&mut Messages>,
) {
    for (exits_entity, exits_location) in exits_query.iter() {
        if let Ok(room) = rooms_query.get(exits_location.room) {
            let exits = room
                .exits
                .keys()
                .map(Direction::as_str)
                .map(|str| str.to_string())
                .sorted()
                .collect_vec();

            let message = if exits.is_empty() {
                "This room has no obvious exits.\r\n".to_string()
            } else if exits.len() == 1 {
                format!("There is an exit {}.\r\n", word_list(exits))
            } else {
                format!("There are exits {}.\r\n", word_list(exits))
            };

            queue_message!(commands, messages, exits_entity, message);
        }

        commands.entity(exits_entity).remove::<WantsExits>();
    }
}

#[derive(Default)]
pub struct WantsToLook {
    direction: Option<Direction>,
}

fn look_system(
    mut commands: Commands,
    players: Res<Players>,
    looking_query: Query<(Entity, &Location, &WantsToLook), With<Player>>,
    players_query: Query<&Player>,
    rooms_query: Query<&Room>,
    mut messages: Query<&mut Messages>,
) {
    for (looking_entity, looking_location, wants_to_look) in looking_query.iter() {
        let looking_room = if let Some(direction) = wants_to_look.direction {
            if let Ok(room) = rooms_query.get(looking_location.room) {
                if let Some(looking_room) = room.exits.get(&direction) {
                    *looking_room
                } else {
                    let message = format!("There is no room {}.\r\n", direction.as_to_str());
                    queue_message!(commands, messages, looking_entity, message);
                    return;
                }
            } else {
                return;
            }
        } else {
            looking_location.room
        };

        if let Ok(room) = rooms_query.get(looking_room) {
            let mut message = format!("{}\r\n", room.description);

            let mut present_names = players
                .by_room(looking_room)
                .filter(|player| player != &looking_entity)
                .filter_map(|player| players_query.get(player).ok())
                .map(|player| player.name.clone())
                .collect_vec();

            if !present_names.is_empty() {
                present_names.sort();

                let singular = present_names.len() == 1;

                let mut player_list = word_list(present_names);
                if singular {
                    player_list.push_str(" is here.");
                } else {
                    player_list.push_str(" are here.");
                };
                message.push_str(player_list.as_str());
                message.push_str("\r\n");
            }

            queue_message!(commands, messages, looking_entity, message);
        }

        commands.entity(looking_entity).remove::<WantsToLook>();
    }
}

pub struct WantsToMove {
    pub direction: Direction,
}

fn move_system(
    mut commands: Commands,
    mut players: ResMut<Players>,
    mut moving_query: Query<(Entity, &Player, &WantsToMove, &mut Location)>,
    rooms_query: Query<&Room>,
    mut messages: Query<&mut Messages>,
) {
    for (moving_entity, player, wants_to_move, mut location) in moving_query.iter_mut() {
        let destination = if let Some(destination) = rooms_query
            .get(location.room)
            .ok()
            .and_then(|room| room.exits.get(&wants_to_move.direction))
        {
            *destination
        } else {
            let message = "There is nothing in that direction.\r\n".to_string();
            queue_message!(commands, messages, moving_entity, message);

            commands.entity(moving_entity).remove::<WantsToMove>();

            continue;
        };

        players.change_room(moving_entity, location.room, destination);

        players
            .by_room(location.room)
            .filter(|player| player != &moving_entity)
            .for_each(|present_player| {
                let message = format!(
                    "{} leaves {}.\r\n",
                    player.name,
                    wants_to_move.direction.as_to_str()
                );
                queue_message!(commands, messages, present_player, message);
            });

        let direction_from = rooms_query.get(destination).ok().and_then(|destination| {
            destination
                .exits
                .iter()
                .find(|(_, from_room)| **from_room == location.room)
                .map(|(direction, _)| direction)
                .copied()
        });

        location.room = destination;

        players
            .by_room(destination)
            .filter(|player| player != &moving_entity)
            .for_each(|present_player| {
                let message = direction_from
                    .map(|from| format!("{} arrives {}.\r\n", player.name, from.as_from_str()))
                    .unwrap_or_else(|| format!("{} appears.\r\n", player.name));
                queue_message!(commands, messages, present_player, message);
            });

        commands
            .entity(moving_entity)
            .insert(WantsToLook::default())
            .remove::<WantsToMove>();
    }
}

pub struct WantsToSay {
    pub message: String,
}

fn say_system(
    mut commands: Commands,
    players: Res<Players>,
    saying_query: Query<(Entity, &Player, &Location, &WantsToSay)>,
    mut messages: Query<&mut Messages>,
) {
    for (saying_entity, saying_player, saying_location, wants_to_say) in saying_query.iter() {
        players
            .by_room(saying_location.room)
            .filter(|player| player != &saying_entity)
            .for_each(|present_player| {
                let message = format!(
                    "{} says \"{}\"\r\n",
                    saying_player.name, wants_to_say.message
                );
                queue_message!(commands, messages, present_player, message);
            });

        let message = format!("You say \"{}\"\r\n", wants_to_say.message);
        queue_message!(commands, messages, saying_entity, message);

        commands.entity(saying_entity).remove::<WantsToSay>();
    }
}

pub struct WantsToSendMessage {
    pub recipient: Entity,
    pub message: String,
}

fn send_message_system(
    mut commands: Commands,
    send_query: Query<(Entity, &Player, &WantsToSendMessage)>,
    player_query: Query<&Player>,
    mut messages: Query<&mut Messages>,
) {
    for (send_entity, send_player, send_message) in send_query.iter() {
        if send_entity == send_message.recipient {
            let message = "Your term trills: \"Invalid recipient: Self.\"\r\n".to_string();
            queue_message!(commands, messages, send_entity, message);

            commands.entity(send_entity).remove::<WantsToSendMessage>();
            continue;
        }

        let sent_message = format!(
            "{} sends \"{}\".\r\n",
            send_player.name, send_message.message
        );
        queue_message!(commands, messages, send_message.recipient, sent_message);

        if let Ok(name) = player_query
            .get(send_message.recipient)
            .map(|player| player.name.clone())
        {
            let message = format!(
                "Your term chirps happily: \"Message sent to '{}'.\"\r\n",
                name
            );
            queue_message!(commands, messages, send_entity, message);
        }

        commands.entity(send_entity).remove::<WantsToSendMessage>();
    }
}

pub struct WantsToTeleport {
    pub room: Entity,
}

fn teleport_system(
    mut commands: Commands,
    mut players: ResMut<Players>,
    mut teleporting_query: Query<(Entity, &Player, &WantsToTeleport, &mut Location)>,
    mut messages: Query<&mut Messages>,
) {
    for (teleporting_entity, teleporting_player, wants_to_teleport, mut location) in
        teleporting_query.iter_mut()
    {
        players
            .by_room(location.room)
            .filter(|player| player != &teleporting_entity)
            .for_each(|present_player| {
                let message = format!(
                    "{} disappears in the blink of an eye.\r\n",
                    teleporting_player.name
                );
                queue_message!(commands, messages, present_player, message);
            });

        players.change_room(teleporting_entity, location.room, wants_to_teleport.room);

        location.room = wants_to_teleport.room;

        players
            .by_room(location.room)
            .filter(|player| player != &teleporting_entity)
            .for_each(|present_player| {
                let message = format!(
                    "{} appears in a flash of light.\r\n",
                    teleporting_player.name
                );
                queue_message!(commands, messages, present_player, message);
            });

        commands
            .entity(teleporting_entity)
            .insert(WantsToLook::default())
            .remove::<WantsToTeleport>();
    }
}

pub struct WantsWhoInfo {}

fn who_system(
    mut commands: Commands,
    who_query: Query<Entity, With<WantsWhoInfo>>,
    players_query: Query<&Player>,
    mut messages: Query<&mut Messages>,
) {
    for who_entity in who_query.iter() {
        let players = players_query
            .iter()
            .map(|player| format!("  {}", player.name))
            .sorted()
            .join("\r\n");

        let message = format!("Online Players:\r\n{}\r\n", players);
        queue_message!(commands, messages, who_entity, message);

        commands.entity(who_entity).remove::<WantsWhoInfo>();
    }
}
