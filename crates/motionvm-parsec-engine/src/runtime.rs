//! What a host drives: one opened game, in the shape the window-to-game
//! contract (`motionvm-playable`) asks of a game.
//!
//! The trait is the family's own and the contract is not, so the two stay
//! apart: a host outside the family reaches a game through
//! `motionvm_playable::Playable`, which the family's front door implements
//! by handing every call to this. Method for method the same, so that the
//! front door does nothing but forward — and errors are this crate's
//! [`Error`], whose `Display` is the player's sentence.
//!
//! [`crate::Game`] is the implementation: the game a directory holds,
//! behind the one face.

use std::path::Path;
use std::time::Duration;

use motionvm_playable::{Button, Diagnostic, Frame, KeyPress, LocationRequest, PixelAspect, Size};

use crate::error::Result;
use crate::music_sink::MusicSink;

#[cfg(doc)]
use crate::error::Error;

/// One opened game, as a host drives it. See the module header.
///
/// `Send`, because a host may open a game on one thread and step it on
/// another; a runtime holding something tied to its thread would rule that
/// out.
pub trait Runtime: Send {
    /// The game's full name, as a window shows it.
    fn name(&self) -> &str;
    /// The size of the picture [`Runtime::frame`] answers with.
    fn display_size(&self) -> Size;
    /// The shape of one of that picture's pixels on the game's own monitor.
    fn pixel_aspect(&self) -> PixelAspect;
    /// What the game would take from the machine's clock, before
    /// [`Runtime::start`] and not again. Without it every run is the same.
    fn seed(&mut self, seed: u64);
    /// Begins play, after [`Runtime::set_saves`], once.
    fn start(&mut self) -> Result<()>;
    /// One step of the game — one pass of the original's frame loop.
    fn step(&mut self) -> Result<()>;
    /// The pointer, in the game's own coordinates, delivered as it moves.
    fn pointer(&mut self, x: i32, y: i32);
    /// One button transition, a press or a release; read in the next step.
    fn button(&mut self, which: Button, down: bool);
    /// One key press; read in a later step.
    fn key_down(&mut self, press: &KeyPress);
    /// One key release. A key the program's keyboard hook reads is let go
    /// here; the default body drops it.
    fn key_up(&mut self, press: &KeyPress) {
        let _ = press;
    }
    /// The frame to show, pointer and all, with the palette its indices
    /// mean.
    fn frame(&mut self) -> Frame<'_>;
    /// How long the picture the last step left should stand on screen.
    fn frame_duration(&self) -> Option<Duration>;
    /// Where the music goes — installed by the front door before
    /// [`Runtime::start`].
    fn set_music(&mut self, sink: Box<dyn MusicSink>);
    /// The bytes of the music driver the game installs, as the game ships
    /// it — the file the sink plays through — or why sound is off.
    fn music_driver(&self) -> Result<Vec<u8>>;
    /// The bytes of the digital driver the game installs beside it, as the
    /// game ships it — the file its effects and a song's digital voices play
    /// through — or `None` for a game without one.
    fn dac_driver(&self) -> Result<Option<Vec<u8>>>;
    /// Points saving and loading at a directory; the game keeps its files
    /// in a directory of its own under it, named for its slug.
    fn set_saves(&mut self, dir: &Path) -> Result<()>;
    /// Where saving and loading go, or `None` while there is nowhere.
    fn saves(&self) -> Option<&Path>;
    /// What the run has to report about itself.
    fn diagnostics(&self) -> Vec<Diagnostic>;
    /// Whether the game has run to its end: the player quit.
    fn finished(&self) -> bool;
    /// Asks the game to go to a location, and optionally through which
    /// arrival point, instead of where it would.
    fn request_location(&mut self, at: LocationRequest) -> Result<()>;
    /// The location the game is in or about to enter, if it has locations.
    fn start_location(&self) -> Option<i32>;
    /// What that location calls itself, when the game keeps a name for it.
    fn location_name(&self) -> Option<&str> {
        None
    }
}
