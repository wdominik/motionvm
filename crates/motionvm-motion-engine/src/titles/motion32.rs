//! What every MOTION 32-bit game needs to be opened: the resource bank, the
//! kernel table out of `ENGINE.EXE`, and every script module in the
//! containers.
//!
//! None of it is one game's. `NNN.RSC` containers beside an `ENGINE.EXE` is
//! the generation's shape, and two games ship it: Dunkle Schatten 2 on
//! `ENGINE.EXE` V0.06.06/R109 and Checker 2000 on V0.04.15/R78. What a
//! game's own module says is which files it must have, which words tell its
//! container apart from the other's, and where its shell keeps its state;
//! [`super::ds2`] and [`super::checker`] are that for the two.
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
use motionvm_motion_forth::Machine;
use motionvm_motion_forth::m32;

use crate::Engine;
use crate::Error;
use crate::Result;
use crate::game::{Game, LocationScheme};
use crate::titles::{Driven, Generation, Title};

/// The games this engine knows on the 32-bit machine, each with the words
/// that tell its container from the other's — `(module, word)` pairs a
/// container has to export.
///
/// The counterpart of the 16-bit table, and it cannot work the same way: there
/// the engine binary beside the container names the game, here every game
/// ships an `ENGINE.EXE`, so the answer has to come out of the data. Dunkle
/// Schatten 2 goes first because its two words are the more specific claim:
/// its module 2 is Checker 2000's grown, so a word that is only in the
/// smaller one does not exist, and the older game is told by what its module
/// 4 drives the story with instead.
const GAMES: &[(Title, &[(u32, &str)])] = &[
    (Title::DunkleSchatten2, super::ds2::SIGNATURE),
    (Title::Checker2000, super::checker::SIGNATURE),
];

/// Which 32-bit game `dir` holds, or `None` if it holds no container, or a
/// container whose scripts export none of the roster's signatures.
///
/// File names go no further than the generation here — both games ship
/// `NNN.RSC` beside an `ENGINE.EXE` — so the container is opened and its
/// scripts asked, the way the 16-bit side asks the binary. That costs a read
/// of the containers where the 16-bit probe costs a directory listing; the
/// answer is a game rather than a shape, and a directory holding some other
/// 32-bit game is refused by name instead of opened under the wrong one.
pub(super) fn detect(dir: &Path) -> Option<Title> {
    if !motionvm_motion_formats::m32::has_container(dir) {
        return None;
    }
    let bank = Bank::open_dir(dir).ok()?;
    GAMES
        .iter()
        .find(|(_, signature)| missing_signature(&bank, signature).is_none())
        .map(|&(title, _)| title)
}

/// The first `(module, word)` of `signature` the bank's scripts do not
/// export, or `None` when the container is the game's.
fn missing_signature(bank: &Bank, signature: &[(u32, &str)]) -> Option<(u32, String)> {
    let exports = |module: u32, word: &str| -> bool {
        bank.item(Kind::Script, cell_index(module))
            .ok()
            .flatten()
            .and_then(|item| ScrModule::parse(item).ok())
            .is_some_and(|m| m.entries.iter().any(|e| e.name == word))
    };
    signature
        .iter()
        .find(|(module, word)| !exports(*module, word))
        .map(|&(module, word)| (module, word.to_string()))
}

/// A module number as a slot index.
fn cell_index(module: u32) -> usize {
    motionvm_motion_forth::cell::index(module)
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

/// Opens the bank, binds the kernel out of `ENGINE.EXE`, and checks that the
/// container is the game the caller named. The modules are the game's own to
/// load, `4 =>GET` first, as `SYSTEM.RSC` has it.
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
/// both games have them. Checker 2000 exports `STARTUP`, `START` and
/// `INCLLOC` from modules 3, 4 and 5 exactly as Dunkle Schatten 2 does. What
/// is a game's own is what its scripts do with the template — see
/// [`super::ds2::SIGNATURE`] and [`super::checker::SIGNATURE`]. [`detect`]
/// asks the same question to name the game; asking it here again is what
/// keeps a caller that names a game itself from running its words against
/// the other game's data and failing somewhere inside the VM, under the
/// wrong game's name — the part that misleads.
///
/// The other directory this catches is a copy of the right game missing the
/// container its script is in, which reaches exactly the same state — hence a
/// message that names the two cases rather than deciding between them, which
/// the data cannot do.
pub(super) fn open(
    dir: &Path,
    title: Title,
    required: &[(&'static str, &'static str)],
    signature: &[(u32, &str)],
    location: LocationScheme,
    shell: Shell,
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
    // Bound out of this build's own init: the domain table's ordinals begin
    // where the count of everything registered before it puts them, and the
    // two builds count differently.
    let binding = motionvm_motion_formats::m32::le::binding_of(&img, &kernel)
        .map_err(|e| Error::data(&engine_exe, e))?;
    let vm = m32::Vm::new(&binding);

    // The one thing the opener reads out of the scripts is the signature, and
    // it reads it off the container: the machine holds what the game has
    // loaded and nothing else, and nothing is loaded before `START` runs.
    if let Some((_, word)) = missing_signature(&bank, signature) {
        return Err(Error::NotThisGame {
            dir: dir.to_path_buf(),
            title: title.short(),
            word,
        });
    }

    // The two capabilities the two 32-bit builds disagree on, read off this
    // build's handlers rather than off the game: what a text's insert slot
    // is, and with it whether a text without one is formatted at all; and
    // whether a curtain waits between its bands or runs at the presenter's
    // pace.
    let inserts = if motionvm_motion_formats::m32::le::sdinsert_takes_kind(&img, &kernel)
        .map_err(|e| Error::data(&engine_exe, e))?
    {
        crate::profile::TextInserts::Kinded
    } else {
        crate::profile::TextInserts::Addresses
    };
    let curtains_wait = motionvm_motion_formats::m32::le::fades_wait(&img, &kernel)
        .map_err(|e| Error::data(&engine_exe, e))?;
    let profile = crate::Profile {
        inserts,
        curtains_wait,
        ..crate::Profile::motion32()
    };
    let mut engine = Engine::new(profile).with_bank(dir, bank);
    // Which of the engine's words each of this kernel's ordinals is, decided
    // here and not again. The 32-bit reading, because ten names mean something
    // else on the other machine.
    engine.bind_words(&binding, true);
    Ok(Game {
        vm,
        engine,
        title,
        location,
        shell: Some(shell),
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

    fn generation(&self) -> Generation {
        Generation::Motion32
    }

    fn display_size(&self) -> motionvm_playable::Size {
        self.engine.display_size()
    }

    /// Square at the 640×480 the game asks for; a mode with another grid
    /// would answer its own shape.
    fn pixel_aspect(&self) -> motionvm_playable::PixelAspect {
        super::pixel_aspect_of(self.engine.display_size())
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

    fn seed(&mut self, seed: u64) {
        Machine::seed(&mut self.vm, seed);
    }

    fn pointer(&mut self, x: i32, y: i32) {
        self.pointer_position(x, y);
    }

    fn button(&mut self, which: motionvm_playable::Button, down: bool) {
        self.note_button(which, down);
    }

    fn key_down(&mut self, press: &KeyPress) {
        self.engine.push_key(press);
    }

    fn frame(&mut self) -> motionvm_render::Frame<'_> {
        self.engine.frame()
    }

    fn frame_duration(&self) -> Option<std::time::Duration> {
        Game::<m32::Vm>::frame_duration(self)
    }

    fn set_music(&mut self, sink: Box<dyn crate::MusicSink>) {
        Game::<m32::Vm>::set_music(self, sink);
    }

    fn set_saves(&mut self, dir: &Path) -> Result<()> {
        Game::<m32::Vm>::set_saves(self, dir)
    }

    fn saves(&self) -> Option<&Path> {
        Game::<m32::Vm>::saves(self)
    }

    /// The engine's, plus the machine's own: every address in a module that
    /// is not loaded that the game read or wrote.
    ///
    /// `@` validates nothing (`0x626fc`) and the game leans on it — five of
    /// the sixteen locations read through a pointer that lands in module 0 —
    /// so these are expected rather than alarming, and the number is the
    /// point: it says how much of a run went past an address nobody can
    /// account for. The first few addresses are named because a *new* one
    /// appearing is what would be worth looking at.
    fn diagnostics(&self) -> Vec<motionvm_playable::Diagnostic> {
        let mut out = self.engine.diagnostics();
        let loose = self.vm.mem.loose();
        if !loose.is_empty() {
            let total: u32 = loose.values().sum();
            let (shown, rest) = (loose.iter().take(8), loose.len().saturating_sub(8));
            let mut detail = format!(
                "{total} read{} at {} address{}: {}",
                if total == 1 { "" } else { "s" },
                loose.len(),
                if loose.len() == 1 { "" } else { "es" },
                shown
                    .map(|(a, n)| format!("{a} ×{n}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            if rest > 0 {
                detail.push_str(&format!(", and {rest} more"));
            }
            out.push(motionvm_playable::Diagnostic {
                subject: "reads into modules that are not loaded",
                detail,
            });
        }
        out
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
/// generation's, in the functions below, which read the pair the opener put
/// on the game.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shell {
    /// The module the compiler placed the shell variables in.
    pub(super) module: u32,
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
        // The first of those two lines is the one no bytecode contains, so it
        // is done from here: `4 =>GET`, which loads module 4 into slot 1, and
        // everything `START` loads follows behind it.
        self.engine.get_module(&mut self.vm.mem, 4)?;
        let addr = self.address(4, "START").ok_or_else(|| Error::NoWord {
            module: 4,
            name: "START".into(),
        })?;
        self.vm.start(addr);
        self.running = true;
        Ok(())
    }

    /// Loads module `n` as `=>GET` would, for a test that drives a word of it
    /// without going through the game's own way into the location that
    /// loads it.
    pub fn load_module(&mut self, n: u32) -> Result<()> {
        self.engine.get_module(&mut self.vm.mem, n)?;
        Ok(())
    }

    /// The task and phase the location is currently in — the intro's
    /// progress in Dunkle Schatten 2, the story step and the location's task
    /// in Checker 2000.
    pub fn task_phase(&self) -> (i32, i32) {
        let Some(shell) = self.shell else {
            return (0, 0);
        };
        (
            self.get_var(shell.module, shell.task).unwrap_or(0),
            self.get_var(shell.module, shell.task_phase).unwrap_or(0),
        )
    }
}
