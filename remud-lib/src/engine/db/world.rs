use std::{collections::HashMap, convert::TryFrom, str::FromStr};

use bevy_app::Events;
use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use itertools::Itertools;
use sqlx::{Row, SqlitePool};

use crate::{
    engine::db::{DbResult, Error, HookRow, ObjectRow},
    world::{
        scripting::{
            RunInitScript, Script, ScriptHook, ScriptHooks, ScriptName, Scripts, TriggerEvent,
            TriggerKind,
        },
        types::{
            object::{
                Keywords, Object, ObjectFlags, ObjectId, Objects, Prototype, PrototypeBundle,
                PrototypeId, Prototypes,
            },
            room::{Direction, Regions, Room, RoomBundle, RoomId, Rooms},
            Configuration, Contents, Description, Id, Location, Named,
        },
    },
};

#[tracing::instrument(name = "loading world")]
pub async fn load_world(pool: &SqlitePool, world: &mut World) -> Result<(), Error> {
    load_configuration(pool, world).await?;
    load_rooms(pool, world).await?;
    load_exits(pool, world).await?;
    load_prototypes(pool, world).await?;
    load_room_objects(pool, world).await?;
    load_scripts(pool, world).await?;
    load_room_scripts(pool, world).await?;
    load_prototype_scripts(pool, world).await?;
    load_object_scripts(pool, world).await?;

    Ok(())
}

#[tracing::instrument(name = "loading configuration")]
async fn load_configuration(pool: &SqlitePool, world: &mut World) -> DbResult<()> {
    let config_row = sqlx::query(r#"SELECT value FROM config WHERE key = "spawn_room""#)
        .fetch_one(pool)
        .await?;

    let spawn_room_str: String = config_row.get("value");
    let spawn_room = RoomId::try_from(
        spawn_room_str
            .parse::<i64>()
            .map_err(|_| Error::Deserialize("spawn room config value"))?,
    )
    .map_err(|_| Error::Deserialize("spawn room Room ID"))?;

    let configuration = Configuration {
        restart: false,
        shutdown: false,
        spawn_room,
    };

    world.insert_resource(configuration);

    Ok(())
}

#[tracing::instrument(name = "loading rooms")]
async fn load_rooms(pool: &SqlitePool, world: &mut World) -> DbResult<()> {
    let mut rooms_by_id = HashMap::new();

    let mut results =
        sqlx::query_as::<_, RoomRow>("SELECT id, name, description FROM rooms").fetch(pool);

    while let Some(room) = results.try_next().await? {
        let regions = sqlx::query(
            r#"SELECT name FROM regions
                INNER JOIN room_regions ON region_id = regions.id
                        AND room_id = ?"#,
        )
        .bind(room.id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect_vec();

        let id = RoomId::try_from(room.id).map_err(|_| Error::Deserialize("room ID"))?;
        let entity = world
            .spawn()
            .insert_bundle(RoomBundle {
                id: Id::Room(id),
                room: Room::from(id),
                name: Named::from(room.name),
                description: Description::from(room.description.clone()),
                regions: Regions::new(regions),
                contents: Contents::default(),
            })
            .id();
        rooms_by_id.insert(id, entity);
    }

    let highest_id = sqlx::query("SELECT MAX(id) AS max_id FROM rooms")
        .fetch_one(pool)
        .await?
        .get("max_id");

    let rooms = Rooms::new(rooms_by_id, highest_id);
    world.insert_resource(rooms);

    Ok(())
}

#[tracing::instrument(name = "loading exits")]
async fn load_exits(pool: &SqlitePool, world: &mut World) -> DbResult<()> {
    let mut results =
        sqlx::query_as::<_, ExitRow>("SELECT room_from, room_to, direction FROM exits").fetch(pool);

    while let Some(exit) = results.try_next().await? {
        let (from, to) = {
            let rooms = &world.get_resource::<Rooms>().unwrap();
            let from = rooms
                .by_id(
                    RoomId::try_from(exit.room_from)
                        .map_err(|_| Error::Deserialize("room exit from Room ID"))?,
                )
                .unwrap();
            let to = rooms
                .by_id(
                    RoomId::try_from(exit.room_to)
                        .map_err(|_| Error::Deserialize("room exit to Room ID"))?,
                )
                .unwrap();
            (from, to)
        };

        let direction = Direction::from_str(exit.direction.as_str()).unwrap();

        world
            .get_mut::<Room>(from)
            .unwrap()
            .insert_exit(direction, to);
    }

    Ok(())
}

#[tracing::instrument(name = "loading prototypes")]
async fn load_prototypes(pool: &SqlitePool, world: &mut World) -> DbResult<()> {
    let mut results = sqlx::query_as::<_, PrototypeRow>(
        r#"SELECT id, flags, keywords, name, description FROM prototypes"#,
    )
    .fetch(pool);

    let mut by_id = HashMap::new();

    while let Some(prototype_row) = results.try_next().await? {
        let id = PrototypeId::try_from(prototype_row.id)
            .map_err(|_| Error::Deserialize("prototype ID"))?;
        let bundle = PrototypeBundle {
            prototype: Prototype::from(id),
            flags: ObjectFlags::from(prototype_row.flags),
            name: Named::from(prototype_row.name.clone()),
            description: Description::from(prototype_row.description.clone()),
            keywords: Keywords::from(prototype_row.keywords()),
        };

        let object_entity = world.spawn().insert_bundle(bundle).id();

        by_id.insert(id, object_entity);
    }

    let results = sqlx::query("SELECT MAX(id) AS max_id FROM prototypes")
        .fetch_one(pool)
        .await?;
    let highest_id = results.get("max_id");

    world.insert_resource(Prototypes::new(highest_id, by_id));

    Ok(())
}

#[tracing::instrument(name = "loading room objects")]
async fn load_room_objects(pool: &SqlitePool, world: &mut World) -> DbResult<()> {
    let mut results = sqlx::query_as::<_, ObjectRow>(
        r#"SELECT objects.id, objects.prototype_id, objects.inherit_scripts, room_id AS location,
                    COALESCE(objects.name, prototypes.name) AS name, COALESCE(objects.description, prototypes.description) AS description,
                    COALESCE(objects.flags, prototypes.flags) AS flags, COALESCE(objects.keywords, prototypes.keywords) AS keywords
                FROM objects
                INNER JOIN room_objects ON room_objects.object_id = objects.id
                INNER JOIN prototypes ON objects.prototype_id = prototypes.id"#,
    )
    .fetch(pool);

    let mut by_id = HashMap::new();

    while let Some(object_row) = results.try_next().await? {
        let room_id = RoomId::try_from(object_row.location.unwrap())
            .map_err(|_| Error::Deserialize("room ID"))?;
        let room_entity = world
            .get_resource::<Rooms>()
            .unwrap()
            .by_id(room_id)
            .ok_or(Error::MissingData("room not found"))?;
        let id = ObjectId::try_from(object_row.id).map_err(|_| Error::Deserialize("object ID"))?;
        let prototype = world
            .get_resource::<Prototypes>()
            .unwrap()
            .by_id(
                PrototypeId::try_from(object_row.prototype_id)
                    .map_err(|_| Error::Deserialize("prototype ID"))?,
            )
            .ok_or(Error::MissingData("prototype not found"))?;

        let bundle = object_row.into_object_bundle(prototype, Location::from(room_entity))?;
        let object_entity = world.spawn().insert_bundle(bundle).id();

        world
            .get_mut::<Contents>(room_entity)
            .ok_or(Error::MissingData("room contents"))?
            .insert(object_entity);

        by_id.insert(id, object_entity);
    }

    let results = sqlx::query("SELECT MAX(id) AS max_id FROM objects")
        .fetch_one(pool)
        .await?;
    let highest_id = results.get("max_id");

    world.insert_resource(Objects::new(highest_id, by_id));

    Ok(())
}

#[tracing::instrument(name = "loading scripts")]
pub async fn load_scripts(pool: &SqlitePool, world: &mut World) -> DbResult<()> {
    world.insert_resource(Scripts::default());

    let mut results = sqlx::query_as::<_, ScriptRow>(
        r#"SELECT name, trigger, code
                    FROM scripts"#,
    )
    .fetch(pool);

    while let Some(script_row) = results.try_next().await? {
        let script = Script::try_from(script_row)?;
        let name = script.name().clone();
        let entity = world.spawn().insert(script).id();
        world
            .get_resource_mut::<Scripts>()
            .unwrap()
            .insert(name, entity);
    }

    Ok(())
}

#[tracing::instrument(name = "loading prototype scripts")]
async fn load_prototype_scripts(pool: &SqlitePool, world: &mut World) -> DbResult<()> {
    let prototypes = world
        .query::<&Prototype>()
        .iter(world)
        .map(|prototype| prototype.id())
        .collect_vec();

    for prototype_id in prototypes {
        let prototype = world
            .get_resource::<Prototypes>()
            .unwrap()
            .by_id(prototype_id)
            .unwrap();

        let mut results = sqlx::query_as::<_, HookRow>(
            r#"SELECT kind, script, trigger FROM prototype_scripts WHERE prototype_id = ?"#,
        )
        .bind(prototype_id)
        .fetch(pool);

        while let Some(hook_row) = results.try_next().await? {
            let hook = ScriptHook::try_from(hook_row)?;

            if let Some(mut hooks) = world.get_mut::<ScriptHooks>(prototype) {
                hooks.insert(hook)
            } else {
                world.entity_mut(prototype).insert(ScriptHooks::new(hook));
            }
        }
    }

    Ok(())
}

#[tracing::instrument(name = "loading prototype scripts")]
async fn load_room_scripts(pool: &SqlitePool, world: &mut World) -> DbResult<()> {
    let rooms = world
        .query::<&Room>()
        .iter(world)
        .map(|room| room.id())
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
        .fetch(pool);

        while let Some(hook_row) = results.try_next().await? {
            let hook = ScriptHook::try_from(hook_row)?;

            if let Some(mut hooks) = world.get_mut::<ScriptHooks>(room) {
                hooks.insert(hook)
            } else {
                world.entity_mut(room).insert(ScriptHooks::new(hook));
            }
        }
    }

    Ok(())
}

#[tracing::instrument(name = "loading object scripts")]
async fn load_object_scripts(pool: &SqlitePool, world: &mut World) -> DbResult<()> {
    let objects = world
        .query::<&Object>()
        .iter(world)
        .map(|object| (object.id(), object.inherit_scripts(), object.prototype()))
        .collect_vec();

    for (object_id, inherit, prototype) in objects {
        let object = world
            .get_resource::<Objects>()
            .unwrap()
            .by_id(object_id)
            .unwrap();

        let mut results = if inherit {
            let prototype_id = world.get::<Prototype>(prototype).unwrap().id();
            sqlx::query_as::<_, HookRow>(
                r#"SELECT kind, script, trigger FROM prototype_scripts WHERE prototype_id = ?"#,
            )
            .bind(prototype_id)
            .fetch(pool)
        } else {
            sqlx::query_as::<_, HookRow>(
                r#"SELECT kind, script, trigger FROM object_scripts WHERE object_id = ?"#,
            )
            .bind(object_id)
            .fetch(pool)
        };

        while let Some(hook_row) = results.try_next().await? {
            let hook = ScriptHook::try_from(hook_row)?;

            if hook.trigger.kind() == TriggerKind::Init {
                world
                    .get_resource_mut::<Events<RunInitScript>>()
                    .unwrap()
                    .send(RunInitScript::new(object, hook.script.clone()));
            }

            if let Some(mut hooks) = world.get_mut::<ScriptHooks>(object) {
                hooks.insert(hook)
            } else {
                world.entity_mut(object).insert(ScriptHooks::new(hook));
            }
        }
    }

    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
struct RoomRow {
    id: i64,
    name: String,
    description: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ExitRow {
    room_from: i64,
    room_to: i64,
    direction: String,
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
struct PrototypeRow {
    id: i64,
    flags: i64,
    name: String,
    keywords: String,
    description: String,
}

impl PrototypeRow {
    fn keywords(&self) -> Vec<String> {
        self.keywords
            .split(',')
            .map(ToString::to_string)
            .collect_vec()
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ScriptRow {
    name: String,
    trigger: String,
    code: String,
}

impl TryFrom<ScriptRow> for Script {
    type Error = Error;

    fn try_from(value: ScriptRow) -> Result<Self, Self::Error> {
        let name =
            ScriptName::try_from(value.name).map_err(|_| Error::Deserialize("script name"))?;
        let trigger = TriggerEvent::from_str(value.trigger.as_str())
            .map_err(|_| Error::Deserialize("script trigger event"))?;

        Ok(Script::new(name, trigger, value.code))
    }
}
