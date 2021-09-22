use std::{
    cmp::Reverse,
    time::{Duration, Instant},
};

use bevy_app::EventWriter;
use bevy_ecs::prelude::*;
use priority_queue::PriorityQueue;

use crate::world::{action::Action, scripting::QueuedAction};

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
