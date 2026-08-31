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
mod clock;
mod curtain;
mod descriptor;
mod dialogue16;
mod dialogue32;
mod display;
mod draw;
mod error;
mod game;
mod geometry;
mod keys;
mod menu;
mod order;
mod persist;
mod request;
mod resources;
mod save;
mod screen;
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
pub use error::{Error, Result};
pub use game::Game;
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
pub use descriptor::{Descriptor, Placement, Shows, TextTemplate};

use crate::geometry::line_height;
use std::collections::BTreeMap;

use motionvm_motion_formats::TextTable;
use motionvm_motion_formats::font::{Font, FontRefTable};
use motionvm_render::Picture;
// The machine's own `Error` and `Result` are *not* imported here, and this
// crate's `Error` and `Result` — re-exported just above — are what the bare
// names mean throughout it. The two are different channels and the distinction
// is load-bearing: a kernel-word handler fails on the machine's terms, and
// every file under `words/` says so by importing `motionvm_motion_forth::Result`
// itself rather than picking up whatever a `use` in this file happens to name.
// The five signatures below that really do answer the machine spell it out.
use motionvm_motion_forth::Address;
use motionvm_render::Framebuffer;

/// A window slide the 16-bit `->SCRX`/`->SCRY` started; see [`Engine::scroll`].
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
    pub(crate) display: Display,
    pub(crate) descriptors: Vec<Descriptor>,
    pub(crate) selected: Option<usize>,
    pub(crate) templates: Vec<TextTemplate>,
    /// Fonts by the handle `+FONT` handed out.
    pub(crate) fonts: BTreeMap<i32, Font>,
    pub(crate) font_refs: Option<FontRefTable>,
    /// `000.FNT`, which a text descriptor uses when nothing chose a font.
    pub(crate) system_font: Option<Font>,
    pub(crate) sprites: BTreeMap<u32, Picture>,
    /// Video mode requested through `SETRES`.
    pub(crate) mode: i32,
    pub(crate) graphics: bool,
    next_descriptor: u32,
    next_font: i32,
    resources: Option<crate::resources::Resources>,
    /// Words that were reached but do nothing yet, with how often.
    pub(crate) stubbed: BTreeMap<String, usize>,
    /// Every fade that has been started, in order — see [`Fade`].
    pub(crate) fades: Vec<Fade>,
    /// `0xdbd64`: an offset queued for the next conversation step, spent when
    /// it is taken (0x7b644).
    dialog_offset: i32,
    /// `0xdbd60`: the answer node the player was last on, so that the node 4000
    /// can send the conversation back to it (0x7b675, 0x7b6a9).
    dialog_return: i32,
    /// The mouse pointer: sprite and hotspot, as `XATMOUSE` sets it.
    pub(crate) cursor: Option<(u32, i32, i32)>,
    pub(crate) pointer_visible: bool,
    /// The 16-bit pointer's show counter (`ds:0x16B4`): `SHOWMOUSE` adds
    /// one, `HIDEMOUSE` takes one, the pointer shows while it stands at one
    /// or more — and neither moves it before a shape armed the pointer
    /// (`ds:0x16AA`; both handlers leave without it, `14ee:0877`,
    /// `14ee:094e`). Zero at power-on: the pointer is invisible until the
    /// first `SHOWMOUSE` after `FATMOUSE`, which is why the intro shows
    /// none — `RUN` arms the shape before `STARTINTRO` but shows only
    /// after it.
    pub(crate) pointer_shows: i32,
    /// Whether this machine's `SHOWMOUSE`/`HIDEMOUSE` keep that counter —
    /// the read 16-bit behavior. The 32-bit pair is unread and keeps the
    /// plain on/off it always had here.
    pub(crate) pointer_counted: bool,
    /// The palette range `SETCYCLE` asked to cycle and the tick delay per
    /// step, or `None` where it asked for none. Held, not turned.
    pub(crate) palette_cycle: Option<(i32, i32, i32)>,
    /// The message box the game is waiting on, if it is waiting on one.
    pub(crate) request: Option<crate::request::Request>,
    /// Whether `?XINSIDE` passes over a hot area whose four corners are all
    /// zero. This is the build's and not the format's: `ENVIRO.EXE`
    /// (`0a40:1b37`) and `BMZ.EXE` test for it, `HPPLAY.EXE` and `LL.EXE` do
    /// not — 101 instructions against 71, and the older two have no `cmpw $0`
    /// in the handler at all. Set from the binary the game was opened with;
    /// the default is the 32-bit reading, where the area search treats a
    /// record of four zeros as a hole and `?XINSIDE` applies the same rule
    /// (the click dispatch at `0x7ce73` and its area walk).
    pub(crate) skips_holes: bool,
    /// Set when the game asks for a redraw. A still frame is composed on
    /// demand, so this only records that it was asked for.
    pub(crate) dirty: bool,
    /// Places a descriptor carrying `SDAUTOBUF` has left, waiting to be built
    /// again out of the descriptor list by the next drawing pass.
    ///
    /// The original remembers a copy of the picture instead and pastes it back
    /// (0x6ac33); see [`Descriptor::auto_buffer`].
    pub(crate) rebuild: Vec<(u32, (i32, i32, i32, i32))>,
    /// Whether a block descriptor — `SDBL`, a picture by the same id pool as
    /// a sprite — is copied onto its screen with every pixel, index 0
    /// included.
    ///
    /// The 16-bit drawer (`ENVIRO.EXE` `016a:0aac`) keeps the two kinds
    /// apart by bit 15 of the descriptor's picture cell and takes two paths:
    /// a sprite goes through the keyed blit at `14ee:0d1e`, a block through
    /// the plain copy at `14ee:0d47` — the one the save-under restores with.
    /// Die Enviro-Kids greifen ein builds its backgrounds out of 80-pixel
    /// block strips that are dark where they hold index 0, and drawing those
    /// keyed let the previous location show through. The 32-bit drawer's
    /// block path is not read on this point; the 32-bit game keeps the keyed
    /// blit it has always had.
    ///
    /// Read on the latest build and set for the whole 16-bit generation,
    /// which the earlier framing confirms rather than assumes: Victor Loomes'
    /// intro is eight full-screen block pictures, and all eight match a
    /// recording of the original pixel for pixel. A keyed block path would
    /// have shown the picture before through every index 0 in them.
    pub(crate) opaque_blocks: bool,
    /// Whether text is drawn and measured the run drawer's way
    /// (`ENVIRO.EXE` `016a:0aac` and `14ee:11cf`): the shadow pass shifted by
    /// the template's x/y offsets on an axis that is not centered, `#` eaten
    /// by the run drawer, `SDBLK` justifying a block, and the `GD*` sizes
    /// measured bare — where the 32-bit engine stores them padded by 4.
    ///
    /// Confirmed on the earlier framing the same way as
    /// [`Engine::opaque_blocks`]: three of the eight intro pictures Victor
    /// Loomes holds are text over a picture, and they match to the pixel.
    pub(crate) text_runs: bool,
    /// Whether `SDTB` allocates the text record itself, marking the
    /// descriptor as `SDTXT` would (`0x71d45`). Where it does not, the value
    /// is read as a block until `SDTXT` runs (`05f1:0d7b`).
    pub(crate) sdtb_allocates_text: bool,
    /// Whether `SDTDT` takes only templates 1..=20 (`05f1:0c78`: `cmp $1` /
    /// `jl`, `cmp $0x14` / `jg` skip the store and the dirty mark alike), so
    /// `0 SDTDT` cannot clear a template. The 32-bit handler has no such
    /// gate.
    pub(crate) templates_gated: bool,
    /// Whether `GDTB` answers a sprite with its bit 15 still on
    /// (`05f1:0d5c` reads `+0x10` and masks nothing). Only that machine
    /// keeps the marker in the value; the other has a type field of its own.
    pub(crate) table_marks_sprites: bool,
    /// Whether `SDLEV` re-inserts into the level chain even when the level
    /// does not change — the 16-bit engine's read behavior; see
    /// [`Descriptor::stamp`].
    pub(crate) level_chain: bool,
    /// The stamp counter behind [`Descriptor::stamp`].
    pub(crate) level_stamp: u64,
    /// Whether the `SD*` setters mark the descriptor dirty even when the
    /// value does not change — the 16-bit handlers write and set the bit
    /// unconditionally (`SDX` `05f1:0df2`, `SDFNT` `05f1:1038`, `SDLEV`,
    /// `SDNORM`), where the read 32-bit ones skip an unchanged value
    /// (`SDX` 0x7111a, `SDSPR` 0x71715, `SD%SHR` 0x721e5).
    pub(crate) sd_marks_always: bool,
    /// Whether a descriptor's `SDWORD` callback fires only while the
    /// descriptor is **active**. The 16-bit frame loop's callback walk
    /// (`016a:05d6`–`06da`) gates on `cb != -1 && (flags & 0x80) &&
    /// wait != -1` — flag bit 0x80 is `SDACTIVE` — and it runs with the
    /// descriptor made current (`DS:0x5de2`/`DS:0x3058`), so a skipped
    /// callback also leaves the script's own `SMDESC` selection
    /// standing. The intro leans on that: its `_DINFO` text descriptor
    /// carries `1082 SDWORD` from birth but stays `SDINACTIVE` through
    /// the motif phases, whose per-frame `SDH%SHR`/`SDSPR` writes come
    /// without a re-select.
    pub(crate) callbacks_need_active: bool,
    /// Whether descriptor handles are indices into the *active screen's*
    /// list, as the 16-bit kernel has them: `NEWDESC`/`NEWSETDESC` (`ENVIRO.EXE`
    /// file `0x9bb8`, `0x9bd8`) hand back the screen's count and raise it,
    /// `ACTDESC` (file `0x9b92`) stores the number and nothing else — the pair
    /// screen and number is resolved when a word touches the descriptor — and
    /// `KILLNDESC n` (file `0x9d3a`) frees the active screen's descriptors
    /// from `n` up and sets its count back to `n`. So the same number names
    /// one descriptor on each screen, and a number the scripts compute —
    /// `?LPD 1 +`, the slot after the permanent ones — means what it means
    /// on whichever screen is active. The 32-bit engine's handles are unique
    /// across screens and stay as they were.
    pub(crate) per_screen_descriptors: bool,
    /// Which game's savegame files this engine writes and reads.
    pub(crate) save_layout: save::Layout,
    /// A `->SCRX`/`->SCRY` scroll in flight — the 16-bit engine's blocking
    /// window slide, run here as a transition: one step a frame, the
    /// interpreter held, the way the fades are.
    pub(crate) scroll: Option<Scroll>,
    /// The number `ACTDESC` last stored, for the re-resolution a later
    /// `ACTSCR` does in the per-screen scheme.
    pub(crate) selected_handle: Option<u32>,
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
    /// How often this frame's bytecode has asked for the pointer or a key.
    ///
    /// The original's input words read live hardware, so a script may wait in
    /// a loop of its own — `RUN`'s start-up page does (`BEGIN … MOUSELK …
    /// MOUSEX … UNTIL`), location 5's newspaper does — and the loop turns as
    /// the player moves. Here a frame's input is fixed for the frame, so such
    /// a loop would spin forever. Past [`Engine::POLL_BUDGET`] polls in one
    /// frame the word is taken to be waiting, and from then on every poll
    /// yields the frame: the loop turns once per frame, with the frame's
    /// input. A frame of `CTRL` polls a handful of times and never gets
    /// near the budget.
    pub(crate) polls: u32,
    /// Set once a frame has spent its poll budget; every later poll in the
    /// same word yields. Cleared when the word has finished.
    pub(crate) polling: bool,
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
    /// Where savegames go, and the only directory anything here ever writes to.
    ///
    /// The original has no such notion: `PUT`, `=>PUTAS` and `PUTANIM` build
    /// their names from bare templates (`"#F0R3i.blk"` at 0xd4872 and its
    /// siblings) with no path component, so a save lands beside `ENGINE.EXE` —
    /// in among the game's own data. That is exactly what must not happen here,
    /// so the directory is explicit, it is separate, and [`Engine::set_saves`]
    /// refuses one that lies inside the game data.
    ///
    /// `None` means no saving: the reading words answer as if the slot were
    /// empty, and the writing words stop by name rather than pick a directory
    /// of their own.
    saves: Option<std::path::PathBuf>,
    /// Which module sits in each of the resource manager's descriptor slots.
    ///
    /// Bookkeeping only. Every module is loaded here from the start and none is
    /// ever evicted, so `=>GET` and `=>ERASE` have nothing to load or free —
    /// but *which* modules the original would have resident, and in what order,
    /// is not a detail: `=>PUTAS` writes exactly that set, in exactly that
    /// order, and a savegame that carried more would restore state the original
    /// throws away on every change of location.
    ///
    /// Slot 0 is the kernel's own `Basismodul` and never holds a game module;
    /// `=>PUTAS` starts its walk at 1, which is why the array is indexed the
    /// same way. Thirty-two slots, from `[0xDB027]`.
    ///
    /// The sequence is fixed by the bytecode, not guessed. `SYSTEM.RSC` is two
    /// lines, `4 =>GET` and `START`; `4:START` at 0x5420 does `2 5 6 11 13 3
    /// =>GET`, runs `STARTUP`, `3 =>ERASE`, `12 =>GET`, `DS_INIT`, `12
    /// =>ERASE`; and `5:INCLLOC` frees the old location's three modules before
    /// it takes the new one's, so those keep slots 7, 8 and 9 across every
    /// change of location. In a running game that comes out as
    /// `[4, 2, 5, 6, 11, 13, L+100, L+200, L+300]`.
    slots: [Option<u32>; 32],
    /// Every sprite `GFXVFLIP` has mirrored, as `(source, destination)`.
    ///
    /// The mirrored copy lands in the graphics pool under an id that no
    /// resource file contains, so it cannot be loaded again — a savegame has to
    /// carry the instruction to make it. Only the ids: the source is a real
    /// resource, so redoing the flip costs nothing and keeps a savegame from
    /// carrying pixels.
    flips: Vec<(u32, u32)>,
    /// Where the pointer is and which buttons are down.
    ///
    /// Every `MOUSE…` word fills the same four-field record at 0x253d4 and
    /// reads one field back out: x at +0, y at +4, the left button at +8 and
    /// the right at +12. So the mouse does reach the game through kernel
    /// words, not only through the `_MLK` and `_MRK` variables the location
    /// handler reads — `ICTRL` calls `MOUSELK` itself and stores the result.
    pub(crate) mouse: Mouse,
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
    /// The key waiting to be read, or 0 for none.
    ///
    /// The mouse reaches the game through module variables, because the native
    /// loop wrote them and no bytecode does. The keyboard does not: `?KEY` and
    /// its relatives are kernel words that asked the BIOS directly, so the
    /// state has to live here instead.
    pub(crate) key: i32,
    /// The keystrokes the game has not taken yet, oldest first — the BIOS
    /// type-ahead buffer `?KEY` reads through INT 16h, [`keys::SLOTS`] deep,
    /// drained one keystroke per frame the way the original's loop drains it.
    pub(crate) keys: std::collections::VecDeque<i32>,
    pub(crate) palettes: BTreeMap<i32, motionvm_render::Palette>,
    /// Text tables by resource id, as `SDTB` names them.
    pub(crate) texts: BTreeMap<i32, TextTable>,
    /// Transitions waiting to play, oldest first.
    ///
    /// A queue rather than a single one because a phase of the intro calls
    /// `FADEOUT`, swaps what is on the screen and calls `FADEIN` all within one
    /// invocation — the original blocks inside each handler, so both run in
    /// full before the phase is over. `DO_INVSEL` does it three deep: its
    /// documents branch fades the bar in, the picture out and the picture in
    /// again (module 4, 0x01c44–0x01ca8), and it can, because it runs as a
    /// descriptor callback through `m32::Vm::call_nested`, where the interpreter
    /// does not pause.
    pub(crate) curtains: std::collections::VecDeque<Curtain>,
    /// The 16-bit engine's transitions, queued the same way — box wipes,
    /// not band curtains; see [`Wipe`].
    pub(crate) wipes: std::collections::VecDeque<Wipe>,
    /// Where `STARTTUNE` sends its songs, if anywhere.
    pub(crate) music: Option<Box<dyn MusicSink>>,
    /// Handles are ours, counted from 1, the way descriptor handles are. The
    /// original answers with its SOS sequence handle; nothing in the game does
    /// anything with the number except hand it back to `ENDTUNE`.
    next_tune: i32,
    /// The off-screen buffers of the 16-bit kernel's `SETBUF`/`SDBUF`/`BUFON`
    /// family — see [`buffer::Buffers`]. Empty for the 32-bit game, whose
    /// `SETBUF` is inert.
    pub(crate) buffers: buffer::Buffers,
    /// The text backing's remap row, kept until the palette changes.
    backing: crate::draw::BackingCache,
    /// What is on the screen — the original's video memory.
    ///
    /// Kept rather than composed afresh every frame because the original keeps
    /// it: nothing reaches the visible screen except through the presenter
    /// (0x1457D), and the presenter only copies the tiles somebody marked. A
    /// curtain marks its bands and nothing else, so what it does not touch
    /// stays standing. See [`Self::present`] and [`Self::advance_curtain`].
    pub(crate) video: Framebuffer,
}

/// Words that genuinely have no effect in a silent, still-frame run.
///
/// This list exists because the alternative bit twice. A stub that pops and
/// pushes the right number of values but does nothing is indistinguishable from
/// a working word — until something far downstream reads the state it should
/// have written. `=>GET` left a stray value that became a jump address two
/// hundred steps later, and `GET` looked harmless while being the resource
/// loader the whole location system depends on.
///
/// So a word may only be inert if there is a reason, stated here. Anything else
/// is implemented or fails loudly with its name.
///
/// * `RESETTI` — timers, and a still frame has no time passing.
/// * The `…STAT`, `…STAT+` and `…STAT-` family, plain and `X`-prefixed — hints
///   about which resource ranges are wanted, may be dropped, or are pinned.
///   Nothing here evicts anything, so every one of them is inert. Their arities
///   are not uniform (`GFXSTAT+` takes three where its siblings take two, and
///   `XGFXSTAT+` four), which is why they are listed with counts rather than
///   handled by name pattern.
/// * `RESETANIM`, `RESETFONT` — reset subsystems that start out reset.
/// * `CUTPAL`, `SETBUF`, `RESETBUF` — palette and buffer bookkeeping for the
///   engine's double buffering, which a composed still frame does not use.
/// * `SDNORM`, `SDPOS` — descriptor modes with no argument whose effect only
///   shows in animation.
/// * `SDBLK` — sets bit 7 of the descriptor's byte +0x17 (its only writer, at
///   0x73099). That bit makes the output routine center the whole text block on
///   one width rather than each line on its own (0x25833). Nothing on the paths
///   walked so far calls it, so the behavior is unobserved; when something
///   does, this entry has to go and the two centering modes have to be built.
/// * `ERRORLEVEL` — sets the exit code DOS would report. There is no DOS here,
///   and nothing in the game reads it back.
/// * `DREQUEST` — hands three values to the diagnostic call at `0x5294d`
///   under category 0x1b, the same one `ANIMPLAY` uses to complain about being
///   entered twice. There is no debug channel here for it to reach.
/// * `SPEEDMODE` — writes a flag at `0xd6608`. All 356 kernel words were
///   scanned for that address and `SPEEDMODE` is the only one that touches it,
///   so no word can read it back; whatever consumes it lives in the native
///   loop, which is replaced here by a frame clock of our own.
/// * `GFXCRUNCH`, `XGFXCRUNCH` — read, not assumed: the routine behind them
///   (`0x59768`) walks a range of graphics and does nothing but `orb $8,7(%eax)`
///   or `andb $0xf7,7(%eax)` on a 22-byte record each. One bit, no pixels. The
///   sibling `GFXVFLIP` at `0x595be` does transform, which is why the two are
///   treated differently despite looking alike at the call site.
/// * `FADEIN`, `FADEOUT` **in mode 2** — mode 1 is implemented as a curtain;
///   the handlers' second branch has not been read, and nothing reaches it.
///
/// `SDINSERT` and `XATMOUSE` are deliberately absent: they do something.
///
/// `=>GET` and `=>ERASE` are deliberately *not* here, although they look like
/// candidates: they move no memory, because every module is resident from the
/// start, but they keep the descriptor-slot table that decides what a savegame
/// contains. Inert in what they load, load-bearing in what they record.
const NO_EFFECT: &[&str] = &[
    "RESETTI",
    "RESETANIM",
    "RESETFONT",
    "CUTPAL",
    "SETBUF",
    "RESETBUF",
    "SDBLK",
    "SDNORM",
    "SDPOS",
    "GFXCRUNCH",
    "XGFXCRUNCH",
    "ERRORLEVEL",
    "SPEEDMODE",
    "DREQUEST",
];

/// The resource status hints, with how many arguments each takes.
///
/// One word family per row — the plain, the `-` and the `+` form of each. The
/// layout is the table's index: reading down the first column lists the
/// families, reading across a row lists the three forms of one. Left to itself
/// rustfmt would set thirty-six entries one to a line and that is gone.
#[rustfmt::skip]
const STATUS_HINTS: &[(&str, usize)] = &[
    ("GFXSTAT", 2), ("GFXSTAT-", 2), ("GFXSTAT+", 3),
    ("XGFXSTAT", 3), ("XGFXSTAT-", 3), ("XGFXSTAT+", 4),
    ("BLKSTAT", 2), ("BLKSTAT-", 2), ("BLKSTAT+", 2),
    ("XBLKSTAT", 3), ("XBLKSTAT-", 3), ("XBLKSTAT+", 3),
    ("PALSTAT", 2), ("PALSTAT-", 2), ("PALSTAT+", 2),
    ("XPALSTAT", 3), ("XPALSTAT-", 3), ("XPALSTAT+", 3),
    ("TXTSTAT", 2), ("TXTSTAT-", 2), ("TXTSTAT+", 2),
    ("XTXTSTAT", 3), ("XTXTSTAT-", 3), ("XTXTSTAT+", 3),
    ("FNTSTAT", 2), ("FNTSTAT-", 2), ("FNTSTAT+", 2),
    ("XFNTSTAT", 3), ("XFNTSTAT-", 3), ("XFNTSTAT+", 3),
    ("SCRSTAT", 1), ("SCRSTAT-", 2), ("SCRSTAT+", 2),
    ("XSCRSTAT", 3), ("XSCRSTAT-", 3), ("XSCRSTAT+", 3),
];

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

/// The numbers the three video-mode words push.
///
/// Read at the handlers, not inferred: each word is four instructions that
/// push a small ordinal — 1, 2 and 4 at 0x6f023, 0x6f047 and 0x6f06b. Packing
/// width and height into a word is the plausible guess, on the reasoning that
/// only `SETRES` ever sees them again, and nothing in the game would catch it:
/// `SETRES` only stores the value.
const MODE_320X200X256: i32 = 1;
const MODE_640X480X256: i32 = 2;
const MODE_640X480X32K: i32 = 4;

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
        self.wipes
            .iter()
            .rev()
            .find_map(|w| w.palette_after.as_ref())
            .or_else(|| {
                self.curtains
                    .iter()
                    .rev()
                    .find_map(|c| c.palette_after.as_ref())
            })
            .unwrap_or(&self.display.palette)
    }

    /// The size of the composed picture.
    pub fn display_size(&self) -> (u16, u16) {
        self.display.size
    }

    /// The off-screen buffers a 16-bit game has asked for.
    pub fn buffers(&self) -> &buffer::Buffers {
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
        &self.descriptors
    }

    /// The text templates `DEFTDT` has defined.
    pub fn templates(&self) -> &[TextTemplate] {
        &self.templates
    }

    /// Fonts by the handle `+FONT` handed out.
    pub fn fonts(&self) -> &BTreeMap<i32, Font> {
        &self.fonts
    }

    /// The glyph reference table, once a font has been loaded.
    pub fn font_refs(&self) -> Option<&FontRefTable> {
        self.font_refs.as_ref()
    }

    /// `000.FNT`, which a text descriptor uses when nothing chose a font.
    pub fn system_font(&self) -> Option<&Font> {
        self.system_font.as_ref()
    }

    /// A sprite that has already been decoded, without decoding one.
    ///
    /// Deliberately not the loading path: that one is `&mut self` because a
    /// load can set the palette. See the note on `Engine::sprite`.
    pub fn cached_sprite(&self, id: u32) -> Option<&Picture> {
        self.sprites.get(&id)
    }

    /// Every fade that has been started, in order — see [`Fade`].
    ///
    /// Instrumentation: the engine never reads it back. It is here because a
    /// fade is otherwise invisible to a test, being over before a frame is
    /// asked for.
    pub fn fades(&self) -> &[Fade] {
        &self.fades
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

    /// Words that were reached but do nothing yet, with how often.
    ///
    /// The other half of the instrumentation. A run can be asked afterwards
    /// what it walked past; the inert words are listed on `NO_EFFECT`.
    pub fn stubbed(&self) -> &BTreeMap<String, usize> {
        &self.stubbed
    }

    /// The pointer's shape and hotspot, as `XATMOUSE` set them.
    pub fn cursor(&self) -> Option<(u32, i32, i32)> {
        self.cursor
    }

    /// Whether the pointer is drawn.
    pub fn pointer_visible(&self) -> bool {
        self.pointer_visible
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
        self.music = Some(sink);
    }

    /// Takes one press into the keyboard buffer, translated on the way in.
    ///
    /// The buffer is the BIOS type-ahead buffer `?KEY` reads through INT 16h:
    /// fifteen keystrokes deep, so a sixteenth is dropped here exactly as the
    /// original dropped it. A press the PC has no answer for — a modifier on
    /// its own, a media key — is never enqueued, because it never reached the
    /// BIOS buffer either.
    pub fn push_key(&mut self, press: &motionvm_playable::KeyPress) {
        if self.keys.len() < keys::SLOTS
            && let Some(code) = keys::code(press)
        {
            self.keys.push_back(code);
        }
    }

    /// Takes the oldest waiting keystroke out of the buffer, or 0 for none —
    /// `?KEY`'s own answer when nothing waits.
    pub fn pop_key(&mut self) -> i32 {
        self.keys.pop_front().unwrap_or(0)
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
        self.descriptors.push(d);
        handle
    }

    /// The descriptor list, to be changed in place.
    pub fn descriptors_mut(&mut self) -> &mut [Descriptor] {
        &mut self.descriptors
    }

    /// Adds a ready-made screen.
    pub fn add_screen(&mut self, screen: Screen) {
        self.display.screens.push(screen);
    }

    /// Puts a sprite in the graphics cache under an id, without a resource file.
    pub fn cache_sprite(&mut self, id: u32, sprite: Picture) {
        self.sprites.insert(id, sprite);
    }
}

impl Engine {
    /// An engine with nothing loaded — no resources, no screens, no
    /// descriptors — and a display of the given size: 640×480 for the 32-bit
    /// engine's game, 320×200 for the 16-bit engine's, whose `TOGFX` enters
    /// that mode and which has no `SETRES`.
    ///
    /// Usable on its own for anything that does not need the game's files —
    /// the descriptor list, the wait arithmetic, the curtain queue. A game
    /// comes from [`Game::open`], which builds one of these and points it at a
    /// directory. This is the only constructor: a display size belongs to a
    /// generation, so whoever builds an engine says which one they mean.
    pub fn with_display(width: u16, height: u16) -> Self {
        Self {
            display: Display::with_size(width, height),
            descriptors: Vec::new(),
            selected: None,
            templates: Vec::new(),
            fonts: BTreeMap::new(),
            font_refs: None,
            system_font: None,
            sprites: BTreeMap::new(),
            mode: 0,
            graphics: false,
            next_descriptor: 1,
            next_font: 1,
            resources: None,
            saves: None,
            slots: [None; 32],
            flips: Vec::new(),
            stubbed: BTreeMap::new(),
            fades: Vec::new(),
            dialog_offset: 0,
            dialog_return: 0,
            cursor: None,
            pointer_visible: true,
            pointer_shows: 0,
            pointer_counted: false,
            palette_cycle: None,
            request: None,
            // The 32-bit handler's own walk tests for it; the 16-bit opener
            // overrides this from the build it read.
            skips_holes: true,
            dirty: false,
            rebuild: Vec::new(),
            controller: None,
            main_loop: false,
            entering_loop: false,
            polls: 0,
            polling: false,
            // The 32-bit `START`'s `25 DELAY`; a 16-bit game's own `DELAY`
            // overwrites it during startup.
            frame_ticks: 8,
            dir: None,
            last_info_mode: None,
            system_fg: 0,
            system_bg: 2,
            pending_call: None,
            opaque_blocks: false,
            text_runs: false,
            sdtb_allocates_text: true,
            templates_gated: false,
            table_marks_sprites: false,
            level_chain: false,
            level_stamp: 0,
            sd_marks_always: false,
            callbacks_need_active: false,
            per_screen_descriptors: false,
            save_layout: save::Layout::Motion32,
            scroll: None,
            selected_handle: None,
            step_multi: 1,
            key: 0,
            keys: std::collections::VecDeque::with_capacity(keys::SLOTS),
            mouse: Mouse::default(),
            palettes: BTreeMap::new(),
            texts: BTreeMap::new(),
            curtains: std::collections::VecDeque::new(),
            wipes: std::collections::VecDeque::new(),
            music: None,
            next_tune: 1,
            buffers: buffer::Buffers::default(),
            backing: crate::draw::BackingCache::default(),
            video: Framebuffer::new(width, height),
        }
    }

    /// Records a word that ran but deliberately did nothing.
    ///
    /// Only for words on [`NO_EFFECT`]. Everything else must either be
    /// implemented or fail — see that list's documentation for why.
    pub(crate) fn note_no_effect(&mut self, name: &str) {
        debug_assert!(
            NO_EFFECT.contains(&name),
            "{name} was treated as inert but is not on the NO_EFFECT list"
        );
        self.note_unhandled(name.to_string());
    }

    /// A word that pops its arguments and does nothing else.
    ///
    /// The two halves belong together and were written out four times: take
    /// the arity off the stack, then count the word as inert. Saying it once
    /// puts the contract in one place — `name` must be on [`NO_EFFECT`], and
    /// [`Engine::note_no_effect`] asserts it in a debug build.
    pub(crate) fn inert(
        &mut self,
        stack: &mut Vec<i32>,
        arity: usize,
        name: &'static str,
    ) -> motionvm_motion_forth::Result<()> {
        crate::stack::pop_n(stack, arity, name)?;
        self.note_no_effect(name);
        Ok(())
    }

    /// Records a *case* that was reached and is not built.
    ///
    /// Different from [`Engine::note_no_effect`], which is about a word being
    /// inert as a whole. This is about one argument reaching a word that
    /// handles the rest of them: `FADEIN` with mode 2, `SCRX` with something
    /// other than zero. The word is implemented; that case is not, and it is
    /// counted so a run can be asked afterwards whether it happened.
    ///
    /// Separate from that function rather than sharing it, because its
    /// `debug_assert!` checks the name against `NO_EFFECT`. The strings here
    /// are cases, not words — `SCRX (non-zero)` can never be on that list — so
    /// routing them through it kills a debug build on ordinary game data.
    /// Polls of the pointer or a key one frame may make before the word is
    /// taken to be waiting for input; see [`Engine::polls`].
    pub(crate) const POLL_BUDGET: u32 = 256;

    /// One poll of the pointer or the key buffer by the bytecode.
    pub(crate) fn polled(&mut self) {
        self.polls += 1;
        if self.polls > Self::POLL_BUDGET {
            self.polling = true;
        }
    }

    /// Whether the running word should yield the frame because it is
    /// polling for input that cannot change until the next one.
    fn poll_yield(&mut self) -> bool {
        if self.polling && self.polls > 0 {
            // Spent: the next poll after the resume counts afresh.
            self.polls = 0;
            return true;
        }
        false
    }

    fn note_unhandled(&mut self, what: String) {
        *self.stubbed.entry(what).or_default() += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{DESCRIPTOR_FIELDS, DESCRIPTOR_SETTERS};
    use motionvm_motion_forth::m32;

    /// Every one-argument setter is also a field a savegame may carry.
    ///
    /// The two tables are separate because `INSERT` is a field and not a
    /// setter; this is what keeps that the only difference between them.
    #[test]
    fn every_descriptor_setter_is_a_descriptor_field() {
        for s in DESCRIPTOR_SETTERS {
            assert!(
                DESCRIPTOR_FIELDS.contains(s),
                "{s} sets a field a savegame cannot name"
            );
        }
    }

    /// Module memory the tests can hand to the host; none of them write to it.
    fn mem() -> m32::Memory {
        m32::Memory::default()
    }

    #[test]
    fn newsetdesc_takes_six_and_returns_a_handle() {
        let mut mem = mem();
        let mut e = Engine::with_display(640, 480);
        // x y lev spr 0 0 — the shape XYLSITEM. builds.
        let mut stack = vec![10, 20, 3, 99, 0, 0];
        assert!(e.plain_word32("NEWSETDESC", &mut stack, &mut mem).unwrap());
        assert_eq!(stack, vec![1], "should leave just the handle");
        let d = &e.descriptors[0];
        assert_eq!((d.x, d.y, d.level), (10, 20, 3));
    }

    #[test]
    fn descriptor_setters_reach_the_selected_descriptor() {
        let mut mem = mem();
        let mut e = Engine::with_display(640, 480);
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
        let mut e = Engine::with_display(640, 480);
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
            let mut e = Engine::with_display(640, 480);
            e.opaque_blocks = true;
            let mut s = Screen::new(1);
            s.size = (4, 4);
            s.view = (4, 4);
            s.buffer = Framebuffer::new(4, 4);
            s.active = true;
            e.display.screens.push(s);
            for (id, color) in [(7u32, 0u8), (8, 9)] {
                e.sprites.insert(
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
                    e.set_block(id as i32).expect("SDBL");
                } else {
                    e.set_sprite(id as i32).expect("SDSPR");
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
        let mut e = Engine::with_display(640, 480);
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
            e.sprites.insert(
                id,
                Picture {
                    width: 16,
                    height: 32,
                    pixels: vec![color; 16 * 32],
                },
            );
        }
        for (handle, screen, sprite) in [(1u32, 1u32, 10u32), (2, 2, 11)] {
            e.descriptors.push(Descriptor {
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
        let mut e = two_screens(3, 7);
        e.level_chain = true;
        // The fixture wires its screens by hand; the damage map behind the
        // marks `SDLEV` makes has to exist for this path.
        for s in &mut e.display.screens {
            let (w, h) = s.view;
            s.set_view(w, h);
        }
        // A second sprite on screen 1, same level as the first: creation
        // order paints it on top.
        e.sprites.insert(
            12,
            Picture {
                width: 16,
                height: 32,
                pixels: vec![5; 16 * 32],
            },
        );
        let stamp = e.next_stamp();
        e.descriptors.push(Descriptor {
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
        e.selected = e.descriptors.iter().position(|d| d.handle == 1);
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
        let mut e = two_screens(3, 7);
        for s in &mut e.display.screens {
            let (w, h) = s.view;
            s.set_view(w, h);
        }
        let i = e
            .descriptors
            .iter()
            .position(|d| d.handle == 1)
            .expect("the fixture's descriptor");
        e.selected = Some(i);
        let (x, mode) = (e.descriptors[i].x, e.descriptors[i].x_mode);
        e.draw();
        assert!(!e.descriptors[i].dirty, "drawing settles the descriptor");

        e.place_x(x, mode).expect("SDX");
        assert!(
            !e.descriptors[i].dirty,
            "the 32-bit setter skips an unchanged value"
        );
        e.sd_marks_always = true;
        e.place_x(x, mode).expect("SDX");
        assert!(
            e.descriptors[i].dirty,
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
        let mut e = two_screens(3, 7);
        for s in &mut e.display.screens {
            let (w, h) = s.view;
            s.set_view(w, h);
        }
        e.templates_gated = true;
        e.selected = e.descriptors.iter().position(|d| d.handle == 1);
        e.set_template(2).expect("SDTDT");
        for outside in [0, -1, 21] {
            e.set_template(outside).expect("SDTDT");
            assert_eq!(
                e.descriptors[e.selected.unwrap()].template,
                Some(2),
                "{outside} SDTDT left the template standing"
            );
        }
        e.set_template(9).expect("SDTDT");
        assert_eq!(e.descriptors[e.selected.unwrap()].template, Some(9));
    }

    /// Repaints screen 1's descriptor in `color`, ready for a fade to reveal.
    fn repaint(e: &mut Engine, color: u8) {
        let sprite = e.sprites.get_mut(&10).expect("the top sprite");
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
            e.curtains.len(),
            2,
            "both curtains are queued before either plays"
        );

        let mut closing = true;
        while e.in_transition() {
            if closing && e.curtains.front().map(|c| c.opening) == Some(true) {
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
            e.curtains.front().expect("a curtain").area.2,
            16,
            "a fade in spans SCRSIZE"
        );
        e.curtains.clear();

        fade(&mut e, &mut mem, "FADEOUT", 1);
        assert_eq!(
            e.curtains.front().expect("a curtain").area.2,
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
        e.sprites.insert(
            99,
            Picture {
                width: 2,
                height: 2,
                pixels: vec![9; 4],
            },
        );
        e.cursor = Some((99, 0, 0));
        e.pointer_visible = true;
        e.mouse.x = 0;
        e.mouse.y = 0;
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
        assert!(e.pointer_visible, "the script's own state is left alone");
    }

    /// Mode 2 is a different effect, and asking for it must not bring us down.
    ///
    /// It is a *translucent* fade: 0x74eef fills its bands with color 0x102,
    /// which the fill routine reads as a level in the darkening tables
    /// `SETPAL` builds (0x147ff and its siblings) rather than as a color. All
    /// 180 calls in the game pass mode 1, so it is recorded and skipped. It
    /// must not go through `note_no_effect`, whose `debug_assert!` fires for
    /// any name not on `NO_EFFECT`, and neither fade word is on that list.
    #[test]
    fn a_mode_two_fade_is_recorded_rather_than_fatal() {
        let mut mem = mem();
        let mut e = two_screens(3, 7);
        e.display.set_current(1);
        let mut stack = vec![2, 50, 8];
        e.plain_word32("FADEIN", &mut stack, &mut mem).unwrap();
        assert!(!e.in_transition(), "mode 2 is not the curtain");
        assert_eq!(
            e.stubbed.get("FADEIN (mode 2)"),
            Some(&1),
            "and it is reported, not swallowed"
        );
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
            let mut e = Engine::with_display(640, 480);
            let mut s = Screen::new(1);
            s.size = (640, height as u16);
            s.view = (640, height as u16);
            s.buffer = Framebuffer::new(640, height as u16);
            e.display.screens.push(s);
            e.display.set_current(1);

            let mut stack = vec![1, 50, 8];
            e.plain_word32("FADEOUT", &mut stack, &mut mem).unwrap();
            let c = e.curtains.front().expect("a curtain").clone();
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
        let mut e = Engine::with_display(640, 480);
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
        let mut e = Engine::with_display(640, 480);
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
        let mut e = Engine::with_display(640, 480);
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
