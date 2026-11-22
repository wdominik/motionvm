//! A parsec1994 game program run whole: `_main` and its main loop.
//!
//! The program a title's batch file runs after its title program — in
//! *Dunkle Schatten* `XP01.EXE` — is the generation's library linked with
//! the title's own code. [`Program`] is that program as the host plays it:
//! the runtime, the location being visited, the start's load and the
//! settings, the main loop's visits and the files the options write.
//! Everything that is the title's own — which file, where its program
//! keeps what, which logic each row has, how its program is read — comes
//! from the title's [`Build`].
//!
//! The addresses in the comments are *Dunkle Schatten*'s `XP01.EXE`, the
//! build the generation has been read from.

use std::path::{Path, PathBuf};
use std::time::Duration;

use motionvm_parsec_formats::detect::resource_dir;
use motionvm_render::{Framebuffer, Palette};

use super::logic::{LogicTable, Visit};
use super::{
    Adventure, AnimHook, Api, Cfg, DsMap, Files, Hook, Kinds, MenuKey, MusicMap, Opened,
    OptionsInput, OptionsMap, PointerMap, Request, SaveMap, cfg_read, sav_valid,
};
use crate::error::{Error, Result};
use crate::exe::Image;
use crate::num;
use crate::titles::Start;
use crate::titles::palette1994::PanelCurves;
use crate::titles::xp1994::Locations;

/// The files a build reads beside its program.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Names {
    /// The FGROUP whose slots are the locations' playfields.
    pub rooms: &'static str,
    /// The RIB of the verb strip and its palette bank.
    pub panel: &'static str,
    /// The RIB of the pointers, their bank and the font.
    pub pointers: &'static str,
    /// The RIB of the player's cels and their bank.
    pub player: &'static str,
    /// The RIB of the two global dialogue banks and the figure's
    /// animations.
    pub dialog: &'static str,
    /// The music library.
    pub music: &'static str,
    /// The RIB the sound drivers are kept in.
    pub drivers: &'static str,
}

/// How the program's init sets up the digital driver.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Sound {
    /// The channels it installs the driver with.
    pub channels: u16,
    /// The effects' volume the init sets after the driver's default.
    pub effect_volume: u16,
}

/// What the title reads out of its own program before the runtime opens:
/// the facts it keeps where only its own reading finds them.
pub(crate) struct Reading {
    /// Where the build keeps what the logic shares with the engine.
    pub ds_map: DsMap,
    /// Where in the program the status-line templates are, and how many.
    pub templates: (usize, usize),
    /// The substitution row as the image ships it.
    pub substitutions: Vec<Vec<u8>>,
    /// The carried list a fresh game starts with, and its first visible
    /// slot.
    pub initial_items: Vec<u8>,
    /// See [`Self::initial_items`].
    pub initial_scroll: usize,
    /// The panel's colour curves and the gray ramp.
    pub panel_curves: PanelCurves,
    /// See [`Self::panel_curves`].
    pub gray_ramp: (u8, Vec<u8>),
    /// The location and the entry a fresh game starts at.
    pub start: (u16, u16),
}

/// What one parsec1994 title hands the generation's runtime.
pub(crate) struct Build {
    /// The game program's file name and its unpacked length.
    pub program: (&'static str, usize),
    /// The files beside it.
    pub names: Names,
    /// What the location table's kinds mean.
    pub kinds: Kinds,
    /// Where the saved game and the settings are kept, and their shape.
    pub save: SaveMap,
    /// Where the game options read and write.
    pub options: OptionsMap,
    /// Where the pointer's words and tables are.
    pub pointer: PointerMap,
    /// Where the music's and the effects' words are.
    pub music: MusicMap,
    /// The digital driver's setup.
    pub sound: Sound,
    /// Which logic each row has.
    pub logic: LogicTable,
    /// The rule handlers by far address, the location's and the global
    /// table's.
    pub rule: fn(&mut Adventure, (u16, u16), u16) -> Option<i16>,
    /// The hook the library calls with its codes.
    pub hook: Hook,
    /// The location's own hook.
    pub location_hook: Hook,
    /// The location's animation hook.
    pub anim_hook: AnimHook,
}

/// One install's game program.
pub(crate) struct Program {
    /// The title's description.
    build: &'static Build,
    /// The generation's parts: locations, rooms, strip, palette banks.
    pub inner: Adventure,
    /// The location being visited, whose handler's frame loop the ticks
    /// run. `None` before the main loop and after it, and after a host's
    /// jump without an entry.
    pub visit: Option<Visit>,
    /// The install, which savegames must not be written into.
    pub game_dir: PathBuf,
    /// Where the saved game and the settings are read and written; the
    /// original uses the DOS working directory (`0590h:0DD5h`, `*` names).
    saves: Option<PathBuf>,
    /// Whether play has begun.
    pub started: bool,
    /// Whether the main loop has ended: the player quit.
    pub over: bool,
    /// Where a fresh game starts.
    start: (u16, u16),
}

impl Program {
    /// Opens the program of the install at `dir`; `read` is the title's
    /// reading of it, given the location table the dispatch was read
    /// from. Nothing is played yet.
    pub(crate) fn open(
        dir: &Path,
        build: &'static Build,
        read: impl FnOnce(&Locations) -> Result<Reading>,
    ) -> Result<Self> {
        let res = resource_dir(dir);
        let image = Image::open(dir, build.program.0, build.program.1)?;
        let locations = Locations::read(&image)?;
        let reading = read(&locations)?;
        let mut inner = Adventure::open(
            &res,
            image,
            locations,
            build.kinds,
            reading.ds_map,
            Files {
                rooms: build.names.rooms,
                panel: build.names.panel,
                pointers: build.names.pointers,
                player: build.names.player,
                dialog: build.names.dialog,
                templates: reading.templates,
                music: (build.names.music, build.music),
            },
        )?;
        inner.options_map = Some(build.options);
        inner.install_pointer(build.pointer);
        inner.rule_hook = build.rule;
        inner.hook = build.hook;
        inner.location_hook = build.location_hook;
        inner.anim_hook = build.anim_hook;
        inner.substitutions = reading.substitutions;
        inner.initialize_inventory(&reading.initial_items, reading.initial_scroll)?;
        inner.enable_panel_colors(reading.panel_curves, reading.gray_ramp)?;
        if (build.logic.logic)(reading.start.0).is_none() {
            return Err(format!(
                "the program opens at location {} and that location cannot be entered",
                reading.start.0
            )
            .into());
        }
        Ok(Program {
            build,
            inner,
            visit: None,
            game_dir: dir.to_path_buf(),
            saves: None,
            started: false,
            over: false,
            start: reading.start,
        })
    }

    /// A file of the save directory, by the name the program builds.
    pub(crate) fn save_path(&self, name_at: u16) -> Option<PathBuf> {
        let dir = self.saves.as_ref()?;
        let name = self.inner.save_file_name(name_at);
        Some(motionvm_parsec_formats::find_ci(dir, &name).unwrap_or_else(|| dir.join(name)))
    }

    /// Whether the saved game is there to be read.
    pub(crate) fn has_save(&self) -> bool {
        self.save_path(self.build.save.sav_name)
            .is_some_and(|p| p.is_file())
    }

    /// The saved game deleted, as a new game from the title program
    /// deletes it.
    pub(crate) fn delete_save(&self) {
        if let Some(p) = self.save_path(self.build.save.sav_name)
            && p.is_file()
        {
            let _ = std::fs::remove_file(p);
        }
    }

    /// Resource `n` of the drivers' RIB, a driver file.
    pub(crate) fn sound_driver(&self, n: usize) -> Result<Vec<u8>> {
        let res = resource_dir(&self.game_dir);
        let name = self.build.names.drivers;
        let path = motionvm_parsec_formats::find_ci(&res, name)
            .ok_or_else(|| format!("RESOURCE/{name}: not found"))?;
        Ok(motionvm_parsec_formats::p2l::driver(
            &std::fs::read(path)?,
            n,
        )?)
    }

    /// `_main` from `0x4F19` to the options (`0x4F87`), after the sound of
    /// its init: the main loop's words, the start's load (`147Ah:0072h`), a
    /// fresh game when nothing was loaded (`0x4F2A`), then the options,
    /// unconditionally.
    pub(crate) fn boot(&mut self, how: Start) {
        self.started = true;
        let map = self.build.save;
        let sound = self.build.sound;
        let a = &mut self.inner;
        // The init before it: the digital driver with its channels
        // (`0x5BD3`–`0x5C15`), the effects' volume set again (`0x4E85`),
        // and the effects every room keeps (`0x053AB`).
        a.music.install(sound.channels);
        a.music.send(Request::EffectVolume(sound.effect_volume));
        a.effects_preload();
        a.here_word = 0xFFFF; // 0x4F19
        a.entry_word = 0; // 0x4F1F
        a.save_buffer_clear(&map); // 0x14821
        // 0x14847: the settings, when the file is exactly its length.
        if let Some(bytes) = self
            .save_path(map.cfg_name)
            .and_then(|p| std::fs::read(p).ok())
            && let Cfg::Taken { mouse, text, speed } = cfg_read(&bytes, &map, self.inner.rng_mut())
        {
            self.inner.cfg_take(&map, mouse, text, speed);
        }
        // 0x148ED: the saved game, when the file is exactly its length.
        // A new game is one whose file the title program deleted.
        let sav = match how {
            Start::New => None,
            Start::Continue => self
                .save_path(map.sav_name)
                .and_then(|p| std::fs::read(p).ok()),
        };
        if let Some(bytes) = sav.filter(|b| b.len() == map.len) {
            self.inner.save_buffer_fill(&map, &bytes);
            if sav_valid(&bytes, &map, self.inner.rng_mut()) {
                // 0x149D3: held, and restored at once (`147Ah:052Ch`), by
                // the restore row's own handler.
                self.inner.set_word(map.loaded, 1);
                if let Some(restore) = (self.build.logic.logic)(map.restore_row) {
                    (restore.enter)(&mut self.inner);
                }
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
            a.location_word = self.start.0;
            a.entry_word = self.start.1;
        }
        a.options_open(Opened::Start);
    }

    /// The file the options' save left, written (`1E69h:003Ch`, `00C0h`).
    /// Where the original ends the program, the message stays up instead.
    fn write_save(&mut self, name_at: u16, bytes: &[u8]) {
        use std::io::Write;
        let map = self.build.save;
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

    /// The saved game the options wrote, if they wrote one this turn.
    fn write_pending_save(&mut self) {
        if let Some(bytes) = self.inner.save_written.take() {
            self.write_save(self.build.save.sav_name, &bytes);
        }
    }

    /// After the main loop: the settings written when they changed
    /// (`0x4FF0`, `147Ah:0373h`).
    fn finish(&mut self) {
        self.over = true;
        let map = self.build.save;
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
            self.visit = Some(Visit::enter(&mut self.inner, self.build.logic));
        }
    }

    /// A button. The options take any button the same (`cs:0C86h`: events
    /// 5, 6 and 7 go to one arm); the game takes the left.
    pub(crate) fn click(&mut self, left: bool) {
        self.inner.notice = None;
        if self.inner.options_up() {
            self.inner.options_input(OptionsInput::Click);
        } else if left {
            self.inner.click();
        }
    }

    /// A key as `179Eh:0611h` maps it (see the title's `key`): the
    /// options take it while they are up, then the choice menu, then the
    /// game.
    pub(crate) fn key(&mut self, mapped: u16) {
        if self.inner.options_up() {
            self.inner.options_input(OptionsInput::Key(mapped));
            return;
        }
        // The choice menu's own table (`cs:11ACh`, `0x0E0B6`): `20h` and
        // `101h` take, `105h` and `106h` move.
        if self.inner.menu.up {
            let key = match mapped {
                0x20 | 0x101 => MenuKey::Select,
                0x105 => MenuKey::Down,
                0x106 => MenuKey::Up,
                _ => return,
            };
            self.inner.menu_key(key);
            return;
        }
        self.inner.key_event(mapped);
    }

    /// One tick of the program: `false` once the main loop is over.
    pub(crate) fn tick(&mut self) -> bool {
        if self.over {
            return false;
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
            self.write_pending_save();
            match closed {
                // `_main` goes on into its main loop (`0x4F8C`).
                Some(Opened::Start) => self.main_loop(),
                // The gate goes on after its call (`0x0F033`).
                Some(Opened::Gate) => self.inner.options_gate_close(),
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
        self.write_pending_save();
        !self.over
    }

    /// A host's jump to location `n`. A passage row is not a place: the
    /// main loop hands it to the walk on to the next row that is one,
    /// which names the entry, `7Fh`, itself. The jump goes the same way.
    pub(crate) fn set_location(&mut self, n: i32) {
        let passage = Some(self.inner.kinds.passage);
        if u16::try_from(n)
            .is_ok_and(|r| self.inner.locations.kind(&self.inner.image, usize::from(r)) == passage)
        {
            self.set_location_via(n, 0x7F);
            return;
        }
        self.started = true;
        self.inner.options_drop();
        self.inner.complete_entry();
        if let Some(v) = self.visit.take() {
            v.leave(&mut self.inner);
        }
        self.inner.set_location(n);
    }

    /// Whether location `n` has logic to enter: a number past the table
    /// names no handler.
    pub(crate) fn enterable(&self, n: i32) -> bool {
        u16::try_from(n)
            .ok()
            .and_then(self.build.logic.logic)
            .is_some()
    }

    /// A host's jump to location `n` through the entry `entry`; one past
    /// the table is ignored.
    pub(crate) fn set_location_via(&mut self, n: i32, entry: i16) {
        if !self.enterable(n) {
            return;
        }
        self.started = true;
        self.inner.options_drop();
        self.inner.complete_entry();
        if let Some(v) = self.visit.take() {
            v.leave(&mut self.inner);
        }
        self.inner.location_word = num::low16(n);
        self.inner.entry_word = num::unsigned(entry);
        self.visit = Some(Visit::enter(&mut self.inner, self.build.logic));
    }

    /// The save directory, refused inside the install, which is treated as
    /// read-only.
    pub(crate) fn set_saves(&mut self, dir: &Path) -> Result<()> {
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

    /// The palette the program shows.
    pub(crate) fn palette(&self) -> &Palette {
        self.inner.palette()
    }

    /// The picture the program shows.
    pub(crate) fn render(&self) -> Framebuffer {
        self.inner.render()
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
    pub(crate) fn frame_duration() -> Duration {
        Duration::from_nanos(super::Clock::tick_nanos())
    }
}
