//! `XP00`, the title program of a parsec1994 game: what every title's is
//! built from.
//!
//! The batch file runs `xp00.exe`, and `xp01.exe` only when it left
//! errorlevel 0. Between the two programs there is nothing but that exit
//! code and `SAVEGAME.SAV`: a new game is one whose saved game the title
//! program deleted. Each title's `XP00` is its own program — its own order
//! of pictures, its own menu lines, its own extras — linked against the
//! same library as `XP01`: Captain Zins's shares 256 functions byte for
//! byte with Dunkle Schatten's and 94 more but for their addresses. What
//! that library does is here: the machine the program runs on ([`Lib`]:
//! the screen, the DAC, the clock, the pointer, the poll), the fades, the
//! intro every title program plays ([`intro`]), the menu routine each is
//! built around ([`menu`]) and the roll of credits ([`roll`]); what the program
//! around them does is the title's (`titles/<title>/xp00.rs`), and how a
//! title program hands over to `XP01` is [`host`].
//!
//! `XP01` draws what it shows after its main loop with the same parts —
//! Dunkle Schatten's end rolls the credits, Captain Zins's parade draws
//! the wave's columns — so the sequences on the way out
//! (`titles/<title>/ending.rs`) run on this machine too ([`host::Ending`]).
//!
//! Each routine is a loop of its own in the original. Here they are turned
//! inside out: one tick is one vertical retrace, and what the original
//! does between two retraces is done in one call.
//!
//! **The clock.** `XP00` has no frame pass. Its palette loops wait for the
//! start of the vertical retrace before every DAC write (Dunkle Schatten's
//! `0A35h:00B8h`, `0xA408`: port `3DAh` bit 3), so a fade advances one step
//! a retrace; everything else counts the `+0Eh` word of the record the
//! timer counts into (Dunkle Schatten's `[13C8h]`, Captain Zins's
//! `[297Ch]`), which goes up by one every `2E9Ah` cycles of the timer chip
//! — a hundredth of a second. The timer module is `XP01`'s. The retrace
//! rate is the machine's and not in the program: VGA mode 13h scans 449
//! lines of 800 dots at 25.175 MHz, 70.086 frames a second.
//!
//! **Music.** The library is `XP01`'s, and each call is the driver's
//! entry; they leave the program as [`Request`]s for the game to hand on.
//! Every one of them is a no-op without a sound card, and the pictures and
//! timing do not depend on them.

pub(crate) mod anim;
pub(crate) mod host;
pub(crate) mod intro;
pub(crate) mod menu;
pub(crate) mod roll;

use std::collections::VecDeque;
use std::path::Path;

use motionvm_parsec_formats::{Dat, Fgroup, Font, Picture, Rib, Slot, find_ci, open_dat, to_cp437};
use motionvm_render::{Framebuffer, Palette};

use crate::dac::Apply as _;
use crate::draw;
use crate::error::Result;
use crate::exe::Image;
use crate::num;
use crate::titles::parsec1994::cursor::Slot as TimerSlot;
use crate::titles::parsec1994::keyboard::{KeyMap, Keyboard, Route};
use crate::titles::parsec1994::{RETRACE_DEN, RETRACE_NUM, Request};

/// Cycles a second of the timer chip.
const PIT_HZ: u64 = 1_193_182;
/// The hundredth's period: `0xC128` takes the elapsed cycles off
/// `cs:[0EAh]` and `0xC142` adds `2E9Ah` back for each count (Dunkle
/// Schatten's; `XP01` `0x19F88`, `0x19FA2`).
const HUNDREDTH_PERIOD: u64 = 0x2E9A;

/// How long one tick of the title program lasts: one retrace of VGA mode
/// 13h, the rate the entry effect of `XP01` waits on too — not in the
/// program, the card's.
pub(crate) fn frame_nanos() -> u64 {
    u64::try_from(
        u128::from(RETRACE_NUM) * 1_000_000_000 / (u128::from(PIT_HZ) * u128::from(RETRACE_DEN)),
    )
    .unwrap_or(u64::MAX)
}

/// Timer cycles elapsed after `frames` retraces.
fn pit_cycles(frames: u64) -> u64 {
    u64::try_from(u128::from(frames) * u128::from(RETRACE_NUM) / u128::from(RETRACE_DEN))
        .unwrap_or(u64::MAX)
}

/// Hundredths the interrupt has counted after `cycles`: one each time the
/// countdown, a full period at the start, goes under zero — strictly, as
/// the `sub`/`jae` of the original borrows.
fn hundredths(cycles: u64) -> u64 {
    cycles.saturating_sub(1) / HUNDREDTH_PERIOD
}

/// The whole fade: `push 100h`, `push 0` (Dunkle Schatten's
/// `0x3324`/`0x3331`).
pub(crate) const FULL: i32 = 0x100;
/// The screen's size, and the surface's the programs draw into (Dunkle
/// Schatten's `0x3C62`/`0x3C67`: `140h` by `0C8h`).
pub(crate) const SURFACE_W: i32 = 320;
pub(crate) const SURFACE_H: i32 = 200;
/// What the title program's blank fills the screen with, Dunkle
/// Schatten's `0x28F9`: `push 0FEh`. `XP01`'s fills it with 0 (`0x110A2`).
const BLANK_FILL: u8 = 0xFE;
pub(crate) const BLANK_FILL_XP01: u8 = 0;
/// Escape as the input module maps it (Dunkle Schatten's `0x3A80`: `cmp
/// word [106Eh], 100h`).
pub(crate) const KEY_ESCAPE: u16 = 0x100;

/// What a title program decided, as `_main` takes its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Choice {
    /// A new game: `SAVEGAME.SAV` deleted when it was there, exit code 0.
    New,
    /// The saved game continued: exit code 0.
    Continue,
    /// Exit code −1: the batch file stops before `XP01`.
    Quit,
}

/// An event as the poll leaves it in its word (Dunkle Schatten's
/// `08E6h:00F0h`, `[106Ah]`): a button pressed, 5 for the left and 6 for
/// the right (four and the button bits), or −1 for a key, its code beside
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    Button(u16),
    Key(u16),
}

impl Event {
    fn word(self) -> u16 {
        match self {
            Event::Button(b) => b,
            Event::Key(_) => 0xFFFF,
        }
    }
}

/// What a stage did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step<T> {
    /// Waits: for the next retrace, or for time or input to pass.
    Yield,
    /// Goes on now.
    Next,
    /// The part is over, with what it answers.
    Done(T),
}

/// A palette the program has loaded: an index into [`Lib::palettes`].
pub(crate) type Pal = usize;

/// A palette fade, Dunkle Schatten's `0x3290` (Captain Zins's `0323h:0121h`):
/// from one level to another by `step`, one level a retrace, the skip
/// asked after each, then the last level once more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fade {
    pub palette: Pal,
    level: i32,
    step: i32,
    to: i32,
    /// Whether the loop is over and only the final write is left.
    last: bool,
}

impl Fade {
    /// From `from` to `to` by `step`.
    pub(crate) fn between(palette: Pal, from: i32, to: i32, step: i32) -> Self {
        Fade {
            palette,
            level: from,
            step,
            to,
            last: false,
        }
    }

    /// Dunkle Schatten's `0323h:00E7h` (`0x3317`): a positive step fades in
    /// from 0 to `100h`, a negative one out from `100h` to 0.
    pub(crate) fn full(palette: Pal, step: i32) -> Self {
        let (from, to) = if step > 0 { (0, FULL) } else { (FULL, 0) };
        Fade::between(palette, from, to, step)
    }
}

/// What a fade step did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Faded {
    /// This retrace is used: the step waits for the next.
    Yield,
    /// A level written; the fade goes on as this.
    On(Fade),
    /// The last level written again: the fade is over.
    Done,
}

/// What the library reads out of `CMBMOU.DAT` and the image, which every
/// title program draws with.
pub(crate) struct Common {
    /// Resource 0, the bank the blank writes again (Dunkle Schatten's
    /// `0x2928`); resource 1 the pointer (`0x0C9D`); resource 15 the font
    /// (`0x0A3D`, `+3Ch`).
    pub bank: Vec<u8>,
    pub pointer: Picture,
    pub font: Font,
    /// The pointer's limits and where it starts, at `DS:019Eh` and
    /// `DS:027Ah` in Dunkle Schatten's and Captain Zins's title programs,
    /// `DS:0194h` and `DS:0270h` in Bi-Fi 2's (`XP01` `DS:030Ah`,
    /// `DS:03E6h`).
    pub limits: [i32; 4],
    pub start: (i32, i32),
    /// The pointer library's keyboard, for a program whose library takes
    /// the keys ([`crate::titles::parsec1994::keyboard`]), and the
    /// interval its timer slot calls the pointer at.
    pub keys: Option<(Keyboard, u16)>,
}

/// Where a title program keeps its keyboard's words, and the interval the
/// pointer's timer slot is registered with.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TitleKeys {
    pub map: KeyMap,
    /// `[1A8h]` in Dunkle Schatten's and Captain Zins's programs, `XP01`'s
    /// `[314h]`.
    pub interval: u16,
    /// Where the program keeps the pointer's four limits and its position.
    pub limits: usize,
    pub position: usize,
}
/// `CMBMOU.DAT`'s bank, pointer and font.
const BANK_RES: usize = 0;
const POINTER_RES: usize = 1;
const FONT_RES: usize = 15;

impl Common {
    /// A title program's: its limits and position where [`TitleKeys`] names
    /// them.
    pub(crate) fn read(image: &Image, res: &Path, keys: TitleKeys) -> Result<Self> {
        let mut common = Common::read_at(image, res, keys.limits, keys.position)?;
        let ds = image
            .bytes()
            .get(image.at(0)..)
            .ok_or("DGROUP outside the program")?;
        let interval = num::signed(image.word(image.at(usize::from(keys.interval)))?);
        common.keys = Some((
            Keyboard::load(&keys.map, ds, image.bytes()),
            num::unsigned(interval),
        ));
        Ok(common)
    }

    /// The same out of a program that keeps the pointer's limits and
    /// position at `limits_at` and `position_at`: `XP01`, whose
    /// [`PointerMap`](crate::titles::parsec1994::PointerMap) names them.
    pub(crate) fn read_at(
        image: &Image,
        res: &Path,
        limits_at: usize,
        position_at: usize,
    ) -> Result<Self> {
        let mut limits = [0i32; 4];
        for (k, l) in limits.iter_mut().enumerate() {
            *l = i32::from(num::signed(image.word(image.at(limits_at + 2 * k))?));
        }
        let start = (
            i32::from(num::signed(image.word(image.at(position_at))?)),
            i32::from(num::signed(image.word(image.at(position_at + 2))?)),
        );
        let path = find_ci(res, "CMBMOU.DAT").ok_or("CMBMOU.DAT not found")?;
        let cmb = Rib::parse(&std::fs::read(path)?)?;
        let get = |i: usize| -> Result<&[u8]> {
            cmb.get(i)
                .ok_or_else(|| format!("CMBMOU.DAT has no resource {i}").into())
        };
        Ok(Common {
            bank: get(BANK_RES)?.to_vec(),
            pointer: Picture::parse(get(POINTER_RES)?)?,
            font: Font::parse(get(FONT_RES)?)?,
            limits,
            start,
            keys: None,
        })
    }
}

/// An FGROUP of pictures in `RESOURCE`, as `0072h:1598h` (`XP01`'s
/// `0590h:1644h`) hands its slots over: `INTROS.DAT`, the title program's,
/// and `EXTROS.DAT`, the game's end's.
pub(crate) struct Slots {
    name: &'static str,
    group: Fgroup,
}

impl Slots {
    pub(crate) fn open(res: &Path, name: &'static str) -> Result<Self> {
        let path = find_ci(res, name).ok_or_else(|| format!("{name} not found"))?;
        let Dat::Fgroup(group) = open_dat(&std::fs::read(path)?)? else {
            return Err(format!("{name} is not an FGROUP").into());
        };
        Ok(Slots { name, group })
    }

    pub(crate) fn slot(&self, i: usize) -> Result<&Slot> {
        self.group
            .slots
            .get(i)
            .ok_or_else(|| format!("{} has no slot {i}", self.name).into())
    }

    /// How many slots the group has: the opener counts its offsets up to
    /// the null one (`XP01`'s `0590h:1297h`, `0x6C9D`–`0x6CC2`).
    pub(crate) fn len(&self) -> usize {
        self.group.slots.len()
    }

    /// A slot holding one picture.
    pub(crate) fn picture(&self, i: usize) -> Result<Picture> {
        match self.slot(i)? {
            Slot::Rib(r) => Ok(Picture::parse(&r.decoded)?),
            _ => Err(format!("{} slot {i} is not a picture", self.name).into()),
        }
    }

    /// A slot holding a palette, as its bytes: `start, count, triples`.
    pub(crate) fn palette(&self, i: usize) -> Result<Vec<u8>> {
        match self.slot(i)? {
            Slot::Raw(b) => Ok(b.clone()),
            Slot::Rib(r) => Ok(r.decoded.clone()),
            Slot::Null => Err(format!("{} slot {i} is empty", self.name).into()),
        }
    }

    /// A slot holding a directory of several resources (`01F4h:0145h`,
    /// `XP01`'s `0563h:0147h`, makes them far pointers): their bytes. The
    /// directory is the slot's decoded RIB, or the slot's bytes as they
    /// are when it is not packed.
    pub(crate) fn resources(&self, i: usize) -> Result<Vec<Vec<u8>>> {
        let blob = match self.slot(i)? {
            Slot::Rib(r) => &r.decoded,
            Slot::Raw(b) => b,
            Slot::Null => return Err(format!("{} slot {i} is empty", self.name).into()),
        };
        let dir = motionvm_parsec_formats::rib::directory(blob, "a picture slot")?
            .ok_or_else(|| format!("{} slot {i} has no directory", self.name))?;
        Ok(dir.into_iter().map(|span| blob[span].to_vec()).collect())
    }
}

/// The machine a title program runs on, as its library keeps it.
pub(crate) struct Lib {
    pub common: Common,
    /// The palettes the program has loaded, each `start, count, triples`
    /// (a count of 0 is 256).
    pub palettes: Vec<Vec<u8>>,
    /// Retraces since the program started.
    frames: u64,
    /// Whether the program has waited for this retrace already: a second
    /// wait is for the next one.
    retraced: bool,
    /// Whether `SAVEGAME.SAV` was there at the start (`_main`'s probe).
    pub saved: bool,
    /// The screen, `A000h`.
    pub screen: Framebuffer,
    /// The surface the programs draw into before they copy to the screen.
    pub surface: Framebuffer,
    /// The DAC.
    pub dac: Palette,
    /// The pointer: where, and whether the library has it drawn.
    pub mouse: (i32, i32),
    pub shown: bool,
    /// The pointer hidden, its sampling with it: the hide sets bit 15 of
    /// the pointer's state word and the timer's callback samples nothing
    /// while it stands (Dunkle Schatten `XP01`'s `179Eh:0882h` and the
    /// `js` at `0x18488`), until the thaw clears it (`179Eh:091Ah`).
    pub frozen: bool,
    /// The buttons pressed since the last poll, bit 0 the left and bit 1
    /// the right: flags, not a count, which the timer's sample sets from
    /// the buttons held (`INT 33h` function 5's `AL` into `[3D5h]`,
    /// `0x184FC`) and the poll clears all at once (`and [3D4h], 0303h`,
    /// `0x17B39`).
    pub buttons: u16,
    /// The keys not yet taken by a poll, the BIOS's ring.
    pub keys: VecDeque<u16>,
    /// The poll's word as the last poll left it: only the poll writes it
    /// (Captain Zins's three stores, all in `0970h:001Bh`/`00F0h`).
    pub last_poll: u16,
    /// The skip the checks latch every event into, and the mask over it
    /// (Dunkle Schatten's `[1404h]`/`[1402h]`, Captain Zins's
    /// `[29C2h]`/`[29C0h]`).
    pub skip: u16,
    pub skip_mask: u16,
    /// What the program asked the music driver since the game last took
    /// it.
    pub music: Vec<Request>,
    /// What the blank fills the screen with: the program's own.
    pub blank_fill: u8,
    /// The pointer's timer slot, while the library takes the keys.
    slot: Option<TimerSlot>,
}

impl Lib {
    /// The library as `XP01` holds it after its main loop: its own
    /// pointer words, `XP01`'s blank, and the screen and the DAC the game
    /// left.
    pub(crate) fn for_ending(exit: &host::Exit<'_>) -> Result<Self> {
        let common = Common::read_at(
            exit.image,
            exit.res,
            usize::from(exit.pointer.limits),
            usize::from(exit.pointer.position),
        )?;
        let mut lib = Lib::new(common);
        lib.blank_fill = BLANK_FILL_XP01;
        lib.screen = exit.screen.clone();
        lib.dac = exit.dac.clone();
        Ok(lib)
    }

    pub(crate) fn new(common: Common) -> Self {
        Lib {
            common,
            palettes: Vec::new(),
            frames: 0,
            retraced: false,
            saved: false,
            screen: draw::display(),
            surface: draw::display(),
            dac: Palette::default(),
            mouse: (0, 0),
            shown: false,
            frozen: false,
            buttons: 0,
            keys: VecDeque::new(),
            last_poll: 0,
            skip: 0,
            skip_mask: 0,
            music: Vec::new(),
            blank_fill: BLANK_FILL,
            slot: None,
        }
    }

    /// A palette loaded, for fades to name.
    pub(crate) fn add_palette(&mut self, bytes: Vec<u8>) -> Pal {
        self.palettes.push(bytes);
        self.palettes.len() - 1
    }

    /// The program started, `SAVEGAME.SAV` there or not. The pointer is
    /// where the library's image has it until the host moves it.
    pub(crate) fn begin(&mut self, saved: bool) {
        self.saved = saved;
        self.mouse_move(self.common.start.0, self.common.start.1);
        self.slot = self.common.keys.as_ref().map(|&(_, n)| TimerSlot::every(n));
    }

    /// The mouse setting a settings file holds, `+12h`, taken as both
    /// programs take it at their start: a file of at least 14h bytes
    /// (`0x6AB`–`0x6B7`), the divisors by the same table as `XP01`'s
    /// (`0x6C7`–`0x700`; a setting past 4 is 20 and 20).
    pub(crate) fn mouse_setting(&mut self, cfg: &[u8]) {
        let Some(setting) = cfg.get(0x12..0x14).filter(|_| cfg.len() >= 0x14) else {
            return;
        };
        let (y, x) = match u16::from_le_bytes([setting[0], setting[1]]) {
            0 => (10, 10),
            2 => (40, 40),
            3 => (40, 20),
            4 => (20, 40),
            _ => (20, 20),
        };
        if let Some((k, _)) = &mut self.common.keys {
            k.params.divisors = [x, y];
        }
    }

    /// A retrace begun: the hundredths it crossed counted off the pointer's
    /// timer slot, and the pointer called when it runs out.
    pub(crate) fn retrace(&mut self) {
        let before = self.now();
        self.frames += 1;
        self.retraced = false;
        let crossed = num::low_word(u32::try_from(self.now() - before).unwrap_or(u32::MAX));
        if let Some(n) = self.slot.as_mut().and_then(|s| s.advance(crossed)) {
            self.pointer_callback(n);
        }
    }

    /// The pointer's callback with `n` units: the keyboard's step, the
    /// conversion, the move inside the limits, y first — as `XP01`'s
    /// (`crate::titles::parsec1994::keyboard`). A hidden pointer samples
    /// nothing.
    fn pointer_callback(&mut self, n: u16) {
        if self.frozen {
            return;
        }
        let Some((k, _)) = &mut self.common.keys else {
            return;
        };
        k.step(n);
        let (dx, dy) = k.convert();
        let [x0, y0, x1, y1] = self.common.limits;
        let step = |at: i32, d: i16, lo: i32, hi: i32| {
            if d == 0 {
                at
            } else {
                (at + i32::from(d)).clamp(lo, hi)
            }
        };
        self.mouse = (
            step(self.mouse.0, dx, x0, x1),
            step(self.mouse.1, dy, y0, y1),
        );
    }

    /// A key, as the library's keyboard hook reads it.
    pub(crate) fn pointer_key(&mut self, scan: u8, down: bool) -> Route {
        match &mut self.common.keys {
            Some((k, _)) => k.key(scan, down),
            None => Route::Passed,
        }
    }

    /// Waits for the retrace: `true` when this one is still unused, and
    /// then it is used.
    pub(crate) fn wait_retrace(&mut self) -> bool {
        if self.retraced {
            return false;
        }
        self.retraced = true;
        true
    }

    /// The timer cycles the program has run for.
    pub(crate) fn cycles(&self) -> u64 {
        pit_cycles(self.frames)
    }

    /// The hundredths the timer has counted.
    pub(crate) fn now(&self) -> u64 {
        hundredths(self.cycles())
    }

    /// The pointer, inside the library's limits (Dunkle Schatten's `0x9A98`
    /// region; the clamp is `XP01`'s `0x185B2`, the same module).
    pub(crate) fn mouse_move(&mut self, x: i32, y: i32) {
        let [x0, y0, x1, y1] = self.common.limits;
        self.mouse = (x.clamp(x0, x1), y.clamp(y0, y1));
    }

    /// A button pressed: its flag set, unless the pointer is hidden, when
    /// nothing samples it. Two presses before a poll are the one.
    pub(crate) fn click(&mut self, left: bool) {
        if !self.frozen {
            self.buttons |= if left { 1 } else { 2 };
        }
    }

    /// A key, as the input module maps it.
    pub(crate) fn key(&mut self, code: u16) {
        self.keys.push_back(code);
    }

    /// Whether nothing waits for a poll.
    pub(crate) fn idle(&self) -> bool {
        self.buttons == 0 && self.keys.is_empty()
    }

    /// The hide (`XP01`'s `179Eh:0882h`, `0x18262`): the pointer taken
    /// off, and nothing sampled until the thaw.
    pub(crate) fn hide(&mut self) {
        self.shown = false;
        self.frozen = true;
    }

    /// The thaw (`179Eh:091Ah`, `0x182FA`): sampling goes on. It resets a
    /// press latched before the hide when the hide lasted long enough
    /// (`0x1830D`–`0x18313`); every hide the sequences make follows a
    /// poll, so none is latched then.
    pub(crate) fn thaw(&mut self) {
        self.frozen = false;
    }

    /// What the program asked the music driver, in order, since the last
    /// time this was asked.
    pub(crate) fn take_music(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.music)
    }

    /// The screen, the pointer over it while the library draws it.
    pub(crate) fn render(&self) -> Framebuffer {
        let mut fb = self.screen.clone();
        if self.shown {
            draw::blit_masked(
                &mut fb,
                &self.common.pointer,
                self.mouse.0,
                self.mouse.1,
                Some(0),
            );
        }
        fb
    }

    /// Dunkle Schatten's `0254h:03A4h` (`0x28E4`; `XP01`'s `10B8h:050Dh`):
    /// the screen switched off, filled with the program's fill, every DAC
    /// entry black, `CMBMOU.DAT`'s bank written again (`0x2930`), the
    /// screen switched on again by the caller before the next retrace — so
    /// only the fill and the DAC are seen.
    pub(crate) fn blank(&mut self) {
        self.screen.fill(self.blank_fill);
        self.dac = Palette::default();
        self.dac.apply(&self.common.bank);
    }

    /// Dunkle Schatten's `08E6h:065Dh`: the keys waiting are dropped.
    /// The mouse's flags are not touched (`XP01`'s `0x17F13`).
    pub(crate) fn flush_keys(&mut self) {
        self.keys.clear();
    }

    /// Dunkle Schatten's `0254h:0A37h`: the keys flushed, the mouse's state
    /// taken afresh (`08E6h:1241h`, so a press before now is not one), the
    /// pointer drawn.
    pub(crate) fn input_afresh(&mut self) {
        self.keys.clear();
        self.buttons = 0;
        self.shown = true;
    }

    /// The palette at `level`, written from DAC entry 0 (`0072h:0991h(buf,
    /// 0, count)`), `count` the palette's second byte, 0 for 256 (Dunkle
    /// Schatten's `0x33D9`–`0x33EC`).
    pub(crate) fn set_level(&mut self, pal: Pal, level: i32) {
        let bytes = &self.palettes[pal];
        let count = match bytes.get(1) {
            Some(0) | None => 0x100,
            Some(&n) => usize::from(n),
        };
        let rgb: Vec<u8> = bytes
            .iter()
            .skip(2)
            .take(count * 3)
            .map(|&v| scale_channel(v, level))
            .collect();
        self.dac.load(0, &rgb);
    }

    /// One step of a fade: a level written at this retrace, then the skip
    /// asked (Dunkle Schatten's `0x3290`–`0x3313`). When the loop is done,
    /// or skipped, the last level is written once more at the next
    /// retrace.
    pub(crate) fn fade(&mut self, f: Fade) -> Faded {
        if !self.wait_retrace() {
            return Faded::Yield;
        }
        if f.last {
            self.set_level(f.palette, f.to);
            return Faded::Done;
        }
        self.set_level(f.palette, f.level);
        let skipped = self.check();
        let level = f.level + f.step;
        let over = if f.step > 0 {
            level > f.to
        } else {
            level < f.to
        };
        Faded::On(Fade {
            level,
            last: skipped || over,
            ..f
        })
    }

    /// Dunkle Schatten's `0323h:000Ch` (`0x323C`): a poll, its event latched
    /// into the skip and masked; whether the intro is being skipped.
    pub(crate) fn check(&mut self) -> bool {
        let e = self.poll();
        self.skip = (self.skip | e) & self.skip_mask;
        self.skip != 0
    }

    /// The poll: the next event, its word kept (0 for none). The buttons
    /// come first, four and their bits, and are all cleared by it; a key
    /// only when no button was pressed (`0x17B53`–`0x17BB0`).
    pub(crate) fn poll_event(&mut self) -> Option<Event> {
        let e = if self.buttons != 0 {
            let b = 4 | self.buttons;
            self.buttons = 0;
            Some(Event::Button(b))
        } else {
            self.keys.pop_front().map(Event::Key)
        };
        self.last_poll = e.map_or(0, Event::word);
        e
    }

    /// The same, as the word alone.
    pub(crate) fn poll(&mut self) -> u16 {
        self.poll_event().map_or(0, Event::word)
    }

    /// Dunkle Schatten's `1601h:1465h` twin `0546h:01A3h`: a line's width.
    pub(crate) fn width(&self, text: &[u8]) -> i32 {
        i32::from(self.common.font.measure(text))
    }

    /// Dunkle Schatten's `0254h:0497h` (`10B8h:059Bh` in `XP01`): a line
    /// centered on `cx`.
    pub(crate) fn line(fb: &mut Framebuffer, font: &Font, text: &[u8], cx: i32, y: i32, ink: u8) {
        let x = cx - (i32::from(font.measure(text)) >> 1);
        draw::text::line(fb, font, text, x, y, ink);
    }
}

/// Dunkle Schatten's `01F4h:0029h` (`0x1F69`): one channel at a level of
/// `100h`, rounded at half and held below 40h — `mul bx`, `xchg ah, al`,
/// `shl ah, 1` for the rounding bit, `adc`, `cmp ax, 3Fh`. Level 0 is a
/// `rep stosb` of zeros. The routine is `XP01`'s `0563h:002Bh` byte for
/// byte.
/// One step of a stage machine's fade: the stage it goes on in made by
/// `on`, and `then` once the fade is over.
pub(crate) fn fade_stage<S, T>(
    lib: &mut Lib,
    stage: &mut S,
    f: Fade,
    on: impl FnOnce(Fade) -> S,
    then: S,
) -> Step<T> {
    match lib.fade(f) {
        Faded::Yield => Step::Yield,
        Faded::On(f) => {
            *stage = on(f);
            Step::Next
        }
        Faded::Done => {
            *stage = then;
            Step::Next
        }
    }
}

/// A picture as a whole screen of it, to copy rectangles out of.
pub(crate) fn picture_fb(p: &Picture) -> Framebuffer {
    let mut fb = draw::display();
    draw::blit_masked(&mut fb, p, 0, 0, None);
    fb
}

/// A list of text lines in the program: the far pointers at the offset
/// the instruction at `at`, opened by `head`, names, up to the null one.
pub(crate) fn far_lines(image: &Image, at: usize, head: &[u8], what: &str) -> Result<Vec<Vec<u8>>> {
    let list = image.imm16(at, head, what)?;
    Ok(image
        .ptr_list(image.at(usize::from(list)))?
        .iter()
        .map(|l| to_cp437(l))
        .collect())
}

pub(crate) fn scale_channel(v: u8, level: i32) -> u8 {
    if level == 0 {
        return 0;
    }
    let p = u32::from(v) * num::unsigned32(level);
    num::low8(num::low_word(((p >> 8) + ((p >> 7) & 1)).min(0x3F)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `01F4h:0029h`: rounded at half a step, held at 3Fh, level 0 black.
    #[test]
    fn a_channel_is_scaled_and_rounded_at_half() {
        assert_eq!(scale_channel(63, 0x100), 63);
        assert_eq!(scale_channel(63, 4), 1, "252 / 256 rounds up");
        assert_eq!(scale_channel(1, 0x80), 1, "a half rounds up");
        assert_eq!(scale_channel(1, 0x7F), 0);
        assert_eq!(scale_channel(0xFF, 0x100), 0x3F, "held below 40h");
        assert_eq!(scale_channel(63, 0), 0);
    }
}
