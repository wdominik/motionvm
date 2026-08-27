//! What both 16-bit games need to be opened and run: the same player, the same
//! authoring template, the same frame handler.
//!
//! There is less here than for Dunkle Schatten 2, and that is the engine's
//! doing, not an omission: input reaches the scripts through kernel words alone
//! (`CTRL` opens with `?KEY DUP _AKTKEY !` and reads the mouse with `MOUSELK`),
//! the boot is a header word rather than a bootstrap file, and the frame
//! handler is installed by the scripts with `SCRCTRL`.
//!
//! A game's own module — [`super::enviro`], [`super::jeffjet`] — says which
//! files it ships and which binary the kernel comes out of, and nothing more.
//! Rust allows one [`Playable`] for one concrete `Game<Vm>`, so the two games
//! share this one and answer [`Playable::title`] out of the field the opener
//! set.

use std::path::Path;

use motionvm_formats::m16::{Container, Segment, mz, scr::ScrModule};
use motionvm_forth::m16::Vm;
use motionvm_forth::{Address, Machine};

use crate::Engine;
use crate::game::{Game, Hooks, Res};
use crate::resources::Resources;
use crate::titles::{Playable, Title};

/// Which of `required` the directory `dir` does not hold, as `(what, what
/// for)`.
pub(super) fn missing_data(
    dir: &Path,
    required: &[(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    required
        .iter()
        .filter(|(name, _)| motionvm_formats::find_ci(dir, name).is_none())
        .copied()
        .collect()
}

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
) -> Res<Game<Vm>> {
    if !dir.is_dir() {
        return Err(format!("{}: no such directory", dir.display()).into());
    }
    let missing = missing_data(dir, required);
    if !missing.is_empty() {
        let names: Vec<&str> = missing.iter().map(|(n, _)| *n).collect();
        return Err(format!(
            "{} is not a complete {} directory\n  missing: {}\n  \
                 This needs the files of an original installation; \
                 see \"Game data\" in the README.",
            dir.display(),
            title.name(),
            names.join(", "),
        )
        .into());
    }
    let container = Container::open_dir(dir)?;
    let exe = motionvm_formats::find_ci(dir, exe)
        .ok_or_else(|| format!("{}: no {exe}", dir.display()))?;
    let img = mz::Image::open(exe)?;
    let binding = mz::binding_of(&mz::kernel_words(&img))?;
    let mut vm = Vm::new(&binding);
    let boot = container.boot();
    let item = container
        .item(Segment::Scr, boot.module as usize)?
        .ok_or_else(|| {
            format!(
                "{}: the boot module {} is empty",
                container.source(),
                boot.module
            )
        })?;
    let parsed = ScrModule::parse(item)?;
    vm.load(item, &parsed)?;
    // 320×200: the mode `TOGFX` enters in this engine, which has no
    // `SETRES` to ask for another.
    let engine = Engine::with_display(320, 200).with_container(dir, container);
    Ok(Game {
        vm,
        engine,
        title,
        running: false,
        ending: false,
        over: false,
        parked: None,
    })
}

impl Game<Vm> {
    /// Begins the game the way it begins itself: at the word the container's
    /// header names — module 100's `RUN`, word id 401 — which loads the
    /// library, plays the intro, enters location 1 and runs `ANIMPLAY`.
    pub fn start(&mut self) -> Res<()> {
        let Some(Resources::Motion16(c)) = self.engine.resources.as_ref() else {
            return Err("the engine holds no 16-bit container".into());
        };
        let boot = c.boot();
        let addr = self
            .vm
            .callback_target(boot.word as i32)
            .ok_or_else(|| format!("the boot word {} is bound to no loaded module", boot.word))?;
        self.engine.mark_resident(boot.module as u32);
        self.vm.start(addr)?;
        self.running = true;
        Ok(())
    }

    /// Hands the game this frame's input: the pointer and the buttons into
    /// the mouse record the `MOUSE…` words read, the key where `?KEY` finds
    /// it. No script variable is written — `CTRL` stores `?KEY` into
    /// `_AKTKEY` and reads `MOUSELK` itself.
    pub fn set_input(&mut self, x: i32, y: i32, left: bool, right: bool, key: i32) -> Res<()> {
        self.engine.key = key;
        self.engine.mouse.x = x;
        self.engine.mouse.y = y;
        self.engine.mouse.left = left as i32;
        self.engine.mouse.right = right as i32;
        Ok(())
    }
}

impl Hooks for Game<Vm> {
    /// Nothing stands in: `RUN` installs `CTRL` with `400 SCRCTRL` before it
    /// enters `ANIMPLAY`, and the intro installs `ICTRL` before its own, so a
    /// frame without a controller has nothing to run.
    fn fallback_controller(&mut self) -> Res<Option<Address>> {
        Ok(None)
    }
}

impl Playable for Game<Vm> {
    /// Out of the field, not out of the type: both 16-bit games are the same
    /// `Game<Vm>`, and only the opener knows which of them it opened.
    fn title(&self) -> Title {
        self.title
    }

    fn display_size(&self) -> (u16, u16) {
        self.engine.display_size()
    }

    fn pixel_aspect(&self) -> (u32, u32) {
        // The 320×200×256 mode `TOGFX` enters filled a 4:3 monitor, so one
        // pixel stood (4/3)/(320/200) = 6/5 as tall as wide.
        (6, 5)
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

    /// Through `NEXTLOC` in module 601, the way the game itself moves
    /// between locations: `CTRL` (module 100, word 400) runs
    /// `NEXTLOC @ -1 != IF NEXTLOC @ INCLLOC -1 NEXTLOC ! THEN` every frame,
    /// and the scripts store their exits there — `13 NEXTLOC !` in module
    /// 609, for one. There is no way around `RUN`'s own first location: it
    /// stores 1 into `STARTLOC` and enters it before `CTRL` gets a frame, so
    /// the request is honored one frame later, from inside location 1.
    fn request_location(&mut self, n: i32) -> Res<()> {
        self.set_var(601, "NEXTLOC", n)
    }

    /// The pending `NEXTLOC` if one is set, else 1 — the location `RUN`
    /// enters itself.
    fn start_location(&self) -> Option<i32> {
        match self.get_var(601, "NEXTLOC") {
            Some(n) if n >= 0 => Some(n),
            _ => Some(1),
        }
    }
}
