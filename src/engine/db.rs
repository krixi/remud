use std::{
    borrow::Cow, collections::HashMap, convert::TryFrom, ops::DerefMut, str::FromStr, sync::Arc,
};

use anyhow::bail;
use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use itertools::Itertools;
use lazy_static::lazy_static;
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use tokio::sync::RwLock;

use crate::world::{
    action::MissingComponent,
    types::{
        object::{self, Object, Objects},
        player::{self, Player, PlayerBundle, Players},
        room::{self, Direction, Room, RoomBundle, Rooms},
        Configuration, Contents,
    },
    VOID_ROOM_ID,
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
            .fetch_one(&self.pool)
            .await?;

        Ok(!result.is_empty())
    }

    pub async fn create_player(
        &self,
        user: &str,
        hash: &str,
        room: room::Id,
    ) -> anyhow::Result<i64> {
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
        let mut world = World::new();

        self.load_configuration(&mut world).await?;
        self.load_rooms(&mut world).await?;
        self.load_exits(&mut world).await?;
        self.load_room_objects(&mut world).await?;

        Ok(world)
    }

    async fn load_configuration(&self, world: &mut World) -> anyhow::Result<()> {
        let config_row = sqlx::query(r#"SELECT value FROM config WHERE key = "spawn_room""#)
            .fetch_one(&self.pool)
            .await?;

        let spawn_room_str: String = config_row.get("value");
        let spawn_room = room::Id::try_from(spawn_room_str.parse::<i64>()?)?;

        let configuration = Configuration {
            shutdown: false,
            spawn_room,
        };

        world.insert_resource(configuration);

        Ok(())
    }

    async fn load_rooms(&self, world: &mut World) -> anyhow::Result<()> {
        let mut rooms_by_id = HashMap::new();

        let mut results =
            sqlx::query_as::<_, RoomRow>("SELECT id, description FROM rooms").fetch(&self.pool);

        while let Some(room) = results.try_next().await? {
            let id = room.id;
            let entity = world
                .spawn()
                .insert_bundle(RoomBundle {
                    room: Room::try_from(room)?,
                    contents: Contents::default(),
                })
                .id();
            rooms_by_id.insert(room::Id::try_from(id)?, entity);
        }

        let highest_id = sqlx::query("SELECT MAX(id) AS max_id FROM rooms")
            .fetch_one(&self.pool)
            .await?
            .get("max_id");

        let rooms = Rooms::new(rooms_by_id, highest_id);
        world.insert_resource(rooms);

        Ok(())
    }

    async fn load_exits(&self, world: &mut World) -> anyhow::Result<()> {
        let mut results =
            sqlx::query_as::<_, ExitRow>("SELECT room_from, room_to, direction FROM exits")
                .fetch(&self.pool);

        while let Some(exit) = results.try_next().await? {
            let (from, to) = {
                let rooms = &world.get_resource::<Rooms>().unwrap();
                let from = rooms.by_id(room::Id::try_from(exit.room_from)?).unwrap();
                let to = rooms.by_id(room::Id::try_from(exit.room_to)?).unwrap();
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

    async fn load_room_objects(&self, world: &mut World) -> anyhow::Result<()> {
        let mut results = sqlx::query_as::<_, ObjectRow>(
            r#"SELECT id, room_id AS container, keywords, short, long
                FROM objects
                INNER JOIN room_objects ON room_objects.object_id = objects.id"#,
        )
        .fetch(&self.pool);

        let mut by_id = HashMap::new();

        while let Some(object) = results.try_next().await? {
            let room_id = match room::Id::try_from(object.container) {
                Ok(id) => id,
                Err(_) => bail!("Failed to deserialize room ID: {}", object.container),
            };

            let room_entity = match world.get_resource::<Rooms>().unwrap().by_id(room_id) {
                Some(room) => room,
                None => bail!("Failed to retrieve Room for room {}", room_id),
            };

            let id = match object::Id::try_from(object.id) {
                Ok(id) => id,
                Err(_) => bail!("Failed to deserialize object ID: {}", object.id),
            };

            let object = Object::new(
                id,
                room_entity,
                object.keywords(),
                object.short,
                object.long,
            );

            let object_entity = world.spawn().insert(object).id();
            match world.get_mut::<Contents>(room_entity) {
                Some(mut contents) => contents.objects.push(object_entity),
                None => bail!("Failed to retrieve Room for room {:?}", room_entity),
            }

            by_id.insert(id, object_entity);
        }

        let results = sqlx::query("SELECT MAX(id) AS max_id FROM objects")
            .fetch_one(&self.pool)
            .await?;
        let highest_id = results.get("max_id");

        world.insert_resource(Objects::new(highest_id, by_id));

        Ok(())
    }

    pub async fn load_player(
        &self,
        world: Arc<RwLock<World>>,
        name: &str,
    ) -> anyhow::Result<Entity> {
        let mut world = world.write().await;

        let player_row =
            sqlx::query_as::<_, PlayerRow>("SELECT id, room FROM players WHERE username = ?")
                .bind(name)
                .fetch_one(&self.pool)
                .await?;

        let id = match player::Id::try_from(player_row.id) {
            Ok(id) => id,
            Err(_) => bail!("Failed to deserialize object ID: {}", player_row.id),
        };

        let room = room::Id::try_from(player_row.room)
            .ok()
            .and_then(|id| world.get_resource::<Rooms>().unwrap().by_id(id))
            .unwrap_or_else(|| {
                world
                    .get_resource::<Rooms>()
                    .unwrap()
                    .by_id(*VOID_ROOM_ID)
                    .unwrap()
            });

        let player = world
            .spawn()
            .insert_bundle(PlayerBundle {
                player: Player {
                    id,
                    name: name.to_string(),
                    room,
                },
                contents: Contents::default(),
            })
            .id();

        world
            .get_mut::<Room>(room)
            .ok_or_else(|| MissingComponent::new(room, "Room"))?
            .players
            .push(player);

        world
            .get_resource_mut::<Players>()
            .unwrap()
            .insert(player, name.to_string());

        self.load_player_inventory(world.deref_mut(), name).await?;

        Ok(player)
    }

    async fn load_player_inventory(&self, world: &mut World, name: &str) -> anyhow::Result<()> {
        let mut results = sqlx::query_as::<_, ObjectRow>(
            r#"SELECT objects.id, player_id AS container, keywords, short, long
                FROM objects
                INNER JOIN player_objects ON player_objects.object_id = objects.id
                INNER JOIN players ON player_objects.player_id = players.id
                    AND players.username = ?"#,
        )
        .bind(name)
        .fetch(&self.pool);

        while let Some(object) = results.try_next().await? {
            let player_entity = match world.get_resource::<Players>().unwrap().by_name(name) {
                Some(room) => room,
                None => bail!("Failed to retrieve Player {}.", name),
            };

            let id = match object::Id::try_from(object.id) {
                Ok(id) => id,
                Err(_) => bail!("Failed to deserialize object ID: {}", object.id),
            };

            let object = Object::new(
                id,
                player_entity,
                object.keywords(),
                object.short,
                object.long,
            );

            let object_entity = world.spawn().insert(object).id();
            world
                .get_mut::<Contents>(player_entity)
                .unwrap()
                .objects
                .push(object_entity);

            world
                .get_resource_mut::<Objects>()
                .unwrap()
                .insert(id, object_entity);
        }

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct PlayerRow {
    id: i64,
    room: i64,
}

#[derive(sqlx::FromRow)]
struct RoomObjectRow {
    room_id: i64,
    object_id: i64,
}

impl TryFrom<RoomObjectRow> for (room::Id, object::Id) {
    type Error = anyhow::Error;

    fn try_from(value: RoomObjectRow) -> Result<Self, Self::Error> {
        let room_id = room::Id::try_from(value.room_id)?;
        let object_id = object::Id::try_from(value.object_id)?;
        Ok((room_id, object_id))
    }
}

#[derive(sqlx::FromRow)]
struct ObjectRow {
    id: i64,
    container: i64,
    keywords: String,
    long: String,
    short: String,
}

impl ObjectRow {
    fn keywords(&self) -> Vec<String> {
        self.keywords
            .split(',')
            .map(ToString::to_string)
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
        Ok(Room::new(room::Id::try_from(value.id)?, value.description))
    }
}

#[derive(sqlx::FromRow)]
struct ExitRow {
    room_from: i64,
    room_to: i64,
    direction: String,
}
