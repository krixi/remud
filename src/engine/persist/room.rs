use anyhow::bail;
use async_trait::async_trait;
use bevy_ecs::prelude::*;
use sqlx::SqlitePool;

use crate::{
    engine::persist::Persist,
    world::types::{
        object::Object,
        room::{self, Room},
    },
};

pub struct AddObject {
    room: Entity,
    object: Entity,
}

impl AddObject {
    pub fn new(room: Entity, object: Entity) -> Box<Self> {
        Box::new(AddObject { room, object })
    }
}

#[async_trait]
impl Persist for AddObject {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()> {
        let room_id = match world.get::<Room>(self.room).map(|room| room.id) {
            Some(id) => id,
            None => bail!("Room {:?} does not have Room", self.room),
        };

        let object_id = match world.get::<Object>(self.object).map(|object| object.id) {
            Some(id) => id,
            None => bail!("Object {:?} does not have Object", self.object),
        };

        sqlx::query("INSERT INTO room_objects (room_id, object_id) VALUES (?, ?)")
            .bind(room_id)
            .bind(object_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct New {
    room: Entity,
}

impl New {
    pub fn new(room: Entity) -> Box<Self> {
        Box::new(New { room })
    }
}

#[async_trait]
impl Persist for New {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()> {
        let room = match world.get::<Room>(self.room) {
            Some(room) => room,
            None => {
                bail!(
                    "Failed to persist created room, room does not exist: {:?}",
                    self.room
                );
            }
        };

        sqlx::query("INSERT INTO rooms (id, description) VALUES (?, ?)")
            .bind(room.id)
            .bind(&room.description)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Remove {
    room_id: room::Id,
}

impl Remove {
    pub fn new(room_id: room::Id) -> Box<Self> {
        Box::new(Remove { room_id })
    }
}

#[async_trait]
impl Persist for Remove {
    async fn enact(&self, pool: &SqlitePool, _world: &World) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM rooms WHERE id = ?")
            .bind(self.room_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct RemoveObject {
    room: Entity,
    object: Entity,
}

impl RemoveObject {
    pub fn new(room: Entity, object: Entity) -> Box<Self> {
        Box::new(RemoveObject { room, object })
    }
}

#[async_trait]
impl Persist for RemoveObject {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()> {
        let room_id = match world.get::<Room>(self.room).map(|room| room.id) {
            Some(id) => id,
            None => bail!("Room {:?} does not have Room", self.room),
        };

        let object_id = match world.get::<Object>(self.object).map(|object| object.id) {
            Some(id) => id,
            None => bail!("Object {:?} does not have Object", self.object),
        };

        sqlx::query("DELETE FROM room_objects WHERE room_id = ? AND object_id = ?")
            .bind(room_id)
            .bind(object_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Update {
    room: Entity,
}

impl Update {
    pub fn new(room: Entity) -> Box<Self> {
        Box::new(Update { room })
    }
}

#[async_trait]
impl Persist for Update {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()> {
        let room = match world.get::<Room>(self.room) {
            Some(room) => room,
            None => {
                bail!(
                    "Failed to persist room update, room does not exist: {:?}",
                    self.room
                );
            }
        };

        sqlx::query("UPDATE rooms SET description = ? WHERE id = ?")
            .bind(&room.description)
            .bind(room.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Exits {
    room: Entity,
}

impl Exits {
    pub fn new(room: Entity) -> Box<Self> {
        Box::new(Exits { room })
    }
}

#[async_trait]
impl Persist for Exits {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()> {
        let room = match world.get::<Room>(self.room) {
            Some(room) => room,
            None => {
                bail!(
                    "Failed to persist room exits, room does not exist: {:?}",
                    self.room
                );
            }
        };

        sqlx::query("DELETE FROM exits WHERE room_from = ?")
            .bind(room.id)
            .execute(pool)
            .await?;

        for (direction, destination) in &room.exits {
            let to_room = match world.get::<Room>(*destination) {
                Some(room) => room,
                None => bail!(
                    "Failed to retrieve destination room during exit update: {:?} -> {:?} ({:?})",
                    self.room,
                    destination,
                    direction
                ),
            };

            sqlx::query("INSERT INTO exits (room_from, room_to, direction) VALUES (?, ?, ?)")
                .bind(room.id)
                .bind(to_room.id)
                .bind(direction.as_str())
                .execute(pool)
                .await?;
        }

        Ok(())
    }
}
