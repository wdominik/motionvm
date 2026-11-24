//! How a title program hands over to `XP01`, the same for every title: the
//! batch file runs `XP00`, and `XP01` only on errorlevel 0; and how `XP01`
//! plays what it shows after its main loop, the game's end, on the same
//! library.

use std::path::Path;

use motionvm_render::{Framebuffer, Palette};

use super::{Choice, Lib, Step};
use crate::error::Result;
use crate::exe::Image;
use crate::titles::Start;
use crate::titles::parsec1994::program::Program;
use crate::titles::parsec1994::{BiosTick, PointerMap, Request};

/// What a title's `XP00` offers the game that runs it: its routine, on
/// the library, which takes the rest.
pub(crate) trait TitleProgram {
    fn lib(&self) -> &Lib;
    fn lib_mut(&mut self) -> &mut Lib;
    /// The title routine, as far as it goes before this retrace is used.
    fn run(&mut self) -> Step<Choice>;

    /// Started, `SAVEGAME.SAV` there or not.
    fn begin(&mut self, saved: bool) {
        self.lib_mut().begin(saved);
    }
    /// One retrace; `Some` once the program has ended. The picture a
    /// retrace shows is the one after everything the program does before
    /// it waits again: a palette write lands in the blanking interval, and
    /// what is drawn right after it is drawn before the beam reaches it.
    fn tick(&mut self) -> Option<Choice> {
        self.lib_mut().retrace();
        loop {
            match self.run() {
                Step::Yield => return None,
                Step::Next => {}
                Step::Done(c) => return Some(c),
            }
        }
    }
    /// The music requests made since the last call.
    fn take_music(&mut self) -> Vec<Request> {
        self.lib_mut().take_music()
    }
    /// The timer cycles it has run for.
    fn cycles(&self) -> u64 {
        self.lib().cycles()
    }
    fn palette(&self) -> &Palette {
        &self.lib().dac
    }
    fn render(&self) -> Framebuffer {
        self.lib().render()
    }
    fn mouse_move(&mut self, x: i32, y: i32) {
        self.lib_mut().mouse_move(x, y);
    }
    fn click(&mut self, left: bool) {
        self.lib_mut().click(left);
    }
    fn key(&mut self, code: u16) {
        self.lib_mut().key(code);
    }
    /// A key at the keyboard interrupt, before the BIOS: the library's hook
    /// reads it first.
    fn pointer_key(&mut self, scan: u8, down: bool) -> crate::titles::parsec1994::KeyRoute {
        self.lib_mut().pointer_key(scan, down)
    }
}

/// A title's `XP00` while it runs, before `XP01`.
pub(crate) struct TitleRun<T> {
    pub program: Option<Box<T>>,
}

impl<T: TitleProgram> TitleRun<T> {
    pub(crate) fn new() -> Self {
        TitleRun { program: None }
    }

    /// The title program begun: read by `open`, `SAVEGAME.SAV` looked for
    /// in the save directory the way `_main` probes it. `XP00` installs its
    /// digital driver with three channels (Dunkle Schatten's
    /// `0x9A2`–`0x9E4`, Captain Zins's `push 3` at `0x9A2`).
    pub(crate) fn begin(
        &mut self,
        game: &mut Program,
        open: impl FnOnce(&Path) -> Result<T>,
    ) -> Result<()> {
        let mut x = open(&game.game_dir)?;
        let saved = game.has_save();
        game.inner.music.install(3);
        x.begin(saved);
        // Both programs read the settings file at their start for the
        // mouse setting alone (`0x69E`–`0x700`).
        if let Some(cfg) = game.settings_file() {
            x.lib_mut().mouse_setting(&cfg);
        }
        self.program = Some(Box::new(x));
        game.started = true;
        Ok(())
    }

    /// A host's jump while the title program runs: it is taken down, and
    /// `XP01` starts as it does after "continue" — the settings read, the
    /// saved game when there is one — so that the jump lands where a jump
    /// from `XP01`'s start lands. The menu's song is stopped as a chosen
    /// line stops it, and the driver installed anew as `XP01` does.
    pub(crate) fn leave_for_a_jump(&mut self, game: &mut Program) {
        if self.program.take().is_some() {
            game.inner.music.send(Request::Stop);
            game.inner.music.send(Request::Reinstall);
            game.boot(Start::Continue);
        }
    }

    /// The title program's end, as `_main` and the batch file take it.
    ///
    /// A new game deletes `SAVEGAME.SAV` when it was there at the start
    /// (Dunkle Schatten's `0x625`–`0x649`, Captain Zins's the same, whose
    /// answer is not looked at); either way the batch file runs `XP01` on
    /// errorlevel 0. Quitting leaves −1, and the batch file stops: nothing
    /// of `XP01` runs, the settings are not written. The BIOS tick has
    /// gone on while the title program ran: once each time the carry word
    /// its timer chains through went over (`XP01`'s `0x19FB6`–`0x19FDA`,
    /// the same module), from 0 when the program hooked the timer.
    pub(crate) fn end(&mut self, game: &mut Program, choice: Choice) {
        let cycles = self.program.take().map_or(0, |x| x.cycles());
        // The title program's driver goes with it, and `XP01`'s init loads
        // and installs its own.
        let a = &mut game.inner;
        a.music.send(Request::Reinstall);
        let bios = u64::from(a.bios.count()) + cycles / 0x1_0000;
        a.bios = BiosTick::seeded(bios);
        match choice {
            Choice::Quit => game.over = true,
            Choice::New => {
                game.delete_save();
                game.boot(Start::New);
            }
            Choice::Continue => game.boot(Start::Continue),
        }
    }

    /// One tick while the title program runs; `None` when it is not
    /// running and `XP01` ticks instead.
    pub(crate) fn tick(&mut self, game: &mut Program) -> Option<bool> {
        let x = self.program.as_mut()?;
        let choice = x.tick();
        for request in x.take_music() {
            game.inner.music.send(request);
        }
        if let Some(choice) = choice {
            self.end(game, choice);
        }
        Some(!game.over)
    }
}

/// What `XP01` shows after its main loop, before it exits: its pictures
/// on the title programs' library, one tick a retrace.
pub(crate) trait Ending: Send {
    fn lib(&self) -> &Lib;
    fn lib_mut(&mut self) -> &mut Lib;
    /// The sequence, as far as it goes before this retrace is used;
    /// `Done` once it is over.
    fn run(&mut self) -> Step<()>;

    /// One retrace; `false` once the sequence is over and the program
    /// exits.
    fn tick(&mut self) -> bool {
        self.lib_mut().retrace();
        loop {
            match self.run() {
                Step::Yield => return true,
                Step::Next => {}
                Step::Done(()) => return false,
            }
        }
    }
    /// The music requests made since the last call.
    fn take_music(&mut self) -> Vec<Request> {
        self.lib_mut().take_music()
    }
    fn palette(&self) -> &Palette {
        &self.lib().dac
    }
    fn render(&self) -> Framebuffer {
        self.lib().render()
    }
    fn click(&mut self, left: bool) {
        self.lib_mut().click(left);
    }
    fn key(&mut self, code: u16) {
        self.lib_mut().key(code);
    }
}

/// What an ending is opened with: the program as its main loop left it.
pub(crate) struct Exit<'a> {
    /// The install's `RESOURCE` directory.
    pub res: &'a Path,
    pub image: &'a Image,
    /// Where the program keeps the pointer's words.
    pub pointer: PointerMap,
    /// The location word the main loop ended on, below zero.
    pub location: i16,
    /// The screen and the DAC the game left.
    pub screen: Framebuffer,
    pub dac: Palette,
}

/// A title's endings by the location the main loop ended on: `None` where
/// the program exits at once.
pub(crate) type Endings = fn(&Exit<'_>) -> Result<Option<Box<dyn Ending>>>;
