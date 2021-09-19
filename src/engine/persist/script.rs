use async_trait::async_trait;
use sqlx;

use crate::{
    engine::persist::Persist,
    world::{
        scripting::{ScriptName, ScriptTrigger},
        types::Id,
    },
};

pub struct Attach {
    target: Id,
    script: ScriptName,
    trigger: ScriptTrigger,
}

impl Attach {
    pub fn new(target: Id, script: ScriptName, trigger: ScriptTrigger) -> Box<Self> {
        Box::new(Attach {
            target,
            script,
            trigger,
        })
    }
}

#[async_trait]
impl Persist for Attach {
    async fn enact(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        match self.target {
            Id::Player(id) => {
                sqlx::query(
                "INSERT INTO player_scripts (player_id, kind, script, trigger) VALUES (?, ?, ?, ?)",
                )
                .bind(id.to_string())
                .bind(self.trigger.kind().to_string())
                .bind(self.script.to_string())
                .bind(self.trigger.trigger().to_string())
                .execute(pool)
                .await?;
            }
            Id::Object(id) => {
                sqlx::query(
                "INSERT INTO object_scripts (object_id, kind, script, trigger) VALUES (?, ?, ?, ?)",
                )
                .bind(id.to_string())
                .bind(self.trigger.kind().to_string())
                .bind(self.script.to_string())
                .bind(self.trigger.trigger().to_string())
                .execute(pool)
                .await?;
            }
            Id::Room(id) => {
                sqlx::query(
                    "INSERT INTO room_scripts (room_id, kind, script, trigger) VALUES (?, ?, ?, ?)",
                )
                .bind(id.to_string())
                .bind(self.trigger.kind().to_string())
                .bind(self.script.to_string())
                .bind(self.trigger.trigger().to_string())
                .execute(pool)
                .await?;
            }
        };

        Ok(())
    }
}

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
        sqlx::query("INSERT INTO scripts (name, trigger, code) VALUES (?, ?, ?)")
            .bind(self.name.as_str())
            .bind(self.trigger.to_string())
            .bind(self.code.as_str())
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Detach {
    target: Id,
    script: ScriptName,
    trigger: ScriptTrigger,
}

impl Detach {
    pub fn new(target: Id, script: ScriptName, trigger: ScriptTrigger) -> Box<Self> {
        Box::new(Detach {
            target,
            script,
            trigger,
        })
    }
}

#[async_trait]
impl Persist for Detach {
    async fn enact(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        match self.target {
            Id::Player(id) => {
                sqlx::query(
                "DELETE FROM player_scripts WHERE player_id = ? AND kind = ? AND script = ? AND trigger = ?",
                )
                .bind(id.to_string())
                .bind(self.trigger.kind().to_string())
                .bind(self.script.to_string())
                .bind(self.trigger.trigger().to_string())
                .execute(pool)
                .await?;
            }
            Id::Object(id) => {
                sqlx::query(
                "DELETE FROM object_scripts WHERE object_id = ? AND kind = ? AND script = ? AND trigger = ?",
                )
                .bind(id.to_string())
                .bind(self.trigger.kind().to_string())
                .bind(self.script.to_string())
                .bind(self.trigger.trigger().to_string())
                .execute(pool)
                .await?;
            }
            Id::Room(id) => {
                sqlx::query(
                "DELETE FROM room_scripts WHERE room_id = ? AND kind = ? AND script = ? AND trigger = ?",
                )
                .bind(id.to_string())
                .bind(self.trigger.kind().to_string())
                .bind(self.script.to_string())
                .bind(self.trigger.trigger().to_string())
                .execute(pool)
                .await?;
            }
        };

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
        sqlx::query("UPDATE scripts SET trigger = ?, code = ? WHERE name = ?")
            .bind(self.trigger.to_string())
            .bind(self.code.as_str())
            .bind(self.name.as_str())
            .execute(pool)
            .await?;

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
