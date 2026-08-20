//! The game runtime behind the VM's [`Host`] trait: screens, descriptors,
//! fonts and resources.
//!
//! Every word here has its arity taken from the original handler — the count
//! of argument fetches it makes — rather than from how call sites look. That
//! distinction matters: `NEWSETDESC` reads as a three-argument word everywhere
//! it is used and actually takes six, because its callers leave the first three
//! on the stack. An arity guessed from call sites produces nothing but stack
//! underflows.

mod clock;
mod curtain;
mod descriptor;
mod dialogue;
mod draw;
mod game;
mod geometry;
mod menu;
mod order;
mod persist;
mod resources;
mod save;
mod screen;
mod stack;
mod walk;
mod words;
pub use game::Game;
// Re-exported rather than moved out of sight: `Descriptor` and its enums are
// part of what a caller reads off the engine, and the app, the tools and
// sixteen test files name them. Where they are defined is this crate's
// business; that they are here is everyone else's.
pub use curtain::{Curtain, Fade};
pub use descriptor::{Descriptor, DescriptorKind, Placement, TextTemplate};

use crate::geometry::line_height;
use std::collections::BTreeMap;

use motionvm_formats::font::{Font, FontRefTable};
use motionvm_formats::{Sprite, TextTable, rsc::Bank};
use motionvm_forth::{Address, Error, Host, Memory, Result, Vm};
use motionvm_render::{Display, Framebuffer};

/// Where the music goes.
///
/// A trait rather than a concrete player because the two callers want opposite
/// things: the window hands the commands to an audio thread, and a test wants
/// to see what was asked for without a sound card in the room. `None` is the
/// third case and the faithful one — the original's words answer 0 when sound
/// never initialized.
pub trait MusicSink {
    /// `handle` is what `STARTTUNE` answered with; `song` is the whole block,
    /// exactly as the original hands the file to its MIDI layer (`0x853B0`).
    fn start(&mut self, handle: i32, tune: i32, looping: bool, song: &[u8]);
    /// Stops the song that `handle` was answered for.
    fn stop(&mut self, handle: i32);
}

/// The game runtime: everything the bytecode's words act on.
///
/// One value holds the whole of it — screens, descriptors, fonts, palettes,
/// texts, sprites, the conversation cursor, the transition queue, the pointer,
/// the clock. There is **no global mutable state anywhere in this workspace**;
/// state is threaded through `&mut Engine` and `&mut Vm`, which is what makes
/// two games in one process, or a test that builds a scene by hand, ordinary
/// rather than delicate.
///
/// It implements [`Host`], so from the interpreter's side it is simply the
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
    pub(crate) sprites: BTreeMap<u32, Sprite>,
    /// Video mode requested through `SETRES`.
    pub(crate) mode: i32,
    pub(crate) graphics: bool,
    next_descriptor: u32,
    next_font: i32,
    bank: Option<Bank>,
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
    /// Set when the game asks for a redraw. A still frame is composed on
    /// demand, so this only records that it was asked for.
    pub(crate) dirty: bool,
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
    /// it is -1 (0x68fbd to 0x68fd4). `DELAY n` puts `200/n` here, and `START`
    /// asks for `25 DELAY`, so eight.
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
    /// A bytecode word a primitive asked to have called after it. See
    /// [`motionvm_forth::Host::pending_call`].
    pending_call: Option<(Address, Vec<i32>)>,
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
    pub(crate) palettes: BTreeMap<i32, motionvm_formats::Palette>,
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
    /// descriptor callback through `Vm::call_nested`, where the interpreter
    /// does not pause.
    pub(crate) curtains: std::collections::VecDeque<Curtain>,
    /// Where `STARTTUNE` sends its songs, if anywhere.
    pub(crate) music: Option<Box<dyn MusicSink>>,
    /// Handles are ours, counted from 1, the way descriptor handles are. The
    /// original answers with its SOS sequence handle; nothing in the game does
    /// anything with the number except hand it back to `ENDTUNE`.
    next_tune: i32,
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
///   does, this entry has to go and the two centring modes have to be built.
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
pub struct Mouse {
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

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// What the engine shows the outside world.
///
/// Every one of these answers with a borrow and none of them lets a caller
/// write. That is not a new rule: a survey of the frontend, the tools and the
/// sixteen test files found **not one** write to an engine field from outside
/// this crate — only [`Engine::set_music`], which installs the sink. The
/// accessors make the compiler keep a promise the code was already keeping.
///
/// It matters more here than in most programs. This is a reimplementation of a
/// 1996 engine, and the property worth enforcing is that game state changes
/// only the way the game changes it — through the kernel words the bytecode
/// runs. A frontend that could reach in and set a descriptor's position would
/// be able to produce a picture the original never could, and nothing would
/// report it.
///
/// The tools crate reads a great deal through here, and that is what it is
/// for: it is the inspector, and inspection is reading.
impl Engine {
    /// The palette in force. Everything drawn is indices into it.
    pub fn palette(&self) -> &motionvm_formats::Palette {
        &self.display.palette
    }

    /// The screens, in the order they were created.
    pub fn screens(&self) -> &[motionvm_render::Screen] {
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
    pub fn cached_sprite(&self, id: u32) -> Option<&Sprite> {
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
}

/// Building engine state directly, which the game never does.
///
/// The game makes descriptors with `NEWSETDESC`, screens with `NEWSCREEN` and
/// loads sprites by drawing them. These four exist because a test that checks
/// one behavior should not have to play the game up to the point that produces
/// it — `tick_descriptor`'s wait arithmetic needs one descriptor and no game at
/// all — and because `motionvm-tools` builds scenes to hold against the
/// original.
///
/// **This is the deliberate hole in the facade, and it is named so it can be
/// found.** Everything else outside this crate can only read; a frontend that
/// reached in here would show up in a search for these four names, where a
/// `pub` field would have shown up nowhere. The frontend does not use them, and
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
    pub fn add_screen(&mut self, screen: motionvm_render::Screen) {
        self.display.screens.push(screen);
    }

    /// Puts a sprite in the graphics cache under an id, without a resource file.
    pub fn cache_sprite(&mut self, id: u32, sprite: Sprite) {
        self.sprites.insert(id, sprite);
    }
}

impl Engine {
    /// An engine with nothing loaded: no resources, no screens, no descriptors.
    ///
    /// Usable on its own for anything that does not need the game's files —
    /// the descriptor list, the wait arithmetic, the curtain queue. A game
    /// comes from [`Game::open`], which builds one of these and points it at a
    /// directory.
    pub fn new() -> Self {
        Self {
            display: Display::new(),
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
            bank: None,
            saves: None,
            slots: [None; 32],
            flips: Vec::new(),
            stubbed: BTreeMap::new(),
            fades: Vec::new(),
            dialog_offset: 0,
            dialog_return: 0,
            cursor: None,
            pointer_visible: true,
            dirty: false,
            controller: None,
            main_loop: false,
            entering_loop: false,
            // What `START` asks for: `25 DELAY`, so 200/25 = 8.
            frame_ticks: 8,
            dir: None,
            last_info_mode: None,
            pending_call: None,
            step_multi: 1,
            key: 0,
            mouse: Mouse::default(),
            palettes: BTreeMap::new(),
            texts: BTreeMap::new(),
            curtains: std::collections::VecDeque::new(),
            music: None,
            next_tune: 1,
            backing: crate::draw::BackingCache::default(),
            video: Framebuffer::new(motionvm_render::DISPLAY_W, motionvm_render::DISPLAY_H),
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
    ) -> Result<()> {
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
    fn note_unhandled(&mut self, what: String) {
        *self.stubbed.entry(what).or_default() += 1;
    }
}

impl Host for Engine {
    /// Stops the interpreter while a transition plays, which is what the
    /// original does by never returning from `FADEOUT` until it is finished.
    fn pending_call(&mut self) -> Option<(Address, Vec<i32>)> {
        self.pending_call.take()
    }

    fn wants_pause(&mut self) -> bool {
        self.in_transition() || self.entering_loop
    }

    fn word(&mut self, name: &str, vm: &mut Vm) -> Result<bool> {
        // The interaction machine runs bytecode of its own and therefore needs
        // the machine, not just its stack and memory.
        if name == "DOORDER" {
            return self.do_order(vm);
        }
        let Vm { data, mem, .. } = vm;
        self.plain_word(name, data, mem)
    }
}

impl Engine {
    /// Runs one kernel word by name, with its arguments on `stack`.
    ///
    /// Every word that does not need the machine itself — which is all of them
    /// so far. A word that has to run bytecode of its own gets `&mut Vm` in
    /// [`Host::word`] above and is handled there instead.
    ///
    /// Public so a test can drive the same words the bytecode does — loading a
    /// savegame, for instance, is `GET`, `INCLLOC`, `GETANIM` and `=>GETAS` in
    /// that order, and reproducing a reported state is far quicker from a
    /// savegame than from an hour of play.
    ///
    /// **The order of these calls is the order of the original's own match, and
    /// it has to stay that way.** Two of the arms match on table membership
    /// rather than on a literal — the descriptor setters and the resource
    /// status hints — so where they sit decides what reaches them. A duplicate
    /// of the second once sat earlier in the match, won silently, and faulted
    /// debug builds. Splitting the match into files is exactly the change that
    /// could bring that back.
    pub fn plain_word(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut Memory,
    ) -> Result<bool> {
        if self.words_state(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_screens(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_descriptors(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_text(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_resources(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_transitions(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_input(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_inventory(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_dialogue(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_sound(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_pointer(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_palette(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_redraw(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{DESCRIPTOR_FIELDS, DESCRIPTOR_SETTERS};

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
    fn mem() -> Memory {
        Memory::default()
    }

    #[test]
    fn newsetdesc_takes_six_and_returns_a_handle() {
        let mut mem = mem();
        let mut e = Engine::new();
        // x y lev spr 0 0 — the shape XYLSITEM. builds.
        let mut stack = vec![10, 20, 3, 99, 0, 0];
        assert!(e.plain_word("NEWSETDESC", &mut stack, &mut mem).unwrap());
        assert_eq!(stack, vec![1], "should leave just the handle");
        let d = &e.descriptors[0];
        assert_eq!((d.x, d.y, d.level), (10, 20, 3));
    }

    #[test]
    fn descriptor_setters_reach_the_selected_descriptor() {
        let mut mem = mem();
        let mut e = Engine::new();
        let mut stack = vec![0, 0, 0, 0, 0, 0];
        e.plain_word("NEWSETDESC", &mut stack, &mut mem).unwrap();
        stack.clear();

        stack.push(42);
        e.plain_word("SDX", &mut stack, &mut mem).unwrap();
        assert!(stack.is_empty(), "SDX takes exactly one value");
        e.plain_word("GDX", &mut stack, &mut mem).unwrap();
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
        };
        while !c.done() {
            c.advance(1);
            steps += 1;
        }
        assert_eq!(steps, 31, "480 rows in steps of eight from the middle out");
    }

    /// Two screens tiling the display, each showing one flat color.
    ///
    /// Small on purpose, but not smaller than the effect: the band height is a
    /// literal 8, so a screen has to be at least a few bands tall for a fade to
    /// have a middle at all. Thirty-two rows is three steps a direction.
    fn two_screens(top: u8, bottom: u8) -> Engine {
        let mut e = Engine::new();
        for (handle, y) in [(1u32, 0i16), (2, 32)] {
            let mut s = motionvm_render::Screen::new(handle);
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
                Sprite {
                    width: 16,
                    height: 32,
                    palette: motionvm_formats::Palette::from_6bit(&[]),
                    pixels: vec![color; 16 * 32],
                },
            );
        }
        for (handle, screen, sprite) in [(1u32, 1u32, 10u32), (2, 2, 11)] {
            e.descriptors.push(Descriptor {
                handle,
                screen,
                sprite: Some(sprite),
                active: true,
                ..Default::default()
            });
        }
        e.draw();
        e.present();
        e
    }

    /// Repaints screen 1's descriptor in `color`, ready for a fade to reveal.
    fn repaint(e: &mut Engine, color: u8) {
        let sprite = e.sprites.get_mut(&10).expect("the top sprite");
        sprite.pixels.fill(color);
    }

    fn fade(e: &mut Engine, mem: &mut Memory, name: &str, screen: u32) {
        e.display.set_current(screen);
        let mut stack = vec![1, 50, 8];
        e.plain_word(name, &mut stack, mem).unwrap();
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
            Sprite {
                width: 2,
                height: 2,
                palette: motionvm_formats::Palette::from_6bit(&[]),
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
        e.plain_word("FADEIN", &mut stack, &mut mem).unwrap();
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
            let mut e = Engine::new();
            let mut s = motionvm_render::Screen::new(1);
            s.size = (640, height as u16);
            s.view = (640, height as u16);
            s.buffer = Framebuffer::new(640, height as u16);
            e.display.screens.push(s);
            e.display.set_current(1);

            let mut stack = vec![1, 50, 8];
            e.plain_word("FADEOUT", &mut stack, &mut mem).unwrap();
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
        let mut e = Engine::new();
        for x in [1, 2] {
            let mut s = vec![x, 0, 0, 0, 0, 0];
            e.plain_word("NEWSETDESC", &mut s, &mut mem).unwrap();
        }
        let mut stack = vec![1]; // the first descriptor's handle
        e.plain_word("ACTDESC", &mut stack, &mut mem).unwrap();
        let mut out = Vec::new();
        e.plain_word("GDX", &mut out, &mut mem).unwrap();
        assert_eq!(out, vec![1], "should have selected the first descriptor");

        // A handle that names nothing leaves the selection alone. The original
        // resolves it first and jumps past both stores when that fails
        // (0x71618 → 0x71624 → 0x71636), so the descriptor that was selected
        // stays selected and the `SD…` words that follow still land.
        let mut stack = vec![999];
        e.plain_word("ACTDESC", &mut stack, &mut mem).unwrap();
        let mut out = Vec::new();
        e.plain_word("GDX", &mut out, &mut mem).unwrap();
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
    fn centring_a_text_round_trips_through_gdcx() {
        let mut mem = mem();
        let mut e = Engine::new();
        let mut stack = vec![0, 0, 0, 0, 0, 0];
        e.plain_word("NEWSETDESC", &mut stack, &mut mem).unwrap();
        stack.clear();

        // A text descriptor, centered on a point, with a measurable extent. No
        // font is loaded, so the measurement is whatever `extent` makes of an
        // empty text — the identity has to hold for any width, not a lucky one.
        for (word, v) in [("SDTB", 8), ("SDTXT", 1), ("SDCEN", 200), ("SDVCEN", 100)] {
            stack.push(v);
            e.plain_word(word, &mut stack, &mut mem).unwrap();
        }

        for (get, set, want) in [("GDCX", "SDCEN", 200), ("GDCY", "SDVCEN", 100)] {
            let mut out = Vec::new();
            e.plain_word(get, &mut out, &mut mem).unwrap();
            assert_eq!(
                out,
                vec![want],
                "{get} must answer with the point it was centered on"
            );
            e.plain_word(set, &mut out, &mut mem).unwrap();
            let mut again = Vec::new();
            e.plain_word(get, &mut again, &mut mem).unwrap();
            assert_eq!(again, vec![want], "{get} {set} moved the text");
        }
    }

    #[test]
    fn screen_words_configure_the_newest_screen() {
        let mut mem = mem();
        let mut e = Engine::new();
        let mut stack = Vec::new();
        e.plain_word("NEWSCREEN", &mut stack, &mut mem).unwrap();
        assert_eq!(stack, vec![1]);
        stack.clear();

        stack.extend([640, 400]);
        e.plain_word("SCRSIZE", &mut stack, &mut mem).unwrap();
        stack.extend([0, 400]);
        e.plain_word("SCRVPOS", &mut stack, &mut mem).unwrap();

        let s = &e.display.screens[0];
        assert_eq!(s.size, (640, 400));
        assert_eq!(s.view_pos, (0, 400));
        assert_eq!(s.buffer.width, 640);
    }
}
