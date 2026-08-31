//! What every MOTION 32-bit game needs to be opened: the resource bank, the
//! kernel table out of `ENGINE.EXE`, and every script module in the
//! containers.
//!
//! None of it is one game's. `NNN.RSC` containers beside an `ENGINE.EXE` is
//! the generation's shape, and MOTION made more games in it than the one this
//! engine plays today — Checker 2000 ships exactly that, with its own
//! `ENGINE.EXE` V0.04.15/R78. What a game's own module says is which files it
//! must have, which words tell its container apart from another's, and what
//! its bootstrap is called; [`super::ds2`] is that for Dunkle Schatten 2.
//!
//! The 16-bit counterpart is [`super::motion16`], and this file is shaped like
//! it on purpose: two generations that read the same way are the property
//! CONTRIBUTING's "Two generations" section asks for. That extends to the one
//! [`Driven`] a `Game<m32::Vm>` may have, which lives here rather than in a
//! game's module and answers [`Driven::name`] out of the field the opener
//! set — so a second game costs a manifest and no more, as it does on the
//! 16-bit side.

use motionvm_playable::KeyPress;
use std::path::Path;

use motionvm_motion_formats::m32::{Kind, ScrModule, rsc::Bank};
use motionvm_motion_forth::m32;
use motionvm_motion_forth::{Address, Machine};

use crate::Engine;
use crate::Error;
use crate::Result;
use crate::game::{Game, Hooks, LocationScheme};
use crate::titles::{Driven, Title};

/// Width of the display this generation's games ask for.
///
/// `640x480x256 SETRES`, which is the only mode Dunkle Schatten 2 asks for and
/// the one every measurement on this engine was taken in. It is the game's
/// request and not the renderer's assumption, which is why it is written down
/// here: the 16-bit engine has no `SETRES` at all and its `TOGFX` enters
/// 320×200.
pub const DISPLAY_W: u16 = 640;
/// Height of the same. Screens are composited onto a surface of that size.
pub const DISPLAY_H: u16 = 480;

/// The games this engine knows on the 32-bit machine, each with the words
/// that tell its container from another game's.
///
/// The counterpart of the 16-bit table, and it cannot work the same way: there
/// the engine binary beside the container names the game, here every game
/// ships an `ENGINE.EXE`, so the answer has to come out of the data. That is
/// what [`super::ds2::SIGNATURE`] is, and [`open`] is where it is asked —
/// after the modules are loaded, because that is the earliest a word can be
/// looked up.
const GAMES: &[(Title, &[&str])] = &[(Title::DunkleSchatten2, super::ds2::SIGNATURE)];

// A second entry silently turns `detect` below into `None` for everything —
// which stops the first game opening at all — so the table refuses to grow
// until the signature check moves into `detect`, as the procedure for a
// second 32-bit game in `CONTRIBUTING.md` describes.
const _: () = assert!(
    GAMES.len() == 1,
    "move the signature check into detect before adding a game"
);

/// Which 32-bit game `dir` holds, or `None` if it holds no container at all.
///
/// File names go no further than the generation here: MOTION made more games
/// on this machine than the one in the table above, and Checker 2000 ships
/// `NNN.RSC` beside an `ENGINE.EXE` exactly as Dunkle Schatten 2 does. While
/// the table has one entry the shape is therefore the whole answer, and the
/// signature [`open`] asks turns a directory holding some *other* 32-bit game
/// into an error that says so. A second entry is what makes that signature a
/// tie-breaker rather than a check, and this is the function it belongs in.
pub(super) fn detect(dir: &Path) -> Option<Title> {
    if !motionvm_motion_formats::m32::has_container(dir) {
        return None;
    }
    match GAMES {
        [(title, _)] => Some(*title),
        _ => None,
    }
}

/// Which of `required` the directory `dir` does not hold, as `(what, what
/// for)`.
///
/// An entry whose name begins with "a resource container" is answered by
/// [`motionvm_motion_formats::m32::has_container`]; every other is a file name, looked for
/// case-insensitively like the container scan, because reporting a file as
/// missing while it sits in the directory is worse than not finding it at all.
///
/// An existence check, not a validity one — a truncated `ENGINE.EXE` still
/// fails later, and says so itself.
pub(super) fn missing_data(
    dir: &Path,
    required: &[(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    let container = motionvm_motion_formats::m32::has_container(dir);
    required
        .iter()
        .filter(|(name, _)| match *name {
            n if n.starts_with("a resource container") => !container,
            n => motionvm_motion_formats::find_ci(dir, n).is_none(),
        })
        .copied()
        .collect()
}

/// Opens the bank, binds the kernel out of `ENGINE.EXE`, loads every script
/// module in the containers, and checks that the container is the game the
/// caller named.
///
/// Checks the required files first. Not for safety — the opens below would
/// fail anyway — but because of *what* they fail with: a bare
/// `Os { code: 2, kind: NotFound }` names neither the file nor the fact that a
/// game directory was expected at all, and the person reading it has just
/// copied a 1996 CD and has no way to guess which of its thirty files
/// mattered.
///
/// `signature` is how a container says which game it is. A `NNN.RSC` beside an
/// `ENGINE.EXE` says MOTION 32-bit and nothing more, and the words a bootstrap
/// names do not say it either: they come from the authoring template, so
/// another MOTION game has them too. Checker 2000 exports `STARTUP`, `START`
/// and `INCLLOC` from modules 3, 4 and 5 exactly as Dunkle Schatten 2 does.
/// What is a game's own is its module 2 script variables — see
/// [`super::ds2::SIGNATURE`] — and Checker 2000's module 2 has none of Dunkle
/// Schatten 2's.
///
/// Without the check the template's words would bind, run against another
/// game's data, and fail somewhere inside the VM — under the wrong game's
/// name, which is the part that misleads. The 16-bit opener answers the same
/// question by the engine binary beside the container; here the binary is
/// `ENGINE.EXE` in every game, so the answer has to come from the data.
///
/// The other directory this catches is a copy of the right game missing the
/// container its script is in, which reaches exactly the same state — hence a
/// message that names the two cases rather than deciding between them, which
/// the data cannot do.
pub(super) fn open(
    dir: &Path,
    title: Title,
    required: &[(&'static str, &'static str)],
    signature: &[&str],
    location: LocationScheme,
) -> Result<Game<m32::Vm>> {
    // A path that is not there at all gets its own answer. Listing three
    // missing files for a directory that does not exist describes the symptom
    // and hides the cause, which is usually a typo.
    if !dir.is_dir() {
        return Err(Error::NoSuchDirectory {
            dir: dir.to_path_buf(),
        });
    }
    let missing = missing_data(dir, required);
    if !missing.is_empty() {
        return Err(Error::Incomplete {
            dir: dir.to_path_buf(),
            title: title.short(),
            missing: missing.iter().map(|(n, _)| *n).collect(),
        });
    }
    let bank = Bank::open_dir(dir).map_err(|e| Error::data(dir, e))?;
    let engine_exe = motionvm_motion_formats::find_ci(dir, "ENGINE.EXE")
        .ok_or_else(|| Error::missing_file(dir, "ENGINE.EXE"))?;
    let img = motionvm_motion_formats::m32::le::Image::open(&engine_exe)
        .map_err(|e| Error::data(&engine_exe, e))?;
    let kernel = motionvm_motion_formats::m32::le::kernel_words(&img);
    let mut vm = m32::Vm::new(&kernel);

    for (_, id) in bank.present(Kind::Script) {
        // `present` reads the index, `item` reads the data behind it, and a
        // truncated container can index an item it does not hold. Skipping
        // for the same reason a module that will not parse is skipped: one
        // bad entry should not stop the game from starting.
        let Some(item) = bank
            .item(Kind::Script, id)
            .map_err(|e| Error::data(dir, e))?
        else {
            continue;
        };
        // A module that will not parse is skipped rather than fatal: the set
        // of modules is large and one bad entry should not stop the game from
        // starting. A missing module announces itself loudly later, when
        // something calls into it.
        if let Ok(parsed) = ScrModule::parse(item) {
            vm.load(item, &parsed);
        }
    }

    if let Some(name) = signature
        .iter()
        .find(|name| vm.word_address(2, name).is_none())
    {
        return Err(Error::NotThisGame {
            dir: dir.to_path_buf(),
            title: title.short(),
            word: (*name).to_string(),
        });
    }

    let engine = Engine::with_display(DISPLAY_W, DISPLAY_H).with_bank(dir, bank);
    Ok(Game {
        vm,
        engine,
        title,
        location,
        buttons: (false, false),
        stretched: (false, false),
        running: false,
        frame_controllers: Vec::new(),
        ending: false,
        over: false,
        parked: None,
    })
}

impl Driven for Game<m32::Vm> {
    fn name(&self) -> &str {
        self.title.name()
    }

    fn display_size(&self) -> (u16, u16) {
        self.engine.display_size()
    }

    fn pixel_aspect(&self) -> motionvm_playable::PixelAspect {
        // 640×480 on a 4:3 monitor: the grid already matches, so the pixels
        // are square.
        motionvm_playable::PixelAspect::default()
    }

    /// Startup runs to the game's own parked loop before returning, under
    /// [`Game::park`]'s budget: the window does not exist yet, so a startup
    /// that never parks has to answer instead of spin.
    fn start(&mut self) -> Result<()> {
        Game::<m32::Vm>::start(self)?;
        Game::<m32::Vm>::park(self)
    }

    /// One keystroke per step, because a step is one round of `ICTRL` and
    /// `ICTRL` opens with a single `?KEY` (module 4, `0x022a0`) — which takes
    /// one keystroke out of the buffer and no more. The buttons cross
    /// as the level the original's `MOUSELK` read — held is held, and the
    /// scripts do their own debouncing, as they always did.
    fn step(&mut self) -> Result<()> {
        let key = self.engine.pop_key();
        self.deliver_key(key)?;
        let (left, right) = self.buttons_this_frame();
        Game::<m32::Vm>::pointer_buttons(self, left, right)?;
        Game::<m32::Vm>::step(self)
    }

    fn pointer(&mut self, x: i32, y: i32) {
        self.pointer_position(x, y);
    }

    fn button(&mut self, which: motionvm_playable::Button, down: bool) {
        self.note_button(which, down);
    }

    fn key(&mut self, press: &KeyPress, down: bool) {
        // Releases cross and are dropped here: `?KEY` answers keystrokes,
        // and a keystroke is a press.
        if down {
            self.engine.push_key(press);
        }
    }

    fn render(&mut self) -> motionvm_render::Framebuffer {
        Game::<m32::Vm>::render(self)
    }

    fn palette(&self) -> &motionvm_render::Palette {
        Game::<m32::Vm>::palette(self)
    }

    fn frame_duration(&self) -> Option<std::time::Duration> {
        Game::<m32::Vm>::frame_duration(self)
    }

    fn set_music(&mut self, sink: Box<dyn crate::MusicSink>) {
        Game::<m32::Vm>::set_music(self, sink)
    }

    fn set_saves(&mut self, dir: &Path) -> Result<()> {
        Game::<m32::Vm>::set_saves(self, dir)
    }

    fn saves(&self) -> Option<&Path> {
        Game::<m32::Vm>::saves(self)
    }

    fn finished(&self) -> bool {
        Game::<m32::Vm>::finished(self)
    }

    fn request_location(&mut self, n: i32) -> Result<()> {
        Game::<m32::Vm>::request_location(self, n)
    }

    fn start_location(&self) -> Option<i32> {
        Game::<m32::Vm>::start_location(self)
    }
}

/// Where a game of this machine keeps its shell variables: the module its
/// compiler put them in and the names it gave them.
///
/// The names are the game's — its manifest carries a constant of these, the
/// way it carries a [`LocationScheme`] — and the mechanism is the
/// generation's, in the functions below. With one game on this machine the
/// functions reach the one manifest directly, the way [`GAMES`] reaches its
/// `SIGNATURE`; a second game moves the constant into the opener's plumbing,
/// and the roster's compile-time guard above is what makes that impossible
/// to forget.
pub(super) struct Shell {
    /// The module the compiler placed the shell variables in.
    pub(super) module: u32,
    /// The left button, as the location handlers read it.
    pub(super) left: &'static str,
    /// The right button.
    pub(super) right: &'static str,
    /// The key `?KEY` answered, as the handlers read it.
    pub(super) key: &'static str,
    /// The location the stand-in frame loop moves to.
    pub(super) next_location: &'static str,
    /// The handler the stand-in frame loop runs.
    pub(super) handler: &'static str,
    /// The task the location is in.
    pub(super) task: &'static str,
    /// The task's phase.
    pub(super) task_phase: &'static str,
}

/// What every game of this machine does the same way, on the words the
/// authoring template gives it: `STARTUP` in module 3, `START` in module 4,
/// `INCLLOC` in module 5 — Checker 2000 exports all three from the same
/// modules, which is what says they are the template's and not one game's.
impl Game<m32::Vm> {
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
    /// Hands the game this frame's buttons — the live level the step derived
    /// from the platform's transitions, a stretched short press included.
    ///
    /// Both routes are fed, because the game uses both: `ICTRL` reads the
    /// pointer through the kernel words `MOUSEX`, `MOUSEY`, `MOUSELK` and
    /// `MOUSERK` and stores the result in these variables itself, while the
    /// location handlers read the variables.
    pub fn pointer_buttons(&mut self, left: bool, right: bool) -> Result<()> {
        let shell = &super::ds2::SHELL;
        self.set_var(shell.module, shell.left, left as i32)?;
        self.set_var(shell.module, shell.right, right as i32)?;
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
        let shell = &super::ds2::SHELL;
        self.set_var(shell.module, shell.key, key)?;
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
            self.get_var(super::ds2::SHELL.module, super::ds2::SHELL.task)
                .unwrap_or(0),
            self.get_var(super::ds2::SHELL.module, super::ds2::SHELL.task_phase)
                .unwrap_or(0),
        )
    }
}

impl Hooks for Game<m32::Vm> {
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
        if let Some(next) = self
            .get_var(super::ds2::SHELL.module, super::ds2::SHELL.next_location)
            .filter(|&n| n != 0)
        {
            self.set_var(super::ds2::SHELL.module, super::ds2::SHELL.next_location, 0)?;
            self.begin_location(next)?;
            self.pump()?;
            return Ok(None);
        }
        Ok(self
            .get_var(super::ds2::SHELL.module, super::ds2::SHELL.handler)
            .filter(|&h| h != 0)
            .map(|h| Address::new(h as u32 >> 16, h as u32 & 0xffff)))
    }
}
