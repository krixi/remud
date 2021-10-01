use async_trait::async_trait;
use sqlx::SqlitePool;
use tracing::Instrument;

use crate::{
    engine::persist::Persist,
    world::types::object::{self, InheritableFields, ObjectId, PrototypeId},
};

#[derive(Debug)]
pub struct Create {
    id: ObjectId,
    prototype: PrototypeId,
}

impl Create {
    pub fn new(id: ObjectId, prototype: PrototypeId) -> Box<Self> {
        Box::new(Create { id, prototype })
    }
}

#[async_trait]
impl Persist for Create {
    #[tracing::instrument(name = "create object", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO objects (id, prototype_id, inherit_scripts, name) VALUES (?, ?, ?, NULL)",
        )
        .bind(self.id)
        .bind(self.prototype)
        .bind(true)
        .execute(pool)
        .in_current_span()
        .await?;

        Ok(())
    }
}

#[derive(Debug)]
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
    #[tracing::instrument(name = "update object description", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE objects SET description = ? WHERE id = ?")
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
    id: ObjectId,
    flags: object::Flags,
}

impl Flags {
    pub fn new(id: ObjectId, flags: object::Flags) -> Box<Self> {
        Box::new(Flags { id, flags })
    }
}

#[async_trait]
impl Persist for Flags {
    #[tracing::instrument(name = "update object flags", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE objects SET flags = ? WHERE id = ?")
            .bind(self.flags.bits())
            .bind(self.id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Inherit {
    id: ObjectId,
    fields: Vec<InheritableFields>,
}

impl Inherit {
    pub fn new(id: ObjectId, fields: Vec<InheritableFields>) -> Box<Self> {
        Box::new(Inherit { id, fields })
    }
}

#[async_trait]
impl Persist for Inherit {
    #[tracing::instrument(name = "update object inherits", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        for field in self.fields.iter() {
            match field {
                InheritableFields::Flags => {
                    sqlx::query("UPDATE objects SET flags = null WHERE id = ?")
                        .bind(self.id)
                        .execute(pool)
                        .in_current_span()
                        .await?;
                }
                InheritableFields::Name => {
                    sqlx::query("UPDATE objects SET name = null WHERE id = ?")
                        .bind(self.id)
                        .execute(pool)
                        .in_current_span()
                        .await?;
                }
                InheritableFields::Description => {
                    sqlx::query("UPDATE objects SET description = null WHERE id = ?")
                        .bind(self.id)
                        .execute(pool)
                        .in_current_span()
                        .await?;
                }
                InheritableFields::Keywords => {
                    sqlx::query("UPDATE objects SET keywords = null WHERE id = ?")
                        .bind(self.id)
                        .execute(pool)
                        .in_current_span()
                        .await?;
                }
                InheritableFields::Scripts => {
                    sqlx::query("UPDATE objects SET inherit_scripts = true WHERE id = ?")
                        .bind(self.id)
                        .execute(pool)
                        .in_current_span()
                        .await?;

                    sqlx::query("DELETE FROM object_scripts WHERE object_id = ?")
                        .bind(self.id)
                        .execute(pool)
                        .in_current_span()
                        .await?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
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
    #[tracing::instrument(name = "update object keywords", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        let keywords = self.keywords.join(",");
        sqlx::query("UPDATE objects SET keywords = ? WHERE id = ?")
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
    #[tracing::instrument(name = "update object name", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE objects SET name = ? WHERE id = ?")
            .bind(self.name.as_str())
            .bind(self.id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
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
    #[tracing::instrument(name = "remove object", skip(pool))]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM objects WHERE id = ?")
            .bind(self.object_id)
            .execute(pool)
            .in_current_span()
            .await?;

        Ok(())
    }
}
