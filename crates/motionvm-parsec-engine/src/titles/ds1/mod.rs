//! *Dunkle Schatten* (1994).
//!
//! The game this engine opens. It loads the install and runs walking,
//! interaction, animation and dialogue machinery, and every row of the
//! location table has its handler's logic, one module per location. The
//! music goes to the Ad Lib driver, the sound effects and the songs'
//! digital voices to the digital driver's mixer.
//!
//! What is read, with the address it came from ([`xp01data`] carries the
//! sites):
//!
//! - A location is a row in a 52-byte record table at `204Bh:0000` whose
//!   `+23h` is the far pointer to that location's handler, and the main
//!   loop at `0x4FC0` dispatches on `[1F4Ah]`.
//! - A fresh game opens at location **2**, entry `7Fh` (`0x4F37`), which
//!   the program takes when the loader left `[1F4Ah]` at zero (`0x4F2A`).
//! - The status line comes from a sixteen-entry far-pointer row at
//!   `DS:1084` indexed by `[288Dh]` (`0x10124`). Those templates carry
//!   `%c` ink controls — `%cBetrachte %c%s`.
//!
//! What the shipped files say, read with `motionvm-parsec-formats`:
//!
//! - `DSROOM.DAT` is an FGROUP of 60 slots; 28 of them carry an `RDV1.00`
//!   playfield, and slot 0 is the `NULLPTR` marker.
//! - `PANELS.DAT` resource 0 is a **320×72** picture and resource 1 a
//!   palette bank at 160.
//! - `CMBMOU.DAT` resource 0 is a palette bank at 224 and resource 15 the
//!   font; `PLAY00.DAT` resource 0 is a bank at 128 and the rest are the
//!   player's 25×46 cels.

// The location logic, one module per location the game names in its
// location table, written against the engine boundary `parsec1994::Api` and
// driven by `logic`.
mod bogenstrasse;
mod brand;
mod f1;
mod f2;
mod f3;
mod forest_maze;
mod hook;
mod in_der_ruine;
mod joerg;
mod jz;
mod karte;
mod kiosk;
mod krankenhaus;
mod kueche;
mod laden;
mod logic;
mod mauer;
mod monopol;
mod onward;
mod ralf;
mod restore;
mod ruine;
mod schule;
mod schuppen;
mod stammtisch;
mod story;
mod strasse;
#[cfg(test)]
mod trace;
mod wald;
mod werkstatt;
mod wohnzimmer;
mod xp00;
mod xp01data;
mod zimmer;

use crate::num;
use std::path::{Path, PathBuf};
use std::time::Duration;

use motionvm_parsec_formats::Title;
use motionvm_parsec_formats::detect::resource_dir;
use motionvm_render::{Framebuffer, Palette};

use crate::error::{Error, Result};
use crate::exe::Image;
use crate::titles::xp1994::Locations;
use crate::titles::{Start, parsec1994};
use parsec1994::{Api, SaveMap};

use logic::Visit;
use xp01data::ExeText;

/// One loaded install of *Dunkle Schatten*.
pub(crate) struct DunkleSchatten {
    /// This title's own reading of its own program.
    ///
    /// Most of it is read and pinned before the runtime needs it, which is
    /// deliberate: a fact taken out of a listing with an address beside it
    /// is worth keeping the moment it is read.
    text: ExeText,
    /// The parsec1994 generation's parts: locations, rooms, strip, palette
    /// banks.
    inner: parsec1994::Adventure,
    /// The location being visited, whose handler's frame loop the ticks
    /// run. `None` before the main loop and after it, and after a host's
    /// jump without an entry.
    visit: Option<Visit>,
    /// The install, which savegames must not be written into.
    game_dir: PathBuf,
    /// Where `SAVEGAME.SAV` and `SAVEGAME.CFG` are read and written; the
    /// original uses the DOS working directory (`0590h:0DD5h`, `*` names).
    saves: Option<PathBuf>,
    /// Whether play has begun: the title program, or `XP01`'s start
    /// (`_main`, `0x4F19`–`0x4F87`) directly.
    started: bool,
    /// The title program `XP00` while it runs, before `XP01`.
    xp00: Option<Box<xp00::Xp00>>,
    /// Whether the main loop has ended: the player quit.
    over: bool,
}

impl DunkleSchatten {
    /// Opens the install at `dir`. Nothing is played yet: the start, which
    /// reads the save directory, is [`crate::titles::Runtime::start`].
    pub(crate) fn open(dir: &Path) -> Result<Self> {
        let res = resource_dir(dir);
        let text = ExeText::read(dir)?;
        let image = Image::open(dir, "XP01.EXE", 150_096)?;
        let locations = Locations::read(&image)?;
        let map = logic::ds_map(&locations, text.inventory_at);
        let mut inner = parsec1994::Adventure::open(
            &res,
            image,
            locations,
            map,
            parsec1994::Files {
                rooms: "DSROOM.DAT",
                panel: "PANELS.DAT",
                pointers: "CMBMOU.DAT",
                player: "PLAY00.DAT",
                templates: (text.template_row, xp01data::VERB_TEMPLATES),
                music: ("PSMP2L.DAT", parsec1994::MusicMap::DS1),
            },
        )?;
        inner.options_map = Some(parsec1994::OptionsMap::DS1);
        inner.install_pointer(parsec1994::PointerMap::DS1);
        inner.rule_hook = logic::rule_handler;
        inner.hook = hook::answer;
        inner.location_hook = logic::location_hook;
        inner.anim_hook = logic::anim_hook;
        // The substitution row as the image ships it. Nothing is drawn
        // into it at the start: the roll at `0x1189` that fills entry 0
        // sits inside the Wohnzimmer's handler, for a wrong number on the
        // telephone.
        inner.substitutions = text
            .substitutions
            .iter()
            .map(|s| motionvm_parsec_formats::to_cp437(s))
            .collect();
        inner.initialize_inventory(&text.initial_items, text.initial_scroll)?;
        inner.enable_panel_colors(text.panel_curves, text.gray_ramp.clone())?;
        if logic::logic(num::low16(text.start_location)).is_none() {
            return Err(format!(
                "the program opens at location {} and that location cannot be entered",
                text.start_location
            )
            .into());
        }
        Ok(DunkleSchatten {
            text,
            inner,
            visit: None,
            game_dir: dir.to_path_buf(),
            saves: None,
            started: false,
            xp00: None,
            over: false,
        })
    }

    /// A file of the save directory, by the name the program builds.
    fn save_path(&self, name_at: u16) -> Option<PathBuf> {
        let dir = self.saves.as_ref()?;
        let name = self.inner.save_file_name(name_at);
        Some(motionvm_parsec_formats::find_ci(dir, &name).unwrap_or_else(|| dir.join(name)))
    }

    /// `XP01` started directly, as its batch line runs it, with the saved
    /// game read or not: what the title program leaves behind, chosen by
    /// the caller instead. Tests that begin in the first location start
    /// here, and so does a host that skips the title program
    /// ([`Runtime::start_past_title`]); play starts in the title program
    /// ([`Runtime::start`]).
    ///
    /// [`Runtime::start`]: crate::titles::Runtime::start
    /// [`Runtime::start_past_title`]: crate::titles::Runtime::start_past_title
    pub(crate) fn start_in_xp01(&mut self, how: Start) -> Result<()> {
        if self.started {
            return Err(Error::AlreadyStarted);
        }
        self.boot(how);
        Ok(())
    }

    /// Resource `n` of `RESOURCE\PSMDRV.DAT`, a driver file.
    fn sound_driver(&self, n: usize) -> Result<Vec<u8>> {
        let res = resource_dir(&self.game_dir);
        let path = motionvm_parsec_formats::find_ci(&res, "PSMDRV.DAT")
            .ok_or("RESOURCE/PSMDRV.DAT: not found")?;
        Ok(motionvm_parsec_formats::p2l::driver(
            &std::fs::read(path)?,
            n,
        )?)
    }

    /// The title program begun: `XP00.EXE` read, `SAVEGAME.SAV` looked
    /// for in the save directory the way `_main` probes it (`0x5EC`).
    fn begin_xp00(&mut self) -> Result<()> {
        let mut x = xp00::Xp00::open(&self.game_dir)?;
        let saved = self
            .save_path(SaveMap::DS1.sav_name)
            .is_some_and(|p| p.is_file());
        // XP00 `0x9A2`–`0x9E4`: its digital driver with three channels.
        self.inner.music.install(3);
        x.begin(saved);
        self.xp00 = Some(Box::new(x));
        self.started = true;
        Ok(())
    }

    /// A host's jump while the title program runs: it is taken down, and
    /// `XP01` starts as it does after "continue" — the settings read, the
    /// saved game when there is one — so that the jump lands where a jump
    /// from `XP01`'s start lands. The menu's song is stopped as a chosen
    /// line stops it (`0x3B07`), and the driver installed anew as `XP01`
    /// does.
    fn leave_xp00_for_a_jump(&mut self) {
        if self.xp00.take().is_some() {
            self.inner.music.send(parsec1994::Request::Stop);
            self.inner.music.send(parsec1994::Request::Reinstall);
            self.boot(Start::Continue);
        }
    }

    /// The title program's end, as `_main` and the batch file take it.
    ///
    /// A new game deletes `SAVEGAME.SAV` when it was there at the start
    /// (`0x625`–`0x649`, `0D66h:0150h`, whose answer is not looked at);
    /// either way the batch file runs `XP01` on errorlevel 0. Quitting
    /// leaves −1 (`0x64E`), and the batch file stops: nothing of `XP01`
    /// runs, the settings are not written. The BIOS tick has gone on
    /// while the title program ran: once each time the carry word its
    /// timer chains through went over (`XP01`'s `0x19FB6`–`0x19FDA`, the
    /// same module), from 0 when the program hooked the timer.
    fn end_xp00(&mut self, choice: xp00::Choice) {
        let cycles = self.xp00.take().map_or(0, |x| x.cycles());
        // The title program's driver goes with it, and `XP01`'s init loads
        // and installs its own (`0x5B7B`).
        self.inner.music.send(parsec1994::Request::Reinstall);
        let bios = u64::from(self.inner.bios.count()) + cycles / 0x1_0000;
        self.inner.bios = parsec1994::BiosTick::seeded(bios);
        match choice {
            xp00::Choice::Quit => self.over = true,
            xp00::Choice::New => {
                if let Some(p) = self.save_path(SaveMap::DS1.sav_name)
                    && p.is_file()
                {
                    let _ = std::fs::remove_file(p);
                }
                self.boot(Start::New);
            }
            xp00::Choice::Continue => self.boot(Start::Continue),
        }
    }

    /// `_main` from `0x4F19` to the options (`0x4F87`), after the sound of
    /// its init: the main loop's words, the start's load (`147Ah:0072h`), a
    /// fresh game when nothing was loaded (`0x4F2A`), then the options,
    /// unconditionally.
    fn boot(&mut self, how: Start) {
        self.started = true;
        let map = SaveMap::DS1;
        let a = &mut self.inner;
        // The init before it: the digital driver with six channels
        // (`0x5BD3`–`0x5C15`), the effects' volume set again (`0x4E85`),
        // and the twelve effects every room keeps (`0x053AB`).
        a.music.install(6);
        a.music.send(parsec1994::Request::EffectVolume(0x3C));
        a.effects_preload();
        a.here_word = 0xFFFF; // 0x4F19
        a.entry_word = 0; // 0x4F1F
        a.save_buffer_clear(); // 0x14821
        // 0x14847: the settings, when the file is exactly 1Fh bytes.
        if let Some(bytes) = self
            .save_path(map.cfg_name)
            .and_then(|p| std::fs::read(p).ok())
            && let parsec1994::Cfg::Taken { mouse, text, speed } =
                parsec1994::cfg_read(&bytes, self.inner.rng_mut())
        {
            self.inner.cfg_take(&map, mouse, text, speed);
        }
        // 0x148ED: the saved game, when the file is exactly 8B1h bytes.
        // A new game is one whose file the title program deleted.
        let sav = match how {
            Start::New => None,
            Start::Continue => self
                .save_path(map.sav_name)
                .and_then(|p| std::fs::read(p).ok()),
        };
        if let Some(bytes) = sav.filter(|b| b.len() == parsec1994::SAV_LEN) {
            self.inner.save_buffer_fill(&bytes);
            if parsec1994::sav_valid(&bytes, self.inner.rng_mut()) {
                // 0x149D3: held, and restored at once (`147Ah:052Ch`).
                self.inner.set_word(map.loaded, 1);
                restore::enter(&mut self.inner);
            } else {
                // 0x149C7: the original ends here.
                let msg = self.inner.save_message(map.msg_invalid, "");
                self.inner.notice = Some(msg);
            }
        }
        let a = &mut self.inner;
        if a.location_word == 0 {
            // 0x4F31: a fresh game.
            a.here_word = 0xFFFF;
            a.location_word = num::low16(self.text.start_location);
            a.entry_word = num::unsigned(self.text.start_entry);
        }
        a.options_open(parsec1994::Opened::Start);
    }

    /// The file the options' save left, written (`1E69h:003Ch`, `00C0h`).
    /// Where the original ends the program, the message stays up instead.
    fn write_save(&mut self, name_at: u16, bytes: &[u8]) {
        use std::io::Write;
        let map = SaveMap::DS1;
        let name = self.inner.save_file_name(name_at);
        let file = self
            .save_path(name_at)
            .and_then(|p| std::fs::File::create(p).ok());
        let Some(mut file) = file else {
            self.inner.notice = Some(self.inner.save_message(map.msg_create, &name));
            return;
        };
        if file.write_all(bytes).is_err() {
            self.inner.notice = Some(self.inner.save_message(map.msg_write, &name));
        }
    }

    /// After the main loop: the settings written when they changed
    /// (`0x4FF0`, `147Ah:0373h`).
    fn finish(&mut self) {
        self.over = true;
        let map = SaveMap::DS1;
        let tick = self.inner.bios.count();
        if let Some(cfg) = self.inner.cfg_0373(&map, tick) {
            self.write_save(map.cfg_name, &cfg);
        }
    }

    /// The main loop's next location entered — or, below zero, its end.
    fn main_loop(&mut self) {
        if num::signed(self.inner.location_word) < 0 {
            self.finish();
        } else {
            self.visit = Some(Visit::enter(&mut self.inner));
        }
    }
}

/// The frontend's view of this title.
///
/// Pointer input, keys and ticks reach the parsec1994 runtime; the visit's
/// location logic runs its frame loop one turn per tick, and the options
/// run theirs while they are up.
impl crate::titles::Runtime for DunkleSchatten {
    fn title(&self) -> Title {
        Title::DunkleSchatten
    }

    fn palette(&self) -> &Palette {
        match &self.xp00 {
            Some(x) => x.palette(),
            None => self.inner.palette(),
        }
    }

    fn render(&self) -> Framebuffer {
        match &self.xp00 {
            Some(x) => x.render(),
            None => self.inner.render(),
        }
    }

    fn mouse_move(&mut self, x: i32, y: i32) {
        if let Some(t) = &mut self.xp00 {
            t.mouse_move(x, y);
        }
        self.inner.mouse_move(x, y);
    }

    /// A button. The options take any button the same (`cs:0C86h`: events
    /// 5, 6 and 7 go to one arm); the game takes the left.
    fn click(&mut self, left: bool) {
        if let Some(x) = &mut self.xp00 {
            x.click(left);
            return;
        }
        self.inner.notice = None;
        if self.inner.options_up() {
            self.inner.options_input(parsec1994::OptionsInput::Click);
        } else if left {
            self.inner.click();
        }
    }

    /// A key, mapped the way `179Eh:0611h` maps the BIOS's (`0x18041`–
    /// `0x18097`): Escape `011Bh` is `100h`, Enter `1C0Dh` is `101h`, Tab
    /// `0F09h` is `105h` and Shift-Tab (scan `0Fh` without a character)
    /// `106h`; a character is itself. The host sends 27, 13, 9 and
    /// `100h + 0Fh` for those four.
    fn key(&mut self, code: i32) {
        self.inner.notice = None;
        let mapped = match code {
            27 => parsec1994::KEY_ESCAPE,
            13 => 0x101,
            9 => parsec1994::KEY_TAB,
            0x10F => 0x106,
            c @ 0x20..=0xFF => num::small(c),
            _ => return,
        };
        // The title program's input module is this one's (`08E6h` there,
        // `179Eh` here: the mapping at `0x9401` is `0x18041`'s bytes).
        if let Some(x) = &mut self.xp00 {
            x.key(mapped);
            return;
        }
        if self.inner.options_up() {
            self.inner
                .options_input(parsec1994::OptionsInput::Key(mapped));
            return;
        }
        // The choice menu's own table (`cs:11ACh`, `0x0E0B6`): `20h` and
        // `101h` take, `105h` and `106h` move.
        if self.inner.menu.up {
            let key = match mapped {
                0x20 | 0x101 => parsec1994::MenuKey::Select,
                0x105 => parsec1994::MenuKey::Down,
                0x106 => parsec1994::MenuKey::Up,
                _ => return,
            };
            self.inner.menu_key(key);
            return;
        }
        self.inner.key_event(mapped);
    }

    fn tick(&mut self) -> bool {
        if !self.started && self.begin_xp00().is_err() {
            // An install whose title program cannot be read starts as
            // `XP01` alone would; `start` answers the error instead.
            self.boot(Start::Continue);
        }
        if self.over {
            return false;
        }
        if let Some(x) = &mut self.xp00 {
            let choice = x.tick();
            for request in x.take_music() {
                self.inner.music.send(request);
            }
            if let Some(choice) = choice {
                self.end_xp00(choice);
            }
            return !self.over;
        }
        // The timer's tick, then one turn of the running handler's frame
        // loop: the input polled and dispatched, the handler's own check,
        // the shared frame pass — and when the turn ends the visit, the
        // handler leaves and the main loop enters the next location's.
        self.inner.interrupt();
        // A screen effect blocks the game: the clock goes on, the handler's
        // frame loop waits.
        if self.inner.effects.busy() {
            self.inner.effects_tick();
            return true;
        }
        if self.inner.options_up() {
            let closed = self.inner.options_tick();
            if let Some(bytes) = self.inner.save_written.take() {
                self.write_save(SaveMap::DS1.sav_name, &bytes);
            }
            match closed {
                // `_main` goes on into its main loop (`0x4F8C`).
                Some(parsec1994::Opened::Start) => self.main_loop(),
                // The gate goes on after its call (`0x0F033`).
                Some(parsec1994::Opened::Gate) => self.inner.options_gate_close(),
                None => {}
            }
            return !self.over;
        }
        match self.visit.take() {
            Some(v) => {
                self.visit = v.frame(&mut self.inner);
                if self.visit.is_none() {
                    self.finish();
                }
            }
            None => {
                self.inner.dispatch();
                self.inner.frame_pass();
            }
        }
        if let Some(bytes) = self.inner.save_written.take() {
            self.write_save(SaveMap::DS1.sav_name, &bytes);
        }
        !self.over
    }

    fn set_location(&mut self, n: i32) {
        // A row of kind 0 is not a place: the main loop hands it to the
        // walk on to the next row that is one, which names the entry,
        // `7Fh`, itself ([`onward`]). The jump goes the same way.
        if u16::try_from(n)
            .is_ok_and(|r| self.inner.locations.kind(&self.inner.image, usize::from(r)) == Some(0))
        {
            self.set_location_via(n, 0x7F);
            return;
        }
        self.leave_xp00_for_a_jump();
        self.started = true;
        self.inner.options_drop();
        self.inner.complete_entry();
        if let Some(v) = self.visit.take() {
            v.leave(&mut self.inner);
        }
        self.inner.set_location(n);
    }

    fn set_location_via(&mut self, n: i32, entry: i16) {
        // The same gate as the jump's: a number past the table names no
        // handler, and is ignored.
        if u16::try_from(n).ok().and_then(logic::logic).is_none() {
            return;
        }
        self.leave_xp00_for_a_jump();
        self.started = true;
        self.inner.options_drop();
        self.inner.complete_entry();
        if let Some(v) = self.visit.take() {
            v.leave(&mut self.inner);
        }
        self.inner.location_word = num::low16(n);
        self.inner.entry_word = num::unsigned(entry);
        self.visit = Some(Visit::enter(&mut self.inner));
    }

    /// The BIOS tick the machine starts with, `0:046Ch`: on the original
    /// machine the time since midnight, here the seed modulo a day. The
    /// interrupt counts it on from there, and the program reads it when it
    /// writes a save or the settings, both of which reseed the generator
    /// from it. Without a seed it starts at 0.
    fn seed(&mut self, seed: u64) {
        self.inner.bios = parsec1994::BiosTick::seeded(seed);
    }

    /// Refused inside the install, which is treated as read-only.
    fn set_saves(&mut self, dir: &Path) -> Result<()> {
        let game = std::fs::canonicalize(&self.game_dir).unwrap_or_else(|_| self.game_dir.clone());
        let want = if dir.exists() {
            std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())
        } else {
            dir.to_path_buf()
        };
        if want.starts_with(&game) {
            return Err(Error::SavesInsideGame);
        }
        std::fs::create_dir_all(&want)?;
        self.saves = Some(want);
        Ok(())
    }

    /// Play begins where the batch file begins it, in the title program.
    /// `how` is not this title's to take from the host: the title
    /// program's menu decides it, by deleting the saved game or not.
    /// Tests start `XP01` directly with `start_in_xp01`.
    fn start(&mut self, _how: Start) -> Result<()> {
        if self.started {
            return Err(Error::AlreadyStarted);
        }
        self.begin_xp00()
    }

    /// `XP01` started directly, the title program skipped.
    fn start_past_title(&mut self, how: Start) -> Result<()> {
        self.start_in_xp01(how)
    }

    fn location(&self) -> Option<i32> {
        Some(self.inner.location())
    }

    fn location_name(&self) -> Option<&str> {
        self.inner.location_name()
    }

    /// A 120th of a second, read from the timer.
    ///
    /// The library's init hooks INT 8 (`1950h:1883h`) and registers the
    /// record at `DS:1C74h` with it (`0x5B45`, `push 1C74h`; `1950h:170Fh`).
    /// The handler at `0x19F40` then counts the elapsed PIT cycles into
    /// that record, and its second group of counters — `[1C76h]`, which
    /// the frame pass takes as its delta at `0x0ED6E` — advances once
    /// every `26D6h` cycles (`0x19F7F`): 9942 cycles of the 1 193 182 Hz
    /// chip, 120 ticks a second.
    ///
    /// While the title program runs, a frame is a retrace of the screen,
    /// which is what its fades wait for — see [`xp00`].
    fn frame_duration(&self) -> Duration {
        if self.xp00.is_some() {
            return Duration::from_nanos(xp00::frame_nanos());
        }
        Duration::from_nanos(parsec1994::Clock::tick_nanos())
    }

    /// Both programs' music goes the one way: `XP00`'s calls are forwarded
    /// as it makes them, `XP01`'s made by its runtime.
    fn set_music(&mut self, sink: Box<dyn crate::MusicSink>) {
        self.inner.music.set_sink(sink);
    }

    /// `MUSADL.DRV` as the program loads it: resource 5 of
    /// `RESOURCE\PSMDRV.DAT` (`DS:0A06h`, installed at `0x5B7B`), itself a
    /// RIB holding the one file. Without the music library there is no
    /// music to drive, and that is the answer instead.
    fn music_driver(&self) -> Result<Vec<u8>> {
        if let Some(why) = self.inner.music.missing() {
            return Err(Error::Unexpected(why.to_string()));
        }
        self.sound_driver(motionvm_parsec_formats::p2l::AD_LIB)
    }

    /// The digital driver beside it: resource 1 of `PSMDRV.DAT`,
    /// `DMABLAST.DRV` (`0x5BB3`, installed at `0x5BEC`), the same RIB in a
    /// RIB.
    fn dac_driver(&self) -> Result<Option<Vec<u8>>> {
        if !self.inner.music.dac() {
            return Ok(None);
        }
        self.sound_driver(motionvm_parsec_formats::p2l::DMABLAST)
            .map(Some)
    }
}

/// The install's directory, or `None` on a machine that has not got the
/// game — said on the test's output, so that a skip does not read as a
/// pass. Every test helper that answers `None` for want of the game asks
/// here, so its callers return without saying it again.
#[cfg(test)]
pub(crate) fn gamedata_or_skip() -> Option<PathBuf> {
    let dir = motionvm_parsec_testutil::gamedata_ds1();
    if dir.is_none() {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
    }
    dir
}

/// The install, or `None` on a machine that has not got this title.
#[cfg(test)]
fn ds1_or_skip() -> Option<DunkleSchatten> {
    let dir = gamedata_or_skip()?;
    let mut g = DunkleSchatten::open(&dir).expect("open Dunkle Schatten");
    g.start_in_xp01(Start::New).expect("start");
    close_start_options(&mut g);
    Some(g)
}

/// The options `XP01` opens before its first location, closed the way a
/// player closes them: Escape once they have dissolved in (keys typed
/// while they do are flushed, `0x12106`). The main loop then enters the
/// first location.
#[cfg(test)]
fn close_start_options(g: &mut DunkleSchatten) {
    use crate::titles::Runtime;
    while g.inner.options_fading() {
        g.tick();
    }
    g.key(27);
    g.tick();
    assert!(!g.inner.options_up(), "Escape closes the options");
}

/// The parsec1994 runtime of a fresh game, for the shared runtime's own
/// tests — which need an install to stand on, and this is the one they have.
#[cfg(test)]
pub(crate) fn adventure_for_test() -> Option<parsec1994::Adventure> {
    ds1_or_skip().map(|g| g.inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::titles::parsec1994::{Api, Event, Far};
    use motionvm_parsec_formats::Bank;

    /// Plays the screen effects a jump queued to their end, the way the
    /// original's blocking loops would before anything else runs.
    fn settle_effects(g: &mut DunkleSchatten) {
        let mut guard = 0;
        while g.inner.effects.busy() && guard < 10_000 {
            crate::titles::Runtime::tick(g);
            guard += 1;
        }
    }

    /// `[2725h]`, the byte every location handler switches on.
    const STATE: usize = 0x2725;
    /// `[270Fh]`: 2 while Karsten sleeps in the bed.
    const BED: usize = 0x270F;

    /// The row of names at `DS:1060h` a wrong number draws from, as the
    /// running program holds it.
    fn wrong_numbers(g: &DunkleSchatten) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let mut at = 0x1060u16;
        loop {
            let off = usize::from(g.inner.word(at));
            let seg = usize::from(g.inner.word(at + 2));
            if off == 0 && seg == 0 {
                return out;
            }
            let name = g.inner.image.c_string(seg * 16 + off).expect("a name");
            out.push(motionvm_parsec_formats::to_cp437(&name));
            at += 4;
        }
    }

    /// The Wohnzimmer father's base animation, `[18A4h]`.
    fn father_base(g: &DunkleSchatten) -> i16 {
        num::signed(g.inner.word(0x18A4))
    }

    macro_rules! ds1 {
        () => {
            match ds1_or_skip() {
                Some(g) => g,
                None => return,
            }
        };
    }

    /// The Zimmer on a later day: entered with the state the wake-up arm
    /// leaves, so its handler plays no opening and a click reaches the
    /// room. The opening's own text takes every click while it is up
    /// (`0CFDh:0FD9h`), so the tests that click go here first.
    fn zimmer_on_a_later_day() -> Option<DunkleSchatten> {
        let mut g = ds1_or_skip()?;
        crate::titles::Runtime::set_location_via(&mut g, 3, 100);
        settle_effects(&mut g);
        g.inner.sav_set(STATE, 4);
        g.inner.sav_set(BED, 0);
        // The opening is over on a later day: waking up clears `[1892h]`
        // (`0xA5E`), or the Zimmer's hook would arm task 0 at every page.
        g.inner.set_word(0x1892, 0);
        crate::titles::Runtime::set_location_via(&mut g, 2, 0x7F);
        settle_effects(&mut g);
        Some(g)
    }

    macro_rules! zimmer_later {
        () => {
            match zimmer_on_a_later_day() {
                Some(g) => g,
                None => {
                    eprintln!("skipping: no Dunkle Schatten gamedata directory");
                    return;
                }
            }
        };
    }

    /// The middle of a hotspot.
    fn middle(h: &motionvm_parsec_formats::Hotspot) -> (i32, i32) {
        (
            i32::from(h.x) + i32::from(h.w) / 2,
            i32::from(h.y) + i32::from(h.h) / 2,
        )
    }

    /// A text without its controls and breaks, single-spaced, for
    /// comparing what a caption shows with what a record says.
    fn flat(s: &str) -> String {
        let mut out = String::new();
        let mut it = s.chars();
        while let Some(c) = it.next() {
            match c {
                '\u{1}' | '\u{2}' | '\u{3}' => {
                    it.next();
                }
                '\n' | '\r' => out.push(' '),
                c => out.push(c),
            }
        }
        out.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Whether task 0 shows a text or a dialogue — not a hover text, which
    /// is kind 2 and stays while the pointer does.
    fn text_up(g: &DunkleSchatten) -> bool {
        matches!(g.inner.tasks.t[0].kind, 3 | 4) && g.inner.caption_is_up()
    }

    /// Lets whatever task 0 is doing run out, with the pointer off the
    /// room so that no hover text keeps it, until the machine is not
    /// acting any more.
    fn settle(g: &mut DunkleSchatten) {
        g.inner.mouse_move(0, 0);
        for _ in 0..12_000 {
            if !matches!(g.inner.tasks.t[0].kind, 3..=5) && !matches!(g.inner.mode, 4..=8) {
                return;
            }
            // A menu waits for the player: the first choice is taken.
            if g.inner.tasks.t[0].kind == 5 {
                g.inner.menu_key(parsec1994::MenuKey::Down);
                g.inner.menu_key(parsec1994::MenuKey::Select);
            }
            crate::titles::Runtime::tick(g);
        }
        panic!(
            "the machine did not settle: mode {} kind {} bits {:x} target {} gate {:?} node {} walking {:?} verb {} object {}",
            g.inner.mode,
            g.inner.tasks.t[0].kind,
            g.inner.bits,
            g.inner.walk_target,
            g.inner.gate,
            g.inner.player_node_id(),
            g.inner.walker.as_ref().map(|w| w.walking),
            g.inner.verb,
            g.inner.object1
        );
    }

    /// `verb` on `spot`, the way a player does it: the button, the pointer
    /// over the object, the click, and the walk there. Answers with the
    /// text that came up, or `None` when the rule said nothing in a
    /// hundred seconds of frames.
    fn act(
        g: &mut DunkleSchatten,
        verb: u8,
        spot: &motionvm_parsec_formats::Hotspot,
    ) -> Option<String> {
        settle(g);
        // The button (`0x0FC12`): mode 2 and the verb, nothing chosen.
        g.inner.mode = 2;
        g.inner.verb = verb;
        g.inner.verb_chosen = u16::from(verb);
        g.inner.object1 = 0;
        g.inner.object2 = 0;
        g.inner.two_object = 0;
        g.inner.mouse_move(0, 0);
        crate::titles::Runtime::tick(g);
        let (x, y) = middle(spot);
        g.inner.mouse_move(x, y);
        crate::titles::Runtime::tick(g);
        if !g.inner.click() {
            return None;
        }
        for _ in 0..12_000 {
            crate::titles::Runtime::tick(g);
            if text_up(g) {
                return Some(g.inner.caption_text());
            }
            if g.inner.mode == 0 && g.inner.tasks.t[0].kind == 0 && g.inner.player_linked() {
                return None;
            }
        }
        None
    }

    /// Every row of the location table has its handler's logic, the rows
    /// of kind 0 among them: the main loop reaches no handler it cannot
    /// run.
    #[test]
    fn every_row_has_its_logic() {
        let g = ds1!();
        let locs = &g.inner.locations;
        let missing: Vec<usize> = (0..locs.count)
            .filter(|&n| logic::logic(u16::try_from(n).unwrap()).is_none())
            .collect();
        assert!(missing.is_empty(), "rows without logic: {missing:?}");
    }

    /// Row 20 is no place: entered on an entry, its handler walks on to the
    /// Schuppen, row 21, on entry `7Fh`, and the Schuppen's handler takes
    /// over. A jump to row 20 without an entry goes the same way, and so
    /// does one to row 0, which walks on to the Karte.
    #[test]
    fn row_20_walks_on_to_the_schuppen() {
        for jump in [false, true] {
            let mut g = ds1!();
            if jump {
                crate::titles::Runtime::set_location(&mut g, 20);
            } else {
                crate::titles::Runtime::set_location_via(&mut g, 20, 0x7F);
            }
            assert_eq!(g.inner.location_word, 21);
            assert_eq!(g.inner.entry_word, 0x7F);
            settle_effects(&mut g);
            crate::titles::Runtime::tick(&mut g);
            settle_effects(&mut g);
            assert_eq!(g.inner.location(), 21);
            assert_eq!(g.inner.here_word, 21);
        }
        let mut g = ds1!();
        crate::titles::Runtime::set_location(&mut g, 0);
        assert_eq!((g.inner.location_word, g.inner.entry_word), (1, 0x7F));
        settle_effects(&mut g);
        crate::titles::Runtime::tick(&mut g);
        settle_effects(&mut g);
        assert_eq!(g.inner.location(), 1);
    }

    /// A forest row is drawn from slot 20, the forest picture, with its own
    /// object map bound to the picture's records: the Wald moves `D5h` from
    /// (255, 110) to (79, 119) and shows `D7h` on the frame its map names,
    /// cel 240 rather than the file's 238; the maze's row 30 moves `D6h`
    /// from (160, 113) to (223, 113).
    ///
    /// Held against the original under DOSBox-X, arrived at through its
    /// developer's location choice from a saved game on the map: the
    /// maze's row 30 agrees pixel for pixel in all 38 frames of the
    /// capture's playfield, the Wald in 25 of 38. The other 13 differ by 4
    /// to 19 pixels inside `D7h`'s box alone — its animation, started by
    /// the Wald's handler (`0883h:2099h(D7h, 0)`, `0x03F8B`), walks its
    /// frames at random (opcode `9Bh`) on the game's generator, whose state
    /// at the capture is not known.
    #[test]
    fn a_forest_row_shows_slot_20_through_its_object_map() {
        let mut g = ds1!();
        let object = |g: &DunkleSchatten, id: u8| {
            let o = &g.inner.objects[g.inner.object_index(id).expect("the object")];
            (o.x, o.y, o.cel, o.visible())
        };
        let picture = |g: &DunkleSchatten| {
            let room = g.inner.room().expect("a playfield");
            let slot = g.inner.rooms[20].as_ref().expect("slot 20");
            room.room.palette_rgb == slot.room.palette_rgb && room.walks.len() == slot.walks.len()
        };
        crate::titles::Runtime::set_location_via(&mut g, 23, 109);
        assert!(picture(&g));
        assert_eq!(object(&g, 0xD5), (79, 119, object(&g, 0xD5).2, true));
        assert_eq!(object(&g, 0xD7).2, 240);
        crate::titles::Runtime::set_location_via(&mut g, 30, 102);
        assert!(picture(&g));
        assert_eq!((object(&g, 0xD6).0, object(&g, 0xD6).1), (223, 113));
        assert!(object(&g, 0xD6).3);
    }

    /// The ids a forest row's hotspots carry once its map is bound, in the
    /// picture's table order, with whether each is active.
    fn forest_exits(g: &DunkleSchatten) -> Vec<(u16, bool)> {
        let room = g.inner.room().expect("a playfield");
        room.hotspots
            .spots
            .iter()
            .zip(&g.inner.hotspots_rt)
            .map(|(h, rt)| (h.verb_mask & 0xFF, rt.active))
            .collect()
    }

    /// A forest row's map gives the picture's exits the row's own ids.
    ///
    /// The picture's hotspots are exits 100 to 104, 109 and 105, and so are
    /// the walk graph's nodes, with twins at 2100 to 2104 and 2109. Row 30
    /// lists its exits 2 to 5 as 103, 104, 101 and 102: the hotspot and the
    /// node that were 101 become 103, the twin 2101 becomes 2103, and so on
    /// round; 105 and 109 belong to the Wald and go to id 0, inactive. The
    /// Wald itself has exit 4 alone, 101: 101 and 102 and 104 hide, 103
    /// becomes 101, its twin 2101, and the other twins 2000.
    #[test]
    fn a_forest_row_renames_the_pictures_exits_to_its_own() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 30, 100);
        settle_effects(&mut g);
        assert_eq!(
            forest_exits(&g),
            [
                (100, true),
                (103, true),
                (104, true),
                (101, true),
                (102, true),
                (0, false),
                (0, false)
            ]
        );
        let graph = g.inner.room().and_then(|r| r.walk()).expect("a walk graph");
        let ids: Vec<u16> = graph.nodes.iter().map(|n| n.id).collect();
        for (was, now) in [
            (0, 2103),
            (1, 103),
            (2, 2102),
            (5, 102),
            (7, 101),
            (11, 2101),
            (13, 2104),
            (24, 104),
        ] {
            assert_eq!(ids[was], now, "node {was}");
        }
        assert_eq!((ids[20], ids[21], ids[25], ids[26]), (2100, 100, 2109, 0));

        crate::titles::Runtime::set_location_via(&mut g, 23, 100);
        settle_effects(&mut g);
        assert_eq!(
            forest_exits(&g),
            [
                (100, true),
                (0, false),
                (0, false),
                (101, true),
                (0, false),
                (109, true),
                (105, true)
            ]
        );
    }

    /// Waits until a forest row is the player's again: the effects over,
    /// nothing on task 0 but at most the hover's line (kind 2), the figure
    /// standing.
    fn settle_in_forest(g: &mut DunkleSchatten) {
        let mut frames = 0;
        loop {
            let standing = g.inner.walker.as_ref().is_some_and(|w| !w.walking);
            if !g.inner.effects.busy()
                && g.inner.player_linked()
                && standing
                && g.inner.tasks.t[0].kind <= 2
            {
                return;
            }
            crate::titles::Runtime::tick(g);
            frames += 1;
            assert!(frames < 6000, "the row must settle");
        }
    }

    /// Clicks the active hotspot that carries exit `id`, at a point no
    /// later hotspot covers, and plays until the location changes.
    fn click_exit(g: &mut DunkleSchatten, id: u16) -> i32 {
        let s = forest_exits(g)
            .iter()
            .position(|&(e, active)| e == id && active)
            .expect("the exit is on the picture");
        click_spot(g, s)
    }

    /// Clicks hotspot `s` of the room's table, at a point no later hotspot
    /// covers, and plays until the location changes.
    fn click_spot(g: &mut DunkleSchatten, s: usize) -> i32 {
        let h = g.inner.hotspots_rt[s].clone();
        let point = (h.y..h.y + h.h)
            .step_by(4)
            .flat_map(|y| (h.x..h.x + h.w).step_by(4).map(move |x| (x, y)))
            .find(|&(x, y)| {
                g.inner.mouse_move(x, y);
                g.inner.hotspot_index_under_pointer() == Some(s)
            })
            .expect("a point of the hotspot's own");
        crate::titles::Runtime::mouse_move(g, point.0, point.1);
        crate::titles::Runtime::tick(g);
        crate::titles::Runtime::click(g, true);
        let from = g.inner.location();
        let mut frames = 0;
        while g.inner.location() == from {
            crate::titles::Runtime::tick(g);
            frames += 1;
            assert!(frames < 4000, "the walk must leave through hotspot {s}");
        }
        assert!(frames > 1, "the click starts a walk, not a jump");
        g.inner.location()
    }

    /// Through the maze by clicks, as a player plays it, each click on a
    /// path of the one forest picture: from the Wald by its left-hand path
    /// (the picture's hotspot 0, exit 100) into row 30; there by the
    /// right-hand path (hotspot 3, which row 30 calls 101) to row 31; there
    /// by the middle path (hotspot 2, which row 31 calls 104) to row 34;
    /// and by the left-hand path out of the maze to the Ruine, row 18.
    /// Without the map's renaming the same clicks would lead from row 30
    /// to row 33 and from row 31 to row 33. Every step is a walk to the
    /// exit's node and through its twin, and each arrival snaps the figure
    /// onto the twin of the exit it came by and walks it in.
    #[test]
    fn the_maze_is_walked_by_clicks_from_the_wald_to_the_ruine() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 23, 100);
        settle_in_forest(&mut g);
        let mut path = vec![g.inner.location()];
        for (spot, exit) in [(0, 100), (3, 101), (2, 104), (0, 100)] {
            assert_eq!(forest_exits(&g)[spot], (exit, true));
            path.push(click_spot(&mut g, spot));
            assert_eq!(g.inner.entry_word, 0, "the arrival took the entry");
            if g.inner.location() != 18 {
                settle_in_forest(&mut g);
            }
        }
        assert_eq!(path, [23, 30, 31, 34, 18]);
    }

    /// The slot unload on the way from one forest row to the next gives
    /// every object the map bound its file id and its file place back
    /// (`0x875D`, `0x876B`, `0x8779`), and the entry it was bound to as its
    /// defaults (`0x8712`–`0x8751`); the hotspots their rectangles
    /// (`0x87AF`, `0x87BD`).
    #[test]
    fn the_slot_unload_between_forest_rows_gives_the_records_back() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 30, 100);
        settle_in_forest(&mut g);
        let slot = g.inner.rooms[20].as_ref().expect("slot 20");
        let file: Vec<_> = slot
            .room
            .overlays
            .iter()
            .map(|o| (o.id, i32::from(o.x), i32::from(o.y)))
            .collect();
        let rects: Vec<_> = slot
            .hotspots
            .spots
            .iter()
            .map(|h| (i32::from(h.x), i32::from(h.y)))
            .collect();
        let d6 = g.inner.object_index(0xD6).expect("D6h");
        assert_ne!(g.inner.objects[d6].x, file[d6].1, "row 30 moves D6h");
        // The door's gate has chosen row 31.
        g.inner.location_word = 31;
        g.inner.slot_release();
        let back: Vec<_> = g.inner.objects.iter().map(|o| (o.id, o.x, o.y)).collect();
        assert_eq!(back, file);
        let back: Vec<_> = g.inner.hotspots_rt.iter().map(|h| (h.x, h.y)).collect();
        assert_eq!(back, rects);
        let d = g.inner.objects[d6].defaults;
        assert_eq!((d.id, d.live, d.cel, d.dx, d.dy), (0xD6, 1, 0, 63, 0));
    }

    /// Between two forest rows the picture stays loaded: the slot unload
    /// gives every record the map bound back its id and its place
    /// (`0x86CE`), and the next row's map binds onto the records as they
    /// stand. Row 31 after row 30 is what row 31 is when entered afresh,
    /// but for what the rows leave alone — a scale planted on `D6h` before
    /// the walk is still there after it, and gone after a jump, which
    /// loads the slot afresh.
    #[test]
    fn between_two_forest_rows_the_picture_is_kept_and_rebound() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 31, 100);
        settle_in_forest(&mut g);
        let state = |g: &DunkleSchatten| -> Vec<_> {
            g.inner
                .objects
                .iter()
                .map(|o| (o.id, o.x, o.y, o.flags))
                .collect()
        };
        let fresh = state(&g);
        let fresh_exits = forest_exits(&g);
        crate::titles::Runtime::set_location_via(&mut g, 30, 100);
        settle_in_forest(&mut g);
        let d6 = g.inner.object_index(0xD6).expect("D6h");
        let file_scale = g.inner.objects[d6].scale;
        g.inner.objects[d6].scale = (0x123, 0x123);
        assert_eq!(click_exit(&mut g, 101), 31);
        settle_in_forest(&mut g);
        assert!(g.inner.picture_kept);
        assert_eq!(state(&g), fresh);
        assert_eq!(forest_exits(&g), fresh_exits);
        assert_eq!(
            g.inner.objects[d6].scale,
            (0x123, 0x123),
            "the records were kept"
        );
        crate::titles::Runtime::set_location_via(&mut g, 30, 100);
        settle_effects(&mut g);
        assert!(!g.inner.picture_kept);
        assert_eq!(g.inner.objects[d6].scale, file_scale, "a jump loads afresh");
    }

    /// The forest's song: entering the maze from the Ruine, a place of
    /// another kind, the entry plays the picture's song (`0x0ECA9`); from
    /// one maze row to the next nothing is stopped and nothing played —
    /// the leaving skips the fade (`0x0ED37`), the slot unload keeps the
    /// song for the map's bit 3 (`0x85ED`) and the entry clears its song
    /// (`0x0EC74`).
    #[test]
    fn the_forests_song_plays_on_from_row_to_row() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 18, 109);
        settle_in_forest(&mut g);
        g.inner.music.sent.clear();
        // The Ruine's left-hand Wald is its exit 102, to row 30.
        assert_eq!(click_exit(&mut g, 102), 30);
        settle_in_forest(&mut g);
        assert!(
            g.inner
                .music
                .sent
                .iter()
                .any(|r| matches!(r, parsec1994::Request::Play { .. })),
            "the forest's song starts: {:?}",
            g.inner.music.sent
        );
        g.inner.music.sent.clear();
        assert_eq!(click_exit(&mut g, 101), 31);
        settle_in_forest(&mut g);
        // Footsteps play on the way; nothing else reaches either driver —
        // the map's bit 2 keeps the effects as bit 3 keeps the song
        // (`0x85D4`).
        assert!(
            g.inner
                .music
                .sent
                .iter()
                .all(|r| matches!(r, parsec1994::Request::Effect { row: 0 | 1 })),
            "and plays on: {:?}",
            g.inner.music.sent
        );
    }

    /// The Ruine on the day its two figures talk in the background: the
    /// handler binds one of its scenes 0 to 2 to task 1 (`0x3C16`,
    /// `0CFDh:19A4h`), which plays its pages on its own and rolls the next
    /// when it is over (`0x0E381`).
    #[test]
    fn the_ruine_plays_its_background_scenes_on_task_1() {
        let mut g = ds1!();
        g.inner.sav_set(0x271D, 2);
        crate::titles::Runtime::set_location_via(&mut g, 18, 109);
        settle_effects(&mut g);
        let t = &g.inner.tasks.t[1];
        assert!(t.dialogue.is_some(), "a scene is bound to task 1");
        assert_eq!(t.range, (0, 2));
        assert_eq!(t.flags & 1, 1, "and rolls the next one");
        let mut pages = 0;
        for _ in 0..3000 {
            crate::titles::Runtime::tick(&mut g);
            pages += g
                .inner
                .events
                .drain(..)
                .filter(|e| matches!(e, Event::PageUp { task: 1, .. }))
                .count();
        }
        assert!(pages >= 2, "task 1 shows its pages, {pages} seen");
    }

    /// The map's rule reads the object the click chose from the engine's
    /// mode block (`[2891h]`, `0x610`) and leaves through exit 109 for its
    /// location — through the runtime, not only the trace stand-in.
    #[test]
    fn the_map_sends_the_player_to_the_chosen_place() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 1, 0x6D);
        settle_effects(&mut g);
        // Place object 205 stands for location 5, the Ralf; its scene byte
        // at DS:262Eh+CDh is zero in a fresh game, so the map leaves.
        g.inner.object1 = 0xCD;
        let answer = logic::rule_handler(&mut g.inner, (0x0060, 0x000A), 1);
        assert_eq!(answer, Some(1));
        assert_eq!((g.inner.entry_word, g.inner.location_word), (0x6D, 5));
    }

    /// A location keeps its objects' states between visits, through the
    /// 32-byte row the handler stores before it leaves (`0x9EF`) — also on
    /// the generic path a title without written logic takes.
    #[test]
    fn a_room_keeps_its_objects_between_visits_on_the_generic_path() {
        let mut g = ds1!();
        g.inner.set_location_via(2, 100);
        settle_effects(&mut g);
        // An object whose state the row keeps (`14DDh:068Ah` stores kinds
        // 0, 1 and 5), live: the Zimmer's toolbox, kind 0.
        let live = |g: &DunkleSchatten, id: u8| {
            g.inner
                .objects
                .iter()
                .find(|o| o.id == id)
                .map(|o| (o.kind, o.flags & 1))
        };
        assert_eq!(live(&g, 1), Some((0, 1)), "the toolbox, live");
        g.inner.object_hide(1);
        assert_eq!(live(&g, 1), Some((0, 0)));
        g.inner.set_location_via(3, 100);
        settle_effects(&mut g);
        g.inner.set_location_via(2, 100);
        settle_effects(&mut g);
        assert_eq!(
            live(&g, 1),
            Some((0, 0)),
            "hidden on the way out, hidden on the way back in"
        );
    }

    /// The opening without a click, the way the original's reference run
    /// goes: Karsten wakes, walks into the Küche, the choice menu comes up,
    /// its first line is taken — and the Küche is left through exit 109
    /// for location 1, the map. Answers the ticks the menu came up and the
    /// map was reached at.
    ///
    /// The menu comes up 4458 ticks (37.15 s) into the Zimmer: the slide
    /// into it (64 retraces, 110 ticks, before the first page is
    /// composed), the opening's pages, waking up with its reveal (116
    /// ticks), the transition out of the Zimmer (32 steps of three
    /// hundredths, 115 ticks) and the slide
    /// into the Küche (64 retraces, 110 ticks), and for every walk the
    /// logic waits out, the one pass the arrival comes after the leg's end
    /// (`0x0B4E9`, `0x0B32F`). The walks are at the factor 0Eh per edge:
    /// the options the program opens before its first location set it on
    /// their close, since the image's 0Dh is none of their three speeds
    /// (`0x12619`–`0x12633`). The reference run of the
    /// original clicked twice in the Zimmer, which cut a page short, so its
    /// timing is not a like-for-like check of this run.
    pub(super) fn play_opening_to_the_map(g: &mut DunkleSchatten) -> Option<(u32, u32)> {
        play_opening_answering(g, 1)
    }

    /// The same, with the Küche's answer `line` (1 to 3) clicked: the
    /// menu's rows are 13 apart from `y` 132.
    fn play_opening_answering(g: &mut DunkleSchatten, line: i32) -> Option<(u32, u32)> {
        let mut menu_at = None;
        for tick in 0..30_000 {
            crate::titles::Runtime::tick(g);
            let events = std::mem::take(&mut g.inner.events);
            if menu_at.is_none() && events.iter().any(|e| matches!(e, Event::MenuUp { .. })) {
                menu_at = Some(tick);
            }
            // The reference run clicks the first line a while after the
            // menu is up: the pointer on it, a tick, the button.
            if menu_at.is_some_and(|m| tick == m + 60) {
                crate::titles::Runtime::mouse_move(g, 60, 0x84 + 6 + 13 * (line - 1));
            }
            if menu_at.is_some_and(|m| tick == m + 61) {
                crate::titles::Runtime::click(g, true);
            }
            if g.inner.location() == 1 {
                return Some((menu_at?, tick));
            }
        }
        None
    }

    /// Every page tells the hook its slot with code 8 before its controls
    /// are scanned (`0CFDh:01BEh`, `0x0D1B7`), and the Küche's hook keeps
    /// it as the mother's next animation (`0xF70`), which the page's end
    /// plays while she is in the dialogue (`0xF78`). So the opening's
    /// first page, Karsten's (slot 0), puts her on animation 1 and her
    /// own (slot 1) on 2; the menu's code 8 with −1 puts her on 0. The
    /// figure follows the same codes: animation 5 when his first page's
    /// task starts, 4 on her page and on the menu. That is the order the
    /// original asks for them in, `0883h:1F82h` with 3 at the entry, then
    /// `0883h:2099h(200, 1)`, `20D9h(5)`, `20D9h(4)` with `2099h(200, 2)`,
    /// and `20D9h(4)` with `2099h(200, 0)`.
    #[test]
    fn the_mother_follows_the_pages_slots() {
        let mut g = ds1!();
        let mut asked = Vec::new();
        for _ in 0..30_000 {
            crate::titles::Runtime::tick(&mut g);
            let events = std::mem::take(&mut g.inner.events);
            let here = g.inner.location() == 3;
            let mut menu = false;
            for e in events {
                match e {
                    Event::ObjectAnim { object: 200, anim } if here => {
                        asked.push(format!("mother {anim}"));
                    }
                    Event::FigureAnim(k) if here => asked.push(format!("figure {k}")),
                    Event::MenuUp { .. } if here => menu = true,
                    _ => {}
                }
            }
            if menu {
                break;
            }
        }
        assert_eq!(
            asked,
            [
                "mother 3", "mother 1", "figure 5", "figure 4", "mother 2", "figure 4", "mother 0"
            ]
        );
    }

    /// A click on the map walks the player to the place and leaves for it.
    ///
    /// On the first day the Jugendzentrum (object 208, location 8) is one
    /// of the two places the story opens (`DS:26F6h+8`); the pointer over
    /// its hotspot names it, the click sends the player along walk graph 1
    /// with his footsteps, and the map's rule (`0x0060`) leaves through
    /// exit 109 when he arrives. Through the runtime, as a player plays it.
    #[test]
    fn a_click_on_the_map_walks_to_the_place() {
        let mut g = ds1!();
        play_opening_to_the_map(&mut g).expect("the map is reached");
        for _ in 0..400 {
            crate::titles::Runtime::tick(&mut g);
        }
        crate::titles::Runtime::mouse_move(&mut g, 120, 94);
        for _ in 0..30 {
            crate::titles::Runtime::tick(&mut g);
        }
        let named = g
            .inner
            .events
            .drain(..)
            .any(|e| matches!(e, Event::PageUp { ref text, .. } if text == "Jugendzentrum"));
        assert!(named, "the pointer names the place");
        crate::titles::Runtime::click(&mut g, true);
        let mut steps = 0;
        for _ in 0..3000 {
            crate::titles::Runtime::tick(&mut g);
            steps += g
                .inner
                .events
                .drain(..)
                .filter(|e| matches!(e, Event::Sample(_)))
                .count();
            if g.inner.location() != 1 {
                break;
            }
        }
        assert_eq!(g.inner.location(), 8);
        assert!(steps > 0, "the walk is heard");
    }

    /// Karsten's greeting in the Jugendzentrum is shown where he stands
    /// once he has walked in.
    ///
    /// The JZ's first arm plays scene 0 armed (`0x1B0D`), so its first
    /// page, `Moin, Klaus!`, is composed on entering — against the point
    /// Karsten has at the door — and waits. When the task is started the
    /// speaker has moved, and `0CFDh:04B4h` composes again from the page
    /// the box itself carries (`[bx+1DDCh]`, `0x0D50A`), which the
    /// compositor wrote as it began (`0x12743`). The task's own cursor is
    /// already past the page (`0x0D8C7`); composing from there would show
    /// the next page, or leave the old box at the door.
    #[test]
    fn the_greeting_in_the_jz_is_composed_again_where_the_speaker_stands() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 8, 0x6D);
        let armed_at = g.inner.tasks.t[0].point;
        let mut shown = false;
        for _ in 0..2000 {
            crate::titles::Runtime::tick(&mut g);
            if g.inner.caption_is_up() {
                shown = true;
                break;
            }
        }
        assert!(shown, "the greeting goes up");
        assert_eq!(g.inner.caption_text(), "\u{3}o\u{3}gMoin, Klaus!");
        let t = &g.inner.tasks.t[0];
        assert_ne!(t.point, armed_at, "Karsten has walked in");
        let page = motionvm_parsec_formats::to_cp437("\u{3}o\u{3}gMoin, Klaus!");
        let (want, _) = crate::titles::caption::Caption::compose(
            g.inner.font_for_test(),
            &page,
            t.point,
            t.style_word,
        );
        let b = t.box_.as_ref().expect("a box");
        assert_eq!((b.x, b.y, b.w, b.h), (want.x, want.y, want.w, want.h));
    }

    /// A click on the map lays the place's name again at the pointer.
    ///
    /// The hover text follows the hotspot, not the pointer: moved within
    /// the Jugendzentrum's rectangle it stays where it was laid, because
    /// the dispatcher's tail finds the same hotspot as last frame
    /// (`0x0FDE1`). A click forgets that hotspot first (`0x0FD94`:
    /// `[28A5h]` and `[28A3h]` set to `FFFFh`, the arm `cs:1479h` sends
    /// the button events to), so the tail lays the text again
    /// (`0CFDh:1810h`, `0x0FE8F`) at the pointer, ten rows lower here.
    #[test]
    fn a_click_lays_the_hover_text_again_at_the_pointer() {
        let mut g = ds1!();
        // The first day opens the Jugendzentrum (`DS:26F6h + 8`).
        g.inner.sav_set(0x26FE, 1);
        crate::titles::Runtime::set_location_via(&mut g, 1, 0x6D);
        let run = |g: &mut DunkleSchatten, n: usize| {
            for _ in 0..n {
                crate::titles::Runtime::tick(g);
            }
        };
        run(&mut g, 400);
        crate::titles::Runtime::mouse_move(&mut g, 120, 90);
        run(&mut g, 100);
        assert_eq!(g.inner.caption_text(), "Jugendzentrum");
        let laid = g.inner.tasks.t[0].box_.as_ref().map(|b| (b.x, b.y));
        crate::titles::Runtime::mouse_move(&mut g, 120, 100);
        run(&mut g, 100);
        assert_eq!(
            g.inner.tasks.t[0].box_.as_ref().map(|b| (b.x, b.y)),
            laid,
            "the same hotspot: the text stays"
        );
        g.inner.events.clear();
        crate::titles::Runtime::click(&mut g, true);
        run(&mut g, 1);
        assert!(
            g.inner
                .events
                .iter()
                .any(|e| matches!(e, Event::PageUp { text, .. } if text == "Jugendzentrum")),
            "the name is laid again"
        );
        let (x, y) = laid.expect("a box");
        assert_eq!(
            g.inner.tasks.t[0].box_.as_ref().map(|b| (b.x, b.y)),
            Some((x, y + 10)),
            "at the pointer, ten rows lower"
        );
    }

    /// The way out of a room plays in that room's colors, and the next
    /// room's come after it.
    ///
    /// The exit effect `10B8h:0210h` runs synchronously inside the leaving
    /// (`0x0ED4F`) and builds its mosaic's brightness from the DAC as it
    /// finds it; only then does the main loop enter the next location,
    /// whose setup `0EB9h:0001h` loads that room's banks through the entry
    /// pass (`14DDh:0293h`, `0x1535F`). So while the transition out of the
    /// Zimmer plays, the DAC's room range is still the Zimmer's, and the
    /// Küche's is there once the effects are over — with the DAC then
    /// what a fresh build for the Küche gives.
    #[test]
    fn the_exit_effect_plays_in_the_old_rooms_colors() {
        let mut g = ds1!();
        let range = |g: &DunkleSchatten| {
            let r = &g.inner.room().expect("a room").room;
            let off = usize::from(r.palette_start) * 3;
            (off, off + r.palette_count * 3)
        };
        let (a, b) = range(&g);
        let zimmer = g.inner.palette().raw[a..b].to_vec();
        crate::titles::Runtime::set_location_via(&mut g, 3, 100);
        assert!(g.inner.effects.busy(), "the transition is queued");
        let (a, b) = range(&g);
        let kueche = g.inner.room().expect("the Küche").room.palette_rgb.clone();
        assert_ne!(zimmer, kueche, "two rooms, two palettes");
        assert_eq!(
            g.inner.palette().raw[a..b],
            zimmer[..],
            "still the Zimmer's"
        );
        for _ in 0..40 {
            crate::titles::Runtime::tick(&mut g);
        }
        assert!(g.inner.effects.busy(), "the exit is playing");
        assert_eq!(
            g.inner.palette().raw[a..b],
            zimmer[..],
            "the Zimmer's colors under the mosaic"
        );
        settle_effects(&mut g);
        assert_eq!(
            g.inner.palette().raw[a..b],
            kueche[..],
            "the Küche's after it"
        );
        let shown = g.inner.palette().clone();
        g.inner.rebuild_dac();
        assert_eq!(
            shown.raw[..],
            g.inner.palette().raw[..],
            "and nothing else differs"
        );
    }

    /// The frame of a tick that leaves a room shows the room being left.
    ///
    /// The leaving presents its last frame (`0x0ED2C`, `10B8h:0009h` at
    /// `0x0ED8C`) and draws the exit effect over it at once (`0x0ED4F`);
    /// the main loop loads the next location only after that call returns
    /// (`0EB9h:0001h`). So no frame of the next room is ever on the screen
    /// before the effects that lead into it. Played from a fresh game: the
    /// Zimmer's opening ends in its script exit to the Küche.
    #[test]
    fn the_tick_that_leaves_a_room_shows_that_room() {
        let mut g = ds1!();
        let from = g.inner.location();
        let mut before = crate::titles::Runtime::render(&g);
        for _ in 0..20_000 {
            crate::titles::Runtime::tick(&mut g);
            if g.inner.location() != from {
                break;
            }
            before = crate::titles::Runtime::render(&g);
        }
        assert_ne!(g.inner.location(), from, "the opening leaves the Zimmer");
        assert!(g.inner.effects.busy(), "with its effects queued");
        let shown = crate::titles::Runtime::render(&g);
        // Pixels of the playfield in which two pictures differ.
        let apart = |a: &Framebuffer, b: &Framebuffer| {
            let field = 320 * 128;
            (a.pixels[..field].iter())
                .zip(&b.pixels[..field])
                .filter(|(x, y)| x != y)
                .count()
        };
        let next = g.inner.render_scene();
        assert!(apart(&next, &before) > 0, "two rooms, two pictures");
        assert_eq!(apart(&shown, &before), 0, "the room the last frame showed");
    }

    /// How many pixels of the status band carry one of the painter's inks
    /// (`0F4h`, `0FAh`, `0FCh`, each with the glyph's second shade).
    fn status_ink(g: &DunkleSchatten) -> usize {
        let fb = g.inner.render();
        (128..140)
            .flat_map(|y| (0..320).map(move |x| (x, y)))
            .filter(|&(x, y)| matches!(fb.get(x, y), Some(0xF4 | 0xF5 | 0xFA | 0xFB | 0xFC | 0xFD)))
            .count()
    }

    /// The status line is set four frames after it was asked for, and not
    /// before.
    ///
    /// The pointer onto the Jugendzentrum chooses it (`0EB9h:0A0Dh`),
    /// which asks for the line with `[28C3h] = 1` (`0x0F743`). The
    /// composer has one caller, the gate at the head of every frame's
    /// hand-round (`0x101D5`): it counts the age up and composes only once
    /// the age it found is above 3. So the band is empty for three frames
    /// and names the place on the fourth.
    #[test]
    fn the_status_line_is_set_through_its_gate() {
        let mut g = ds1!();
        g.inner.sav_set(0x26FE, 1);
        crate::titles::Runtime::set_location_via(&mut g, 1, 0x6D);
        for _ in 0..400 {
            crate::titles::Runtime::tick(&mut g);
        }
        assert_eq!(status_ink(&g), 0, "nothing chosen, nothing said");
        crate::titles::Runtime::mouse_move(&mut g, 120, 90);
        for frame in 1..=3 {
            crate::titles::Runtime::tick(&mut g);
            assert_eq!(status_ink(&g), 0, "frame {frame}: not yet");
        }
        crate::titles::Runtime::tick(&mut g);
        assert!(status_ink(&g) > 0, "the fourth frame sets the line");
        assert_eq!(g.inner.status_line, b"\x09Gehe zu \x09Jugendzentrum");
    }

    /// The status line is set in the inks its `%c` choose: red while the
    /// pointer names the place, yellow once it is clicked.
    ///
    /// The composer picks the `%c` by the mode (`0x10056`–`0x10092`, the
    /// arms of `cs:162Ch`): in mode 0 with something to do it is `09h`
    /// throughout, from mode 4 on `0Dh`. The painter's table `cs:04C2h`
    /// (`0x0C8B2`) turns `09h` into ink `0FAh` (`0x0C788`) and `0Dh` into
    /// `0F4h` (`0x0C79E`), the ink it starts with (`0x0C750`).
    #[test]
    fn the_status_line_takes_its_inks_from_the_mode() {
        let count = |g: &DunkleSchatten, inks: &[u8]| {
            let fb = g.inner.render();
            (128..140)
                .flat_map(|y| (0..320).map(move |x| (x, y)))
                .filter(|&(x, y)| fb.get(x, y).is_some_and(|p| inks.contains(&p)))
                .count()
        };
        let mut g = ds1!();
        g.inner.sav_set(0x26FE, 1);
        crate::titles::Runtime::set_location_via(&mut g, 1, 0x6D);
        for _ in 0..400 {
            crate::titles::Runtime::tick(&mut g);
        }
        crate::titles::Runtime::mouse_move(&mut g, 120, 90);
        for _ in 0..10 {
            crate::titles::Runtime::tick(&mut g);
        }
        assert_eq!((g.inner.mode, g.inner.two_object), (0, 2));
        assert!(count(&g, &[0xFA, 0xFB]) > 0, "named in 0FAh");
        assert_eq!(count(&g, &[0xF4, 0xF5]), 0, "and in nothing else");
        crate::titles::Runtime::click(&mut g, true);
        for _ in 0..10 {
            crate::titles::Runtime::tick(&mut g);
        }
        assert_eq!(g.inner.mode, 4, "clicked: on the way");
        assert!(count(&g, &[0xF4, 0xF5]) > 0, "now in 0F4h");
        assert_eq!(count(&g, &[0xFA, 0xFB]), 0);
    }

    /// While Karsten walks out of the Küche the status line stays as it
    /// was: empty.
    ///
    /// The script exit (`1586h:05E1h`, `0x15E08`–`0x15E17`) puts verb
    /// `0Ah` and mode 4 in place — what the composer would set as
    /// `Gehe durch` — and sets bit 0 of `[289Bh]`, which the gate tests
    /// (`0x101DC`, `test word [289Bh], 5`): nothing is composed while the
    /// machine walks, and the painter draws the line it holds.
    #[test]
    fn the_status_line_holds_while_the_figure_walks_out() {
        let mut g = ds1!();
        let mut menu_at = None;
        let mut leaving = false;
        let mut said = false;
        for tick in 0..30_000 {
            crate::titles::Runtime::tick(&mut g);
            let events = std::mem::take(&mut g.inner.events);
            if menu_at.is_none() && events.iter().any(|e| matches!(e, Event::MenuUp { .. })) {
                menu_at = Some(tick);
            }
            if menu_at.is_some_and(|m| tick == m + 60) {
                crate::titles::Runtime::mouse_move(&mut g, 60, 0x84 + 6);
            }
            if menu_at.is_some_and(|m| tick == m + 61) {
                crate::titles::Runtime::click(&mut g, true);
            }
            if events.iter().any(|e| matches!(e, Event::ScriptExit(109))) {
                leaving = true;
            }
            if g.inner.location() == 1 {
                break;
            }
            if leaving && !g.inner.effects.busy() {
                said |= g.inner.status().starts_with("Gehe durch");
                assert_eq!(status_ink(&g), 0, "tick {tick}: the band says nothing");
            }
        }
        assert!(leaving && said, "the walk out ran, under verb 0Ah");
        assert_eq!(g.inner.location(), 1);
    }

    #[test]
    fn the_opening_plays_through_the_kuche_to_the_map() {
        let mut g = ds1!();
        let (menu_at, map_at) = play_opening_to_the_map(&mut g).expect("the map is reached");
        assert_eq!(menu_at, 4458);
        assert_eq!(map_at, 9881);
        assert_eq!(g.inner.here_word, 1);
    }

    /// The main loop's globals are what the loop's own instructions name.
    ///
    /// `0x4FC0` reads `[1F4Ah]` to dispatch on and `0x4FD1`/`0x4FD4` compare
    /// it against `[1F4Ch]` afterwards; `0x4F3D` sets `[1F48h]`. Reading
    /// them instead of writing them down means a drifted site fails the
    /// load.
    #[test]
    fn the_main_loop_dispatches_on_the_globals_it_names() {
        let g = ds1!();
        assert_eq!(g.inner.locations.location, 0x1F4A);
        assert_eq!(g.text.entry, 0x1F48);
    }

    /// A wrong number asks for one of eight, and the text only points at
    /// it.
    ///
    /// `0x117D` counts the row at `DS:1060h`, `0x1189` asks the generator
    /// `1EB5h:0028h` for an index into it, and `0x119E` puts the answer in
    /// the substitution table at `DS:1689h` — all of it inside the
    /// Wohnzimmer's handler, on the way to one of its telephone scenes.
    /// Scene 12's *Nein, hier ist bestimmt nicht «0»* therefore carries
    /// `02 00` rather than a name, which is why the eight are in the
    /// program and not in the text — and why a fresh game's row still
    /// holds what the image ships.
    #[test]
    fn a_wrong_number_asks_for_one_of_eight() {
        let g = ds1!();
        assert_eq!(
            wrong_numbers(&g),
            g.text
                .wrong_numbers
                .iter()
                .map(|s| motionvm_parsec_formats::to_cp437(s))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            g.inner.substitutions,
            g.text
                .substitutions
                .iter()
                .map(|s| motionvm_parsec_formats::to_cp437(s))
                .collect::<Vec<_>>(),
            "nothing drawn at the start"
        );
        assert_eq!(
            g.text.wrong_numbers,
            [
                "der Kleintierzuchtverein",
                "Familie Penzelklöt",
                "das Amt für Denkmalschutz",
                "der Fanclub der Roten Dosen",
                "die Frau Klorbinger",
                "ihre \"liebe Großmutter\"",
                "das städtische Finanzamt",
                "die Weihnachtsmannvermittlung",
            ]
        );
    }

    /// Eleven speaker styles, and one rule for flattening what is under a
    /// caption that has no panel of its own.
    ///
    /// The styles are three bytes each at `2139h:0009h`, indexed by
    /// `di * 3 + 9` (`0x0D95A`); the flattening comes from the one call to
    /// the table builder, `(0C0h, 10h)` at `0x59CD`. Both are read by
    /// shape.
    #[test]
    fn a_caption_has_eleven_styles_and_one_flattening_rule() {
        let g = ds1!();
        assert_eq!(g.inner.styles.len(), 11);
        assert_eq!(g.inner.styles[0].ink, 248);
        assert_eq!(g.inner.styles[0].panel, 232);
        assert_eq!(g.inner.styles[1].panel, 238);
        assert_eq!(g.inner.flatten.to, 192);
        assert_eq!(g.inner.flatten.first, 192);
        assert_eq!(g.inner.flatten.count, 16);
    }

    /// A verb and a hotspot produce a caption, and while it is up a
    /// click only shortens it.
    ///
    /// Picking `Betrachte` and clicking the wardrobe answers with the
    /// wardrobe's own `texts[3]`; picking it and clicking something with
    /// nothing to say falls through to bank 999. A click while the text
    /// is up goes to the text (`0x0DFF7`): without a hold on the page it
    /// leaves at most `28h` ticks of it (`0x0E028`).
    #[test]
    fn a_verb_and_a_hotspot_put_a_caption_up() {
        let mut g = zimmer_later!();
        let spot = g
            .inner
            .room()
            .and_then(|r| {
                r.hotspots
                    .spots
                    .iter()
                    .find(|h| h.name == "Schrank")
                    .cloned()
            })
            .expect("the wardrobe");
        assert!(
            act(&mut g, 1, &spot).is_some(),
            "the click ends in a caption"
        );
        assert!(g.inner.caption_is_up(), "and it is up");
        assert!(
            g.inner.tasks.t[0].countdown > 0x28,
            "with more than 40 ticks to go"
        );
        assert!(g.inner.click(), "a second click is taken by the text");
        assert_eq!(g.inner.tasks.t[0].countdown, 0x28, "and leaves it 40 ticks");
        assert!(g.inner.caption_is_up(), "still up");

        // It takes itself away once its pages have been up long enough.
        for _ in 0..4000 {
            g.inner.tick();
            if !g.inner.caption_is_up() {
                break;
            }
        }
        assert!(!g.inner.caption_is_up(), "and then it is gone");
    }

    /// The shipped data at `DS:15FEh` and `DS:1602h` contains one item,
    /// `13h`; the original capture also shows its cel in the first slot.
    #[test]
    fn a_fresh_game_carries_the_original_inventory_entry() {
        let g = ds1!();
        assert!(g.inner.carried(19), "the original starts with item 19");
        for id in 1..23 {
            assert_eq!(g.inner.carried(id), id == 19, "initial item {id}");
        }
    }

    /// The strip's slots hold items, and an item names itself.
    ///
    /// The eight slots are widgets `11Eh` to `125h`, slot `n` holds what
    /// is carried at `scroll + n` (`0x0CAC0`), and the name comes out of
    /// the row at `DS:1000h` counting from one (`0x0F5AC`). A fresh game
    /// carries item 19. This test removes it to exercise an isolated
    /// two-item list without changing the original startup expectation.
    #[test]
    fn a_slot_holds_an_item_and_the_item_names_itself() {
        let mut g = zimmer_later!();
        g.inner.set_location_via(2, 0x7F);
        settle_effects(&mut g);
        g.inner.take_item(19);
        g.inner.give(1);
        g.inner.give(13);
        // The empty template is verb 0, so the line only says anything
        // once a verb is picked; `Betrachte` is 1.
        g.inner.verb = 1;

        // Slot 0 is the first thing carried, and the strip names it.
        let (x, y) = (0xC8 + 12, 0x92 + 12);
        g.inner.mouse_move(x, y);
        g.inner.tick();
        assert!(
            g.inner.status().contains("Werkzeugkasten"),
            "slot 0 should name the toolbox, got {:?}",
            g.inner.status()
        );

        // Slot 1 is the second, twenty-seven pixels along.
        g.inner.mouse_move(x + 0x1B, y);
        g.inner.tick();
        assert!(
            g.inner.status().contains("Schlüssel"),
            "slot 1 should name the key, got {:?}",
            g.inner.status()
        );

        // And an empty slot names nothing.
        g.inner.mouse_move(x + 2 * 0x1B, y);
        g.inner.tick();
        let line = g.inner.status();
        assert!(
            !line.contains("Werkzeugkasten") && !line.contains("Schlüssel"),
            "an empty slot names no item, got {line:?}"
        );

        // Scrolled on by one, slot 0 shows what slot 1 showed.
        g.inner.scroll_to(1);
        g.inner.mouse_move(x, y);
        g.inner.tick();
        assert!(
            g.inner.status().contains("Schlüssel"),
            "scrolled, slot 0 should name the key, got {:?}",
            g.inner.status()
        );
        g.inner.scroll_to(0);

        // `Betrachte` on a filled slot puts bank 2's description up.
        g.inner.mouse_move(x, y);
        assert!(g.inner.click(), "examining slot 0 says something");
        assert!(g.inner.caption_is_up());
        // Place `id - 1` of bank 2, which for the toolbox is its own line.
        assert!(
            g.inner.caption_text().contains("Werkzeugkasten"),
            "got {:?}",
            g.inner.caption_text()
        );
        for _ in 0..4000 {
            g.inner.tick();
            if !g.inner.caption_is_up() {
                break;
            }
        }

        // Four columns twenty-seven apart from 0C8h, and two rows at 92h
        // and 0ADh — the numbers `0x0C566` divides and multiplies.
        use crate::titles::parsec1994::Adventure;
        assert_eq!(Adventure::slot_origin(0), (0xC8, 0x92));
        assert_eq!(Adventure::slot_origin(3), (0xC8 + 3 * 0x1B, 0x92));
        assert_eq!(Adventure::slot_origin(4), (0xC8, 0xAD));
        assert_eq!(Adventure::slot_origin(7), (0xC8 + 3 * 0x1B, 0xAD));

        // The cel really lands where the drawing loop puts it: twenty-four
        // pixels of item at 0C8h, 92h, and nothing at the column before.
        let frame = g.inner.render();
        let bare = {
            let mut empty = ds1!();
            empty.inner.set_location_via(2, 0x7F);
            // The jump's effects hold the screen until they are over.
            settle_effects(&mut empty);
            empty.inner.take_item(19);
            empty.inner.render()
        };
        let (sx, sy) = (0xC8 + 12, 0x92 + 12);
        assert_ne!(
            frame.get(sx, sy),
            bare.get(sx, sy),
            "slot 0 should have an item in it"
        );
        assert_eq!(
            frame.get(sx - 0x1B, sy),
            bare.get(sx - 0x1B, sy),
            "and the column before it should be untouched"
        );
    }

    /// What a verb does to an object is a table, and the table is read.
    ///
    /// `0x51B6` hands the global one to its installer as `2140h:0050h`,
    /// and `0x13E41` walks it fourteen bytes at a time until the phase
    /// word is zero. The four rules that match *any* object are the
    /// generic answers, and they are the ones a click falls back on.
    #[test]
    fn the_global_interaction_table_is_thirty_three_rules() {
        let g = ds1!();
        let rules = &g.inner.global_rules;
        assert_eq!(rules.len(), 33);

        // Verb 1 is `Betrachte`, verb 4 `Benutze`, and both answer for
        // anything — object -1 — while acting on one object, phase bit 2.
        let generic = |verb: u16| {
            rules
                .iter()
                .find(|r| r.verb == verb && r.object == -1 && r.phase == 4)
                .unwrap_or_else(|| panic!("no generic rule for verb {verb}"))
        };
        assert_eq!(generic(1).object2, 0);
        assert_eq!(generic(4).object2, 0);

        // The two-object verbs carry phase bit 5 and name a second object.
        for verb in [5, 9] {
            let r = rules
                .iter()
                .find(|r| r.verb == verb && r.object == -1)
                .unwrap_or_else(|| panic!("no generic rule for verb {verb}"));
            assert_eq!(r.phase, 0x20, "verb {verb} acts on two objects");
            assert_eq!(r.object2, -1);
        }

        // And the object-specific ones are 1-based item numbers: 6 is the
        // pendant, 13 the key.
        for object in [6, 13] {
            assert!(
                rules.iter().any(|r| r.object == object),
                "no rule about object {object}"
            );
        }
    }

    /// A location is a row in a record table, not a case in a jump table.
    ///
    /// `mov ax, 204Bh` / `imul bx, bx, 34h` / `call far es:[bx+23h]` at
    /// `0x4FC7`–`0x4FCC`.
    #[test]
    fn a_location_is_a_row_of_fifty_two_bytes_with_a_far_handler() {
        let g = ds1!();
        assert_eq!(g.inner.locations.table, 0x204B);
        assert_eq!(g.inner.locations.stride, 0x34);
        assert_eq!(g.inner.locations.handler, 0x23);
        // The bound the program checks before it dispatches, not a walk.
        assert_eq!(g.inner.locations.count, 61);
    }

    /// Every slot of `DSROOM.DAT` that holds a playfield has a row in that
    /// table — which is what says a location number is a slot number.
    ///
    /// `DunkleSchatten::open` refuses an install where that does not hold,
    /// so reaching here at all is half the proof; the count is the other
    /// half. 60 slots, 28 with an `RDV1.00`.
    #[test]
    fn every_location_with_a_playfield_has_a_handler() {
        let g = ds1!();
        assert_eq!(g.inner.rooms.len(), 60);
        let rooms = g.inner.rooms.iter().filter(|r| r.is_some()).count();
        assert_eq!(rooms, 28);
        for (n, room) in g.inner.rooms.iter().enumerate() {
            if room.is_some() {
                assert!(
                    g.inner.locations.handler(&g.inner.image, n).is_some(),
                    "location {n}"
                );
            }
        }
    }

    /// A fresh game opens at location 2, entry `7Fh`.
    ///
    /// `0x4F2A` tests whether the loader left `[1F4Ah]` at zero and, when
    /// it did, `0x4F37`/`0x4F3D` write those two.
    #[test]
    fn a_fresh_game_starts_at_location_two() {
        let g = ds1!();
        assert_eq!(g.text.start_location, 2);
        assert_eq!(g.text.start_entry, 0x7F);
        assert_eq!(g.inner.location(), 2);
    }

    /// Sixteen status-line templates, indexed by `[288Dh]`, and every one
    /// of them carries ink controls.
    ///
    /// `0x10124` reads the verb and `0x1012F` pushes the far pointer out of
    /// the row at `DS:1084`. The `%c` in `%cBetrachte %c%s` is the ink
    /// control pinned here.
    #[test]
    fn the_status_templates_carry_ink_controls() {
        let g = ds1!();
        assert_eq!(g.text.verb_index, 0x288D);
        assert_eq!(g.inner.templates.len(), 16);
        assert_eq!(g.inner.templates[0], "");
        assert_eq!(g.inner.templates[1], "%cBetrachte %c%s");
        assert_eq!(g.inner.templates[5], "%cBenutze %c%s%c mit %c%s");
        for (i, t) in g.inner.templates.iter().enumerate().skip(1) {
            assert!(t.contains("%c"), "template {i} {t:?} has no ink control");
        }
    }

    /// Every location the record table reaches carries its own name.
    ///
    /// `0x1016B` takes `es:[bx+27h]` off the record of `[1F4Ch]` and
    /// `0x10191` formats it as `[ %s (%02d) ]`. Thirty rows carry a
    /// pointer and the rest carry none — which is the same thirty the
    /// name-carrying half of the table covers, and it is why locations
    /// 30 and up read as somewhere else in kind.
    ///
    /// Location 20's name is the empty string, not a missing one. A
    /// reader that flattened the two would lose the distinction the file
    /// makes.
    #[test]
    fn every_named_location_names_itself_in_its_own_record() {
        let g = ds1!();
        assert_eq!(g.inner.locations.name, 0x27);
        let names = &g.inner.locations.names;
        assert_eq!(names[2].as_deref(), Some("Zimmer"));
        assert_eq!(names[3].as_deref(), Some("Küche"));
        assert_eq!(names[6].as_deref(), Some("Straße"));
        assert_eq!(names[16].as_deref(), Some("Krankenhaus"));
        assert_eq!(names[20].as_deref(), Some(""));
        assert_eq!(names[29].as_deref(), Some("F3"));
        let named = names.iter().filter(|n| n.is_some()).count();
        assert_eq!(named, 30);
        assert!(names[30..].iter().all(Option::is_none));
        // The engine hands out a name only where there is one to hand.
        assert_eq!(g.inner.location_name(), Some("Zimmer"));
    }

    /// An entry puts the player on a node, or on nothing.
    ///
    /// The four ids `0x0EF4B` tries, in order: the entry plus `7D0h`,
    /// the entry, then 127, then the first exit of the location's own row
    /// (`0x0EFF3`). `Karte` carries node 127 at `(180, 60)`, scale `70h`,
    /// layer 0, and no hotspot there ends in 127, so the facing is 2 and
    /// the standing cel is block 2's last — six phases here, so
    /// seven-cel blocks and index 20.
    ///
    /// **The opening location carries none of the first three**: `Zimmer`
    /// has one exit and its nodes are 100, 64, 201, 2100 and the rest,
    /// and 127 is not among them. So the fourth id puts the player on the
    /// door's node, 100 at `(259, 94)`, facing away from it: the door's
    /// hotspot carries facing 1 in its mask and the snap turns that round
    /// (`0x0B6E3`).
    #[test]
    fn an_entry_stands_the_player_where_the_room_has_a_node_for_it() {
        let mut g = ds1!();
        let w = g.inner.walker.as_ref().expect("the entry stands one up");
        assert_eq!((w.x, w.y), (259, 94));
        assert_eq!(w.scale, 283);
        assert_eq!(w.dest_id, 100, "on the first exit's node");
        assert_eq!(w.facing(), 3);

        // A jump without an entry is not an arrival: the record stays,
        // but out of every chain, and is not drawn.
        g.inner.set_location(2);
        let w = g.inner.walker.as_ref().expect("the record stays");
        assert!(w.node.is_none() && w.layer.is_none());

        g.inner.set_location_via(1, 0x7F);

        settle_effects(&mut g);
        let w = g.inner.walker.as_ref().expect("Karte carries node 127");
        assert_eq!((w.x, w.y), (180, 60));
        assert_eq!(w.scale, 112);
        assert_eq!(w.layer, Some(0));
        assert_eq!(w.cel(), (20, false));

        g.inner.set_location(1);
        assert!(g.inner.walker.as_ref().is_some_and(|w| w.layer.is_none()));
    }

    /// A room draws the objects its chain holds, and only the live ones.
    ///
    /// Bit 0 of an object's `+2` is what says so, and the shipped rooms
    /// are authored to it: the Zimmer has four objects and one of them
    /// is up, and every one of the Stammtisch's seven is — its regulars.
    #[test]
    fn a_room_draws_the_objects_that_are_up() {
        let g = ds1!();
        let counts = |n: usize| {
            let room = g.inner.rooms[n].as_ref().expect("a playfield");
            let all = room.room.overlays.len();
            let live = room
                .room
                .overlays
                .iter()
                .filter(|o| o.flags & 1 == 1)
                .count();
            (all, live)
        };
        assert_eq!(counts(2), (4, 1), "Zimmer");
        assert_eq!(counts(22), (7, 7), "Stammtisch");
        // Every object in every shipped room names the room's own cel
        // table — no room of the game's reaches for another bundle — and every
        // live one has a picture there. They are counted rather than
        // asserted one by one, because a silent miss is what a wrong
        // bundle would look like too.
        let (mut live, mut drawable) = (0usize, 0usize);
        for room in g.inner.rooms.iter().flatten() {
            for o in room.room.overlays.iter().filter(|o| o.flags & 1 == 1) {
                assert_eq!(
                    o.bundle, 0,
                    "object {} draws from bundle {}",
                    o.id, o.bundle
                );
                live += 1;
                drawable += usize::from(o.cel.is_some());
            }
        }
        assert_eq!((live, drawable), (94, 94));
    }

    /// A verb comes off the strip and the status line says so.
    ///
    /// The button at `(6,146)` is widget `103h`, so it picks verb 3 —
    /// `%cNimm/Gib %c%s` — and choosing the Küche's front door fills the
    /// `%s` with its name.
    #[test]
    fn a_verb_button_picks_its_verb_and_the_line_says_so() {
        let mut g = ds1!();
        g.inner.set_location_via(3, 100);
        settle_effects(&mut g);
        assert_eq!(g.inner.verb, 0, "a fresh location has no verb");
        assert_eq!(g.inner.status(), "", "and so nothing to say");

        g.inner.mouse_move(20, 155);
        assert!(g.inner.click(), "the button did not take");
        assert_eq!(g.inner.verb, 3);

        g.inner.mouse_move(88, 60);
        assert_eq!(
            g.inner.hotspot_under_pointer().map(|h| h.name.as_str()),
            Some("Haustür")
        );
        // Nothing is chosen until the dispatcher looks, so the line has
        // the verb and the composer's stand-in for a missing name,
        // `DS:053Fh` (`0x100CB`).
        assert_eq!(g.inner.status(), "Nimm/Gib  ??? ");
        // The dispatcher chooses what the pointer is over, which stores
        // its name (`0x0F6A0`) and asks the phase-2 rules: the door's
        // answers 5 with a verb of its own, `Nimm`, which takes `[288Dh]`
        // while the verb picked stays in `[289Dh]`.
        g.inner.tick();
        assert_eq!(
            (g.inner.verb, g.inner.verb_chosen, g.inner.two_object),
            (12, 3, 4)
        );
        assert_eq!(g.inner.status(), "Nimm Haustür");

        // A scroll arrow is not a verb, and leaves the one that is.
        g.inner.mouse_move(310, 150);
        assert!(!g.inner.click());
        assert_eq!(g.inner.verb_chosen, 3);
    }

    /// The map is data: a location's exits are two byte lists in its own
    /// record, and a click on the hotspot that carries an exit's id goes
    /// where the list says.
    ///
    /// `0x05436` looks an id up in the list at `+9` and `0x05471` takes
    /// the destination from `+11h`. Eight slots.
    #[test]
    fn a_click_on_the_door_of_the_bedroom_reaches_the_kitchen() {
        let mut g = zimmer_later!();
        assert_eq!(g.inner.locations.exits, 9);
        assert_eq!(g.inner.locations.destinations, 0x11);
        // The Zimmer has one exit: id 100 to location 3, the Küche.
        assert_eq!(g.inner.locations.exits(&g.inner.image, 2), vec![(100, 3)]);
        // Its `Tür` is at (254,29) 20×65 and reads `In die Küche`.
        g.inner.mouse_move(260, 60);
        let spot = g.inner.hotspot_under_pointer().expect("the door");
        assert_eq!(spot.name, "Tür");
        assert_eq!(spot.verb_mask & 0xFF, 100);
        // The figure stands on the door's node already, so the click
        // turns it round and the machine goes on from the arrival: the
        // walk verb's rule, the door's own rule — it opens — and the
        // door's gate, which walks the figure through the twin node and
        // ends the visit.
        assert!(g.inner.player_linked(), "the entry stood one up");
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.verb, 10, "over a door the verb is the walk verb");
        assert!(g.inner.click(), "the click went nowhere");
        assert_eq!(
            g.inner.mode, 7,
            "standing there already, the arrival ran at once"
        );
        let mut frames = 0;
        let mut opened = false;
        while g.inner.location() == 2 {
            crate::titles::Runtime::tick(&mut g);
            frames += 1;
            assert!(frames < 2000, "the walk must reach the door");
            opened |= g.inner.events.drain(..).any(|e| {
                e == Event::ObjectAnim {
                    object: 100,
                    anim: 1,
                }
            });
        }
        assert!(opened, "the door's animation 1 was asked for");
        assert!(frames > 1, "and the walk through took more than an instant");
        assert_eq!(g.inner.location(), 3);
        assert_eq!(g.inner.location_name(), Some("Küche"));

        // Arriving on entry 100 puts the figure on the twin node and
        // walks it in, so the way back is walked too: the Küche's
        // Treppenhaus is exit 100. The Küche's own arm for this state
        // plays a scene first, and a click while it is up is the text's.
        assert!(g.inner.walker.as_ref().is_some_and(|w| w.walking));
        let mut frames = 0;
        while !g.inner.player_linked() || g.inner.tasks.t[0].kind != 0 {
            crate::titles::Runtime::tick(&mut g);
            frames += 1;
            assert!(frames < 6000, "the Küche's scene must end");
        }
        g.inner.mouse_move(132, 60);
        assert_eq!(
            g.inner.hotspot_under_pointer().map(|h| h.name.as_str()),
            Some("Treppenhaus")
        );
        assert!(g.inner.click());
        assert_eq!(g.inner.location(), 3, "the click starts a walk, not a jump");
        let mut frames = 0;
        while g.inner.location() == 3 {
            crate::titles::Runtime::tick(&mut g);
            frames += 1;
            assert!(frames < 2000, "the walk must reach the door");
        }
        assert_eq!(g.inner.location(), 2);
        assert!(frames > 1, "and it must take more than an instant");
    }

    /// A bare click on the wardrobe looks at it.
    ///
    /// With no verb picked, the pointer over an object runs the phase-1
    /// rules (`0x0F636`), and the one for any object makes `Betrachte` the
    /// verb (`1586h:0356h`); the click then walks the figure to the
    /// wardrobe's own node — the hotspot's mask names it, facing 1
    /// (`0x0F276`) — and the arrival reads its `texts[3]`.
    #[test]
    fn a_bare_click_on_the_wardrobe_looks_at_it() {
        let mut g = zimmer_later!();
        g.inner.mouse_move(290, 40);
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(
            g.inner.hotspot_under_pointer().map(|h| h.name.as_str()),
            Some("Schrank")
        );
        assert_eq!(g.inner.verb, 1, "Betrachte by default");
        assert_eq!(g.inner.status(), "Betrachte Schrank");
        assert!(g.inner.click());
        assert_eq!(g.inner.mode, 4, "walking there first");
        assert!(g.inner.walker.as_ref().is_some_and(|w| w.walking));
        let mut frames = 0;
        while !text_up(&g) {
            crate::titles::Runtime::tick(&mut g);
            frames += 1;
            assert!(frames < 2000, "the walk must end in a look");
        }
        assert!(frames > 1);
        assert_eq!(g.inner.player_node_id(), 201, "on the wardrobe's node");
        assert_eq!(g.inner.walker.as_ref().map(|w| w.facing()), Some(1));
        assert!(g.inner.caption_text().contains("Schrank") || !g.inner.caption_text().is_empty());
        assert_eq!(g.inner.location(), 2);
    }

    /// No exit id matches more than one hotspot.
    ///
    /// This is what makes matching a hotspot against the exit table safe:
    /// over all 28 rooms, a click can never
    /// resolve to two doors. Some exits have no hotspot at all — those are
    /// the ones the original reaches by script rather than by clicking.
    #[test]
    fn no_exit_id_matches_two_hotspots_in_one_location() {
        let g = ds1!();
        let (mut matched, mut unmatched) = (0, 0);
        for (n, room) in g.inner.rooms.iter().enumerate() {
            let Some(room) = room else { continue };
            for (id, _) in g.inner.locations.exits(&g.inner.image, n) {
                let hits = room
                    .hotspots
                    .spots
                    .iter()
                    .filter(|h| h.active == 1 && u8::try_from(h.verb_mask & 0xFF).unwrap() == id)
                    .count();
                assert!(hits <= 1, "location {n} exit {id} matches {hits} hotspots");
                if hits == 1 {
                    matched += 1;
                } else {
                    unmatched += 1;
                }
            }
        }
        assert_eq!((matched, unmatched), (32, 17));
    }

    /// The entry function fills the subject slots, and it fills them the
    /// way its own loop counts.
    ///
    /// Slot 0 is the player (`0x0EDE7` hands the walker record `DS:1693h`
    /// to the `2000h` setter), slots 1 to 9 are the record's nine object
    /// positions in order — holes included, because the loop counts past a
    /// zero rather than closing up — and slot 10 is the literal point
    /// (160, 64) with code `53h` (`0x0EE58`).
    ///
    /// The Küche is the case worth pinning: its record lists one object,
    /// 200, the `Mutter`. She lands in slot **1**, not slot 0.
    #[test]
    fn the_entry_fills_the_subject_slots() {
        use crate::titles::subject::Subject;
        let mut g = ds1!();
        g.inner.set_location_via(3, 100);
        settle_effects(&mut g);

        assert_eq!(
            g.inner.subjects.get(0).map(|s| s.subject),
            Some(Subject::Player),
            "slot 0 is the player"
        );
        assert_eq!(
            g.inner.subjects.get(1).map(|s| s.subject),
            Some(Subject::Object(200)),
            "slot 1 is the first object of the record"
        );
        for n in 2..=9 {
            assert_eq!(
                g.inner.subjects.get(n).map(|s| s.subject),
                Some(Subject::Nothing),
                "slot {n} has nothing in it"
            );
        }
        let screen = g.inner.subjects.get(10).expect("slot 10");
        assert_eq!(screen.subject, Subject::Point(160, 64));
        assert_eq!(screen.code, 0x53, "kind 0, payload 53h");
        assert_eq!(screen.payload(), 0x53);
        assert_eq!(g.inner.subjects.point(&g.inner, 10), Some((160, 64)));

        // The Wohnzimmer lists three, and they keep their order.
        g.inner.set_location_via(4, 0x7F);
        settle_effects(&mut g);
        let kinds: Vec<Subject> = (1..=3)
            .map(|n| g.inner.subjects.get(n).unwrap().subject)
            .collect();
        assert_eq!(
            kinds,
            vec![
                Subject::Object(200),
                Subject::Object(130),
                Subject::Object(202)
            ]
        );

        // The Zimmer lists none, so every slot but 0 and 10 is empty.
        g.inner.set_location_via(2, 0x7F);
        settle_effects(&mut g);
        for n in 1..=9 {
            assert_eq!(
                g.inner.subjects.get(n).map(|s| s.subject),
                Some(Subject::Nothing),
                "the Zimmer names no object for slot {n}"
            );
        }
    }

    /// A slot resolves to a point the way `0x0D290` resolves one.
    ///
    /// The Küche's `Mutter` is an object of the room, so the resolver
    /// reads her record: the point at `+4`/`+6`, the cel scaled by `+0Eh`
    /// and `+10h`, and the alignment at `+3` moving it. That is a
    /// different move from the drawer's — the drawer centres align 1 on
    /// both axes, the resolver only lifts y — so the two are pinned apart.
    #[test]
    fn a_slot_resolves_to_a_point() {
        use crate::titles::subject::{align_point, scaled};
        let mut g = ds1!();
        g.inner.set_location_via(3, 100);
        settle_effects(&mut g);

        let mother = g
            .inner
            .room()
            .expect("the Küche")
            .room
            .overlays
            .iter()
            .find(|o| o.id == 200)
            .cloned()
            .expect("object 200");
        let cel = mother.cel.as_ref().expect("she has a picture");
        let w = scaled(i32::from(cel.width), mother.scale.0);
        let h = scaled(i32::from(cel.height), mother.scale.1);
        let want = align_point(mother.align, i32::from(mother.x), i32::from(mother.y), w, h);
        assert_eq!(g.inner.subjects.point(&g.inner, 1), Some(want));

        // The three arms, each one instruction in the original.
        assert_eq!(align_point(0, 100, 50, 30, 40), (115, 50), "x += w/2");
        assert_eq!(align_point(1, 100, 50, 30, 40), (100, 30), "y -= h/2");
        assert_eq!(align_point(2, 100, 50, 30, 40), (100, 10), "y -= h");
        // And the scale is skipped for 0 and 100h, halved for 80h.
        assert_eq!(scaled(46, 0), 46);
        assert_eq!(scaled(46, 0x100), 46);
        assert_eq!(scaled(46, 0x80), 23);

        // A location with a figure resolves slot 0 to the top of its head.
        let p = g.inner.player_point().expect("the Küche stands her up");
        assert_eq!(g.inner.subjects.point(&g.inner, 0), Some(p));
        let w = g.inner.walker.as_ref().unwrap();
        assert!(p.1 < w.y, "the head is above the feet");
    }

    /// The bedroom tail calls `0A9Fh:0F42h` at `0x09A7`, which removes
    /// the figure from its draw chain via `0x0AA1F`. At `0x0AA78` the
    /// layer head becomes the figure's successor; `0x0AB1A` similarly
    /// bypasses a figure in the middle. Neither path clears its node.
    #[test]
    fn the_bedroom_handler_removes_the_sleeping_figure_from_the_draw_chain() {
        use crate::titles::subject::Subject;
        let g = ds1!();
        assert_eq!(g.inner.sav_byte(STATE), 0, "a fresh game is in state 0");
        assert_eq!(g.inner.sav_byte(BED), 2, "and the opening arm marked it");
        assert_eq!(
            g.inner.subjects.get(0).map(|s| s.subject),
            Some(Subject::Object(0xC8)),
            "slot 0 is the object the opening is about, not the player"
        );
        let w = g.inner.walker.as_ref().expect("the figure record remains");
        assert_eq!(w.layer, None, "the sleeping figure is not drawn");
        assert_eq!((w.x, w.y, w.dest_id), (259, 94, 100));
        assert!(w.node.is_some(), "removing a draw link preserves the node");

        // Calling the shared entry alone bypasses the Küche handler:
        // slot 0 stays the player.
        let mut g = ds1!();
        g.inner.set_location_via(3, 100);
        settle_effects(&mut g);
        assert_eq!(
            g.inner.subjects.get(0).map(|s| s.subject),
            Some(Subject::Player)
        );
    }

    /// `Nimm` answers, and the button that says it is not the verb the
    /// rules speak of.
    ///
    /// The strip's six buttons are `103h 104h 102h 106h 105h 101h`, so
    /// one of them is verb **3** — `%cNimm/Gib %c%s`, one button for two
    /// actions. The rule table has no generic verb 3 that answers; it has
    /// rule 28, whose handler asks whether the object is carried and
    /// writes **9** (`Gib … an`) or **0Ch** (`Nimm`) into the verb before
    /// anything else looks at it. Resolving that is what makes the button
    /// do something.
    #[test]
    fn take_and_give_are_one_button_and_two_verbs() {
        let mut g = zimmer_later!();
        let spot = g
            .inner
            .room()
            .and_then(|r| r.hotspots.spots.iter().find(|h| h.name == "Bett").cloned())
            .expect("the bed");
        // With the button picked, the pointer over the bed runs the
        // phase-2 rules, and rule 28 writes the verb it means.
        g.inner.mode = 2;
        g.inner.verb = 3;
        g.inner.verb_chosen = 3;
        let (x, y) = middle(&spot);
        g.inner.mouse_move(x, y);
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.verb, 12, "nothing carried: Nimm");
        // Carrying it turns the same button into Gib.
        g.inner.give(200);
        g.inner.mouse_move(0, 0);
        crate::titles::Runtime::tick(&mut g);
        g.inner.verb = 3;
        g.inner.mouse_move(x, y);
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.verb, 9, "carried: Gib");
        g.inner.take_item(200);

        let said = act(&mut g, 3, &spot).expect("Nimm on the bed says something");
        assert!(
            said.contains("Möbelpacker"),
            "the bed's own texts[5], got {said:?}"
        );

        // Something with nothing of its own to say falls back on the roll:
        // one of bank 999's three ways of refusing to take a thing.
        let spot = g
            .inner
            .room()
            .and_then(|r| {
                r.hotspots
                    .spots
                    .iter()
                    .find(|h| {
                        h.texts[5].is_empty() && h.active == 1 && (h.verb_mask & 0xFF) >= 0x17
                    })
                    .cloned()
            })
            .expect("a hotspot with no Nimm text of its own");
        let said = act(&mut g, 3, &spot)
            .expect("and it still says something")
            .replace('\n', " ");
        // The caption carries the line's own ink control and breaks, so
        // the check is on a fragment each refusal has to itself.
        assert!(
            [
                "hier nehmen will",
                "lassen wir mal lieber",
                "will ich nicht"
            ]
            .iter()
            .any(|r| said.contains(r)),
            "one of bank 999's three refusals, got {said:?}"
        );

        // And it is exactly those three, not a range that happens to
        // cover them: twenty rolls stay inside it.
        for _ in 0..20 {
            let said = act(&mut g, 3, &spot).expect("a refusal").replace('\n', " ");
            assert!(
                [
                    "hier nehmen will",
                    "lassen wir mal lieber",
                    "will ich nicht"
                ]
                .iter()
                .any(|r| said.contains(r)),
                "the roll left 9..=11: {said:?}"
            );
        }
    }

    /// A refusal with nothing of its own to say is drawn from bank 999,
    /// and `Nimm`'s is **rolled** out of three.
    ///
    /// `0x15AB9` asks the generator for 9..=11 and complements it, so a
    /// place is picked rather than fixed. The other verbs have fixed
    /// places, and every one of them reads back as the right refusal for
    /// its verb — which is what says the table is read correctly.
    #[test]
    fn die_rueckfaelle_kommen_aus_bank_999() {
        let g = ds1!();
        let place = |n: usize| {
            g.inner
                .bank999_place_for_test(n)
                .unwrap_or_else(|| panic!("place {n}"))
        };
        // The shipped lines carry their own breaks, so the check is on
        // the halves either side of one.
        let has = |n: usize, s: &str| place(n).replace('\n', " ").contains(s);
        assert!(has(16, "Ich kann nichts Besonderes erkennen."), "Betrachte");
        assert!(has(3, "ziemlich ödes Gespräch"), "Sprich mit");
        assert!(has(6, "Das funktioniert leider nicht!"), "Benutze");
        assert!(has(8, "Das nutzt nichts."), "Benutze … mit");
        assert!(has(5, "nur anderen Leuten geben!"), "Gib … an");
        // And Nimm's three ways of saying no, which is why it rolls.
        assert!(has(9, "das hier nehmen will"));
        assert!(has(10, "Das lassen wir mal lieber."));
        assert!(has(11, "Das will ich nicht."));
    }

    /// Every verb reads the hotspot slot its own rule reads, and falls
    /// back on the place its own rule names.
    ///
    /// Run over every hotspot of the opening room, through the same road a
    /// click takes. That is what pins the two tables at once: which slot
    /// belongs to which verb (`+1Eh`, `+22h`, `+26h`, `+2Ah` in the four
    /// handlers) and which bank-999 place each verb refuses with.
    #[test]
    fn every_verb_reads_its_slot_and_falls_back_on_its_place() {
        let mut g = zimmer_later!();
        let spots: Vec<_> = g
            .inner
            .room()
            .expect("the Zimmer")
            .hotspots
            .spots
            .iter()
            .filter(|h| h.active == 1)
            .cloned()
            .collect();
        assert!(spots.len() > 3, "the Zimmer has hotspots to try");

        // verb, its own slot, and the place it refuses with.
        let cases: &[(u8, usize, &[usize])] = &[
            (1, 3, &[16]),
            (2, 4, &[3, 4]),
            (4, 2, &[6, 7]),
            (12, 5, &[9, 10, 11]),
        ];
        let places: Vec<String> = (0..20)
            .map(|n| g.inner.bank999_place_for_test(n).unwrap_or_default())
            .collect();

        // Pairs the location claims for itself go through its own rule
        // and answer with whatever that names, so they are not this
        // test's business.
        let claimed: Vec<(u16, i16)> = g
            .inner
            .locations
            .rules(&g.inner.image, 2)
            .iter()
            .map(|r| (r.verb, r.object))
            .collect();

        let mut own = 0;
        let mut fell = 0;
        for spot in &spots {
            let object = num::signed(spot.verb_mask & 0xFF);
            for &(verb, slot, refusals) in cases {
                if claimed
                    .iter()
                    .any(|&(v, o)| (v == u16::from(verb) || v == 0) && o == object)
                {
                    continue;
                }
                let Some(said) = act(&mut g, verb, spot) else {
                    continue;
                };
                // The caption shows the first page; a record may run on.
                let said = flat(&said);
                let want_own = &spot.texts[slot];
                if !want_own.is_empty() {
                    assert!(
                        flat(want_own).starts_with(&said),
                        "verb {verb} on {:?} should read its slot {slot}: {said:?}",
                        spot.name
                    );
                    own += 1;
                } else {
                    assert!(
                        refusals
                            .iter()
                            .any(|&n| flat(&places[n]).starts_with(&said)),
                        "verb {verb} on {:?} refused with {said:?}, not one of {refusals:?}",
                        spot.name
                    );
                    fell += 1;
                }
            }
        }
        assert!(
            own > 0 && fell > 0,
            "both roads were taken: {own} own, {fell} refusals"
        );
    }

    /// Nothing in the shipped game is taken by the **generic** rule: no
    /// `Nimm` reaches its take arm.
    ///
    /// Rule 24's take arm is built and faithful: an object below `17h`
    /// that is found and live has its live bit cleared and its number put
    /// in the player's hands. But no hotspot in any of the twenty-eight
    /// rooms reaches it. Every takeable thing is claimed by a rule of its
    /// **location's own**, which is searched first — the Zimmer's
    /// `Werkzeugkasten` is verb 12 on object 1 with the handler
    /// `0089:0069` — and every one of those handlers is a scene load.
    ///
    /// This test is the survey that says so, over every room, so that the
    /// claim is held and not only stated.
    #[test]
    fn no_object_is_taken_by_the_generic_rule() {
        let g = ds1!();
        let mut reachable = 0;
        let mut claimed = 0;
        for loc in 0..g.inner.rooms.len() {
            let Some(room) = g.inner.rooms[loc].as_ref() else {
                continue;
            };
            let own: Vec<(u16, i16)> = g
                .inner
                .locations
                .rules(&g.inner.image, loc)
                .iter()
                .map(|r| (r.verb, r.object))
                .collect();
            for h in room.hotspots.spots.iter().filter(|h| h.active == 1) {
                let id = u8::try_from(h.verb_mask & 0xFF).unwrap();
                if id >= 0x17 || !h.texts[5].is_empty() {
                    continue;
                }
                if !room
                    .room
                    .overlays
                    .iter()
                    .any(|o| o.id == id && o.flags & 1 == 1)
                {
                    continue;
                }
                if own
                    .iter()
                    .any(|&(v, o)| (v == 12 || v == 0) && o == i16::from(id))
                {
                    claimed += 1;
                } else {
                    reachable += 1;
                }
            }
        }
        assert_eq!(reachable, 0, "no hotspot reaches the generic take");
        assert!(claimed > 0, "and at least one is claimed by its location");
    }

    /// A location's own rule answers out of the location's own bank.
    ///
    /// The Zimmer's rule for `Nimm` on the toolbox (`0089:0069`) is
    /// `szene_lade(3)`, and a non-negative scene is a place in the bank the
    /// room loader keeps at `[2855h]` — resource 3 of the slot, which the
    /// loader takes as the fourth of six far pointers (`0x7B0D`, `0x7B14`).
    /// The Zimmer's holds ten places, and the third names the toolbox.
    #[test]
    fn a_location_rule_answers_out_of_the_locations_own_bank() {
        let mut g = zimmer_later!();
        let zimmer = g.inner.own_banks[2]
            .as_ref()
            .expect("the Zimmer keeps a bank");
        assert_eq!(zimmer.len(), 10, "the Zimmer keeps ten places");
        assert!(
            g.inner.own_banks[3].as_ref().is_some_and(|b| !b.is_empty()),
            "and the Küche keeps some too"
        );

        let spot = g
            .inner
            .room()
            .and_then(|r| {
                r.hotspots
                    .spots
                    .iter()
                    .find(|h| h.name == "Werkzeugkasten")
                    .cloned()
            })
            .expect("the toolbox");
        let said = act(&mut g, 3, &spot).expect("the location's own rule answered");
        assert!(
            said.contains("Werkzeugkasten"),
            "scene 3 is the toolbox line, got {said:?}"
        );
    }

    /// A scene number past the bank plays the programmers' joke.
    ///
    /// `0x0D5EA` compares the number against the count in `[1EEAh]`; past
    /// it the pointer stays null, and `0x0D5FD` takes bank 999's place
    /// `13h` — *Diese Programmierer mal wieder!* Whether a shipped rule
    /// ever names such a number is not established; the resolver's answer
    /// for one is.
    #[test]
    fn a_scene_past_the_bank_plays_the_programmers_joke() {
        let g = ds1!();
        let joke = g.inner.bank999_place_for_test(0x13).expect("place 19");
        assert!(joke.contains("Grr"), "place 19 is the joke, got {joke:?}");
        let past = g.inner.own_banks[2].as_ref().map_or(0, Bank::len);
        let text = g
            .inner
            .scene_text_for_test(i16::try_from(past).unwrap())
            .expect("the joke plays");
        assert!(text.contains("Grr"), "got {text:?}");
        let inside = g.inner.scene_text_for_test(3).expect("scene 3 is a place");
        assert!(inside.contains("Werkzeugkasten"), "got {inside:?}");
    }

    /// A second click on the same verb button parks it, and a parked verb
    /// lets a click through again.
    ///
    /// This is the mode word, and it is the whole of what a click means.
    /// A fresh location is mode 0 because the entry function clears the
    /// block the verb index heads. Picking a verb goes to mode 2. Picking
    /// the **same** one again goes to mode 1 — and mode 1 is not on the
    /// object road's list of answers (`0x0F5F4` takes 0, 2 and 3), so a
    /// click on a hotspot does nothing there.
    ///
    /// Parking is not un-picking: the verb stays and the status line goes
    /// on naming it. Only mode 0 draws no line at all (`0EB9:14D9`), and
    /// the way back to mode 0 is to leave the location.
    #[test]
    fn a_second_click_on_the_same_verb_parks_it() {
        let mut g = zimmer_later!();
        assert_eq!(g.inner.mode, 0, "a fresh location is mode 0");
        assert_eq!(g.inner.verb, 0);

        // The first button is `103h`, so it picks verb 3.
        g.inner.mouse_move(20, 155);
        assert!(g.inner.click());
        assert_eq!((g.inner.mode, g.inner.verb), (2, 3), "picked");

        // The same button again parks it.
        g.inner.mouse_move(20, 155);
        assert!(g.inner.click());
        assert_eq!((g.inner.mode, g.inner.verb), (1, 3), "parked, verb kept");

        // The parked state lasts while the pointer is on the strip: over
        // the room the dispatcher resets it (`0x0FE1E`), and a click there
        // is a bare click.
        let bett = g
            .inner
            .room()
            .and_then(|r| r.hotspots.spots.iter().find(|h| h.name == "Bett").cloned())
            .expect("the bed");
        g.inner.mouse_move(middle(&bett).0, middle(&bett).1);
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.mode, 0, "the pointer over the room ends mode 1");
        assert_eq!(g.inner.verb, 1, "and the bed's class means Betrachte");

        // Arriving somewhere puts the machine back to mode 0.
        crate::titles::Runtime::set_location_via(&mut g, 3, 100);
        settle_effects(&mut g);
        assert_eq!(
            (g.inner.mode, g.inner.verb),
            (0, 0),
            "a new location resets"
        );
        // The Küche's own arm plays a scene on arrival, and the strip
        // takes no click while a text is up.
        let mut frames = 0;
        while g.inner.tasks.t[0].kind != 0 {
            crate::titles::Runtime::tick(&mut g);
            frames += 1;
            assert!(frames < 6000, "the Küche's scene must end");
        }

        // A third click on a parked verb takes it again: 1 is one of the
        // three modes the take-a-verb arm answers.
        g.inner.mouse_move(20, 155);
        assert!(g.inner.click());
        assert_eq!(g.inner.mode, 2);
        g.inner.mouse_move(20, 155);
        assert!(g.inner.click());
        assert_eq!(g.inner.mode, 1);
        g.inner.mouse_move(20, 155);
        assert!(g.inner.click());
        assert_eq!(g.inner.mode, 2, "and back again");
    }

    /// The strip's rectangle is the program's, not this engine's
    /// arithmetic.
    ///
    /// `0x0D012` pushes 72, 320, 128, 0 and calls — 72 rows at row 128,
    /// reaching the bottom of the screen — and `PANELS.DAT` resource 0 is
    /// exactly that picture, so it goes on the area's first row. Inside
    /// that area the program marks a 12-row band at 128 (`0x0C875`) and a
    /// 60-row strip at 140 (`0x0C7E3`).
    #[test]
    fn the_strip_sits_where_the_program_reserves_it() {
        let g = ds1!();
        assert_eq!(g.inner.panel_y, 128);
        assert_eq!((g.inner.panel.width, g.inner.panel.height), (320, 72));
    }

    /// `0x0EEF0`/`0x0ECF4` bracket entry with panel owner 4. The
    /// original refresh releases it on the third draw pass, not at entry.
    #[test]
    fn entry_panel_release_is_delayed_and_rendering_does_not_advance_it() {
        let mut g = ds1!();
        // Enter without invoking the title handler that immediately claims task 0.
        g.inner.set_location_via(3, 100);
        settle_effects(&mut g);
        assert_eq!(g.inner.status_dim & 4, 4);
        let lookup = g.inner.flatten;
        let raw = g.inner.banks[0][2..].to_vec();
        for _ in 0..5 {
            let _ = g.inner.render();
        }
        assert_eq!(g.inner.status_dim & 4, 4);
        g.inner.tick();
        g.inner.tick();
        assert_eq!(g.inner.status_dim & 4, 4);
        g.inner.tick();
        assert_eq!(g.inner.status_dim, 0);
        assert_eq!(&g.inner.palette().raw[480..576], raw);
        assert_eq!(
            g.inner.flatten, lookup,
            "raw palette selection must not update flattening"
        );
        let room = &g.inner.room().unwrap().room;
        assert_eq!((room.palette_start, room.palette_count), (0, 128));
    }

    /// The restored ramp is `3,7,..,63` through row zero (divisor 20),
    /// not the transition's `0,4,..,60`. The capture has RGB8 (12,12,12)
    /// in the dimmed status band, and RGB8 (12,4,4) in the panel.
    #[test]
    fn opening_dialogue_uses_the_restored_ramp_and_separate_panel_colors() {
        let mut g = ds1!();
        assert_eq!(
            g.text.panel_curves,
            [[[3, 30], [1, 25], [1, 25]], [[0, 8]; 3]]
        );
        assert_eq!(g.text.gray_ramp.0, 192);
        assert_eq!(&g.text.gray_ramp.1[..3], &[3, 3, 3]);
        assert_eq!(
            g.text
                .gray_ramp
                .1
                .as_chunks::<3>()
                .0
                .iter()
                .map(|c| c[0])
                .collect::<Vec<_>>(),
            [3, 7, 11, 15, 19, 22, 24, 26, 28, 30, 31, 33, 34, 35, 37, 38]
        );
        for _ in 0..1440 {
            crate::titles::Runtime::tick(&mut g);
        }
        assert_eq!(g.inner.palette().rgb8(192), [12, 12, 12]);
        assert_eq!(g.inner.palette().rgb8(160), [12, 4, 4]);
        assert_eq!(
            g.inner.flatten.map(160),
            192,
            "use base RGB, not tinted RGB"
        );
        let frame = g.inner.render();
        for y in 128..140 {
            for x in 0..320 {
                assert_eq!(
                    g.inner.palette().rgb8(frame.get(x, y).unwrap()),
                    [12, 12, 12]
                );
            }
        }
    }

    /// The banks the location's own palette is covered with: the strip's at
    /// 160, the pointer's at 224, the player's at 128.
    #[test]
    fn three_palette_banks_go_over_the_locations_own() {
        let g = ds1!();
        let starts: Vec<u8> = g.inner.banks.iter().map(|b| b[0]).collect();
        assert_eq!(starts, vec![160, 224, 128]);
    }

    /// A tick is a 120th of a second.
    ///
    /// The timer the library installs counts the elapsed PIT cycles into
    /// the record at `DS:1C74h`, and the counter the frame pass reads,
    /// `[1C76h]`, advances once every `26D6h` = 9942 cycles (`0x19F7F`).
    /// 9942 cycles of 1 193 182 a second is 8 332 341 nanoseconds.
    #[test]
    fn a_tick_is_a_hundred_and_twentieth_of_a_second() {
        let g = ds1!();
        assert_eq!(
            crate::titles::Runtime::frame_duration(&g),
            Duration::from_nanos(8_332_341)
        );
    }

    /// The exit search runs fifteen slots and the overlap is inert.
    ///
    /// `04E3h:05D9h` compares the entry against `es:[bx+9+dx]` for `dx`
    /// from 0 to 14 (`0x545C`, `cmp dx, 0Fh`), which reaches seven bytes
    /// into the destination list that starts at `+11h`. A destination is a
    /// location number below 61, so the overlap can only ever match an
    /// exit id of 60 or less — and no row carries one.
    #[test]
    fn the_exit_search_runs_fifteen_slots_and_the_overlap_is_inert() {
        let g = ds1!();
        for row in 0..g.inner.locations.count {
            for (id, dest) in g.inner.locations.exits(&g.inner.image, row) {
                assert!(
                    id > 60,
                    "row {row}: exit id {id} would collide with a destination"
                );
                assert!(usize::from(dest) < g.inner.locations.count);
            }
        }
    }

    /// A page is held for the compositor's own count, which is one less
    /// than its characters and lines.
    ///
    /// `1269h:0082h` adds each line's characters (`0x1277C`) and one per
    /// line (`0x127A5`), then takes one off before it answers (`0x12861`,
    /// `dec di`); `0x0D871` multiplies that by `[167Fh]`, 8. The opening's
    /// first page, `* Schnarch! *` with its two ink bytes, is one line of
    /// 15: `120 + 8 × 15 + (10 × 15 >> 5)` = 244 ticks, where a count of
    /// 16 would hold it eight ticks longer. The page is composed when the
    /// entry effect is over (`0x0ED02`).
    #[test]
    fn a_page_is_held_for_the_compositors_count_less_one() {
        let mut g = ds1!();
        settle_effects(&mut g);
        let up = g.inner.events.drain(..).find_map(|e| match e {
            Event::PageUp { task: 0, text } => Some(text),
            _ => None,
        });
        assert_eq!(up.as_deref(), Some("\u{1}\u{1}* Schnarch! *"));
        assert_eq!(
            g.inner.tasks.t[0].countdown,
            120 + 8 * 15 + ((10 * 15) >> 5)
        );
    }

    /// The entry's effect is synchronous in the original: `0EB9h:00B8h`
    /// calls it (`0x0EC7F`) and goes on to the pointer and the tasks'
    /// first tick (`0x0ED02`) only when it returns. So the Zimmer's first
    /// page is composed — and the hook hears its code 9, which arms the
    /// task again and zeroes `[1C82h]` — on the tick the slide ends, not
    /// before it plays; nothing of the opening's text is set while it does.
    #[test]
    fn the_entry_goes_on_to_its_first_page_when_its_effect_is_over() {
        let mut g = ds1!();
        assert!(g.inner.effects.busy(), "the slide into the Zimmer");
        let mut ended = None;
        let mut composed = None;
        for tick in 1..=400 {
            let busy = g.inner.effects.busy();
            crate::titles::Runtime::tick(&mut g);
            if busy && !g.inner.effects.busy() {
                ended = Some(tick);
            }
            for e in std::mem::take(&mut g.inner.events) {
                if let Event::PageUp { task: 0, .. } = e {
                    composed.get_or_insert(tick);
                }
            }
            if composed.is_some() {
                break;
            }
        }
        assert_eq!(ended, Some(110), "64 retraces");
        assert_eq!(composed, ended, "the first page when the slide is over");
    }

    /// The opening plays page by page, and leaves through the door.
    ///
    /// The Zimmer's arm loads scene 0 and arms it (`0x952`, `0x960`). The
    /// entry effect (`10B8:0CAB`, 64 retraces) plays first, and only when
    /// it is over does the entry go on to the tasks' first tick
    /// (`0CFDh:1B89h(1)`, `0x0ED02`): that composes the first page on tick
    /// 110, and its code 9 — the Zimmer's hook arms the task again and
    /// zeroes `[1C82h]` (`0x0A3A`) — starts the second from there. The page
    /// is not shown — an armed task is not painted (`0x0D5B0`) — until the
    /// handler's own check starts it (`0x9CD`, `[1C82h] > 64h`, which is
    /// tick 231). A page counts down `120 + 8 × (chars + lines − 1) + 10 ×
    /// chars / 32` ticks (`0x0D865`, the composer's `dec di` at `0x12861`)
    /// from the frame that starts it, so `* Schnarch! *` with its two ink
    /// bytes — 15 characters, 244 ticks — gives way on tick 474. The
    /// hook arms the task again at every page's end while the opening is
    /// on, so the yawn waits its second too and shows on tick 595. Its
    /// `03 0A` control, event 10,
    /// is the waking up: it ends the opening before the page's end is
    /// reached, so the third page shows at once, the bed is emptied and
    /// the figure stands at node 200 facing front (`0x0C0E`).
    ///
    /// The last page opens with `03 78`, event 120: the script exit
    /// (`1586h:05E1h`) puts the machine into mode 7, and once the text is
    /// over the gate walks the figure to the door's node, the arrival
    /// runs the walk verb's rule, that hands the door — not live — to the
    /// seventh verb's rule, which opens it, and the door's gate walks the
    /// figure through the twin node and ends the visit in the Küche.
    ///
    /// Waking up plays the reveal (`10B8:03BA`, `0x0C3D`): 24 steps of four
    /// hundredths, 96 hundredths in all, during which nothing else runs —
    /// everything after it comes 116 ticks later than it would without.
    ///
    /// The door takes its time: its animation 1 reports code 8 on the
    /// frame pass of the tick the rule ran on, shows three frames ten
    /// ticks apart, and reports code 2 on the twenty-ninth tick after —
    /// the first frame's wait having lost the tick that ran the countdown
    /// out — so the machine, which waits for that code in mode 8, asks
    /// the gate on the thirtieth. The door closes once the figure has
    /// left its node, and the visit ends on the twin node fifty-five
    /// ticks after the walk began, as it did before the door was built.
    #[test]
    fn the_opening_plays_page_by_page_and_leaves_through_the_door() {
        let mut g = ds1!();
        assert!(!g.inner.caption_is_up(), "armed, not yet shown");
        let mut pages = Vec::new();
        let mut events = Vec::new();
        let mut ups = Vec::new();
        let mut downs = Vec::new();
        let mut was_up = false;
        let mut at_bed = None;
        let mut left = None;
        for tick in 1..=3600 {
            crate::titles::Runtime::tick(&mut g);
            if g.inner.location() != 2 {
                left = Some(tick);
                break;
            }
            let up = g.inner.caption_is_up();
            if up && !was_up {
                ups.push(tick);
            }
            if !up && was_up {
                downs.push(tick);
            }
            was_up = up;
            for e in g.inner.events.drain(..) {
                match e {
                    Event::PageUp { task: 0, text } => pages.push((tick, text)),
                    Event::Script { n, .. } => events.push(n),
                    Event::ObjectHidden(200) => {
                        let w = g.inner.walker.as_ref().expect("a figure");
                        at_bed = Some((tick, w.x, w.y, w.dest_id, w.facing()));
                    }
                    other => events_note(tick, other),
                }
            }
        }
        let texts: Vec<&str> = pages.iter().map(|(_, p)| p.as_str()).collect();
        assert!(texts[0].contains("* Schnarch! *"), "{texts:?}");
        assert!(texts[1].contains("* Gähn! *"), "{texts:?}");
        assert!(
            texts[2].contains("Mist, ich hab total verpennt."),
            "{texts:?}"
        );
        assert!(texts[3].contains("Dabei fangen wir heute an"), "{texts:?}");
        assert!(texts[4].contains("Ich glaub, ich mach mich"), "{texts:?}");
        let prepared: Vec<i32> = pages.iter().map(|&(t, _)| t).collect();
        assert_eq!(prepared, [110, 474, 822, 1556, 2208]);
        assert_eq!(ups, [231, 595], "shown a second after each arming");
        assert_eq!(
            downs,
            [474, 2785],
            "dark while armed, and when the text is over"
        );
        assert_eq!(events, [12, 10, 102, 120]);
        assert_eq!(at_bed, Some((822, 168, 87, 200, 2)), "up, by the bed");
        assert_eq!(left, Some(3082));
        assert_eq!(g.inner.location(), 3, "the door leads to the Küche");
        let w = g.inner.walker.as_ref().expect("and the figure is there");
        assert_eq!((w.x, w.y), (108, 75), "on the Küche's twin node 2100");
        assert!(w.walking, "walking in");
    }

    /// The opening's first page carries the mode-4 indicator: the text
    /// asks for cue 4 (`0x0E4BE`), three task passes later the setter
    /// copies `02D4h` into the slot (`0x0E1D1`), and the pointer — shown
    /// by the entry's end because hover is on (`0x0ECA4`) — walks the
    /// eight cels of resources 7 to 14, centered on the mouse. The page is
    /// up on tick 231, a second after the entry effect is over.
    #[test]
    fn the_opening_first_page_shows_the_mode_4_indicator() {
        let mut g = ds1!();
        let mut first_up = None;
        for tick in 1..=400 {
            crate::titles::Runtime::tick(&mut g);
            if g.inner.caption_is_up() {
                first_up = Some(tick);
                break;
            }
        }
        assert_eq!(first_up, Some(231), "the first page is up");
        let c = g
            .inner
            .cursor
            .as_ref()
            .expect("Dunkle Schatten installs the pointer");
        assert_eq!(c.mode, 4);
        assert!(c.visible(), "hover is on in the Zimmer");
        assert_eq!(c.current, 0, "the slot's descriptor, not the timeout's");
        let (at, d) = c.shown_descriptor_for_test();
        assert_eq!((at, d.flags, d.base, d.phases), (0x0334, 0x002F, 6, 8));
        let (cel, x, y) = c.sprite().expect("drawn");
        assert!(
            (6..=13).contains(&cel),
            "resources 7 to 14, got {}",
            cel + 1
        );
        let first = g.inner.pointer_cel(6).expect("resource 7");
        let (w, h) = (i32::from(first.width), i32::from(first.height));
        assert_eq!((x, y), (160 - (w >> 1), 100 - (h >> 1)));
        let pic = g.inner.pointer_cel(cel).expect("the cel");
        let frame = g.inner.render();
        let mut drawn = 0;
        for sy in 0..i32::from(pic.height) {
            for sx in 0..i32::from(pic.width) {
                let p = pic.pixels[usize::try_from(sy * i32::from(pic.width) + sx).unwrap()];
                if p != 0 {
                    assert_eq!(frame.get(x + sx, y + sy), Some(p));
                    drawn += 1;
                }
            }
        }
        assert!(drawn > 0);
        // And it walks while the page is up (to tick 481): the generator
        // moves and more than one of the eight cels shows.
        let seed = c.seed;
        let mut cels = std::collections::BTreeSet::new();
        for _ in 232..=481 {
            crate::titles::Runtime::tick(&mut g);
            let c = g.inner.cursor.as_ref().unwrap();
            assert_eq!((c.mode, c.current), (4, 0));
            cels.insert(c.sprite().expect("drawn").0);
        }
        assert_ne!(g.inner.cursor.as_ref().unwrap().seed, seed);
        assert!(cels.len() > 2, "{cels:?}");
        assert!(cels.iter().all(|n| (6..=13).contains(n)), "{cels:?}");
    }

    /// The calls the opening makes that this engine only notes, with the
    /// tick each came on.
    fn events_note(tick: i32, e: Event) {
        match (tick, e) {
            (
                1,
                Event::ObjectAnim {
                    object: 200,
                    anim: 0,
                },
            )
            | (
                474,
                Event::ObjectAnim {
                    object: 200,
                    anim: 1,
                },
            )
            | (822, Event::Effect(0x10B8, 0x03BA))
            | (822 | 1556 | 2208, Event::FigureAnim(7))
            | (822 | 2785, Event::Sample(2))
            | (2208, Event::ScriptExit(100))
            | (1, Event::ObjectShown(200))
            | (
                2992,
                Event::ObjectAnim {
                    object: 100,
                    anim: 1,
                },
            )
            | (
                2992,
                Event::AnimCode {
                    object: 100,
                    code: 8,
                },
            )
            | (2992, Event::Sample(3))
            | (
                3021,
                Event::AnimCode {
                    object: 100,
                    code: 2,
                },
            )
            | (
                3022,
                Event::ObjectAnim {
                    object: 100,
                    anim: 0,
                },
            )
            | (
                3061,
                Event::AnimCode {
                    object: 100,
                    code: 9,
                },
            ) => {}
            (
                _,
                Event::Hook { .. }
                | Event::LocationHook { .. }
                | Event::Cue(_)
                | Event::SceneBound { .. }
                | Event::Reset(_)
                // The footsteps of a walk (`0x0B5EE`, `0x0B5F2`).
                | Event::Sample(0 | 1),
            ) => {}
            (tick, other) => panic!("unexpected at {tick}: {other:?}"),
        }
    }

    /// The Küche's opening: Karsten walks in and greets his mother, she
    /// answers, and three replies go up as a menu.
    ///
    /// The Küche's state-0 arm (`0x0E51`) walks the figure to the mother
    /// and loads scene 0: `Moin, Mutter!`, then her line, then records 2
    /// to 4 — a choice list, since each has a speaker nibble of 0. The
    /// choice arm (`0x0DB06`) takes the list out of `[1E36h]`, the record
    /// before, and with three survivors builds the menu: three one-line
    /// choices, each on its own row from 132 down in steps of 13
    /// (`DS:04F8h`). The pointer's row highlights a choice (`0x0E061`),
    /// the arrow keys move it round, a click takes it (`0x0E0D5`), and
    /// `0CFDh:0EFEh` plays the record chosen as the player's own line,
    /// after which its answer follows. While the menu is up the last line
    /// is not painted (`0x0E6E4`) — the menu is.
    #[test]
    fn the_kitchen_puts_three_answers_up_as_a_menu_and_plays_the_one_chosen() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 3, 100);
        settle_effects(&mut g);
        g.inner.events.clear();
        let mut menu_at = None;
        for tick in 1..=2000 {
            crate::titles::Runtime::tick(&mut g);
            let lines = g.inner.events.drain(..).find_map(|e| match e {
                Event::MenuUp { lines } => Some(lines),
                _ => None,
            });
            if let Some(lines) = lines {
                menu_at = Some((tick, lines));
                break;
            }
        }
        let (tick, lines) = menu_at.expect("the menu goes up");
        assert_eq!(tick, 1152);
        assert_eq!(
            lines,
            vec![
                (1, "Ich hab den Wecker total überhört!".to_string()),
                (2, "Auf dürren und wackligen Beinen!".to_string()),
                (3, "Guter Spruch für dein Alter, Mutti!".to_string()),
            ]
        );
        assert_eq!(g.inner.tasks.t[0].kind, 5);
        assert!(!g.inner.caption_is_up(), "the box gives way to the menu");
        assert!(g.inner.menu.up);
        assert_eq!(g.inner.menu.highlight, 0, "nothing highlighted yet");
        assert_eq!(
            g.inner.palette().rgb8(160),
            [0, 0, 0],
            "choice palette variant 1"
        );
        let frame = g.inner.render();
        assert!(
            (132..140).any(|y| (0..320).any(|x| frame.get(x, y) == Some(248))),
            "the menu's first line must keep its ink over the status band"
        );

        // The pointer's row.
        g.inner.mouse_move(100, 132 + 13 + 5);
        assert_eq!(g.inner.menu.highlight, 2);
        g.inner.mouse_move(100, 100);
        assert_eq!(g.inner.menu.highlight, 0, "off the menu, none");
        g.inner.mouse_move(300, 132 + 2 * 13);
        assert_eq!(g.inner.menu.highlight, 3);
        g.inner.mouse_move(300, 132 + 3 * 13);
        assert_eq!(g.inner.menu.highlight, 0, "a row with no line is none");

        // The keys go round.
        crate::titles::Runtime::key(&mut g, 9);
        assert_eq!(g.inner.menu.highlight, 1, "past none, the first");
        crate::titles::Runtime::key(&mut g, 0x10F);
        assert_eq!(g.inner.menu.highlight, 3, "before the first, the last");
        crate::titles::Runtime::key(&mut g, 9);
        assert_eq!(g.inner.menu.highlight, 1);

        // A click on the third takes it.
        g.inner.mouse_move(50, 132 + 2 * 13 + 12);
        assert_eq!(g.inner.menu.highlight, 3);
        assert!(g.inner.click());
        assert!(!g.inner.menu.up, "the menu is down");
        assert_eq!(g.inner.tasks.t[0].kind, 4, "and a line plays");
        let mut spoken = Vec::new();
        for e in g.inner.events.drain(..) {
            match e {
                Event::ChoiceTaken(rec) => spoken.push(format!("record {rec}")),
                Event::PageUp { task: 0, text } => spoken.push(text),
                _ => {}
            }
        }
        assert_eq!(
            spoken,
            ["record 4", "Guter Spruch für dein Alter, Mutti!"],
            "the third choice is record 4, spoken as the player's line"
        );
        // Her answer comes when the line is over.
        let mut next = None;
        for _ in 0..2000 {
            crate::titles::Runtime::tick(&mut g);
            next = g.inner.events.drain(..).find_map(|e| match e {
                Event::PageUp { task: 0, text } => Some(text),
                _ => None,
            });
            if next.is_some() {
                break;
            }
        }
        assert_eq!(
            next.as_deref(),
            Some("Schlechter Anblick für\ndein Alter, mein Sohn!")
        );
    }

    /// Over the menu the pointer takes the menu region's shape: its record
    /// (`2139:002A`) says shape 1, the descriptor at `[3B4h]` = `0254h`,
    /// resource 2 centered (`0x186A0`). A key moves the mouse to the right
    /// end of the highlighted choice (`0x0E15F`), which is inside the
    /// region; leaving the region gives the mode's pointer back
    /// (`0x1870E`).
    #[test]
    fn over_the_menu_the_pointer_is_the_regions_shape() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 3, 100);
        let mut up = false;
        for _ in 1..=2000 {
            crate::titles::Runtime::tick(&mut g);
            if g.inner.menu.up {
                up = true;
                break;
            }
        }
        assert!(up, "the menu goes up");
        // The mother's line asked for mode 3 (`0x0DE43`): resource 6, the
        // pointer a DOSBox-X recording shows with the menu up and the mouse
        // above it.
        let c = g.inner.cursor.as_ref().unwrap();
        assert_eq!((c.mode, c.current), (3, 0));
        assert_eq!(c.sprite().map(|s| s.0), Some(5));
        crate::titles::Runtime::key(&mut g, 9);
        assert_eq!(g.inner.menu.highlight, 1);
        // Choice 1 is line 0: y = 132 + (0 + 1) * 13 / 2, x = 5 + 310 - 3.
        assert_eq!(g.inner.mouse, (312, 138));
        for _ in 0..3 {
            crate::titles::Runtime::tick(&mut g);
        }
        let c = g.inner.cursor.as_ref().unwrap();
        assert_eq!(c.current, 0x80, "region shape 1");
        let (at, _) = c.shown_descriptor_for_test();
        assert_eq!(at, 0x0254);
        let (cel, x, y) = c.sprite().expect("shown");
        assert_eq!(cel, 1, "resource 2");
        let pic = g.inner.pointer_cel(1).unwrap();
        assert_eq!(
            (x, y),
            (
                312 - i32::from(pic.width >> 1),
                138 - i32::from(pic.height >> 1)
            )
        );
        g.inner.mouse_move(100, 100);
        for _ in 0..3 {
            crate::titles::Runtime::tick(&mut g);
        }
        assert_eq!(g.inner.cursor.as_ref().unwrap().current, 0);
        // A choice taken: the region goes, the player's line asks for mode
        // 4 (`0x0DDFE`), applied three task passes on — the ball the
        // recording shows once the menu is gone.
        g.inner.mouse_move(50, 132 + 2);
        assert!(g.inner.click());
        assert!(!g.inner.cursor.as_ref().unwrap().menu_registered);
        for _ in 0..4 {
            crate::titles::Runtime::tick(&mut g);
        }
        let c = g.inner.cursor.as_ref().unwrap();
        assert_eq!((c.mode, c.current), (4, 0));
        assert!((6..=13).contains(&c.sprite().expect("shown").0));
    }

    /// A choice longer than a row is cut into rows of at most 310 pixels,
    /// at spaces, and all of it is kept.
    #[test]
    fn a_long_choice_is_wrapped_at_spaces_into_rows_of_the_menus_width() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 3, 100);
        settle_effects(&mut g);
        let long = g
            .inner
            .dialogue_of(parsec1994::DialogueRef {
                bank: parsec1994::BankId::Own,
                place: 0,
            })
            .and_then(|d| d.record(9))
            .map(|r| motionvm_parsec_formats::to_cp437(r.text))
            .expect("the Küche's record 9");
        let prepared = parsec1994::prepare_menu_text(&long);
        assert!(!prepared.contains(&b'\r') && !prepared.contains(&b'\n'));
        g.inner.menu = Default::default();
        g.inner.menu_add_for_test(&prepared, 1);
        let lines = &g.inner.menu.lines;
        assert!(lines.len() >= 3, "{lines:?}");
        let joined: Vec<u8> = lines
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
            .join(&b' ');
        assert_eq!(joined, prepared, "nothing lost between the rows");
        for (i, l) in lines.iter().enumerate() {
            let (_, w) = g.inner.font_for_test().measure_task_text(&l.text);
            assert!(w <= 310, "row {i} is {w} wide");
            assert!(l.choice == 1);
            if let Some(nxt) = lines.get(i + 1) {
                // The next row's first word would not have fit.
                let word: Vec<u8> = nxt
                    .text
                    .iter()
                    .take_while(|&&b| b != b' ')
                    .copied()
                    .collect();
                let mut with = l.text.clone();
                with.push(b' ');
                with.extend_from_slice(&word);
                let (_, ww) = g.inner.font_for_test().measure_task_text(&with);
                assert!(
                    ww > 310,
                    "row {i} could have taken {:?}",
                    String::from_utf8_lossy(&word)
                );
            }
        }
    }

    /// The pointer over an object picks the verb its class means, and
    /// shows the object's own hover text at the pointer.
    ///
    /// Once a frame the dispatcher looks at what the pointer is over
    /// (`0x0FDE1`). Over a hotspot it plays the hotspot's `texts[0]` as a
    /// kind-2 text on slot 11, the pointer's own point (`0CFDh:1810h`),
    /// and chooses the object (`0EB9h:0A0Dh`), which runs the phase-1
    /// rules: a door's class 1 is key −2, and that rule makes the walk verb
    /// the verb (`1586h:03CEh`); the bed's class 0 falls to the rule for
    /// any object, `Betrachte` (`1586h:0356h`). Off every object the text
    /// stops and the object is let go (`0x0FEC9`). A click then walks to
    /// the node the mask names and acts: the bed's mask says node 200,
    /// facing 3 (`0x0F276`), and `Betrachte` reads its `texts[3]`.
    #[test]
    fn the_pointer_picks_the_verb_by_class_and_shows_the_hover_text() {
        let mut g = zimmer_later!();
        let door = g
            .inner
            .room()
            .and_then(|r| r.hotspots.spots.iter().find(|h| h.name == "Tür").cloned())
            .expect("the door");
        let bett = g
            .inner
            .room()
            .and_then(|r| r.hotspots.spots.iter().find(|h| h.name == "Bett").cloned())
            .expect("the bed");
        assert_eq!((door.verb_mask >> 8) & 0xF, 1, "the door is class 1");
        assert_eq!((bett.verb_mask >> 8) & 0xF, 0, "the bed is class 0");

        let (x, y) = middle(&door);
        g.inner.mouse_move(x, y);
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.verb, 10);
        assert_eq!(g.inner.status(), "Gehe durch Tür");
        assert_eq!(g.inner.object1, 100);
        assert_eq!(g.inner.two_object, 2, "something to do with it");
        assert_eq!(
            g.inner.tasks.t[0].kind, 2,
            "its own text is up at the pointer"
        );
        assert_eq!(flat(&g.inner.caption_text()), flat(&door.texts[0]));
        assert!(!door.texts[0].is_empty());

        let (x, y) = middle(&bett);
        g.inner.mouse_move(x, y);
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.verb, 1);
        assert_eq!(g.inner.status(), "Betrachte Bett");
        assert_eq!(g.inner.object1, 200);

        g.inner.mouse_move(0, 0);
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.verb, 0, "off every object the verb goes");
        assert_eq!(g.inner.object1, 0);
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.tasks.t[0].kind, 0, "and the hover text is gone");

        // A click on the bed walks to its node, facing left, and looks.
        g.inner.mouse_move(x, y);
        crate::titles::Runtime::tick(&mut g);
        assert!(g.inner.click());
        assert_eq!(g.inner.mode, 4);
        assert_eq!(g.inner.walk_target, 200);
        let mut frames = 0;
        while !text_up(&g) {
            crate::titles::Runtime::tick(&mut g);
            frames += 1;
            assert!(frames < 2000);
        }
        assert_eq!(g.inner.player_node_id(), 200);
        assert_eq!(g.inner.walker.as_ref().map(|w| w.facing()), Some(3));
        assert!(flat(&bett.texts[3]).starts_with(&flat(&g.inner.caption_text())));
        // The arrival sets mode 7 and the rule puts its text up at once
        // (`0x0F4BA`); the next step makes it 8 (`0x0F51E`).
        assert_eq!(g.inner.mode, 7, "the action's text is up");
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.mode, 8, "waiting for the text");
        settle(&mut g);
        assert_eq!(
            (g.inner.mode, g.inner.verb),
            (0, 0),
            "an action's end resets"
        );
    }

    /// Karsten sleeps in the bed, and turns over.
    ///
    /// The Zimmer's tail plays `0883h:20B3h(0C8h)`: the bed shown and its
    /// animation 0 started — `03 53 01 | 06 | 0C 52 01 | 9B 28 50 10 …`, a
    /// random walk through sixteen entries of the five cels 1 to 5, forty
    /// to eighty ticks each, round and round. So the bed is drawn from the
    /// first frame with Karsten in it and shows more than one cel over
    /// the opening's first seconds. The yawn (event 12 on tick 373,
    /// animation 1) takes over at once, the condition being 1.
    #[test]
    fn karsten_sleeps_in_the_bed_and_turns_over() {
        let mut g = ds1!();
        let bed = g.inner.object_index(200).expect("the bed");
        assert!(
            g.inner.objects[bed].visible(),
            "shown by the handler's tail"
        );
        assert_eq!(g.inner.objects[bed].state, 7, "bound to its entry");
        let ai = g.inner.objects[bed].ani.expect("its entry");
        assert_eq!(g.inner.anis[ai].current, 0);
        let mut cels = std::collections::BTreeSet::new();
        for _ in 0..600 {
            crate::titles::Runtime::tick(&mut g);
            let o = &g.inner.objects[bed];
            assert!(o.visible(), "asleep the whole time");
            cels.insert(o.cel);
        }
        assert!(cels.iter().all(|c| (1..=5).contains(c)), "{cels:?}");
        assert!(cels.len() > 1, "turns over: {cels:?}");
        assert_eq!(g.inner.anis[ai].current, 1, "the yawn took over");
        // And the bed is on the picture: the cel's pixels at its point.
        let frame = g.inner.render();
        let bare = {
            let mut empty = ds1!();
            empty.inner.object_hide(200);
            empty.inner.render()
        };
        let o = &g.inner.objects[bed];
        let (w, h) = g.inner.object_cel_size(0, o.cel).expect("a cel");
        let differs = (0..h)
            .flat_map(|dy| (0..w).map(move |dx| (dx, dy)))
            .filter(|&(dx, dy)| frame.get(o.x + dx, o.y + dy) != bare.get(o.x + dx, o.y + dy))
            .count();
        assert!(
            differs > 100,
            "the bed's cel is drawn, {differs} pixels differ"
        );
    }

    /// The Küche's mother is two objects placed as one.
    ///
    /// The Küche's block binds entry 0 — id 200, a chain — to objects 200
    /// and 201: the head, twelve cels from 180, and the body, one cel
    /// (192) with placement 1, under the head's box. The handler shows
    /// her (`14DDh:084Ah`, every part live and hidden) and plays her
    /// animation 3, whose first step places both. Placement 1 puts the
    /// body's top on the head's bottom, and the file's own position for
    /// the body, `(44, 75)`, is exactly that: the head's 26 rows below
    /// its `(44, 49)`.
    #[test]
    fn the_mother_stands_in_the_kitchen_as_head_and_body() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 3, 100);
        settle_effects(&mut g);
        let head = g.inner.object_index(200).expect("the head");
        let body = g.inner.object_index(201).expect("the body");
        assert_eq!(g.inner.objects[head].state, 8);
        assert_eq!(g.inner.objects[body].state, 9);
        assert_eq!(g.inner.objects[head].ani, g.inner.objects[body].ani);
        for _ in 0..2 {
            crate::titles::Runtime::tick(&mut g);
        }
        let (h, b) = (g.inner.objects[head].clone(), g.inner.objects[body].clone());
        assert!(h.visible() && b.visible(), "{h:?} {b:?}");
        assert_eq!((h.x, h.y), (44, 49));
        assert!((180..=191).contains(&h.cel), "head cel {}", h.cel);
        assert_eq!(b.cel, 192);
        let (_, hh) = g.inner.object_cel_size(0, h.cel).expect("the head's cel");
        assert_eq!((b.x, b.y), (44, 49 + hh), "placement 1: under the head");
        assert_eq!((b.x, b.y), (44, 75), "which is where the file put it");
        // Her hotspot follows her live bit (`14DDh:074Bh`, kind 10).
        let spot = g
            .inner
            .room()
            .and_then(|r| {
                r.hotspots
                    .spots
                    .iter()
                    .position(|s| s.verb_mask & 0xFF == 200)
            })
            .expect("her hotspot");
        assert!(g.inner.hotspots_rt[spot].active);
        // And over a while she moves her head: more than one cel is shown.
        let mut cels = std::collections::BTreeSet::new();
        for _ in 0..1200 {
            crate::titles::Runtime::tick(&mut g);
            cels.insert(g.inner.objects[head].cel);
        }
        assert!(cels.len() > 1, "{cels:?}");
    }

    /// The door opens frame by frame, and the machine waits for it.
    ///
    /// The seventh verb's rule plays the door's animation 1 and answers
    /// 12, so the machine waits in mode 8 for code 2 (`[28C1h]`). The
    /// stream — `05 | 17 08 00 | 8A 0A 03 FF 00 01 | 17 02 00 | 00` — reports
    /// code 8 on the frame pass of the tick the rule ran on, shows the
    /// door hidden, closed (cel 8) and open (cel 9) ten ticks apart, and
    /// reports code 2 on the twenty-ninth tick after, the first wait
    /// having lost the tick that ran the countdown out. On the thirtieth
    /// the gate is asked and the figure walks for the twin node.
    #[test]
    fn the_door_opens_frame_by_frame_before_the_figure_goes_through() {
        let mut g = zimmer_later!();
        let spot = g
            .inner
            .room()
            .and_then(|r| r.hotspots.spots.iter().find(|h| h.name == "Tür").cloned())
            .expect("the door");
        let door = g.inner.object_index(100).expect("the door's object");
        assert!(!g.inner.objects[door].visible(), "shut and unseen at first");
        let (x, y) = middle(&spot);
        g.inner.mouse_move(x, y);
        crate::titles::Runtime::tick(&mut g);
        assert!(g.inner.click());
        let mut opened = None;
        let mut code = None;
        let mut cels = Vec::new();
        for tick in 1..=600 {
            crate::titles::Runtime::tick(&mut g);
            for e in g.inner.events.drain(..) {
                match e {
                    Event::ObjectAnim {
                        object: 100,
                        anim: 1,
                    } => opened = Some(tick),
                    Event::AnimCode {
                        object: 100,
                        code: 2,
                    } => code = Some(tick),
                    _ => {}
                }
            }
            if let Some(t) = opened {
                let o = &g.inner.objects[door];
                let shown = if o.visible() { Some(o.cel) } else { None };
                if cels.last().map(|&(_, c)| c) != Some(shown) {
                    cels.push((tick - t, shown));
                }
                if tick == t + 30 {
                    break;
                }
            }
        }
        let t = opened.expect("the rule opened the door");
        assert_eq!(code, Some(t + 29), "code 2 twenty-nine ticks after");
        assert_eq!(cels, [(0, None), (9, Some(8)), (19, Some(9))]);
        assert_eq!(g.inner.walk_target, 2100, "and the walk to the twin began");
        assert_eq!(g.inner.word_28c1, 0);
    }

    /// What a visit does to a room's things is there at the next visit.
    ///
    /// `Nimm` on the toolbox is the Zimmer's rule `0089:0069`, scene 3,
    /// whose text carries event 15 — the hook hides object 1 and gives
    /// item 1. On the way out `14DDh:068Ah` writes a byte per object of
    /// kinds 0, 1 and 5 into the location's record at `DS:1F56h +
    /// 20h × 2` (`0x9EF`), the toolbox's — kind 0 — as its live bit plus
    /// one; at the next entry `14DDh:0293h` puts the byte back as the
    /// flags (`0x15221`), and the hotspot follows (`14DDh:074Bh`). So the
    /// toolbox stays taken, and its hotspot stays inactive.
    #[test]
    fn a_taken_item_stays_taken_at_the_next_visit() {
        let mut g = zimmer_later!();
        let spot = g
            .inner
            .room()
            .and_then(|r| {
                r.hotspots
                    .spots
                    .iter()
                    .find(|h| h.name == "Werkzeugkasten")
                    .cloned()
            })
            .expect("the toolbox");
        let toolbox = g.inner.object_index(1).expect("object 1");
        assert!(g.inner.objects[toolbox].visible(), "on its shelf");
        assert!(act(&mut g, 12, &spot).is_some(), "the rule answers");
        settle(&mut g);
        assert!(g.inner.carried(1), "event 15 gave item 1");
        assert!(!g.inner.objects[toolbox].visible(), "and hid the object");
        crate::titles::Runtime::set_location_via(&mut g, 3, 100);
        settle_effects(&mut g);
        // Kind 0 writes its live bit plus one; the drawer behind it, kind
        // 1 and shipped not live, writes 1 too.
        assert_eq!(&g.inner.location_records[2][..2], &[1, 1]);
        crate::titles::Runtime::set_location_via(&mut g, 2, 0x7F);
        settle_effects(&mut g);
        let toolbox = g.inner.object_index(1).expect("object 1");
        assert!(!g.inner.objects[toolbox].visible(), "still taken");
        let spot_index = g
            .inner
            .room()
            .and_then(|r| {
                r.hotspots
                    .spots
                    .iter()
                    .position(|h| h.name == "Werkzeugkasten")
            })
            .expect("its hotspot");
        assert!(!g.inner.hotspots_rt[spot_index].active, "and not clickable");
    }

    /// Whether object `id` is shown, when the room has it.
    fn shown(g: &DunkleSchatten, id: u8) -> Option<bool> {
        g.inner
            .object_index(id)
            .map(|i| g.inner.objects[i].visible())
    }

    /// The first page task 0 puts up within `ticks` that `want` accepts.
    fn first_page(
        g: &mut DunkleSchatten,
        ticks: u32,
        want: impl Fn(&str) -> bool,
    ) -> Option<String> {
        for _ in 0..ticks {
            crate::titles::Runtime::tick(g);
            let said = g.inner.events.drain(..).find_map(|e| match e {
                Event::PageUp { task: 0, text } if want(&text) => Some(text),
                _ => None,
            });
            if said.is_some() {
                return said;
            }
        }
        None
    }

    /// Every animation stopped, the walker's too: the exit frame at
    /// `0x0ED2C` advances running animations, and those can roll.
    fn stop_outgoing(g: &mut DunkleSchatten) {
        for o in &mut g.inner.objects {
            o.state = 0;
        }
        if let Some(w) = &mut g.inner.walker {
            w.state = 0;
        }
    }

    /// Into the Küche and back into the Wohnzimmer by exit 101, with the
    /// outgoing animations stopped before each transition.
    fn out_and_back(g: &mut DunkleSchatten) {
        for loc in [3, 4] {
            stop_outgoing(g);
            crate::titles::Runtime::set_location_via(g, loc, 101);
            settle_effects(g);
        }
    }

    /// The Wohnzimmer on the first evening: the father before the
    /// television, and the talk that sends Karsten to his mother.
    ///
    /// `[2716h]` is 1 — neither 3 nor `80h` — so the handler's third arm
    /// (`0x10D7`) wakes the father and the television and shows object
    /// 204: `0883h:20B3h` shows each and starts its animation 0, and the
    /// father is a chain, so his body 201 comes up with his head. Mike
    /// and his body stay down. `Sprich mit` on the father reaches the
    /// handler's own rule (`0100:0001`): with `[2716h]` at 1 it loads
    /// scene 0 and writes the four bytes that send the Küche into its
    /// state 2.
    ///
    /// The hook then follows the dialogue. Code 0 says who speaks —
    /// Karsten and the father, bits 0 and 1 — so `[18A4h]` is 1. Every
    /// page's controls are scanned after code 8 with the page's slot
    /// (`0x0D1B7`): 0 for Karsten's pages, 1 for the father's, so a
    /// page's end plays the base plus the slot — 1 while Karsten speaks,
    /// 2 while the father does. A record that ends resets the task
    /// (code 6), which plays `[18A4h] − 1` and clears the base, and the
    /// next record's code 0 sets it again. The text's own `03 0Ah` puts
    /// the base at 5, and that page — the father's — plays 6; the pages
    /// after it 5 and 6; its `03 0Bh` puts it back at 1, and that page
    /// plays 2; the dialogue's end (code 5) plays `[18A4h] − 1` once more
    /// and clears the three. The same last page carries `03 79h`, event 121:
    /// the machine leaves through exit 101, into the Küche, whose state
    /// is now 2 — and its arm at `0x0E8B` walks Karsten to his mother
    /// and loads scene 5, *Hi, Mutti*.
    #[test]
    fn the_father_sits_before_the_television_on_the_first_evening() {
        let mut g = ds1!();
        g.inner.sav_set(0x2716, 1);
        crate::titles::Runtime::set_location_via(&mut g, 4, 101);
        settle_effects(&mut g);
        assert_eq!(shown(&g, 200), Some(true), "the father's head");
        assert_eq!(shown(&g, 201), Some(true), "and his body");
        assert_eq!(shown(&g, 205), Some(true), "the television");
        assert_eq!(shown(&g, 204), Some(true), "object 204");
        assert_eq!(shown(&g, 130), Some(false), "Mike is not there");
        assert_eq!(shown(&g, 203), Some(false), "nor his body");
        let father = g.inner.object_index(200).expect("the father");
        let ai = g.inner.objects[father].ani.expect("bound to his entry");
        assert_eq!(g.inner.anis[ai].current, 0, "on his idle animation");
        g.inner.events.clear();

        // `Sprich mit` on him: the figure walks to node 200 and the rule
        // runs on arrival. Every menu on the way takes its first line.
        g.inner.verb = 2;
        g.inner.mouse_move(120, 80);
        assert!(g.inner.click());
        let mut speakers = None;
        let mut base_at_load = None;
        let mut plays = Vec::new();
        let mut script_events = Vec::new();
        let mut over_at = None;
        for tick in 1..=30000 {
            crate::titles::Runtime::tick(&mut g);
            if g.inner.menu.up {
                crate::titles::Runtime::key(&mut g, 9);
                crate::titles::Runtime::key(&mut g, 13);
            }
            let events = std::mem::take(&mut g.inner.events);
            for e in events {
                match e {
                    Event::LocationHook { code: 0, arg, .. } => {
                        speakers = Some(arg);
                        base_at_load = Some(father_base(&g));
                    }
                    Event::LocationHook { code: 5 | 6, .. } => over_at = Some(tick),
                    Event::ObjectAnim { object: 200, anim } => plays.push((tick, anim)),
                    Event::Script { n, .. } => script_events.push((tick, n)),
                    _ => {}
                }
            }
            if over_at.is_some() || g.inner.location() != 4 {
                break;
            }
        }
        assert_eq!(speakers, Some(3), "Karsten and the father speak");
        assert_eq!(base_at_load, Some(1));
        assert_eq!(g.inner.sav_byte(0x2726), 0);
        assert_eq!(g.inner.sav_byte(STATE), 2);
        assert_eq!(g.inner.sav_byte(0x271E), 0);
        assert_eq!(g.inner.sav_byte(0x2715), 1);
        let over_at = over_at.expect("the dialogue ends");
        let events: Vec<u8> = script_events.iter().map(|&(_, n)| n).collect();
        assert_eq!(events, [103, 10, 11, 121], "{script_events:?}");
        let at = |n: u8| {
            script_events
                .iter()
                .find(|&&(_, e)| e == n)
                .map(|&(t, _)| t)
        };
        assert_eq!(
            plays.iter().map(|&(_, k)| k).collect::<Vec<_>>(),
            [
                1, 2, 1, 2, 0, 1, 2, 0, 1, 2, 1, 2, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 1, 2, 6, 5, 6,
                5, 2, 0
            ],
            "{plays:?}"
        );
        let play_at = |t: u32| plays.iter().find(|&&(at, _)| at == t).map(|&(_, k)| k);
        assert_eq!(
            play_at(at(10).unwrap()),
            Some(6),
            "6 on the father's page that set the base to 5"
        );
        assert_eq!(
            play_at(at(11).unwrap()),
            Some(2),
            "2 on the father's page that set it back"
        );
        assert_eq!(
            plays.last(),
            Some(&(over_at, 0)),
            "and 0 when the dialogue is over"
        );
        assert_eq!(father_base(&g), 0);

        // Event 121 asked for exit 101, and the Küche's state is 2.
        let left = (1..=3000).any(|_| {
            crate::titles::Runtime::tick(&mut g);
            g.inner.location() != 4
        });
        assert!(left, "the machine leaves through the door");
        assert_eq!(g.inner.location(), 3, "into the Küche");
        let said = first_page(&mut g, 3000, |_| true);
        assert_eq!(
            said.as_deref(),
            Some("Hi, Mutti."),
            "the Küche's state 2 plays scene 5"
        );
    }

    /// With the father out, the telephone rings on a roll.
    ///
    /// `[2716h]` at `80h` hides the father (`0x10F3`) and switches on
    /// `[2714h]`. At 1 the arm at `0x1152` rolls 0 to 2, and only a 0
    /// goes on: `[2714h] = 0` and the wrong number at `0x1179`, which
    /// counts the eight names, rolls one into substitution 0 and rolls
    /// scene 11, 12 or 13. Any other `[2714h]` rolls 0 to 4 (`0x1169`),
    /// again only a 0 going on.
    ///
    /// The generator is shared, and every change of location rolls it
    /// once for the transition effect (`0x0ED42`, `1EB5h:0028h(1, 6)`).
    /// This test stops outgoing animations before each transition to
    /// isolate the handler's rolls: the exit frame at `0x0ED2C` otherwise
    /// advances animations that can also consume randomness. With those
    /// animations excluded, from seed zero the transition rolls once,
    /// and the first visit's roll of 0 to 2 then answers
    /// 0; the draw answers 5 and 12 (the arithmetic is
    /// [`crate::titles::rng`], worked by hand for these four). So the
    /// telephone rings at once: *ihre "liebe Großmutter"* is asked for in
    /// scene 12. Playing the scene rolls too — the figure's own
    /// animations wait at random — so the visits after it are worked
    /// from the seed as it then stands, with the same arithmetic: two
    /// transitions, the visit's roll, and the draw's two only on a 0.
    /// What is pinned there is that a visit rolls exactly what the
    /// listing says. The three arms that do not roll are pinned on the
    /// way back: 2, 4 and 3 load scenes 4, 10 and 8 and write their
    /// bytes.
    #[test]
    fn the_telephone_rings_with_a_wrong_number_when_the_father_is_out() {
        use crate::titles::rng::LcRng;
        let mut g = ds1!();
        // The second morning as the Zimmer's wake-up leaves it, with the
        // Küche's own arms kept out of the way.
        g.inner.sav_set(STATE, 1);
        g.inner.sav_set(0x2715, 0x80);
        g.inner.sav_set(0x2716, 0x80);
        g.inner.sav_set(0x2714, 1);
        g.inner.events.clear();
        // The start options' dissolve has drawn from the generator
        // (`0x1208C`), as many times as the machine fits into a second;
        // the rolls below are worked from seed zero, so they start there.
        *g.inner.rng_mut() = LcRng::default();
        assert_eq!(g.inner.seed_for_test(), (0, 0), "nothing has rolled yet");
        let bound = |g: &mut DunkleSchatten| -> Vec<i16> {
            g.inner
                .events
                .drain(..)
                .filter_map(|e| match e {
                    Event::SceneBound { scene, .. } => Some(scene),
                    _ => None,
                })
                .collect()
        };
        let names = wrong_numbers(&g);
        let grossmutter = motionvm_parsec_formats::to_cp437("ihre \"liebe Großmutter\"");
        assert_eq!(names[5], grossmutter);

        stop_outgoing(&mut g);
        crate::titles::Runtime::set_location_via(&mut g, 4, 101);
        settle_effects(&mut g);
        assert_eq!(shown(&g, 200), Some(false), "the father is out");
        assert_eq!(shown(&g, 201), Some(false), "body and all");
        assert_eq!(g.inner.sav_byte(0x2714), 0, "the roll answers 0");
        assert_eq!(bound(&mut g), vec![12]);
        assert_eq!(g.inner.substitutions[0], grossmutter);
        assert_eq!(
            g.inner.seed_for_test(),
            (0x96AC, 0xFE32),
            "four rolls: the transition, the visit's, the draw's two"
        );
        // And the page that names the party carries the name.
        let said = first_page(&mut g, 6000, |text| text.contains("bestimmt"));
        let said = said.expect("the seventh page of scene 12");
        assert!(
            said.replace('\n', " ")
                .contains("bestimmt nicht ihre \"liebe Großmutter\""),
            "got {said:?}"
        );

        for visit in 2..=4 {
            let (lo, hi) = g.inner.seed_for_test();
            let mut rng = LcRng { lo, hi };
            rng.inclusive(1, 6);
            rng.inclusive(1, 6);
            let draw = (rng.inclusive(0, 4) == 0).then(|| {
                let name = names[usize::from(rng.inclusive(0, 7))].clone();
                (name, num::signed(rng.inclusive(11, 13)))
            });
            out_and_back(&mut g);
            assert_eq!(g.inner.sav_byte(0x2714), 0);
            match draw {
                Some((name, scene)) => {
                    assert_eq!(bound(&mut g), vec![scene], "visit {visit}");
                    assert_eq!(g.inner.substitutions[0], name, "visit {visit}");
                }
                None => {
                    assert_eq!(bound(&mut g), Vec::<i16>::new(), "visit {visit} is quiet");
                }
            }
            assert_eq!(
                g.inner.seed_for_test(),
                (rng.lo, rng.hi),
                "visit {visit} rolled what the listing says"
            );
        }

        // `0x1110`: Tom's call.
        g.inner.sav_set(0x2714, 2);
        g.inner.sav_set(0x272D, 7);
        out_and_back(&mut g);
        assert_eq!(bound(&mut g), vec![4]);
        assert_eq!(g.inner.sav_byte(0x2700), 1);
        assert_eq!(g.inner.sav_byte(0x272E), 0);
        assert_eq!(g.inner.sav_byte(0x272D), 1);
        assert_eq!(g.inner.sav_byte(0x2714), 1);

        // `0x1134`: Stauf's call.
        g.inner.sav_set(0x2714, 4);
        out_and_back(&mut g);
        assert_eq!(bound(&mut g), vec![10]);
        assert_eq!(g.inner.sav_byte(0x272D), 5);
        assert_eq!(g.inner.sav_byte(0x2714), 1);

        // `0x1130`: Jörg's call; `0x1110` again with `[272Dh]` at 2, which
        // it leaves alone.
        g.inner.sav_set(0x2714, 3);
        out_and_back(&mut g);
        assert_eq!(bound(&mut g), vec![8]);
        assert_eq!(g.inner.sav_byte(0x2714), 1);
        g.inner.sav_set(0x2714, 2);
        g.inner.sav_set(0x272D, 2);
        g.inner.sav_set(0x272E, 9);
        out_and_back(&mut g);
        assert_eq!(bound(&mut g), vec![4]);
        assert_eq!(g.inner.sav_byte(0x272D), 2);
        assert_eq!(g.inner.sav_byte(0x272E), 9);
    }

    /// Event 12 stands the father up, and his animation reports back.
    ///
    /// The hook's arm at `0x135A` arms task 0, plays samples 8 and 9,
    /// zeroes his base, starts his animation 8 and marks `[270Fh]`.
    /// Animation 8 in the room's block is `03 79 02 | 05 | 8A 0F 08 28 …
    /// 2F | 17 64 00 | 8A 14 06 30 … 28 | 17 65 00 | 17 01 00 | 0D 33 02`:
    /// eight frames of fifteen, code 100, six frames of twenty, code 101,
    /// then code 1 — the callback's own, which lets a held page go
    /// (`0CFDh:1BA2h`) — and a jump into animation 6's loop, the father
    /// on his feet. He is on animation 0, past its first step — the
    /// entry's pass with no ticks took that (`0x0EC50`) — when 8 is asked
    /// for, so the ask caps his countdown (`0x0A889`) and the next pass
    /// ends the loop, starts 8 and shows its first frame. From there a
    /// frame of `d` holds `d` passes, so the codes come 120 and 240
    /// passes on — the 120th and the 240th tick. Code 100 reaches the
    /// animation hook at `0100:01FC`: task 0 started, sample 19 twice;
    /// code 101: samples 9 and 10, and his base back to 1.
    #[test]
    fn the_father_gets_up_and_his_animation_reports_twice() {
        let mut g = ds1!();
        g.inner.sav_set(0x2716, 1);
        crate::titles::Runtime::set_location_via(&mut g, 4, 101);
        settle_effects(&mut g);
        g.inner.events.clear();
        logic::location_hook(&mut g.inner, 7, 12, 0);
        assert_eq!(
            g.inner.events,
            vec![
                Event::Sample(8),
                Event::Sample(9),
                Event::ObjectAnim {
                    object: 200,
                    anim: 8
                }
            ]
        );
        assert_eq!(g.inner.sav_byte(BED), 1);
        assert_eq!(father_base(&g), 0);
        assert_ne!(g.inner.tasks.t[0].flags & 0x8000, 0, "task 0 armed");
        g.inner.events.clear();
        let mut log = Vec::new();
        for tick in 1..=300 {
            crate::titles::Runtime::tick(&mut g);
            let events = std::mem::take(&mut g.inner.events);
            for e in events {
                match e {
                    Event::AnimCode { object: 200, code } => {
                        log.push((tick, format!("code {code}"), father_base(&g)));
                    }
                    // Footsteps (samples 0 and 1) are not the father's.
                    Event::Sample(k) if k > 1 => {
                        log.push((tick, format!("sample {k}"), father_base(&g)));
                    }
                    _ => {}
                }
            }
        }
        let want: Vec<(i32, String, i16)> = [
            (120, "code 100", 0),
            (120, "sample 19", 0),
            (120, "sample 19", 0),
            // The location's hook runs inside the signal it answers
            // (`0x109BB`), so code 101's samples come before code 1.
            (240, "code 101", 1),
            (240, "sample 9", 1),
            (240, "sample 10", 1),
            (240, "code 1", 1),
        ]
        .iter()
        .map(|&(t, s, b)| (t, s.to_string(), b))
        .collect();
        assert_eq!(log, want);
        assert_eq!(
            g.inner.tasks.t[0].flags & 0x8000,
            0,
            "task 0 started at code 100"
        );
    }

    /// Wake-up snaps and inserts the figure (`0x0C0E`, `0x0B7E2`);
    /// event 11 removes it again (`0x0C86`) while the bed animation runs.
    #[test]
    fn waking_inserts_the_figure_and_returning_to_bed_removes_only_its_draw_link() {
        let mut g = ds1!();
        // Fifteen ticks past the wake-up on tick 822.
        for _ in 0..837 {
            crate::titles::Runtime::tick(&mut g);
        }
        let w = g.inner.walker.as_ref().expect("awake figure");
        assert_eq!((w.x, w.y, w.dest_id), (168, 87, 200));
        assert!(w.layer.is_some(), "the wake-up snap inserts the figure");
        let node = w.node;
        logic::location_hook(&mut g.inner, 7, 11, 0);
        let w = g.inner.walker.as_ref().expect("sleeping figure record");
        assert_eq!(w.layer, None);
        assert_eq!(w.node, node);
        assert_eq!((w.x, w.y, w.dest_id), (168, 87, 200));
        assert_eq!(g.inner.sav_byte(BED), 2);
    }

    /// The Laden's person, 83h, walks from node 12Ch to 12Dh through the
    /// handle `0BB6h:05B0h` gives out, and the logic's question — the
    /// record's `+28h` read through the far pointer (`0x21E0`) — answers
    /// `FFFFh` until the pass after the leg ends, then 12Dh.
    ///
    /// The same person fills subject slot 2 on the Laden's entry, because
    /// the location row names 131 (83h) and no object carries that id
    /// (`0x0EE2E`–`0x0EE3E`); the Straße's 128 (80h) fills its slot 1.
    #[test]
    fn the_ladens_person_walks_through_its_handle_and_answers_its_node() {
        use crate::titles::subject::Subject;
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 6, 0x7F);
        settle_effects(&mut g);
        assert_eq!(
            g.inner.subjects.get(1).map(|s| (s.subject, s.code)),
            Some((Subject::Mover(0), 0x2000)),
            "the Straße's slot 1 is its person"
        );
        crate::titles::Runtime::set_location_via(&mut g, 9, 0x7F);
        settle_effects(&mut g);
        assert_eq!(
            g.inner.subjects.get(2).map(|s| s.subject),
            Some(Subject::Mover(0)),
            "the Laden's slot 2 is its person"
        );
        let a = &mut g.inner;
        let rec = a.mover_find(0x83);
        assert_eq!(rec, Far::mover(0), "the room's record 0");
        let node = |a: &mut parsec1994::Adventure| {
            a.far_word(Far {
                off: rec.off + 0x28,
                seg: rec.seg,
            })
        };
        let at = |a: &mut parsec1994::Adventure| {
            let x = a.far_word(Far {
                off: rec.off + 4,
                seg: rec.seg,
            });
            let y = a.far_word(Far {
                off: rec.off + 6,
                seg: rec.seg,
            });
            (x, y)
        };
        assert_eq!(node(a), 0, "+28h as the file has it");
        assert_eq!(a.walker_walk_to(rec, 0x12D, 3), 0, "mode FFFFh: no walk");
        assert!(
            a.chains
                .iter()
                .flatten()
                .all(|&b| b != parsec1994::Body::Mover(0)),
            "in no chain before the bind"
        );
        assert_eq!(a.walker_bind_graph(rec, 1, 0x12C, 3), 1);
        assert_eq!(node(a), 0x12C);
        assert_eq!(at(a), (166, 91));
        assert_eq!(
            a.chain_head(0),
            Some(parsec1994::Body::Mover(0)),
            "linked at the head of node 12Ch's layer"
        );
        assert_eq!(a.walker_walk_to(rec, 0x12D, 3), 2, "a walk started");
        assert_eq!(node(a), 0xFFFF);
        let mut ticks = 0;
        while node(a) == 0xFFFF {
            a.interrupt();
            a.frame_pass();
            ticks += 1;
            assert!(ticks < 1000, "the walk ends");
        }
        assert_eq!(ticks, 114, "the leg ends in pass 113, the arrival is 114");
        assert_eq!(node(a), 0x12D);
        assert_eq!(at(a), (121, 88));
        // Taken out, it stays where it is and answers the same.
        a.walker_unlink(rec);
        assert!(
            a.chains
                .iter()
                .flatten()
                .all(|&b| b != parsec1994::Body::Mover(0))
        );
        assert_eq!(node(a), 0x12D);
    }

    /// The F1 in sub-state 4 binds its person to node 193h (`0x409D`),
    /// which neither of the room's graphs has: the snap takes the record
    /// out of its chain and answers 0 (`0x0B722`, `0x0B733`), the header
    /// keeps the file's mode `FFFFh`, so a walk is refused too — and the
    /// person, though named in subject slot 1 (`0x40B1`), is not drawn.
    #[test]
    fn the_f1_binds_its_person_to_a_node_no_graph_has() {
        use crate::titles::subject::Subject;
        let mut g = ds1!();
        g.inner.set_byte(0x2726, 4);
        crate::titles::Runtime::set_location_via(&mut g, 27, 0x7F);
        settle_effects(&mut g);
        assert_eq!(g.inner.location(), 27);
        let a = &mut g.inner;
        let rec = a.mover_find(0xC8);
        assert_eq!(rec, Far::mover(0));
        assert_eq!(a.movers[0].graph, Some(1), "bound, +24h written");
        assert!(
            a.chains
                .iter()
                .flatten()
                .all(|&b| b != parsec1994::Body::Mover(0)),
            "but in no chain"
        );
        assert_eq!(
            a.walker_bind_graph(rec, 1, 0x193, 1),
            0,
            "the bind answers 0"
        );
        assert_eq!(a.walker_walk_to(rec, 0x192, 0), 0, "mode FFFFh");
        assert_eq!(
            a.subjects.get(1).map(|s| s.subject),
            Some(Subject::Mover(0))
        );
        let before = a.render_scene();
        a.movers[0].flags = 0;
        assert_eq!(
            before.pixels,
            a.render_scene().pixels,
            "the person adds nothing to the picture"
        );
    }

    /// A bound person is drawn in its layer: after that layer's tiles and
    /// objects, under the next layer's tiles, and nowhere outside its cel.
    #[test]
    fn a_bound_person_is_drawn_in_its_layer() {
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 9, 0x7F);
        settle_effects(&mut g);
        let a = &mut g.inner;
        let rec = a.mover_find(0x83);
        let without = a.render_scene();
        assert_eq!(a.walker_bind_graph(rec, 1, 0x12C, 3), 1);
        let with = a.render_scene();
        let m = &a.movers[0];
        let (cel, mirror) = m.walker.cel();
        assert_eq!((cel, mirror), (7, true));
        let pic = a
            .object_cel(0, u16::try_from(cel).unwrap())
            .expect("the standing cel");
        let (w, h) = (i32::from(pic.width), i32::from(pic.height));
        // Align 2: centered on x, standing on y.
        let (x0, y0) = (m.walker.x - (w >> 1), m.walker.y - h);
        let layer = usize::from(m.walker.rec_layer);
        let room = a.room().expect("the Laden");
        let mut drawn = 0;
        for y in 0..128i32 {
            for x in 0..320i32 {
                let i = usize::try_from(y * 320 + x).unwrap();
                let inside = (x0..x0 + w).contains(&x) && (y0..y0 + h).contains(&y);
                if !inside {
                    assert_eq!(with.pixels[i], without.pixels[i], "({x},{y}) outside");
                    continue;
                }
                let covered = room.room.layers[layer + 1..]
                    .iter()
                    .any(|l| l.pixels.get(i).is_some_and(|&p| p != 0));
                if covered {
                    assert_eq!(with.pixels[i], without.pixels[i], "({x},{y}) under a tile");
                } else if with.pixels[i] != without.pixels[i] {
                    drawn += 1;
                }
            }
        }
        assert!(drawn > 50, "{drawn} pixels of the person");
    }

    /// What the save copies as bytes is the engine's own records: the
    /// object rows at `DS:1F56h` (`0x14C42`), the carried list from its
    /// count at `DS:15FEh` (`0x14C8B`) and the player's record at
    /// `DS:1693h` (`0x14C73`), its factor per edge at `DS:16A7h`
    /// (`0x0AEF9`). A read through the data segment sees the record, and a
    /// write lands in it.
    #[test]
    fn the_data_segment_is_the_records_the_engine_keeps() {
        let mut g = ds1!();
        let a = &mut g.inner;
        // Object rows.
        a.location_records[5][3] = 0x42;
        assert_eq!(a.byte(0x1F56 + 5 * 32 + 3), 0x42);
        a.set_byte(0x1F56 + 7 * 32 + 1, 0x17);
        assert_eq!(a.location_records[7][1], 0x17);
        // The carried list: count, scroll, items.
        let items = a.items.clone();
        assert_eq!(usize::from(a.byte(0x15FE)), items.len());
        assert_eq!(a.byte(0x1602), items[0]);
        let bytes: Vec<u8> = (0..0x23).map(|i| a.byte(0x15FE + i)).collect();
        a.give_item(3);
        assert_ne!(a.items, items);
        for (i, &b) in bytes.iter().enumerate() {
            a.set_byte(0x15FE + u16::try_from(i).unwrap(), b);
        }
        assert_eq!(a.items, items, "a block copied back is the list it was");
        // The factor per edge, an odd address inside no shared word.
        a.set_byte(0x16A7, 0x0E);
        assert_eq!(a.walker.as_ref().map(|w| w.gait.speed), Some(0x0E));
        assert_eq!(a.byte(0x16A7), 0x0E);
        // A shared word read by its high byte.
        a.verb = 0x34;
        assert_eq!(a.byte(0x288D), 0x34);
        assert_eq!(a.byte(0x288E), 0);
    }

    /// The game in the Wohnzimmer with nothing going on: no text up, the
    /// player standing.
    fn idle_in_the_wohnzimmer() -> Option<DunkleSchatten> {
        let mut g = ds1_or_skip()?;
        crate::titles::Runtime::set_location_via(&mut g, 4, 0x7F);
        for _ in 0..600 {
            crate::titles::Runtime::tick(&mut g);
        }
        assert_eq!(g.inner.tasks.t[0].kind, 0, "no text up");
        assert!(g.inner.player_linked(), "standing");
        Some(g)
    }

    /// Ticks until the options are up and have dissolved in; answers how
    /// many it took.
    fn until_the_options_take_input(g: &mut DunkleSchatten) -> u32 {
        for n in 0..400 {
            if g.inner.options_up() && !g.inner.options_fading() {
                return n;
            }
            crate::titles::Runtime::tick(g);
        }
        panic!("the options did not come up");
    }

    /// The BIOS tick `ticks` interrupts after a seed of `seed`: one count
    /// for every 65 536 PIT cycles, `26D6h` an interrupt, from a carry
    /// word the seed leaves at zero (`0x19FB6`).
    fn bios_after(seed: u32, ticks: u32) -> u32 {
        seed + ticks * parsec1994::clock::TICK_PERIOD / 0x1_0000
    }

    /// Row `id`'s record, 1-based, as the program keeps it: its `y` and
    /// its index (`DS:14A4h`, `+2`, `+4`).
    fn option_row(g: &DunkleSchatten, id: u16) -> (i32, u16) {
        let rec = g.inner.word(0x14A4 + 4 * (id - 1));
        (i32::from(g.inner.word(rec + 2)), g.inner.word(rec + 4))
    }

    /// A click on row `id` and the pass that takes it.
    fn click_option_row(g: &mut DunkleSchatten, id: u16) {
        let (y, _) = option_row(g, id);
        crate::titles::Runtime::mouse_move(g, 160, y + 3);
        crate::titles::Runtime::tick(g);
        crate::titles::Runtime::click(g, true);
        crate::titles::Runtime::tick(g);
    }

    /// A click on a confirmation's yes or no: the box is 60 rows centered
    /// on the playfield, its answers 20 rows each below the question
    /// (`0x11B50`, `0x11BCF`).
    fn answer(g: &mut DunkleSchatten, yes: bool) {
        let y0 = (128 - 60) / 2;
        let y = y0 + if yes { 30 } else { 50 };
        crate::titles::Runtime::mouse_move(g, 160, y);
        crate::titles::Runtime::tick(g);
        crate::titles::Runtime::click(g, true);
        crate::titles::Runtime::tick(g);
    }

    /// A fresh directory for saves.
    fn save_dir(name: &str) -> PathBuf {
        motionvm_parsec_testutil::saves_dir(&format!("ds1-{name}"))
    }

    /// The options `XP01` opens at its start, not yet dissolved in, on a
    /// machine whose dissolve takes `draws`.
    fn start_options_on(draws: u32) -> Option<DunkleSchatten> {
        let dir = gamedata_or_skip()?;
        let mut g = DunkleSchatten::open(&dir).expect("open Dunkle Schatten");
        g.start_in_xp01(Start::New).expect("start");
        assert!(g.inner.options_fading(), "the start options dissolving in");
        g.inner.dissolve_draws = draws;
        Some(g)
    }

    /// The dissolve draws from the game's generator (`0x12091`,
    /// `1EB5h:0028h(0, 5000h)`), once a pass, and the machine says how
    /// many passes there are: with a count of 1000 the seed has moved
    /// 1000 steps when the menu takes input, and every one of them by
    /// the time the hundred hundredths are over.
    #[test]
    fn the_dissolve_draws_the_machines_count_from_the_games_generator() {
        let Some(mut g) = start_options_on(1000) else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let (lo, hi) = g.inner.seed_for_test();
        let mut want = crate::titles::rng::LcRng { lo, hi };
        for _ in 0..1000 {
            want.inclusive(0, 0x5000);
        }
        until_the_options_take_input(&mut g);
        assert!(g.inner.clock.hundredths_1c82() >= 100);
        assert_eq!(g.inner.seed_for_test(), (want.lo, want.hi));
        // A count of none is still one pass: the loop tests at its foot.
        let Some(mut g) = start_options_on(0) else {
            return;
        };
        until_the_options_take_input(&mut g);
        let mut one = crate::titles::rng::LcRng { lo, hi };
        one.inclusive(0, 0x5000);
        assert_eq!(g.inner.seed_for_test(), (one.lo, one.hi));
    }

    /// On the reference machine the start options leave the generator
    /// where the original's left it there: `7698h:E605h` at `1EB5h:0006h`
    /// after the loop, from `0000:0000` before it.
    #[test]
    fn the_start_options_leave_the_seed_the_reference_machine_left() {
        let Some(mut g) = start_options_on(parsec1994::REFERENCE_DISSOLVE_DRAWS) else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        assert_eq!(g.inner.seed_for_test(), (0, 0));
        until_the_options_take_input(&mut g);
        assert_eq!(g.inner.seed_for_test(), (0xE605, 0x7698));
    }

    /// The entry's pass with no ticks (`0x0EC50`) steps the bed, whose
    /// animation the Zimmer has just started, before the entry effect
    /// plays: on the tick that closes the start options the generator
    /// has given the bed's two rolls, the wait of its walk's frame
    /// (`19D2h(28h, 50h)`) and the step (`19D2h(−15, 31)`), while the
    /// effect is still to come. The original draws them in that order,
    /// `0..=40` then `0..=46`, as the first two calls after the options.
    #[test]
    fn the_entry_steps_a_started_animation_before_its_effect() {
        let Some(mut g) = start_options_on(parsec1994::REFERENCE_DISSOLVE_DRAWS) else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        until_the_options_take_input(&mut g);
        let (lo, hi) = g.inner.seed_for_test();
        let mut want = crate::titles::rng::LcRng { lo, hi };
        want.inclusive(0, 40);
        want.inclusive(0, 46);
        crate::titles::Runtime::key(&mut g, 27);
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.location(), 2, "the Zimmer");
        assert!(g.inner.effects.busy(), "the entry effect still to play");
        assert_eq!(g.inner.seed_for_test(), (want.lo, want.hi));
    }

    /// Escape in game asks the gate in place with 1, locks the input and
    /// puts the options' gate up in mode 7 (`0x0FD20`–`0x0FD63`); the step
    /// asks it, the gate stops task 0 and opens the menu. Escape closes
    /// it, the gate's tail resets the machine and lets the input go, and
    /// the visit goes on where it was.
    #[test]
    fn escape_in_game_opens_the_options_through_their_gate() {
        let Some(mut g) = idle_in_the_wohnzimmer() else {
            return;
        };
        crate::titles::Runtime::key(&mut g, 27);
        assert_eq!(g.inner.gate, parsec1994::Gate::Options);
        assert_eq!((g.inner.verb, g.inner.mode), (6, 7));
        until_the_options_take_input(&mut g);
        crate::titles::Runtime::key(&mut g, 27);
        crate::titles::Runtime::tick(&mut g);
        assert!(!g.inner.options_up());
        assert_eq!(g.inner.gate, parsec1994::Gate::None);
        assert_eq!((g.inner.verb, g.inner.mode), (0, 0));
        assert_eq!(g.inner.bits & 2, 0, "the input let go");
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.location(), 4, "still in the Wohnzimmer");
    }

    /// The sixth verb picked on the strip goes straight to mode 5
    /// (`0x0FC3A`); the figure standing, its rule `0EB9h:04BDh` installs
    /// the options' gate and answers 9 (`0x0F04D`), and the step asks the
    /// gate.
    #[test]
    fn the_sixth_verb_opens_the_options_in_game() {
        let Some(mut g) = idle_in_the_wohnzimmer() else {
            return;
        };
        let at = (0..320)
            .flat_map(|x| (128..200).map(move |y| (x, y)))
            .find(|&(x, y)| {
                g.inner
                    .panel_widgets
                    .hit(x, y)
                    .and_then(crate::titles::panel::Panel::verb_of)
                    == Some(6)
            })
            .expect("the strip has a sixth verb");
        crate::titles::Runtime::mouse_move(&mut g, at.0, at.1);
        crate::titles::Runtime::click(&mut g, true);
        assert_eq!(g.inner.mode, 5);
        until_the_options_take_input(&mut g);
        assert!(g.inner.options_up());
    }

    /// Each row's setting, and what it does to the engine on the close:
    /// the text row's four words are the program's row for that speed
    /// (`DS:14CEh + 8·i`, `0x125E5`), and they time the next page; the walk
    /// row's byte (`DS:14E6h + i`) is the player's factor per edge; the
    /// mouse row is the setting `[16F3h]` and its divisors at once
    /// (`0x1257B`, `147Ah:05DCh`).
    #[test]
    fn the_options_set_what_the_engine_runs_with() {
        let Some(mut g) = idle_in_the_wohnzimmer() else {
            return;
        };
        crate::titles::Runtime::key(&mut g, 27);
        until_the_options_take_input(&mut g);
        assert_eq!(option_row(&g, 3).1, 1, "text: normal");
        assert_eq!(option_row(&g, 4).1, 0, "walk: slow, the 0Eh the start set");
        click_option_row(&mut g, 3);
        click_option_row(&mut g, 4);
        click_option_row(&mut g, 2);
        assert_eq!(option_row(&g, 3).1, 2, "text: fast");
        assert_eq!(option_row(&g, 4).1, 1, "walk: normal");
        // The mouse at once: setting 2, divisors 40 and 40.
        assert_eq!(g.inner.word(0x16F3), 2);
        assert_eq!((g.inner.word(0x03DC), g.inner.word(0x03DA)), (40, 40));
        crate::titles::Runtime::key(&mut g, 27);
        crate::titles::Runtime::tick(&mut g);
        assert!(!g.inner.options_up());
        let fast: Vec<u16> = (0..4).map(|i| g.inner.word(0x14CE + 16 + 2 * i)).collect();
        let now: Vec<u16> = (0..4).map(|i| g.inner.word(0x167D + 2 * i)).collect();
        assert_eq!(now, fast);
        let t = g.inner.timing();
        assert_eq!(
            [t.base, t.per_line, t.per_char, t.max],
            [fast[0], fast[1], fast[2], fast[3]]
        );
        let normal = g.inner.byte(0x14E6 + 1);
        assert_eq!(g.inner.byte(0x16A7), normal);
        assert_eq!(
            g.inner.walker.as_ref().map(|w| w.gait.speed),
            Some(i32::from(normal))
        );
        assert_eq!(g.inner.word(0x1ED2), 1, "the settings want writing");
    }

    /// The walk row skips `****` unless the command line's `#SpeedHigh`
    /// set `[1EDCh]` bit 4 (`0x1236F`).
    #[test]
    fn the_fourth_walk_speed_is_the_developers() {
        let Some(mut g) = idle_in_the_wohnzimmer() else {
            return;
        };
        crate::titles::Runtime::key(&mut g, 27);
        until_the_options_take_input(&mut g);
        for want in [1, 2, 0, 1, 2] {
            click_option_row(&mut g, 4);
            assert_eq!(option_row(&g, 4).1, want);
        }
        g.inner.set_word(0x1EDC, 0x10);
        click_option_row(&mut g, 4);
        assert_eq!(option_row(&g, 4).1, 3);
    }

    /// Saving from the options, then loading: the file is the buffer, 8B1h
    /// bytes under a header whose sum and xor the start's load accepts;
    /// "load" reads no file but goes through location 60, whose handler
    /// copies the blocks back and returns to the location saved in.
    #[test]
    fn a_game_saved_and_loaded_is_the_game_it_was() {
        let Some(mut g) = idle_in_the_wohnzimmer() else {
            return;
        };
        let dir = save_dir("roundtrip");
        crate::titles::Runtime::set_saves(&mut g, &dir).unwrap();
        crate::titles::Runtime::seed(&mut g, 0x000C_1234);
        // Something to keep: a story byte, an item, an object row.
        g.inner.sav_set(0x2700, 0x5A);
        g.inner.give_item(3);
        g.inner.location_records[9][4] = 0x21;
        let items = g.inner.items.clone();
        // The figure somewhere else than where the entry put it, facing
        // left.
        let here = g.inner.walker.as_ref().unwrap().dest_id;
        let other = g
            .inner
            .room()
            .and_then(|r| r.walk())
            .and_then(|gr| {
                gr.nodes
                    .iter()
                    .map(|n| num::signed(n.id))
                    .find(|&id| id != here && id < 0x7D0)
            })
            .expect("a second node");
        assert!(g.inner.snap_player(other, 1));
        let stood = {
            let w = g.inner.walker.as_ref().unwrap();
            (w.dest_id, w.facing(), w.x, w.y)
        };
        crate::titles::Runtime::key(&mut g, 27);
        // The ticks from the seed to the save: the options coming up, two
        // for the row and two for the answer, whose last writes the file.
        let ticks = until_the_options_take_input(&mut g) + 4;
        click_option_row(&mut g, 5);
        answer(&mut g, true);
        assert!(!g.inner.options_up(), "a save closes the options");
        let file = std::fs::read(dir.join("SAVEGAME.SAV")).expect("the file is written");
        assert_eq!(file.len(), 0x8B1);
        assert_eq!(&file[..8], b"DSchSAV\0");
        assert_eq!(
            &file[0x0A..0x0E],
            &bios_after(0x000C_1234, ticks).to_le_bytes(),
            "the BIOS tick, counted on from the seed"
        );
        assert_eq!(
            file[0x7F9..0x801],
            [1, 0, 4, 0, 0, 0, 0, 0],
            "copied while pushed"
        );
        let mut rng = crate::titles::rng::LcRng::default();
        assert!(parsec1994::sav_valid(&file, &mut rng));
        assert_eq!(
            (rng.lo, rng.hi),
            (g.inner.seed_for_test().0, g.inner.seed_for_test().1),
            "the save reseeded the game's generator and drew two"
        );
        // Changed after the save, and undone by the load.
        g.inner.sav_set(0x2700, 0);
        g.inner.take_item(3);
        g.inner.location_records[9][4] = 0;
        for _ in 0..50 {
            crate::titles::Runtime::tick(&mut g);
        }
        crate::titles::Runtime::key(&mut g, 27);
        until_the_options_take_input(&mut g);
        click_option_row(&mut g, 6);
        answer(&mut g, true);
        assert!(!g.inner.options_up());
        assert_eq!(
            (g.inner.location_word, g.inner.entry_word),
            (60, 1),
            "the main loop sent to location 60 (`0x14CBF`)"
        );
        for _ in 0..1500 {
            crate::titles::Runtime::tick(&mut g);
            if g.inner.here_word == 4 && g.inner.entry_word == 0 {
                break;
            }
        }
        assert_eq!(g.inner.location(), 4);
        assert_eq!(g.inner.sav_byte(0x2700), 0x5A);
        assert_eq!(g.inner.items, items);
        assert_eq!(g.inner.location_records[9][4], 0x21);
        // The entry −1 puts the figure back where its record says
        // (`0x0EFBC`).
        let w = g.inner.walker.as_ref().unwrap();
        assert_eq!((w.dest_id, w.facing(), w.x, w.y), stood);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The start's load: a saved game in the directory is read, checked and
    /// restored before the options come up, and the main loop then enters
    /// the location it was saved in with entry −1; one byte changed and the
    /// sum does not hold, which the original ends the program on and
    /// this engine shows; a file of another size is passed over.
    #[test]
    fn the_start_loads_a_saved_game_and_refuses_a_bad_one() {
        let Some(mut g) = idle_in_the_wohnzimmer() else {
            return;
        };
        let dir = save_dir("start");
        crate::titles::Runtime::set_saves(&mut g, &dir).unwrap();
        g.inner.sav_set(0x2701, 0x77);
        let tick = 0x0001_0203;
        let bytes = g
            .inner
            .save_044e(&SaveMap::DS1, tick)
            .expect("in a location, a save is written");
        std::fs::write(dir.join("SAVEGAME.SAV"), &bytes).unwrap();

        let open = |dir: &Path, how: Start| {
            let game = motionvm_parsec_testutil::gamedata_ds1().unwrap();
            let mut g = DunkleSchatten::open(&game).unwrap();
            crate::titles::Runtime::set_saves(&mut g, dir).unwrap();
            g.start_in_xp01(how).unwrap();
            g
        };
        let mut c = open(&dir, Start::Continue);
        assert_eq!(c.inner.word(0x1744), 1, "a game in the buffer");
        assert_eq!(c.inner.sav_byte(0x2701), 0x77);
        assert_eq!((c.inner.location_word, c.inner.entry_word), (4, 0xFFFF));
        let mut seed = crate::titles::rng::LcRng {
            lo: 0x0203,
            hi: 0x0001,
        };
        seed.inclusive(1, 0xFFFE);
        seed.inclusive(1, 0xFFFE);
        assert_eq!(
            c.inner.seed_for_test(),
            (seed.lo, seed.hi),
            "seeded by the file's tick"
        );
        close_start_options(&mut c);
        assert_eq!(c.inner.location(), 4);

        let n = open(&dir, Start::New);
        assert_eq!(n.inner.word(0x1744), 0);
        assert_eq!(n.inner.location_word, 2);

        let mut bad = bytes.clone();
        bad[0x100] ^= 1;
        std::fs::write(dir.join("SAVEGAME.SAV"), &bad).unwrap();
        let b = open(&dir, Start::Continue);
        assert_eq!(b.inner.word(0x1744), 0);
        assert_eq!(b.inner.location_word, 2, "a fresh game instead");
        let invalid = b.inner.save_message(SaveMap::DS1.msg_invalid, "");
        assert_eq!(b.inner.notice.as_deref(), Some(invalid.as_str()));

        std::fs::write(dir.join("SAVEGAME.SAV"), &bytes[..0x8B0]).unwrap();
        let s = open(&dir, Start::Continue);
        assert_eq!((s.inner.word(0x1744), s.inner.notice.is_none()), (0, true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Quitting through the options: two questions, then the main loop
    /// ends on `[1F4Ah]` = −2 (`0x12403`) and the settings are written
    /// because they changed (`0x4FF0`) — and the start takes them back.
    #[test]
    fn quitting_writes_the_settings_and_the_next_start_reads_them() {
        let Some(mut g) = idle_in_the_wohnzimmer() else {
            return;
        };
        let dir = save_dir("quit");
        crate::titles::Runtime::set_saves(&mut g, &dir).unwrap();
        crate::titles::Runtime::seed(&mut g, 0x0009_8765);
        crate::titles::Runtime::key(&mut g, 27);
        // Every tick from the seed to the end counts: the last one, which
        // answers `false`, is the one whose interrupt ran before the main
        // loop ended and the settings took the tick.
        let mut ticks = until_the_options_take_input(&mut g) + 8;
        click_option_row(&mut g, 2);
        click_option_row(&mut g, 7);
        answer(&mut g, true);
        assert!(g.inner.options_up(), "the second question");
        answer(&mut g, false);
        assert!(!g.inner.options_up());
        let mut over = false;
        for _ in 0..1000 {
            ticks += 1;
            if !crate::titles::Runtime::tick(&mut g) {
                over = true;
                break;
            }
        }
        assert!(over, "the game is over");
        let tick = bios_after(0x0009_8765, ticks);
        assert!(!dir.join("SAVEGAME.SAV").exists(), "no, not saved");
        let cfg = std::fs::read(dir.join("SAVEGAME.CFG")).expect("the settings");
        assert_eq!(cfg.len(), 0x1F);
        assert_eq!(&cfg[..8], b"DSchCFG\0");
        assert_eq!(&cfg[0x0A..0x0E], &tick.to_le_bytes(), "the BIOS tick");
        assert_eq!(u16::from_le_bytes([cfg[0x12], cfg[0x13]]), 2, "the mouse");
        assert_eq!(cfg[0x1C], 0x0E, "the walk speed the start set");

        let game = motionvm_parsec_testutil::gamedata_ds1().unwrap();
        let mut n = DunkleSchatten::open(&game).unwrap();
        crate::titles::Runtime::set_saves(&mut n, &dir).unwrap();
        n.start_in_xp01(Start::Continue).unwrap();
        assert_eq!(n.inner.word(0x16F3), 2);
        assert_eq!((n.inner.word(0x03DC), n.inner.word(0x03DA)), (40, 40));
        assert_eq!(
            n.inner.seed_for_test(),
            (
                u16::try_from(tick & 0xFFFF).unwrap(),
                u16::try_from(tick >> 16).unwrap()
            ),
            "seeded from the file's tick, not rolled"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Nobody seeded this machine, so its BIOS tick starts at 0: the
    /// engine reads no clock, and a run without a seed is the same run
    /// every time.
    #[test]
    fn without_a_seed_the_bios_tick_starts_at_0() {
        let Some(dir) = gamedata_or_skip() else {
            return;
        };
        let mut g = DunkleSchatten::open(&dir).expect("open Dunkle Schatten");
        assert_eq!(g.inner.bios.count(), 0);
        g.start_in_xp01(Start::Continue).expect("start");
        for _ in 0..6 {
            crate::titles::Runtime::tick(&mut g);
        }
        assert_eq!(g.inner.bios.count(), 0, "six ticks are 59 652 cycles");
        crate::titles::Runtime::tick(&mut g);
        assert_eq!(g.inner.bios.count(), 1, "the seventh carries");
    }

    /// A save directory inside the install is refused.
    #[test]
    fn saves_do_not_go_into_the_install() {
        let Some(mut g) = ds1_or_skip() else {
            return;
        };
        let game = motionvm_parsec_testutil::gamedata_ds1().unwrap();
        assert!(crate::titles::Runtime::set_saves(&mut g, &game.join("SAVES")).is_err());
    }

    /// The files the original wrote, from `MOTIONVM_SAVES`: the save's
    /// sum, xor and magic hold under this reading, the settings' check
    /// value too, and the start takes both — the save made on the map, so
    /// the main loop goes back to location 1. Saved again with the file's
    /// own tick, every block the restore brings back comes out as the
    /// original wrote it, the player's record whole — the entry −1 put the
    /// figure back on its node, and that changed none of its bytes.
    #[test]
    fn the_originals_saved_game_and_settings_are_read() {
        let Some(dir) = motionvm_parsec_testutil::saves_dir_named() else {
            eprintln!("skipping: MOTIONVM_SAVES names no directory");
            return;
        };
        let Some(sav) = motionvm_parsec_formats::find_ci(&dir, "SAVEGAME.SAV") else {
            eprintln!("skipping: no SAVEGAME.SAV in {}", dir.display());
            return;
        };
        let Some(game) = gamedata_or_skip() else {
            return;
        };
        let file = std::fs::read(&sav).unwrap();
        let mut rng = crate::titles::rng::LcRng::default();
        assert!(
            parsec1994::sav_valid(&file, &mut rng),
            "the original's checksum"
        );
        let cfg = motionvm_parsec_formats::find_ci(&dir, "SAVEGAME.CFG")
            .map(|p| std::fs::read(p).unwrap());
        if let Some(cfg) = &cfg {
            assert!(matches!(
                parsec1994::cfg_read(cfg, &mut rng),
                parsec1994::Cfg::Taken { .. }
            ));
        }
        // A copy of the directory, so that nothing writes beside the
        // original's files.
        let work = save_dir("resaved");
        std::fs::copy(&sav, work.join("SAVEGAME.SAV")).unwrap();
        if let Some(cfg) = &cfg {
            std::fs::write(work.join("SAVEGAME.CFG"), cfg).unwrap();
        }
        let mut g = DunkleSchatten::open(&game).unwrap();
        crate::titles::Runtime::set_saves(&mut g, &work).unwrap();
        g.start_in_xp01(Start::Continue).unwrap();
        assert_eq!(g.inner.word(0x1744), 1);
        let saved_in = g.inner.location_word;
        close_start_options(&mut g);
        assert_eq!(g.inner.location(), i32::from(saved_in));
        assert!(g.inner.notice.is_none());
        // The same game saved again, with the file's tick.
        let tick = u32::from_le_bytes([file[0x0A], file[0x0B], file[0x0C], file[0x0D]]);
        let again = g.inner.save_044e(&SaveMap::DS1, tick).unwrap();
        assert_eq!(
            &again[..0x0E],
            &file[..0x0E],
            "magic, version word and tick"
        );
        for (name, at, n) in [
            ("story", 0x14, 0x45),
            ("objects", 0x59, 0x7A0),
            ("stack", 0x7F9, 8),
            ("player", 0x801, 0x30),
            ("carried", 0x831, 0x23),
        ] {
            assert_eq!(again[at..at + n], file[at..at + n], "{name}");
        }
        // What differs is what the restore does not take back: of the 7Fh
        // bytes from `DS:15FEh` the 5Ch past its 23h, the far pointers to
        // the items' cels at `DS:1621h` (`0x0C29F`), which the original
        // fills from its heap and this engine has no addresses for. With
        // those, the sum and xor differ too.
        // The settings, written again with their own tick: the same 31
        // bytes.
        if let Some(cfg) = &cfg {
            g.inner.set_word(SaveMap::DS1.cfg_dirty, 1);
            let tick = u32::from_le_bytes([cfg[0x0A], cfg[0x0B], cfg[0x0C], cfg[0x0D]]);
            assert_eq!(
                g.inner.cfg_0373(&SaveMap::DS1, tick).as_deref(),
                Some(&cfg[..])
            );
        }
        let differs: Vec<usize> = (0x14..0x8B1).filter(|&i| again[i] != file[i]).collect();
        assert!(
            differs.iter().all(|&i| (0x854..0x8B0).contains(&i)),
            "{differs:x?}"
        );
        let _ = std::fs::remove_dir_all(&work);
    }

    /// The player's record at `DS:1693h` in the image (`03 01 01 02 …`):
    /// standing on no node, block 0, base 0.
    fn image_record(g: &DunkleSchatten) -> Vec<u8> {
        let at = g.inner.image.at(0x1693);
        g.inner.image.bytes()[at..at + 0x30].to_vec()
    }

    /// A record whose `+0Ch` and `+2Ch` disagree: the image's with the
    /// factor 08h, node `CEh` of the map and block 0Eh (facing 2) — what an
    /// earlier build of this engine wrote, `+2Ch` from the facing and the
    /// rest the image's.
    fn disagreeing_record(g: &DunkleSchatten) -> Vec<u8> {
        let mut r = image_record(g);
        r[0x14] = 0x08;
        r[0x28..0x2A].copy_from_slice(&0xCEu16.to_le_bytes());
        r[0x2C..0x2E].copy_from_slice(&0x0Eu16.to_le_bytes());
        r
    }

    /// A record the original saved on the map: facing 3 (block 15h, base
    /// 7, mirrored), on node `CBh`, with three far pointers into its heap —
    /// the `AN` entry at `+18h`, nothing at `+20h`, the walk graph at
    /// `+24h` — which this engine carries as they are.
    const SAVED_ON_THE_MAP: [u8; 0x30] = [
        0x03, 0x01, 0x41, 0x02, 0x8C, 0x00, 0x2D, 0x00, 0x01, 0x06, 0x0D, 0x00, 0x07, 0x00, 0x70,
        0x00, 0x70, 0x00, 0x00, 0xFF, 0x0E, 0x06, 0x00, 0x00, 0x00, 0x00, 0x38, 0x48, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0xA1, 0x5C, 0xCB, 0x00, 0x00, 0x00, 0x15,
        0x00, 0xFF, 0xFF,
    ];

    fn write_record(g: &mut DunkleSchatten, r: &[u8]) {
        for (i, &b) in r.iter().enumerate() {
            g.inner.set_byte(0x1693 + u16::try_from(i).unwrap(), b);
        }
    }

    /// The record is the walker, both ways: 30h bytes written through the
    /// data segment, one at a time as the restore's block copy writes them
    /// (`0x14D0C`), read back as they were — with the figure's walker in
    /// place and before there is one — and the walker holds what they say.
    #[test]
    fn the_players_record_is_the_walker_both_ways() {
        let mut g = ds1!();
        assert!(g.inner.walker.is_some(), "the Zimmer placed the figure");
        let records = [
            image_record(&g),
            disagreeing_record(&g),
            SAVED_ON_THE_MAP.to_vec(),
        ];
        for r in &records {
            for walker in [true, false] {
                if !walker {
                    g.inner.walker = None;
                }
                write_record(&mut g, r);
                assert_eq!(&g.inner.player_record_bytes(), r, "walker {walker}");
                let w = g.inner.take_player();
                g.inner.walker = Some(w);
                assert_eq!(&g.inner.player_record_bytes(), r, "made from the bytes");
            }
        }
        // What the walker makes of them.
        let r = disagreeing_record(&g);
        write_record(&mut g, &r);
        let w = g.inner.walker.as_ref().unwrap();
        assert_eq!((w.facing(), w.base, w.dest_id), (2, 0, 0xCE));
        assert_eq!(w.cel(), (0, false), "+0Ah as given, 0");
        write_record(&mut g, &SAVED_ON_THE_MAP);
        let w = g.inner.walker.as_ref().unwrap();
        assert_eq!((w.facing(), w.base, w.block, w.mirror), (3, 7, 0x15, true));
        assert_eq!((w.x, w.y, w.scale, w.phase), (140, 45, 0x70, 6));
        assert_eq!((w.dest_id, w.final_facing, w.gait.speed), (0xCB, -1, 0x0E));
        assert_eq!(w.cel(), (13, true));
        assert!(w.cel_held.is_none(), "+0Ah is +0Ch plus +9's standing cel");
        // A byte the logic writes alone lands among the rest: the live
        // bit, then the node's low bit (`0x3412`, `0x0C00`).
        let flags = g.inner.byte(0x1695);
        g.inner.set_byte(0x1695, flags & 0xFE);
        g.inner.set_word(0x16BB, g.inner.word(0x16BB) | 1);
        let mut want = SAVED_ON_THE_MAP;
        want[2] = 0x40;
        want[0x28] = 0xCB;
        assert_eq!(g.inner.player_record_bytes(), want);
        assert_eq!(g.inner.player_node_id(), 0xCB);
    }

    /// A save file made here with the player's record replaced by `rec`,
    /// its sum and xor made to hold again.
    fn with_record(file: &[u8], rec: &[u8]) -> Vec<u8> {
        let mut f = file.to_vec();
        f[0x801..0x831].copy_from_slice(rec);
        let mut rng = crate::titles::rng::LcRng::default();
        let (sum, xor) = parsec1994::sav_check(&f, &mut rng);
        f[0x0E..0x12].copy_from_slice(&sum.to_le_bytes());
        f[0x12..0x14].copy_from_slice(&xor.to_le_bytes());
        f
    }

    /// The map entered with −1 from a saved game whose `rec` stands at
    /// `+801h`: the figure as the entry put it.
    fn restored_on_the_map(rec: &[u8]) -> Option<crate::titles::walker::Walker> {
        let mut g = ds1_or_skip()?;
        crate::titles::Runtime::set_location_via(&mut g, 1, 0x6D);
        settle_effects(&mut g);
        let file = g
            .inner
            .save_044e(&SaveMap::DS1, 0x0001_0000)
            .expect("in a location, a save is written");
        let dir = save_dir(&format!("restored-{:02x}", rec[0x0C]));
        std::fs::write(dir.join("SAVEGAME.SAV"), with_record(&file, rec)).unwrap();
        let mut g =
            DunkleSchatten::open(&motionvm_parsec_testutil::gamedata_ds1().expect("the install"))
                .unwrap();
        crate::titles::Runtime::set_saves(&mut g, &dir).unwrap();
        g.start_in_xp01(Start::Continue).unwrap();
        close_start_options(&mut g);
        assert_eq!(g.inner.location(), 1);
        assert!(g.inner.notice.is_none());
        let _ = std::fs::remove_dir_all(&dir);
        g.inner.walker.clone()
    }

    /// The entry −1 snaps the figure onto `+28h` facing `+2Ch / (+15h + 1)`
    /// (`0x0BAA5`), and the snap's turn does nothing when that is the
    /// block `+2Ch` already holds (`0x0ABF1`): the cel stands on the
    /// restored `+0Ch`, `+0Ch + +15h`. A record whose `+0Ch` (0) and `+2Ch`
    /// (0Eh, facing 2) disagree comes back on cel 6, block 0's standing
    /// cel — the figure from behind — where one whose `+0Ch` is its block
    /// comes back on cel 20, from the front.
    #[test]
    fn a_restored_figure_stands_on_its_records_own_cel() {
        let Some(g) = ds1_or_skip() else {
            return;
        };
        let rec = disagreeing_record(&g);
        let Some(w) = restored_on_the_map(&rec) else {
            return;
        };
        let node = g.inner.rooms[1]
            .as_ref()
            .and_then(|r| r.walk())
            .and_then(|gr| gr.nodes.iter().find(|n| n.id == 0xCE).cloned())
            .expect("node CEh of the map");
        assert_eq!((w.x, w.y), (i32::from(node.x), i32::from(node.y)));
        assert_eq!((w.dest_id, w.facing(), w.base), (0xCE, 2, 0));
        assert_eq!(w.cel(), (6, false), "block 0's standing cel");
        let mut agreeing = rec;
        agreeing[0x0C] = 0x0E;
        let Some(w) = restored_on_the_map(&agreeing) else {
            return;
        };
        assert_eq!((w.dest_id, w.facing(), w.base), (0xCE, 2, 0x0E));
        assert_eq!(w.cel(), (20, false), "block 2's standing cel");
    }

    /// Played to the map the way the original's saved game was made — a
    /// fresh game, the start's options closed, the Küche's second answer —
    /// saved from the options, and loaded into a fresh program from the
    /// file: the figure stands where it stood, facing as it faced, on the
    /// same cel, and the record is the one the file holds. Against the
    /// original's own save (`MOTIONVM_SAVES`) the record is the same but
    /// for the three far pointers into the original's heap.
    #[test]
    fn a_game_saved_on_the_map_is_loaded_with_the_figure_as_it_stood() {
        let mut g = ds1!();
        play_opening_answering(&mut g, 2).expect("the map is reached");
        for _ in 0..2000 {
            crate::titles::Runtime::tick(&mut g);
            if g.inner.walker.as_ref().is_some_and(|w| !w.walking) && g.inner.player_node_id() >= 0
            {
                break;
            }
        }
        let stood = {
            let w = g.inner.walker.as_ref().unwrap();
            (w.dest_id, w.facing(), w.x, w.y, w.scale, w.cel())
        };
        let before = g.inner.player_record_bytes();
        let dir = save_dir("map-figure");
        crate::titles::Runtime::set_saves(&mut g, &dir).unwrap();
        crate::titles::Runtime::key(&mut g, 27);
        until_the_options_take_input(&mut g);
        click_option_row(&mut g, 5);
        answer(&mut g, true);
        let file = std::fs::read(dir.join("SAVEGAME.SAV")).expect("the file is written");
        assert_eq!(file[0x801..0x831], before[..], "the live record");

        let mut h =
            DunkleSchatten::open(&motionvm_parsec_testutil::gamedata_ds1().unwrap()).unwrap();
        crate::titles::Runtime::set_saves(&mut h, &dir).unwrap();
        h.start_in_xp01(Start::Continue).unwrap();
        close_start_options(&mut h);
        assert_eq!(h.inner.location(), 1);
        let w = h.inner.walker.as_ref().unwrap();
        assert_eq!((w.dest_id, w.facing(), w.x, w.y, w.scale, w.cel()), stood);
        assert_eq!(h.inner.player_record_bytes(), before);
        let _ = std::fs::remove_dir_all(&dir);

        let Some(sav) = motionvm_parsec_testutil::saves_dir_named()
            .and_then(|d| motionvm_parsec_formats::find_ci(&d, "SAVEGAME.SAV"))
        else {
            eprintln!(
                "skipping the comparison with a saved game: MOTIONVM_SAVES names no SAVEGAME.SAV"
            );
            return;
        };
        let original = std::fs::read(sav).unwrap();
        let heap = |i: usize| (0x18..0x1C).contains(&i) || (0x20..0x28).contains(&i);
        let differs: Vec<usize> = (0..0x30)
            .filter(|&i| !heap(i) && before[i] != original[0x801 + i])
            .collect();
        assert!(differs.is_empty(), "{differs:x?}: {before:02x?}");
    }

    /// A host's jump while the start's options are up takes them down and
    /// puts back what they hold: the strip's colors and the pointer's mode.
    #[test]
    fn a_jump_takes_the_start_options_down() {
        let Some(game) = gamedata_or_skip() else {
            return;
        };
        let mut g = DunkleSchatten::open(&game).unwrap();
        g.start_in_xp01(Start::New).unwrap();
        for _ in 0..200 {
            crate::titles::Runtime::tick(&mut g);
        }
        assert!(g.inner.options_up());
        assert_eq!(g.inner.status_dim & 2, 2, "the strip held by the options");
        crate::titles::Runtime::set_location_via(&mut g, 4, 0x7F);
        assert!(!g.inner.options_up());
        for _ in 0..400 {
            crate::titles::Runtime::tick(&mut g);
        }
        assert_eq!(g.inner.status_dim & 2, 0, "let go");
        assert_ne!(g.inner.cursor.as_ref().map(|c| c.mode), Some(2));
        assert_eq!(g.inner.location(), 4);
        // The same jump without the options having been up: the same
        // picture and the same colors, the clock's phase aside.
        let mut h = DunkleSchatten::open(&game).unwrap();
        crate::titles::Runtime::set_location_via(&mut h, 4, 0x7F);
        for _ in 0..400 {
            crate::titles::Runtime::tick(&mut h);
        }
        assert_eq!(g.inner.palette(), h.inner.palette());
        assert_eq!(g.inner.status_dim, h.inner.status_dim);
    }

    /// The title program begun the way the host begins play, with
    /// `dir` as the save directory, and taken to its menu: a click skips
    /// the intro.
    fn title_menu(dir: Option<&Path>) -> Option<DunkleSchatten> {
        use crate::titles::Runtime;
        let game = gamedata_or_skip()?;
        let mut g = DunkleSchatten::open(&game).unwrap();
        if let Some(d) = dir {
            g.set_saves(d).unwrap();
        }
        g.start(Start::New).unwrap();
        g.click(true);
        while !g.xp00.as_ref().is_some_and(|x| x.menu_up()) {
            assert!(g.tick());
        }
        Some(g)
    }

    /// Clicks line `i` of the title program's menu.
    fn choose(g: &mut DunkleSchatten, i: usize) {
        use crate::titles::Runtime;
        let (x, y, _, _) = g.xp00.as_ref().unwrap().menu_box(i);
        g.mouse_move(x + 1, y + 1);
        g.click(true);
    }

    /// Ticks while the title program runs.
    fn through_xp00(g: &mut DunkleSchatten) {
        use crate::titles::Runtime;
        let mut n = 0;
        while g.xp00.is_some() && g.tick() {
            if g.xp00.as_ref().is_some_and(|x| x.page().is_some()) {
                g.key(32);
            }
            n += 1;
            assert!(n < 5000);
        }
    }

    /// Play begins in the title program, a frame a retrace of the screen;
    /// once it ends, `XP01` runs on its own clock and its options are up.
    #[test]
    fn play_begins_in_the_title_program() {
        use crate::titles::Runtime;
        let Some(mut g) = title_menu(None) else {
            return;
        };
        assert_eq!(g.frame_duration().as_nanos(), 14_268_123);
        assert!(!g.inner.options_up(), "XP01 has not started");
        choose(&mut g, 0);
        through_xp00(&mut g);
        assert!(g.xp00.is_none());
        assert_eq!(
            g.frame_duration(),
            Duration::from_nanos(parsec1994::Clock::tick_nanos())
        );
        assert!(g.inner.options_up(), "XP01's start opens the options");
    }

    /// "Neues Spiel beginnen" deletes `SAVEGAME.SAV` (XP00 `0x625`), and
    /// `XP01` then starts a fresh game.
    #[test]
    fn a_new_game_from_the_title_program_deletes_the_saved_game() {
        let dir = save_dir("xp00-new");
        let sav = dir.join("SAVEGAME.SAV");
        std::fs::write(&sav, [0u8; 16]).unwrap();
        let Some(mut g) = title_menu(Some(&dir)) else {
            return;
        };
        choose(&mut g, 0);
        through_xp00(&mut g);
        assert!(!sav.exists(), "deleted");
        assert_eq!(g.inner.word(0x1744), 0, "nothing loaded");
        close_start_options(&mut g);
        assert_eq!(g.inner.location(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// "Altes Spiel fortsetzen" leaves the saved game for `XP01` to load
    /// at its start — the original's own file, when the suite has one.
    #[test]
    fn continue_from_the_title_program_loads_the_saved_game() {
        let Some(orig) = motionvm_parsec_testutil::saves_dir_named()
            .and_then(|d| motionvm_parsec_formats::find_ci(&d, "SAVEGAME.SAV"))
        else {
            eprintln!("skipping: MOTIONVM_SAVES names no SAVEGAME.SAV");
            return;
        };
        let dir = save_dir("xp00-continue");
        std::fs::copy(&orig, dir.join("SAVEGAME.SAV")).unwrap();
        let Some(mut g) = title_menu(Some(&dir)) else {
            return;
        };
        choose(&mut g, 1);
        through_xp00(&mut g);
        assert_eq!(g.inner.word(0x1744), 1, "loaded (0x149D3)");
        assert!(dir.join("SAVEGAME.SAV").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Quitting in the title program ends the game before `XP01`: nothing
    /// is written, the settings least of all.
    #[test]
    fn quitting_the_title_program_ends_the_game() {
        use crate::titles::Runtime;
        let dir = save_dir("xp00-quit");
        let Some(mut g) = title_menu(Some(&dir)) else {
            return;
        };
        choose(&mut g, 2);
        through_xp00(&mut g);
        assert!(!g.tick(), "over");
        assert!(!g.inner.options_up());
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none(), "no file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A host's jump takes the title program down and starts `XP01`
    /// first, as a jump from `XP01`'s start did: the settings are read.
    #[test]
    fn a_jump_takes_the_title_program_down() {
        use crate::titles::Runtime;
        let dir = save_dir("xp00-jump");
        let Some(mut first) = title_menu(Some(&dir)) else {
            return;
        };
        // Settings with the slow walk (0Eh, `DS:14E6h`), written the way
        // the original writes them.
        let map = SaveMap::DS1;
        first.inner.set_byte(0x16A7, 0x0E);
        first.inner.set_word(0x1ED2, 1);
        let cfg = first.inner.cfg_0373(&map, 0x1234).expect("dirty");
        std::fs::write(dir.join("SAVEGAME.CFG"), cfg).unwrap();
        let Some(mut g) = title_menu(Some(&dir)) else {
            return;
        };
        g.set_location(3);
        assert!(g.xp00.is_none());
        assert!(!g.inner.options_up());
        assert_eq!(g.inner.location(), 3);
        assert_eq!(g.inner.byte(0x16A7), 0x0E, "the settings were read");
        assert_eq!(
            g.inner.music.sent,
            [
                parsec1994::Request::Install { effects: 3 },
                parsec1994::Request::EffectVolume(0x50),
                parsec1994::Request::Digital(Some(0)),
                parsec1994::Request::Play {
                    piece: 0,
                    loops: -1
                },
                parsec1994::Request::Stop,
                parsec1994::Request::Digital(None),
                parsec1994::Request::Reinstall,
                parsec1994::Request::Install { effects: 6 },
                parsec1994::Request::EffectVolume(0x50),
                parsec1994::Request::EffectVolume(0x3C),
            ],
            "the menu's song stopped with the title program, and XP01's drivers installed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The BIOS tick goes on while the title program runs: once each time
    /// the carry word passes 65 536 cycles, from 0 at its start.
    #[test]
    fn the_title_programs_time_counts_on_the_bios_tick() {
        let Some(mut g) = title_menu(None) else {
            return;
        };
        choose(&mut g, 2);
        let cycles = loop {
            let c = g.xp00.as_ref().unwrap().cycles();
            if !crate::titles::Runtime::tick(&mut g) {
                break c;
            }
        };
        assert!(g.xp00.is_none());
        assert!(cycles / 0x1_0000 > 0);
        assert_eq!(u64::from(g.inner.bios.count()), cycles / 0x1_0000);
    }

    /// The brochure answers the question chosen, and the questions come
    /// back.
    ///
    /// `Betrachte` on an item falls to the global rule `1586h:0125h`
    /// (image `0x15985`), which binds scene `~(item + 4000h)`: bank 2,
    /// place `item - 1`, place 18 for the brochure, item 19. Its choices
    /// are records 1 to 5; each question's answer is a page that starts
    /// `03 05 01 01`, the headline, `0A 01 00` and then the text. The
    /// `00` after that `01` is the control's argument — `13A7h:0023h`
    /// copies the pair without looking at it (`0x13AC8`) — so the answer
    /// goes on to its `0D 00`. The page waits for a click (`03 05`,
    /// `0x0DFFD`), and the record's next-list puts the questions up again.
    /// The answer is spoken by slot 10, and its box is laid out by that
    /// slot's payload.
    #[test]
    fn the_brochure_answers_the_question_chosen() {
        // On the map, where the owner's save stands.
        let mut g = ds1!();
        crate::titles::Runtime::set_location_via(&mut g, 1, 0x6D);
        settle_effects(&mut g);
        assert!(g.inner.carried(19), "a fresh game carries the brochure");
        settle(&mut g);
        g.inner.mode = 2;
        g.inner.verb = 1;
        g.inner.verb_chosen = 1;
        g.inner.object1 = 0;
        g.inner.object2 = 0;
        g.inner.two_object = 0;
        // Slot 0, widget `11Eh`, holds the one item carried.
        g.inner.mouse_move(0xC8 + 12, 0x92 + 12);
        crate::titles::Runtime::tick(&mut g);
        assert!(
            g.inner.status().contains("Broschüre"),
            "{:?}",
            g.inner.status()
        );
        assert!(g.inner.click());

        let mut menus = 0;
        let mut pages = Vec::new();
        for _ in 0..6000 {
            crate::titles::Runtime::tick(&mut g);
            let events = std::mem::take(&mut g.inner.events);
            for e in events {
                match e {
                    Event::MenuUp { lines } => {
                        menus += 1;
                        assert_eq!(lines.len(), 5, "four questions and enough");
                        assert_eq!(lines[0].1, "Wieviele Ausländer leben in Deutschland?");
                        // The first time the first question, then enough.
                        let pick = if menus == 1 { 1 } else { 5 };
                        for _ in 0..pick {
                            g.inner.menu_key(parsec1994::MenuKey::Down);
                        }
                        g.inner.menu_key(parsec1994::MenuKey::Select);
                    }
                    Event::PageUp { task: 0, text } if menus > 0 => pages.push(text),
                    _ => {}
                }
            }
            // The answer waits for a click.
            if text_up(&g) && g.inner.tasks.t[0].flags & 0x400 != 0 {
                // Slot 10 is the playfield's middle with code `53h`, and
                // its payload is the style word (`0x0D3DF`): the box
                // centered on (160, 64), its lines flush left — where a
                // DOSBox-X capture of the original has it, pixel for pixel.
                let t = &g.inner.tasks.t[0];
                assert_eq!((t.slot, t.point, t.style_word), (10, (160, 64), 0x53));
                let b = t.box_.as_ref().expect("the answer's box");
                assert_eq!((b.x, b.y, b.w, b.h, b.lines.len()), (43, 7, 235, 114, 9));
                g.inner.mouse_move(0, 0);
                assert!(g.inner.click());
            }
            if menus == 2 && g.inner.tasks.t[0].kind == 0 {
                break;
            }
        }
        assert_eq!(menus, 2, "the questions come back after the answer");
        assert_eq!(pages[0], "Wieviele Ausländer leben in Deutschland?");
        let answer = &pages[1];
        assert!(
            answer.starts_with(
                "\u{3}\u{5}\u{1}\u{1}WIEVIELE AUSLÄNDER LEBEN IN DEUTSCHLAND?\n\u{1}\0"
            ),
            "{answer:?}"
        );
        assert!(
            answer.contains("Ende 1993 lebten in der Bundesrepublik Deutschland")
                && answer.ends_with("Großbritannien: 4,2%."),
            "the answer is the whole page, got {answer:?}"
        );
        assert_eq!(
            g.inner.tasks.t[0].kind, 0,
            "enough read, the dialogue is over"
        );
    }

    /// What the driver was asked, and on which tick, while `ticks` ticks
    /// run.
    fn music_over(g: &mut DunkleSchatten, ticks: u32) -> Vec<(u32, parsec1994::Request)> {
        let mut out = Vec::new();
        for tick in 0..ticks {
            crate::titles::Runtime::tick(g);
            for r in g.inner.music.sent.drain(..) {
                out.push((tick, r));
            }
        }
        out
    }

    /// The title program's menu plays module 0 endlessly once it has faded
    /// in (`0x3854`), and a line chosen fades it over 2000 ms (`0x3AA2`)
    /// and stops it once the menu has faded out (`0x3B07`) — the stop some
    /// retraces after the fade. Then the program ends, and `XP01` installs
    /// the driver anew.
    #[test]
    fn the_title_menu_plays_module_0_and_fades_it_when_a_line_is_chosen() {
        use crate::titles::Runtime;
        let Some(mut g) = title_menu(None) else {
            return;
        };
        let sent = std::mem::take(&mut g.inner.music.sent);
        assert_eq!(
            sent,
            [
                parsec1994::Request::Install { effects: 3 },
                parsec1994::Request::EffectVolume(0x50),
                parsec1994::Request::Digital(Some(0)),
                parsec1994::Request::Play {
                    piece: 0,
                    loops: -1
                }
            ]
        );
        choose(&mut g, 2);
        let mut after = Vec::new();
        let mut n = 0;
        while g.tick() {
            for r in g.inner.music.sent.drain(..) {
                after.push((n, r));
            }
            n += 1;
            assert!(n < 1000);
        }
        for r in g.inner.music.sent.drain(..) {
            after.push((n, r));
        }
        let what: Vec<_> = after.iter().map(|&(_, r)| r).collect();
        assert_eq!(
            what[..5],
            [
                parsec1994::Request::FadeOut(2000),
                parsec1994::Request::Stop,
                parsec1994::Request::Digital(None),
                parsec1994::Request::StopEffects,
                parsec1994::Request::Reinstall
            ]
        );
        assert!(
            after[1].0 > after[0].0,
            "the stop waits for the menu's fade"
        );
    }

    /// The setup decides the sound, not whether a host listens: a game
    /// with a sink and one without run the Zimmer's opening into the Küche
    /// alike — every event, every effect slot, every request to the
    /// drivers.
    #[test]
    fn the_game_runs_the_same_with_sound_and_without() {
        struct Heard(std::sync::Arc<std::sync::atomic::AtomicUsize>);
        impl Heard {
            fn one(&self) {
                self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        impl crate::MusicSink for Heard {
            fn play(&mut self, _: &[u8], _: i16) {
                self.one();
            }
            fn play_fade(&mut self, _: &[u8], _: i16, _: u16) {
                self.one();
            }
            fn fade_out(&mut self, _: u16) {
                self.one();
            }
            fn stop(&mut self) {
                self.one();
            }
            fn reinstall(&mut self) {
                self.one();
            }
            fn install_digital(&mut self, _: u16) {
                self.one();
            }
            fn effect_volume(&mut self, _: u16) {
                self.one();
            }
            fn play_effect(&mut self, _: u16, _: &[u8]) {
                self.one();
            }
            fn stop_effects(&mut self) {
                self.one();
            }
            fn set_digital(&mut self, _: Option<(u16, &[u8])>) {
                self.one();
            }
        }
        let heard = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut quiet = ds1!();
        let mut loud = ds1!();
        crate::titles::Runtime::set_music(&mut loud, Box::new(Heard(heard.clone())));
        for _ in 0..4000 {
            crate::titles::Runtime::tick(&mut quiet);
            crate::titles::Runtime::tick(&mut loud);
        }
        assert_eq!(quiet.inner.location(), 3);
        assert_eq!(
            format!("{:?}", quiet.inner.events),
            format!("{:?}", loud.inner.events)
        );
        assert_eq!(quiet.inner.music.slots, loud.inner.music.slots);
        assert_eq!(quiet.inner.music.sent, loud.inner.music.sent);
        assert!(heard.load(std::sync::atomic::Ordering::Relaxed) > 20);
    }

    /// The Zimmer's song is module 1 (`13E8h[2]`), played endlessly once the
    /// entry is over (`0x0ECB7`); the opening's walk into the Küche fades it
    /// over 1500 ms (`0x0ED39`), stops it once the transition has played
    /// (`0x0ED55`) and once more as the slot is unloaded (`0x85F3`), which
    /// finds nothing playing, and the Küche plays module 10 (`13E8h[3]`).
    #[test]
    fn the_zimmer_plays_module_1_and_the_kueche_module_10() {
        use parsec1994::Request;
        let mut g = ds1!();
        let mut all: Vec<_> = g.inner.music.sent.drain(..).map(|r| (0, r)).collect();
        all.extend(music_over(&mut g, 4000));
        // The digital driver's side, effects aside: installed by the init,
        // each song's digital part before its play (`0x6080`), entry 21 with
        // nothing after each stop (`0x6207`), and the slot unload's
        // `StopAll` before its stop (`0x85DA`).
        let digital: Vec<_> = all
            .iter()
            .map(|&(_, r)| r)
            .filter(|r| !matches!(r, Request::Effect { .. }))
            .collect();
        assert_eq!(
            digital,
            [
                Request::Install { effects: 6 },
                Request::EffectVolume(0x50),
                Request::EffectVolume(0x3C),
                Request::Digital(Some(1)),
                Request::Play {
                    piece: 1,
                    loops: -1
                },
                Request::FadeOut(1500),
                Request::Stop,
                Request::Digital(None),
                Request::StopEffects,
                Request::Stop,
                Request::Digital(None),
                Request::Digital(Some(10)),
                Request::Play {
                    piece: 10,
                    loops: -1
                },
            ]
        );
        let sent: Vec<_> = all.into_iter().filter(|(_, r)| r.is_fm()).collect();
        let what: Vec<_> = sent.iter().map(|&(_, r)| r).collect();
        assert_eq!(
            what,
            [
                Request::Play {
                    piece: 1,
                    loops: -1
                },
                Request::FadeOut(1500),
                Request::Stop,
                Request::Stop,
                Request::Play {
                    piece: 10,
                    loops: -1
                },
            ]
        );
        // On the engine's 120 Hz ticks: the fade 2972 of them into the
        // Zimmer's song, the stop 117 after it, once the transition has
        // played.
        let ticks: Vec<u32> = sent.iter().map(|s| s.0).collect();
        assert_eq!(ticks, [109, 3081, 3198, 3198, 3306]);
        assert_eq!(g.inner.location(), 3);
    }
}
