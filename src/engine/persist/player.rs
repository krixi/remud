use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    engine::persist::Persist,
    world::types::{object::ObjectId, player::PlayerId, room::RoomId},
};

pub struct AddObject {
    player_id: PlayerId,
    object_id: ObjectId,
}

impl AddObject {
    pub fn new(player_id: PlayerId, object_id: ObjectId) -> Box<Self> {
        Box::new(AddObject {
            player_id,
            object_id,
        })
    }
}

#[async_trait]
impl Persist for AddObject {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO player_objects (player_id, object_id) VALUES (?, ?)")
            .bind(self.player_id)
            .bind(self.object_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Description {
    id: PlayerId,
    description: String,
}

impl Description {
    pub fn new(id: PlayerId, description: String) -> Box<Self> {
        Box::new(Description { id, description })
    }
}

#[async_trait]
impl Persist for Description {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE players SET description = ? WHERE id = ?")
            .bind(self.description.as_str())
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct RemoveObject {
    player_id: PlayerId,
    object_id: ObjectId,
}

impl RemoveObject {
    pub fn new(player_id: PlayerId, object_id: ObjectId) -> Box<Self> {
        Box::new(RemoveObject {
            player_id,
            object_id,
        })
    }
}

#[async_trait]
impl Persist for RemoveObject {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM player_objects WHERE player_id = ? AND object_id = ?")
            .bind(self.player_id)
            .bind(self.object_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Room {
    player_id: PlayerId,
    room_id: RoomId,
}

impl Room {
    pub fn new(player_id: PlayerId, room_id: RoomId) -> Box<Self> {
        Box::new(Room { player_id, room_id })
    }
}

#[async_trait]
impl Persist for Room {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE players SET room = ? WHERE id = ?")
            .bind(self.room_id)
            .bind(self.player_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}
