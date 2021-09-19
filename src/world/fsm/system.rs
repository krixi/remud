use crate::world::fsm::StateMachine;
use bevy_ecs::prelude::*;
use itertools::Itertools;

pub fn state_machine_system(world: &mut World) {
    // Get all entities with a state machine component, call on_update on them.
    let fsm_entities = world
        .query_filtered::<Entity, With<StateMachine>>()
        .iter(world)
        .collect_vec();

    for entity in fsm_entities {
        // we have to do this little dance because mutability reasons
        let mut fsm = world.entity_mut(entity).remove::<StateMachine>().unwrap();
        fsm.on_update(entity, world);
        world.entity_mut(entity).insert(fsm);
    }
}
