use crate::world::fsm::StateMachines;
use bevy_ecs::prelude::*;
use itertools::Itertools;

pub fn state_machine_system(world: &mut World) {
    // Get all entities with a state machine component, call on_update on them.
    let fsm_entities = world
        .query_filtered::<Entity, With<StateMachines>>()
        .iter(world)
        .collect_vec();

    for entity in fsm_entities {
        // we have to do this little dance because mutability reasons
        if let Some(mut fsm) = world.get_mut::<StateMachines>(entity).unwrap().pop() {
            fsm.on_update(entity, world);
            world.get_mut::<StateMachines>(entity).unwrap().push(fsm);
        }
    }
}
