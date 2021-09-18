use std::{
    borrow::Cow,
    collections::HashMap,
    convert::TryFrom,
    str::FromStr,
    sync::{Arc, RwLock},
};

use anyhow::bail;
use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use itertools::Itertools;
use lazy_static::lazy_static;
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};

use crate::world::{
    action::{self},
    scripting::{
        PostEventScriptHooks, PreEventScriptHooks, Script, ScriptHook, ScriptName, Scripts, Trigger,
    },
    types::{
        self,
        object::{Object, ObjectBundle, ObjectFlags, ObjectId, Objects},
        player::{Messages, Player, PlayerBundle, PlayerId, Players},
        room::{Direction, Room, RoomBundle, RoomId, Rooms},
        Configuration, Container, Contents, Description, Id, Keywords, Location, Named,
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
        let mut world = World::new();

        self.load_configuration(&mut world).await?;
        self.load_rooms(&mut world).await?;
        self.load_exits(&mut world).await?;
        self.load_room_objects(&mut world).await?;
        self.load_scripts(&mut world).await?;
        self.load_object_scripts(&mut world).await?;
        self.load_room_scripts(&mut world).await?;

        Ok(world)
    }

    async fn load_configuration(&self, world: &mut World) -> anyhow::Result<()> {
        let config_row = sqlx::query(r#"SELECT value FROM config WHERE key = "spawn_room""#)
            .fetch_one(&self.pool)
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

    async fn load_rooms(&self, world: &mut World) -> anyhow::Result<()> {
        let mut rooms_by_id = HashMap::new();

        let mut results =
            sqlx::query_as::<_, RoomRow>("SELECT id, description FROM rooms").fetch(&self.pool);

        while let Some(room) = results.try_next().await? {
            let id = room.id;
            let entity = world
                .spawn()
                .insert_bundle(RoomBundle {
                    id: Id::Room(RoomId::try_from(id)?),
                    description: Description {
                        text: room.description.clone(),
                    },
                    room: Room::try_from(room)?,
                    contents: Contents::default(),
                })
                .id();
            rooms_by_id.insert(RoomId::try_from(id)?, entity);
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

    async fn load_room_objects(&self, world: &mut World) -> anyhow::Result<()> {
        let mut results = sqlx::query_as::<_, ObjectRow>(
            r#"SELECT id, flags, room_id AS container, keywords, name, description
                FROM objects
                INNER JOIN room_objects ON room_objects.object_id = objects.id"#,
        )
        .fetch(&self.pool);

        let mut by_id = HashMap::new();

        while let Some(object_row) = results.try_next().await? {
            let room_id = match RoomId::try_from(object_row.container) {
                Ok(id) => id,
                Err(_) => bail!("Failed to deserialize room ID: {}", object_row.container),
            };

            let room_entity = match world.get_resource::<Rooms>().unwrap().by_id(room_id) {
                Some(room) => room,
                None => bail!("Failed to retrieve Room for room {}", room_id),
            };

            let id = match ObjectId::try_from(object_row.id) {
                Ok(id) => id,
                Err(_) => bail!("Failed to deserialize object ID: {}", object_row.id),
            };

            let bundle = ObjectBundle {
                id: Id::Object(id),
                flags: types::Flags {
                    flags: types::object::ObjectFlags::from_bits_truncate(object_row.flags),
                },
                name: Named {
                    name: object_row.name.clone(),
                },
                description: Description {
                    text: object_row.description.clone(),
                },
                keywords: Keywords {
                    list: object_row.keywords(),
                },
                object: Object { id },
            };

            let location = Location { room: room_entity };

            let object_entity = world.spawn().insert_bundle(bundle).insert(location).id();
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

    pub async fn load_scripts(&self, world: &mut World) -> anyhow::Result<()> {
        world.insert_resource(Scripts::default());

        let mut results = sqlx::query_as::<_, ScriptRow>(
            r#"SELECT name, trigger, code
                    FROM scripts"#,
        )
        .fetch(&self.pool);

        while let Some(script_row) = results.try_next().await? {
            let script = Script::try_from(script_row)?;
            let name = script.name.clone();
            let entity = world.spawn().insert(script).id();
            world
                .get_resource_mut::<Scripts>()
                .unwrap()
                .insert(name, entity);
        }

        Ok(())
    }

    async fn load_room_scripts(&self, world: &mut World) -> anyhow::Result<()> {
        let rooms = world
            .query::<&Room>()
            .iter(world)
            .map(|room| room.id)
            .collect_vec();

        for room_id in rooms {
            let room = world
                .get_resource::<Rooms>()
                .unwrap()
                .by_id(room_id)
                .unwrap();

            let mut results = sqlx::query_as::<_, HookRow>(
                r#"SELECT kind, script, trigger FROM room_scripts WHERE room_id = ?"#,
            )
            .bind(room_id)
            .fetch(&self.pool);

            while let Some(hook_row) = results.try_next().await? {
                let pre = match hook_row.kind.as_str() {
                    "pre" => true,
                    "post" => false,
                    _ => bail!("Unknown kind field for room script hook: {:?}", hook_row),
                };

                let hook = ScriptHook::try_from(hook_row)?;

                if pre {
                    if let Some(mut hooks) = world.get_mut::<PreEventScriptHooks>(room) {
                        hooks.list.push(hook)
                    } else {
                        world
                            .entity_mut(room)
                            .insert(PreEventScriptHooks { list: vec![hook] });
                    }
                } else if let Some(mut hooks) = world.get_mut::<PostEventScriptHooks>(room) {
                    hooks.list.push(hook)
                } else {
                    world
                        .entity_mut(room)
                        .insert(PostEventScriptHooks { list: vec![hook] });
                }
            }
        }

        Ok(())
    }

    async fn load_object_scripts(&self, world: &mut World) -> anyhow::Result<()> {
        let objects = world
            .query::<&Object>()
            .iter(world)
            .map(|object| object.id)
            .collect_vec();

        for object_id in objects {
            let object = world
                .get_resource::<Objects>()
                .unwrap()
                .by_id(object_id)
                .unwrap();

            let mut results = sqlx::query_as::<_, HookRow>(
                r#"SELECT kind, script, trigger FROM object_scripts WHERE object_id = ?"#,
            )
            .bind(object_id)
            .fetch(&self.pool);

            while let Some(hook_row) = results.try_next().await? {
                let pre = match hook_row.kind.as_str() {
                    "pre" => true,
                    "post" => false,
                    _ => bail!("Unknown kind field for object script hook: {:?}", hook_row),
                };

                let hook = ScriptHook::try_from(hook_row)?;

                if pre {
                    if let Some(mut hooks) = world.get_mut::<PreEventScriptHooks>(object) {
                        hooks.list.push(hook)
                    } else {
                        world
                            .entity_mut(object)
                            .insert(PreEventScriptHooks { list: vec![hook] });
                    }
                } else if let Some(mut hooks) = world.get_mut::<PostEventScriptHooks>(object) {
                    hooks.list.push(hook)
                } else {
                    world
                        .entity_mut(object)
                        .insert(PostEventScriptHooks { list: vec![hook] });
                }
            }
        }

        Ok(())
    }

    pub async fn load_player(
        &self,
        world: Arc<RwLock<World>>,
        name: &str,
    ) -> anyhow::Result<Entity> {
        let (player, id) = {
            let player_row =
                sqlx::query_as::<_, PlayerRow>("SELECT id, room FROM players WHERE username = ?")
                    .bind(name)
                    .fetch_one(&self.pool)
                    .await?;

            let mut world = world.write().unwrap();

            let id = match PlayerId::try_from(player_row.id) {
                Ok(id) => id,
                Err(_) => bail!("Failed to deserialize object ID: {}", player_row.id),
            };

            let room = RoomId::try_from(player_row.room)
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
                    name: Named {
                        name: name.to_string(),
                    },
                    location: Location { room },
                    player: Player { id },
                    contents: Contents::default(),
                    messages: Messages::default(),
                    id: Id::Player(id),
                })
                .id();

            world
                .get_mut::<Room>(room)
                .ok_or(action::Error::MissingComponent(room, "Room"))?
                .players
                .push(player);

            world
                .get_resource_mut::<Players>()
                .unwrap()
                .insert(player, name.to_string());

            (player, id)
        };

        self.load_player_inventory(world.clone(), name, player)
            .await?;
        self.load_player_scripts(world, id, player).await?;

        Ok(player)
    }

    async fn load_player_inventory(
        &self,
        world: Arc<RwLock<World>>,
        name: &str,
        player: Entity,
    ) -> anyhow::Result<()> {
        let mut results = sqlx::query_as::<_, ObjectRow>(
            r#"SELECT objects.id, flags, player_id AS container, keywords, name, description
                FROM objects
                INNER JOIN player_objects ON player_objects.object_id = objects.id
                INNER JOIN players ON player_objects.player_id = players.id
                    AND players.username = ?"#,
        )
        .bind(name)
        .fetch(&self.pool);

        while let Some(object_row) = results.try_next().await? {
            let mut world = world.write().unwrap();

            let id = match ObjectId::try_from(object_row.id) {
                Ok(id) => id,
                Err(_) => bail!("Failed to deserialize object ID: {}", object_row.id),
            };

            let bundle = ObjectBundle {
                id: Id::Object(id),
                flags: types::Flags {
                    flags: ObjectFlags::from_bits_truncate(object_row.flags),
                },
                name: Named {
                    name: object_row.name.clone(),
                },
                description: Description {
                    text: object_row.description.clone(),
                },
                keywords: Keywords {
                    list: object_row.keywords(),
                },
                object: Object { id },
            };

            let container = Container { entity: player };

            let object_entity = world.spawn().insert_bundle(bundle).insert(container).id();
            world
                .get_mut::<Contents>(player)
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

    async fn load_player_scripts(
        &self,
        world: Arc<RwLock<World>>,
        id: PlayerId,
        player: Entity,
    ) -> anyhow::Result<()> {
        let mut results = sqlx::query_as::<_, HookRow>(
            r#"SELECT kind, script, trigger FROM player_scripts WHERE player_id = ?"#,
        )
        .bind(id)
        .fetch(&self.pool);

        while let Some(hook_row) = results.try_next().await? {
            let mut world = world.write().unwrap();
            let pre = match hook_row.kind.as_str() {
                "pre" => true,
                "post" => false,
                _ => bail!("Unknown kind field for player script hook: {:?}", hook_row),
            };

            let hook = ScriptHook::try_from(hook_row)?;

            if pre {
                if let Some(mut hooks) = world.get_mut::<PreEventScriptHooks>(player) {
                    hooks.list.push(hook)
                } else {
                    world
                        .entity_mut(player)
                        .insert(PreEventScriptHooks { list: vec![hook] });
                }
            } else if let Some(mut hooks) = world.get_mut::<PostEventScriptHooks>(player) {
                hooks.list.push(hook)
            } else {
                world
                    .entity_mut(player)
                    .insert(PostEventScriptHooks { list: vec![hook] });
            }
        }

        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PlayerRow {
    id: i64,
    room: i64,
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
        let name = ScriptName::from(value.name.as_str());
        let trigger = Trigger::from_str(value.trigger.as_str())?;

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
        let script = ScriptName::from(value.script.as_str());
        let trigger = Trigger::from_str(value.trigger.as_str())?;

        Ok(ScriptHook { script, trigger })
    }
}
