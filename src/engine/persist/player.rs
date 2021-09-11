use anyhow::bail;
use async_trait::async_trait;
use bevy_ecs::prelude::*;
use sqlx::SqlitePool;

use crate::{
    engine::persist::Persist,
    world::types::{object::Object, player::Player, room},
};

pub struct AddObject {
    player: Entity,
    object: Entity,
}

impl AddObject {
    pub fn new(player: Entity, object: Entity) -> Box<Self> {
        Box::new(AddObject { player, object })
    }
}

#[async_trait]
impl Persist for AddObject {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()> {
        let player_id = match world.get::<Player>(self.player).map(|player| player.id) {
            Some(id) => id,
            None => bail!("Player {:?} does not have Player", self.player),
        };

        let object_id = match world.get::<Object>(self.object).map(|object| object.id) {
            Some(id) => id,
            None => bail!("Object {:?} does not have Object", self.object),
        };

        sqlx::query("INSERT INTO player_objects (player_id, object_id) VALUES (?, ?)")
            .bind(player_id)
            .bind(object_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct RemoveObject {
    player: Entity,
    object: Entity,
}

impl RemoveObject {
    pub fn new(player: Entity, object: Entity) -> Box<Self> {
        Box::new(RemoveObject { player, object })
    }
}

#[async_trait]
impl Persist for RemoveObject {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()> {
        let player_id = match world.get::<Player>(self.player).map(|player| player.id) {
            Some(id) => id,
            None => bail!("Player {:?} does not have Player", self.player),
        };

        let object_id = match world.get::<Object>(self.object).map(|object| object.id) {
            Some(id) => id,
            None => bail!("Object {:?} does not have Object", self.object),
        };

        sqlx::query("DELETE FROM player_objects WHERE player_id = ? AND object_id = ?")
            .bind(player_id)
            .bind(object_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct Room {
    player: Entity,
}

impl Room {
    pub fn new(player: Entity) -> Box<Self> {
        Box::new(Room { player })
    }
}

#[async_trait]
impl Persist for Room {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()> {
        let (player_id, room) = match world.get::<Player>(self.player) {
            Some(player) => (player.id, player.room),
            None => bail!("Player {:?} does not have Player."),
        };

        let room_id = match world.get::<room::Room>(room) {
            Some(room) => room.id,
            None => bail!("Room {:?} does not have Room."),
        };

        sqlx::query("UPDATE players SET room = ? WHERE id = ?")
            .bind(room_id)
            .bind(player_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}
