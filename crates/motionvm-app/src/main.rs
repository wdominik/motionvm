//! A window for the engine.
//!
//! The frontend is deliberately thin. The engine renders into an indexed
//! framebuffer and this does two things with it: expand it through the
//! current palette, and put it on screen at an integer scale.
//!
//! That framebuffer has been checked pixel for pixel against the original
//! engine's own output: the title screen, location 23, all 307 200 pixels of
//! it over 206 distinct indices, every index mapping to one color and every
//! color back to one index. **One frame** — the claim is worth exactly that:
//! one scene, drawn through scaling, layering, the palette and the composition
//! rule, agreeing completely.
//!
//! Integer scaling is not a preference. The games are hand-drawn pixel art;
//! any other factor resamples it and invents colors that were never in the
//! palette. So the picture is scaled by whole numbers and centered in
//! whatever space is left, with black around it. The two axes carry their own
//! whole number, because the pixels themselves were not square everywhere:
//! the 16-bit games' 320×200 filled a 4:3 monitor, each pixel 6/5 as tall as
//! wide ([`Playable::pixel_aspect`]), so their picture is drawn in
//! sx×sy blocks with sy/sx as close to 6/5 as whole numbers allow — exact at
//! ×5/×6 and its multiples. Dunkle Schatten 2's 640×480 is square-pixel 4:3
//! and keeps sx = sy.

// On Windows the release build is a windowed program, not a console one, so a
// double-clicked `motionvm.exe` opens the game and not a black console behind
// it. The price is that everything written to stderr — `savegames in …`,
// `sound is off`, `the game stopped`, and `--help` — goes nowhere there; what
// a double-clicking player has to see reaches them through the folder dialog
// and its message box instead. Debug builds keep the console, so a Windows
// developer still sees the messages. The attribute means nothing anywhere else.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use motionvm_engine::{Playable, Title, titles};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

/// How many keystrokes wait for the game.
///
/// The original does not hold one key, it reads a queue: `0x83e9c` is INT 16h
/// AH=00, which takes the oldest keystroke out of the BIOS buffer, and that
/// buffer holds fifteen. A single slot loses the second of two presses inside
/// one 40 ms frame — visible when paging through the mailbox quickly, and in
/// the debug input line at module 4 `0x051a0`, which takes one character per
/// frame. A full buffer drops what arrives, as the BIOS does.
const KEYS: usize = 15;

mod keys;
mod scale;
mod sound;

use scale::{base_pair, blit, opening_pair, scale_pair};

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
/// **Not the working directory.** A relative default would land these
/// wherever the program happens to be started — for anyone building from a
/// checkout, in the source tree. Runtime output does
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

/// Where a file the program writes goes: under [`data_dir`] when there is one,
/// otherwise the plain relative name.
fn data_path(name: &str) -> PathBuf {
    data_dir().map_or_else(|| PathBuf::from(name), |d| d.join(name))
}

const USAGE: &str = "\
motionvm — the MOTION engine, for the games built with it

usage: motionvm [GAMEDIR] [options]

  GAMEDIR         the directory a game is installed in: 001.RSC and
                  ENGINE.EXE (Dunkle Schatten 2), DATA.-1- and ENVIRO.EXE
                  (Die Enviro-Kids greifen ein), DATA.-1-, DATA.-2- and
                  HPPLAY.EXE (Jeff Jet), or DATA.-1-,
                  DATA.-2- and BMZ.EXE (Hilfe für Amajambere), or DATA.-1-
                  and LL.EXE (Victor Loomes). Without one, a folder dialog
                  asks for it.

options:
  --loc N         start in location N: instead of the intro (Dunkle
                  Schatten 2), or right after it (the 16-bit games).
  --no-sound      do not open an audio device.
  -h, --help      this text.
";

/// Everything the command line can say.
#[derive(Debug)]
struct Options {
    /// The game directory, when the command line names one. `None` means
    /// nobody typed a path — a double-clicked binary, most often — and the
    /// folder dialog asks instead. There is deliberately no silent default: a
    /// relative `../gamedata` that happens to exist is a checkout's accident,
    /// not a player's choice, and one that does not exist fails with a message
    /// about a directory the player never named.
    dir: Option<PathBuf>,
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
        dir: None,
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
    opt.dir = positional.map(PathBuf::from);
    Ok(opt)
}

/// Asks for the game directory with the platform's own folder dialog, and
/// keeps asking while the answer is not one.
///
/// `None` when the dialog is dismissed — that is the player deciding not to
/// play, not an error — or when a wrong directory's complaint is answered with
/// Cancel. A wrong directory is reported where the player is looking: the
/// message [`titles::open`] writes for exactly this case, in a message box, with
/// OK opening the dialog again. `Game::open` checks for the required files
/// before it reads anything, so a wrong answer costs nothing and the loop is
/// cheap to go round.
///
/// Called before the event loop exists, on the main thread, which is where
/// rfd's synchronous dialogs belong in a program that has no window yet. On
/// Linux the dialog is the XDG desktop portal's, so nothing links at build
/// time — and a desktop with neither the portal service nor `zenity` answers
/// `None` here, the same as a dismissal, which is why the caller's message
/// says how to name the directory without the dialog.
fn choose_game() -> Option<(PathBuf, Box<dyn Playable>)> {
    loop {
        let dir = rfd::FileDialog::new()
            .set_title(
                "Choose the game directory (it holds 001.RSC and ENGINE.EXE, \
                 or DATA.-1- and the 16-bit player)",
            )
            .pick_folder()?;
        match titles::open(&dir) {
            Ok(game) => return Some((dir, game)),
            Err(e) => {
                let again = rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("motionvm")
                    .set_description(format!(
                        "{e}\n\nOK chooses another directory; Cancel quits."
                    ))
                    .set_buttons(rfd::MessageButtons::OkCancel)
                    .show();
                if !matches!(again, rfd::MessageDialogResult::Ok) {
                    return None;
                }
            }
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Options {
        dir,
        wanted,
        quiet,
        help,
    } = parse_args(&args)?;
    if help {
        print!("{USAGE}");
        return Ok(());
    }

    // A path from the command line is taken at its word: a wrong one fails on
    // stderr, the way a script wants. Only when nothing was typed does the
    // program ask — and then the answer, and any complaint about it, goes
    // through a window, because whoever double-clicked the binary has no
    // stderr to read.
    let (dir, mut game) = match dir {
        Some(dir) => {
            let game = titles::open(&dir)?;
            (dir, game)
        }
        None => match choose_game() {
            Some(chosen) => chosen,
            None => {
                eprintln!(
                    "no game directory chosen.\n\
                     If no dialog appeared, name the directory on the command line: \
                     motionvm GAMEDIR"
                );
                return Ok(());
            }
        },
    };
    // Before `start()`: the very first location is entered during startup and
    // its `STARTTUNE` has to find a sink already in place. Each generation
    // brings its own stack — Dunkle Schatten 2's HMI songs through the rebuilt
    // MIDI driver, the 16-bit games' PSM 2 tunes through the rebuilt
    // `MUSADL.DRV` sequencer, whose driver file is byte-identical in all three.
    let audio = if quiet {
        None
    } else {
        let opened = match game.title() {
            Title::DunkleSchatten2 => sound::open_motion32(&dir).map(|(stream, music)| {
                game.set_music(Box::new(music));
                stream
            }),
            Title::HilfeFuerAmajambere
            | Title::DieEnviroKidsGreifenEin
            | Title::JeffJet
            | Title::VictorLoomes => sound::open_motion16(&dir).map(|(stream, music)| {
                game.set_music(Box::new(music));
                stream
            }),
        };
        match opened {
            Ok(stream) => Some(stream),
            // Silence is not a reason to stop: the game is playable without it.
            Err(e) => {
                eprintln!("sound is off: {e}");
                None
            }
        }
    };
    // Where the program writes — savegames and the F12 picture — is the
    // platform data directory, and nothing on the command line moves it. The
    // original keeps its five save slots beside `ENGINE.EXE`, among the
    // shipped data; that is not a place to write to here, because the game
    // directory may well be a read-only copy of the discs, which is also why
    // the engine refuses a save directory inside it. And a flag that points the
    // slots elsewhere is mostly a way to point them at something that is not a
    // save directory; the one place they belong is the one `data_dir` names.
    // All five games name their slots alike — `701.blk`, `701.anm`, `701.FRZ`
    // and so on up to 705 — and each asks at start-up whether a slot exists,
    // so they cannot share a directory: one would find another's saves and
    // open its load page on them. The four 16-bit games would go further and
    // load one, because the savegame magic is the generation's and not the
    // game's. Each therefore gets a subdirectory of `saves/` named for it, and
    // none is the special case: `saves/ds2/`, `saves/enviro/`, `saves/jeffjet/`,
    // `saves/hfa/`, `saves/vloomes/`.
    let saves = data_path("saves").join(game.title().slug());
    let shot = data_path("shot.png");
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
        game.request_location(loc)?;
        // Said only when asked for: a plain start is not a diagnostic.
        eprintln!(
            "starting at location {}",
            game.start_location().unwrap_or(0)
        );
    }

    let event_loop = EventLoop::new()?;
    let size = game.display_size();
    let mut app = App {
        size: (size.0 as u32, size.1 as u32),
        aspect: game.pixel_aspect(),
        game,
        window: None,
        surface: None,
        next_frame: Instant::now(),
        click: false,
        right_click: false,
        pending: VecDeque::with_capacity(KEYS),
        mods: ModifiersState::empty(),
        cursor: (0, 0),
        paused: false,
        told_input_error: false,
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
    game: Box<dyn Playable>,
    /// The game's picture size — 640×480 for Dunkle Schatten 2, 320×200 for
    /// the two 16-bit games — which the window is a whole multiple of,
    /// axis by axis.
    size: (u32, u32),
    /// The shape of one game pixel on the original's monitor, height:width —
    /// [`Playable::pixel_aspect`]. Everything that maps between window and
    /// game — [`blit`], the pointer, the opening size — scales its two axes
    /// through this.
    aspect: (u32, u32),
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    next_frame: Instant,
    /// Set by a press, cleared once the game has seen it. The task manager
    /// advances a phase whenever a button is down, so a held button would race
    /// through the intro; one frame per press is what a click means here.
    click: bool,
    right_click: bool,
    /// The keystrokes the game has not taken yet, oldest first — the BIOS
    /// buffer [`KEYS`] stands for, drained one per frame the way `?KEY` drains
    /// it.
    pending: VecDeque<i32>,
    /// Which modifiers are down. `0x2379b` asks the BIOS separately
    /// (`0x83eb8`, AH=02) rather than reading them off the keystroke, and so
    /// does this: winit reports them in their own event.
    mods: ModifiersState,
    /// The pointer in game coordinates, not window ones.
    cursor: (i32, i32),
    /// Whether the frame clock is held, so a shot can be compared at leisure.
    paused: bool,
    /// Whether an input error has been reported. `set_input` runs every
    /// frame; a persisting failure said once is a report, said 25 times a
    /// second it is a torrent that buries the report.
    told_input_error: bool,
    /// How many frames have been stepped, printed with a shot so two captures
    /// can be shown to be the same moment.
    frames: u64,
    /// Where F12 writes: under the platform data directory, printed with every
    /// shot, because a path the player cannot see is one they cannot find.
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
    fn note(
        &mut self,
        render: Duration,
        blit: Duration,
        width: u32,
        height: u32,
        scale: (u32, u32),
    ) {
        self.presents += 1;
        self.render += render;
        self.blit += blit;
        self.blit_max = self.blit_max.max(blit);
        if self.since.elapsed() >= Duration::from_secs(1) {
            eprintln!(
                "perf: {}/s, render {:.1?} avg, blit+present {:.1?} avg, {:.1?} worst, {}x{} at {}x{}",
                self.presents,
                self.render / self.presents,
                self.blit / self.presents,
                self.blit_max,
                width,
                height,
                scale.0,
                scale.1,
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
        let (width, height) = self.size;
        let (bx, by) = base_pair(self.aspect);
        let attrs = Window::default_attributes()
            .with_title(self.game.title().name())
            .with_inner_size(winit::dpi::LogicalSize::new(width * bx, height * by));
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

        // The picture only ever scales by whole numbers, so a window that is
        // not an exact multiple of it is necessarily bigger than its contents
        // and shows a black margin. The logical size asked for can come back
        // as anything — a screen 1440 points tall has no room for a 1440
        // point window plus its title bar, and a HiDPI screen doubles it —
        // and then the picture drops a step and floats in the middle. So the
        // window is put back to a pair that fits what we were actually given:
        // the largest pair with the pixel aspect met *exactly* where one
        // fits (on a 2× screen the 960×800 request comes back 1920×1600
        // physical and lands on 1600×1200, five by six), the largest whole
        // pair otherwise. Only the opening snap prefers exactness — a hand
        // dragging the edge afterwards gets every whole step, and Alt+Enter
        // the largest that fits the screen.
        //
        // Once, here: doing it on every resize would snap the window back
        // while it was still being dragged.
        let got = window.inner_size();
        let (sx, sy) = opening_pair(self.size, self.aspect, got.width, got.height);
        let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(width * sx, height * sy));

        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            // Which modifiers are down arrives on its own, not on the
            // keystroke — the same split the original works with, where
            // `0x2379b` asks INT 16h AH=02 for the shift state after it has
            // taken the key.
            WindowEvent::ModifiersChanged(mods) => self.mods = mods.state(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                // Alt+Enter is the window's too: borderless fullscreen, on and
                // off. Borderless, so the display keeps its mode; the picture
                // takes the largest whole multiple that fits and black fills
                // the rest, which is what [`blit`] does with any window size,
                // and winit puts the window back to its previous size on the
                // way out. On macOS this is the same native fullscreen the
                // green button gives.
                //
                // Keeping the key costs the game nothing. The translator would
                // answer `0x91c` for it — Alt `0x800`, scan code `0x100`,
                // Enter's `0x1c` — and the keystroke dispatches read out of the
                // disassembly do not include it: module 216 tests 328, 336,
                // 331 and 333 at `0x0c21c`, module 4 tests 315, 316 and 323 at
                // `0x027a0` and `0x052e0`, and `ICTRL` compares against 27 at
                // `0x02c40`. Not on repeat: a held key would flip in and out
                // for as long as it is down.
                if self.mods.alt_key()
                    && !event.repeat
                    && matches!(
                        event.physical_key,
                        PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter)
                    )
                {
                    if let Some(w) = &self.window {
                        w.set_fullscreen(if w.fullscreen().is_some() {
                            None
                        } else {
                            Some(Fullscreen::Borderless(None))
                        });
                    }
                    return;
                }
                // F12 freezes the picture and writes it out indexed — the same
                // bytes the engine composed, without the window's scaling and
                // without a screen capture's color profile. Comparing against
                // the original has to happen on palette indices; on colors, a
                // capture is off by one in every channel and buries a real
                // shift in noise.
                //
                // Frozen because the scene keeps running: two shots a few
                // frames apart show a different line of dialogue or a different
                // step of an animation, which reads as a displacement and is
                // none.
                //
                // It is the one key the window keeps for itself, and it costs
                // the game nothing: the debug layer reads F9 (module 4,
                // `0x052e0`), never F12.
                if event.physical_key == PhysicalKey::Code(KeyCode::F12) {
                    self.paused = !self.paused;
                    let frame = self.game.render();
                    let path = self.shot.as_path();
                    // The data directory need not exist yet: a player who has
                    // never saved has never caused it to be made.
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
                    return;
                }
                // Everything else goes through the translator, Escape included.
                // Escape belongs to the game, not to the window: `ICTRL`
                // compares `_AKTKEY` against 27 and opens the quit page on it
                // (module 4, `0x02c40`: `_AKTKEY @ _PutLit 27 =`), which is how
                // the original is left — through its own confirmation page,
                // `QUITANIM`, and `ENDGAME`. Exiting the event loop here
                // instead skipped all three, and there was no way to reach the
                // quit page at all.
                //
                // Repeats are kept. The BIOS buffer fills from the keyboard's
                // own typematic repeat too, which is what lets a held cursor key
                // walk down the mailbox's list.
                if let Some(code) = keys::code(event.physical_key, &event.logical_key, self.mods)
                    && self.pending.len() < KEYS
                {
                    self.pending.push_back(code);
                }
            }
            // The window is scaled by a whole number and centered, so the
            // pointer has to be mapped back the same way `blit` maps the
            // picture out — otherwise the game is told about a position that
            // is not where the player is looking.
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(w) = &self.window {
                    let size = w.inner_size();
                    let (sx, sy) = scale_pair(self.size, self.aspect, size.width, size.height);
                    let (ox, oy) = (
                        (size.width.saturating_sub(self.size.0 * sx)) / 2,
                        (size.height.saturating_sub(self.size.1 * sy)) / 2,
                    );
                    // Clamped to the picture: the pointer the engine draws
                    // cannot leave it, and a hand in the black margin means
                    // the edge, not a place outside the screen.
                    self.cursor = (
                        ((position.x as i32 - ox as i32) / sx as i32)
                            .clamp(0, self.size.0 as i32 - 1),
                        ((position.y as i32 - oy as i32) / sy as i32)
                            .clamp(0, self.size.1 as i32 - 1),
                    );
                    // The arrow is the engine's, so it only moves when a new
                    // picture is drawn — and waiting for the next frame put up
                    // to 40 ms between the hand and the pointer, which reads
                    // as lag even when nothing is slow. So the position goes
                    // to the game at once and a repaint is asked for. Safe
                    // between steps: the interpreter reads input only inside
                    // `step`, and `tick` writes all of this again immediately
                    // before it. The pending click and key ride along so this
                    // cannot clobber an edge the game has not seen yet — the
                    // key is only *looked* at, because taking it here would
                    // spend a keystroke no step has run on. winit coalesces the
                    // requests, so a fast hand costs at most the display's own
                    // rate in repaints.
                    let (mx, my) = self.cursor;
                    let waiting = self.pending.front().copied().unwrap_or(0);
                    if let Err(e) =
                        self.game
                            .set_input(mx, my, self.click, self.right_click, waiting)
                    {
                        // Field-wise: `w` above still borrows the window.
                        if !self.told_input_error {
                            self.told_input_error = true;
                            eprintln!("input: {e}");
                        }
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
    /// Says what `set_input` refused — once. See [`App::told_input_error`].
    fn tell_input_error(&mut self, e: &dyn std::fmt::Display) {
        if !self.told_input_error {
            self.told_input_error = true;
            eprintln!("input: {e}");
        }
    }

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
        // One keystroke per step, because a step is one round of `ICTRL` and
        // `ICTRL` opens with a single `?KEY` (module 4, `0x022a0`) — which
        // takes one keystroke out of the buffer and no more. Held by F12
        // nothing steps, so nothing is taken: the buffer keeps what was struck
        // for the picture that runs next.
        let key = if self.paused {
            0
        } else {
            self.pending.pop_front().unwrap_or(0)
        };
        if let Err(e) = self
            .game
            .set_input(mx, my, self.click, self.right_click, key)
        {
            self.tell_input_error(&e);
        }
        self.click = false;
        self.right_click = false;

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
            // already delivered above, and the next keystroke waiting — it is a
            // round of `ICTRL` like any other, so it drains one.
            let key = self.pending.pop_front().unwrap_or(0);
            if let Err(e) = self.game.set_input(mx, my, false, false, key) {
                self.tell_input_error(&e);
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

        blit(
            &frame,
            &self.colors,
            &mut out,
            size.width,
            size.height,
            self.aspect,
        );
        let _ = out.present();

        if let (Some(perf), Some(t0), Some(t1)) = (self.perf.as_mut(), started, rendered) {
            let scale = scale_pair(self.size, self.aspect, size.width, size.height);
            perf.note(t1 - t0, t1.elapsed(), size.width, size.height, scale);
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
    /// `--loc 5` becomes the game directory, and a game gets looked for
    /// inside a directory called `5`.
    #[test]
    fn a_flags_value_is_not_the_game_directory() {
        let o = parse(&["--loc", "5"]).unwrap();
        assert_eq!(o.dir, None, "nothing was named, so nothing is taken");
        assert_eq!(o.wanted, Some(5));
    }

    #[test]
    fn the_directory_can_come_before_or_after_the_flags() {
        for args in [
            vec!["/games/ds2", "--loc", "5"],
            vec!["--loc", "5", "/games/ds2"],
            vec!["--no-sound", "/games/ds2", "--loc", "5"],
        ] {
            let o = parse(&args).unwrap();
            assert_eq!(o.dir, Some(PathBuf::from("/games/ds2")), "{args:?}");
            assert_eq!(o.wanted, Some(5), "{args:?}");
        }
    }

    #[test]
    fn defaults_are_what_the_readme_says() {
        let o = parse(&[]).unwrap();
        assert_eq!(o.dir, None, "no path means the dialog, not ../gamedata");
        assert_eq!(o.wanted, None);
        assert!(!o.quiet);
    }

    /// What the program writes must not land in the working directory, because
    /// for anyone building from a checkout that directory is the source tree —
    /// and nothing on the command line can send it there either.
    ///
    /// Asserted as "absolute", not as a literal path, because the answer is the
    /// platform's and this suite runs on more than one. The one environment
    /// that legitimately has no answer — no `HOME` at all — is the documented
    /// fallback, and there the relative name is the right behavior.
    #[test]
    fn what_the_program_writes_does_not_land_in_the_working_directory() {
        if data_dir().is_none() {
            eprintln!("skipping: the environment names no home directory");
            return;
        }
        for p in [data_path("saves"), data_path("shot.png")] {
            assert!(p.is_absolute(), "{} is relative", p.display());
            assert!(
                p.starts_with(data_dir().unwrap()),
                "{} is not under the data directory",
                p.display()
            );
        }
    }

    #[test]
    fn a_flag_without_its_value_is_refused() {
        assert!(parse(&["--loc"]).is_err());
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
}
