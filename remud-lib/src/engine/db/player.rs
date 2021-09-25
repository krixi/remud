use std::convert::TryFrom;

use anyhow::bail;
use bevy_app::Events;
use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use sqlx::SqlitePool;

use crate::{
    ecs::SharedWorld,
    engine::db::HookRow,
    world::{
        scripting::{RunInitScript, ScriptHook, ScriptHooks, TriggerKind},
        types::{
            object::{Container, ObjectId, Objects, PrototypeId, Prototypes},
            player::{Messages, Player, PlayerBundle, PlayerFlags, PlayerId, Players},
            room::{Room, RoomId, Rooms},
            Contents, Description, Id, Location, Named,
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
    world: SharedWorld,
    name: &str,
) -> anyhow::Result<Entity> {
    let (player, id) = {
        let player_row = sqlx::query_as::<_, PlayerRow>(
            "SELECT id, description, room, flags FROM players WHERE username = ?",
        )
        .bind(name)
        .fetch_one(pool)
        .await?;

        let mut world = world.write().await;

        let id = PlayerId::try_from(player_row.id)?;

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
                id: Id::Player(id),
                player: Player::from(id),
                messages: Messages::default(),
                name: Named::from(name.to_string()),
                description: Description::from(player_row.description),
                flags: PlayerFlags::from(player_row.flags),
                location: Location::from(room),
                contents: Contents::default(),
                health: Health::new(&attributes),
                attributes,
            })
            .id();

        world.get_mut::<Room>(room).unwrap().insert_player(player);

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
    world: SharedWorld,
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
            let mut world = world.write().await;

            let prototype = match world
                .get_resource::<Prototypes>()
                .unwrap()
                .by_id(PrototypeId::try_from(object_row.prototype_id)?)
            {
                Some(entity) => entity,
                None => bail!("Prototype {} not found", object_row.prototype_id),
            };

            let bundle = object_row.into_object_bundle(prototype)?;

            let container = Container::from(player);

            let object_entity = world.spawn().insert_bundle(bundle).insert(container).id();

            world
                .get_mut::<Contents>(player)
                .unwrap()
                .insert(object_entity);

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
            let mut world = world.write().await;
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

async fn load_player_scripts(
    pool: &SqlitePool,
    world: SharedWorld,
    id: PlayerId,
    player: Entity,
) -> anyhow::Result<()> {
    let mut results = sqlx::query_as::<_, HookRow>(
        r#"SELECT kind, script, trigger FROM player_scripts WHERE player_id = ?"#,
    )
    .bind(id)
    .fetch(pool);

    while let Some(hook_row) = results.try_next().await? {
        let mut world = world.write().await;

        let hook = ScriptHook::try_from(hook_row)?;

        if let Some(mut hooks) = world.get_mut::<ScriptHooks>(player) {
            hooks.insert(hook)
        } else {
            world.entity_mut(player).insert(ScriptHooks::new(hook));
        }
    }

    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
struct PlayerRow {
    id: i64,
    description: String,
    room: i64,
    flags: i64,
}
