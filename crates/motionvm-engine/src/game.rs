//! Setting the game up and stepping it, without a screen attached.
//!
//! [`Game<Vm>`] is the whole machine assembled and ready to step: the containers
//! opened, the kernel table lifted out of the engine binary, the modules
//! loaded, the VM wired to the engine as its host. It is generic over the
//! machine, and named with it — `Game<m32::Vm>` or `Game<m16::Vm>`, neither
//! of them the default — and what one game does that another does not lives
//! in `titles/`, behind [`Hooks`]. What it does not own is the clock or
//! the window — a caller decides when a frame happens and what becomes of the
//! picture. That split is what lets the same setup drive a window, a test and a
//! headless run without any of the three knowing about the others.
//!
//! Deliberately no rendering backend and no input device: the engine produces
//! an indexed [`Framebuffer`] and reads its input out of ordinary module
//! variables. That is what let the whole renderer be verified against the
//! original engine without a window ever existing, and it stays true here.

use std::path::Path;

use motionvm_forth::{Address, Host, Machine, Run};
use motionvm_render::Framebuffer;

use crate::Engine;

pub(crate) type Res<T> = Result<T, Box<dyn std::error::Error>>;

/// The loaded game: the virtual machine and the runtime behind it.
///
/// Both halves are public because this is also the inspection point:
/// `motionvm-tools` reads the machine's memory and the engine's descriptors,
/// and the test suite drives them. A **frontend** needs neither — see
/// [`Game::palette`], [`Game::frame_duration`] and [`Game::set_music`], which
/// are the three things a window turns out to want.
pub struct Game<M: Machine> {
    /// The interpreter, with the game's modules loaded.
    pub vm: M,
    /// The runtime the interpreter's words act on.
    pub engine: Engine,
    /// Which game this is.
    ///
    /// The one thing a loaded game cannot work out from its own parts: the two
    /// 16-bit titles run the same machine on the same container format, and
    /// Rust allows one `Playable` for one concrete `Game<M>`. The opener knows,
    /// and writes it down here.
    pub(crate) title: crate::titles::Title,
    /// Whether a word is part-way through and waiting to be resumed.
    pub(crate) running: bool,
    /// Whether the execution now on the machine is the one put back after
    /// `QUITANIM`, i.e. `START` running on into `ENDGAME`.
    pub(crate) ending: bool,
    /// Whether that has finished. See [`Game::finished`].
    pub(crate) over: bool,
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
    pub(crate) parked: Option<M::Context>,
}

/// What one game does that the driver cannot know: the frame to run while no
/// controller is installed. Implemented once per game in `titles/`, and
/// public only because the generic driver is bounded on it — nothing outside
/// this crate implements it.
pub trait Hooks {
    /// The word to run this frame when `CTRL`/`SCRCTRL` has not installed a
    /// controller yet — or `None` when the frame is spent or there is nothing
    /// to run.
    fn fallback_controller(&mut self) -> Res<Option<Address>>;
}

impl<M: Machine> Game<M>
where
    Engine: Host<M>,
    Self: Hooks,
{
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
        self.vm.word_address(module, word)
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
        self.vm.data().extend_from_slice(args);
        self.call_at(addr)
    }

    /// Runs the word at `addr`, letting any transition it starts play out.
    pub fn call_at(&mut self, addr: Address) -> Res<()> {
        self.vm.start(addr)?;
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
                //
                // A game may have more than one such loop: ENVIRO's `RUN`
                // enters `ANIMPLAY` for the intro, comes back when the intro
                // quits, and enters it again for the game. So parking also
                // forgets that the resumed word was on its way out — it has
                // found another loop to stand in.
                if self.engine.entering_loop {
                    self.engine.entering_loop = false;
                    self.parked = Some(self.vm.park());
                    self.running = false;
                    self.ending = false;
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
        let addr = self.address(module, name)?;
        self.vm.variable(addr)
    }

    /// Writes a module variable. This is how input reaches the game: the
    /// original engine's native loop fills `_MLK`, `_MRK` and `_AKTKEY` the
    /// same way, since no bytecode anywhere writes them.
    pub fn set_var(&mut self, module: u32, name: &str, value: i32) -> Res<()> {
        let addr = self
            .address(module, name)
            .ok_or_else(|| format!("module {module} has no variable {name}"))?;
        self.vm.set_variable(addr, value)?;
        Ok(())
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
        // A new frame, new input: the poll budget starts over, and a word
        // that had spent it and is now finished is no longer waiting.
        self.engine.polls = 0;
        if !self.running {
            self.engine.polling = false;
        }
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
            None => match self.fallback_controller()? {
                Some(addr) => addr,
                None => return Ok(()),
            },
        };
        self.vm.start(addr)?;
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
        for (screen, handle) in self.engine.frame_order() {
            if let Some(word) = self.engine.tick_descriptor_on(screen, handle) {
                let Some(addr) = self.vm.callback_target(word) else {
                    continue;
                };
                // The 16-bit loop makes the descriptor's screen active along
                // with it (`016a:05d6`: the screen into `DS:0x5de2`, the
                // number into `DS:0x3058`) — on that machine a number names a
                // descriptor only together with its screen.
                if self.engine.per_screen_descriptors {
                    self.engine.select_screen(screen);
                }
                self.engine.select_descriptor(handle);
                self.vm.call_nested(addr, &mut self.engine)?;
            }
        }
        Ok(())
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
