use std::mem;

use anyhow::bail;
use async_trait::async_trait;
use bevy_ecs::prelude::*;
use sqlx::SqlitePool;

use crate::world::types::{
    object::{Location, Object, ObjectId},
    room::{Room, RoomId},
};

pub type DynUpdate = Box<dyn Update + Send + Sync>;

#[derive(Default)]
pub struct Updates {
    updates: Vec<DynUpdate>,
}

impl Updates {
    pub fn queue(&mut self, update: DynUpdate) {
        self.updates.push(update);
    }

    pub fn take(&mut self) -> Vec<DynUpdate> {
        let mut updates = Vec::new();
        mem::swap(&mut self.updates, &mut updates);
        updates
    }
}

#[async_trait]
pub trait Update {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()>;
}

pub struct PersistUpdateObject {
    object: Entity,
}

impl PersistUpdateObject {
    pub fn new(object: Entity) -> Box<Self> {
        Box::new(PersistUpdateObject { object })
    }
}

#[async_trait]
impl Update for PersistUpdateObject {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()> {
        let object = match world.get::<Object>(self.object) {
            Some(object) => object,
            None => bail!(
                "Failed to persist object updates, object does not exist: {:?}",
                self.object
            ),
        };

        let keywords = object.keywords.join(",");

        sqlx::query("UPDATE objects SET keywords = ?, short = ?, long = ? WHERE id = ?")
            .bind(keywords)
            .bind(&object.short)
            .bind(&object.long)
            .bind(object.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct PersistNewObject {
    object: Entity,
}

impl PersistNewObject {
    pub fn new(object: Entity) -> Box<Self> {
        Box::new(PersistNewObject { object })
    }
}

#[async_trait]
impl Update for PersistNewObject {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()> {
        let object = match world.get::<Object>(self.object) {
            Some(object) => object,
            None => bail!(
                "Failed to persist new object, object does not exist: {:?}",
                self.object
            ),
        };

        let keywords = object.keywords.join(",");

        sqlx::query("INSERT INTO objects (id, keywords, short, long) VALUES (?, ?, ?, ?)")
            .bind(object.id)
            .bind(keywords)
            .bind(&object.short)
            .bind(&object.long)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct PersistNewRoom {
    room: Entity,
}

impl PersistNewRoom {
    pub fn new(room: Entity) -> Box<Self> {
        Box::new(PersistNewRoom { room })
    }
}

#[async_trait]
impl Update for PersistNewRoom {
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

pub struct PersistRoomExits {
    room: Entity,
}

impl PersistRoomExits {
    pub fn new(room: Entity) -> Box<Self> {
        Box::new(PersistRoomExits { room })
    }
}

#[async_trait]
impl Update for PersistRoomExits {
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

pub struct PersistObjectLocation {
    object: Entity,
}

impl PersistObjectLocation {
    pub fn new(object: Entity) -> Box<Self> {
        Box::new(PersistObjectLocation { object })
    }
}

#[async_trait]
impl Update for PersistObjectLocation {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()> {
        let (object_id, location) = match world.get::<Object>(self.object) {
            Some(object) => (object.id, object.location),
            None => bail!(
                "Failed to persist object room, object does not exist: {:?}",
                self.object
            ),
        };

        match location {
            Location::Room(room) => {
                let room_id = match world.get::<Room>(room) {
                    Some(room) => room.id,
                    None => bail!(
                        "Failed to persist object room, room does not exist: {:?}",
                        room
                    ),
                };

                sqlx::query("INSERT INTO room_objects (room_id, object_id) VALUES (?, ?)")
                    .bind(room_id)
                    .bind(object_id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }
}

pub struct PersistRemoveObject {
    object_id: ObjectId,
}

impl PersistRemoveObject {
    pub fn new(object_id: ObjectId) -> Box<Self> {
        Box::new(PersistRemoveObject { object_id })
    }
}

#[async_trait]
impl Update for PersistRemoveObject {
    async fn enact(&self, pool: &SqlitePool, _world: &World) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM objects WHERE id = ?")
            .bind(self.object_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct PersistRemoveRoom {
    room_id: RoomId,
}

impl PersistRemoveRoom {
    pub fn new(room_id: RoomId) -> Box<Self> {
        Box::new(PersistRemoveRoom { room_id })
    }
}

#[async_trait]
impl Update for PersistRemoveRoom {
    async fn enact(&self, pool: &SqlitePool, _world: &World) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM rooms WHERE id = ?")
            .bind(self.room_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct PersistRoomUpdates {
    room: Entity,
}

impl PersistRoomUpdates {
    pub fn new(room: Entity) -> Box<Self> {
        Box::new(PersistRoomUpdates { room })
    }
}

#[async_trait]
impl Update for PersistRoomUpdates {
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
