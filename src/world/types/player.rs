use std::collections::{hash_set, HashMap, HashSet, VecDeque};

use bevy_ecs::prelude::*;

pub struct Player {
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
    by_room: HashMap<Entity, HashSet<Entity>>,
}

impl Players {
    pub fn by_room(&self, room: Entity) -> PlayersByRoom {
        if let Some(players) = self.by_room.get(&room) {
            PlayersByRoom {
                iter: Some(players.iter()),
            }
        } else {
            PlayersByRoom { iter: None }
        }
    }

    pub fn by_name(&self, name: &str) -> Option<Entity> {
        self.by_name.get(name).copied()
    }

    pub fn spawn(&mut self, player: Entity, name: String, room: Entity) {
        self.by_room.entry(room).or_default().insert(player);
        self.by_name.insert(name, player);
    }

    pub fn despawn(&mut self, player: Entity, name: &str, room: Entity) {
        self.by_room.entry(room).and_modify(|players| {
            players.remove(&player);
        });
        self.by_name.remove(name);
    }

    pub fn change_room(&mut self, player: Entity, from: Entity, to: Entity) {
        if let Some(list) = self.by_room.get_mut(&from) {
            list.remove(&player);
        }

        self.by_room.entry(to).or_default().insert(player);
    }
}

pub struct PlayersByRoom<'a> {
    iter: Option<hash_set::Iter<'a, Entity>>,
}

impl<'a> Iterator for PlayersByRoom<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = &mut self.iter {
            iter.next().copied()
        } else {
            None
        }
    }
}
