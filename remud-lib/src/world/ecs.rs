use std::sync::{Arc, RwLock};

use bevy_app::Events;
use bevy_core::Time;
use bevy_ecs::{prelude::*, schedule::SystemDescriptor};

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
    world: SharedWorld,
    pre_event: Schedule,
    main: Schedule,
    post_event: Schedule,
}

impl Ecs {
    pub fn new(world: World) -> Self {
        let mut pre_event = Schedule::default();
        pre_event.add_stage(Phase::First, SystemStage::parallel());
        pre_event.add_stage_after(Phase::First, Phase::Update, SystemStage::parallel());

        let mut main = Schedule::default();
        main.add_stage(Phase::First, SystemStage::parallel());
        main.add_stage_after(Phase::First, Phase::Update, SystemStage::parallel());

        let mut post_event = Schedule::default();
        post_event.add_stage(Phase::Update, SystemStage::parallel());

        let world = Arc::new(RwLock::new(world));

        Ecs {
            world,
            pre_event,
            main,
            post_event,
        }
    }

    pub fn world(&self) -> SharedWorld {
        self.world.clone()
    }

    pub fn run(&mut self, step: Step) {
        match step {
            Step::PreEvent => self.pre_event.run_once(&mut *self.world.write().unwrap()),
            Step::Main => self.main.run_once(&mut *self.world.write().unwrap()),
            Step::PostEvent => self.post_event.run_once(&mut *self.world.write().unwrap()),
        }
    }

    pub fn init_resource<T: Default + Send + Sync + 'static>(&mut self) -> &mut Self {
        self.world.write().unwrap().insert_resource(T::default());
        self
    }

    pub fn add_event<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        self.world
            .write()
            .unwrap()
            .insert_resource(Events::<T>::default());
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

    pub fn register(&mut self, plugin: impl Plugin) -> &mut Self {
        plugin.build(self);
        self
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