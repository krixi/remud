mod player;
mod world;

use std::{borrow::Cow, convert::TryFrom, str::FromStr};

use argon2::{password_hash, Argon2, PasswordHash, PasswordVerifier};
use async_trait::async_trait;
use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use itertools::Itertools;
use sqlx::{migrate::MigrateError, sqlite::SqliteConnectOptions, Row, SqlitePool};
use thiserror::Error;

use crate::world::{
    scripting::{ScriptHook, ScriptHooks, ScriptName, ScriptTrigger, TriggerEvent, TriggerKind},
    types::{
        self,
        object::{
            Keywords, Object, ObjectBundle, ObjectFlags, ObjectId, Objects, PrototypeId, Prototypes,
        },
        player::{Player, PlayerFlags},
        room::RoomId,
        Contents, Description, Id, Location, Named,
    },
};

const DEFAULT_PLAYER_DESCRIPTION: &str = "A being exists here.";
const DB_NOT_FOUND_CODE: &str = "14";

type DbResult<T> = Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to execute migrations: {0}")]
    Migration(#[from] MigrateError),
    #[error("SQL error: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("failed to deserialize value: {0}")]
    Deserialize(&'static str),
    #[error("missing data: {0}")]
    MissingData(&'static str),
    #[error("password verification error: {0}")]
    PasswordVerification(String),
}

#[async_trait]
pub trait AuthDb {
    async fn verify_player(&self, player: &str, password: &str) -> Result<bool, Error>;
    async fn is_immortal(&self, player: &str) -> Result<bool, Error>;
    async fn register_tokens(
        &self,
        player: &str,
        access_issued_secs: i64,
        refresh_issued_secs: i64,
    ) -> Result<(), Error>;
    async fn logout(&self, player: &str) -> Result<(), Error>;
    async fn access_issued_secs(&self, player: &str) -> Result<Option<i64>, Error>;
    async fn refresh_issued_secs(&self, player: &str) -> Result<Option<i64>, Error>;
}

#[async_trait]
pub trait GameDb {
    async fn load_world(&self, world: &mut World) -> DbResult<()>;
    async fn has_player(&self, user: &str) -> anyhow::Result<bool>;
    async fn create_player(&self, user: &str, hash: &str, room: RoomId) -> anyhow::Result<i64>;
    async fn load_player(&self, world: &mut World, name: &str) -> anyhow::Result<Entity>;
    async fn reload_prototype(
        &self,
        world: &mut World,
        prototype_id: PrototypeId,
    ) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    #[tracing::instrument(name = "initializing database")]
    pub async fn new(path: Option<&str>) -> DbResult<Self> {
        let uri = path
            .map(|path| format!("sqlite://{}", path))
            .unwrap_or_else(|| "sqlite::memory:".to_string());

        let db = match SqlitePool::connect(uri.as_str()).await {
            Ok(pool) => match sqlx::migrate!("../migrations").run(&pool).await {
                Ok(_) => Ok(Db { pool }),
                Err(e) => Err(e.into()),
            },
            Err(e) => {
                if let sqlx::Error::Database(ref de) = e {
                    if de.code() == Some(Cow::Borrowed(DB_NOT_FOUND_CODE)) {
                        tracing::warn!("world database {} not found, creating new instance.", uri);
                        let options = SqliteConnectOptions::from_str(&uri)
                            .unwrap()
                            .create_if_missing(true);
                        let pool = SqlitePool::connect_with(options).await?;
                        sqlx::migrate!("../migrations").run(&pool).await?;
                        Ok(Db { pool })
                    } else {
                        return Err(Error::Sql(e));
                    }
                } else {
                    Err(Error::Sql(e))
                }
            }
        };

        if let Ok(db) = &db {
            db.vacuum().await?;
        }

        db
    }

    pub fn get_pool(&self) -> SqlitePool {
        self.pool.clone()
    }

    pub async fn vacuum(&self) -> Result<(), sqlx::Error> {
        sqlx::query("VACUUM").execute(&self.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl AuthDb for Db {
    async fn verify_player(&self, player: &str, password: &str) -> Result<bool, Error> {
        let results = sqlx::query("SELECT password FROM players WHERE username = ?")
            .bind(player)
            .fetch_one(&self.pool)
            .await?;

        let hash = results.get("password");

        match verify_password(hash, password) {
            Ok(_) => Ok(true),
            Err(e) => match e {
                VerifyError::BadPassword => Ok(false),
                VerifyError::Unknown(s) => Err(Error::PasswordVerification(s)),
            },
        }
    }

    async fn is_immortal(&self, player: &str) -> Result<bool, Error> {
        let results = sqlx::query("SELECT flags FROM players WHERE username = ?")
            .bind(player)
            .fetch_one(&self.pool)
            .await?;

        let flags: i64 = results.get("flags");
        let flags = PlayerFlags::from(flags);

        Ok(flags.contains(types::player::Flags::IMMORTAL))
    }

    async fn register_tokens(
        &self,
        player: &str,
        access_issued_secs: i64,
        refresh_issued_secs: i64,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"INSERT INTO tokens
        SELECT id AS player_id, ? AS access, ? AS refresh
        FROM players WHERE username = ?
        ON CONFLICT(player_id)
        DO UPDATE SET access = excluded.access, refresh = excluded.refresh"#,
        )
        .bind(access_issued_secs)
        .bind(refresh_issued_secs)
        .bind(player)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn logout(&self, player: &str) -> Result<(), Error> {
        sqlx::query(
            r#"DELETE FROM tokens
            WHERE player_id IN (
                SELECT id FROM players WHERE username = ?
            )"#,
        )
        .bind(player)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn access_issued_secs(&self, player: &str) -> Result<Option<i64>, Error> {
        let results = sqlx::query(
            r#"SELECT access FROM tokens
        INNER JOIN players ON players.id = tokens.player_id
        WHERE players.username = ?"#,
        )
        .bind(player)
        .fetch_optional(&self.pool)
        .await?;

        Ok(results.map(|r| r.get("access")))
    }

    async fn refresh_issued_secs(&self, player: &str) -> Result<Option<i64>, Error> {
        let results = sqlx::query(
            r#"SELECT refresh FROM tokens
        INNER JOIN players ON players.id = tokens.player_id
        WHERE players.username = ?"#,
        )
        .bind(player)
        .fetch_optional(&self.pool)
        .await?;

        Ok(results.map(|r| r.get("refresh")))
    }
}

#[async_trait]
impl GameDb for Db {
    async fn load_world(&self, world: &mut World) -> DbResult<()> {
        world::load_world(&self.pool, world).await
    }

    async fn has_player(&self, user: &str) -> anyhow::Result<bool> {
        let result = sqlx::query("SELECT * FROM players WHERE username = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;

        Ok(result.is_some())
    }

    async fn create_player(&self, user: &str, hash: &str, room: RoomId) -> anyhow::Result<i64> {
        let results = sqlx::query(
            "INSERT INTO players (username, password, room, description, flags) VALUES (?, ?, ?, \
             ?, ?) RETURNING id",
        )
        .bind(user)
        .bind(hash)
        .bind(room)
        .bind(DEFAULT_PLAYER_DESCRIPTION)
        .bind(0)
        .fetch_one(&self.pool)
        .await?;

        let id = results.get("id");

        // Player 1 is always an immortal by default.
        if id == 1 {
            sqlx::query("UPDATE players SET flags = ? WHERE username = ?")
                .bind(types::player::Flags::IMMORTAL.bits())
                .bind(user)
                .execute(&self.pool)
                .await?;
        }

        Ok(id)
    }

    async fn load_player(&self, world: &mut World, name: &str) -> anyhow::Result<Entity> {
        player::load_player(&self.pool, world, name).await
    }

    #[tracing::instrument(name = "reload prototype", skip(self, world))]
    async fn reload_prototype(
        &self,
        world: &mut World,
        prototype_id: PrototypeId,
    ) -> anyhow::Result<()> {
        let mut results = sqlx::query_as::<_, ObjectRow>(
            r#"SELECT objects.id, objects.prototype_id, objects.inherit_scripts, NULL AS location,
                        COALESCE(objects.name, prototypes.name) AS name, COALESCE(objects.description, prototypes.description) AS description,
                        COALESCE(objects.flags, prototypes.flags) AS flags, COALESCE(objects.keywords, prototypes.keywords) AS keywords
                    FROM objects
                    INNER JOIN room_objects ON room_objects.object_id = objects.id
                    INNER JOIN prototypes ON objects.prototype_id = prototypes.id
                    WHERE prototypes.id = ?"#,
        )
        .bind(prototype_id)
        .fetch(&self.pool);

        let prototype = world
            .get_resource::<Prototypes>()
            .unwrap()
            .by_id(prototype_id)
            .unwrap();

        while let Some(object_row) = results.try_next().await? {
            let inherit_scripts = object_row.inherit_scripts;

            let object_id = ObjectId::try_from(object_row.id)?;
            let object_entity = world
                .get_resource::<Objects>()
                .unwrap()
                .by_id(object_id)
                .unwrap();

            let (id, object, named, description, flags, keywords) =
                object_row.into_components(prototype)?;

            world
                .entity_mut(object_entity)
                .insert(id)
                .insert(object)
                .insert(named)
                .insert(description)
                .insert(flags)
                .insert(keywords);

            if inherit_scripts {
                let mut results = sqlx::query_as::<_, HookRow>(
                    r#"SELECT kind, script, trigger FROM prototype_scripts WHERE prototype_id = ?"#,
                )
                .bind(prototype_id)
                .fetch(&self.pool);

                world.entity_mut(object_entity).remove::<ScriptHooks>();

                while let Some(hook_row) = results.try_next().await? {
                    let hook = ScriptHook::try_from(hook_row)?;

                    if let Some(mut hooks) = world.get_mut::<ScriptHooks>(object_entity) {
                        hooks.insert(hook);
                    } else {
                        world
                            .entity_mut(object_entity)
                            .insert(ScriptHooks::new(hook));
                    }
                }
            }
        }

        let player_objects = {
            world
                .query_filtered::<&Contents, With<Player>>()
                .iter(&*world)
                .flat_map(|contents| contents.get_objects())
                .dedup()
                .collect_vec()
        };

        for object_entity in player_objects {
            let id = world.get::<Object>(object_entity).unwrap().id();
            let object_row = sqlx::query_as::<_, ObjectRow>(
            r#"SELECT objects.id, objects.prototype_id, objects.inherit_scripts, NULL AS location,
                        COALESCE(objects.name, prototypes.name) AS name, COALESCE(objects.description, prototypes.description) AS description,
                        COALESCE(objects.flags, prototypes.flags) AS flags, COALESCE(objects.keywords, prototypes.keywords) AS keywords
                    FROM objects
                    INNER JOIN prototypes ON objects.prototype_id = prototypes.id
                    WHERE objects.id = ?"#,
            )
            .bind(id)
            .fetch_one(&self.pool).await?;

            let inherit_scripts = object_row.inherit_scripts;

            let prototype_id = object_row.prototype_id;
            let prototype = world
                .get_resource::<Prototypes>()
                .unwrap()
                .by_id(PrototypeId::try_from(object_row.prototype_id)?)
                .unwrap();

            let (id, object, named, description, flags, keywords) =
                object_row.into_components(prototype)?;

            world
                .entity_mut(object_entity)
                .insert(id)
                .insert(object)
                .insert(named)
                .insert(description)
                .insert(flags)
                .insert(keywords);

            if inherit_scripts {
                let mut results = sqlx::query_as::<_, HookRow>(
                    r#"SELECT kind, script, trigger FROM prototype_scripts WHERE prototype_id = ?"#,
                )
                .bind(prototype_id)
                .fetch(&self.pool);

                while let Some(hook_row) = results.try_next().await? {
                    let hook = ScriptHook::try_from(hook_row)?;

                    if let Some(mut hooks) = world.get_mut::<ScriptHooks>(object_entity) {
                        hooks.insert(hook)
                    } else {
                        world
                            .entity_mut(object_entity)
                            .insert(ScriptHooks::new(hook));
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct HookRow {
    kind: String,
    script: String,
    trigger: String,
}

impl TryFrom<HookRow> for ScriptHook {
    type Error = Error;

    fn try_from(value: HookRow) -> Result<Self, Self::Error> {
        let script =
            ScriptName::try_from(value.script).map_err(|_| Error::Deserialize("script name"))?;
        let kind = TriggerKind::from_str(value.kind.as_str())
            .map_err(|_| Error::Deserialize("script trigger kind"))?;
        let trigger = match kind {
            TriggerKind::Init => ScriptTrigger::Init,
            TriggerKind::PreEvent => {
                let trigger = TriggerEvent::from_str(value.trigger.as_str())
                    .map_err(|_| Error::Deserialize("script trigger event"))?;
                ScriptTrigger::PostEvent(trigger)
            }
            TriggerKind::PostEvent => {
                let trigger = TriggerEvent::from_str(value.trigger.as_str())
                    .map_err(|_| Error::Deserialize("script trigger event"))?;
                ScriptTrigger::PostEvent(trigger)
            }
            TriggerKind::Timer => ScriptTrigger::Timer(value.trigger),
        };

        Ok(ScriptHook { script, trigger })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ObjectRow {
    id: i64,
    prototype_id: i64,
    inherit_scripts: bool,
    location: Option<i64>,
    flags: i64,
    name: String,
    keywords: String,
    description: String,
}

impl ObjectRow {
    fn into_object_bundle(self, prototype: Entity, location: Location) -> DbResult<ObjectBundle> {
        let id = ObjectId::try_from(self.id).map_err(|_| Error::Deserialize("object ID"))?;

        Ok(ObjectBundle {
            id: Id::Object(id),
            object: Object::new(id, prototype, self.inherit_scripts),
            name: Named::from(self.name.clone()),
            description: Description::from(self.description.clone()),
            flags: ObjectFlags::from(self.flags),
            keywords: Keywords::from(self.keywords()),
            location,
        })
    }

    fn into_components(
        self,
        prototype: Entity,
    ) -> DbResult<(Id, Object, Named, Description, ObjectFlags, Keywords)> {
        let id = ObjectId::try_from(self.id).map_err(|_| Error::Deserialize("object ID"))?;

        Ok((
            Id::Object(id),
            Object::new(id, prototype, self.inherit_scripts),
            Named::from(self.name.clone()),
            Description::from(self.description.clone()),
            ObjectFlags::from(self.flags),
            Keywords::from(self.keywords()),
        ))
    }

    fn keywords(&self) -> Vec<String> {
        self.keywords
            .split(',')
            .map(ToString::to_string)
            .collect_vec()
    }
}

pub enum VerifyError {
    BadPassword,
    Unknown(String),
}

pub fn verify_password(hash: &str, password: &str) -> Result<(), VerifyError> {
    let password_hash = PasswordHash::new(hash)
        .map_err(|e| VerifyError::Unknown(format!("Hash parsing error: {}", e)))?;
    let hasher = Argon2::default();
    hasher
        .verify_password(password.as_bytes(), &password_hash)
        .map_err(|e| match e {
            password_hash::Error::Password => VerifyError::BadPassword,
            e => VerifyError::Unknown(format!("Verify password error: {}", e)),
        })?;
    Ok(())
}
