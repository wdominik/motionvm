//! Dunkle Schatten 2: what the engine has to know about this one game to
//! open it and run it — the files it ships, the words its bootstrap names,
//! the script variables its input goes through.
//!
//! Everything here is the game's, not the engine's: module 4's `START`,
//! module 3's `STARTUP`, module 5's `INCLLOC`, module 2's `_STARTLOC`,
//! `_NEXTLOC`, `_LTHANDLER`, `_MLK`, `_MRK`, `_AKTKEY`, `_LOCTASK` and
//! `_LOCTASKPHA` are names the game's own compiler gave its words, and
//! another MOTION game has others.
//!
//! Everything the game shares with any other MOTION 32-bit title — the
//! resource bank, the kernel table out of `ENGINE.EXE`, loading the script
//! modules, and the check that the container is this game's — is in
//! [`super::motion32`], as the 16-bit games' shared half is in
//! [`super::motion16`].

use std::path::Path;

use motionvm_motion_forth::Address;
use motionvm_motion_forth::m32::Vm;

use crate::game::{Game, Hooks, LocationScheme};
use crate::titles::{Title, motion32};
use crate::{Error, Result};

/// What a directory must hold before [`Game::open`] can do anything with it.
///
/// Only three entries, and each was established by taking it away and watching
/// what broke, not by reading the loader:
///
/// - **A resource container.** Everything the game *is* lives in the `NNN.RSC`
///   files — the script modules, the artwork, the texts, the music, the fonts,
///   the palettes. This one ships three; how many a game has is the game's
///   business, and `motion32::has_container` looks for the pattern.
/// - **`ENGINE.EXE`.** The 356-word kernel table is read out of the LE image.
///   Without it the VM has no primitives to bind the bytecode's ordinals to.
/// - **`000.FRT`.** Character to glyph. Text rendering leaves early without it,
///   so a game that has everything else draws its pictures and not one word.
///
/// `000.FNT` is deliberately *not* here. It is read when present, but only as
/// the fallback for text that names no font of its own — and every string this
/// game draws names one, so removing it changes nothing on screen. The three
/// sound files are not here either: the frontend reports their absence and
/// plays on in silence.
const REQUIRED: &[(&str, &str)] = &[
    (
        "a resource container (NNN.RSC)",
        "scripts, artwork, texts, music, fonts",
    ),
    ("ENGINE.EXE", "the kernel word table"),
    (
        "000.FRT",
        "the font reference table, without which no text is drawn",
    ),
];

/// The words that make a MOTION 32-bit container *this* game.
///
/// Both are module 2 variables the game's own compiler named, and both are
/// read from here: `_STARTLOC` is the location a run begins at,
/// [`crate::Driven::start_location`], and `_NEXTLOC` is the one the stand-in
/// frame loop moves to. A container that does not define them is not a container
/// this code can drive, whatever else it holds.
pub(super) const SIGNATURE: &[&str] = &["_STARTLOC", "_NEXTLOC"];

/// Where it keeps the location it is in and the one it is going to.
///
/// One variable does both jobs here: `_STARTLOC` in module 2 is what a run
/// begins at and what the stand-in frame loop writes to move, so there is
/// nothing to fall back to and no value that means "none" — the variable
/// exists only once the container is open, and `None` before that is the
/// lookup failing rather than a sentinel.
pub(crate) const LOCATION: LocationScheme = LocationScheme {
    module: 2,
    next: "_STARTLOC",
    fallback: None,
    unset_below: None,
};

/// Which of the required files `dir` does not hold, as `(what, what for)`.
///
/// Empty means [`Game::open`] will get as far as parsing.
pub fn missing_data(dir: &Path) -> Vec<(&'static str, &'static str)> {
    motion32::missing_data(dir, REQUIRED)
}

/// Opens Dunkle Schatten 2 in `dir` — the free spelling of
/// [`Game::<Vm>::open`](Game::open), so the roster's five arms read alike.
pub fn open(dir: &Path) -> Result<Game<Vm>> {
    Game::<Vm>::open(dir)
}

impl Game<Vm> {
    /// Opens Dunkle Schatten 2 in `dir`.
    ///
    /// An associated function rather than a free one, unlike the 16-bit
    /// games': there is one 32-bit game today, so `Game::<Vm>::open` names its
    /// machine unambiguously, and it is the name the tests already use. The
    /// work is `motion32::open`'s; what this hands it is what makes the
    /// container this game's — `REQUIRED` and `SIGNATURE`.
    pub fn open(dir: &Path) -> Result<Self> {
        motion32::open(dir, Title::DunkleSchatten2, REQUIRED, SIGNATURE, LOCATION)
    }

    /// Begins the game the way it begins itself.
    ///
    /// `SYSTEM.RSC` holds two lines — `4 =>GET` and `START` — and `4:START`
    /// loads the modules it needs, runs `STARTUP`, initializes through
    /// `DS_INIT`, and then hands over with `0x42150 CTRL`. That address is
    /// `ICTRL`, the game's own per-frame controller.
    ///
    /// Entered as the bootstrap enters it, rather than by calling `STARTUP` and
    /// `INCLLOC` by hand. `START` is the word that decides what a run consists
    /// of, and it sits past the boundary a module parser stops at if it takes
    /// module memory to end at `0x50 + 16004` — so it is easy to conclude the
    /// word does not exist.
    pub fn start(&mut self) -> Result<()> {
        // The first of those two lines is the one no bytecode contains, so the
        // slot it takes has to be granted from here. Module 4 lands in slot 1,
        // and everything `START` loads follows behind it.
        self.engine.mark_resident(4);
        let addr = self.address(4, "START").ok_or_else(|| Error::NoWord {
            module: 4,
            name: "START".into(),
        })?;
        self.vm.start(addr);
        self.running = true;
        Ok(())
    }
    /// Runs `STARTUP` alone, for tests that want the state without the game.
    pub fn startup_only(&mut self) -> Result<()> {
        self.call(3, "STARTUP", &[])
    }
    /// Enters a location and lets the entry play out at once.
    ///
    /// Convenient where only the settled picture matters. Anything with a frame
    /// clock wants [`Self::begin_location`] instead, or the entry's own fade is
    /// consumed before a single frame reaches the screen.
    pub fn enter_location(&mut self, location: i32) -> Result<()> {
        self.call(5, "INCLLOC", &[location])
    }
    /// Starts entering a location without running it to the end.
    ///
    /// `INCLLOC` fades out, runs the location's macro, and fades back in — the
    /// second of those is how a location appears at all. Driving it frame by
    /// frame is what makes that visible.
    pub fn begin_location(&mut self, location: i32) -> Result<()> {
        let addr = self.address(5, "INCLLOC").ok_or_else(|| Error::NoWord {
            module: 5,
            name: "INCLLOC".into(),
        })?;
        self.vm.data.push(location);
        self.vm.start(addr);
        self.running = true;
        Ok(())
    }
    /// Hands the game this frame's buttons — the one-frame pulse the latch
    /// derived from the platform's transitions.
    ///
    /// One frame per press on purpose. The task manager advances a phase
    /// whenever it sees a button down, so reporting a held button on every
    /// frame would race through the whole intro in a fraction of a second.
    ///
    /// Both routes are fed, because the game uses both: `ICTRL` reads the
    /// pointer through the kernel words `MOUSEX`, `MOUSEY`, `MOUSELK` and
    /// `MOUSERK` and stores the result in these variables itself, while the
    /// location handlers read the variables.
    pub fn pointer_buttons(&mut self, left: bool, right: bool) -> Result<()> {
        self.set_var(2, "_MLK", left as i32)?;
        self.set_var(2, "_MRK", right as i32)?;
        self.engine.mouse.left = left as i32;
        self.engine.mouse.right = right as i32;
        Ok(())
    }

    /// Hands the game this frame's whole pointer — position and buttons in
    /// one call, the shape the tests use.
    pub fn pointer(&mut self, x: i32, y: i32, left: bool, right: bool) -> Result<()> {
        self.pointer_position(x, y);
        self.pointer_buttons(left, right)
    }

    /// Hands the game the key `?KEY` answers this frame, on both of its
    /// routes: `_AKTKEY` for the location handlers, the engine's record for
    /// the kernel word.
    pub fn deliver_key(&mut self, key: i32) -> Result<()> {
        self.set_var(2, "_AKTKEY", key)?;
        self.engine.key = key;
        Ok(())
    }

    /// Hands the game this frame's whole input: the pointer half and the key
    /// half in one call, which is the shape the tests feed raw `?KEY` codes
    /// through.
    pub fn set_input(&mut self, x: i32, y: i32, left: bool, right: bool, key: i32) -> Result<()> {
        self.pointer(x, y, left, right)?;
        self.deliver_key(key)
    }
    /// The task and phase the location is currently in — the intro's progress.
    pub fn task_phase(&self) -> (i32, i32) {
        (
            self.get_var(2, "_LOCTASK").unwrap_or(0),
            self.get_var(2, "_LOCTASKPHA").unwrap_or(0),
        )
    }
}

impl Hooks for Game<Vm> {
    /// The hand-built frame loop that stands in until `START` has handed
    /// `CTRL` a controller: act on `_NEXTLOC`, else run `_LTHANDLER`.
    ///
    /// It serves the path `startup_only` + `enter_location`, which is how a
    /// caller reaches a rendered scene without playing to it — the intro
    /// tests and the pixel-for-pixel comparison against the original both
    /// enter that way. It applies only until `START` sets a controller;
    /// removing it would take those entry points with it. Answers the word
    /// to run this frame, or `None` when the frame is spent.
    fn fallback_controller(&mut self) -> Result<Option<Address>> {
        if let Some(next) = self.get_var(2, "_NEXTLOC").filter(|&n| n != 0) {
            self.set_var(2, "_NEXTLOC", 0)?;
            self.begin_location(next)?;
            self.pump()?;
            return Ok(None);
        }
        Ok(self
            .get_var(2, "_LTHANDLER")
            .filter(|&h| h != 0)
            .map(|h| Address::new(h as u32 >> 16, h as u32 & 0xffff)))
    }
}
