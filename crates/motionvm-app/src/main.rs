//! A window for the engine.
//!
//! The frontend is deliberately thin. The engine renders into an indexed
//! 640x480 framebuffer and this does two things with it: expand it through the
//! current palette, and put it on screen at an integer scale.
//!
//! That framebuffer has been checked pixel for pixel against the original
//! engine's own output: the title screen, location 23, all 307 200 pixels of
//! it over 206 distinct indices, every index mapping to one color and every
//! color back to one index. **One frame** — the claim is worth exactly that:
//! one scene, drawn through scaling, layering, the palette and the composition
//! rule, agreeing completely.
//!
//! Integer scaling is not a preference. The game is 640x480 of hand-drawn
//! pixel art; any other factor resamples it and invents colors that were never
//! in the palette. So the picture is scaled by whole numbers and centered in
//! whatever space is left, with black around it.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use motionvm_engine::Game;
use motionvm_render::Framebuffer;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
/// How much bigger than the game the window is asked to be.
///
/// Only whole numbers: the art is 640x480 hand-drawn pixels and a fractional
/// factor smears them. Nothing else needs telling — [`blit`] works the factor
/// out from the window it is given, and the pointer mapping divides by the
/// same one, so both follow this on their own.
const SCALE: u32 = 3;

// The key codes `?KEY` answers with.
//
// **`?KEY` returns plain ASCII**, not a flag. Settled by the comparisons the
// game's own bytecode makes against `_AKTKEY`: every one of them, across
// modules 2, 4 and 5, is against an ASCII code.
//
// | code | key | what reads it |
// |---|---|---|
// | 8 | Backspace | the debug input line, deleting a character (module 4, `0x051a0`) |
// | 13 | Return | the dialogue and caption advance |
// | 27 | Escape | the quit page (`0x02c40`) |
// | 48…57 | `0`…`9` | the debug teleport, built up digit by digit (`0x04e60`) |
// | 103 | `g` | the debug sprite viewer (`0x04d40`) |
// | 105 | `i` | the debug Forth input line (`0x05020`) |
//
// Anything else is only ever tested as `_AKTKEY @ 0 >`, i.e. "a key was
// pressed", so character keys simply pass their own code through.
const BACKSPACE: i32 = 8;
const RETURN: i32 = 13;
const ESCAPE: i32 = 27;
const SPACE: i32 = 32;

mod sound;

fn main() {
    // Not `main() -> Result`, which would `Debug`-print whatever came back:
    // `Error: Io(Os { code: 2, kind: NotFound, … })` is the shape of the type,
    // not of the problem, and the one error a first-time player is most likely
    // to hit — a game directory that is not one — has a message written for
    // them that only survives if it goes out through `Display`.
    if let Err(e) = run() {
        eprintln!("motionvm: {e}");
        std::process::exit(1);
    }
}

/// Where the program keeps the files it writes: savegames and screenshots.
///
/// **Not the working directory.** Both of these used to default to a relative
/// path, which meant they landed wherever the program happened to be started —
/// for anyone building from a checkout, in the source tree. Runtime output does
/// not belong beside the sources, and no amount of ignoring it there makes it
/// belong.
///
/// Resolved by hand rather than through a crate, for the same reason the
/// argument parser is: three environment variables and a join is not worth a
/// dependency.
///
/// | platform | directory |
/// |---|---|
/// | macOS | `$HOME/Library/Application Support/motionvm` |
/// | Windows | `%APPDATA%\\motionvm` |
/// | anything else | `$XDG_DATA_HOME/motionvm`, else `$HOME/.local/share/motionvm` |
///
/// `None` when the environment names no home at all — a bare `cron` job, a
/// daemon, a container without `HOME`. The caller then falls back to a relative
/// path, because refusing to run would be worse than writing where the old
/// default wrote.
fn data_dir() -> Option<PathBuf> {
    let var = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    if cfg!(target_os = "macos") {
        Some(var("HOME")?.join("Library/Application Support/motionvm"))
    } else if cfg!(target_os = "windows") {
        Some(var("APPDATA")?.join("motionvm"))
    } else {
        var("XDG_DATA_HOME")
            .or_else(|| var("HOME").map(|h| h.join(".local/share")))
            .map(|d| d.join("motionvm"))
    }
}

/// The default for a file the program writes: under [`data_dir`] when there is
/// one, otherwise the plain relative name it always used.
fn data_path(name: &str) -> PathBuf {
    data_dir().map_or_else(|| PathBuf::from(name), |d| d.join(name))
}

const USAGE: &str = "\
motionvm — the MOTION engine, for the games built with it

usage: motionvm [GAMEDIR] [options]

  GAMEDIR         the directory holding 001.RSC and ENGINE.EXE.
                  Defaults to ../gamedata.

options:
  --saves DIR     where savegames are written. Defaults to saves/ under the
                  platform data directory, printed on startup.
                  Refused if it is inside GAMEDIR, which stays read-only.
  --shot PATH     where F12 writes its screenshot. Defaults to shot.png in
                  the same place; the path is printed with every shot.
  --loc N         start in location N instead of playing the intro.
  --no-sound      do not open an audio device.
  -h, --help      this text.
";

/// Everything the command line can say.
#[derive(Debug)]
struct Options {
    dir: PathBuf,
    /// Where the game's own save menu writes. The original puts its five slots
    /// beside `ENGINE.EXE`, among the shipped data. That is not a place to
    /// write to here — the game directory may well be a read-only copy of the
    /// discs — so the slots go under [`data_dir`], never beside the sources.
    saves: PathBuf,
    /// Where F12 writes the indexed picture. Under [`data_dir`] for the same
    /// reason as [`Options::saves`].
    shot: PathBuf,
    wanted: Option<i32>,
    quiet: bool,
    help: bool,
}

/// Reads the command line, or says what is wrong with it.
///
/// A hand-written parser rather than a crate, to keep the dependency list at
/// the six it needs to run at all.
///
/// The one thing worth being careful about is what a positional argument is.
/// "The first argument that does not start with `--`" is the obvious rule and
/// the wrong one: it reads `motionvm --loc 5` as a game directory called `5`,
/// because the value of a flag is indistinguishable from a positional unless
/// the flag is consumed together with it. So flags are walked in order and
/// value-taking ones swallow their operand, and only what is left over can be
/// the directory.
fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut opt = Options {
        dir: PathBuf::from("../gamedata"),
        saves: data_path("saves"),
        shot: data_path("shot.png"),
        wanted: None,
        quiet: false,
        help: false,
    };
    let mut positional = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |flag: &str| -> Result<String, String> {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match a.as_str() {
            "-h" | "--help" => opt.help = true,
            "--no-sound" => opt.quiet = true,
            "--saves" => opt.saves = PathBuf::from(value("--saves")?),
            "--shot" => opt.shot = PathBuf::from(value("--shot")?),
            "--loc" => {
                let v = value("--loc")?;
                opt.wanted = Some(
                    v.parse()
                        .map_err(|_| format!("--loc wants a number, not {v:?}"))?,
                );
            }
            // Rejected rather than ignored: a mistyped flag that is silently
            // dropped looks exactly like one that did nothing, and the two are
            // worth telling apart.
            _ if a.starts_with('-') && a.len() > 1 => {
                return Err(format!("unknown option {a}\n\n{USAGE}"));
            }
            _ if positional.is_some() => return Err(format!("more than one game directory: {a}")),
            _ => positional = Some(a.clone()),
        }
    }
    if let Some(p) = positional {
        opt.dir = PathBuf::from(p);
    }
    Ok(opt)
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Options {
        dir,
        saves,
        shot,
        wanted,
        quiet,
        help,
    } = parse_args(&args)?;
    if help {
        print!("{USAGE}");
        return Ok(());
    }

    let mut game = Game::open(&dir)?;
    // Before `start()`: the very first location is entered during startup and
    // its `STARTTUNE` has to find a sink already in place.
    let audio = if quiet {
        None
    } else {
        match sound::open(&dir) {
            Ok((stream, music)) => {
                game.set_music(Box::new(music));
                Some(stream)
            }
            // Silence is not a reason to stop: the game is playable without it.
            Err(e) => {
                eprintln!("sound is off: {e}");
                None
            }
        }
    };
    if let Err(e) = game.set_saves(&saves) {
        // Not fatal: the game runs, the slot row simply stays empty and a click
        // on one stops by name rather than writing somewhere it should not.
        eprintln!("savegames are off: {e}");
    } else {
        // Printed because the default is no longer somewhere the player is
        // standing. A directory they cannot find is a directory they will
        // think is empty.
        eprintln!("savegames in {}", saves.display());
    }
    // `4:START` loads the modules, initializes, registers `ICTRL` as the
    // controller and then enters `ANIMPLAY`, the game's own frame loop. It
    // parks there; from that point every frame belongs to the controller, and
    // the controller is what enters the first location.
    //
    // Nothing may follow this with an `INCLLOC` of its own: an entry here
    // overwrites the execution `START` has just parked, and the machine runs
    // off to address 0.
    game.start()?;
    while game.pump()? {}

    // `--loc` is for looking at some other location without playing to it. It
    // is set after the run above because `STARTUP` assigns `_STARTLOC` itself,
    // and it goes through that variable rather than around it: `ICTRL` enters
    // whatever stands there once no location is active.
    if let Some(loc) = wanted {
        game.set_var(2, "_STARTLOC", loc)?;
    }
    eprintln!(
        "starting at location {}",
        game.start_location().unwrap_or(0)
    );

    let event_loop = EventLoop::new()?;
    let mut app = App {
        game,
        window: None,
        surface: None,
        next_frame: Instant::now(),
        click: false,
        right_click: false,
        key: 0,
        cursor: (0, 0),
        paused: false,
        frames: 0,
        shot,
        colors: [0; 256],
        lut_palette: None,
        perf: std::env::var_os("MOTIONVM_PERF").is_some().then(Perf::new),
        _audio: audio,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

/// The pace to fall back on before the game has asked for one.
///
/// A frame is the game's own unit of time — `!LTWAIT` is a single decrement of
/// `_LOCTASKWAI` per call of the task manager, so a task asking to wait fifty
/// waits fifty of these.
///
/// The game asks early — `START` runs `25 DELAY` before entering its loop — so
/// this only covers the first few frames. Everything after comes from the game,
/// through [`Game::frame_duration`](motionvm_engine::Game::frame_duration).
/// A fixed 60 here runs the whole game at two and a half times its speed,
/// dialogue and animation alike. 25 is what `START` asks for, so it is what a
/// frame costs until the game says otherwise.
const FRAME: Duration = Duration::from_nanos(1_000_000_000 / 25);

/// The shortest a presented picture is allowed to last.
///
/// During a curtain the game's clock runs a band at a time — down to 5 ms on
/// the title screen, two hundred pictures a second. The original could afford
/// that: its present was a write to VGA memory. Here every present walks the
/// whole physical window, and no display shows two hundred pictures a second
/// anyway; 120 is the fastest panel this runs on. So [`App::tick`] gathers
/// steps until they add up to at least this before presenting. The fade's
/// wall-clock pace is untouched — it just arrives in pictures a screen can
/// actually show.
const MIN_PRESENT: Duration = Duration::from_nanos(1_000_000_000 / 120);

struct App {
    game: Game,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    next_frame: Instant,
    /// Set by a press, cleared once the game has seen it. The task manager
    /// advances a phase whenever a button is down, so a held button would race
    /// through the intro; one frame per press is what a click means here.
    click: bool,
    right_click: bool,
    key: i32,
    /// The pointer in game coordinates, not window ones.
    cursor: (i32, i32),
    /// Whether the frame clock is held, so a shot can be compared at leisure.
    paused: bool,
    /// How many frames have been stepped, printed with a shot so two captures
    /// can be shown to be the same moment.
    frames: u64,
    /// Where F12 writes. Under the platform data directory unless `--shot`
    /// named somewhere else; printed with every shot, because a default the
    /// player cannot see is one they cannot find.
    shot: PathBuf,
    /// The palette expanded to window pixels, one color per index, so [`blit`]
    /// pays a lookup per pixel instead of three reads and two shifts.
    colors: [u32; 256],
    /// The palette [`App::colors`] was built from. A frame with an unchanged
    /// palette — almost all of them — pays a 768-byte compare instead of a
    /// rebuild.
    lut_palette: Option<motionvm_formats::Palette>,
    /// Where a frame's time goes, when `MOTIONVM_PERF` asks to be told.
    perf: Option<Perf>,
    /// Holds the audio stream open. Dropping it stops the music, so it lives
    /// here even though nothing ever reads it.
    _audio: Option<sound::Sound>,
}

/// The tally behind `MOTIONVM_PERF=1`: how many pictures a second actually
/// went out and what each cost, split where the work splits — the engine
/// composing its 640x480, and the blit-and-present that scales with the
/// window. Printed once a second, so a laggy machine can be told apart from a
/// laggy build with numbers rather than impressions.
struct Perf {
    since: Instant,
    presents: u32,
    render: Duration,
    blit: Duration,
    blit_max: Duration,
}

impl Perf {
    fn new() -> Self {
        Perf {
            since: Instant::now(),
            presents: 0,
            render: Duration::ZERO,
            blit: Duration::ZERO,
            blit_max: Duration::ZERO,
        }
    }

    /// One presented picture: `render` is the engine's share, `blit` the
    /// window's. Prints and starts over once a second has been gathered.
    fn note(&mut self, render: Duration, blit: Duration, width: u32, height: u32, scale: u32) {
        self.presents += 1;
        self.render += render;
        self.blit += blit;
        self.blit_max = self.blit_max.max(blit);
        if self.since.elapsed() >= Duration::from_secs(1) {
            eprintln!(
                "perf: {}/s, render {:.1?} avg, blit+present {:.1?} avg, {:.1?} worst, {}x{} at x{}",
                self.presents,
                self.render / self.presents,
                self.blit / self.presents,
                self.blit_max,
                width,
                height,
                scale,
            );
            *self = Perf::new();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Dunkle Schatten 2")
            .with_inner_size(winit::dpi::LogicalSize::new(WIDTH * SCALE, HEIGHT * SCALE));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("cannot open a window: {e}");
                event_loop.exit();
                return;
            }
        };
        match softbuffer::Context::new(window.clone())
            .and_then(|ctx| softbuffer::Surface::new(&ctx, window.clone()))
        {
            Ok(surface) => self.surface = Some(surface),
            Err(e) => {
                eprintln!("cannot get a drawing surface: {e}");
                event_loop.exit();
                return;
            }
        }
        // The engine draws its own pointer, as the original does on the
        // fullscreen VGA surface; two arrows would fight.
        window.set_cursor_visible(false);

        // The picture only ever scales by a whole number, so a window that is
        // not an exact multiple of it is necessarily bigger than its contents
        // and shows a black margin. Asking for three times the game can come
        // back smaller — a screen 1440 points tall has no room for a 1440
        // point window plus its title bar — and then the picture drops a whole
        // step and floats in the middle. So the window is put back to the
        // largest multiple that fits what we were actually given.
        //
        // Once, here: doing it on every resize would snap the window back
        // while it was still being dragged.
        let got = window.inner_size();
        let scale = whole_scale(got.width, got.height);
        let _ =
            window.request_inner_size(winit::dpi::PhysicalSize::new(WIDTH * scale, HEIGHT * scale));

        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                match &event.logical_key {
                    // Escape belongs to the game, not to the window. `ICTRL`
                    // compares `_AKTKEY` against 27 and opens the quit page on
                    // it (module 4, `0x02c40`: `_AKTKEY @ _PutLit 27 =`), which
                    // is how the original is left — through its own confirmation
                    // page, `QUITANIM`, and `ENDGAME`. Exiting the event loop
                    // here instead skipped all three, and there was no way to
                    // reach the quit page at all.
                    Key::Named(NamedKey::Escape) => self.key = ESCAPE,
                    // F12 freezes the picture and writes it out indexed — the
                    // same bytes the engine composed, without the window's
                    // scaling and without a screen capture's color profile.
                    // Comparing against the original has to happen on palette
                    // indices; on colors, a capture is off by one in every
                    // channel and buries a real shift in noise.
                    //
                    // Frozen because the scene keeps running: two shots a few
                    // frames apart show a different line of dialogue or a
                    // different step of an animation, which reads as a
                    // displacement and is none.
                    Key::Named(NamedKey::F12) => {
                        self.paused = !self.paused;
                        let frame = self.game.render();
                        let path = self.shot.as_path();
                        // The data directory need not exist yet: a player who
                        // has never saved has never caused it to be made.
                        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        match frame.write_png(path, self.game.palette()) {
                            Ok(()) => eprintln!(
                                "wrote {} ({}), picture {}",
                                path.display(),
                                if self.paused { "held" } else { "running again" },
                                self.frames
                            ),
                            Err(e) => eprintln!("could not write {}: {e}", path.display()),
                        }
                    }
                    Key::Named(NamedKey::Enter) => self.key = RETURN,
                    Key::Named(NamedKey::Space) => self.key = SPACE,
                    Key::Named(NamedKey::Backspace) => self.key = BACKSPACE,
                    Key::Character(c) => self.key = c.chars().next().map(|c| c as i32).unwrap_or(0),
                    _ => {}
                }
            }
            // The window is scaled by a whole number and centered, so the
            // pointer has to be mapped back the same way `blit` maps the
            // picture out — otherwise the game is told about a position that
            // is not where the player is looking.
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(w) = &self.window {
                    let size = w.inner_size();
                    let scale = whole_scale(size.width, size.height);
                    let (ox, oy) = (
                        (size.width.saturating_sub(WIDTH * scale)) / 2,
                        (size.height.saturating_sub(HEIGHT * scale)) / 2,
                    );
                    self.cursor = (
                        (position.x as i32 - ox as i32) / scale as i32,
                        (position.y as i32 - oy as i32) / scale as i32,
                    );
                    // The arrow is the engine's, so it only moves when a new
                    // picture is drawn — and waiting for the next frame put up
                    // to 40 ms between the hand and the pointer, which reads
                    // as lag even when nothing is slow. So the position goes
                    // to the game at once and a repaint is asked for. Safe
                    // between steps: the interpreter reads input only inside
                    // `step`, and `tick` writes all of this again immediately
                    // before it. The pending click and key ride along so this
                    // cannot clobber an edge the game has not seen yet. winit
                    // coalesces the requests, so a fast hand costs at most the
                    // display's own rate in repaints.
                    let (mx, my) = self.cursor;
                    if let Err(e) =
                        self.game
                            .set_input(mx, my, self.click, self.right_click, self.key)
                    {
                        eprintln!("input: {e}");
                    }
                    w.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    match button {
                        winit::event::MouseButton::Left => self.click = true,
                        winit::event::MouseButton::Right => self.right_click = true,
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame {
            // Catching up frame by frame would make a stall turn into
            // fast-forward, so a late frame just resets the clock.
            let spent = self.tick(event_loop);
            self.next_frame = now + spent;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }
}

impl App {
    /// One picture: hand the game its input, let it step — sometimes more than
    /// once — ask for a repaint, and say how long what was stepped should last
    /// on screen.
    ///
    /// More than once, because a step is not always a frame. During a curtain
    /// the game's clock runs a band at a time, down to 5 ms on the title
    /// screen, and presenting each band separately meant two hundred pictures
    /// a second — which no display shows and which backed the whole event loop
    /// up behind the presents. So steps are gathered until they are worth at
    /// least [`MIN_PRESENT`] and shown together; the sum is returned and
    /// becomes the wait, so the fade's wall-clock pace is unchanged by the
    /// batching. Outside a curtain a step lasts 40 ms and the loop runs once,
    /// so the batching costs nothing there.
    fn tick(&mut self, event_loop: &ActiveEventLoop) -> Duration {
        let (mx, my) = self.cursor;
        if let Err(e) = self
            .game
            .set_input(mx, my, self.click, self.right_click, self.key)
        {
            eprintln!("input: {e}");
        }
        self.click = false;
        self.right_click = false;
        self.key = 0;

        // Held by F12: the picture stays put so it can be compared against the
        // original at the same moment. Input still reaches the game, so a click
        // releases nothing by accident.
        if self.paused {
            self.draw();
            return self.game.frame_duration().unwrap_or(FRAME);
        }
        let mut spent = Duration::ZERO;
        // The bound is a guard, not a tuning knob: no shipped band is shorter
        // than 5 ms, so two steps always reach `MIN_PRESENT`.
        for _ in 0..8 {
            // Asked before the step, because this is the duration *of* the
            // step about to run — afterwards the clock already belongs to the
            // next one.
            let lasts = self.game.frame_duration().unwrap_or(FRAME);
            self.frames += 1;
            if let Err(e) = self.game.step() {
                // A missing word stops the game rather than limping on, on
                // purpose: a silently skipped word is indistinguishable from a
                // working one until something far downstream goes wrong.
                eprintln!("the game stopped: {e}");
                event_loop.exit();
                return FRAME;
            }
            // The game ended itself: `QUITANIM` stopped the main loop and
            // `START` ran on through `ENDGAME`. That is the original's own way
            // out, and it is a different thing from the window being closed.
            if self.game.finished() {
                event_loop.exit();
                return FRAME;
            }
            spent += lasts;
            if spent >= MIN_PRESENT {
                break;
            }
            // A further step in the same picture gets what its own frame would
            // have given it: the pointer where it is, the edge-triggered click
            // and key already delivered above and cleared.
            if let Err(e) = self.game.set_input(mx, my, false, false, 0) {
                eprintln!("input: {e}");
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        spent
    }
}

impl App {
    fn draw(&mut self) {
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut out) = surface.buffer_mut() else {
            return;
        };

        let started = self.perf.as_ref().map(|_| Instant::now());
        let frame = self.game.render();
        let palette = self.game.palette();
        // 256 lookups instead of one per pixel: at 307200 pixels a frame that
        // difference is the whole cost of presenting. Rebuilt only when the
        // palette itself has moved — the compare is 768 bytes, and the frames
        // that do move it, the fades, are exactly the ones with no time to
        // spare.
        if self.lut_palette.as_ref() != Some(palette) {
            for (i, color) in self.colors.iter_mut().enumerate() {
                let [r, g, b] = palette.rgb8(i as u8);
                *color = (r as u32) << 16 | (g as u32) << 8 | b as u32;
            }
            self.lut_palette = Some(palette.clone());
        }
        let rendered = started.map(|_| Instant::now());

        blit(&frame, &self.colors, &mut out, size.width, size.height);
        let _ = out.present();

        if let (Some(perf), Some(t0), Some(t1)) = (self.perf.as_mut(), started, rendered) {
            let scale = whole_scale(size.width, size.height);
            perf.note(t1 - t0, t1.elapsed(), size.width, size.height, scale);
        }
    }
}

/// How many window pixels one game pixel gets: a whole number, never zero.
///
/// The one definition, because the picture and the pointer have to agree. Two
/// copies of this arithmetic is two places to drift apart in, and a pointer
/// that disagrees with the picture by one scale step lands every click in the
/// wrong place.
fn whole_scale(width: u32, height: u32) -> u32 {
    (width / WIDTH).min(height / HEIGHT).max(1)
}

/// Draws the frame into the window buffer, scaled by a whole number and centered.
///
/// This walks every physical window pixel — on a Retina display five million
/// of them, sixteen times the game's own 307 200 — so it is the one loop in
/// the frontend where the shape of the code is the cost. So no arithmetic runs
/// per destination pixel: every source row is expanded through the palette
/// once and then repeated with `copy_within`, which is a straight memmove, and
/// there is no whole-buffer clear. Dividing each destination coordinate back to
/// its source instead is the obvious shape and costs a division per pixel.
///
/// Every pixel of `out` is still written every call — the picture over its
/// rectangle, the margins by the strip fills. That is a promise, not a
/// leftover: softbuffer only hands out a freshly zeroed buffer on some
/// platforms; on others it persists with whatever it held, and a pixel left
/// unwritten shows it.
fn blit(frame: &Framebuffer, colors: &[u32; 256], out: &mut [u32], width: u32, height: u32) {
    let scale = whole_scale(width, height);
    let (dw, dh) = (frame.width as u32 * scale, frame.height as u32 * scale);
    // Left-over space is split evenly; an odd remainder leaves the extra pixel
    // on the right and bottom, which is invisible and keeps the arithmetic in
    // integers.
    let (ox, oy) = (
        (width.saturating_sub(dw)) / 2,
        (height.saturating_sub(dh)) / 2,
    );
    // How much of the picture the window has room for: all of it, unless the
    // window is smaller than 640x480 — then the scale is already pinned at 1
    // and the picture is cut off at the right and bottom.
    let rows = dh.min(height.saturating_sub(oy)) as usize;
    let cols = dw.min(width.saturating_sub(ox)) as usize;
    let (width, ox, oy, scale) = (width as usize, ox as usize, oy as usize, scale as usize);

    // The margins. Top and bottom are contiguous runs; the side strips only
    // exist when the width is not an exact multiple, and the loop is skipped
    // entirely when they are empty.
    out[..oy * width].fill(0);
    out[(oy + rows) * width..].fill(0);
    if ox > 0 || ox + cols < width {
        for y in oy..oy + rows {
            out[y * width..y * width + ox].fill(0);
            out[y * width + ox + cols..(y + 1) * width].fill(0);
        }
    }

    for sy in 0..rows.div_ceil(scale) {
        let y0 = sy * scale;
        let base = (oy + y0) * width + ox;
        let src = &frame.pixels[sy * frame.width as usize..];
        // The row, expanded once: each source pixel becomes `scale` copies of
        // its color.
        let dst = &mut out[base..base + cols];
        for (chunk, &index) in dst.chunks_exact_mut(scale).zip(src) {
            chunk.fill(colors[index as usize]);
        }
        // A row cut off mid-pixel. `whole_scale` cannot actually produce one —
        // a scale above 1 means the window fits the whole width, and at 1 every
        // chunk is a pixel — but the promise above is that every pixel of
        // `out` gets written, and that must not hang on that arithmetic.
        let rem = cols % scale;
        if rem > 0 {
            dst[cols - rem..].fill(colors[src[cols / scale] as usize]);
        }
        // And repeated: the other window rows this source row covers are
        // copies of the one just written.
        for r in 1..scale.min(rows - y0) {
            out.copy_within(base..base + cols, base + r * width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, String> {
        parse_args(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    /// A flag's operand must not fall through to the positional.
    ///
    /// Taken as "the first argument not starting with `--`", the `5` of
    /// `--loc 5` becomes the game directory and the game looks for `001.RSC`
    /// inside a directory called `5`. The same goes for `--saves`.
    #[test]
    fn a_flags_value_is_not_the_game_directory() {
        let o = parse(&["--loc", "5"]).unwrap();
        assert_eq!(o.dir, PathBuf::from("../gamedata"), "the default survives");
        assert_eq!(o.wanted, Some(5));

        let o = parse(&["--saves", "mysaves"]).unwrap();
        assert_eq!(o.dir, PathBuf::from("../gamedata"));
        assert_eq!(o.saves, PathBuf::from("mysaves"));
    }

    #[test]
    fn the_directory_can_come_before_or_after_the_flags() {
        for args in [
            vec!["/games/ds2", "--loc", "5"],
            vec!["--loc", "5", "/games/ds2"],
            vec!["--no-sound", "/games/ds2", "--loc", "5"],
        ] {
            let o = parse(&args).unwrap();
            assert_eq!(o.dir, PathBuf::from("/games/ds2"), "{args:?}");
            assert_eq!(o.wanted, Some(5), "{args:?}");
        }
    }

    #[test]
    fn defaults_are_what_the_readme_says() {
        let o = parse(&[]).unwrap();
        assert_eq!(o.dir, PathBuf::from("../gamedata"));
        assert_eq!(o.wanted, None);
        assert!(!o.quiet);
        assert!(o.saves.ends_with("saves"), "{}", o.saves.display());
        assert!(o.shot.ends_with("shot.png"), "{}", o.shot.display());
    }

    /// The point of the change that moved these: what the program writes must
    /// not land in the working directory, because for anyone building from a
    /// checkout that directory is the source tree.
    ///
    /// Asserted as "absolute", not as a literal path, because the answer is the
    /// platform's and this suite runs on more than one. The one environment
    /// that legitimately has no answer — no `HOME` at all — is the documented
    /// fallback, and there the old relative default is the right behavior.
    #[test]
    fn what_the_program_writes_does_not_land_in_the_working_directory() {
        if data_dir().is_none() {
            eprintln!("skipping: the environment names no home directory");
            return;
        }
        let o = parse(&[]).unwrap();
        for p in [&o.saves, &o.shot] {
            assert!(p.is_absolute(), "{} is relative", p.display());
            assert!(
                p.starts_with(data_dir().unwrap()),
                "{} is not under the data directory",
                p.display()
            );
        }
    }

    #[test]
    fn both_written_paths_can_be_overridden() {
        let o = parse(&["--saves", "/tmp/s", "--shot", "/tmp/p.png"]).unwrap();
        assert_eq!(o.saves, PathBuf::from("/tmp/s"));
        assert_eq!(o.shot, PathBuf::from("/tmp/p.png"));
    }

    #[test]
    fn a_flag_without_its_value_is_refused() {
        assert!(parse(&["--loc"]).is_err());
        assert!(parse(&["--saves"]).is_err());
        assert!(parse(&["--shot"]).is_err());
        // And a value that is not a number says so rather than being dropped.
        let e = parse(&["--loc", "seven"]).unwrap_err();
        assert!(e.contains("wants a number"), "{e}");
    }

    #[test]
    fn an_unknown_option_is_refused_rather_than_ignored() {
        let e = parse(&["--sound"]).unwrap_err();
        assert!(e.contains("unknown option --sound"), "{e}");
    }

    #[test]
    fn two_game_directories_are_refused() {
        assert!(parse(&["/one", "/two"]).is_err());
    }

    #[test]
    fn help_is_recognized_both_ways() {
        assert!(parse(&["-h"]).unwrap().help);
        assert!(parse(&["--help"]).unwrap().help);
    }

    /// The blit as it was first written: one destination pixel at a time, a
    /// divide per coordinate, a full clear up front. Sixteen times slower than
    /// [`blit`] and obviously right, which is exactly what an oracle is for.
    fn blit_reference(
        frame: &Framebuffer,
        colors: &[u32; 256],
        out: &mut [u32],
        width: u32,
        height: u32,
    ) {
        out.fill(0);
        let scale = whole_scale(width, height);
        let (dw, dh) = (frame.width as u32 * scale, frame.height as u32 * scale);
        let (ox, oy) = (
            (width.saturating_sub(dw)) / 2,
            (height.saturating_sub(dh)) / 2,
        );
        for y in 0..dh.min(height.saturating_sub(oy)) {
            let src_row = (y / scale) as usize * frame.width as usize;
            let dst_row = (oy + y) as usize * width as usize + ox as usize;
            for x in 0..dw.min(width.saturating_sub(ox)) {
                let index = frame.pixels[src_row + (x / scale) as usize];
                out[dst_row + x as usize] = colors[index as usize];
            }
        }
    }

    #[test]
    fn the_fast_blit_agrees_with_the_slow_one() {
        // A frame with structure in it: every pixel its own mix of position,
        // so a swapped row or a column off by one cannot cancel out.
        let mut frame = Framebuffer::new(WIDTH as u16, HEIGHT as u16);
        for (i, p) in frame.pixels.iter_mut().enumerate() {
            *p = (i * 7 % 251) as u8;
        }
        let mut colors = [0u32; 256];
        for (i, c) in colors.iter_mut().enumerate() {
            *c = (i as u32) * 0x0101 + 3;
        }
        for (w, h) in [
            (WIDTH * 3, HEIGHT * 3),         // an exact multiple, no margins
            (WIDTH * 3 + 9, HEIGHT * 3 + 5), // odd margins on every side
            (2560, 1920),                    // a Retina window, scale 4
            (WIDTH, HEIGHT),                 // scale 1, exact
            (700, 500),                      // scale 1 with margins
            (500, 400),                      // smaller than the picture: clipped
            (700, 300),                      // clipped in one direction only
            (639, 481),                      // one pixel short, one over
            (1, 1),                          // degenerate
        ] {
            // Prefilled with a color neither blit writes, so a pixel either
            // of them missed cannot pass as agreement.
            let mut fast = vec![0xdead_beefu32; (w * h) as usize];
            let mut slow = vec![0xdead_beefu32; (w * h) as usize];
            blit(&frame, &colors, &mut fast, w, h);
            blit_reference(&frame, &colors, &mut slow, w, h);
            assert_eq!(fast, slow, "{w}x{h}");
        }
    }
}
