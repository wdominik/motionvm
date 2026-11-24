//! One loaded install of a title on this generation, as the frontend sees
//! it: the batch file's two programs, the title program `XP00` while it
//! runs and then the game program `XP01` on the generation's runtime
//! ([`Program`]). A title names its build, its title program and its own
//! reading of `XP01`; the rest is the same for every title.

use std::path::Path;
use std::time::Duration;

use motionvm_parsec_formats::Title;
use motionvm_render::{Framebuffer, Palette};

use crate::error::{Error, Result};
use crate::num;
use crate::titles::{Runtime, Start};

use super::program::Program;
use super::title::host::{TitleProgram, TitleRun};

/// A title's install: `XP01` and, before it, the title program `X`.
pub(crate) struct Install<X> {
    /// Which title it is.
    title: Title,
    /// `XP01.EXE`, the game program, on the generation's runtime.
    pub(crate) game: Program,
    /// The title program `XP00` while it runs, before `XP01`.
    pub(crate) xp00: TitleRun<X>,
    /// How the title program is read out of the install's directory.
    open_xp00: fn(&Path) -> Result<X>,
}

impl<X: TitleProgram> Install<X> {
    /// The install of `title` whose game program is `game`. Nothing is
    /// played yet: the start, which reads the save directory, is
    /// [`Runtime::start`].
    pub(crate) fn new(title: Title, game: Program, open_xp00: fn(&Path) -> Result<X>) -> Self {
        Install {
            title,
            game,
            xp00: TitleRun::new(),
            open_xp00,
        }
    }

    /// `XP01` started directly, as its batch line runs it, with the saved
    /// game read or not: what the title program leaves behind, chosen by
    /// the caller instead. A host that skips the title program starts here
    /// ([`Runtime::start_past_title`]), and so do tests that begin in the
    /// first location; play starts in the title program
    /// ([`Runtime::start`]).
    pub(crate) fn start_in_xp01(&mut self, how: Start) -> Result<()> {
        if self.game.started {
            return Err(Error::AlreadyStarted);
        }
        self.game.boot(how);
        Ok(())
    }

    /// The title program begun ([`TitleRun::begin`]).
    fn begin_xp00(&mut self) -> Result<()> {
        self.xp00.begin(&mut self.game, self.open_xp00)
    }
}

/// While the title program runs it takes the pointer, the keys and the
/// ticks; then `XP01` does.
impl<X: TitleProgram + Send> Runtime for Install<X> {
    fn title(&self) -> Title {
        self.title
    }

    fn palette(&self) -> &Palette {
        match &self.xp00.program {
            Some(x) => x.palette(),
            None => self.game.palette(),
        }
    }

    fn render(&self) -> Framebuffer {
        match &self.xp00.program {
            Some(x) => x.render(),
            None => self.game.render(),
        }
    }

    fn mouse_move(&mut self, x: i32, y: i32) {
        if let Some(t) = &mut self.xp00.program {
            t.mouse_move(x, y);
        }
        self.game.inner.mouse_move(x, y);
    }

    /// The pointer library's keyboard hook ([`super::keyboard`]) sees the
    /// key first: the title program's while it runs, then `XP01`'s.
    fn pointer_key(&mut self, scan: u8, down: bool) -> super::KeyRoute {
        if let Some(x) = &mut self.xp00.program {
            return x.pointer_key(scan, down);
        }
        match &mut self.game.inner.cursor {
            Some(c) => c.key(scan, down),
            None => super::KeyRoute::Passed,
        }
    }

    fn click(&mut self, left: bool) {
        if let Some(x) = &mut self.xp00.program {
            x.click(left);
            return;
        }
        self.game.click(left);
    }

    /// A key, mapped the way the input module's table maps the BIOS's:
    /// Escape `011Bh` is `100h`, Enter `1C0Dh` is `101h`, Tab `0F09h` is
    /// `105h` and Shift-Tab (scan `0Fh` without a character) `106h`; a
    /// character is itself. The host sends 27, 13, 9 and `100h + 0Fh` for
    /// those four. The title program's input module is the game program's,
    /// so the one mapping serves both.
    fn key(&mut self, code: i32) {
        self.game.inner.notice = None;
        let mapped = match code {
            27 => super::KEY_ESCAPE,
            13 => 0x101,
            9 => super::KEY_TAB,
            0x10F => 0x106,
            c @ 0x20..=0xFF => num::small(c),
            _ => return,
        };
        if let Some(x) = &mut self.xp00.program {
            x.key(mapped);
            return;
        }
        self.game.key(mapped);
    }

    fn tick(&mut self) -> bool {
        if !self.game.started && self.begin_xp00().is_err() {
            // An install whose title program cannot be read starts as
            // `XP01` alone would; `start` answers the error instead.
            self.game.boot(Start::Continue);
        }
        if self.game.over {
            return false;
        }
        if let Some(going) = self.xp00.tick(&mut self.game) {
            return going;
        }
        self.game.tick()
    }

    fn set_location(&mut self, n: i32) {
        self.xp00.leave_for_a_jump(&mut self.game);
        self.game.set_location(n);
    }

    fn set_location_via(&mut self, n: i32, entry: i16) {
        // The same gate as the jump's: a number past the table names no
        // handler, and is ignored.
        if !self.game.enterable(n) {
            return;
        }
        self.xp00.leave_for_a_jump(&mut self.game);
        self.game.set_location_via(n, entry);
    }

    /// The BIOS tick the machine starts with, `0:046Ch`: on the original
    /// machine the time since midnight, here the seed modulo a day. The
    /// interrupt counts it on from there, and the program reads it when it
    /// writes a save or the settings, both of which reseed the generator
    /// from it. Without a seed it starts at 0.
    fn seed(&mut self, seed: u64) {
        self.game.inner.bios = super::BiosTick::seeded(seed);
    }

    fn diagnostics(&self) -> Vec<motionvm_playable::Diagnostic> {
        self.game.diagnostics()
    }

    fn settle_events(&mut self) {
        self.game.settle_events();
    }

    /// Refused inside the install, which is treated as read-only.
    fn set_saves(&mut self, dir: &Path) -> Result<()> {
        self.game.set_saves(dir)
    }

    /// Play begins where the batch file begins it, in the title program.
    /// `how` is not the host's to choose: the title program's menu decides
    /// it, by deleting the saved game or not.
    fn start(&mut self, _how: Start) -> Result<()> {
        if self.game.started {
            return Err(Error::AlreadyStarted);
        }
        self.begin_xp00()
    }

    /// `XP01` started directly, the title program skipped, as its menu's
    /// choice `how` would leave it.
    fn start_past_title(&mut self, how: Start) -> Result<()> {
        self.start_in_xp01(how)
    }

    fn location(&self) -> Option<i32> {
        Some(self.game.inner.location())
    }

    fn location_name(&self) -> Option<&str> {
        self.game.inner.location_name()
    }

    /// `XP01`'s timer tick ([`Program::tick_duration`]). While the title
    /// program runs, a frame is a retrace of the screen, which is what its
    /// fades wait for — see [`super::title`]; so it is while the game's
    /// end plays.
    fn frame_duration(&self) -> Duration {
        if self.xp00.program.is_some() {
            return Duration::from_nanos(super::title::frame_nanos());
        }
        self.game.tick_duration()
    }

    /// Both programs' music goes the one way: `XP00`'s calls are forwarded
    /// as it makes them, `XP01`'s made by its runtime.
    fn set_music(&mut self, sink: Box<dyn crate::MusicSink>) {
        self.game.inner.music.set_sink(sink);
    }

    /// `MUSADL.DRV` as the program loads it: resource 5 of
    /// `RESOURCE\PSMDRV.DAT`, itself a RIB holding the one file. Without
    /// the music library there is no music to drive, and that is the
    /// answer instead.
    fn music_driver(&self) -> Result<Vec<u8>> {
        if let Some(why) = self.game.inner.music.missing() {
            return Err(Error::Unexpected(why.to_string()));
        }
        self.game.sound_driver(motionvm_parsec_formats::p2l::AD_LIB)
    }

    /// The digital driver beside it: resource 1 of `PSMDRV.DAT`,
    /// `DMABLAST.DRV`, the same RIB in a RIB.
    fn dac_driver(&self) -> Result<Option<Vec<u8>>> {
        if !self.game.inner.music.dac() {
            return Ok(None);
        }
        self.game
            .sound_driver(motionvm_parsec_formats::p2l::DMABLAST)
            .map(Some)
    }
}
