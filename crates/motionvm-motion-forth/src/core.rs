//! What both interpreters keep that has nothing to do with either's address
//! model: the step budget, the trace, the counters, the generator, and how
//! deep a nested call is.
//!
//! **This is deliberately small, and what is *not* here is the point.** The
//! two machines look alike from a distance and are not: the 32-bit one keeps a
//! loop's limit in a frame of its own beside the return stack, the 16-bit one
//! keeps limit and index *on* the return stack — measured, at `LL.EXE`
//! `0af7:05d5` and `ENVIRO.EXE` `12c8:01b8`, and proved from the other side by
//! Victor Loomes' `STOPLOOP` (module 605), which walks out of a loop early by
//! rewriting those very cells through `R>` and `>R`. A machine that kept the
//! limit somewhere of its own turned that into a loop that never ends. Their
//! return stacks hold different things, their cells are different widths,
//! their addresses are different shapes, and `EXECUTE` takes an address on one
//! and a word id on the other.
//!
//! So the primitives stay two files. What is genuinely one thing is here, and
//! it is six fields: a machine that ran away with itself, a trace nobody is
//! listening to, a random number, and a depth. None of them is about the
//! original — they are this rebuild's own bookkeeping, which is exactly why
//! they can be shared where the measured behavior cannot.

use crate::{Counters, Error, Result};

/// The bookkeeping both machines do the same way.
#[derive(Debug)]
pub(crate) struct Core {
    /// Cells executed since the last [`Core::restart`], against `step_limit`.
    steps: u64,
    /// What the machine has run since it was built. Cumulative, and reset by
    /// nothing — see [`Counters`], which says why it is kept beside `steps`.
    counters: Counters,
    /// Guards against a runaway program; generous but finite.
    step_limit: u64,
    /// When set, every executed word is appended here.
    trace: Option<Vec<String>>,
    /// Deterministic source for `RANDOM`: a seeded LCG, never the clock or
    /// the operating system's entropy.
    ///
    /// The engine reads no wall clock anywhere, and this is the other half of
    /// that: a given input state has to render a given frame, every time, or
    /// comparing two renderings of a scene across a change proves nothing —
    /// which is how the drawing code in this project is verified. A player's
    /// run is seeded from the platform instead; see [`Core::seed`].
    rng: u32,
    /// How deep inside a nested call the machine is. Non-zero means pausing is
    /// suppressed.
    nested: u32,
}

impl Default for Core {
    fn default() -> Self {
        Self {
            steps: 0,
            counters: Counters::default(),
            step_limit: 5_000_000,
            trace: None,
            rng: 0x1234_5678,
            nested: 0,
        }
    }
}

impl Core {
    /// Counts one executed cell, and stops a program that will not.
    ///
    /// The two counters move together and mean different things: `steps` is
    /// the budget for *this* execution and starts over at every
    /// [`Core::restart`], `counters.cells` is what the machine has done since
    /// it was built and starts over never.
    pub(crate) fn tick(&mut self) -> Result<()> {
        self.steps += 1;
        self.counters.cells += 1;
        if self.steps > self.step_limit {
            return Err(Error::StepLimit(self.step_limit));
        }
        Ok(())
    }

    /// Counts one word handed to the host.
    pub(crate) fn host_word(&mut self) {
        self.counters.host_words += 1;
    }

    /// What the machine has run since it was built.
    pub(crate) fn counters(&self) -> Counters {
        self.counters
    }

    /// The step budget, for the two places that save and restore it.
    pub(crate) fn steps(&self) -> u64 {
        self.steps
    }

    /// Puts a saved budget back — [`crate::Machine::unpark`], and the return
    /// from a nested call.
    pub(crate) fn set_steps(&mut self, steps: u64) {
        self.steps = steps;
    }

    /// Begins a fresh budget: a new execution is not the old one's remainder.
    pub(crate) fn restart(&mut self) {
        self.steps = 0;
    }

    /// Raises or lowers the runaway guard.
    pub(crate) fn set_step_limit(&mut self, steps: u64) {
        self.step_limit = steps;
    }

    /// Enters a nested call, where pausing is suppressed.
    pub(crate) fn enter_nested(&mut self) {
        self.nested += 1;
    }

    /// Leaves one.
    pub(crate) fn leave_nested(&mut self) {
        self.nested = self.nested.saturating_sub(1);
    }

    /// Whether the machine is inside a nested call, and so may not pause.
    pub(crate) fn nested(&self) -> bool {
        self.nested != 0
    }

    /// Starts recording every executed word.
    pub(crate) fn start_trace(&mut self) {
        self.trace.get_or_insert_with(Vec::new);
    }

    /// What has been recorded, or `None` when nothing is.
    pub(crate) fn trace(&self) -> Option<&[String]> {
        self.trace.as_deref()
    }

    /// Whether anyone is listening.
    ///
    /// Asked before a line is built, because building one costs a lookup and
    /// an allocation and a trace is off in every normal run.
    pub(crate) fn tracing(&self) -> bool {
        self.trace.is_some()
    }

    /// Appends a line, if anyone is listening.
    pub(crate) fn push_trace(&mut self, line: String) {
        if let Some(t) = self.trace.as_mut() {
            t.push(line);
        }
    }

    /// The last line recorded, for a report that wants to say what came
    /// before it.
    pub(crate) fn last_trace(&self) -> Option<&str> {
        self.trace.as_ref()?.last().map(String::as_str)
    }

    /// Reseeds `RANDOM`'s generator; see [`crate::Machine::seed`].
    ///
    /// The state is 32 bits wide, so the low half of the seed is what reaches
    /// it and the rest is dropped rather than folded in: a caller with real
    /// entropy has it in the low bits, and a fold would make two seeds that
    /// look different behave the same in a way nothing would report.
    pub(crate) fn seed(&mut self, seed: u64) {
        let [a, b, c, d, ..] = seed.to_le_bytes();
        self.rng = u32::from_le_bytes([a, b, c, d]);
    }

    /// The next number `RANDOM` draws.
    ///
    /// Any decent generator will do; what matters is that it is repeatable.
    /// The original's is not a generator at all but a hash of its interrupt
    /// clock — read, and deliberately not reproduced, since this machine
    /// reads no clock; this one is a recorded departure.
    pub(crate) fn next_rng(&mut self) -> u32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.rng
    }
}
