pub mod object;
pub mod player;
pub mod prototype;
pub mod room;
pub mod script;

use std::mem;

use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    ecs::{Ecs, Plugin},
    world::types::object::PrototypeId,
};

pub type DynPersist = Box<dyn Persist + Send + Sync>;

#[derive(Default)]
pub struct PersistPlugin {}

impl Plugin for PersistPlugin {
    fn build(&self, ecs: &mut Ecs) {
        ecs.init_resource::<Updates>();
    }
}

#[derive(Default)]
pub struct Updates {
    updates: Vec<DynPersist>,
    reloads: Vec<PrototypeId>,
}

impl Updates {
    pub fn persist(&mut self, update: DynPersist) {
        self.updates.push(update);
    }

    pub fn reload(&mut self, prototype: PrototypeId) {
        self.reloads.push(prototype);
    }

    pub fn take_updates(&mut self) -> Vec<DynPersist> {
        let mut updates = Vec::new();
        mem::swap(&mut self.updates, &mut updates);
        updates
    }

    pub fn take_reloads(&mut self) -> Vec<PrototypeId> {
        let mut reloads = Vec::new();
        mem::swap(&mut self.reloads, &mut reloads);
        reloads
    }
}

#[async_trait]
pub trait Persist {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()>;
}

// An list of Persist operations that must be completed in order.
pub struct UpdateGroup {
    list: Vec<DynPersist>,
}

impl UpdateGroup {
    pub fn new(list: Vec<DynPersist>) -> Box<Self> {
        Box::new(UpdateGroup { list })
    }

    pub fn append(&mut self, update: DynPersist) {
        self.list.push(update);
    }
}

#[async_trait]
impl Persist for UpdateGroup {
    #[tracing::instrument(name = "update group", skip_all)]
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        for update in self.list.iter() {
            update.enact(pool).await?;
        }

        Ok(())
    }
}
