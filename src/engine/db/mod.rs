mod player;
mod world;

use std::{
    borrow::Cow,
    convert::TryFrom,
    str::FromStr,
    sync::{Arc, RwLock},
};

use bevy_ecs::prelude::*;
use itertools::Itertools;
use lazy_static::lazy_static;
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};

use crate::world::{
    scripting::{Script, ScriptHook, ScriptName, TriggerEvent, TriggerKind},
    types::{
        object::ObjectId,
        room::{Room, RoomId},
    },
};

lazy_static! {
    static ref DB_NOT_FOUND_CODE: &'static str = "14";
}

pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn new(db: &str) -> anyhow::Result<Self> {
        let uri = format!("sqlite://{}", db);
        let db = match SqlitePool::connect(&uri).await {
            Ok(pool) => match sqlx::migrate!("./migrations").run(&pool).await {
                Ok(_) => Ok(Db { pool }),
                Err(e) => Err(e.into()),
            },
            Err(e) => {
                if let sqlx::Error::Database(de) = e {
                    if de.code() == Some(Cow::Borrowed(&DB_NOT_FOUND_CODE)) {
                        tracing::warn!("World database {} not found, creating new instance.", uri);
                        let options = SqliteConnectOptions::from_str(&uri)
                            .unwrap()
                            .create_if_missing(true);
                        let pool = SqlitePool::connect_with(options).await?;
                        sqlx::migrate!("./migrations").run(&pool).await?;
                        Ok(Db { pool })
                    } else {
                        Err(de.into())
                    }
                } else {
                    Err(e.into())
                }
            }
        };

        if let Ok(db) = &db {
            db.vacuum().await?;
        }

        db
    }

    pub fn get_pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn vacuum(&self) -> anyhow::Result<()> {
        sqlx::query("VACUUM").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn has_player(&self, user: &str) -> anyhow::Result<bool> {
        let result = sqlx::query("SELECT * FROM players WHERE username = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;

        Ok(result.is_some())
    }

    pub async fn create_player(&self, user: &str, hash: &str, room: RoomId) -> anyhow::Result<i64> {
        let results = sqlx::query(
            "INSERT INTO players (username, password, room) VALUES (?, ?, ?) RETURNING id",
        )
        .bind(user)
        .bind(hash)
        .bind(room)
        .fetch_one(&self.pool)
        .await?;

        Ok(results.get("id"))
    }

    pub async fn get_user_hash(&self, user: &str) -> anyhow::Result<String> {
        let results = sqlx::query("SELECT password FROM players WHERE username = ?")
            .bind(user)
            .fetch_one(&self.pool)
            .await?;

        Ok(results.get("password"))
    }

    pub async fn load_world(&self) -> anyhow::Result<World> {
        world::load_world(&self.pool).await
    }

    pub async fn load_player(
        &self,
        world: Arc<RwLock<World>>,
        name: &str,
    ) -> anyhow::Result<Entity> {
        player::load_player(&self.pool, world, name).await
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RoomObjectRow {
    room_id: i64,
    object_id: i64,
}

impl TryFrom<RoomObjectRow> for (RoomId, ObjectId) {
    type Error = anyhow::Error;

    fn try_from(value: RoomObjectRow) -> Result<Self, Self::Error> {
        let room_id = RoomId::try_from(value.room_id)?;
        let object_id = ObjectId::try_from(value.object_id)?;
        Ok((room_id, object_id))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ObjectRow {
    id: i64,
    flags: i64,
    container: i64,
    keywords: String,
    description: String,
    name: String,
}

impl ObjectRow {
    fn keywords(&self) -> Vec<String> {
        self.keywords
            .split(',')
            .map(ToString::to_string)
            .collect_vec()
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RoomRow {
    id: i64,
    description: String,
}

impl TryFrom<RoomRow> for Room {
    type Error = anyhow::Error;

    fn try_from(value: RoomRow) -> Result<Self, Self::Error> {
        Ok(Room::new(RoomId::try_from(value.id)?))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ExitRow {
    room_from: i64,
    room_to: i64,
    direction: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ScriptRow {
    name: String,
    trigger: String,
    code: String,
}

impl TryFrom<ScriptRow> for Script {
    type Error = anyhow::Error;

    fn try_from(value: ScriptRow) -> Result<Self, Self::Error> {
        let name = ScriptName::try_from(value.name)?;
        let trigger = TriggerEvent::from_str(value.trigger.as_str())?;

        Ok(Script {
            name,
            trigger,
            code: value.code,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct HookRow {
    kind: String,
    script: String,
    trigger: String,
}

impl TryFrom<HookRow> for ScriptHook {
    type Error = anyhow::Error;

    fn try_from(value: HookRow) -> Result<Self, Self::Error> {
        let script = ScriptName::try_from(value.script)?;
        let trigger = TriggerEvent::from_str(value.trigger.as_str())?;
        let kind = TriggerKind::from_str(value.kind.as_str())?;

        let trigger = kind.with_trigger(trigger);

        Ok(ScriptHook { script, trigger })
    }
}
