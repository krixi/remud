use std::{borrow::Cow, collections::HashMap, convert::TryFrom, str::FromStr};

use anyhow::bail;
use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use itertools::Itertools;
use lazy_static::lazy_static;
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};

use crate::world::types::{
    object::{Location, Object, ObjectId, Objects},
    room::{Direction, Room, RoomId, Rooms},
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
        let room_objects = load_room_objects(&self.pool).await?;
        load_objects(&self.pool, &mut world, room_objects).await?;

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

    let rooms = Rooms::new(rooms_by_id, highest_id);
    world.insert_resource(rooms);

    Ok(())
}

async fn load_exits(pool: &SqlitePool, world: &mut World) -> anyhow::Result<()> {
    let mut results =
        sqlx::query_as::<_, ExitRow>("SELECT room_from, room_to, direction FROM exits").fetch(pool);

    while let Some(exit) = results.try_next().await? {
        let (from, to) = {
            let rooms = &world.get_resource::<Rooms>().unwrap();
            let from = rooms.by_id(RoomId::try_from(exit.room_from)?).unwrap();
            let to = rooms.by_id(RoomId::try_from(exit.room_to)?).unwrap();
            (from, to)
        };

        let direction = Direction::from_str(exit.direction.as_str()).unwrap();

        world
            .get_mut::<Room>(from)
            .unwrap()
            .exits
            .insert(direction, to);
    }

    Ok(())
}

async fn load_objects(
    pool: &SqlitePool,
    world: &mut World,
    room_objects: HashMap<i64, i64>,
) -> anyhow::Result<()> {
    let mut results =
        sqlx::query_as::<_, ObjectRow>("SELECT id, keywords, short, long FROM objects").fetch(pool);

    let mut highest_id = 0;
    let mut by_id = HashMap::new();

    while let Some(object) = results.try_next().await? {
        if object.id > highest_id {
            highest_id = object.id;
        }

        let room_id = match room_objects.get(&object.id) {
            Some(room) => *room,
            None => bail!("Failed to find room for object {}", object.id),
        };

        let room_id = match RoomId::try_from(room_id) {
            Ok(id) => id,
            Err(_) => bail!("Failed to deserialize room ID: {}", room_id),
        };

        let room_entity = match world.get_resource::<Rooms>().unwrap().by_id(room_id) {
            Some(room) => room,
            None => bail!("Failed to retrieve Room for room {}", room_id),
        };

        let id = match ObjectId::try_from(object.id) {
            Ok(id) => id,
            Err(_) => bail!("Failed to deserialize object ID: {}", object.id),
        };

        let object = Object::new(
            id,
            Location::Room(room_entity),
            object.keywords(),
            object.short,
            object.long,
        );

        let object_entity = world.spawn().insert(object).id();
        match world.get_mut::<Room>(room_entity) {
            Some(mut room) => room.objects.push(object_entity),
            None => bail!("Failed to retrieve Room for room {:?}", room_entity),
        }

        by_id.insert(id, object_entity);
    }

    world.insert_resource(Objects::new(highest_id, by_id));

    Ok(())
}

async fn load_room_objects(pool: &SqlitePool) -> anyhow::Result<HashMap<i64, i64>> {
    let mut results =
        sqlx::query_as::<_, RoomObjectRow>("SELECT room_id, object_id FROM room_objects")
            .fetch(pool);

    let mut map = HashMap::new();

    while let Some(room_object) = results.try_next().await? {
        map.insert(room_object.object_id, room_object.room_id);
    }

    Ok(map)
}

#[derive(sqlx::FromRow)]
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

#[derive(sqlx::FromRow)]
struct ObjectRow {
    id: i64,
    keywords: String,
    long: String,
    short: String,
}

impl ObjectRow {
    fn keywords(&self) -> Vec<String> {
        self.keywords
            .split(',')
            .map(|keyword| keyword.to_string())
            .collect_vec()
    }
}

#[derive(sqlx::FromRow)]
struct RoomRow {
    id: i64,
    description: String,
}

impl TryFrom<RoomRow> for Room {
    type Error = anyhow::Error;

    fn try_from(value: RoomRow) -> Result<Self, Self::Error> {
        Ok(Room::new(RoomId::try_from(value.id)?, value.description))
    }
}

#[derive(sqlx::FromRow)]
struct ExitRow {
    room_from: i64,
    room_to: i64,
    direction: String,
}
