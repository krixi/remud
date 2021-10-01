use crate::{
    text::Tokenizer,
    world::{
        action::{into_action, Action},
        types::{player::Messages, Attributes, Health},
    },
};
use bevy_app::EventReader;
use bevy_ecs::prelude::*;

pub fn parse_stats(player: Entity, mut _tokenizer: Tokenizer) -> Result<Action, String> {
    Ok(Action::from(Stats { actor: player }))
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Stats {
    pub actor: Entity,
}

into_action!(Stats);

#[tracing::instrument(name = "stats system", skip_all)]
pub fn stats_system(
    mut action_reader: EventReader<Action>,
    mut stats_query: Query<(&Health, &Attributes)>,
    mut messages_query: Query<&mut Messages>,
) {
    for action in action_reader.iter() {
        if let Action::Stats(Stats { actor }) = action {
            if let Ok((health, stats)) = stats_query.get_mut(*actor) {
                if let Ok(mut messages) = messages_query.get_mut(*actor) {
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
