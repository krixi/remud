use std::{
    convert::TryFrom,
    sync::{Arc, RwLock},
};

use anyhow::bail;
use bevy_ecs::prelude::*;
use futures::TryStreamExt;
use sqlx::SqlitePool;

use crate::world::types::{Attributes, Health};
use crate::{
    engine::db::{HookRow, ObjectRow},
    world::{
        action,
        scripting::{ScriptHook, ScriptHooks},
        types::{
            self,
            object::{Object, ObjectBundle, ObjectFlags, ObjectId, Objects},
            player::{Messages, Player, PlayerBundle, PlayerId, Players},
            room::{Room, RoomId, Rooms},
            Container, Contents, Description, Id, Keywords, Location, Named,
        },
        VOID_ROOM_ID,
    },
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
        r#"SELECT objects.id, flags, player_id AS container, keywords, name, objects.description
                FROM objects
                INNER JOIN player_objects ON player_objects.object_id = objects.id
                INNER JOIN players ON player_objects.player_id = players.id
                    AND players.username = ?"#,
    )
    .bind(name)
    .fetch(pool);

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
