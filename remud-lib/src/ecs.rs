use std::sync::{Arc, RwLock};

use bevy_app::Events;
use bevy_core::Time;
use bevy_ecs::{prelude::*, schedule::SystemDescriptor};

use crate::world::{
    scripting::time::Timers,
    types::{object::Container, room::Room, Location},
};

pub type SharedWorld = Arc<RwLock<World>>;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum Step {
    PreEvent,
    Main,
    PostEvent,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, StageLabel)]
pub enum Phase {
    First,
    Update,
}

pub struct Ecs {
    world: Option<World>,
    pre_event: Schedule,
    main: Schedule,
    post_event: Schedule,
}

impl Ecs {
    pub fn new() -> Self {
        let world = World::new();

        let mut pre_event = Schedule::default();
        pre_event.add_stage(Phase::First, SystemStage::parallel());
        pre_event.add_stage_after(Phase::First, Phase::Update, SystemStage::parallel());

        let mut main = Schedule::default();
        main.add_stage(Phase::First, SystemStage::parallel());
        main.add_stage_after(Phase::First, Phase::Update, SystemStage::parallel());

        let mut post_event = Schedule::default();
        post_event.add_stage(Phase::Update, SystemStage::parallel());

        Ecs {
            world: Some(world),
            pre_event,
            main,
            post_event,
        }
    }

    pub fn world(&self) -> &World {
        self.world.as_ref().unwrap()
    }

    pub fn world_mut(&mut self) -> &mut World {
        self.world.as_mut().unwrap()
    }

    pub fn run(&mut self, step: Step) {
        let world = self.world.as_mut().unwrap();
        match step {
            Step::PreEvent => self.pre_event.run_once(world),
            Step::Main => self.main.run_once(world),
            Step::PostEvent => self.post_event.run_once(world),
        }
    }

    pub fn init_resource<T: Default + Send + Sync + 'static>(&mut self) -> &mut Self {
        self.world_mut().insert_resource(T::default());
        self
    }

    pub fn add_event<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        self.world_mut().insert_resource(Events::<T>::default());
        self.pre_event
            .add_system_to_stage(Phase::First, Events::<T>::update_system.system());
        self
    }

    pub fn add_system(
        &mut self,
        step: Step,
        phase: Phase,
        system: impl Into<SystemDescriptor>,
    ) -> &mut Self {
        match step {
            Step::PreEvent => self.pre_event.add_system_to_stage(phase, system),
            Step::Main => self.main.add_system_to_stage(phase, system),
            Step::PostEvent => self.post_event.add_system_to_stage(phase, system),
        };

        self
    }

    pub async fn register(&mut self, plugin: impl Plugin) -> &mut Self {
        plugin.build(self);
        self
    }

    pub fn with_shared_world<F: FnMut(Arc<RwLock<World>>)>(&mut self, mut f: F) {
        let world = self.world.take().unwrap();
        let shared_world = Arc::new(RwLock::new(world));

        f(shared_world.clone());

        let world = RwLock::into_inner(Arc::try_unwrap(shared_world).unwrap()).unwrap();
        self.world = Some(world);
    }
}

pub trait Plugin {
    fn build(&self, ecs: &mut Ecs);
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, SystemLabel)]
pub enum CoreSystem {
    Time,
}

#[derive(Default)]
pub struct CorePlugin {}

impl Plugin for CorePlugin {
    fn build(&self, ecs: &mut Ecs) {
        ecs.init_resource::<Time>().add_system(
            Step::PreEvent,
            Phase::First,
            time_system.system().label(CoreSystem::Time),
        );
    }
}

fn time_system(mut time: ResMut<Time>) {
    time.update()
}

pub trait WorldExt {
    fn with_timers<F: FnMut(&mut Timers)>(&mut self, entity: Entity, f: F);
    fn location_of(&self, entity: Entity) -> Entity;
}

impl WorldExt for World {
    fn with_timers<F: FnMut(&mut Timers)>(&mut self, entity: Entity, mut f: F) {
        if let Some(mut timers) = self.get_mut::<Timers>(entity) {
            f(&mut *timers);
        } else {
            let mut timers = Timers::default();
            f(&mut timers);
            self.entity_mut(entity).insert(timers);
        }
    }

    fn location_of(&self, entity: Entity) -> Entity {
        if let Some(location) = self.get::<Location>(entity) {
            location.room()
        } else if self.entity(entity).contains::<Room>() {
            entity
        } else {
            let mut contained = entity;

            while let Some(next_container) = self.get::<Container>(contained) {
                contained = next_container.entity();
            }

            if let Some(location) = self.get::<Location>(contained) {
                location.room()
            } else {
                panic!("target entity {:?} not located within a room", entity)
            }
        }
    }
}
