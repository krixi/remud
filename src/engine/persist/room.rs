use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::{
    engine::persist::Persist,
    world::types::{
        object::ObjectId,
        room::{Direction, RoomId},
    },
};

pub struct AddExit {
    from_id: RoomId,
    to_id: RoomId,
    direction: Direction,
}

impl AddExit {
    pub fn new(from_id: RoomId, to_id: RoomId, direction: Direction) -> Box<Self> {
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

pub struct AddObject {
    room_id: RoomId,
    object_id: ObjectId,
}

impl AddObject {
    pub fn new(room_id: RoomId, object_id: ObjectId) -> Box<Self> {
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

pub struct Create {
    id: RoomId,
    description: String,
}

impl Create {
    pub fn new(id: RoomId, description: String) -> Box<Self> {
        Box::new(Create { id, description })
    }
}

#[async_trait]
impl Persist for Create {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO rooms (id, description) VALUES (?, ?)")
            .bind(self.id)
            .bind(self.description.as_str())
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Delete {
    id: RoomId,
}

impl Delete {
    pub fn new(id: RoomId) -> Box<Self> {
        Box::new(Delete { id })
    }
}

#[async_trait]
impl Persist for Delete {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM rooms WHERE id = ?")
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct RemoveExit {
    id: RoomId,
    direction: Direction,
}

impl RemoveExit {
    pub fn new(id: RoomId, direction: Direction) -> Box<Self> {
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

pub struct RemoveObject {
    room_id: RoomId,
    object_id: ObjectId,
}

impl RemoveObject {
    pub fn new(room_id: RoomId, object_id: ObjectId) -> Box<Self> {
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

pub struct UpdateDescription {
    id: RoomId,
    description: String,
}

impl UpdateDescription {
    pub fn new(id: RoomId, description: String) -> Box<Self> {
        Box::new(UpdateDescription { id, description })
    }
}

#[async_trait]
impl Persist for UpdateDescription {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE rooms SET description = ? WHERE id = ?")
            .bind(self.description.as_str())
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct AddRegions {
    id: RoomId,
    regions: Vec<String>,
}

impl AddRegions {
    pub fn new(id: RoomId, regions: Vec<String>) -> Box<Self> {
        Box::new(AddRegions { id, regions })
    }
}

#[async_trait]
impl Persist for AddRegions {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        for region in self.regions.iter() {
            let region_id: i64 = if let Ok(region_row) =
                sqlx::query("SELECT id FROM regions WHERE name = ?")
                    .bind(region)
                    .fetch_one(pool)
                    .await
            {
                region_row.get("id")
            } else {
                sqlx::query("INSERT INTO regions(name) VALUES(?) RETURNING id")
                    .bind(region)
                    .fetch_one(pool)
                    .await?
                    .get("id")
            };

            sqlx::query("INSERT INTO room_regions(room_id, region_id) VALUES(?, ?)")
                .bind(self.id)
                .bind(region_id)
                .execute(pool)
                .await?;
        }

        Ok(())
    }
}

pub struct RemoveRegions {
    id: RoomId,
    regions: Vec<String>,
}

impl RemoveRegions {
    pub fn new(id: RoomId, regions: Vec<String>) -> Box<Self> {
        Box::new(RemoveRegions { id, regions })
    }
}

#[async_trait]
impl Persist for RemoveRegions {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        for region in self.regions.iter() {
            let region_id: i64 = if let Ok(region_row) =
                sqlx::query("SELECT id FROM regions WHERE name = ?")
                    .bind(region)
                    .fetch_one(pool)
                    .await
            {
                region_row.get("id")
            } else {
                sqlx::query("INSERT INTO regions(name) VALUES(?) RETURNING id")
                    .bind(region)
                    .fetch_one(pool)
                    .await?
                    .get("id")
            };

            sqlx::query("DELETE FROM room_regions WHERE room_id = ? AND region_id = ?")
                .bind(self.id)
                .bind(region_id)
                .execute(pool)
                .await?;
        }

        Ok(())
    }
}
