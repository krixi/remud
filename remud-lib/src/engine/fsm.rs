use crate::engine::negotiate_login::{ClientData, Params};

use anyhow::bail;
use std::{collections::HashMap, fmt::Debug, hash::Hash};

pub trait Transition: Debug {}
pub trait StateId: Debug + Copy + Clone + Eq + PartialEq + Hash {}

#[async_trait::async_trait]
pub trait State<T, SID>: Send + Sync
where
    T: Transition,
    SID: StateId,
{
    fn id(&self) -> SID;

    fn output_state(&self, next: &T) -> Option<SID>;

    fn next_state(&self, tx: &T) -> Option<SID> {
        return match self.output_state(tx) {
            Some(state_id) => Some(state_id),
            None => {
                tracing::warn!(
                    "{:?} -> {:?} -> ? (unable to find output state)",
                    self.id(),
                    tx
                );
                None
            }
        };
    }

    // whether or not to call the state update again.
    fn keep_going(&self) -> bool {
        false
    }

    #[allow(unused_variables)]
    async fn on_enter<'p>(&mut self, data: &mut ClientData, params: &'p mut Params) {}

    #[allow(unused_variables)]
    async fn decide<'p>(&mut self, data: &mut ClientData, params: &'p mut Params) -> Option<T> {
        None
    }

    #[allow(unused_variables)]
    async fn act<'p>(&mut self, data: &mut ClientData, params: &'p mut Params) {}

    #[allow(unused_variables)]
    async fn on_exit<'p>(&mut self, data: &mut ClientData, params: &'p mut Params) {}
}

pub struct FsmBuilder<T, SID>
where
    T: Transition,
    SID: StateId,
{
    states: Vec<(SID, Box<dyn State<T, SID>>)>,
}

impl<T, SID> FsmBuilder<T, SID>
where
    T: Transition,
    SID: StateId,
{
    pub fn new() -> Self {
        FsmBuilder { states: Vec::new() }
    }

    pub fn build(self) -> anyhow::Result<Fsm<T, SID>> {
        let mut states = HashMap::new();
        let mut first = None;
        for (id, state) in self.states {
            if first == None {
                first = Some(id)
            }
            states.insert(id, state);
        }

        if let Some(current) = first {
            Ok(Fsm { states, current })
        } else {
            bail!("No states found for client fsm")
        }
    }

    pub fn with_state(mut self, state: Box<dyn State<T, SID>>) -> Self {
        self.states.push((state.id(), state));
        self
    }
}

pub struct Fsm<T, SID>
where
    T: Transition,
    SID: StateId,
{
    states: HashMap<SID, Box<dyn State<T, SID>>>,
    current: SID,
}

impl<T, SID> Fsm<T, SID>
where
    T: Transition,
    SID: StateId,
{
    pub async fn on_update<'a>(
        &mut self,
        tx: Option<T>,
        data: &mut ClientData,
        params: &'a mut Params<'a>,
    ) -> bool {
        // delegate to current state -
        let current_state = self.states.get_mut(&self.current).unwrap();

        // check if called with a direct transition or not, if not - decide
        // gets the new current state after any transitions occur
        let current_state = if let Some((next, tx)) = match tx {
            Some(tx) => Some((current_state.next_state(&tx), tx)),
            None => current_state
                .decide(data, params)
                .await
                .map(|tx| (current_state.next_state(&tx), tx)),
        } {
            if let Some(next) = next {
                tracing::info!("{:?} - {:?} -> {:?}", current_state.id(), tx, next);
            }

            // update states if needed
            match next {
                Some(state) => {
                    // exit outgoing state.
                    current_state.on_exit(data, params).await;
                    // assert that the new state exists
                    let next_state = self
                        .states
                        .get_mut(&state)
                        .expect("missing state implementation for");
                    // update the current state reference
                    self.current = state;
                    // enter the new state
                    next_state.on_enter(data, params).await;
                    next_state
                }
                None => current_state,
            }
        } else {
            current_state
        };

        // finally: let the new current state act.
        current_state.act(data, params).await;

        // the new current state knows best if it should be called again right away.
        current_state.keep_going()
    }
}
