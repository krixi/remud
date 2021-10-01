use async_trait::async_trait;
use sqlx::SqlitePool;
use tracing::Instrument;

use crate::{
    engine::persist::Persist,
    world::types::object::{self, PrototypeId},
};

#[derive(Debug)]
pub struct Create {
    id: PrototypeId,
    name: String,
    description: String,
    flags: object::Flags,
    keywords: Vec<String>,
}

impl Create {
    pub fn new(
        id: PrototypeId,
        name: String,
        description: String,
        flags: object::Flags,
        keywords: Vec<String>,
    ) -> Box<Self> {
        Box::new(Create {
            id,
            name,
            description,
            flags,
            keywords,
        })
    }
}

#[async_trait]
impl Persist for Create {
    #[tracing::instrument(name = "create prototype", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO prototypes (id, name, description, flags, keywords) VALUES (?, ?, ?, ?, \
             ?)",
        )
        .bind(self.id)
        .bind(self.name.as_str())
        .bind(self.description.as_str())
        .bind(self.flags.bits())
        .bind(self.keywords.join(",").as_str())
        .execute(pool)
        .in_current_span()
        .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Description {
    id: PrototypeId,
    description: String,
}

impl Description {
    pub fn new(id: PrototypeId, description: String) -> Box<Self> {
        Box::new(Description { id, description })
    }
}

#[async_trait]
impl Persist for Description {
    #[tracing::instrument(name = "create prototype", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE prototypes SET description = ? WHERE id = ?")
            .bind(self.description.as_str())
            .bind(self.id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Flags {
    id: PrototypeId,
    flags: object::Flags,
}

impl Flags {
    pub fn new(id: PrototypeId, flags: object::Flags) -> Box<Self> {
        Box::new(Flags { id, flags })
    }
}

#[async_trait]
impl Persist for Flags {
    #[tracing::instrument(name = "update prototype flags", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE prototypes SET flags = ? WHERE id = ?")
            .bind(self.flags.bits())
            .bind(self.id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Keywords {
    id: PrototypeId,
    keywords: Vec<String>,
}

impl Keywords {
    pub fn new(id: PrototypeId, keywords: Vec<String>) -> Box<Self> {
        Box::new(Keywords { id, keywords })
    }
}

#[async_trait]
impl Persist for Keywords {
    #[tracing::instrument(name = "update prototype keywords", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        let keywords = self.keywords.join(",");
        sqlx::query("UPDATE prototypes SET keywords = ? WHERE id = ?")
            .bind(keywords)
            .bind(self.id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Name {
    id: PrototypeId,
    name: String,
}

impl Name {
    pub fn new(id: PrototypeId, name: String) -> Box<Self> {
        Box::new(Name { id, name })
    }
}

#[async_trait]
impl Persist for Name {
    #[tracing::instrument(name = "update prototype name", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE prototypes SET name = ? WHERE id = ?")
            .bind(self.name.as_str())
            .bind(self.id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}
