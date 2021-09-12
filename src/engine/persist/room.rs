use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    engine::persist::Persist,
    world::types::{
        object::{self},
        room::{self, Direction},
    },
};

pub struct AddObject {
    room_id: room::Id,
    object_id: object::Id,
}

impl AddObject {
    pub fn new(room_id: room::Id, object_id: object::Id) -> Box<Self> {
        Box::new(AddObject { room_id, object_id })
    }
}

#[async_trait]
impl Persist for AddObject {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO room_objects (room_id, object_id) VALUES (?, ?)")
            .bind(self.room_id)
            .bind(self.object_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct New {
    id: room::Id,
    description: String,
}

impl New {
    pub fn new(id: room::Id, description: String) -> Box<Self> {
        Box::new(New { id, description })
    }
}

#[async_trait]
impl Persist for New {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO rooms (id, description) VALUES (?, ?)")
            .bind(self.id)
            .bind(self.description.as_str())
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Remove {
    id: room::Id,
}

impl Remove {
    pub fn new(id: room::Id) -> Box<Self> {
        Box::new(Remove { id })
    }
}

#[async_trait]
impl Persist for Remove {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM rooms WHERE id = ?")
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct RemoveObject {
    room_id: room::Id,
    object_id: object::Id,
}

impl RemoveObject {
    pub fn new(room_id: room::Id, object_id: object::Id) -> Box<Self> {
        Box::new(RemoveObject { room_id, object_id })
    }
}

#[async_trait]
impl Persist for RemoveObject {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM room_objects WHERE room_id = ? AND object_id = ?")
            .bind(self.room_id)
            .bind(self.object_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Update {
    id: room::Id,
    description: String,
}

impl Update {
    pub fn new(id: room::Id, description: String) -> Box<Self> {
        Box::new(Update { id, description })
    }
}

#[async_trait]
impl Persist for Update {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE rooms SET description = ? WHERE id = ?")
            .bind(self.description.as_str())
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct AddExit {
    from_id: room::Id,
    to_id: room::Id,
    direction: Direction,
}

impl AddExit {
    pub fn new(from_id: room::Id, to_id: room::Id, direction: Direction) -> Box<Self> {
        Box::new(AddExit {
            from_id,
            to_id,
            direction,
        })
    }
}

#[async_trait]
impl Persist for AddExit {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO exits (room_from, room_to, direction) VALUES (?, ?, ?)")
            .bind(self.from_id)
            .bind(self.to_id)
            .bind(self.direction.as_str())
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct RemoveExit {
    id: room::Id,
    direction: Direction,
}

impl RemoveExit {
    pub fn new(id: room::Id, direction: Direction) -> Box<Self> {
        Box::new(RemoveExit { id, direction })
    }
}

#[async_trait]
impl Persist for RemoveExit {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM exits WHERE room_from = ? AND direction = ?")
            .bind(self.id)
            .bind(self.direction.as_str())
            .execute(pool)
            .await?;

        Ok(())
    }
}
