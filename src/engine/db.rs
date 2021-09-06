use std::{borrow::Cow, collections::HashMap, str::FromStr};

use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use lazy_static::lazy_static;
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};

use super::world::{Configuration, Room, RoomMetadata};

lazy_static! {
    static ref DB_NOT_FOUND_CODE: &'static str = "14";
}

pub async fn open(db: &str) -> anyhow::Result<SqlitePool> {
    let uri = format!("sqlite://{}", db);
    match SqlitePool::connect(&uri).await {
        Ok(pool) => match sqlx::migrate!("./migrations").run(&pool).await {
            Ok(_) => Ok(pool),
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
                    Ok(pool)
                } else {
                    Err(de.into())
                }
            } else {
                Err(e.into())
            }
        }
    }
}

pub async fn load_world(pool: &SqlitePool) -> anyhow::Result<World> {
    let mut world = World::new();

    load_configuration(pool, &mut world).await?;
    load_rooms(pool, &mut world).await?;

    Ok(world)
}

async fn load_configuration(pool: &SqlitePool, world: &mut World) -> anyhow::Result<()> {
    let config_row = sqlx::query(r#"SELECT value FROM config WHERE key = "spawn_room""#)
        .fetch_one(pool)
        .await?;

    let spawn_room_str: String = config_row.get("value");
    let spawn_room = spawn_room_str.parse::<i64>()?;

    let configuration = Configuration {
        shutdown: false,
        spawn_room,
    };

    world.insert_resource(configuration);

    Ok(())
}

async fn load_rooms(pool: &SqlitePool, world: &mut World) -> anyhow::Result<()> {
    let mut highest_id = 0;
    let mut rooms_by_id = HashMap::new();

    let mut results = sqlx::query_as::<_, RoomRow>("SELECT id, description FROM rooms").fetch(pool);
    while let Some(room) = results.try_next().await? {
        let id = room.id;
        if id > highest_id {
            highest_id = id;
        }
        let entity = world.spawn().insert(Room::from(room)).id();
        rooms_by_id.insert(id, entity);
    }

    let metadata = RoomMetadata {
        rooms_by_id,
        highest_id,
        players_by_room: HashMap::new(),
    };
    world.insert_resource(metadata);

    Ok(())
}

#[derive(sqlx::FromRow)]
struct RoomRow {
    id: i64,
    description: String,
}

impl From<RoomRow> for Room {
    fn from(row: RoomRow) -> Self {
        Room {
            id: row.id,
            description: row.description,
        }
    }
}
