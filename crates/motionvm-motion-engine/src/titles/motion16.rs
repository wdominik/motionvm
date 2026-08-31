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
//! one [`Driven`] for one concrete `Game<Vm>`, so the games share this one
//! and answer [`Driven::name`] out of the field the opener set.

use motionvm_playable::KeyPress;
use std::path::Path;

use motionvm_motion_formats::m16::{Container, Segment, mz, scr::ScrModule};
use motionvm_motion_forth::m16::Vm;
use motionvm_motion_forth::{Address, Machine};

use crate::Result;
use crate::game::{Game, Hooks, LocationScheme};
use crate::resources::Resources;
use crate::titles::{Driven, Title};
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
/// A free function rather than `Game::<Vm>::open`, because an associated
/// function of that name on both machines' `Game` cannot be called by path
/// without naming the machine, and every caller of the 32-bit `Game::open`
/// would have to.
pub(super) fn open(
    dir: &Path,
    title: Title,
    exe: &str,
    required: &[(&'static str, &'static str)],
    location: LocationScheme,
) -> Result<Game<Vm>> {
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
    // agree: the two later ones pass over an all-zero hot area, the two older
    // ones take it as a rectangle at the origin.
    let skips_holes = mz::skips_empty_areas(&img, &words);
    let binding = mz::binding_of(&img, &words).map_err(|e| Error::data(&exe, e))?;
    let mut vm = Vm::new(&binding);
    let boot = container.boot();
    let item = container
        .item(Segment::Scr, boot.module as usize)
        .map_err(|e| Error::data(dir, e))?
        .ok_or_else(|| Error::EmptyBootModule {
            source: container.source().to_string(),
            module: boot.module,
        })?;
    let parsed = ScrModule::parse(item).map_err(|e| Error::data(dir, e))?;
    vm.load(item, &parsed)?;
    // 320×200: the mode `TOGFX` enters in this engine, which has no
    // `SETRES` to ask for another.
    let mut engine = Engine::with_display(320, 200).with_container(dir, container);
    engine.skips_holes = skips_holes;
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

impl Game<Vm> {
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
            .callback_target(boot.word as i32)
            .ok_or(Error::UnboundBootWord { word: boot.word })?;
        self.engine.mark_resident(boot.module as u32);
        self.vm.start(addr)?;
        self.running = true;
        Ok(())
    }

    /// Hands the game this frame's buttons, into the mouse record the
    /// `MOUSE…` words read. No script variable is written — `CTRL` reads
    /// `MOUSELK` itself.
    pub fn pointer_buttons(&mut self, left: bool, right: bool) -> Result<()> {
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

    /// Hands the game the key `?KEY` answers this frame. No script variable
    /// is written — `CTRL` stores `?KEY` into `_AKTKEY` itself.
    pub fn deliver_key(&mut self, key: i32) -> Result<()> {
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
}

impl Hooks for Game<Vm> {
    /// Nothing stands in: `RUN` installs `CTRL` with `400 SCRCTRL` before it
    /// enters `ANIMPLAY`, and the intro installs `ICTRL` before its own, so a
    /// frame without a controller has nothing to run.
    fn fallback_controller(&mut self) -> Result<Option<Address>> {
        Ok(None)
    }
}

impl Driven for Game<Vm> {
    /// Out of the field, not out of the type: the 16-bit games are all the same
    /// `Game<Vm>`, and only the opener knows which of them it opened.
    fn name(&self) -> &str {
        self.title.name()
    }

    fn display_size(&self) -> (u16, u16) {
        self.engine.display_size()
    }

    fn pixel_aspect(&self) -> motionvm_playable::PixelAspect {
        // The 320×200×256 mode `TOGFX` enters filled a 4:3 monitor, so one
        // pixel stood (4/3)/(320/200) = 6/5 as tall as wide: a 5:6 pixel.
        motionvm_playable::PixelAspect {
            width: 5,
            height: 6,
        }
    }

    /// Startup runs to the game's own parked loop before returning, under
    /// [`Game::park`]'s budget: the window does not exist yet, so a startup
    /// that never parks has to answer instead of spin.
    fn start(&mut self) -> Result<()> {
        Game::<Vm>::start(self)?;
        Game::<Vm>::park(self)
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
