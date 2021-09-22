use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    engine::persist::Persist,
    world::{
        action::immortal::object::{
            DEFAULT_OBJECT_DESCRIPTION, DEFAULT_OBJECT_KEYWORD, DEFAULT_OBJECT_NAME,
        },
        types::object::{ObjectFlags, ObjectId},
    },
};

pub struct Create {
    id: ObjectId,
}

impl Create {
    pub fn new(id: ObjectId) -> Box<Self> {
        Box::new(Create { id })
    }
}

#[async_trait]
impl Persist for Create {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO objects (id, flags, keywords, name, description) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(self.id)
        .bind(0)
        .bind(DEFAULT_OBJECT_KEYWORD)
        .bind(DEFAULT_OBJECT_NAME)
        .bind(DEFAULT_OBJECT_DESCRIPTION)
        .execute(pool)
        .await?;

        Ok(())
    }
}

pub struct Description {
    id: ObjectId,
    description: String,
}

impl Description {
    pub fn new(id: ObjectId, description: String) -> Box<Self> {
        Box::new(Description { id, description })
    }
}

#[async_trait]
impl Persist for Description {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE objects SET description = ? WHERE id = ?")
            .bind(self.description.as_str())
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Flags {
    id: ObjectId,
    flags: ObjectFlags,
}

impl Flags {
    pub fn new(id: ObjectId, flags: ObjectFlags) -> Box<Self> {
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
    id: ObjectId,
    keywords: Vec<String>,
}

impl Keywords {
    pub fn new(id: ObjectId, keywords: Vec<String>) -> Box<Self> {
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

pub struct Name {
    id: ObjectId,
    name: String,
}

impl Name {
    pub fn new(id: ObjectId, name: String) -> Box<Self> {
        Box::new(Name { id, name })
    }
}

#[async_trait]
impl Persist for Name {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE objects SET name = ? WHERE id = ?")
            .bind(self.name.as_str())
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Remove {
    object_id: ObjectId,
}

impl Remove {
    pub fn new(object_id: ObjectId) -> Box<Self> {
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
