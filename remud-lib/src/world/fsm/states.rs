use crate::{
    ecs::WorldExt,
    world::{
        action::{communicate::Say, movement::Move, Action},
        fsm::{State, StateId, Transition},
        scripting::QueuedAction,
        types::{
            room::{Direction, Room},
            Named,
        },
    },
};
use bevy_app::Events;
use bevy_ecs::prelude::*;
use itertools::Itertools;
use rand::{prelude::*, thread_rng};
use rhai::{Array, ImmutableString};
use std::{collections::HashMap, time::Duration};
use std::{convert::TryInto, fmt::Debug};

/// Wander around and look for the player
#[derive(Debug, Default)]
pub struct WanderState {
    find_player: bool,
    min_stay: u64,
    max_stay: u64,
    endless: bool,
    min_wanders: u64,
    max_wanders: u64,
    tx: HashMap<Transition, StateId>,

    current_wanders: u64,
    current_max_wanders: u64,
}

impl WanderState {
    const TIMER_NAME: &'static str = "wander_state_timer";

    pub fn new(params: rhai::Map, tx: rhai::Array) -> Self {
        let find_player = params
            .get("find_player")
            .and_then(|p| p.as_bool().ok())
            .unwrap_or(false);

        let min_stay = params
            .get("min_stay")
            .and_then(|p| p.as_int().ok())
            .and_then(|p| p.try_into().ok())
            .unwrap_or(10 * 1000);

        let max_stay = params
            .get("max_stay")
            .and_then(|p| p.as_int().ok())
            .and_then(|p| p.try_into().ok())
            .map(|p| if p < min_stay { min_stay } else { p })
            .unwrap_or(60 * 1000);

        let endless = !params.contains_key("min_wanders") && !params.contains_key("max_wanders");

        let min_wanders = params
            .get("min_wanders")
            .and_then(|p| p.as_int().ok())
            .and_then(|p| p.try_into().ok())
            .unwrap_or(2);

        let max_wanders = params
            .get("max_wanders")
            .and_then(|p| p.as_int().ok())
            .and_then(|p| p.try_into().ok())
            .map(|p| if p < min_wanders { min_wanders } else { p })
            .unwrap_or(5);

        WanderState {
            find_player,
            min_stay,
            max_stay,
            endless,
            min_wanders,
            max_wanders,
            current_wanders: 0,
            current_max_wanders: 0,
            tx: build_tx_map(tx),
        }
    }

    fn set_timer(&self, entity: Entity, world: &mut World) {
        world.with_timers(entity, |timers| {
            timers.add(
                Self::TIMER_NAME.to_string(),
                Duration::from_millis(thread_rng().gen_range(self.min_stay..=self.max_stay)),
            );
        })
    }
}

impl State for WanderState {
    #[tracing::instrument(name = "wander on enter")]
    fn on_enter(&mut self, entity: Entity, world: &mut World) {
        self.set_timer(entity, world);
        self.current_wanders = 0;
        self.current_max_wanders = thread_rng().gen_range(self.min_wanders..=self.max_wanders);
    }

    #[tracing::instrument(name = "wander decide")]
    fn decide(&mut self, entity: Entity, world: &mut World) -> Option<Transition> {
        // If looking for players, check any they have entered the room.
        if self.find_player {
            // If there is no location, the actor cannot see them as they are not in a room.
            let room = world.location_of(entity);
            let players = world.get::<Room>(room).unwrap().players();

            if !players.is_empty() {
                return Some(Transition::SawTarget);
            }
        }

        if !self.endless && self.current_wanders >= self.current_max_wanders {
            return Some(Transition::Done);
        }

        None
    }

    #[tracing::instrument(name = "wander act")]
    fn act(&mut self, entity: Entity, world: &mut World) {
        // Check to see if wander time has elapsed
        let mut done_waiting = false;
        world.with_timers(entity, |timers| {
            done_waiting = timers.finished(Self::TIMER_NAME)
        });

        if !done_waiting {
            return;
        }

        // Reset our timer and do bookkeeping
        self.set_timer(entity, world);
        self.current_wanders += 1;

        // Pick an exit and go through it.
        let room = world.location_of(entity);
        let exits = &world.get::<Room>(room).unwrap().exits();

        if !exits.is_empty() {
            // Choose a random direction.
            let exit = exits.keys().choose(&mut thread_rng());

            // If a suitable exit was found, queue a move action.
            if let Some(exit) = exit.copied() {
                let mut events = world.get_resource_mut::<Events<QueuedAction>>().unwrap();
                events.send(QueuedAction {
                    action: Action::Move(Move {
                        actor: entity,
                        direction: exit,
                    }),
                })
            }
        }
    }

    fn output_state(&self, next: Transition) -> Option<StateId> {
        self.tx.get(&next).copied()
    }
}

// Follow a player until the actor loses sight of them
#[derive(Debug, Default)]
pub struct FollowState {
    target: Option<Entity>,
    acquisition_messages: Option<Vec<String>>,
    min_wait: u64,
    max_wait: u64,
    tx: HashMap<Transition, StateId>,

    move_direction: Option<Direction>,
}

impl FollowState {
    const TIMER_NAME: &'static str = "follow_state_timer";

    pub fn new(params: rhai::Map, tx: rhai::Array) -> Self {
        let target = params
            .get("follow")
            .cloned()
            .and_then(|p| p.try_cast::<Entity>());

        let found_say = if let Some(found_say) = params.get("found_say") {
            if found_say.is::<Array>() {
                let list = found_say
                    .clone()
                    .cast::<Array>()
                    .iter()
                    .filter_map(|p| p.clone().as_string().ok())
                    .collect_vec();

                if list.is_empty() {
                    None
                } else {
                    Some(list)
                }
            } else if found_say.is::<ImmutableString>() {
                Some(vec![found_say.clone().as_string().unwrap()])
            } else {
                None
            }
        } else {
            None
        };

        let min_wait = params
            .get("min_wait")
            .and_then(|p| p.as_int().ok())
            .and_then(|p| p.try_into().ok())
            .unwrap_or(1000);

        let max_wait = params
            .get("max_wait")
            .and_then(|p| p.as_int().ok())
            .and_then(|p| p.try_into().ok())
            .map(|p| if p < min_wait { min_wait } else { p })
            .unwrap_or(5 * 1000);

        FollowState {
            target,
            acquisition_messages: found_say,
            min_wait,
            max_wait,
            tx: build_tx_map(tx),
            ..Default::default()
        }
    }
}

impl State for FollowState {
    #[tracing::instrument(name = "follow on enter")]
    fn on_enter(&mut self, entity: Entity, world: &mut World) {
        if self.target.is_none() {
            // Choose a player in the room to follow.
            let room = world.location_of(entity);
            let players = world.get::<Room>(room).unwrap().players();
            let player = *players.choose(&mut thread_rng()).unwrap();

            self.target = Some(player);

            // Say a message on target acquisition if configured
            if let Some(messages) = &self.acquisition_messages {
                let player_name = world.get::<Named>(player).unwrap().to_string();

                let message = messages.choose(&mut thread_rng()).unwrap();

                let mut events = world.get_resource_mut::<Events<QueuedAction>>().unwrap();
                events.send(QueuedAction {
                    action: Action::Say(Say {
                        actor: entity,
                        message: message.replace("${name}", player_name.as_str()),
                    }),
                })
            }
        }
    }

    #[tracing::instrument(name = "follow decide")]
    fn decide(&mut self, entity: Entity, world: &mut World) -> Option<Transition> {
        // Check current and surrounding rooms for the player we are following.
        // Initialize our move list with the current room and movement direction of None.
        let room = world.location_of(entity);
        let mut rooms = vec![(None, room)];
        world
            .get::<Room>(room)
            .unwrap()
            .exits()
            .iter()
            .for_each(|(d, r)| rooms.push((Some(*d), *r)));

        // Check each room to determine if it contains the target, extracting the movement direction.
        let mut target_found = false;
        for (dir, room) in rooms {
            if world
                .get::<Room>(room)
                .unwrap()
                .players()
                .iter()
                .any(|p| *p == self.target.unwrap())
            {
                target_found = true;
                self.move_direction = dir;

                // If we need to move, start a timer to determine how long to wait.
                if self.move_direction.is_some() {
                    world.with_timers(entity, |t| {
                        t.add(
                            Self::TIMER_NAME.to_string(),
                            Duration::from_millis(
                                thread_rng().gen_range(self.min_wait..=self.max_wait),
                            ),
                        )
                    });
                }

                break;
            }
        }

        // Transition away if we've lost the target
        if target_found {
            None
        } else {
            Some(Transition::Done)
        }
    }

    #[tracing::instrument(name = "follow act")]
    fn act(&mut self, entity: Entity, world: &mut World) {
        // We know where the target is, follow them.
        if let Some(direction) = self.move_direction {
            let mut done_waiting = false;
            world.with_timers(entity, |t| done_waiting = t.finished(Self::TIMER_NAME));

            if done_waiting {
                let mut events = world.get_resource_mut::<Events<QueuedAction>>().unwrap();
                events.send(QueuedAction {
                    action: Action::Move(Move {
                        actor: entity,
                        direction,
                    }),
                });

                self.move_direction = None;
            }
        }
    }

    #[tracing::instrument(name = "follow on exit")]
    fn on_exit(&mut self, _: Entity, _: &mut World) {
        self.target = None;
        self.move_direction = None;
    }

    fn output_state(&self, next: Transition) -> Option<StateId> {
        self.tx.get(&next).copied()
    }
}

/// tx is an array of rhai objects, each with two keys:
// - when: Transition
// - then: StateId.
// We need to translate this into the tx hashmap.
#[tracing::instrument(name = "build_tx_map")]
fn build_tx_map(txs: rhai::Array) -> HashMap<Transition, StateId> {
    let mut result = HashMap::new();
    for ele in txs.iter() {
        if let Some(tx) = ele.clone().try_cast::<rhai::Map>() {
            let when = tx
                .get("when")
                .and_then(|w| w.clone().try_cast::<&Transition>());
            let then = tx
                .get("then")
                .and_then(|t| t.clone().try_cast::<&StateId>());
            if let (Some(when), Some(then)) = (when, then) {
                result.insert(*when, *then);
            } else {
                tracing::warn!(
                    "unable to convert when and then to valid transition -> stateId pair. Got \
                     {:?} -> {:?}",
                    when,
                    then
                );
            }
        }
    }
    result
}
