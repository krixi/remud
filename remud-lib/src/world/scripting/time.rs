use std::{
    cmp::Reverse,
    collections::HashMap,
    time::{Duration, Instant},
};

use bevy_app::EventWriter;
use bevy_core::{Time, Timer};
use bevy_ecs::prelude::*;
use priority_queue::PriorityQueue;

use crate::world::{action::Action, scripting::QueuedAction};

#[derive(Default)]
pub struct Timers {
    map: HashMap<String, Timer>,
}

impl Timers {
    pub fn timers(&self) -> &HashMap<String, Timer> {
        &self.map
    }

    pub fn add(&mut self, name: String, duration: Duration) {
        self.map.insert(name, Timer::new(duration, false));
    }

    pub fn remove(&mut self, name: &str) {
        self.map.remove(name);
    }

    pub fn add_repeating(&mut self, name: String, duration: Duration) {
        self.map.insert(name, Timer::new(duration, true));
    }

    pub fn finished(&mut self, name: &str) -> bool {
        self.map.get(name).map(|t| t.finished()).unwrap_or(true)
    }

    pub fn list_finished(&mut self) -> Vec<String> {
        let mut elapsed = Vec::new();

        for (name, timer) in self.map.iter() {
            if timer.finished() {
                elapsed.push(name.clone());
            }
        }

        elapsed
    }
}

pub fn tick_timers_system(time: Res<Time>, mut timers_query: Query<&mut Timers>) {
    for mut timers in timers_query.iter_mut() {
        for timer in timers.map.values_mut() {
            timer.tick(time.delta());
        }
    }
}

pub fn timer_cleanup_system(mut timers_query: Query<&mut Timers>) {
    for mut timers in timers_query.iter_mut() {
        let mut to_remove = Vec::new();

        for (name, timer) in timers.map.iter() {
            if timer.finished() && !timer.repeating() {
                to_remove.push(name.clone());
            }
        }

        for name in to_remove {
            timers.remove(name.as_str());
        }
    }
}

// Used to prevent deduplication of items in the priority queue.
#[derive(Debug, Hash, Eq, PartialEq)]
pub struct TimedAction {
    id: u64,
    action: Action,
}

impl TimedAction {
    fn new(action: Action, id: u64) -> Self {
        TimedAction { action, id }
    }
}

#[derive(Default)]
pub struct TimedActions {
    pub id: u64,
    pub queue: PriorityQueue<TimedAction, Reverse<Instant>>,
}

impl TimedActions {
    pub fn send_at(&mut self, action: Action, instant: Instant) {
        self.queue
            .push(TimedAction::new(action, self.id), Reverse(instant));
        self.id += 1;
    }

    pub fn send_after(&mut self, action: Action, duration: Duration) {
        self.send_at(action, Instant::now() + duration);
    }

    pub fn pop_ready(&mut self) -> Option<Action> {
        if let Some((_, time)) = self.queue.peek() {
            // Use the un-reversed time to check if it has passed
            if time.0 <= Instant::now() {
                return self.queue.pop().map(|(action, _)| action.action);
            }
        }
        None
    }
}

pub fn timed_actions_system(
    mut queued_action_writer: EventWriter<QueuedAction>,
    mut actions: ResMut<TimedActions>,
) {
    while let Some(action) = actions.pop_ready() {
        queued_action_writer.send(action.into());
    }
}
