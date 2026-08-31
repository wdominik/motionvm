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
//! [`Driven`] a `Game<Vm>` may have, which lives here rather than in a
//! game's module and answers [`Driven::name`] out of the field the opener
//! set — so a second game costs a manifest and no more, as it does on the
//! 16-bit side.

use motionvm_playable::KeyPress;
use std::path::Path;

use motionvm_motion_formats::m32::{Kind, ScrModule, rsc::Bank};
use motionvm_motion_forth::Machine;
use motionvm_motion_forth::m32::Vm;

use crate::Engine;
use crate::Error;
use crate::Result;
use crate::game::{Game, LocationScheme};
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
) -> Result<Game<Vm>> {
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
    let mut vm = Vm::new(&kernel);

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

impl Driven for Game<Vm> {
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
        Game::<Vm>::start(self)?;
        Game::<Vm>::park(self)
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
        Game::<Vm>::pointer_buttons(self, left, right)?;
        Game::<Vm>::step(self)
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
        Game::<Vm>::render(self)
    }

    fn palette(&self) -> &motionvm_render::Palette {
        Game::<Vm>::palette(self)
    }

    fn frame_duration(&self) -> Option<std::time::Duration> {
        Game::<Vm>::frame_duration(self)
    }

    fn set_music(&mut self, sink: Box<dyn crate::MusicSink>) {
        Game::<Vm>::set_music(self, sink)
    }

    fn set_saves(&mut self, dir: &Path) -> Result<()> {
        Game::<Vm>::set_saves(self, dir)
    }

    fn saves(&self) -> Option<&Path> {
        Game::<Vm>::saves(self)
    }

    fn finished(&self) -> bool {
        Game::<Vm>::finished(self)
    }

    fn request_location(&mut self, n: i32) -> Result<()> {
        Game::<Vm>::request_location(self, n)
    }

    fn start_location(&self) -> Option<i32> {
        Game::<Vm>::start_location(self)
    }
}
