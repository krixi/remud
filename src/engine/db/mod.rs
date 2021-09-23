mod player;
mod world;

use std::{
    borrow::Cow,
    convert::TryFrom,
    str::FromStr,
    sync::{Arc, RwLock},
};

use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use itertools::Itertools;
use lazy_static::lazy_static;
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};

use crate::world::{
    scripting::{ScriptHook, ScriptHooks, ScriptName, ScriptTrigger, TriggerEvent, TriggerKind},
    types::{
        self,
        object::{
            Keywords, Object, ObjectBundle, ObjectFlags, ObjectId, Objects, PrototypeId, Prototypes,
        },
        player::Player,
        room::RoomId,
        Contents, Description, Id, Named,
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

        let id = results.get("id");

        // Player 1 is always an immortal by default.
        if id == 1 {
            sqlx::query("UPDATE players SET flags = ? WHERE name = ?")
                .bind(types::player::Flags::IMMORTAL.bits())
                .bind(user)
                .execute(&self.pool)
                .await?;
        }

        Ok(id)
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

    pub async fn reload_prototype(
        &self,
        world: Arc<RwLock<World>>,
        prototype_id: PrototypeId,
    ) -> anyhow::Result<()> {
        let mut results = sqlx::query_as::<_, ObjectRow>(
            r#"SELECT objects.id, objects.prototype_id, objects.inherit_scripts, NULL AS container,
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
            .read()
            .unwrap()
            .get_resource::<Prototypes>()
            .unwrap()
            .by_id(PrototypeId::try_from(prototype_id)?)
            .unwrap();

        while let Some(object_row) = results.try_next().await? {
            let inherit_scripts = object_row.inherit_scripts;

            let object_id = ObjectId::try_from(object_row.id)?;
            let object = world
                .read()
                .unwrap()
                .get_resource::<Objects>()
                .unwrap()
                .by_id(object_id)
                .unwrap();

            let bundle = object_row.into_object_bundle(prototype)?;

            world
                .write()
                .unwrap()
                .entity_mut(object)
                .insert_bundle(bundle);

            if inherit_scripts {
                let mut results = sqlx::query_as::<_, HookRow>(
                    r#"SELECT kind, script, trigger FROM prototype_scripts WHERE prototype_id = ?"#,
                )
                .bind(prototype_id)
                .fetch(&self.pool);

                while let Some(hook_row) = results.try_next().await? {
                    let hook = ScriptHook::try_from(hook_row)?;

                    world
                        .write()
                        .unwrap()
                        .get_mut::<ScriptHooks>(object)
                        .unwrap()
                        .list
                        .push(hook);
                }
            }
        }

        let player_objects = {
            let mut world = world.write().unwrap();
            world
                .query_filtered::<&Contents, With<Player>>()
                .iter(&*world)
                .flat_map(|contents| contents.get_objects())
                .dedup()
                .collect_vec()
        };

        for object in player_objects {
            let id = world.read().unwrap().get::<Object>(object).unwrap().id();
            let object_row = sqlx::query_as::<_, ObjectRow>(
            r#"SELECT objects.id, objects.prototype_id, objects.inherit_scripts, NULL AS container,
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
                .read()
                .unwrap()
                .get_resource::<Prototypes>()
                .unwrap()
                .by_id(PrototypeId::try_from(object_row.prototype_id)?)
                .unwrap();

            let bundle = object_row.into_object_bundle(prototype)?;

            world
                .write()
                .unwrap()
                .entity_mut(object)
                .insert_bundle(bundle);

            if inherit_scripts {
                let mut results = sqlx::query_as::<_, HookRow>(
                    r#"SELECT kind, script, trigger FROM prototype_scripts WHERE prototype_id = ?"#,
                )
                .bind(prototype_id)
                .fetch(&self.pool);

                while let Some(hook_row) = results.try_next().await? {
                    let hook = ScriptHook::try_from(hook_row)?;

                    world
                        .write()
                        .unwrap()
                        .get_mut::<ScriptHooks>(object)
                        .unwrap()
                        .list
                        .push(hook);
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
    type Error = anyhow::Error;

    fn try_from(value: HookRow) -> Result<Self, Self::Error> {
        let script = ScriptName::try_from(value.script)?;
        let kind = TriggerKind::from_str(value.kind.as_str())?;
        let trigger = match kind {
            TriggerKind::PreEvent | TriggerKind::PostEvent => {
                let trigger = TriggerEvent::from_str(value.trigger.as_str())?;
                kind.with_trigger(trigger)
            }
            TriggerKind::Init => ScriptTrigger::Init,
        };

        Ok(ScriptHook { script, trigger })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ObjectRow {
    id: i64,
    prototype_id: i64,
    inherit_scripts: bool,
    container: Option<i64>,
    flags: i64,
    name: String,
    keywords: String,
    description: String,
}

impl ObjectRow {
    fn into_object_bundle(self, prototype: Entity) -> anyhow::Result<ObjectBundle> {
        let id = ObjectId::try_from(self.id)?;

        Ok(ObjectBundle {
            id: Id::Object(id),
            object: Object::new(id, prototype, self.inherit_scripts),
            name: Named::from(self.name.clone()),
            description: Description::from(self.description.clone()),
            flags: ObjectFlags::from(self.flags),
            keywords: Keywords::from(self.keywords()),
            hooks: ScriptHooks::default(),
        })
    }

    fn keywords(&self) -> Vec<String> {
        self.keywords
            .split(',')
            .map(ToString::to_string)
            .collect_vec()
    }
}
