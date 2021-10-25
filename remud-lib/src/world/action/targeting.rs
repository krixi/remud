use crate::world::types::object::Keywords;
use crate::world::types::room::Room;
use crate::world::types::{Contents, Description, Named};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;

pub struct Params {
    actor: Entity,
    room: Entity,
    keywords: Option<Vec<String>>,
    pub players_by_name: bool,
    pub objects_in_room: bool,
    pub objects_carried: bool,
}

impl Params {
    pub fn new(actor: Entity, room: Entity, keywords: Option<Vec<String>>) -> Self {
        Params {
            actor,
            room,
            keywords,
            players_by_name: true,
            objects_in_room: true,
            objects_carried: true,
        }
    }
}

pub struct Target {
    pub entity: Entity,
    pub name: String,
    pub desc: String,
}

#[derive(SystemParam)]
pub struct TargetFinder<'a> {
    name_desc: Query<
        'a,
        (
            Entity,
            &'static Named,
            &'static Description,
            Option<&'static Keywords>,
        ),
    >,
    rooms: Query<'a, &'static Room>,
    contents: Query<'a, &'static Contents>,
}

impl<'a> TargetFinder<'a> {
    pub fn resolve_player(&self, full_name: &str, room: Entity) -> Option<Target> {
        self.rooms.get(room).ok().and_then(|room| {
            room.players()
                .iter()
                .filter_map(|player| self.name_desc.get(*player).ok())
                .find(|(_, name, _, _)| full_name == name.as_str())
                .map(|(entity, name, desc, _)| Target {
                    entity,
                    name: name.to_string(),
                    desc: desc.to_string(),
                })
        })
    }

    pub fn resolve_object(&self, keywords: &[String], container: Entity) -> Option<Target> {
        self.contents.get(container).ok().and_then(|contents| {
            contents
                .objects()
                .iter()
                .filter_map(|object| self.name_desc.get(*object).ok())
                .find(|(_, _, _, object_keywords)| object_keywords.unwrap().contains_all(keywords))
                .map(|(entity, name, desc, _)| Target {
                    entity,
                    name: name.to_string(),
                    desc: desc.to_string(),
                })
        })
    }

    pub fn resolve(&self, input: Params) -> Option<Target> {
        if input.keywords.is_none() {
            return None;
        }
        let keywords = input.keywords.as_ref().unwrap();

        // Players in the room by name
        if input.players_by_name {
            let full_name = keywords.join(" ");
            let target = self.resolve_player(full_name.as_str(), input.room);
            if target.is_some() {
                return target;
            }
        }

        // objects in by keyword
        if input.objects_in_room {
            let target = self.resolve_object(keywords.as_slice(), input.room);
            if target.is_some() {
                return target;
            }
        }

        if input.objects_carried {
            let target = self.resolve_object(keywords.as_slice(), input.actor);
            if target.is_some() {
                return target;
            }
        }

        None
    }
}
