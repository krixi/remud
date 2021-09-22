use std::{
    convert::TryFrom,
    sync::{Arc, RwLock},
};

use anyhow::bail;
use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use sqlx::SqlitePool;

use crate::{
    engine::db::HookRow,
    world::{
        action,
        scripting::{ScriptHook, ScriptHooks},
        types::{
            object::{ObjectId, Objects, PrototypeId, Prototypes},
            player::{Messages, Player, PlayerBundle, PlayerId, Players},
            room::{Room, RoomId, Rooms},
            Container, Contents, Description, Id, Location, Named,
        },
        VOID_ROOM_ID,
    },
};
use crate::{
    engine::db::ObjectRow,
    world::types::{Attributes, Health},
};

pub async fn load_player(
    pool: &SqlitePool,
    world: Arc<RwLock<World>>,
    name: &str,
) -> anyhow::Result<Entity> {
    let (player, id) = {
        let player_row = sqlx::query_as::<_, PlayerRow>(
            "SELECT id, description, room FROM players WHERE username = ?",
        )
        .bind(name)
        .fetch_one(pool)
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

        let attributes = Attributes::default();

        let player = world
            .spawn()
            .insert_bundle(PlayerBundle {
                name: Named {
                    name: name.to_string(),
                },
                description: Description {
                    text: player_row.description,
                },
                location: Location { room },
                player: Player { id },
                contents: Contents::default(),
                messages: Messages::default(),
                id: Id::Player(id),
                health: Health::new(&attributes),
                attributes,
                hooks: ScriptHooks::default(),
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
            .insert(player, name.to_string(), id);

        (player, id)
    };

    load_player_inventory(pool, world.clone(), name, player).await?;
    load_player_scripts(pool, world, id, player).await?;

    Ok(player)
}

async fn load_player_inventory(
    pool: &SqlitePool,
    world: Arc<RwLock<World>>,
    name: &str,
    player: Entity,
) -> anyhow::Result<()> {
    let mut results = sqlx::query_as::<_, ObjectRow>(
        r#"SELECT objects.id, objects.prototype_id, objects.inherit_scripts, NULL AS container,
                    COALESCE(objects.name, prototypes.name) AS name, COALESCE(objects.description, prototypes.description) AS description,
                    COALESCE(objects.flags, prototypes.flags) AS flags, COALESCE(objects.keywords, prototypes.keywords) AS keywords
                FROM objects
                INNER JOIN player_objects ON player_objects.object_id = objects.id
                INNER JOIN players ON player_objects.player_id = players.id
                INNER JOIN prototypes ON objects.prototype_id = prototypes.id
                    WHERE players.username = ?"#,
    )
    .bind(name)
    .fetch(pool);

    while let Some(object_row) = results.try_next().await? {
        let id = ObjectId::try_from(object_row.id)?;
        let inherit_scripts = object_row.inherit_scripts;
        let prototype_id = object_row.prototype_id;

        let object = {
            let mut world = world.write().unwrap();

            let prototype = match world
                .get_resource::<Prototypes>()
                .unwrap()
                .by_id(PrototypeId::try_from(object_row.prototype_id)?)
            {
                Some(entity) => entity,
                None => bail!("Prototype {} not found", object_row.prototype_id),
            };

            let bundle = object_row.into_object_bundle(prototype)?;

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

            object_entity
        };

        let mut results = if inherit_scripts {
            sqlx::query_as::<_, HookRow>(
                r#"SELECT kind, script, trigger FROM prototype_scripts WHERE prototype_id = ?"#,
            )
            .bind(prototype_id)
            .fetch(pool)
        } else {
            sqlx::query_as::<_, HookRow>(
                r#"SELECT kind, script, trigger FROM object_scripts WHERE object_id = ?"#,
            )
            .bind(id)
            .fetch(pool)
        };

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

    Ok(())
}

async fn load_player_scripts(
    pool: &SqlitePool,
    world: Arc<RwLock<World>>,
    id: PlayerId,
    player: Entity,
) -> anyhow::Result<()> {
    let mut results = sqlx::query_as::<_, HookRow>(
        r#"SELECT kind, script, trigger FROM player_scripts WHERE player_id = ?"#,
    )
    .bind(id)
    .fetch(pool);

    while let Some(hook_row) = results.try_next().await? {
        let mut world = world.write().unwrap();

        let hook = ScriptHook::try_from(hook_row)?;

        if let Some(mut hooks) = world.get_mut::<ScriptHooks>(player) {
            hooks.list.push(hook)
        } else {
            world
                .entity_mut(player)
                .insert(ScriptHooks { list: vec![hook] });
        }
    }

    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
struct PlayerRow {
    id: i64,
    description: String,
    room: i64,
}
