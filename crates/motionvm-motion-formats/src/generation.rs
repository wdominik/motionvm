//! Which of the engine's two generations a game's files belong to.
//!
//! **This selects formats and never behavior.** A generation says which
//! container the resources are in, how a script module is laid out, which
//! savegame layout is written and which music stack plays — facts about the
//! *files*, settled by which engine binary produced them. What the engine
//! *does* differently is never chosen here: that is a capability, named for
//! the behavior and carrying the address it was measured at, and three of
//! those already vary within a generation. A rule the tree keeps because the
//! next MOTION build is unknown, and "is it the 16-bit one" is exactly the
//! question that stops being answerable when a third arrives.
//!
//! It lives in this crate because this is the lowest one that knows both: the
//! readers for `NNN.RSC` and for `DATA.-n-` are both here, and everything
//! above needs the same word for the same thing. One type and not one per
//! crate: a roster's, a savegame layout's and a tool's own enum for the same
//! question would agree by convention only, and a convention is what a further
//! build breaks without anything noticing.

/// One of the engine's two generations, told by the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Generation {
    /// The 32-bit engine: `ENGINE.EXE`, 32-bit cells, `NNN.RSC` containers,
    /// HMI music.
    Motion32,
    /// The 16-bit engine: `ENVIRO.EXE` and its older builds, 16-bit cells, a
    /// `DATA.-n-` container, PSM 2 music.
    Motion16,
}

impl Generation {
    /// Which generation `dir` holds a game of, from file names alone.
    ///
    /// Answers *which machine wrote these files*, which is a different
    /// question from *which game a player can play* — the readers reach more
    /// games than the player does, Checker 2000 among them — so this stays
    /// here and a roster's own probe stays with the roster.
    pub fn of(dir: &std::path::Path) -> Option<Self> {
        if crate::find_ci(dir, "DATA.-1-").is_some() {
            return Some(Self::Motion16);
        }
        crate::m32::has_container(dir).then_some(Self::Motion32)
    }
}
