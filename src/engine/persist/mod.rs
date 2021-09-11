pub mod object;
pub mod player;
pub mod room;

use std::mem;

use async_trait::async_trait;
use bevy_ecs::prelude::*;
use sqlx::SqlitePool;

pub type DynUpdate = Box<dyn Persist + Send + Sync>;

#[derive(Default)]
pub struct Updates {
    updates: Vec<DynUpdate>,
}

impl Updates {
    pub fn queue(&mut self, update: DynUpdate) {
        self.updates.push(update);
    }

    pub fn take(&mut self) -> Vec<DynUpdate> {
        let mut updates = Vec::new();
        mem::swap(&mut self.updates, &mut updates);
        updates
    }
}

#[async_trait]
pub trait Persist {
    async fn enact(&self, pool: &SqlitePool, world: &World) -> anyhow::Result<()>;
}
