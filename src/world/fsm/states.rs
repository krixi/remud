use crate::world::{
    action::{communicate::Say, movement::Move, Action},
    fsm::{State, StateId, Transition},
    scripting::QueuedAction,
    types::{
        room::{Direction, Room},
        Location, Named,
    },
};
use bevy_app::Events;
use bevy_ecs::prelude::*;
use rand::{prelude::*, thread_rng};
use std::fmt::Debug;

/// Wander around and look for the player
#[derive(Debug, Default)]
pub struct WanderState {
    tick_timer: u32,
}
impl From<rhai::Map> for WanderState {
    fn from(_params: rhai::Map) -> Self {
        tracing::info!("make chase state: {:?}", _params);
        WanderState::default()
    }
}

impl State for WanderState {
    fn on_enter(&mut self, _: Entity, _: &mut World) {
        self.tick_timer = 0;
    }

    fn decide(&mut self, entity: Entity, world: &mut World) -> Option<Transition> {
        // Check to see if a player has come within range (in the same room) for long enough (requires timer).

        // get the current room from the entity
        let room = world.get::<Location>(entity).unwrap().room;

        // get the players in the room
        let players = &world.get::<Room>(room).unwrap().players;

        if !players.is_empty() {
            Some(Transition::SawPlayer)
        } else {
            None
        }
    }

    fn act(&mut self, entity: Entity, world: &mut World) {
        // no player seen. Pick an exit and go through it.
        let room = world.get::<Location>(entity).unwrap().room;
        let exits = &world.get::<Room>(room).unwrap().exits;

        self.tick_timer += 1;

        if !exits.is_empty() && self.tick_timer > 120 {
            self.tick_timer = 0;

            // pick a random direction to go
            let mut rng = thread_rng();
            let exit = exits.keys().choose(&mut rng);

            // if we found one, post the event that will move us there.
            if let Some(exit) = exit.copied() {
                let mut events = world.get_resource_mut::<Events<QueuedAction>>().unwrap();
                events.send(QueuedAction {
                    action: Action::Move(Move {
                        entity,
                        direction: exit,
                    }),
                })
            }
        }
    }

    fn output_state(&self, next: Transition) -> Option<StateId> {
        match next {
            Transition::SawPlayer => Some(StateId::Chase),
            _ => None,
        }
    }
}

/// Chase after the player until you lose sight of them
#[derive(Debug, Default)]
pub struct ChaseState {
    chasing: Option<Entity>,
    move_direction: Option<Direction>,
    tick_timer: u32,
}
impl From<rhai::Map> for ChaseState {
    fn from(_params: rhai::Map) -> Self {
        tracing::info!("make chase state: {:?}", _params);
        ChaseState::default()
    }
}

impl State for ChaseState {
    fn on_enter(&mut self, entity: Entity, world: &mut World) {
        self.tick_timer = 0;

        // get the players in the room, pick one to chase
        let room = world.get::<Location>(entity).unwrap().room;
        let players = &world.get::<Room>(room).unwrap().players;
        let mut rng = thread_rng();
        let player = *players.as_slice().choose(&mut rng).unwrap();
        let player_name = world.get::<Named>(player).unwrap().name.clone();
        self.chasing = Some(player);

        // say "i'm gonna get you"
        let mut events = world.get_resource_mut::<Events<QueuedAction>>().unwrap();
        events.send(QueuedAction {
            action: Action::Say(Say {
                entity,
                message: format!("I'm gonna get you, {}!", player_name),
            }),
        })
    }

    fn decide(&mut self, entity: Entity, world: &mut World) -> Option<Transition> {
        // Check current an all surrounding rooms for the player we are chasing.
        let room = match world.get::<Location>(entity) {
            Some(location) => location.room,
            None => return None,
        };

        // get the list of rooms attached to the current room
        let mut rooms = vec![(None, room)];
        world
            .get::<Room>(room)
            .unwrap()
            .exits
            .iter()
            .for_each(|(d, r)| rooms.push((Some(*d), *r)));

        // determine the room the player is in and its direction.
        let mut player_room = None;
        for (dir, room) in rooms {
            if world
                .get::<Room>(room)
                .unwrap()
                .players
                .iter()
                .any(|p| *p == self.chasing.unwrap())
            {
                player_room = Some(room);
                self.move_direction = dir;
                break;
            }
        }

        if player_room.is_none() {
            Some(Transition::LostPlayer)
        } else {
            None
        }
    }

    fn act(&mut self, entity: Entity, world: &mut World) {
        self.tick_timer += 1;

        // player is still around, follow them if necessary.
        if self.tick_timer > 120 && self.move_direction.is_some() {
            let mut events = world.get_resource_mut::<Events<QueuedAction>>().unwrap();
            events.send(QueuedAction {
                action: Action::Move(Move {
                    entity,
                    direction: self.move_direction.unwrap(),
                }),
            });

            // gotta reset these to prevent badness and sadness
            self.tick_timer = 0;
            self.move_direction = None;
        }
    }

    fn on_exit(&mut self, _: Entity, _: &mut World) {
        self.chasing = None;
        self.move_direction = None;
    }

    fn output_state(&self, next: Transition) -> Option<StateId> {
        match next {
            Transition::LostPlayer => Some(StateId::Wander),
            _ => None,
        }
    }
}
