//! Setting the game up and stepping it, without a screen attached.
//!
//! [`Game`] is the whole machine assembled and ready to step: the containers
//! opened, the kernel table lifted out of `ENGINE.EXE`, the modules loaded, the
//! VM wired to the engine as its host. What it does not own is the clock or
//! the window — a caller decides when a frame happens and what becomes of the
//! picture. That split is what lets the same setup drive a window, a test and a
//! headless run without any of the three knowing about the others.
//!
//! Deliberately no rendering backend and no input device: the engine produces
//! an indexed [`Framebuffer`] and reads its input out of ordinary module
//! variables. That is what let the whole renderer be verified against the
//! original engine without a window ever existing, and it stays true here.

use std::path::Path;

use motionvm_formats::{Kind, ScrModule, rsc::Bank};
use motionvm_forth::{Address, Context, Run, Vm};
use motionvm_render::Framebuffer;

use crate::Engine;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

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

/// The loaded game: the virtual machine and the runtime behind it.
///
/// Both halves are public because this is also the inspection point:
/// `motionvm-tools` reads the machine's memory and the engine's descriptors,
/// and the test suite drives them. A **frontend** needs neither — see
/// [`Game::palette`], [`Game::frame_duration`] and [`Game::set_music`], which
/// are the three things a window turns out to want.
pub struct Game {
    /// The interpreter, with the game's modules loaded.
    pub vm: Vm,
    /// The runtime the interpreter's words act on.
    pub engine: Engine,
    /// Whether a word is part-way through and waiting to be resumed.
    running: bool,
    /// Whether the execution now on the machine is the one put back after
    /// `QUITANIM`, i.e. `START` running on into `ENDGAME`.
    ending: bool,
    /// Whether that has finished. See [`Game::finished`].
    over: bool,
    /// `START`, set aside inside `ANIMPLAY` while the controller has the frame.
    ///
    /// Put back by [`Game::step`] once `QUITANIM` has cleared
    /// [`Engine::main_loop`]. **Written and never read, this field makes
    /// `ANIMPLAY` a one-way door** and `ENDGAME` unreachable, without anything
    /// failing to say so.
    ///
    /// The original does this with a re-entrant interpreter call: the native
    /// loop keeps its own C stack frame and runs the bytecode machine again for
    /// the controller. Here the outer execution is parked instead, and put back
    /// when the loop ends — which is how `START` gets to reach `ENDGAME`.
    parked: Option<Context>,
}

impl Game {
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
                "{} is not a MOTION game directory\n  missing: {}\n  \
                 This needs the files of an original installation; \
                 see \"Game data\" in the README.",
                dir.display(),
                names.join(", "),
            )
            .into());
        }
        let bank = Bank::open_dir(dir)?;
        let engine_exe = motionvm_formats::find_ci(dir, "ENGINE.EXE")
            .ok_or_else(|| format!("{}: no ENGINE.EXE", dir.display()))?;
        let img = motionvm_formats::le::Image::open(engine_exe)?;
        let kernel = motionvm_formats::le::kernel_words(&img);
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

        let engine = Engine::new().with_resources(dir);
        Ok(Self {
            vm,
            engine,
            running: false,
            ending: false,
            over: false,
            parked: None,
        })
    }

    /// Points saving and loading at a directory.
    ///
    /// Refused if it lies inside the game data: the original has no notion of
    /// a saves directory at all — `PUT` and its siblings build bare filenames,
    /// so a save landed beside `ENGINE.EXE` among the game's own files — and
    /// that is the one thing this port must not reproduce.
    ///
    /// Without one, the slot list stays empty and saving stops by name. That is
    /// the safe default rather than an oversight: the original writes its saves
    /// beside its data files, and here the data files are read-only.
    pub fn set_saves(&mut self, dir: &Path) -> Res<()> {
        self.engine.set_saves(dir).map_err(|e| e.into())
    }

    /// Looks a word up by module and name.
    pub fn address(&self, module: u32, word: &str) -> Option<Address> {
        motionvm_forth::word_address(&self.vm, module, word)
    }

    /// Calls a word with the given arguments already on the stack, letting any
    /// transition it starts play out.
    ///
    /// A word can stop halfway — `INCLLOC` fades the status bar out and does not
    /// come back until that is over — so this pumps frames until it is really
    /// done. Callers that want the frames themselves use [`Self::pump`].
    pub fn call(&mut self, module: u32, word: &str, args: &[i32]) -> Res<()> {
        let addr = self
            .address(module, word)
            .ok_or_else(|| format!("module {module} has no word {word}"))?;
        self.vm.data.extend_from_slice(args);
        self.call_at(addr)
    }

    /// Runs the word at `addr`, letting any transition it starts play out.
    pub fn call_at(&mut self, addr: Address) -> Res<()> {
        self.vm.start(addr);
        self.running = true;
        let mut frames = 0u32;
        while self.pump()? {
            frames += 1;
            if frames > 100_000 {
                return Err("the word never finished".into());
            }
        }
        Ok(())
    }

    /// One frame of whatever is running. Returns whether it is still going.
    ///
    /// Between two resumes the transition moves on by one band; that is the
    /// time the original spends inside its own loop, with the interpreter
    /// stopped exactly where it was.
    pub fn pump(&mut self) -> Res<bool> {
        if !self.running {
            return Ok(false);
        }
        if self.engine.in_transition() {
            self.engine.advance_curtain();
            if self.engine.in_transition() {
                return Ok(true);
            }
        }
        match self.vm.resume(&mut self.engine)? {
            Run::Done => {
                self.running = false;
                // If what just finished was the parked `START`, the game is
                // over: there is nothing after `ENDGAME` in that word.
                if self.ending {
                    self.ending = false;
                    self.over = true;
                }
                Ok(false)
            }
            Run::Yielded => {
                // `ANIMPLAY` has been reached: the game is in its main loop and
                // the word that started it must not run on — in the original
                // that handler does not return until the game ends. Pausing
                // alone would not achieve that, because a pause only hands the
                // frame back and the next resume carries straight on. So the
                // execution is parked here, which also leaves it in the one
                // place it belongs: immediately after `ANIMPLAY`, where
                // `START` continues into `ENDGAME` once the loop is over.
                if self.engine.entering_loop {
                    self.engine.entering_loop = false;
                    self.parked = Some(self.vm.park());
                    self.running = false;
                    return Ok(false);
                }
                Ok(true)
            }
        }
    }

    /// Whether the game has run to its end.
    ///
    /// True once `QUITANIM` has ended the main loop *and* the `START` parked
    /// inside `ANIMPLAY` has run on through `ENDGAME` and returned. That is the
    /// game's own way of finishing, as opposed to the window being closed, and
    /// it is what a frontend should stop on.
    pub fn finished(&self) -> bool {
        self.over
    }

    /// Whether a word is part-way through — the controller has not returned.
    ///
    /// Worth reporting: a controller that never finishes looks exactly like a
    /// game that is running, except that everything after the point it hangs
    /// on never happens.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Reads a module variable.
    ///
    /// A variable's body opens with the `_PutAdr` that pushes its own address,
    /// so the value sits in the cell right after it.
    pub fn get_var(&self, module: u32, name: &str) -> Option<i32> {
        let addr = self.address(module, name)?.next();
        self.vm.fetch(addr).ok().map(|v| v as i32)
    }

    /// Writes a module variable. This is how input reaches the game: the
    /// original engine's native loop fills `_MLK`, `_MRK` and `_AKTKEY` the
    /// same way, since no bytecode anywhere writes them.
    pub fn set_var(&mut self, module: u32, name: &str, value: i32) -> Res<()> {
        let addr = self
            .address(module, name)
            .ok_or_else(|| format!("module {module} has no variable {name}"))?
            .next();
        self.vm.store(addr, value as u32)?;
        Ok(())
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

    /// One step of the game — what the original engine's native loop does once
    /// per frame.
    ///
    /// No bytecode anywhere reads `_LTHANDLER` or writes `_MLK`, so this part
    /// was never in the game files to begin with: the native loop polled the
    /// input, stored it in those variables and called the location's task
    /// manager. That manager is bytecode and does the rest itself.
    ///
    /// A frame is the unit of time here, not a millisecond. `!LTWAIT` is
    /// `_LOCTASKWAI --`, one subtraction per call, so a task that asks to wait
    /// fifty waits for fifty of these steps.
    pub fn step(&mut self) -> Res<()> {
        // Something already part-way through gets this frame instead. That is
        // the whole of the blocking behavior: the task manager is not called
        // again until the word it is stuck in has finished.
        if self.running {
            self.pump()?;
            return Ok(());
        }
        // `QUITANIM` has cleared the main-loop flag, so the game is over. The
        // execution parked when `ANIMPLAY` was entered goes back on the machine
        // and runs on from the cell after it — which is where `START` continues
        // into `ENDGAME`.
        //
        // Before `ANIMPLAY` is ever reached this flag is false as well, but
        // nothing is parked then, so the two cases do not need telling apart.
        if !self.engine.main_loop
            && let Some(saved) = self.parked.take()
        {
            self.vm.unpark(saved);
            self.running = true;
            self.ending = true;
            self.pump()?;
            return Ok(());
        }
        // Once `START` has handed a controller to `CTRL`, that word is the
        // frame. It calls the location handler, acts on `_NEXTLOC` and drives
        // the animations itself.
        //
        // Without one, a hand-built loop stands in. It serves the path
        // `startup_only` + `enter_location`, which is how a caller reaches a
        // rendered scene without playing to it — the intro tests and the
        // pixel-for-pixel comparison against the original both enter that way.
        // It applies only until `START` sets a controller; removing it would
        // take those entry points with it.
        let addr = match self.engine.controller {
            Some(addr) => addr,
            None => {
                if let Some(next) = self.get_var(2, "_NEXTLOC").filter(|&n| n != 0) {
                    self.set_var(2, "_NEXTLOC", 0)?;
                    self.begin_location(next)?;
                    self.pump()?;
                    return Ok(());
                }
                match self.get_var(2, "_LTHANDLER").filter(|&h| h != 0) {
                    Some(h) => Address::new(h as u32 >> 16, h as u32 & 0xffff),
                    None => return Ok(()),
                }
            }
        };
        self.vm.start(addr);
        self.running = true;
        self.pump()?;
        // The walk and the drawer belong *after* the controller, and only if it
        // finished. `ANIMPLAY` calls the controller and waits: a fade does not
        // return until its bands have run, so on such a frame neither the walk
        // nor the drawer happens at all. Doing them anyway redrew the screen
        // right after `FADEOUT` had hidden it — with the descriptors the same
        // bytecode had just switched.
        if self.running || self.engine.in_transition() {
            return Ok(());
        }
        self.descriptor_frame()?;
        // And again, because the walk runs bytecode. A descriptor's callback
        // goes through `call_nested`, where the interpreter does not pause, so
        // a callback that fades — `DO_INVSEL`, the whole game menu — queues its
        // curtains right here. Drawing and presenting now would put the
        // finished state on the screen before a single band had moved, and no
        // fade in the menu would ever be seen.
        if self.engine.in_transition() {
            return Ok(());
        }
        // Nothing else puts a picture on a screen, so a frame that never gets
        // here leaves the buffers as they were — black before the first one,
        // and untouched while a fade runs.
        self.engine.draw();
        // The drawer fills the screens; the presenter (0x1457D) is what makes
        // them visible. Separate for the same reason the original separates
        // them: a curtain writes to the visible screen alone.
        self.engine.present();
        Ok(())
    }

    /// The per-frame walk over the descriptors, as `ANIMPLAY` runs it.
    ///
    /// In the original this is 0x68c64, called from inside the frame loop right
    /// after the controller, recursing down the descriptor tree. Here the order
    /// comes from [`Engine::frame_order`] instead of from links — see there for
    /// why the two are the same order and not merely a plausible one.
    ///
    /// A callback is bytecode and runs re-entrantly, which is what
    /// `call_nested` is for. It must not block: there is no parking place
    /// inside a walk, and nothing reached this way does.
    fn descriptor_frame(&mut self) -> Res<()> {
        for handle in self.engine.frame_order() {
            if let Some(word) = self.engine.tick_descriptor(handle) {
                let addr = Address::new(word as u32 >> 16, word as u32 & 0xffff);
                self.engine.select_descriptor(handle);
                self.vm.call_nested(addr, &mut self.engine)?;
            }
        }
        Ok(())
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

    /// The frame to show, pointer and all.
    pub fn render(&mut self) -> Framebuffer {
        self.engine.render()
    }

    /// The palette the frame's indices mean.
    ///
    /// A frontend needs exactly three things beyond stepping and rendering —
    /// this, [`Game::frame_duration`] and [`Game::set_music`] — so it gets
    /// three methods rather than the engine. `motionvm-app` names `engine`
    /// nowhere, and that is checked rather than hoped: the engine holds the
    /// game's state, and a window has no business reaching into it.
    pub fn palette(&self) -> &motionvm_formats::Palette {
        self.engine.palette()
    }

    /// How long to wait before the next frame, as `DELAY` asked.
    ///
    /// `None` until the game has asked — `START` runs `25 DELAY` before it
    /// enters its loop, so a frontend needs a value of its own for the first
    /// few frames.
    pub fn frame_duration(&self) -> Option<std::time::Duration> {
        self.engine.frame_duration()
    }

    /// Where the music goes. Without one, the game plays silently.
    pub fn set_music(&mut self, sink: Box<dyn crate::MusicSink>) {
        self.engine.set_music(sink);
    }
}
