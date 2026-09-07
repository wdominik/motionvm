//! The game runtime behind the VM's [`motionvm_motion_forth::Host`] trait: screens, descriptors,
//! fonts and resources.
//!
//! Every word here has its arity taken from the original handler — the count
//! of argument fetches it makes — rather than from how call sites look. That
//! distinction matters: `NEWSETDESC` reads as a three-argument word everywhere
//! it is used and actually takes six, because its callers leave the first three
//! on the stack. An arity guessed from call sites produces nothing but stack
//! underflows.

mod buffer;
mod calendar;
mod clock;
mod curtain;
mod cycle;
mod descriptor;
mod dialogue16;
mod dialogue32;
mod display;
mod draw;
mod error;
mod game;
mod geometry;
mod keys;
mod layout;
mod menu;
mod order;
mod paint;
mod persist;
mod request;
mod resources;
mod sample;
mod save;
mod screen;
mod slide;
mod stack;
// Public because the metrics suites hold `text::text_width` and
// `text::backing_map` against measurements of the original; its twin
// `text16` has no such caller and stays private.
pub mod text;
mod text16;
pub mod titles;
mod walk;
mod words;
pub use display::{Display, Screen};
mod transitions;
mod video;
pub(crate) use transitions::Transitions;
mod sound;
pub(crate) use sound::Sound;
mod persistence;
pub(crate) use persistence::Persistence;
mod cursor;
pub(crate) use cursor::Cursor;
pub use cursor::PointerShape;
mod dialogue;
pub(crate) use dialogue::Dialogue;
mod scene;
pub(crate) use scene::Scene;
mod field;
pub use field::{Field, Fields};
mod input;
pub(crate) use input::Input;
mod profile;
pub use error::{Error, Result};
pub use game::Game;
pub use profile::Profile;
pub use titles::{Driven, Title};
// The engine's side of the music seam: bytes leave over it, never a parsed
// song, and the crate that implements it is the one that knows the codecs —
// which is how this crate goes without an audio dependency.
pub use music_sink::MusicSink;
mod music_sink;
// Re-exported rather than moved out of sight: `Descriptor` and its enums are
// part of what a caller reads off the engine, and the test suites that build
// scenes by hand name them. Where they are defined is this crate's business;
// that they are here is everyone else's.
pub use buffer::{Buffer, Buffers};
pub use curtain::{Curtain, Fade, Wipe};
pub use descriptor::{Descriptor, Insert, Placement, Shows, TextTemplate};
pub use sample::SampleNode;
pub(crate) use slide::Slide;

use crate::geometry::line_height;
use std::collections::BTreeMap;

use crate::words::Word;
use motionvm_motion_formats::font::{Font, FontRefTable};
use motionvm_motion_formats::{Binding, Generation};
use motionvm_playable::Size;
use motionvm_render::Picture;
// The machine's own `Error` and `Result` are *not* imported here, and this
// crate's `Error` and `Result` — re-exported just above — are what the bare
// names mean throughout it. The two are different channels and the distinction
// is load-bearing: a kernel-word handler fails on the machine's terms, and
// every file under `words/` says so by importing `motionvm_motion_forth::Result`
// itself rather than picking up whatever a `use` in this file happens to name.
// The five signatures below that really do answer the machine spell it out.
use motionvm_motion_forth::Address;
use motionvm_motion_forth::cell;
use motionvm_render::Framebuffer;

/// A palette index that names a surface column, for the tests that paint a
/// surface so that every column tells its own position: the column modulo
/// 256, never the transparent zero except at zero itself.
#[cfg(test)]
pub(crate) fn cell_of(x: i32) -> u8 {
    cell::low8(x)
}

/// A window slide the 16-bit `->SCRX`/`->SCRY` started; see [`Transitions::scroll`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scroll {
    pub(crate) screen: u32,
    pub(crate) vertical: bool,
    pub(crate) target: i32,
    pub(crate) step: i32,
}

/// The game runtime: everything the bytecode's words act on.
///
/// One value holds the whole of it — screens, descriptors, fonts, palettes,
/// texts, sprites, the conversation cursor, the transition queue, the pointer,
/// the clock. There is **no global mutable state anywhere in this workspace**;
/// state is threaded through `&mut Engine` and `&mut m32::Vm`, which is what makes
/// two games in one process, or a test that builds a scene by hand, ordinary
/// rather than delicate.
///
/// It implements [`motionvm_motion_forth::Host`], so from the interpreter's side it is simply the
/// thing that answers for words the interpreter does not own. From the
/// outside it is read-only: see the accessors below, and `Engine::set_music`
/// for the one exception.
pub struct Engine {
    /// The descriptor list and what a picture is drawn out of. See
    /// [`Scene`].
    pub(crate) scene: Scene,
    /// The dialogue machine's two globals. See [`Dialogue`].
    pub(crate) dialogue: Dialogue,
    /// The pointer's shape, visibility and show counter. See
    /// [`Cursor`].
    pub(crate) cursor_state: Cursor,
    /// Where saves go, the module slot table, and the flips. See
    /// [`Persistence`].
    pub(crate) persistence: Persistence,
    /// The music sink, the handles it hands out, and whether a tune is
    /// playing. See [`Sound`].
    pub(crate) sound: Sound,
    /// The date `GIVEDATE` answers once a suite has fixed one; `None` reads
    /// the clock. See [`Engine::fix_date`].
    pub(crate) today: Option<(i32, i32, i32)>,
    /// The master counter — the original's ~1020 Hz tick behind the pointer
    /// at `0xe7f38` (R109) and `0xc3be0` (R78) — as it stands after the
    /// frames so far, advanced by [`Engine::advance_clock`]. What the timer
    /// objects and the samples' clocks measure against; see [`crate::clock`].
    pub(crate) master_ticks: u64,
    /// The timer objects `OPENTIMER` has handed out. See [`clock::Timers`].
    pub(crate) timers: clock::Timers,
    /// The curtains, wipes and scroll in flight, and the fade log.
    /// See [`Transitions`].
    pub(crate) transitions: Transitions,
    /// The frame's input: the mouse record, the key cell, the type-ahead
    /// buffer and the poll budget. See [`Input`].
    pub(crate) input: Input,
    /// What the opener read off the engine build this game runs on, and
    /// nothing changes after: see [`Profile`].
    pub(crate) profile: Profile,
    pub(crate) display: Display,
    /// The video mode: what `SETRES` selected and `TOGFX` entered. See
    /// [`video::Video`].
    pub(crate) mode: video::Video,
    /// What each text descriptor shows, by handle, as last laid out against
    /// the machine's memory — see [`layout`] and [`Engine::lay_out_texts`].
    pub(crate) laid_out: BTreeMap<u32, String>,
    resources: Option<resources::Resources>,
    /// Words that were reached but do nothing yet, with how often.
    pub(crate) stubbed: BTreeMap<Stub, usize>,
    /// The palette rotation `SETCYCLE` asked for, or `None` where it asked
    /// for none — turned once a frame by [`Engine::tick_palette_cycle`].
    pub(crate) palette_cycle: Option<cycle::PaletteCycle>,
    /// The message box the game is waiting on, if it is waiting on one.
    pub(crate) request: Option<request::Request>,
    /// Set when the game asks for a redraw. A still frame is composed on
    /// demand, so this only records that it was asked for.
    pub(crate) dirty: bool,
    /// Places a descriptor carrying `SDAUTOBUF` has left, waiting to be built
    /// again out of the descriptor list by the next drawing pass.
    ///
    /// The original remembers a copy of the picture instead and pastes it back
    /// (0x6ac33); see [`Descriptor::auto_buffer`].
    pub(crate) rebuild: Vec<(u32, (i32, i32, i32, i32))>,
    /// How many passes the drawer has made: the stamp a descriptor takes
    /// when a pass draws it, and a paint when it is painted between passes
    /// ([`crate::paint`]).
    pub(crate) draw_pass: u64,
    /// The word `CTRL` was handed: the game's own per-frame controller.
    ///
    /// `START` ends with `0x42150 CTRL`, and that address is `ICTRL` in module
    /// 4 — 3236 cells that call the location handler, act on `_NEXTLOC`, call
    /// the animation handler and read the mouse.
    ///
    /// `CTRL` does not run that word — its handler is seven instructions and
    /// only stores the address at `0xdb4a8`. All 356 kernel words were scanned
    /// for that address and exactly one reads it: `ANIMPLAY`. So the game's
    /// frame loop is `ANIMPLAY`, which `START` enters right after and never
    /// returns from until the game ends.
    pub(crate) controller: Option<Address>,
    /// Whether the game is inside `ANIMPLAY`, its main loop.
    ///
    /// Durable: it stays set for as long as the game runs. It is deliberately
    /// not what stops the interpreter — `entering_loop` below is.
    pub(crate) main_loop: bool,
    /// A single request to stop, raised the moment `ANIMPLAY` is entered.
    ///
    /// Cleared by whoever acts on it. Keeping the stop tied to the durable flag
    /// instead looked right and was not: the controller runs inside that loop,
    /// so it would have paused after every primitive it executed and crawled
    /// forward one step a frame.
    pub(crate) entering_loop: bool,
    /// Ticks between frames, as `DELAY` sets them.
    ///
    /// `ANIMPLAY` waits out this many before each frame: it resets a timer and
    /// spins until the elapsed count reaches the value, or skips the wait when
    /// it is -1 (0x68fbd to 0x68fd4). `DELAY n` puts `200/n` here; the 32-bit
    /// `START` asks for `25 DELAY` — so eight, the default below — and the
    /// 16-bit `RUN` for `15 DELAY`, which overwrites it before a window ever
    /// asks.
    pub(crate) frame_ticks: i32,
    /// Where the game data is, for the few words that touch files directly.
    dir: Option<std::path::PathBuf>,
    /// Where `->STARTSAMPLE`'s files are, as `SMPPATH` names it; `None` for
    /// a game that ships no such file, or names a directory the copy has not
    /// got. See `resources::sample_dir`.
    sample_dir: Option<std::path::PathBuf>,
    /// The mode `MOUSEINFO` last saw, in the global the handler keeps at
    /// 0xdbd5c. A mode switch counts as a change even if nothing moved.
    last_info_mode: Option<i32>,
    /// The two colors `SYSFC` and `SYSBC` set, which only the request box is
    /// drawn in: the frame, the button outlines and the text in the first,
    /// the fill in the second. `LL.EXE` starts them at 0 and 2 (`ds:0x1b0`,
    /// `ds:0x1b2`) and only Victor Loomes sets them.
    pub(crate) system_fg: i32,
    pub(crate) system_bg: i32,
    /// A bytecode word a primitive asked to have called after it. See
    /// [`motionvm_motion_forth::Host::pending_call`].
    pending_call: Option<(i32, Vec<i32>)>,
    /// The walker's step size, as `STEPMULTI` sets it (`0xdbc58`). Never zero.
    ///
    /// It scales both halves of a walk: the planner multiplies every step it
    /// lays out by it, and the stepper advances the walk cycle by it. The
    /// options menu is what turns it, through `_GSMODE`.
    pub(crate) step_multi: i32,
    /// Which word each kernel ordinal is, resolved once when the game opened.
    /// See [`Engine::bind_words`]. Empty until then, which is what a test that
    /// drives the words directly leaves it as.
    pub(crate) words: Vec<Option<Word>>,
    /// The off-screen buffers of the 16-bit kernel's `SETBUF`/`SDBUF`/`BUFON`
    /// family — see [`buffer::Buffers`]. Empty for the 32-bit games, whose
    /// `SETBUF` is inert.
    pub(crate) buffers: Buffers,
    /// The text backing's remap row, kept until the palette changes.
    backing: draw::BackingCache,
    /// What is on the screen — the original's video memory.
    ///
    /// Kept rather than composed afresh every frame because the original keeps
    /// it: nothing reaches the visible screen except through the presenter
    /// (0x1457D), and the presenter only copies the tiles somebody marked. A
    /// curtain marks its bands and nothing else, so what it does not touch
    /// stays standing. See [`Self::present`] and [`Self::advance_curtain`].
    pub(crate) video: Framebuffer,
    /// The frame handed out, composed into rather than allocated afresh.
    ///
    /// [`Engine::frame`] paints the video surface, the request box and the
    /// pointer into this and lends it out; it is 307 200 bytes and a window
    /// asks for one up to a hundred times a second, so a fresh one per present
    /// was the largest allocation on the path.
    presented: Framebuffer,
}

impl std::fmt::Debug for Engine {
    /// The frame's standing — mode, screens, descriptors, the controller —
    /// rather than every cached sprite and font: what a failing test wants
    /// to see is where the engine stands, and the picture is its own thing.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("profile", &self.profile)
            .field("mode", &self.mode)
            .field("screens", &self.display.screens.len())
            .field("descriptors", &self.scene.descriptors.len())
            .field("selected", &self.scene.selected)
            .field("controller", &self.controller)
            .field("main_loop", &self.main_loop)
            .field("frame_ticks", &self.frame_ticks)
            .field("transitions", &self.transitions)
            .field("input", &self.input)
            .finish_non_exhaustive()
    }
}

/// The pointer, as the engine's mouse record holds it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Mouse {
    /// Pointer x, field +0 of the record at 0x253d4.
    pub x: i32,
    /// Pointer y, +4.
    pub y: i32,
    /// The left button, +8. Non-zero while it is down.
    pub left: i32,
    /// The right button, +12.
    pub right: i32,
}

/// What the engine shows the outside world.
///
/// Every one of these answers with a borrow and none of them lets a caller
/// write — the one exception is [`Engine::set_music`], which installs the
/// sink. The rule is held by the crate graph as much as by the accessors:
/// the only crate that depends on this one is the family's front door, and
/// what it hands a window is a `Box<dyn Playable>`, so nothing outside the
/// family can even name an engine field. The accessors make the same promise
/// inside the family, where the test suites drive the engine directly.
///
/// It matters more here than in most programs. This is a reimplementation of a
/// 1996 engine, and the property worth enforcing is that game state changes
/// only the way the game changes it — through the kernel words the bytecode
/// runs. A caller that could reach in and set a descriptor's position would
/// be able to produce a picture the original never could, and nothing would
/// report it.
impl Engine {
    /// The palette in force. Everything drawn is indices into it.
    pub fn palette(&self) -> &motionvm_render::Palette {
        &self.display.palette
    }

    /// The palette as the *script* last set it.
    ///
    /// Usually the same as [`Self::palette`]. They part while a fade is
    /// queued: a `SETPAL` behind one waits for its turn on the display
    /// (see the handler), but the script that issued it acts on the new
    /// entries at once — `RGB->COL` searches them, a save records them,
    /// and the drawer's palette-derived tables are built from them, the
    /// way the original rebuilds its tables inside `SETPAL` itself.
    pub(crate) fn script_palette(&self) -> &motionvm_render::Palette {
        self.transitions
            .wipes
            .iter()
            .rev()
            .find_map(|w| w.palette_after.as_ref())
            .or_else(|| {
                self.transitions
                    .curtains
                    .iter()
                    .rev()
                    .find_map(|c| c.palette_after.as_ref())
            })
            .unwrap_or(&self.display.palette)
    }

    /// The size of the composed picture.
    pub fn display_size(&self) -> Size {
        self.display.size
    }

    /// The off-screen buffers a 16-bit game has asked for.
    pub fn buffers(&self) -> &Buffers {
        &self.buffers
    }

    /// The screens, in the order they were created.
    pub fn screens(&self) -> &[Screen] {
        &self.display.screens
    }

    /// The visible frame, composed from the active screens.
    ///
    /// Composing rather than reading: [`Engine::render`] answers with what the
    /// original keeps in video memory, which is not the same thing while a
    /// curtain is running. This is the composition itself, for a caller that
    /// wants it without the presenter's history.
    pub fn compose(&self) -> Framebuffer {
        self.display.compose()
    }

    /// The descriptor list, unsorted — see `frame_order` for drawing order.
    pub fn descriptors(&self) -> &[Descriptor] {
        &self.scene.descriptors
    }

    /// The text templates `DEFTDT` has defined.
    pub fn templates(&self) -> &[TextTemplate] {
        &self.scene.templates
    }

    /// Fonts by the handle `+FONT` handed out.
    pub fn fonts(&self) -> &BTreeMap<i32, Font> {
        &self.scene.fonts
    }

    /// The glyph reference table, once a font has been loaded.
    pub fn font_refs(&self) -> Option<&FontRefTable> {
        self.scene.font_refs.as_ref()
    }

    /// `000.FNT`, which a text descriptor uses when nothing chose a font.
    pub fn system_font(&self) -> Option<&Font> {
        self.scene.system_font.as_ref()
    }

    /// A sprite that has already been decoded, without decoding one.
    ///
    /// Deliberately not the loading path: that one is `&mut self` because a
    /// load can set the palette. See the note on `Engine::sprite`.
    pub fn cached_sprite(&self, id: u32) -> Option<&Picture> {
        self.scene.sprites.get(&id)
    }

    /// Every fade that has been started, in order — see [`Fade`].
    ///
    /// Instrumentation: the engine never reads it back. It is here because a
    /// fade is otherwise invisible to a test, being over before a frame is
    /// asked for.
    pub fn fades(&self) -> &[Fade] {
        &self.transitions.fades
    }

    /// Every screen's controller word id, in screen order, `-1` for a screen
    /// that runs none — what `ANIMPLAY` walks.
    pub fn screen_controllers(&self) -> Vec<i32> {
        self.display.screens.iter().map(|s| s.controller).collect()
    }

    /// Makes the screen that runs `id` the current one, the way `ANIMPLAY`
    /// does before it runs a controller (`0104:5600` in `LL.EXE`).
    pub(crate) fn select_screen_of_controller(&mut self, id: i32) {
        if let Some(h) = self
            .display
            .screens
            .iter()
            .find(|s| s.controller == id)
            .map(|s| s.handle)
        {
            self.select_screen(h);
        }
    }

    /// Whether a request box is up and waiting to be answered.
    pub fn has_request(&self) -> bool {
        self.request.is_some()
    }

    /// Words that were reached but do nothing yet, with how often — by the
    /// kernel's own spelling, a case in parentheses where only a case of the
    /// word is inert (`FADEIN (mode 2)`).
    ///
    /// The other half of the instrumentation. A run can be asked afterwards
    /// what it walked past; the inert words are listed on `NO_EFFECT`.
    pub fn stubbed(&self) -> BTreeMap<String, usize> {
        self.stubbed
            .iter()
            .map(|(stub, &n)| (stub.to_string(), n))
            .collect()
    }

    /// Resolves this kernel's ordinals into the words the engine implements,
    /// once, for the machine the game runs on.
    ///
    /// The same shape and the same moment as the interpreter's own dispatch
    /// table — a `Vec` indexed by ordinal, built when the game opens. One
    /// index per kernel word, where resolving by name would be a map lookup
    /// and then up to seventeen string matches, on every kernel word of every
    /// frame.
    ///
    /// Per generation because a word's *meaning* is: eleven names have a
    /// different handler on the two machines, and which resolver ran is what
    /// says which one this ordinal is. See [`words::Word`].
    ///
    /// An ordinal the engine does not implement stays `None` and reaches
    /// [`motionvm_motion_forth::Error::Unimplemented`] with its name, as
    /// before.
    pub(crate) fn bind_words(&mut self, binding: &Binding, m32: bool) {
        let resolve = if m32 { Word::of_m32 } else { Word::of_m16 };
        let top = cell::index(binding.words.last().map_or(0, |&(o, _)| o));
        self.words = vec![None; top + 1];
        for (ordinal, name) in &binding.words {
            self.words[cell::index(*ordinal)] = resolve(name);
        }
    }

    /// The word an ordinal stands for, or `None` for one this engine does not
    /// implement.
    pub(crate) fn word_of(&self, ordinal: u32) -> Option<Word> {
        self.words.get(cell::index(ordinal)).copied().flatten()
    }

    /// Whether this engine implements the kernel word `name` on `generation`'s
    /// machine — the same question `Engine::bind_words` asks for every
    /// ordinal when a game opens, asked by name.
    ///
    /// For the suite that holds the engine to what the shipped games reach
    /// for: a word a module calls is either one of the interpreter's own
    /// primitives, one of these, or a word the run stops on with
    /// [`motionvm_motion_forth::Error::Unimplemented`]. The inspection CLI
    /// cannot ask this — it deliberately knows no engine — so the answer is
    /// given here, where both resolvers are.
    pub fn implements(name: &str, generation: Generation) -> bool {
        match generation {
            Generation::Motion32 => Word::of_m32(name).is_some(),
            Generation::Motion16 => Word::of_m16(name).is_some(),
        }
    }

    /// The ordinal this game's kernel gives `name`, if the engine implements
    /// it — the inverse of [`Engine::word_of`].
    ///
    /// For a caller that has a name and needs to reach a word the way the
    /// bytecode does, which is a test: loading a savegame by hand is `GET`,
    /// `INCLLOC`, `GETANIM` and `=>GETAS` in that order, and reproducing a
    /// reported state is far quicker from a savegame than from an hour of
    /// play. A linear scan, because nothing on a frame comes this way.
    pub(crate) fn ordinal_of(&self, name: &str) -> Option<u32> {
        let want = [Word::of_m32(name), Word::of_m16(name)];
        self.words
            .iter()
            .position(|w| w.is_some() && want.contains(w))
            .map(cell::narrow)
    }

    /// What this run has to report about itself: the inert words it walked
    /// past, and whatever the music sink has to say.
    ///
    /// The machine's own findings are not here, because they are not the
    /// engine's to know — a stray read is the 32-bit memory's bookkeeping, and
    /// the 16-bit machine has no such notion at all. The generation's `Driven`
    /// impl adds them, which is where the machine is named.
    ///
    /// This is what makes the departures ledger's promise true. It says a
    /// stray read is "counted … and names the total at the end of a run", and
    /// for a long time nothing outside the test suite ever asked.
    pub fn diagnostics(&self) -> Vec<motionvm_playable::Diagnostic> {
        let mut out = Vec::new();
        if !self.stubbed.is_empty() {
            // One line, not one per word: the interesting number is how many
            // kinds, and the list is short enough to read on it.
            let words: Vec<String> = self
                .stubbed
                .iter()
                .map(|(stub, n)| format!("{stub} ×{n}"))
                .collect();
            out.push(motionvm_playable::Diagnostic {
                subject: "words reached that do nothing",
                detail: words.join(", "),
            });
        }
        out.extend(
            self.sound
                .sink
                .iter()
                .flat_map(|sink| sink.diagnostics())
                .map(|detail| motionvm_playable::Diagnostic {
                    subject: "music",
                    detail,
                }),
        );
        out
    }

    /// The pointer's shape: the sprite and hotspot `XATMOUSE` installed, or
    /// the engine's own arrow in the two indices `TOGFX` or `NORMMOUSE`
    /// resolved for it. `None` before graphics.
    pub fn cursor(&self) -> Option<PointerShape> {
        self.cursor_state.shape
    }

    /// Whether the pointer is drawn: its show counter stands at one or more.
    pub fn pointer_visible(&self) -> bool {
        self.cursor_state.visible()
    }

    /// The word `CTRL` was handed: the game's own per-frame controller.
    pub fn controller(&self) -> Option<Address> {
        self.controller
    }

    /// Whether the game is inside `ANIMPLAY`, its main loop.
    pub fn main_loop(&self) -> bool {
        self.main_loop
    }

    /// Ticks between frames, as `DELAY` set them.
    pub fn frame_ticks(&self) -> i32 {
        self.frame_ticks
    }

    /// Where the music goes.
    ///
    /// `None` is the faithful third case — the original's words answer 0 when
    /// sound never initialized — so this is what turns sound on rather than a
    /// switch that defaults to it.
    pub fn set_music(&mut self, sink: Box<dyn MusicSink>) {
        self.sound.sink = Some(sink);
    }

    /// Fixes the date `GIVEDATE` answers — day, month, year — in place of
    /// the machine's own.
    ///
    /// For a suite. The one game that reads the date turns it into the
    /// current issue number of a weekly magazine, so a run on the clock
    /// answers differently every Thursday. Nothing else in the engine reads a
    /// clock, which is what makes a scene compose the same on two machines
    /// and lets its digest be checked in; this keeps that true for the one
    /// word that would break it.
    pub fn fix_date(&mut self, day: i32, month: i32, year: i32) {
        self.today = Some((day, month, year));
    }

    /// The date `GIVEDATE` pushes: the fixed one, or today's.
    pub(crate) fn today(&self) -> (i32, i32, i32) {
        self.today.unwrap_or_else(calendar::today)
    }

    /// Takes one press into the keyboard buffer, translated on the way in.
    ///
    /// The buffer is the BIOS type-ahead buffer `?KEY` reads through INT 16h:
    /// fifteen keystrokes deep, so a sixteenth is dropped here exactly as the
    /// original dropped it. A press the PC has no answer for — a modifier on
    /// its own, a media key — is never enqueued, because it never reached the
    /// BIOS buffer either.
    pub fn push_key(&mut self, press: &motionvm_playable::KeyPress) {
        if self.input.buffer.len() < keys::SLOTS
            && let Some(code) = keys::code(press)
        {
            self.input.buffer.push_back(code);
        }
    }

    /// Takes the oldest waiting keystroke out of the buffer, or 0 for none —
    /// `?KEY`'s own answer when nothing waits.
    pub fn pop_key(&mut self) -> i32 {
        self.input.buffer.pop_front().unwrap_or(0)
    }
}

/// Building engine state directly, which the game never does.
///
/// The game makes descriptors with `NEWSETDESC`, screens with `NEWSCREEN` and
/// loads sprites by drawing them. These four exist because a test that checks
/// one behavior should not have to play the game up to the point that produces
/// it — `tick_descriptor`'s wait arithmetic needs one descriptor and no game at
/// all.
///
/// **This is the deliberate hole in the facade, and it is named so it can be
/// found.** Everything else outside this crate can only read; a caller that
/// reached in here would show up in a search for these four names, where a
/// `pub` field would have shown up nowhere. Only this crate's own tests use
/// them, and
/// `grep -rn "descriptors_mut\|add_descriptor\|add_screen\|cache_sprite"` is how
/// that stays true.
impl Engine {
    /// Adds a ready-made descriptor and answers with its handle.
    pub fn add_descriptor(&mut self, d: Descriptor) -> u32 {
        let handle = d.handle;
        self.scene.descriptors.push(d);
        handle
    }

    /// The descriptor list, to be changed in place.
    pub fn descriptors_mut(&mut self) -> &mut [Descriptor] {
        &mut self.scene.descriptors
    }

    /// Adds a ready-made screen.
    pub fn add_screen(&mut self, screen: Screen) {
        self.display.screens.push(screen);
    }

    /// Puts a sprite in the graphics cache under an id, without a resource file.
    pub fn cache_sprite(&mut self, id: u32, sprite: Picture) {
        self.scene.sprites.insert(id, sprite);
    }
}

impl Engine {
    /// An engine with nothing loaded — no resources, no screens, no
    /// descriptors — on the engine build `profile` describes.
    ///
    /// Usable on its own for anything that does not need the game's files —
    /// the descriptor list, the wait arithmetic, the curtain queue. A game
    /// comes from [`Game::open`], which builds one of these and points it at a
    /// directory. The only constructor, and it takes the whole profile at
    /// once: everything a generation decides arrives before the engine exists,
    /// so there is no moment at which a half-configured engine is reachable.
    pub fn new(profile: Profile) -> Self {
        Self {
            scene: Scene::default(),
            dialogue: Dialogue::default(),
            cursor_state: Cursor::unarmed(),
            persistence: Persistence::default(),
            sound: Sound::default(),
            today: None,
            master_ticks: 0,
            timers: clock::Timers::default(),
            transitions: Transitions::default(),
            input: Input::default(),
            display: Display::with_size(profile.display),
            profile,
            mode: video::Video::seeded(),
            laid_out: BTreeMap::new(),
            resources: None,
            stubbed: BTreeMap::new(),
            palette_cycle: None,
            request: None,
            dirty: false,
            rebuild: Vec::new(),
            draw_pass: 0,
            controller: None,
            main_loop: false,
            entering_loop: false,
            // The 32-bit `START`'s `25 DELAY`; a 16-bit game's own `DELAY`
            // overwrites it during startup.
            frame_ticks: 8,
            dir: None,
            sample_dir: None,
            last_info_mode: None,
            system_fg: 0,
            system_bg: 2,
            pending_call: None,
            step_multi: 1,
            words: Vec::new(),
            buffers: Buffers::default(),
            backing: draw::BackingCache::default(),
            video: Framebuffer::new(profile.display.width, profile.display.height),
            presented: Framebuffer::new(profile.display.width, profile.display.height),
        }
    }

    /// Records a word that ran but deliberately did nothing.
    ///
    /// Only for words [`Word::inert`] admits. Everything else must either be
    /// implemented or fail — see there for why.
    pub(crate) fn note_no_effect(&mut self, word: Word) {
        debug_assert!(
            word.inert(),
            "{} was treated as inert but [`Word::inert`] does not say so",
            word.name()
        );
        self.note_unhandled(word, None);
    }

    /// A word that pops its arguments and does nothing else.
    ///
    /// The two halves belong together: take the arity off the stack, then
    /// count the word as inert. Saying it once puts the contract in one place
    /// — the word has to be one
    /// [`words::Word::inert`] admits, and [`Engine::note_no_effect`] asserts
    /// it in a debug build.
    pub(crate) fn inert(
        &mut self,
        stack: &mut Vec<i32>,
        arity: usize,
        word: Word,
    ) -> motionvm_motion_forth::Result<()> {
        stack::pop_n(stack, arity, word.name())?;
        self.note_no_effect(word);
        Ok(())
    }

    /// How long `ENDTUNE` holds the 16-bit game after starting the music's
    /// fade-out: the stop routine spins until the 200 Hz tick reads 100
    /// (`ENVIRO.EXE` `1696:031d`, `cmp $0x64`) — 500 ms. See [`Wipe::hold`].
    pub(crate) const ENDTUNE_HOLD_TICKS: i32 = 100;

    /// One poll of the pointer or the key buffer by the bytecode.
    pub(crate) fn polled(&mut self) {
        self.input.polls += 1;
        if self.input.polls > Input::BUDGET {
            self.input.polling = true;
        }
    }

    /// Whether the running word should yield the frame because it is
    /// polling for input that cannot change until the next one.
    fn poll_yield(&mut self) -> bool {
        if self.input.polling && self.input.polls > 0 {
            // Spent: the next poll after the resume counts afresh.
            self.input.polls = 0;
            return true;
        }
        false
    }

    /// Counts a word that was reached and did nothing — the whole word, or
    /// the one case of it that is inert.
    fn note_unhandled(&mut self, word: Word, case: Option<String>) {
        *self.stubbed.entry(Stub { word, case }).or_default() += 1;
    }
}

/// A word reached that does nothing here: which word, and which case of it
/// where only a case is inert — `FADEIN` in mode 2, `SCRX` with a non-zero
/// argument. Keyed by the word as a value, so that a stub's identity is the
/// kernel's and not a spelling; the spelling is what [`Engine::stubbed`]
/// answers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Stub {
    word: Word,
    case: Option<String>,
}

impl std::fmt::Display for Stub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.case {
            Some(case) => write!(f, "{} ({case})", self.word.name()),
            None => f.write_str(self.word.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use motionvm_motion_forth::m32;

    /// Every one-argument setter names a field, and every field survives a
    /// savegame.
    ///
    /// A match over the setters and a separate list of the names a savegame
    /// may carry would be two lists nothing holds together, and the way they
    /// drift is silent: a field written by its word and missing from the
    /// loader's list is a slot that saves and then refuses to load, by name.
    /// [`Field`] is one list, and [`Field::of`] is its own inverse, so the
    /// only thing left to check here is that a setter names one at all.
    #[test]
    fn every_descriptor_setter_names_a_field() {
        let setters: Vec<Field> = Word::ALL
            .iter()
            .filter_map(|w| w.descriptor_field())
            .collect();
        assert_eq!(setters.len(), 8, "the setter set changed");
        for f in setters {
            assert_eq!(Field::of(f.name()), Some(f));
        }
    }

    /// Module memory the tests can hand to the host; none of them write to it.
    fn mem() -> m32::Memory {
        m32::Memory::default()
    }

    #[test]
    fn newsetdesc_takes_six_and_returns_a_handle() {
        let mut mem = mem();
        let mut e = Engine::new(Profile::motion32());
        // x y lev spr 0 0 — the shape XYLSITEM. builds.
        let mut stack = vec![10, 20, 3, 99, 0, 0];
        assert!(e.plain_word32("NEWSETDESC", &mut stack, &mut mem).unwrap());
        assert_eq!(stack, vec![1], "should leave just the handle");
        let d = &e.scene.descriptors[0];
        assert_eq!((d.x, d.y, d.level), (10, 20, 3));
    }

    #[test]
    fn descriptor_setters_reach_the_selected_descriptor() {
        let mut mem = mem();
        let mut e = Engine::new(Profile::motion32());
        let mut stack = vec![0, 0, 0, 0, 0, 0];
        e.plain_word32("NEWSETDESC", &mut stack, &mut mem).unwrap();
        stack.clear();

        stack.push(42);
        e.plain_word32("SDX", &mut stack, &mut mem).unwrap();
        assert!(stack.is_empty(), "SDX takes exactly one value");
        e.plain_word32("GDX", &mut stack, &mut mem).unwrap();
        assert_eq!(stack, vec![42]);
    }

    /// The rows the handlers compute, checked against the disassembly.
    ///
    /// `FADEIN` copies one band at `base + i` of height `(h/2 - i) * 2`, so the
    /// visible rows are `[i, h - i)`. `FADEOUT` copies two bands of `step` at
    /// `base + i` and `base + h - step - i`, leaving `[i + step, h - step - i)`
    /// showing. Both run in steps of eight over 480 rows.
    #[test]
    fn the_curtain_matches_the_handlers() {
        let area = (0, 0, 640, 480);
        let screen = 0;
        let mut open = Curtain {
            opening: true,
            screen,
            area,
            step: 8,
            offset: 240,
            ticks_per_band: 1,
            banked: 0,
            palette_after: None,
        };
        assert_eq!(
            open.visible(),
            (240, 240),
            "closed at the middle to begin with"
        );
        open.advance(1);
        assert_eq!(
            open.visible(),
            (232, 248),
            "one step out is 16 rows around the center"
        );

        let mut shut = Curtain {
            opening: false,
            screen,
            area,
            step: 8,
            offset: 0,
            ticks_per_band: 1,
            banked: 0,
            palette_after: None,
        };
        assert_eq!(
            shut.visible(),
            (8, 472),
            "the first bands eat eight rows off each edge"
        );
        shut.advance(1);
        assert_eq!(shut.visible(), (16, 464));

        // Every intermediate band is centered on the screen's middle.
        for c in [&mut open, &mut shut] {
            while !c.done() {
                let (top, bottom) = c.visible();
                assert_eq!(
                    top + bottom,
                    480,
                    "the band is symmetric at offset {}",
                    c.offset
                );
                c.advance(1);
            }
        }

        let mut steps = 0;
        let mut c = Curtain {
            opening: true,
            screen,
            area,
            step: 8,
            offset: 240,
            ticks_per_band: 1,
            banked: 0,
            palette_after: None,
        };
        while !c.done() {
            c.advance(1);
            steps += 1;
        }
        assert_eq!(steps, 31, "480 rows in steps of eight from the middle out");
    }

    /// One picture is one field: what a descriptor shows is either a sprite or
    /// a block, never both at once.
    ///
    /// The original keeps it in `+0x10` with bit 15 marking a sprite, so
    /// `GDSPR` answers -1 for a block and `GDBL` -1 for a sprite (`ENVIRO.EXE`
    /// `05f1:168e` and `05f1:16bd`). That matters beyond tidiness: the verb
    /// menu and the walk cycle read `GDSPR` and count on from it as a frame
    /// number, so a block id coming back there would be animated.
    #[test]
    fn a_descriptor_shows_a_sprite_or_a_block_and_says_which() {
        let mut e = Engine::new(Profile::motion32());
        let handle = e.add_descriptor(Descriptor {
            handle: 1,
            active: true,
            ..Default::default()
        });
        e.select_descriptor(handle);

        e.set_sprite(42).expect("SDSPR");
        assert_eq!(e.descriptor_sprite(), 42);
        assert_eq!(e.descriptor_block(), -1, "it is not a block");

        e.set_block(43).expect("SDBL");
        assert_eq!(e.descriptor_block(), 43);
        assert_eq!(e.descriptor_sprite(), -1, "it is not a sprite any more");
    }

    /// A 16-bit block is painted opaque, a sprite through its key color.
    ///
    /// The distinction is bit 15 of `+0x10` and nothing else — the same
    /// graphics pool, the same id space, two blits (`016a:0fe8`, keyed at
    /// `14ee:0d1e` and opaque at `14ee:0d47`). Color 0 is the key, so a
    /// picture of nothing but zeroes lets the picture under it through as a
    /// sprite and blacks it out as a block.
    #[test]
    fn a_16_bit_block_covers_what_a_sprite_lets_through() {
        let paint = |as_block: bool| {
            let mut e = Engine::new(Profile {
                opaque_blocks: true,
                ..Profile::motion32()
            });
            let mut s = Screen::new(1);
            s.size = (4, 4);
            s.view = (4, 4);
            s.buffer = Framebuffer::new(4, 4);
            s.active = true;
            e.display.screens.push(s);
            for (id, color) in [(7u32, 0u8), (8, 9)] {
                e.scene.sprites.insert(
                    id,
                    Picture {
                        width: 4,
                        height: 4,
                        pixels: vec![color; 16],
                    },
                );
            }
            // Underneath, a block of solid color 9; on top, the picture under
            // test, all key pixels.
            for (handle, level, id) in [(1u32, 0i32, 8u32), (2, 1, 7)] {
                let h = e.add_descriptor(Descriptor {
                    handle,
                    screen: 1,
                    level,
                    active: true,
                    dirty: true,
                    ..Default::default()
                });
                e.select_descriptor(h);
                if id == 8 || as_block {
                    e.set_block(cell::signed(id)).expect("SDBL");
                } else {
                    e.set_sprite(cell::signed(id)).expect("SDSPR");
                }
            }
            e.draw();
            e.display.screens[0].buffer.pixels.clone()
        };
        assert!(
            paint(false).iter().all(|&p| p == 9),
            "a sprite of key pixels lets the block under it through: {:?}",
            paint(false)
        );
        assert!(
            paint(true).iter().all(|&p| p == 0),
            "a block of the same pixels covers it: {:?}",
            paint(true)
        );
    }

    /// Two screens tiling the display, each showing one flat color.
    ///
    /// Small on purpose, but not smaller than the effect: the band height is a
    /// literal 8, so a screen has to be at least a few bands tall for a fade to
    /// have a middle at all. Thirty-two rows is three steps a direction.
    fn two_screens(top: u8, bottom: u8) -> Engine {
        two_screens_on(Profile::motion32(), top, bottom)
    }

    /// The same, on a stated engine build — for the tests that are about what
    /// one build does differently.
    fn two_screens_on(profile: Profile, top: u8, bottom: u8) -> Engine {
        let mut e = Engine::new(profile);
        for (handle, y) in [(1u32, 0i16), (2, 32)] {
            let mut s = Screen::new(handle);
            s.size = (16, 32);
            s.view = (16, 32);
            s.view_pos = (0, y);
            s.buffer = Framebuffer::new(16, 32);
            s.active = true;
            e.display.screens.push(s);
        }
        for (id, color) in [(10u32, top), (11, bottom)] {
            e.scene.sprites.insert(
                id,
                Picture {
                    width: 16,
                    height: 32,
                    pixels: vec![color; 16 * 32],
                },
            );
        }
        for (handle, screen, sprite) in [(1u32, 1u32, 10u32), (2, 2, 11)] {
            e.scene.descriptors.push(Descriptor {
                handle,
                screen,
                shows: Shows::Sprite(sprite),
                active: true,
                dirty: true,
                ..Default::default()
            });
        }
        e.draw();
        e.present();
        e
    }

    /// The 16-bit level chain's tie rule: a fresh `SDLEV` draws on top of
    /// its equals.
    ///
    /// `0362:2007` (the insert `SDLEV` runs at `05f1:1018`) walks to the
    /// first node whose level is *greater* and splices in before it, so the
    /// re-set descriptor lands after every equal; the drawer walks the
    /// chain head first (`016a:0a4f`). And the handler re-inserts even when
    /// the level does not change — a walking figure asserts its level every
    /// step, and that is what keeps it in front of scenery it shares a
    /// level with. On the 32-bit machine nothing bumps the stamps, so the
    /// order stays creation order; its chain handler is unread.
    #[test]
    fn a_fresh_sdlev_draws_on_top_of_its_equals() {
        let mut e = two_screens_on(
            Profile {
                level_chain: true,
                ..Profile::motion32()
            },
            3,
            7,
        );
        // The fixture wires its screens by hand; the damage map behind the
        // marks `SDLEV` makes has to exist for this path.
        for s in &mut e.display.screens {
            let (w, h) = s.view;
            s.set_view(w, h);
        }
        // A second sprite on screen 1, same level as the first: creation
        // order paints it on top.
        e.scene.sprites.insert(
            12,
            Picture {
                width: 16,
                height: 32,
                pixels: vec![5; 16 * 32],
            },
        );
        let stamp = e.next_stamp();
        e.scene.descriptors.push(Descriptor {
            handle: 3,
            screen: 1,
            shows: Shows::Sprite(12),
            active: true,
            dirty: true,
            stamp,
            ..Default::default()
        });
        e.repaint_screen(1);
        e.draw();
        e.present();
        assert_eq!(e.render().get(0, 0), Some(5), "the later equal is on top");

        // `SDLEV` on the first, to the same level it already holds: on the
        // chain that moves it after its equal, and it draws on top now.
        e.scene.selected = e.scene.descriptors.iter().position(|d| d.handle == 1);
        e.set_level(0).expect("SDLEV");
        e.repaint_screen(1);
        e.draw();
        e.present();
        assert_eq!(
            e.render().get(0, 0),
            Some(3),
            "the re-set descriptor draws over the equal it shared a level with"
        );
    }

    /// The 16-bit setters mark even an unchanged value.
    ///
    /// Each 16-bit `SD*` handler writes and sets the dirty bit whatever the
    /// value (`SDX` `05f1:0df2`, `SDFNT` `05f1:1038`), where the read
    /// 32-bit ones skip an unchanged one (0x7111a, 0x71715). The intro's
    /// `.DRAWNEW` is a value written over itself for exactly that mark.
    #[test]
    fn a_sixteen_bit_setter_marks_even_the_unchanged() {
        // Draw the descriptor settled, then set a field to the value it
        // already holds. Two engines rather than one with a flag flipped
        // under it: a capability is decided when a game opens and never
        // after, so the two readings are two builds.
        let settled_then_set = |profile| {
            let mut e = two_screens_on(profile, 3, 7);
            for s in &mut e.display.screens {
                let (w, h) = s.view;
                s.set_view(w, h);
            }
            let i = e
                .scene
                .descriptors
                .iter()
                .position(|d| d.handle == 1)
                .expect("the fixture's descriptor");
            e.scene.selected = Some(i);
            let (x, mode) = (e.scene.descriptors[i].x, e.scene.descriptors[i].x_mode);
            e.draw();
            assert!(
                !e.scene.descriptors[i].dirty,
                "drawing settles the descriptor"
            );
            e.place_x(x, mode).expect("SDX");
            e.scene.descriptors[i].dirty
        };
        assert!(
            !settled_then_set(Profile::motion32()),
            "the 32-bit setter skips an unchanged value"
        );
        assert!(
            settled_then_set(Profile {
                sd_marks_always: true,
                ..Profile::motion32()
            }),
            "the 16-bit setter marks whatever the value"
        );
    }

    /// The 16-bit `SDTDT` takes only 1..=20; anything else is a no-op.
    ///
    /// `05f1:0c78` checks the popped id with `cmp $1` / `jl` and
    /// `cmp $0x14` / `jg` before the store, so `0 SDTDT` cannot clear a
    /// template — the descriptor keeps the one it has.
    /// Die Enviro-Kids greifen ein reaches
    /// that: `SAYDAVID` hands `_SxTDT @` to `SDTDT`, and `_SxTDT` is 0
    /// until the first `SETSAY` (location 17's macro never calls
    /// `TOJEFF`, which is where `SETSAY` runs).
    #[test]
    fn a_sixteen_bit_template_outside_the_table_is_ignored() {
        let mut e = two_screens_on(
            Profile {
                templates_gated: true,
                ..Profile::motion32()
            },
            3,
            7,
        );
        for s in &mut e.display.screens {
            let (w, h) = s.view;
            s.set_view(w, h);
        }
        e.scene.selected = e.scene.descriptors.iter().position(|d| d.handle == 1);
        e.set_template(2).expect("SDTDT");
        for outside in [0, -1, 21] {
            e.set_template(outside).expect("SDTDT");
            assert_eq!(
                e.scene.descriptors[e.scene.selected.unwrap()].template,
                Some(2),
                "{outside} SDTDT left the template standing"
            );
        }
        e.set_template(9).expect("SDTDT");
        assert_eq!(
            e.scene.descriptors[e.scene.selected.unwrap()].template,
            Some(9)
        );
    }

    /// Repaints screen 1's descriptor in `color`, ready for a fade to reveal.
    fn repaint(e: &mut Engine, color: u8) {
        let sprite = e.scene.sprites.get_mut(&10).expect("the top sprite");
        sprite.pixels.fill(color);
    }

    fn fade(e: &mut Engine, mem: &mut m32::Memory, name: &str, screen: u32) {
        e.display.set_current(screen);
        let mut stack = vec![1, 50, 8];
        e.plain_word32(name, &mut stack, mem).unwrap();
    }

    /// A fade in reveals its picture *over* what is on the screen.
    ///
    /// The handler draws the screen once (0x74af9), wipes the update map
    /// (0x74afe) and then marks one growing band. Marking is all it does:
    /// outside the band the presenter copies nothing, so video memory still
    /// holds the previous frame. Blacking that out instead — which is right
    /// for `FADEOUT`, whose surface really is filled with color 0 first — is
    /// what made every menu page and help page blink through black, because
    /// the menu fades in without ever fading out (module 4, 0x01af4, 0x01bb4,
    /// 0x03bc0, 0x03be8 …).
    #[test]
    fn a_fade_in_reveals_its_picture_over_the_old_one() {
        let mut mem = mem();
        let mut e = two_screens(3, 7);
        assert_eq!(
            e.render().get(0, 0),
            Some(3),
            "the old picture is on screen"
        );

        repaint(&mut e, 5);
        fade(&mut e, &mut mem, "FADEIN", 1);

        // The band opens from row 16 outwards in steps of eight. While it is
        // part-way, the middle must be the new picture and both edges the old.
        let mut seen_middle = false;
        while e.in_transition() {
            e.advance_curtain();
            let frame = e.render();
            let (top, bottom) = (frame.get(0, 0), frame.get(0, 31));
            let middle = frame.get(0, 16);
            if middle == Some(5) && top == Some(3) {
                seen_middle = true;
                assert_eq!(
                    bottom,
                    Some(3),
                    "the bottom edge still shows the old picture too"
                );
            }
            assert_eq!(
                frame.get(0, 40),
                Some(7),
                "the other screen is not in this fade"
            );
        }
        assert!(
            seen_middle,
            "the fade never showed new middle and old edges at once"
        );
        assert_eq!(
            e.render().get(0, 0),
            Some(5),
            "and it ends on the new picture"
        );
    }

    /// A fade out queued before a fade in hides the *old* picture.
    ///
    /// The shape a single nested call produces — `DO_INVSEL`'s documents branch
    /// does exactly this, `FADEOUT` on the picture, descriptors swapped,
    /// `FADEIN` on the picture, all before the interpreter pauses (module 4,
    /// 0x01c6c–0x01ca8). The later `FADEIN` has already drawn the new picture
    /// into the screen buffer by the time the earlier `FADEOUT` plays, so a
    /// fade out that composed the buffers afresh would hide a picture nobody
    /// had seen yet.
    #[test]
    fn a_queued_fade_out_hides_the_picture_that_was_showing() {
        let mut mem = mem();
        let mut e = two_screens(3, 7);

        fade(&mut e, &mut mem, "FADEOUT", 1);
        repaint(&mut e, 5);
        fade(&mut e, &mut mem, "FADEIN", 1);
        assert_eq!(
            e.transitions.curtains.len(),
            2,
            "both curtains are queued before either plays"
        );

        let mut closing = true;
        while e.in_transition() {
            if closing && e.transitions.curtains.front().map(|c| c.opening) == Some(true) {
                closing = false;
            }
            e.advance_curtain();
            let frame = e.render();
            if closing {
                for row in 0..32 {
                    assert_ne!(
                        frame.get(0, row),
                        Some(5),
                        "the new picture showed during the fade out"
                    );
                }
            }
        }
        assert_eq!(
            e.render().get(0, 0),
            Some(5),
            "the fade in brings the new picture up"
        );
    }

    /// The band width comes from the field each handler reads.
    ///
    /// `FADEIN` takes `+0x18` at 0x74b22 — `SCRSIZE`, the whole surface — and
    /// `FADEOUT` takes `+0x1C` at 0x74d70, the `SCRVSIZE` window. No screen the
    /// game builds sets the two apart, so this is the only place the difference
    /// can be seen at all.
    #[test]
    fn the_two_directions_read_different_width_fields() {
        let mut mem = mem();
        let mut e = two_screens(3, 7);
        e.display.screen_mut(1).expect("screen 1").view = (8, 32);

        fade(&mut e, &mut mem, "FADEIN", 1);
        assert_eq!(
            e.transitions.curtains.front().expect("a curtain").area.2,
            16,
            "a fade in spans SCRSIZE"
        );
        e.transitions.curtains.clear();

        fade(&mut e, &mut mem, "FADEOUT", 1);
        assert_eq!(
            e.transitions.curtains.front().expect("a curtain").area.2,
            8,
            "a fade out spans SCRVSIZE"
        );
    }

    /// No pointer while a curtain runs.
    ///
    /// Both handlers call `HIDEMOUSE` (0x74ae2) before the band loop and
    /// `SHOWMOUSE` (0x74b64) after it.
    #[test]
    fn the_pointer_stays_away_while_a_curtain_runs() {
        let mut mem = mem();
        let mut e = two_screens(3, 7);
        e.scene.sprites.insert(
            99,
            Picture {
                width: 2,
                height: 2,
                pixels: vec![9; 4],
            },
        );
        e.cursor_state.shape = Some(PointerShape::Sprite {
            id: 99,
            hot_x: 0,
            hot_y: 0,
        });
        e.cursor_state.shows = 1;
        e.input.mouse.x = 0;
        e.input.mouse.y = 0;
        assert_eq!(
            e.render().get(0, 0),
            Some(9),
            "the pointer draws when nothing is fading"
        );

        fade(&mut e, &mut mem, "FADEOUT", 1);
        assert_eq!(
            e.render().get(0, 0),
            Some(3),
            "and not while a curtain is up"
        );
        assert!(
            e.cursor_state.visible(),
            "the script's own state is left alone"
        );
    }

    /// `FADEIN`'s mode 2 is the curtain with `WHITEBOX`'s box painted
    /// first — at 25,122, 452 by 317, the frame two in — and no wait
    /// between its bands: one step opens it whole, and the box is on the
    /// screen's surface under everything the bands reveal.
    #[test]
    fn a_mode_two_fade_in_paints_the_box_and_opens_at_once() {
        let mut mem = mem();
        let mut e = Engine::new(Profile::motion32());
        e.select_mode(video::MODE_640X480X256);
        e.enter_graphics().expect("the mode");
        // A palette with a white at 5 and a black at 9 among grays, so the
        // two indices are neither each other nor the surface's own 0.
        let mut raw = [20u8; motionvm_render::Palette::BYTES];
        raw[15..18].copy_from_slice(&[63, 63, 63]);
        raw[27..30].copy_from_slice(&[0, 0, 0]);
        e.display.palette = motionvm_render::Palette { raw };
        let (white, black) = (5, 9);
        let h = e.display.new_screen();
        let s = e.display.screen_mut(h).unwrap();
        s.set_size(640, 480);
        s.set_view(640, 480);
        e.display.set_current(h);
        let mut stack = vec![2, 50, 8];
        e.plain_word32("FADEIN", &mut stack, &mut mem).unwrap();
        assert!(e.in_transition(), "mode 2 is the curtain");
        assert_eq!(e.step_ticks(), 0, "and it waits for nothing");
        e.advance_curtain();
        assert!(!e.in_transition(), "one step opens it whole");
        let picture = e.render();
        let at = |x, y| picture.get(x, y);
        assert_eq!(at(25, 122), Some(white), "the box's corner");
        assert_eq!(at(27, 124), Some(black), "the frame, two in");
        assert_eq!(at(30, 130), Some(white), "white inside the frame");
        assert_eq!(at(476, 438), Some(white), "the box's far corner");
        assert_eq!(at(477, 439), Some(0), "and nothing past it");
    }

    /// `FADEOUT`'s mode 2 is a different effect — a translucent fade through
    /// the darkening tables (0x74eef, color 0x102) — that no shipped call
    /// reaches, so it is recorded and skipped. It must not go through
    /// `note_no_effect`, whose `debug_assert!` fires for any name not on
    /// `NO_EFFECT`, and neither fade word is on that list.
    #[test]
    fn a_mode_two_fade_out_is_recorded_rather_than_fatal() {
        let mut mem = mem();
        let mut e = two_screens(3, 7);
        e.display.set_current(1);
        let mut stack = vec![2, 50, 8];
        e.plain_word32("FADEOUT", &mut stack, &mut mem).unwrap();
        assert!(!e.in_transition(), "mode 2 is not the curtain");
        assert_eq!(
            e.stubbed().get("FADEOUT (mode 2)"),
            Some(&1),
            "and it is reported, not swallowed"
        );
    }

    /// R78's curtains wait for nothing: with the capability off, a mode-1
    /// fade opens in one step of no ticks.
    #[test]
    fn an_unwaiting_build_opens_its_curtain_in_one_step() {
        let mut mem = mem();
        let mut e = two_screens(3, 7);
        e.profile.curtains_wait = false;
        e.display.set_current(1);
        let mut stack = vec![1, 50, 8];
        e.plain_word32("FADEIN", &mut stack, &mut mem).unwrap();
        assert!(e.in_transition());
        assert_eq!(e.step_ticks(), 0);
        e.advance_curtain();
        assert!(!e.in_transition());
    }

    /// Every fade's pace, worked out from the handler and checked at three
    /// heights — including the one the game only reaches in its title.
    ///
    /// The handler divides twice and the two divisions do different jobs:
    ///
    /// ```text
    /// bands = view_height / 16     0x74cd0   only ever the divisor below
    /// half  = view_height / 2      0x74ce7
    /// delay = duration / bands     0x74cf6   signed, truncating, no floor
    /// loop: i = 0 … half step 8    0x74d55   -> half/8 + 1 passes
    /// ```
    ///
    /// So the pass count is *not* `bands`, and the total is
    /// `(half/8 + 1) * (50 / (h/16))`. Truncation makes the big screen the
    /// fast one: 50/30 is 1 where 50/25 is 2, so the 480-row title fades in
    /// 31 ticks against the 400-row game's 52. That is the original, and the
    /// reason the intro's fades are quicker than everything after them.
    #[test]
    fn the_pace_of_a_fade_follows_the_two_divisions() {
        // height -> ticks a band, passes, ticks in total
        for (height, per, passes, total) in [(480, 1, 31, 31), (400, 2, 26, 52), (80, 10, 6, 60)] {
            let mut mem = mem();
            let mut e = Engine::new(Profile::motion32());
            let mut s = Screen::new(1);
            s.size = (640, cell::low16(height));
            s.view = (640, cell::low16(height));
            s.buffer = Framebuffer::new(640, cell::low16(height));
            e.display.screens.push(s);
            e.display.set_current(1);

            let mut stack = vec![1, 50, 8];
            e.plain_word32("FADEOUT", &mut stack, &mut mem).unwrap();
            let c = e.transitions.curtains.front().expect("a curtain").clone();
            assert_eq!(c.ticks_per_band, per, "{height} rows: 50 / ({height}/16)");
            assert_eq!(e.step_ticks(), per, "and a step is one band's worth");

            let (mut steps, mut ticks) = (0, 0);
            while e.in_transition() {
                ticks += e.step_ticks();
                e.advance_curtain();
                steps += 1;
                assert!(steps < 500, "{height} rows: the curtain never finished");
            }
            assert_eq!(steps, passes, "{height} rows: half/8 + 1 passes");
            assert_eq!(ticks, total, "{height} rows: one wait a pass");
        }
    }

    #[test]
    fn actdesc_selects_by_handle() {
        let mut mem = mem();
        let mut e = Engine::new(Profile::motion32());
        for x in [1, 2] {
            let mut s = vec![x, 0, 0, 0, 0, 0];
            e.plain_word32("NEWSETDESC", &mut s, &mut mem).unwrap();
        }
        let mut stack = vec![1]; // the first descriptor's handle
        e.plain_word32("ACTDESC", &mut stack, &mut mem).unwrap();
        let mut out = Vec::new();
        e.plain_word32("GDX", &mut out, &mut mem).unwrap();
        assert_eq!(out, vec![1], "should have selected the first descriptor");

        // A handle that names nothing leaves the selection alone. The original
        // resolves it first and jumps past both stores when that fails
        // (0x71618 → 0x71624 → 0x71636), so the descriptor that was selected
        // stays selected and the `SD…` words that follow still land.
        let mut stack = vec![999];
        e.plain_word32("ACTDESC", &mut stack, &mut mem).unwrap();
        let mut out = Vec::new();
        e.plain_word32("GDX", &mut out, &mut mem).unwrap();
        assert_eq!(
            out,
            vec![1],
            "an unknown handle must not clear the selection"
        );
    }

    /// `GDCX SDCEN` has to be a round trip, and only the stored size makes it
    /// one.
    ///
    /// `TSC` ends every clamped line of speech with `GDCX SDCEN GDCY SDVCEN`
    /// (module 5), putting the text back where it already was. That only holds
    /// if the size `GDCX` halves is the same size the placement halved:
    /// `(x − w/2) + w/2 = x`. Both are `desk[+0x2e]`/`[+0x32]` in the original —
    /// 0x712a9 halves what 0x6b190 returns, and 0x6bdfa subtracts the very same
    /// call. Halving the bare measurement on one side only had every spoken
    /// line drift two pixels left and two up each time `TSC` ran.
    #[test]
    fn centering_a_text_round_trips_through_gdcx() {
        let mut mem = mem();
        let mut e = Engine::new(Profile::motion32());
        let mut stack = vec![0, 0, 0, 0, 0, 0];
        e.plain_word32("NEWSETDESC", &mut stack, &mut mem).unwrap();
        stack.clear();

        // A text descriptor, centered on a point, with a measurable extent. No
        // font is loaded, so the measurement is whatever `extent` makes of an
        // empty text — the identity has to hold for any width, not a lucky one.
        for (word, v) in [("SDTB", 8), ("SDTXT", 1), ("SDCEN", 200), ("SDVCEN", 100)] {
            stack.push(v);
            e.plain_word32(word, &mut stack, &mut mem).unwrap();
        }

        for (get, set, want) in [("GDCX", "SDCEN", 200), ("GDCY", "SDVCEN", 100)] {
            let mut out = Vec::new();
            e.plain_word32(get, &mut out, &mut mem).unwrap();
            assert_eq!(
                out,
                vec![want],
                "{get} must answer with the point it was centered on"
            );
            e.plain_word32(set, &mut out, &mut mem).unwrap();
            let mut again = Vec::new();
            e.plain_word32(get, &mut again, &mut mem).unwrap();
            assert_eq!(again, vec![want], "{get} {set} moved the text");
        }
    }

    #[test]
    fn screen_words_configure_the_newest_screen() {
        let mut mem = mem();
        let mut e = Engine::new(Profile::motion32());
        let mut stack = Vec::new();
        e.plain_word32("NEWSCREEN", &mut stack, &mut mem).unwrap();
        assert_eq!(stack, vec![1]);
        stack.clear();

        stack.extend([640, 400]);
        e.plain_word32("SCRSIZE", &mut stack, &mut mem).unwrap();
        stack.extend([0, 400]);
        e.plain_word32("SCRVPOS", &mut stack, &mut mem).unwrap();

        let s = &e.display.screens[0];
        assert_eq!(s.size, (640, 400));
        assert_eq!(s.view_pos, (0, 400));
        assert_eq!(s.buffer.width, 640);
    }
}
