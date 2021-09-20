use crate::{
    into_action,
    text::Tokenizer,
    world::{
        action::Action,
        types::{player::Messages, Attributes, Health},
    },
};
use bevy_app::EventReader;
use bevy_ecs::prelude::*;

pub fn parse_stats(player: Entity, mut _tokenizer: Tokenizer) -> Result<Action, String> {
    Ok(Action::from(Stats { entity: player }))
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub entity: Entity,
}

into_action!(Stats);

pub fn stats_system(
    mut action_reader: EventReader<Action>,
    mut stats_query: Query<(&Health, &Attributes)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Stats(Stats { entity }) = action {
            if let Ok((health, stats)) = stats_query.get_mut(*entity) {
                if let Ok(mut messages) = messages_query.get_mut(*entity) {
                    messages.queue(format!("Health {} / {}", health.current, health.max));
                    messages.queue(format!(
                        "Con {} / Dex {} / Int {} / Str {}",
                        stats.constitution, stats.dexterity, stats.intellect, stats.strength
                    ));
                }
            }
        }
    }
}
