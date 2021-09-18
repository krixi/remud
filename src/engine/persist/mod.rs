pub mod object;
pub mod player;
pub mod room;
pub mod script;

use std::mem;

use async_trait::async_trait;
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
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()>;
}

// An list of Persist operations that must be completed in order.
pub struct UpdateGroup {
    list: Vec<DynUpdate>,
}

impl UpdateGroup {
    pub fn new(list: Vec<DynUpdate>) -> Box<Self> {
        Box::new(UpdateGroup { list })
    }

    pub fn append(&mut self, update: DynUpdate) {
        self.list.push(update);
    }
}

#[async_trait]
impl Persist for UpdateGroup {
    async fn enact(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        for update in self.list.iter() {
            update.enact(pool).await?;
        }

        Ok(())
    }
}
