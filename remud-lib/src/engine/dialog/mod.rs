// use std::{cmp::Eq, collections::HashMap, fmt::Debug, hash::Hash};
//
// // pub type DialogTransition = dyn Eq + PartialEq + Hash;
// pub trait DialogTransition: Eq + PartialEq + Hash {}
// pub trait DialogStateId: Eq + PartialEq + Hash {}
// //
// // impl DialogTransition for T where T: Eq + PartialEq + Hash {}
//
// pub trait DialogState<T: DialogTransition, S: DialogStateId, D: Clone>:
//     Debug + Send + Sync
// {
//     fn on_enter(&mut self, &mut _data: D) {}
//     fn decide(&mut self, _input: String, &mut _data: D) -> Option<T> {
//         None
//     }
//     fn act(&mut self, _input: String, &mut _data: D) {}
//     fn on_exit(&mut self, &mut _data: D) {}
//     fn output_state(&self, next: T) -> Option<S>;
// }
//
// // T: transition enum type
// // S: state ID enum type
// // D: data type that holds global state for this dialog.
// #[derive(Default)]
// pub struct DialogFSM<T: DialogTransition, S: DialogStateId, D: Clone> {
//     states: HashMap<S, Box<dyn DialogState<T, S, D>>>,
//     current: S,
//     data: D,
// }
//
// impl<T: DialogTransition, S: DialogStateId, D: Clone> DialogFSM<T, S, D> {
//     pub fn new() -> Self<T, S, D> {
//         DialogFSM {
//             states: Default::default(),
//             current: (),
//             data: (),
//         }
//     }
//
//     pub fn set_data(&mut self, data: D) {
//         self.data = data;
//     }
//
//     pub fn add_state(&mut self, id: S, state: Box<dyn DialogState<T, S, D>>) {
//         self.states.insert(id, state);
//     }
//
//     pub fn process_input(&mut self, input: String) {
//         // delegate to current state
//         // See if transition requested by calling decide
//         let current_state = self.states.get_mut(&self.current).unwrap();
//
//         let mut data = self.data.clone();
//         let next = current_state
//             .decide(input.clone(), data)
//             .and_then(|tx| current_state.output_state(tx));
//
//         let (current_state, data) = match next {
//             Some(state) => {
//                 // if there was a matching output state, do the transition.
//                 let mut data = data.clone();
//                 current_state.on_exit(data);
//                 let next_state = self.states.get_mut(&state).unwrap();
//                 self.current = state;
//                 let mut data = data.clone();
//                 next_state.on_enter(data);
//                 (next_state, data.clone())
//             }
//             None => (current_state, data.clone()),
//         };
//         let mut data = data.clone();
//         current_state.act(input, data);
//         // finally, store it back when done
//         self.data = data.clone();
//     }
// }
