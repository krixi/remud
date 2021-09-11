use anyhow::bail;
use async_trait::async_trait;
use bevy_ecs::prelude::*;
use sqlx::SqlitePool;

use crate::{
    engine::persist::Persist,
    world::types::object::{Id, Object},
};

pub struct New {
    object: Entity,
}

impl New {
    pub fn new(object: Entity) -> Box<Self> {
        Box::new(New { object })
    }
}

#[async_trait]
impl Persist for New {
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

pub struct Update {
    object: Entity,
}

impl Update {
    pub fn new(object: Entity) -> Box<Self> {
        Box::new(Update { object })
    }
}

#[async_trait]
impl Persist for Update {
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

pub struct Remove {
    object_id: Id,
}

impl Remove {
    pub fn new(object_id: Id) -> Box<Self> {
        Box::new(Remove { object_id })
    }
}

#[async_trait]
impl Persist for Remove {
    async fn enact(&self, pool: &SqlitePool, _world: &World) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM objects WHERE id = ?")
            .bind(self.object_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}
