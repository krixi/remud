use std::{borrow::Cow, collections::HashMap, convert::TryFrom, str::FromStr};

use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use lazy_static::lazy_static;
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};

use crate::world::{
    types::room::{Direction, Room, RoomId, Rooms},
    Configuration,
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

    pub async fn load_world(&self) -> anyhow::Result<World> {
        let mut world = World::new();

        load_configuration(&self.pool, &mut world).await?;
        load_rooms(&self.pool, &mut world).await?;
        load_exits(&self.pool, &mut world).await?;

        Ok(world)
    }

    pub fn get_pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn vacuum(&self) -> anyhow::Result<()> {
        sqlx::query("VACUUM").execute(&self.pool).await?;
        Ok(())
    }
}

async fn load_configuration(pool: &SqlitePool, world: &mut World) -> anyhow::Result<()> {
    let config_row = sqlx::query(r#"SELECT value FROM config WHERE key = "spawn_room""#)
        .fetch_one(pool)
        .await?;

    let spawn_room_str: String = config_row.get("value");
    let spawn_room = RoomId::try_from(spawn_room_str.parse::<i64>()?)?;

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
        let entity = world.spawn().insert(Room::try_from(room)?).id();
        rooms_by_id.insert(RoomId::try_from(id)?, entity);
    }

    let metadata = Rooms::new(rooms_by_id, highest_id);
    world.insert_resource(metadata);

    Ok(())
}

async fn load_exits(pool: &SqlitePool, world: &mut World) -> anyhow::Result<()> {
    let mut results =
        sqlx::query_as::<_, ExitRow>("SELECT room_from, room_to, direction FROM exits").fetch(pool);

    while let Some(exit) = results.try_next().await? {
        let (from, to) = {
            let rooms = &world.get_resource::<Rooms>().unwrap();
            let from = rooms.get_room(RoomId::try_from(exit.room_from)?).unwrap();
            let to = rooms.get_room(RoomId::try_from(exit.room_to)?).unwrap();
            (from, to)
        };

        let direction = Direction::from_str(exit.direction.as_str()).unwrap();

        world
            .entity_mut(from)
            .get_mut::<Room>()
            .unwrap()
            .exits
            .insert(direction, to);
    }

    Ok(())
}

#[derive(sqlx::FromRow)]
struct RoomRow {
    id: i64,
    description: String,
}

impl TryFrom<RoomRow> for Room {
    type Error = anyhow::Error;

    fn try_from(value: RoomRow) -> Result<Self, Self::Error> {
        Ok(Room {
            id: RoomId::try_from(value.id)?,
            description: value.description,
            exits: HashMap::new(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct ExitRow {
    room_from: i64,
    room_to: i64,
    direction: String,
}
