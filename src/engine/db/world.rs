use std::{collections::HashMap, convert::TryFrom, str::FromStr};

use anyhow::bail;
use bevy_app::Events;
use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use itertools::Itertools;
use sqlx::{Row, SqlitePool};

use crate::{
    engine::db::{HookRow, ObjectRow, ScriptRow},
    world::{
        scripting::{Script, ScriptHook, ScriptHooks, ScriptInit, Scripts, TriggerKind},
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

pub async fn load_world(pool: &SqlitePool) -> anyhow::Result<World> {
    let mut world = World::new();

    world.insert_resource(Events::<ScriptInit>::default());

    load_configuration(pool, &mut world).await?;
    load_rooms(pool, &mut world).await?;
    load_exits(pool, &mut world).await?;
    load_prototypes(pool, &mut world).await?;
    load_room_objects(pool, &mut world).await?;
    load_scripts(pool, &mut world).await?;
    load_room_scripts(pool, &mut world).await?;
    load_prototype_scripts(pool, &mut world).await?;
    load_object_scripts(pool, &mut world).await?;

    Ok(world)
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

        let id = RoomId::try_from(room.id)?;
        let entity = world
            .spawn()
            .insert_bundle(RoomBundle {
                id: Id::Room(id),
                name: Named::from(room.name),
                description: Description::from(room.description.clone()),
                room: Room::from(id),
                contents: Contents::default(),
                regions: Regions::new(regions),
                hooks: ScriptHooks::default(),
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
            .insert_exit(direction, to);
    }

    Ok(())
}

async fn load_prototypes(pool: &SqlitePool, world: &mut World) -> anyhow::Result<()> {
    let mut results = sqlx::query_as::<_, PrototypeRow>(
        r#"SELECT id, flags, keywords, name, description FROM prototypes"#,
    )
    .fetch(pool);

    let mut by_id = HashMap::new();

    while let Some(prototype_row) = results.try_next().await? {
        let id = PrototypeId::try_from(prototype_row.id)?;
        let bundle = PrototypeBundle {
            prototype: Prototype::from(id),
            flags: ObjectFlags::from(prototype_row.flags),
            name: Named::from(prototype_row.name.clone()),
            description: Description::from(prototype_row.description.clone()),
            keywords: Keywords::from(prototype_row.keywords()),
            hooks: ScriptHooks::default(),
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

async fn load_room_objects(pool: &SqlitePool, world: &mut World) -> anyhow::Result<()> {
    let mut results = sqlx::query_as::<_, ObjectRow>(
        r#"SELECT objects.id, objects.prototype_id, objects.inherit_scripts, room_id AS container,
                    COALESCE(objects.name, prototypes.name) AS name, COALESCE(objects.description, prototypes.description) AS description,
                    COALESCE(objects.flags, prototypes.flags) AS flags, COALESCE(objects.keywords, prototypes.keywords) AS keywords
                FROM objects
                INNER JOIN room_objects ON room_objects.object_id = objects.id
                INNER JOIN prototypes ON objects.prototype_id = prototypes.id"#,
    )
    .fetch(pool);

    let mut by_id = HashMap::new();

    while let Some(object_row) = results.try_next().await? {
        let room_id = RoomId::try_from(object_row.container.unwrap())?;
        let room_entity = match world.get_resource::<Rooms>().unwrap().by_id(room_id) {
            Some(entity) => entity,
            None => bail!("Room {} not found", room_id),
        };
        let id = ObjectId::try_from(object_row.id)?;
        let prototype = match world
            .get_resource::<Prototypes>()
            .unwrap()
            .by_id(PrototypeId::try_from(object_row.prototype_id)?)
        {
            Some(entity) => entity,
            None => bail!("Prototype {} not found", object_row.prototype_id),
        };

        let bundle = object_row.into_object_bundle(prototype)?;
        let location = Location::from(room_entity);

        let object_entity = world.spawn().insert_bundle(bundle).insert(location).id();

        match world.get_mut::<Contents>(room_entity) {
            Some(mut contents) => contents.insert(object_entity),
            None => bail!("Failed to retrieve Room for room {:?}", room_entity),
        }

        by_id.insert(id, object_entity);
    }

    let results = sqlx::query("SELECT MAX(id) AS max_id FROM objects")
        .fetch_one(pool)
        .await?;
    let highest_id = results.get("max_id");

    world.insert_resource(Objects::new(highest_id, by_id));

    Ok(())
}

pub async fn load_scripts(pool: &SqlitePool, world: &mut World) -> anyhow::Result<()> {
    world.insert_resource(Scripts::default());

    let mut results = sqlx::query_as::<_, ScriptRow>(
        r#"SELECT name, trigger, code
                    FROM scripts"#,
    )
    .fetch(pool);

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

async fn load_prototype_scripts(pool: &SqlitePool, world: &mut World) -> anyhow::Result<()> {
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

            world
                .get_mut::<ScriptHooks>(prototype)
                .unwrap()
                .list
                .push(hook);
        }
    }

    Ok(())
}

async fn load_room_scripts(pool: &SqlitePool, world: &mut World) -> anyhow::Result<()> {
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

            world.get_mut::<ScriptHooks>(room).unwrap().list.push(hook)
        }
    }

    Ok(())
}

async fn load_object_scripts(pool: &SqlitePool, world: &mut World) -> anyhow::Result<()> {
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
                    .get_resource_mut::<Events<ScriptInit>>()
                    .unwrap()
                    .send(ScriptInit::new(object, hook.script.clone()));
            }

            world
                .get_mut::<ScriptHooks>(object)
                .unwrap()
                .list
                .push(hook);
        }
    }

    Ok(())
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
