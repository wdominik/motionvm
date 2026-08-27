//! Dunkle Schatten 2: what the engine has to know about this one game to
//! open it and run it — the files it ships, the words its bootstrap names,
//! the script variables its input goes through.
//!
//! Everything here is the game's, not the engine's: module 4's `START`,
//! module 3's `STARTUP`, module 5's `INCLLOC`, module 2's `_STARTLOC`,
//! `_NEXTLOC`, `_LTHANDLER`, `_MLK`, `_MRK`, `_AKTKEY`, `_LOCTASK` and
//! `_LOCTASKPHA` are names the game's own compiler gave its words, and
//! another MOTION game has others.

use std::path::Path;

use motionvm_formats::m32::{Kind, ScrModule, rsc::Bank};
use motionvm_forth::m32::Vm;
use motionvm_forth::{Address, Machine};

use crate::Engine;
use crate::game::{Game, Hooks, Res};
use crate::titles::{Playable, Title};

/// What a directory must hold before [`Game::open`] can do anything with it.
///
/// Only three entries, and each was established by taking it away and watching
/// what broke, not by reading the loader:
///
/// - **A resource container.** Everything the game *is* lives in the `NNN.RSC`
///   files — the script modules, the artwork, the texts, the music, the fonts,
///   the palettes. Named by pattern rather than by number because how many a
///   game ships is the game's business: this one has three, and the loader
///   merges however many it finds.
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
/// [`Playable::start_location`], and `_NEXTLOC` is the one the stand-in frame
/// loop moves to. A container that does not define them is not a container
/// this code can drive, whatever else it holds.
pub const SIGNATURE: &[&str] = &["_STARTLOC", "_NEXTLOC"];

/// Which of the required files `dir` does not hold, as `(what, what for)`.
///
/// Empty means [`Game::open`] will get as far as parsing. It is a existence
/// check, not a validity one — a truncated `ENGINE.EXE` still fails later, and
/// says so itself.
pub fn missing_data(dir: &Path) -> Vec<(&'static str, &'static str)> {
    let has_container = std::fs::read_dir(dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                let p = e.path();
                p.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| x.eq_ignore_ascii_case("rsc"))
                    && p.file_stem()
                        .and_then(|s| s.to_str())
                        .is_some_and(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
            })
        })
        .unwrap_or(false);

    REQUIRED
        .iter()
        .filter(|(name, _)| match *name {
            n if n.starts_with("a resource container") => !has_container,
            // Case-insensitively, like the container scan just above: a copied
            // install often arrives lower-cased, and reporting a file as
            // missing while it sits in the directory is worse than not finding
            // it at all.
            n => motionvm_formats::find_ci(dir, n).is_none(),
        })
        .copied()
        .collect()
}

impl Game<Vm> {
    /// Loads the kernel word table and every script module in the resources.
    ///
    /// Checks the required files first. Not for safety — the opens below would fail
    /// anyway — but because of *what* they fail with: a bare
    /// `Os { code: 2, kind: NotFound }` names neither the file nor the fact
    /// that a game directory was expected at all, and the person reading it has
    /// just copied a 1996 CD and has no way to guess which of its thirty files
    /// mattered.
    pub fn open(dir: &Path) -> Res<Self> {
        // A path that is not there at all gets its own answer. Listing three
        // missing files for a directory that does not exist describes the
        // symptom and hides the cause, which is usually a typo.
        if !dir.is_dir() {
            return Err(format!("{}: no such directory", dir.display()).into());
        }
        let missing = missing_data(dir);
        if !missing.is_empty() {
            let names: Vec<&str> = missing.iter().map(|(n, _)| *n).collect();
            return Err(format!(
                "{} is not a complete {} directory\n  missing: {}\n  \
                 This needs the files of an original installation; \
                 see \"Game data\" in the README.",
                dir.display(),
                Title::DunkleSchatten2.name(),
                names.join(", "),
            )
            .into());
        }
        let bank = Bank::open_dir(dir)?;
        let engine_exe = motionvm_formats::find_ci(dir, "ENGINE.EXE")
            .ok_or_else(|| format!("{}: no ENGINE.EXE", dir.display()))?;
        let img = motionvm_formats::m32::le::Image::open(engine_exe)?;
        let kernel = motionvm_formats::m32::le::kernel_words(&img);
        let mut vm = Vm::new(&kernel);

        for (_, id) in bank.present(Kind::Script) {
            // `present` reads the index, `item` reads the data behind it, and a
            // truncated container can index an item it does not hold. Skipping
            // for the same reason a module that will not parse is skipped: one
            // bad entry should not stop the game from starting.
            let Some(item) = bank.item(Kind::Script, id)? else {
                continue;
            };
            // A module that will not parse is skipped rather than fatal: the
            // set of modules is large and one bad entry should not stop the
            // game from starting. A missing module announces itself loudly
            // later, when something calls into it.
            if let Ok(parsed) = ScrModule::parse(item) {
                vm.load(item, &parsed);
            }
        }

        // A `NNN.RSC` container beside an `ENGINE.EXE` says MOTION 32-bit. It
        // does not say *this* game, and the words the bootstrap names do not
        // say it either: they come from the authoring template, so another
        // MOTION game has them too. Checker 2000 — the same container pattern
        // beside its own `ENGINE.EXE` V0.04.15/R78 — exports `STARTUP`,
        // `START` and `INCLLOC` from modules 3, 4 and 5 exactly as this game
        // does. What is Dunkle Schatten 2's own is module 2's script
        // variables, and `SIGNATURE` holds the two this code reads: the start
        // location and the location the frame loop moves to. Checker 2000's
        // module 2 has neither.
        //
        // Without the check the template's words would bind, run against
        // another game's data, and fail somewhere inside the VM — under this
        // game's name, which is the part that misleads. The 16-bit opener
        // answers the same question by the engine binary beside the container;
        // here the binary is `ENGINE.EXE` in both games, so the answer has to
        // come from the data.
        //
        // The other directory this catches is a copy of this game missing the
        // container its script is in, which reaches exactly the same state —
        // hence a message that names the two cases rather than deciding
        // between them, which the data cannot do.
        if let Some(name) = SIGNATURE
            .iter()
            .find(|name| vm.word_address(2, name).is_none())
        {
            return Err(format!(
                "{} does not hold Dunkle Schatten 2's script\n  \
                 module 2 has no {name}, a variable this game's own compiler named\n  \
                 This is another MOTION 32-bit game, or an incomplete copy of this one; \
                 see \"Game data\" in the README for the files a copy needs.",
                dir.display(),
            )
            .into());
        }

        let engine = Engine::new().with_bank(dir, bank);
        Ok(Self {
            vm,
            engine,
            title: Title::DunkleSchatten2,
            running: false,
            ending: false,
            over: false,
            parked: None,
        })
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
    pub fn start(&mut self) -> Res<()> {
        // The first of those two lines is the one no bytecode contains, so the
        // slot it takes has to be granted from here. Module 4 lands in slot 1,
        // and everything `START` loads follows behind it.
        self.engine.mark_resident(4);
        let addr = self
            .address(4, "START")
            .ok_or("module 4 has no word START")?;
        self.vm.start(addr);
        self.running = true;
        Ok(())
    }
    /// Runs `STARTUP` alone, for tests that want the state without the game.
    pub fn startup_only(&mut self) -> Res<()> {
        self.call(3, "STARTUP", &[])
    }
    /// Enters a location and lets the entry play out at once.
    ///
    /// Convenient where only the settled picture matters. Anything with a frame
    /// clock wants [`Self::begin_location`] instead, or the entry's own fade is
    /// consumed before a single frame reaches the screen.
    pub fn enter_location(&mut self, location: i32) -> Res<()> {
        self.call(5, "INCLLOC", &[location])
    }
    /// Starts entering a location without running it to the end.
    ///
    /// `INCLLOC` fades out, runs the location's macro, and fades back in — the
    /// second of those is how a location appears at all. Driving it frame by
    /// frame is what makes that visible.
    pub fn begin_location(&mut self, location: i32) -> Res<()> {
        let addr = self
            .address(5, "INCLLOC")
            .ok_or("module 5 has no word INCLLOC")?;
        self.vm.data.push(location);
        self.vm.start(addr);
        self.running = true;
        Ok(())
    }
    /// The location the game itself wants to begin at.
    pub fn start_location(&self) -> Option<i32> {
        self.get_var(2, "_STARTLOC")
    }
    /// Hands the game this frame's input.
    ///
    /// The buttons are edge-triggered on purpose. The task manager advances a
    /// phase whenever it sees a button down, so reporting a held button on
    /// every frame would race through the whole intro in a fraction of a
    /// second. One frame per press is what the game means by a click.
    ///
    /// Both routes are fed, because the game uses both: `ICTRL` reads the
    /// pointer through the kernel words `MOUSEX`, `MOUSEY`, `MOUSELK` and
    /// `MOUSERK` and stores the result in these variables itself, while the
    /// location handlers read the variables.
    pub fn set_input(&mut self, x: i32, y: i32, left: bool, right: bool, key: i32) -> Res<()> {
        self.set_var(2, "_MLK", left as i32)?;
        self.set_var(2, "_MRK", right as i32)?;
        self.set_var(2, "_AKTKEY", key)?;
        self.engine.key = key;
        self.engine.mouse.x = x;
        self.engine.mouse.y = y;
        self.engine.mouse.left = left as i32;
        self.engine.mouse.right = right as i32;
        Ok(())
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
    fn fallback_controller(&mut self) -> Res<Option<Address>> {
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

impl Playable for Game<Vm> {
    fn title(&self) -> Title {
        Title::DunkleSchatten2
    }

    fn display_size(&self) -> (u16, u16) {
        self.engine.display_size()
    }

    fn start(&mut self) -> Res<()> {
        Game::<Vm>::start(self)
    }

    fn pump(&mut self) -> Res<bool> {
        Game::<Vm>::pump(self)
    }

    fn step(&mut self) -> Res<()> {
        Game::<Vm>::step(self)
    }

    fn set_input(&mut self, x: i32, y: i32, left: bool, right: bool, key: i32) -> Res<()> {
        Game::<Vm>::set_input(self, x, y, left, right, key)
    }

    fn render(&mut self) -> motionvm_render::Framebuffer {
        Game::<Vm>::render(self)
    }

    fn palette(&self) -> &motionvm_formats::Palette {
        Game::<Vm>::palette(self)
    }

    fn frame_duration(&self) -> Option<std::time::Duration> {
        Game::<Vm>::frame_duration(self)
    }

    fn set_music(&mut self, sink: Box<dyn crate::MusicSink>) {
        Game::<Vm>::set_music(self, sink)
    }

    fn set_saves(&mut self, dir: &Path) -> Res<()> {
        Game::<Vm>::set_saves(self, dir)
    }

    fn finished(&self) -> bool {
        Game::<Vm>::finished(self)
    }

    /// Through `_STARTLOC`, the variable `STARTUP` assigns and `ICTRL` reads
    /// when no location is active — not around it.
    fn request_location(&mut self, n: i32) -> Res<()> {
        self.set_var(2, "_STARTLOC", n)
    }

    fn start_location(&self) -> Option<i32> {
        Game::<Vm>::start_location(self)
    }
}
