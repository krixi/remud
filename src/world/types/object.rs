use std::{collections::HashMap, convert::TryFrom, fmt, str::FromStr};

use bevy_ecs::prelude::Entity;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct ObjectId(i64);

impl TryFrom<i64> for ObjectId {
    type Error = ObjectIdParseError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(ObjectId(value))
        } else {
            Err(ObjectIdParseError {})
        }
    }
}

impl FromStr for ObjectId {
    type Err = ObjectIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let int = s.parse::<i64>().map_err(|_| ObjectIdParseError {})?;
        ObjectId::try_from(int)
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

pub struct ObjectIdParseError {}

pub struct Object {
    pub id: ObjectId,
    pub keywords: Vec<String>,
    pub short: String,
    pub long: String,
}

pub struct Objects {
    by_id: HashMap<ObjectId, Entity>,
    highest_id: i64,
}

impl Objects {
    pub fn new(highest_id: i64) -> Self {
        Objects {
            by_id: HashMap::new(),
            highest_id,
        }
    }

    pub fn add_object(&mut self, id: ObjectId, entity: Entity) {
        self.by_id.insert(id, entity);
    }

    pub fn by_id(&self, id: ObjectId) -> Option<Entity> {
        self.by_id.get(&id).cloned()
    }

    pub fn next_id(&mut self) -> ObjectId {
        self.highest_id += 1;
        ObjectId(self.highest_id)
    }
}
