//! The window and the loop that drives it: from a game and a set of options
//! to pictures on a screen.
//!
//! [`run`] builds the event loop and hands it an [`App`]; everything after
//! that is winit's callbacks. The frame clock, the present rate, the palette
//! lookup and the scaling all live in here, because they are one subject —
//! how the game's 25 pictures a second reach a physical display — and none
//! of them mean anything before a window exists.

use super::cli::{Options, choose_game, parse_args, usage};
use super::scale::{base_pair, blit, opening_pair, scale_pair};
use super::{data_path, fatal, keys, note, roster, sound};
use std::path::PathBuf;

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use motionvm_playable::{Button, Playable};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

pub(crate) fn run() -> Result<(), motionvm_playable::Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Options {
        dir,
        wanted,
        quiet,
        help,
    } = parse_args(&args)?;
    if help {
        print!("{}", usage());
        return Ok(());
    }

    // A path from the command line is taken at its word: a wrong one fails on
    // stderr, the way a script wants. Only when nothing was typed does the
    // program ask — and then the answer, and any complaint about it, goes
    // through a window, because whoever double-clicked the binary has no
    // stderr to read.
    // Once the game is open the window forgets the directory: everything
    // that still needs it — the music's driver files — lives behind the
    // contract.
    let mut game = match dir {
        Some(dir) => {
            let Some(family) = roster::find(&dir) else {
                return Err(roster::nobodys(&dir).into());
            };
            family.open(&dir)?
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
    // Before `start()`, because startup already draws: whatever the game
    // draws random numbers from is seeded from the clock, so two launches are
    // two runs. The engines are built with a fixed seed and stay on it unless
    // somebody says otherwise — which is what lets a test render a scene twice
    // and compare — so this call is the whole difference between a player's
    // run and a checked one, and it belongs on the platform's side because
    // only the platform knows which of the two this is.
    //
    // Nanoseconds since the epoch, whose low bits are what an engine narrowing
    // this to its own state width will keep — and they turn over every few
    // seconds, so two launches are two seeds. A clock set before 1970 is a
    // machine whose time is wrong and seeds zero, which is a seed like any
    // other.
    game.seed(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX)),
    );

    // Before `start()`: the very first location is entered during startup and
    // the game's first song has to find its way out already open. How music
    // happens is the game's own affair behind the contract: the window opens
    // the device, hands the game the device's rate, and takes back a source
    // for the audio thread — or nothing, for a game with nothing to play.
    let audio = if quiet {
        None
    } else {
        let opened = sound::output().and_then(|out| {
            let Some(source) = game.open_music(out.rate()).map_err(|e| e.to_string())? else {
                // A game with nothing to play: silence by design, no line.
                return Ok(None);
            };
            Ok(Some(sound::spawn(&out, source)?))
        });
        match opened {
            Ok(stream) => stream,
            // Silence is not a reason to stop: the game is playable without it.
            Err(e) => {
                note(&format!("sound is off: {e}"));
                None
            }
        }
    };
    // Where the program writes — savegames and the F12 picture — is the
    // platform data directory, and nothing on the command line moves it. The
    // originals keep their save slots beside their own binaries, among the
    // shipped data; that is not a place to write to here, because the game
    // directory may well be a read-only copy of the discs, which is also why
    // a game refuses a save directory inside it. And a flag that points the
    // slots elsewhere is mostly a way to point them at something that is not a
    // save directory; the one place they belong is the one `data_dir` names.
    // Every game gets a subdirectory of `saves/` named for it, and none is
    // the special case. The game puts the name on, so what goes in is
    // `saves/` itself and what comes back out is where the slots really are.
    let saves = data_path("saves");
    let shot = data_path("shot.png");
    if let Err(e) = game.set_saves(&saves) {
        // Not fatal: the game runs, the slot row simply stays empty and a click
        // on one stops by name rather than writing somewhere it should not.
        note(&format!("savegames are off: {e}"));
    } else if let Some(saves) = game.saves() {
        // Printed because the default is no longer somewhere the player is
        // standing. A directory they cannot find is a directory they will
        // think is empty.
        note(&format!("savegames in {}", saves.display()));
    }
    // `start()` returns with the game parked in its native frame loop; from
    // that point every frame belongs to the game, and the game is what
    // enters its first location.
    game.start()?;

    // `--loc` is for looking at some other location without playing to it.
    // It is asked for after the run above because startup assigns the start
    // location itself, and the request goes through the game's own mechanism
    // rather than around it — the game enters what was requested once no
    // location is active.
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
        size: (u32::from(size.width), u32::from(size.height)),
        aspect: game.pixel_aspect(),
        game,
        window: None,
        surface: None,
        next_frame: Instant::now(),
        mods: ModifiersState::empty(),
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

    // Once, on the way out. A run that read through a stray pointer or walked
    // past a word that does nothing has been keeping count all along, and this
    // is the only place anybody asks — the engine's ledger says these totals
    // are named at the end of a run, and this is where that becomes true.
    //
    // On the way out rather than as it happens, because most of it is a total
    // rather than an event and a line per occurrence would bury the game's own
    // output in a run that touches one address ten thousand times.
    for line in app.game.diagnostics() {
        note(&line.to_string());
    }
    Ok(())
}

/// A pointer position as the platform reports it, in whole window pixels.
#[expect(
    clippy::as_conversions,
    reason = "a position in window pixels, truncated toward zero the way a whole-pixel report is"
)]
fn pixel(position: f64) -> i32 {
    position as i32
}

/// A window dimension or scale as a coordinate. A window wider than
/// `i32::MAX` pixels does not exist, and saturating is what keeps the
/// arithmetic finite if a platform ever claims one.
fn coord(n: u32) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// A scroll offset in pixels, folded to lines at sixteen to the line — the
/// contract carries lines as `f32`.
#[expect(
    clippy::as_conversions,
    reason = "a pixel count folded to lines; precision past `f32` is nothing a game reads"
)]
fn lines(pixels: f64) -> f32 {
    (pixels / 16.0) as f32
}

/// The pace to fall back on for a frame the game wants no wait after.
///
/// `frame_duration` answering `None` is the game's "do not wait at all", and
/// free-running the loop on it would burn a core for pictures no display
/// shows. Every game this tree ships asks for a real duration from its first
/// frame on, so the value only matters as a guard — and it matches the pace
/// those games ask for anyway, which keeps the guard invisible if it is ever
/// reached.
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
    /// The game's picture size, which the window is a whole multiple of,
    /// axis by axis.
    size: (u32, u32),
    /// The shape of one game pixel on the original's monitor, height:width —
    /// [`Playable::pixel_aspect`]. Everything that maps between window and
    /// game — [`blit`], the pointer, the opening size — scales its two axes
    /// through this.
    aspect: motionvm_playable::PixelAspect,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    next_frame: Instant,
    /// Which modifiers are down, kept from winit's own modifiers event and
    /// handed to the game on each press inside its [`motionvm_playable::KeyPress`].
    mods: ModifiersState,
    /// The pointer in game coordinates, not window ones.
    cursor: (i32, i32),
    /// Whether the frame clock is held, so a shot can be compared at leisure.
    paused: bool,
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
    lut_palette: Option<motionvm_playable::Palette>,
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
            .with_title(self.game.name())
            .with_inner_size(winit::dpi::LogicalSize::new(width * bx, height * by));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                fatal(&format!("cannot open a window: {e}"));
                event_loop.exit();
                return;
            }
        };
        match softbuffer::Context::new(window.clone())
            .and_then(|ctx| softbuffer::Surface::new(&ctx, window.clone()))
        {
            Ok(surface) => self.surface = Some(surface),
            Err(e) => {
                fatal(&format!("cannot get a drawing surface: {e}"));
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
            // Which modifiers are down arrives on its own event, not on the
            // keystroke, so the current state is kept and rides along on each
            // press.
            WindowEvent::ModifiersChanged(mods) => {
                self.mods = mods.state();
                let m = self.mods;
                self.game
                    .modifiers(m.shift_key(), m.control_key(), m.alt_key());
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Releases cross too, like the mouse's: what a key means —
                // and whether its release means anything — is the game's own
                // reading. The window's two keys below act on presses only.
                let down = event.state == ElementState::Pressed;
                // Alt+Enter is the window's too: borderless fullscreen, on and
                // off. Borderless, so the display keeps its mode; the picture
                // takes the largest whole multiple that fits and black fills
                // the rest, which is what [`blit`] does with any window size,
                // and winit puts the window back to its previous size on the
                // way out. On macOS this is the same native fullscreen the
                // green button gives.
                //
                // Keeping the key costs the games nothing — none of them
                // dispatches on Alt+Enter, which their own keyboard module
                // documents. Not on repeat: a held key would flip in and out
                // of fullscreen for as long as it is down.
                if down
                    && self.mods.alt_key()
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
                // It is the one key the window keeps for itself, and it
                // costs the games nothing: their keyboard module documents
                // that none of them reads F12.
                if down && event.physical_key == PhysicalKey::Code(KeyCode::F12) {
                    self.paused = !self.paused;
                    let path = self.shot.as_path();
                    // The data directory need not exist yet: a player who has
                    // never saved has never caused it to be made.
                    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let frame = self.game.frame();
                    match frame.pixels.write_png(path, frame.palette) {
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
                // Everything else goes through to the game, Escape included.
                // Escape belongs to the game, not to the window: these games
                // open their own quit confirmation on it, which is how their
                // originals are left. Exiting the event loop here instead
                // would skip that page entirely.
                //
                // Repeats are kept. The original's keyboard buffer fills from
                // the keyboard's own typematic repeat too, which is what lets a
                // held cursor key walk down a list. What the press means, and
                // whether the buffer still has room for it, are the game's own
                // affairs behind the contract.
                let press = keys::press(event.physical_key, &event.logical_key, self.mods);
                if down {
                    self.game.key_down(&press);
                } else {
                    self.game.key_up(&press);
                }
                // The typed stream rides beside the keystroke: the whole
                // string the layout and input method produced, where the
                // press carries only its first character for the translator.
                if down && let Some(text) = &event.text {
                    self.game.text(text);
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
                        ((pixel(position.x) - coord(ox)) / coord(sx))
                            .clamp(0, coord(self.size.0) - 1),
                        ((pixel(position.y) - coord(oy)) / coord(sy))
                            .clamp(0, coord(self.size.1) - 1),
                    );
                    // The arrow is the engine's, so it only moves when a new
                    // picture is drawn — and waiting for the next frame put up
                    // to 40 ms between the hand and the pointer, which reads
                    // as lag even when nothing is slow. So the position goes
                    // to the game at once and a repaint is asked for; buttons
                    // and keystrokes are untouched — both wait in the game's
                    // own latches for the step that spends them. winit
                    // coalesces the requests, so a fast hand costs at most
                    // the display's own rate in repaints.
                    let (mx, my) = self.cursor;
                    self.game.pointer(mx, my);
                    w.request_redraw();
                }
            }
            // Both transitions cross, releases included: what a press means
            // — and how long it lasts — is the game's own reading, made at
            // its own pace.
            WindowEvent::MouseInput { state, button, .. } => {
                let down = state == ElementState::Pressed;
                match button {
                    winit::event::MouseButton::Left => self.game.button(Button::Left, down),
                    winit::event::MouseButton::Right => self.game.button(Button::Right, down),
                    _ => {}
                }
            }
            // The wheel, in lines. A pixel-scrolling device is folded at
            // sixteen pixels to the line — a middling choice with nothing to
            // measure it against, which a game that cares can rescale.
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (x, y),
                    winit::event::MouseScrollDelta::PixelDelta(p) => (lines(p.x), lines(p.y)),
                };
                self.game.wheel(dx, dy);
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
        // No input is touched here: keystrokes and presses wait in the
        // game's own latches, and each step takes its own — so held by F12,
        // when nothing steps, both keep what was struck for the picture that
        // runs next.

        // Held by F12: the picture stays put so it can be compared against
        // the original at the same moment.
        if self.paused {
            self.draw();
            return self.game.frame_duration().unwrap_or(FRAME);
        }
        let mut spent = Duration::ZERO;
        let began = Instant::now();
        // Two bounds, and they answer different questions.
        //
        // `spent` is the game's own clock and decides when a picture is worth
        // showing — that is the batching, and it is checked at the bottom of
        // the loop.
        //
        // `STEPS` and `BUDGET` are the guards, and neither is a tuning knob.
        // A bare step count would be a fade's band count in disguise — no
        // shipped band is shorter than 5 ms, so two steps always reach
        // `MIN_PRESENT`, and any small multiple of that is a number belonging
        // to a family the window cannot name. A count on its own is also the
        // wrong guard: a step that takes a long time in *wall-clock* — a
        // location loading, a savegame written — would still run several of
        // itself before the window drew anything or read an event. So the
        // loop stops on either, and what it protects is the event loop's
        // responsiveness rather than any property of a fade.
        const STEPS: u32 = 64;
        const BUDGET: Duration = Duration::from_millis(20);
        for _ in 0..STEPS {
            // Asked before the step, because this is the duration *of* the
            // step about to run — afterwards the clock already belongs to the
            // next one.
            let lasts = self.game.frame_duration().unwrap_or(FRAME);
            self.frames += 1;
            if let Err(e) = self.game.step() {
                // A missing word stops the game rather than limping on, on
                // purpose: a silently skipped word is indistinguishable from a
                // working one until something far downstream goes wrong.
                fatal(&format!("the game stopped: {e}"));
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
            if spent >= MIN_PRESENT || began.elapsed() >= BUDGET {
                break;
            }
            // A further step in the same picture is a round of the game's
            // loop like any other: the keystroke and the button pulse it
            // gets, it takes itself.
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
        let frame = self.game.frame();
        let (pixels, palette) = (frame.pixels, frame.palette);
        // 256 lookups instead of one per pixel: at 307200 pixels a frame that
        // difference is the whole cost of presenting. Rebuilt only when the
        // palette itself has moved — the compare is 768 bytes, and the frames
        // that do move it, the fades, are exactly the ones with no time to
        // spare.
        if self.lut_palette.as_ref() != Some(palette) {
            for (i, color) in (0u8..=u8::MAX).zip(self.colors.iter_mut()) {
                let [r, g, b] = palette.rgb8(i);
                *color = (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
            }
            self.lut_palette = Some(palette.clone());
        }
        let rendered = started.map(|_| Instant::now());

        blit(
            pixels,
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
