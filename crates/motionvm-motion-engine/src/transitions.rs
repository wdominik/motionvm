//! What is playing over the picture: the two curtain kinds, a scrolling
//! view, and the log of what has run.
//!
//! One queue per effect rather than one of both, because the two generations
//! have one each and neither game ever has both up: the 32-bit engine draws a
//! band curtain and the 16-bit one a box wipe. A transition holds the
//! interpreter where the original's handler held it — inside the word, until
//! the last band — which is why `Engine::in_transition` is what
//! `Host::wants_pause` asks.

use crate::{Curtain, Fade, Scroll, Wipe};

/// The effects in flight, and the record of the ones that have run.
#[derive(Debug, Default)]
pub(crate) struct Transitions {
    /// Transitions waiting to play, oldest first.
    ///
    /// A queue rather than a single one because a phase of the intro calls
    /// `FADEOUT`, swaps what is on the screen and calls `FADEIN` all within one
    /// invocation — the original blocks inside each handler, so both run in
    /// full before the phase is over. `DO_INVSEL` does it three deep: its
    /// documents branch fades the bar in, the picture out and the picture in
    /// again (module 4, 0x01c44–0x01ca8), and it can, because it runs as a
    /// descriptor callback through `m32::Vm::call_nested`, where the interpreter
    /// does not pause.
    pub(crate) curtains: std::collections::VecDeque<Curtain>,

    /// The 16-bit engine's transitions, queued the same way — box wipes,
    /// not band curtains; see [`Wipe`].
    pub(crate) wipes: std::collections::VecDeque<Wipe>,

    /// A `->SCRX`/`->SCRY` scroll in flight — the 16-bit engine's blocking
    /// window slide, run here as a transition: one step a frame, the
    /// interpreter held, the way the fades are.
    pub(crate) scroll: Option<Scroll>,

    /// The 32-bit `->SCRX`/`->SCRY` in flight — a strip of the old and the
    /// new view blitted a window at a time; see [`crate::Slide`].
    pub(crate) slide: Option<crate::Slide>,

    /// Every fade that has been started, in order — see [`Fade`].
    pub(crate) fades: Vec<Fade>,
}
