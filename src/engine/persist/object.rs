use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    engine::persist::Persist,
    world::{
        action::{DEFAULT_OBJECT_KEYWORD, DEFAULT_OBJECT_LONG, DEFAULT_OBJECT_SHORT},
        types::object::{self, Id},
    },
};

pub struct Flags {
    id: object::Id,
    flags: object::Flags,
}

impl Flags {
    pub fn new(id: object::Id, flags: object::Flags) -> Box<Self> {
        Box::new(Flags { id, flags })
    }
}

#[async_trait]
impl Persist for Flags {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE objects SET flags = ? WHERE id = ?")
            .bind(self.flags.bits())
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Keywords {
    id: object::Id,
    keywords: Vec<String>,
}

impl Keywords {
    pub fn new(id: object::Id, keywords: Vec<String>) -> Box<Self> {
        Box::new(Keywords { id, keywords })
    }
}

#[async_trait]
impl Persist for Keywords {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        let keywords = self.keywords.join(",");
        sqlx::query("UPDATE objects SET keywords = ? WHERE id = ?")
            .bind(keywords)
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Long {
    id: object::Id,
    long: String,
}

impl Long {
    pub fn new(id: object::Id, long: String) -> Box<Self> {
        Box::new(Long { id, long })
    }
}

#[async_trait]
impl Persist for Long {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE objects SET long = ? WHERE id = ?")
            .bind(self.long.as_str())
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

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
        sqlx::query(
            "INSERT INTO objects (id, flags, keywords, short, long) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(self.id)
        .bind(0)
        .bind(DEFAULT_OBJECT_KEYWORD)
        .bind(DEFAULT_OBJECT_SHORT)
        .bind(DEFAULT_OBJECT_LONG)
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

pub struct Short {
    id: object::Id,
    short: String,
}

impl Short {
    pub fn new(id: object::Id, short: String) -> Box<Self> {
        Box::new(Short { id, short })
    }
}

#[async_trait]
impl Persist for Short {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE objects SET short = ? WHERE id = ?")
            .bind(self.short.as_str())
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}
