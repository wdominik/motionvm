//! Setting the game up and stepping it, without a screen attached.
//!
//! [`Game`] is the whole machine assembled and ready to step: the DAT files
//! opened, the first room composed, the verb bar ready. What it does not own
//! is the clock or the window — a caller decides when a frame happens and
//! what becomes of the picture. That split is what lets the same setup drive
//! a window, a test and a headless run without any of the three knowing
//! about the others.

use std::path::{Path, PathBuf};
use std::time::Duration;

use motionvm_parsec_formats::{Title, detect, find_ci};
use motionvm_playable::{Button, Diagnostic, KeyPress, LocationRequest, PixelAspect, Size};
use motionvm_render::{Frame, Framebuffer};

use crate::draw::{DISPLAY_H, DISPLAY_W};
use crate::music_sink::MusicSink;
use crate::titles::ds1::DunkleSchatten;
use crate::titles::{Runtime as Program, Start};

use crate::error::{Error, Result};

/// What a directory must hold before [`Game::open`] can do anything with
/// it, by the names the title's build reads.
///
/// Established by taking files away and watching what broke, not by reading
/// the loader. The locations are slots of one file, and the cursor file
/// carries the font as well as the pointer.
fn required(title: Title) -> [(&'static str, &'static str); 4] {
    let names = match title {
        Title::DunkleSchatten => &crate::titles::ds1::build::BUILD.names,
    };
    [
        (names.rooms, "every location, as slots of one FGROUP"),
        (names.panel, "the verb strip under the playfield"),
        (names.pointers, "the font and the pointers"),
        (names.player, "the player's cels and their palette bank"),
    ]
}

/// Which of the required files `dir` does not hold, as `(what, what for)`.
///
/// Empty means [`Game::open`] will get as far as parsing. It is an existence
/// check, not a validity one — a truncated room container still fails
/// later.
pub fn missing_data(dir: &Path) -> Vec<(&'static str, &'static str)> {
    let Some(title) = detect::identify(dir) else {
        return vec![(
            "a PARSEC game",
            "XP00.EXE / XP01.EXE and RESOURCE/*.DAT (DSROOM.DAT)",
        )];
    };
    let res = detect::resource_dir(dir);
    required(title)
        .into_iter()
        .filter(|(name, _)| find_ci(&res, name).is_none())
        .collect()
}

/// The loaded game: its runtime, ready to step, behind the
/// [`crate::Runtime`] a host drives.
///
/// The boxed program runtime answers what the game does, and what the
/// machine around it does — the keyboard's buffer, the mouse's latch, the
/// screen a frame is lent from, the save directory's name — is kept here,
/// once.
pub struct Game {
    inner: Box<dyn Program>,
    /// The buttons, latched for the tick that reads them.
    buttons: crate::mouse::Latch,
    /// Keystrokes waiting for the tick that reads them.
    keys: crate::keys::Buffer,
    /// The screen the last frame composed, which it lends out.
    screen: Framebuffer,
    /// Whether [`crate::Runtime::start`] skips the title program, and how
    /// `XP01` then begins.
    past_title: Option<Start>,
    /// The title's own save directory, once one was accepted.
    saves: Option<PathBuf>,
    /// Whether the player has quit.
    over: bool,
}

impl std::fmt::Debug for Game {
    /// Where the game stands around its program; the program's own state is
    /// the frame it draws.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Game")
            .field("past_title", &self.past_title)
            .field("saves", &self.saves)
            .field("over", &self.over)
            .finish_non_exhaustive()
    }
}

impl Game {
    /// Opens a PARSEC install at `dir`.
    ///
    /// The game is identified by its room container's name; a directory
    /// that holds it with other required files missing is refused with
    /// every missing file named.
    pub fn open(dir: &Path) -> Result<Self> {
        let Some(title) = detect::identify(dir) else {
            return Err(Error::Unrecognized {
                missing: missing_data(dir),
            });
        };
        let missing = missing_data(dir);
        if !missing.is_empty() {
            return Err(Error::Incomplete { title, missing });
        }
        let inner: Box<dyn Program> = match title {
            Title::DunkleSchatten => Box::new(DunkleSchatten::open(dir)?),
        };
        Ok(Game {
            inner,
            buttons: crate::mouse::Latch::default(),
            keys: crate::keys::Buffer::default(),
            screen: Framebuffer::default(),
            past_title: None,
            saves: None,
            over: false,
        })
    }

    /// The title this directory held.
    pub fn title(&self) -> Title {
        self.inner.title()
    }

    /// Makes the coming [`crate::Runtime::start`] skip the title program
    /// and begin `XP01` as its menu's choice `how` would: a new game, or
    /// the saved one when there is one.
    ///
    /// Play begins in the title program, whose menu makes that choice
    /// itself, and a window never skips it; this is for a test or a
    /// headless run that wants the first location without playing through
    /// the title.
    pub fn skip_title(&mut self, how: Start) {
        self.past_title = Some(how);
    }
}

/// Every call a host makes, handed to the game's program — after the
/// machine's own part, where there is one.
impl crate::Runtime for Game {
    fn name(&self) -> &str {
        self.title().name()
    }

    /// The VGA mode the game asks for: 320×200.
    fn display_size(&self) -> Size {
        Size {
            width: DISPLAY_W,
            height: DISPLAY_H,
        }
    }

    /// Mode 13h's 320×200 filled a 4:3 monitor, so its pixels were six
    /// units tall for five wide.
    fn pixel_aspect(&self) -> PixelAspect {
        PixelAspect {
            width: 5,
            height: 6,
        }
    }

    /// Seeds what the game takes from the machine's clock: for *Dunkle
    /// Schatten*, the BIOS tick its saves and settings carry and reseed the
    /// generator from. Without it the tick starts at 0 and every run is the
    /// same, which is what a test and a screenshot want. The engine reads
    /// no clock itself.
    fn seed(&mut self, seed: u64) {
        self.inner.seed(seed);
    }

    /// Begins play, once, after [`crate::Runtime::set_saves`], where the
    /// game begins it: for *Dunkle Schatten* in its title program `XP00`,
    /// whose menu decides between a new game and the saved one — unless
    /// [`Game::skip_title`] asked to begin past it.
    fn start(&mut self) -> Result<()> {
        match self.past_title {
            Some(how) => self.inner.start_past_title(how),
            None => self.inner.start(Start::Continue),
        }
    }

    /// One frame. The oldest keystroke waiting and a press latched since
    /// the last frame reach the title first, in that order, the way the
    /// frame loop polls the input before it runs; which of the two the
    /// original's dispatcher takes first when both wait is not read. Once
    /// the player has quit, a step does nothing.
    fn step(&mut self) -> Result<()> {
        if self.over {
            return Ok(());
        }
        if let Some(code) = self.keys.take() {
            self.inner.key(code);
        }
        let (left, right) = self.buttons.take();
        if left {
            self.inner.click(true);
        }
        if right {
            self.inner.click(false);
        }
        self.over = !self.inner.tick();
        Ok(())
    }

    /// The pointer, in game coordinates, at once: the pointer a title draws
    /// follows the hand without waiting for a step.
    fn pointer(&mut self, x: i32, y: i32) {
        self.inner.mouse_move(x, y);
    }

    /// A mouse button going down or up, latched for the next step (see
    /// `mouse`): a press lands there, at the pointer as it stands then, and
    /// a release only lets the next press be one. What the game does with
    /// the right button is its own.
    fn button(&mut self, which: Button, down: bool) {
        self.buttons.button(which, down);
    }

    /// A key press, into the type-ahead buffer a later step reads it from
    /// (see `keys`). What the game reads out of a keystroke is its own.
    fn key_down(&mut self, press: &KeyPress) {
        self.keys.press(press);
    }

    /// The frame to show, pointer and all: the 320×200 picture composed
    /// into the screen the game holds, and the palette its indices mean.
    ///
    /// Composing is all this does. What a screen effect starts from — the
    /// picture last presented — is set by the frame pass inside the step,
    /// so nothing a host does between steps, asking for a frame or not,
    /// changes what the game draws next.
    fn frame(&mut self) -> Frame<'_> {
        self.screen = self.inner.render();
        Frame {
            pixels: &self.screen,
            palette: self.inner.palette(),
        }
    }

    /// How long a frame lasts: the game's answer, always one, since the
    /// game waits on a timer.
    fn frame_duration(&self) -> Option<Duration> {
        Some(self.inner.frame_duration())
    }

    /// Handed to the title.
    fn set_music(&mut self, sink: Box<dyn MusicSink>) {
        self.inner.set_music(sink);
    }

    fn music_driver(&self) -> Result<Vec<u8>> {
        self.inner.music_driver()
    }

    fn dac_driver(&self) -> Result<Option<Vec<u8>>> {
        self.inner.dac_driver()
    }

    /// The saves go into a directory of the game's own under `dir`, named
    /// for its slug — `saves/ds1/SAVEGAME.SAV` — with the original's file names inside it. Refused when `dir`
    /// resolves inside the game directory: the install is treated as
    /// read-only.
    ///
    /// The directory is resolved before the slug goes on, so that a path
    /// spelled through `..` into the install is seen to be inside it even
    /// though the game's own directory there does not exist yet.
    fn set_saves(&mut self, dir: &Path) -> Result<()> {
        let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        let own = dir.join(self.title().slug());
        self.inner.set_saves(&own)?;
        self.saves = Some(own);
        Ok(())
    }

    fn saves(&self) -> Option<&Path> {
        self.saves.as_deref()
    }

    fn diagnostics(&self) -> Vec<Diagnostic> {
        self.inner.diagnostics()
    }

    fn finished(&self) -> bool {
        self.over
    }

    /// A location, by its number in the game's own table, and optionally the entry id to arrive on rather
    /// than the one the title's door table suggests: `0` is what the
    /// dispatcher's own state 0 pushes, and it leaves the figure out of the
    /// room's chains; `7Fh` is the start. An entry the title's 16-bit
    /// entry word cannot hold is refused by name; an unknown location is
    /// ignored rather than failing, so a typo does not abort a running
    /// game.
    fn request_location(&mut self, at: LocationRequest) -> Result<()> {
        match at.entry {
            None => self.inner.set_location(at.location),
            Some(entry) => {
                let word = i16::try_from(entry).map_err(|_| Error::NoSuchEntry { entry })?;
                self.inner.set_location_via(at.location, word);
            }
        }
        Ok(())
    }

    /// The location the player is in, when the title has locations.
    fn start_location(&self) -> Option<i32> {
        self.inner.location()
    }

    fn location_name(&self) -> Option<&str> {
        self.inner.location_name()
    }
}
