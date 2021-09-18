use async_trait::async_trait;

use crate::engine::persist::Persist;

pub struct Create {
    name: String,
    trigger: String,
    code: String,
}

impl Create {
    pub fn new(name: String, trigger: String, code: String) -> Box<Self> {
        Box::new(Create {
            name,
            trigger,
            code,
        })
    }
}

#[async_trait]
impl Persist for Create {
    async fn enact(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        tracing::info!(
            "Inserting new script: {} -> {}",
            self.name.as_str(),
            self.trigger.to_string()
        );
        sqlx::query("INSERT INTO scripts (name, trigger, code) VALUES (?, ?, ?)")
            .bind(self.name.as_str())
            .bind(self.trigger.to_string())
            .bind(self.code.as_str())
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Update {
    name: String,
    trigger: String,
    code: String,
}

impl Update {
    pub fn new(name: String, trigger: String, code: String) -> Box<Self> {
        Box::new(Update {
            name,
            trigger,
            code,
        })
    }
}

#[async_trait]
impl Persist for Update {
    async fn enact(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        tracing::info!(
            "Updating script: {} -> {}: {}",
            self.name.as_str(),
            self.trigger.to_string(),
            self.code.as_str()
        );
        let results = sqlx::query("UPDATE scripts SET trigger = ?, code = ? WHERE name = ?")
            .bind(self.trigger.to_string())
            .bind(self.code.as_str())
            .bind(self.name.as_str())
            .execute(pool)
            .await?;

        let affected = results.rows_affected();
        tracing::info!("Update affected {} rows.", affected);

        Ok(())
    }
}

pub struct Delete {
    name: String,
}

impl Delete {
    pub fn new(name: String) -> Box<Self> {
        Box::new(Delete { name })
    }
}

#[async_trait]
impl Persist for Delete {
    async fn enact(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM scripts WHERE name = ?")
            .bind(self.name.as_str())
            .execute(pool)
            .await?;

        Ok(())
    }
}
