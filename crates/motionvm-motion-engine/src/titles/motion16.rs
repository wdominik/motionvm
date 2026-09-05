//! What the 16-bit games need to be opened and run: the same player, the same
//! authoring template, the same frame handler.
//!
//! There is less here than for Dunkle Schatten 2, and that is the engine's
//! doing, not an omission: input reaches the scripts through kernel words alone
//! (`CTRL` opens with `?KEY DUP _AKTKEY !` and reads the mouse with `MOUSELK`),
//! the boot is a header word rather than a bootstrap file, and the frame
//! handler is installed by the scripts with `SCRCTRL`.
//!
//! A game's own module — [`super::enviro`], [`super::hfa`], [`super::jeffjet`],
//! [`super::vloomes`] — says which files it ships, which binary the kernel
//! comes out of and where it keeps its location, and nothing more. Rust allows
//! one [`Driven`] for one concrete `Game<m16::Vm>`, so the games share this one
//! and answer [`Driven::name`] out of the field the opener set.

use motionvm_playable::KeyPress;
use std::path::Path;

use motionvm_motion_formats::m16::{Container, Segment, mz, scr::ScrModule};
use motionvm_motion_forth::Machine;
use motionvm_motion_forth::m16;

use crate::Result;
use crate::game::{Game, LocationScheme};
use crate::resources::Resources;
use crate::titles::{Driven, Generation, Title};
use crate::{Engine, Error};

/// Which of `required` the directory `dir` does not hold, as `(what, what
/// for)`.
pub(super) fn missing_data(
    dir: &Path,
    required: &[(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    required
        .iter()
        .filter(|(name, _)| motionvm_motion_formats::find_ci(dir, name).is_none())
        .copied()
        .collect()
}

/// The games this engine knows on the 16-bit machine, each with the binary
/// its kernel table comes out of.
///
/// The binary is what tells them apart: all four ship a `DATA.-1-`, and none
/// ships another's player. A game added here without a line in this table
/// opens when it is named and is not found by looking.
const GAMES: &[(Title, &str)] = &[
    (Title::DieEnviroKidsGreifenEin, super::enviro::ENGINE),
    (Title::JeffJet, super::jeffjet::ENGINE),
    (Title::HilfeFuerAmajambere, super::hfa::ENGINE),
    (Title::VictorLoomes, super::vloomes::ENGINE),
];

/// Which 16-bit game `dir` holds, or `None` if the binary beside its container
/// is none of the ones known here.
///
/// Saying "none" beats naming one of them and then failing on its missing
/// files: a `DATA.-1-` with an unknown player is a game this cannot open, and
/// that is the useful thing to report.
pub(super) fn detect(dir: &Path) -> Option<Title> {
    GAMES
        .iter()
        .find(|(_, exe)| motionvm_motion_formats::find_ci(dir, exe).is_some())
        .map(|&(title, _)| title)
}

/// The location scheme the three 1995/96 builds' games share.
///
/// Module 601 declares `NEXTLOC`, `ACTLOC` and `STARTLOC` in all three, with
/// -1 for "none": `CTRL` polls `NEXTLOC`, `ACTLOC` says whether a location has
/// been entered at all, and `STARTLOC` holds the one `RUN` entered. It lives
/// here rather than three times over because the three games really do share
/// it — Victor Loomes, which does not, keeps its own in its own module.
pub(super) const MODULE_601: LocationScheme = LocationScheme {
    module: 601,
    next: "NEXTLOC",
    fallback: Some(("ACTLOC", "STARTLOC")),
    unset_below: Some(0),
};

/// Opens the container, binds the kernel out of `exe`, and loads the boot
/// module the container's header names — and only that one: the 16-bit machine
/// loads modules as the scripts ask for them with `=>GET`, because their word
/// ids only resolve against what is resident.
///
/// A free function rather than `Game::<m16::Vm>::open`, because an associated
/// function of that name on both machines' `Game` cannot be called by path
/// without naming the machine, and every caller of the 32-bit `Game::open`
/// would have to.
pub(super) fn open(
    dir: &Path,
    title: Title,
    exe: &str,
    required: &[(&'static str, &'static str)],
    location: LocationScheme,
) -> Result<Game<m16::Vm>> {
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
    let container = Container::open_dir(dir).map_err(|e| Error::data(dir, e))?;
    let exe =
        motionvm_motion_formats::find_ci(dir, exe).ok_or_else(|| Error::missing_file(dir, exe))?;
    let img = mz::Image::open(&exe).map_err(|e| Error::data(&exe, e))?;
    let words = mz::kernel_words(&img);
    // Read from this build's own `?XINSIDE`, because the four builds do not
    // agree — and not by date: `ENVIRO.EXE` and `BMZ.EXE` pass over an
    // all-zero hot area, `HPPLAY.EXE` and `LL.EXE` take it as a rectangle at
    // the origin, and Jeff Jet's build is the younger of its pair.
    let skips_holes = mz::skips_empty_areas(&img, &words);
    // The walk builder's two build variants, read the same way: only
    // `ENVIRO.EXE` takes a zero shrink as 1000, and only `LL.EXE` closes
    // with the heading pass.
    let walk_defaults_shrink = mz::croute_defaults_shrink(&img, &words);
    let walk_smooths_headings = mz::croute_smooths_headings(&img, &words);
    // And whether a screen refuses its hundred-and-first descriptor, which
    // only `LL.EXE` does not test.
    let screen_holds_a_hundred = mz::newsetdesc_capped(&img, &words);
    let binding = mz::binding_of(&img, &words).map_err(|e| Error::data(&exe, e))?;
    let mut vm = m16::Vm::new(&binding);
    let boot = container.boot();
    let item = container
        .item(Segment::Scr, usize::from(boot.module))
        .map_err(|e| Error::data(dir, e))?
        .ok_or_else(|| Error::EmptyBootModule {
            source: container.source().to_string(),
            module: boot.module,
        })?;
    let parsed = ScrModule::parse(item).map_err(|e| Error::data(dir, e))?;
    vm.load(item, &parsed)?;
    // 320×200: the mode `TOGFX` enters in this engine, which has no
    // `SETRES` to ask for another.
    // The four capabilities that differ between the four 16-bit builds go
    // into the profile before the engine exists, which is the whole point of
    // there being one: nothing writes a capability into a built engine.
    let profile = crate::Profile {
        skips_holes,
        walk_defaults_shrink,
        walk_smooths_headings,
        screen_holds_a_hundred,
        ..crate::Profile::motion16()
    };
    let mut engine = Engine::new(profile).with_container(dir, container);
    // Which of the engine's words each of this kernel's ordinals is, decided
    // here and not again — in the 16-bit reading, which is what gives
    // `FADEIN`, `SETBUF` and eight others this machine's meaning.
    engine.bind_words(&binding, false);
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

impl Game<m16::Vm> {
    /// Begins the game the way it begins itself: at the word the container's
    /// header names — module 100's `RUN`, word id 401 — which loads the
    /// library, plays the intro, enters its first location and runs
    /// `ANIMPLAY`. Which location that is, is the game's: 1 in Die Enviro-Kids
    /// greifen ein, 13 in Jeff Jet, 20 in Hilfe für Amajambere.
    pub fn start(&mut self) -> Result<()> {
        let Some(Resources::Motion16(c)) = self.engine.resources.as_ref() else {
            return Err(Error::NoContainer);
        };
        let boot = c.boot();
        let addr = self
            .vm
            .callback_target(i32::from(boot.word))
            .ok_or(Error::UnboundBootWord { word: boot.word })?;
        self.engine.mark_resident(u32::from(boot.module));
        self.vm.start(addr)?;
        self.running = true;
        Ok(())
    }
}

impl Driven for Game<m16::Vm> {
    /// Out of the field, not out of the type: the 16-bit games are all the same
    /// `Game<m16::Vm>`, and only the opener knows which of them it opened.
    fn name(&self) -> &str {
        self.title.name()
    }

    fn generation(&self) -> Generation {
        Generation::Motion16
    }

    fn display_size(&self) -> motionvm_playable::Size {
        self.engine.display_size()
    }

    /// The 320×200×256 mode `TOGFX` enters filled a 4:3 monitor, so one
    /// pixel stood (4/3)/(320/200) = 6/5 as tall as wide: a 5:6 pixel.
    fn pixel_aspect(&self) -> motionvm_playable::PixelAspect {
        super::pixel_aspect_of(self.engine.display_size())
    }

    /// Startup runs to the game's own parked loop before returning, under
    /// [`Game::park`]'s budget: the window does not exist yet, so a startup
    /// that never parks has to answer instead of spin.
    fn start(&mut self) -> Result<()> {
        Game::<m16::Vm>::start(self)?;
        Game::<m16::Vm>::park(self)
    }

    /// One keystroke per step: the 16-bit engine's `CTRL` likewise reads
    /// `?KEY` once per round and stores it into `_AKTKEY` itself, so each
    /// frame takes one keystroke out of the buffer and no more. The buttons cross
    /// as the level the original's `MOUSELK` read — held is held, and the
    /// scripts do their own debouncing, as they always did.
    fn step(&mut self) -> Result<()> {
        let key = self.engine.pop_key();
        self.deliver_key(key)?;
        let (left, right) = self.buttons_this_frame();
        Game::<m16::Vm>::pointer_buttons(self, left, right)?;
        Game::<m16::Vm>::step(self)
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
        Game::<m16::Vm>::frame_duration(self)
    }

    fn set_music(&mut self, sink: Box<dyn crate::MusicSink>) {
        Game::<m16::Vm>::set_music(self, sink);
    }

    fn set_saves(&mut self, dir: &Path) -> Result<()> {
        Game::<m16::Vm>::set_saves(self, dir)
    }

    fn saves(&self) -> Option<&Path> {
        Game::<m16::Vm>::saves(self)
    }

    /// The engine's, and nothing of the machine's: this machine has one flat
    /// address space with no notion of a read into a module that is not there,
    /// so there is no counterpart to the 32-bit stray-read table.
    fn diagnostics(&self) -> Vec<motionvm_playable::Diagnostic> {
        self.engine.diagnostics()
    }

    fn finished(&self) -> bool {
        Game::<m16::Vm>::finished(self)
    }

    fn request_location(&mut self, n: i32) -> Result<()> {
        Game::<m16::Vm>::request_location(self, n)
    }

    fn start_location(&self) -> Option<i32> {
        Game::<m16::Vm>::start_location(self)
    }
}

#[cfg(test)]
mod tests {
    use super::GAMES;
    use crate::titles::{Generation, Title};

    /// Every 16-bit game is in this file's table, and every entry of it is a
    /// 16-bit game.
    ///
    /// The two rosters are separate on purpose — `Title::ALL` is the order the
    /// documentation lists the games in, `GAMES` is what `detect` walks — and
    /// separate lists agree by convention until something holds them together.
    /// The failure they can drift into is the one the table's own comment
    /// names: "a game added here without a line in this table opens when it is
    /// named and is not found by looking", which is a game the roster offers
    /// and a directory never resolves to.
    #[test]
    fn the_two_rosters_hold_the_same_games() {
        let listed: Vec<Title> = Title::ALL
            .into_iter()
            .filter(|t| t.generation() == Generation::Motion16)
            .collect();
        let detectable: Vec<Title> = GAMES.iter().map(|&(t, _)| t).collect();
        for title in &listed {
            assert!(
                detectable.contains(title),
                "{} is on the roster and not in GAMES, so it opens when named \
                 and is never found by looking",
                title.short()
            );
        }
        for title in &detectable {
            assert!(
                listed.contains(title),
                "{} is in GAMES and not on the roster",
                title.short()
            );
        }
    }

    /// Each of them names a different binary, which is the whole of how they
    /// are told apart: all four ship a `DATA.-1-`.
    #[test]
    fn each_game_is_found_by_a_binary_of_its_own() {
        for (i, &(title, exe)) in GAMES.iter().enumerate() {
            for &(other, other_exe) in &GAMES[i + 1..] {
                assert_ne!(
                    exe,
                    other_exe,
                    "{} and {} would be told apart by nothing",
                    title.short(),
                    other.short()
                );
            }
        }
    }
}
