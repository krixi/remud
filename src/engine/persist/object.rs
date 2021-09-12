use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    engine::persist::Persist,
    world::{
        action::{DEFAULT_OBJECT_KEYWORD, DEFAULT_OBJECT_LONG, DEFAULT_OBJECT_SHORT},
        types::object::{self, Id},
    },
};

pub struct New {
    id: object::Id,
}

impl New {
    pub fn new(id: object::Id) -> Box<Self> {
        Box::new(New { id })
    }
}

#[async_trait]
impl Persist for New {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO objects (id, keywords, short, long) VALUES (?, ?, ?, ?)")
            .bind(self.id)
            .bind(DEFAULT_OBJECT_KEYWORD)
            .bind(DEFAULT_OBJECT_SHORT)
            .bind(DEFAULT_OBJECT_LONG)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Update {
    id: object::Id,
    keywords: Vec<String>,
    short: String,
    long: String,
}

impl Update {
    pub fn new(id: object::Id, keywords: Vec<String>, short: String, long: String) -> Box<Self> {
        Box::new(Update {
            id,
            keywords,
            short,
            long,
        })
    }
}

#[async_trait]
impl Persist for Update {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        let keywords = self.keywords.join(",");
        sqlx::query("UPDATE objects SET keywords = ?, short = ?, long = ? WHERE id = ?")
            .bind(keywords)
            .bind(&self.short)
            .bind(&self.long)
            .bind(self.id)
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
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM objects WHERE id = ?")
            .bind(self.object_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}
