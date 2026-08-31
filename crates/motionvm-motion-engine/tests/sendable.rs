//! The Send promises, held at compile time and with no game data: a game may
//! be opened on a worker thread, so every type that crosses a thread
//! boundary on that path has to be `Send` — the machines, the engine behind
//! them, and the boxed trait the front door drives.

use motionvm_motion_engine::{Driven, Game};
use motionvm_motion_forth::{m16, m32};

/// The claim itself: these compile or the promise is broken.
fn is_send<T: Send>() {}

#[test]
fn the_games_and_their_trait_object_are_send() {
    is_send::<Game<m16::Vm>>();
    is_send::<Game<m32::Vm>>();
    is_send::<Box<dyn Driven>>();
}
