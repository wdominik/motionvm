//! Setting the game up and stepping it, without a screen attached.
//!
//! [`Game<M>`] is the whole machine assembled and ready to step: the containers
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

use motionvm_motion_forth::{Address, Host, Machine, Run};
use motionvm_render::Framebuffer;

use crate::{Engine, Error, Result};

/// The loaded game: the virtual machine and the runtime behind it.
///
/// Both halves are public because this is also the inspection point: the
/// test suites read the machine's memory and the engine's descriptors and
/// drive both directly. A **window** needs neither — it holds a
/// boxed contract and cannot even name them; see [`Game::palette`],
/// [`Game::frame_duration`] and [`Game::set_music`], which are the kinds of
/// things a window turns out to want.
pub struct Game<M: Machine> {
    /// The interpreter, with the game's modules loaded.
    pub vm: M,
    /// The runtime the interpreter's words act on.
    pub engine: Engine,
    /// Which game this is.
    ///
    /// The one thing a loaded game cannot work out from its own parts: the
    /// four 16-bit titles run the same machine on the same container format,
    /// and Rust allows one [`crate::Driven`] for one concrete `Game<M>`. The
    /// opener
    /// knows, and writes it down here.
    pub(crate) title: crate::titles::Title,
    /// The buttons as the platform last reported them — the live level.
    ///
    /// The original's `MOUSELK` reads the hardware: held is held, and every
    /// debounce is the script's own — the 32-bit shell maintains
    /// `_MPRESSED` from this very level (module 4, `0x04a80`: set while a
    /// button is down, cleared once both are up), and the title's task
    /// handler reads the raw level on purpose (module 223, `0x01a38`), which
    /// is what makes holding a button skip the title a card per fade. So
    /// what a step delivers is the level, not an edge.
    pub(crate) buttons: (bool, bool),
    /// One-frame stretches for presses too short to span a step.
    ///
    /// The original polls once per frame and usually catches a real click
    /// because a click outlasts a frame; an event-fed window can see a press
    /// and release both land between two steps, and this keeps that press
    /// visible for the one frame the original's poll would have given it.
    pub(crate) stretched: (bool, bool),
    /// Where this game keeps the location it is in and the one it is going to.
    ///
    /// Written down by the opener from the game's own module, so that
    /// [`Game::request_location`] and [`Game::start_location`] are one
    /// reading over five sets of names rather than a match on the roster.
    pub(crate) location: LocationScheme,
    /// The screens' controllers still to run this frame, innermost last.
    ///
    /// A frame is every screen's controller in turn, not one — see
    /// [`Game::step`]. The list is built when a frame starts and drained as
    /// the words run, so a controller that blocks keeps its place and the
    /// ones behind it wait for the frames it takes.
    pub(crate) frame_controllers: Vec<i32>,
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

/// Where a game keeps the location it is in and the one it is going to.
///
/// Every MOTION game moves between locations by writing a module variable that
/// its own `CTRL` polls, but the module number, the names and the value that
/// means "none" are the compiler's and therefore the game's — module 601's
/// `NEXTLOC`/`ACTLOC`/`STARTLOC` in the three 1995/96 16-bit games, `NAO`/`AO`
/// in module 605 in Victor Loomes, `_STARTLOC` in module 2 in Dunkle Schatten
/// 2. The mechanism is the same in all of them, so it is written once here and
/// each game's module carries its own constant.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LocationScheme {
    /// The module the variables live in.
    pub(crate) module: u32,
    /// The variable a caller writes to ask for a location, and the first one
    /// read back when asking where a run begins.
    pub(crate) next: &'static str,
    /// What to read when `next` holds no location: the variable that says
    /// whether a location has been entered at all, and the one holding the
    /// number to answer with. `None` where the game keeps only `next`.
    pub(crate) fallback: Option<(&'static str, &'static str)>,
    /// The lowest value that is a location number; anything below it means
    /// "none yet". `None` where every value the variable can hold is one.
    pub(crate) unset_below: Option<i32>,
}

/// What a generation's stand-in frame does that the driver cannot know: the
/// frame to run while no controller is installed. Implemented once per
/// machine type in `titles/` — Rust's coherence allows no more than that,
/// which is why the impl is the generation's even where one game's variable
/// names appear in it — and public only because the generic driver is
/// bounded on it; nothing outside this crate implements it.
pub trait Hooks {
    /// The word to run this frame when `CTRL`/`SCRCTRL` has not installed a
    /// controller yet — or `None` when the frame is spent or there is nothing
    /// to run.
    fn fallback_controller(&mut self) -> Result<Option<Address>>;
}

impl<M: Machine> Game<M>
where
    Engine: Host<M>,
    Self: Hooks,
{
    /// Points saving and loading at a directory — this game's own, under the
    /// one given.
    ///
    /// The slug goes on here rather than in the caller because sharing a
    /// directory between two games loses saves. Every game names its slots
    /// alike — `701.blk`, `701.FRZ`, `701.anm` — and the magic in the header
    /// is the *generation's*, so the four 16-bit games write files another of
    /// them reads: pointed at one directory, one would open another's slot
    /// rather than refuse it. Under `saves/`, this game's slots are in
    /// `saves/enviro/` and no other game's are.
    ///
    /// Refused if the result lies inside the game data: the original has no
    /// notion of a saves directory at all — `PUT` and its siblings build bare
    /// filenames, so a save landed beside `ENGINE.EXE` among the game's own
    /// files — and that is the one thing this port must not reproduce.
    ///
    /// Without a directory the slot list stays empty and saving stops by name.
    /// That is the safe default rather than an oversight: the original writes
    /// its saves beside its data files, and here the data files are read-only.
    pub fn set_saves(&mut self, dir: &Path) -> Result<()> {
        self.engine
            .set_saves(&dir.join(self.title.slug()))
            .map_err(Error::Saves)
    }

    /// Where saving and loading go — the directory [`Game::set_saves`] was
    /// given with this game's slug under it, or `None` while there is none.
    pub fn saves(&self) -> Option<&Path> {
        self.engine.saves()
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
    pub fn call(&mut self, module: u32, word: &str, args: &[i32]) -> Result<()> {
        let addr = self.address(module, word).ok_or_else(|| Error::NoWord {
            module,
            name: word.to_string(),
        })?;
        self.vm.data().extend_from_slice(args);
        self.call_at(addr)
    }

    /// Runs the word at `addr`, letting any transition it starts play out.
    pub fn call_at(&mut self, addr: Address) -> Result<()> {
        self.vm.start(addr)?;
        self.running = true;
        self.park()
    }

    /// Runs a started execution to its parked state: [`Game::pump`] until
    /// the machine parks, under the same budget as [`Game::call_at`], so a
    /// startup that never reaches its own frame loop answers
    /// [`Error::Unfinished`] instead of spinning forever — behind no window
    /// at all, when the contract's `start` is what drives this.
    pub fn park(&mut self) -> Result<()> {
        let mut frames = 0u32;
        while self.pump()? {
            frames += 1;
            if frames > 100_000 {
                return Err(Error::Unfinished);
            }
        }
        Ok(())
    }

    /// Moves the pointer the game reads, in its own coordinates.
    ///
    /// Written through at once — both machines read the position out of the
    /// engine's mouse record, and the pointer the engine draws follows this
    /// between frames.
    pub fn pointer_position(&mut self, x: i32, y: i32) {
        self.engine.mouse.x = x;
        self.engine.mouse.y = y;
    }

    /// Takes one button transition from the platform: the level follows it,
    /// and a press also arms the one-frame stretch — the two fields above
    /// say why.
    pub fn note_button(&mut self, which: motionvm_playable::Button, down: bool) {
        let (level, stretch) = match which {
            motionvm_playable::Button::Left => (&mut self.buttons.0, &mut self.stretched.0),
            motionvm_playable::Button::Right => (&mut self.buttons.1, &mut self.stretched.1),
            _ => return,
        };
        *level = down;
        if down {
            *stretch = true;
        }
    }

    /// The buttons for the step about to run: the live level, with a spent
    /// press stretched to this one frame.
    pub(crate) fn buttons_this_frame(&mut self) -> (bool, bool) {
        let stretched = std::mem::take(&mut self.stretched);
        (self.buttons.0 || stretched.0, self.buttons.1 || stretched.1)
    }

    /// One frame of whatever is running. Returns whether it is still going.
    ///
    /// Between two resumes the transition moves on by one band; that is the
    /// time the original spends inside its own loop, with the interpreter
    /// stopped exactly where it was.
    pub fn pump(&mut self) -> Result<bool> {
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
                // A game may have more than one such loop: `RUN` of
                // Die Enviro-Kids greifen ein
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
    pub fn set_var(&mut self, module: u32, name: &str, value: i32) -> Result<()> {
        let addr = self
            .address(module, name)
            .ok_or_else(|| Error::NoVariable {
                module,
                name: name.to_string(),
            })?;
        self.vm.set_variable(addr, value)?;
        Ok(())
    }

    /// Asks the game to go to location `n`, through the variable it polls
    /// itself.
    ///
    /// The later 16-bit builds keep a `NEXTLOC` in module 601 and their `CTRL`
    /// (module 100, word 400) runs
    /// `NEXTLOC @ -1 != IF NEXTLOC @ INCLLOC -1 NEXTLOC ! THEN` every frame,
    /// with the scripts storing their exits there — `13 NEXTLOC !` in module
    /// 609, for one. Victor Loomes has no module 601: its `CTRL` (word 432)
    /// ends on `NAO @ IF NAO @ INCLORT NAO 0! THEN`. Dunkle Schatten 2 writes
    /// `_STARTLOC` in module 2. The same mechanism under each game's own
    /// names, which is why the names are the game's and this is not.
    ///
    /// There is no way around `RUN`'s own first location: it enters one before
    /// `CTRL` gets a frame, so the request is honored one frame later, from
    /// inside that location.
    pub fn request_location(&mut self, n: i32) -> Result<()> {
        let s = self.location;
        self.set_var(s.module, s.next, n)
    }

    /// The pending location if one is set, else the location `RUN` entered
    /// itself — and `None` while it has entered none.
    ///
    /// Read rather than assumed, because the location is the game's and not
    /// the generation's: `RUN` writes `1 STARTLOC !` in Die Enviro-Kids
    /// greifen ein and `20 STARTLOC !` in Hilfe für Amajambere, while Jeff
    /// Jet's leaves the 13 its module 601 declares. It writes it on the way
    /// out of the intro, though, and until then `STARTLOC` holds only what the
    /// module declares — 13 in Die Enviro-Kids greifen ein, which starts at 1.
    /// `ACTLOC` is -1 until a location is entered, so it is what says whether
    /// `STARTLOC` means anything yet.
    ///
    /// Victor Loomes has no `STARTLOC` to write: its `RUN` zeroes `AO` and
    /// then enters its first location itself with `1 INCLORT`, so `AO` is both
    /// the gate and the answer, and 0 means the intro has not left it yet.
    pub fn start_location(&self) -> Option<i32> {
        let s = self.location;
        let set = |n: i32| s.unset_below.is_none_or(|first| n >= first);
        match self.get_var(s.module, s.next) {
            Some(n) if set(n) => Some(n),
            _ => s.fallback.and_then(|(entered, answer)| {
                self.get_var(s.module, entered)
                    .filter(|&n| set(n))
                    .and_then(|_| self.get_var(s.module, answer))
            }),
        }
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
    pub fn step(&mut self) -> Result<()> {
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
            // A request holds the machine but not the frame. The original
            // waits inside its own handler with the frame loop still running
            // under it, so the box has to be read and shown while the word
            // stands still — otherwise nothing could ever answer it.
            if self.engine.request.is_some() {
                self.engine.poll_request();
            }
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
        //
        // A frame runs **every** screen's controller, not one. `ANIMPLAY`
        // walks its three screen slots and for each one whose word id is not
        // `0xFFFF` makes that screen current and runs it (`0104:55ec` to
        // `0x561f` in `LL.EXE`, with the id read from the slot the screen's
        // `+0x14` was copied into). The activity test sits earlier in the same
        // loop and skips only the descriptor work, so an *inactive* screen
        // still gets its controller — which is how Victor Loomes' panel comes
        // back: its screen is hidden, and the controller on it is what watches
        // the pointer and shows it again.
        //
        // The later games give one screen a controller, so their frame is one
        // word either way.
        if self.frame_controllers.is_empty() {
            self.frame_controllers = self
                .engine
                .screen_controllers()
                .into_iter()
                .filter(|&id| id >= 0)
                .collect();
            self.frame_controllers.reverse();
        }
        let addr = loop {
            let Some(id) = self.frame_controllers.pop() else {
                // The engine-wide controller is the 32-bit game's, which sets
                // one without naming a screen. The stand-in is asked for only
                // when there is none, because asking is not free.
                if let Some(addr) = self.engine.controller {
                    break addr;
                }
                match self.fallback_controller()? {
                    Some(addr) => break addr,
                    None => return Ok(()),
                }
            };
            if let Some(addr) = self.vm.callback_target(id) {
                // The screen whose word this is becomes the current one
                // before it runs, so a controller that asks about "the
                // screen" gets its own (`0104:5600`).
                self.engine.select_screen_of_controller(id);
                break addr;
            }
        };
        self.vm.start(addr)?;
        self.running = true;
        self.pump()?;
        // The rest of the frame's controllers, in the same step. `ANIMPLAY`
        // runs them one after another inside one frame, and the input a frame
        // was given has to reach all of them: a click is worth exactly one
        // step — the window clears the flag as soon as it has handed it over —
        // so a controller whose turn came a step later would never see one.
        // That is not a detail of the frontend: it is why the game's own menu
        // drew and answered nothing until this loop was here.
        while !self.running
            && !self.engine.in_transition()
            && let Some(id) = self.frame_controllers.pop()
        {
            let Some(next) = self.vm.callback_target(id) else {
                continue;
            };
            self.engine.select_screen_of_controller(id);
            self.vm.start(next)?;
            self.running = true;
            self.pump()?;
        }
        // The walk and the drawer belong *after* the controller, and only if it
        // finished. `ANIMPLAY` calls the controller and waits: a fade does not
        // return until its bands have run, so on such a frame neither the walk
        // nor the drawer happens at all. Doing them anyway redrew the screen
        // right after `FADEOUT` had hidden it — with the descriptors the same
        // bytecode had just switched.
        if self.running || self.engine.in_transition() {
            return Ok(());
        }
        // The walk and the drawer close the frame, so they wait until every
        // screen's controller has had it.
        if !self.frame_controllers.is_empty() {
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
        // Last in the 16-bit frame, after the blit (`LL.EXE` `0104:5756`):
        // the rotating palette's turn, if `SETCYCLE` armed one.
        self.engine.tick_palette_cycle();
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
    fn descriptor_frame(&mut self) -> Result<()> {
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
    pub fn palette(&self) -> &motionvm_render::Palette {
        self.engine.palette()
    }

    /// How long to wait before the next frame, as `DELAY` asked.
    ///
    /// `None` only for a game that asked not to wait at all: every game asks
    /// for its real pace during its own startup — `25 DELAY` in the 32-bit
    /// bootstrap, `15 DELAY` in the 16-bit one — and startup runs before a
    /// window ever asks.
    pub fn frame_duration(&self) -> Option<std::time::Duration> {
        self.engine.frame_duration()
    }

    /// Where the music goes. Without one, the game plays silently.
    pub fn set_music(&mut self, sink: Box<dyn crate::MusicSink>) {
        self.engine.set_music(sink);
    }
}
