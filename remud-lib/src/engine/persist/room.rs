use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use tracing::Instrument;

use crate::{
    engine::persist::Persist,
    world::types::{
        object::ObjectId,
        room::{Direction, RoomId},
    },
};

#[derive(Debug)]
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
    #[tracing::instrument(name = "add room exit", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO exits (room_from, room_to, direction) VALUES (?, ?, ?)")
            .bind(self.from_id)
            .bind(self.to_id)
            .bind(self.direction.as_str())
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
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
    #[tracing::instrument(name = "add room object", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO room_objects (room_id, object_id) VALUES (?, ?)")
            .bind(self.room_id)
            .bind(self.object_id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}
#[derive(Debug)]
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
    #[tracing::instrument(name = "add room regions", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        for region in self.regions.iter() {
            let region_id: i64 = if let Some(region_row) =
                sqlx::query("SELECT id FROM regions WHERE name = ?")
                    // .bind(region)
                    .fetch_optional(pool)
                    .in_current_span()
                    .await?
            {
                region_row.get("id")
            } else {
                sqlx::query("INSERT INTO regions(name) VALUES(?) RETURNING id")
                    .bind(region)
                    .fetch_one(pool)
                    .in_current_span()
                    .await?
                    .get("id")
            };

            sqlx::query("INSERT INTO room_regions(room_id, region_id) VALUES(?, ?)")
                .bind(self.id)
                .bind(region_id)
                .execute(pool)
                .in_current_span()
                .await?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Create {
    id: RoomId,
    name: String,
    description: String,
}

impl Create {
    pub fn new(id: RoomId, name: String, description: String) -> Box<Self> {
        Box::new(Create {
            id,
            name,
            description,
        })
    }
}

#[async_trait]
impl Persist for Create {
    #[tracing::instrument(name = "add room", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO rooms (id, name, description) VALUES (?, ?, ?)")
            .bind(self.id)
            .bind(self.name.as_str())
            .bind(self.description.as_str())
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
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
    #[tracing::instrument(name = "remove room", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM rooms WHERE id = ?")
            .bind(self.id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Description {
    id: RoomId,
    description: String,
}

impl Description {
    pub fn new(id: RoomId, description: String) -> Box<Self> {
        Box::new(Description { id, description })
    }
}

#[async_trait]
impl Persist for Description {
    #[tracing::instrument(name = "update room description", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE rooms SET description = ? WHERE id = ?")
            .bind(self.description.as_str())
            .bind(self.id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Name {
    id: RoomId,
    name: String,
}

impl Name {
    pub fn new(id: RoomId, name: String) -> Box<Self> {
        Box::new(Name { id, name })
    }
}

#[async_trait]
impl Persist for Name {
    #[tracing::instrument(name = "update room name", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE rooms SET name = ? WHERE id = ?")
            .bind(self.name.as_str())
            .bind(self.id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
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
    #[tracing::instrument(name = "update room exit", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM exits WHERE room_from = ? AND direction = ?")
            .bind(self.id)
            .bind(self.direction.as_str())
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
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
    #[tracing::instrument(name = "remove room object", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM room_objects WHERE room_id = ? AND object_id = ?")
            .bind(self.room_id)
            .bind(self.object_id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
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
    #[tracing::instrument(name = "remove room regions", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        for region in self.regions.iter() {
            if let Some(region_row) = sqlx::query("SELECT id FROM regions WHERE name = ?")
                .bind(region)
                .fetch_optional(pool)
                .in_current_span()
                .await?
            {
                let region_id: i64 = region_row.get("id");

                sqlx::query("DELETE FROM room_regions WHERE room_id = ? AND region_id = ?")
                    .bind(self.id)
                    .bind(region_id)
                    .execute(pool)
                    .in_current_span()
                    .await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct SetRegions {
    id: RoomId,
    regions: Vec<String>,
}

impl SetRegions {
    pub fn new(id: RoomId, regions: Vec<String>) -> Box<Self> {
        Box::new(SetRegions { id, regions })
    }
}

#[async_trait]
impl Persist for SetRegions {
    #[tracing::instrument(name = "set room regions", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM room_regions WHERE room_id = ?")
            .bind(self.id)
            .execute(pool)
            .in_current_span()
            .await?;

        for region in self.regions.iter() {
            let region_id: i64 = if let Some(region_row) =
                sqlx::query("SELECT id FROM regions WHERE name = ?")
                    .bind(region)
                    .fetch_optional(pool)
                    .in_current_span()
                    .await?
            {
                region_row.get("id")
            } else {
                sqlx::query("INSERT INTO regions(name) VALUES(?) RETURNING id")
                    .bind(region)
                    .fetch_one(pool)
                    .in_current_span()
                    .await?
                    .get("id")
            };

            sqlx::query("INSERT INTO room_regions(room_id, region_id) VALUES(?, ?)")
                .bind(self.id)
                .bind(region_id)
                .execute(pool)
                .in_current_span()
                .await?;
        }

        Ok(())
    }
}
