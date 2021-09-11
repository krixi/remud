use std::collections::{HashMap, VecDeque};

use bevy_ecs::prelude::*;

use crate::world::types::Contents;

#[derive(Bundle)]
pub struct PlayerBundle {
    pub player: Player,
    pub contents: Contents,
}

pub struct Player {
    pub id: i64,
    pub name: String,
    pub room: Entity,
}

pub struct Messages {
    pub received_input: bool,
    pub queue: VecDeque<String>,
}

impl Messages {
    pub fn new_with(message: String) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back(message);

        Messages {
            received_input: false,
            queue,
        }
    }

    pub fn queue(&mut self, message: String) {
        self.queue.push_back(message);
    }
}

#[derive(Default)]
pub struct Players {
    by_name: HashMap<String, Entity>,
}

impl Players {
    pub fn by_name(&self, name: &str) -> Option<Entity> {
        self.by_name.get(name).copied()
    }

    pub fn insert(&mut self, player: Entity, name: String) {
        self.by_name.insert(name, player);
    }

    pub fn remove(&mut self, name: &str) {
        self.by_name.remove(name);
    }
}
