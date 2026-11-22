//! The game's program, in a module of its own.
//!
//! PARSEC is not an interpreter: *Dunkle Schatten* is compiled C,
//! `XP00.EXE` and `XP01.EXE`, linked against a library. Rooms, verbs,
//! puzzles and the room handlers are that C, and only the reading of the
//! game's own `XP00`/`XP01` says what it does. So the game has a module
//! here holding its address map, its room handlers and its puzzle logic
//! (`ds1`, the game's slug), and the parts of the program that are the
//! library it is linked against rather than the game's logic belong to
//! the `parsec1994` generation, the one PARSEC generation this engine
//! plays. Its runtime — the mode machine, the animator, the tasks, the
//! dialogue — is the module `parsec1994`; `xp1994` and `palette1994` read
//! its location dispatch, its tables and its panel colors out of the
//! program; and the walk, the movers, the subjects, the caption, the
//! strip's widgets and the generator sit in modules of their own beside
//! them. Each cites its addresses in `XP01.EXE`.
//!
//! What sits below this level: the file formats in [`motionvm_parsec_formats`],
//! the framebuffer in [`motionvm_render`], what the engine draws with in
//! `crate::draw` — the scaler among it — and the image-as-data reader the
//! program's tables come through.

use std::path::Path;
use std::time::Duration;

use motionvm_parsec_formats::Title;
use motionvm_render::{Framebuffer, Palette};

pub(crate) mod caption;
pub(crate) mod ds1;
pub(crate) mod mover;
pub(crate) mod palette1994;
pub(crate) mod panel;
pub(crate) mod parsec1994;
pub(crate) mod rng;
pub(crate) mod subject;
pub(crate) mod walker;
pub(crate) mod xp1994;

use crate::error::Result;

/// How `XP01` begins when the title program is skipped.
///
/// *Dunkle Schatten*'s title program `XP00` asks before `XP01` runs: a new
/// game deletes `SAVEGAME.SAV` (XP00 `0x625`), continuing leaves it for
/// `XP01` to load at its start. Play begins in that program and its menu
/// makes the choice; this is the same choice made by a caller that starts
/// past it — a test, or a headless run that wants the first location
/// without playing through the title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// A fresh game; a saved one on disk is not read.
    New,
    /// The saved game on disk, when there is one; otherwise a fresh game.
    Continue,
}

/// What the game's program has to be able to do.
///
/// `Game` holds it and is the [`crate::Runtime`] a host drives; what the
/// machine around the program does — the keyboard's buffer, the mouse's
/// latch, the save directory's name — is `Game`'s, and what the program
/// does with a key, a click or a tick is this.
///
/// Little is provided here, and each with a stated reason. A default would
/// be a claim about a program nobody has read, which is the mistake this
/// whole layout exists to prevent.
///
/// `Send`, because a host may step the game on a thread other than the one
/// that opened it, and a runtime that holds something tied to its thread
/// (an `Rc`, say) would rule that out. The bound is here so that a runtime
/// which breaks it fails to compile in its own `impl`.
pub(crate) trait Runtime: Send {
    /// Which title this is.
    fn title(&self) -> Title;

    /// The palette the current picture is indexed by.
    fn palette(&self) -> &Palette;

    /// Compose the current 320×200 picture.
    fn render(&self) -> Framebuffer;

    /// The pointer, in game coordinates.
    fn mouse_move(&mut self, x: i32, y: i32);

    /// A mouse button going down, as the frame loop reads it. `left` is
    /// button 0.
    fn click(&mut self, left: bool);

    /// A keystroke, as the frame loop takes it from the keyboard: the
    /// character, or `100h` plus the scan code (see `crate::keys`).
    fn key(&mut self, code: i32);

    /// One frame. `false` when the player has chosen to quit.
    fn tick(&mut self) -> bool;

    /// Jump to location `n`, for `--loc`. Unknown numbers are ignored
    /// rather than failing: a typo should not abort a running game.
    fn set_location(&mut self, n: i32);

    /// The same, arriving on a named entry id rather than the one the
    /// title's own tables suggest.
    fn set_location_via(&mut self, n: i32, entry: i16);

    /// What the title would take from the machine's clock, called before
    /// [`Runtime::start`] and not again.
    ///
    /// A runtime reaches for no clock of its own: without this call every
    /// run is the same run, which is what lets a test step a scene twice and
    /// compare. A host that wants a different game every launch passes a
    /// seed from its clock; one that wants the same one passes nothing.
    /// What a title makes of it is the title's to say in its `impl`.
    fn seed(&mut self, seed: u64);

    /// Where savegames go. Refused when `dir` resolves inside the game
    /// directory: the install is treated as read-only.
    fn set_saves(&mut self, dir: &Path) -> Result<()>;

    /// Begins play, after [`Runtime::set_saves`]. A title whose program
    /// reads nothing at its start has nothing to do here and says so in
    /// its `impl`; one that does reads its files now, once.
    fn start(&mut self, how: Start) -> Result<()>;

    /// Begins play past the title program, as its menu's choice `how`
    /// would leave it. A title whose title program is not apart from its
    /// game — or not read as one — has nothing to skip, and starts as
    /// [`Runtime::start`] does, which is the default.
    fn start_past_title(&mut self, how: Start) -> Result<()> {
        self.start(how)
    }

    /// The location the player is in, when the title has locations.
    fn location(&self) -> Option<i32>;

    /// What that location calls itself, when the title has a name for it.
    ///
    /// None is invented: a location that has no name answers `None` and a
    /// caller prints the number.
    fn location_name(&self) -> Option<&str> {
        None
    }

    /// How long a frame should last.
    ///
    /// The rate is the program's own, read from its code.
    fn frame_duration(&self) -> Duration;

    /// Where the music goes. A title that plays no music drops the sink,
    /// which is the default.
    fn set_music(&mut self, sink: Box<dyn crate::MusicSink>) {
        drop(sink);
    }

    /// The bytes of the music driver the title installs, for the sink to
    /// play through — or why there is none.
    fn music_driver(&self) -> Result<Vec<u8>> {
        Err(crate::error::Error::Unexpected(format!(
            "{} plays no music",
            self.title().name()
        )))
    }

    /// What the run noticed about itself and could not act on. The
    /// default collects nothing.
    fn diagnostics(&self) -> Vec<motionvm_playable::Diagnostic> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    /// Compiles only for a type that may move to another thread.
    fn send<T: Send>() {}

    /// Every runtime, and the game that boxes one, is `Send`. A compile-time
    /// check: the test body is the assertion, and running it proves nothing
    /// further.
    #[test]
    fn every_runtime_and_the_game_may_move_to_another_thread() {
        send::<super::ds1::DunkleSchatten>();
        send::<Box<dyn super::Runtime>>();
        send::<crate::Game>();
    }
}
