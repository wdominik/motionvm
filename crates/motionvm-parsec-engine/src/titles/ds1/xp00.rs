//! `XP00`, *Dunkle Schatten*'s title program.
//!
//! `DS1.BAT` and `SCHATTEN.BAT` run `xp00.exe`, and `xp01.exe` only when it
//! left errorlevel 0. Between the two programs there is nothing but that
//! exit code and `SAVEGAME.SAV`: a new game is one whose saved game the
//! title program deleted. This module is that program, read from the
//! unpacked `XP00.EXE` (64 176 bytes, `DGROUP` `0E3Ah`, image offsets
//! below); its tables and strings are read out of the image at load, and
//! the pictures out of `RESOURCE\INTROS.DAT` the way it reads them.
//!
//! `_main` (`005Ch:0008h`, `0x5C8`) probes `SAVEGAME.SAV` (`0x5EC`, byte
//! `[10A0h]`) and calls the title routine `0323h:08F5h` (`0x3B25`), which
//! runs, in order:
//!
//! 1. the blank `0254h:03A4h`, `INTROS.DAT` opened, the screen on;
//! 2. the intro `0x3700`: the logo of slots 4/5 (`0x3631`), then the
//!    *freelance* logo drawn through eight wave masks (`03B5h:0003h`,
//!    `0x3B53`) — or, with `/FXOFF` on the command line, the plain logo of
//!    slots 6/7;
//! 3. the menu `0x3785` until it answers, with the picture of slots 8/9;
//! 4. for "Neues Spiel beginnen", the backstory `0x336E`, slots 10/11, six
//!    pages of two lines;
//! 5. back in `_main`: a new game deletes `SAVEGAME.SAV` when it was there
//!    (`0x625`–`0x649`), quitting sets the exit code −1 (`0x64E`).
//!
//! Each routine is a loop of its own in the original. Here they are turned
//! inside out: one [`Xp00::tick`] is one vertical retrace, and what the
//! original does between two retraces is done in one call.
//!
//! **The clock.** `XP00` has no frame pass. Its palette loops wait for the
//! start of the vertical retrace before every DAC write (`0A35h:00B8h`,
//! `0xA408`: port `3DAh` bit 3), so a fade advances one step a retrace;
//! everything else counts `[13C8h]`, the `+0Eh` word of the record the
//! timer counts into (`DS:13BAh`, registered at `0x913`–`0x917`), which
//! goes up by one every `2E9Ah` cycles of the timer chip — a hundredth of
//! a second. The timer module is `XP01`'s: the same 8 156 bytes stand at
//! `0x0B16C` here and `0x18FCC` there, the three periods among them. The
//! retrace rate is the machine's and not in the program: VGA mode 13h
//! scans 449 lines of 800 dots at 25.175 MHz, 70.086 frames a second.
//!
//! **Music.** The title plays module 0 of `PSMP2L.DAT` from the menu on:
//! loaded as the menu is built (`04DAh:0291h(0, …)` at `0x37A1`), started
//! endlessly once the menu has faded in (`0072h:0658h(…, −1)` at `0x3854`,
//! after the fade's call at `0x3846`), faded over 2 000 ms when a line is
//! chosen (`0072h:0858h(7D0h)`, `0x3AA2`, which returns at once) and
//! stopped once the menu has faded out (`0072h:08A9h` at `0x3B07`). The
//! library is `XP01`'s, with its switch `[0A8h]` at 1 in the image and no
//! options to change it, so each call is the driver's entry; they leave
//! the program as [`Request`]s for the game to hand on. Every one of them
//! is a no-op without a sound card (`[0A2h]` clear, `0x0D80`, `0x0F85`),
//! and the pictures and timing do not depend on them.

use crate::num;
use std::collections::VecDeque;
use std::path::Path;

use motionvm_parsec_formats::detect::resource_dir;
use motionvm_parsec_formats::{Dat, Font, Picture, Rib, Slot, find_ci, open_dat, to_cp437};
use motionvm_render::{Framebuffer, Palette};

use crate::dac::Apply as _;
use crate::draw::{self, scale};

use crate::error::Result;
use crate::exe::Image;
use crate::titles::parsec1994::{RETRACE_DEN, RETRACE_NUM, Request};

/// The build these addresses were read on.
const IMAGE_LEN: usize = 64_176;

/// Cycles a second of the timer chip.
const PIT_HZ: u64 = 1_193_182;
/// The hundredth's period: `0xC128` takes the elapsed cycles off
/// `cs:[0EAh]` and `0xC142` adds `2E9Ah` back for each count (`XP01`
/// `0x19F88`, `0x19FA2`).
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

// ---- What the routines carry as immediates.

/// `0x3716`: `push 4`, the fade's step; `0x3718`: `push 96h`, the logo's
/// hold in hundredths.
const LOGO_STEP: i32 = 4;
const LOGO_HOLD: u64 = 0x96;
/// `0x371B`: `push 4`, the first logo's picture slot; its palette is the
/// next (`0x3666`: `inc si`).
const LOGO_SLOT: usize = 4;
/// `0x3795`–`0x37A1`: `push 0` six times, module 0 of the music library
/// with no buffer of the caller's.
const MENU_SONG: u16 = 0;
/// `0x3A9F`: `push 7D0h`, the fade's 2000 ms.
const MENU_FADE: u16 = 0x07D0;
/// `0x3B79`: `push 0Ch`, the wave's slot.
const WAVE_SLOT: usize = 0x0C;
/// `0x37B7`/`0x37D7`: the menu's picture and palette.
const MENU_PIC: usize = 8;
const MENU_PAL: usize = 9;
/// `0x3385`/`0x33A5`: the backstory's picture and palette.
const STORY_PIC: usize = 10;
const STORY_PAL: usize = 11;
/// The whole fade, `0x3324`/`0x3331`: `push 100h`, `push 0`.
const FULL: i32 = 0x100;
/// `0x3CB1`: `push 70h`, the wave's first palette level.
const WAVE_LEVEL: i32 = 0x70;
/// `0x3DEA`/`0x3DED`: four levels a mask from 70h (`shl di, 2`, `add di,
/// 70h`), and `0x3EDC`: `add di, 4` on to full.
const WAVE_LEVEL_STEP: i32 = 4;
/// `0x3F8C`: `sub di, 2` — the wave fades out at half the pace.
const WAVE_OUT_STEP: i32 = 2;
/// `0x3F0F`: `cmp word [13C8h], 64h` — the full logo held a second.
const WAVE_HOLD: u64 = 0x64;
/// `0x3C62`/`0x3C67`: the surface the masks are drawn into, `140h` by
/// `0C8h`.
const SURFACE_W: i32 = 320;
const SURFACE_H: i32 = 200;
/// The menu (`0x386B`: `mov word [bp-2], 0A0h`; `0x393C`: `add [bp-2],
/// 0Dh`), three lines (`0x3941`: `cmp di, 3`), each centered on `0A0h`
/// (`0x3931`).
const MENU_Y: i32 = 0xA0;
const MENU_ROW: i32 = 0x0D;
const MENU_LINES: usize = 3;
const CX: i32 = 0xA0;
/// The inks: `0x380A` idle `0FEh`, `0x39DA` hovered `0F4h`, `0x3954`
/// "continue" without a saved game `0F0h` (and `0x3966`, its row,
/// `push 0ADh`). The backstory's lines `0F4h` (`0x3481`).
const INK_IDLE: u8 = 0xFE;
const INK_HOVER: u8 = 0xF4;
const INK_GREY: u8 = 0xF0;
const INK_STORY: u8 = 0xF4;
/// The backstory's band: rows `0A7h`, `21h` of them (`0x3497`–`0x34A7`),
/// its lines `0Fh` apart (`0x34CB`).
const STORY_Y: i32 = 0xA7;
const STORY_H: i32 = 0x21;
const STORY_ROW: i32 = 0x0F;
/// What the blank fills the screen with, `0x28F9`: `push 0FEh`.
const BLANK_FILL: u8 = 0xFE;
/// Escape as the input module maps it (`0x3A80`: `cmp word [106Eh],
/// 100h`).
pub(crate) const KEY_ESCAPE: u16 = 0x100;

/// What the title program decided, by the line chosen (`0x3A88`: Escape
/// is 3) and what `_main` does with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Choice {
    /// Line 1, "Neues Spiel beginnen", after the backstory.
    New,
    /// Line 2, "Altes Spiel fortsetzen".
    Continue,
    /// Line 3, "Spiel sofort beenden", or Escape: exit code −1.
    Quit,
}

/// An event as the poll `08E6h:00F0h` leaves it in `[106Ah]`: a button
/// pressed, 5 for the left and 6 for the right (`0x8F23`–`0x8F3A`: four
/// and the button bits), or −1 for a key, its code in `[106Eh]`.
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

/// One column's span in a wave mask, as `0x3FFE` records it: the first
/// non-zero row from the top and how many non-zero rows follow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Span {
    y: i32,
    count: i32,
}

/// A rectangle as the wave keeps it (`[1408h]`, `[1410h]`): x, y, w, h.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

/// What the program reads off disk and out of its image.
struct Data {
    /// `INTROS.DAT`'s slots, as `0072h:1598h` hands them over: the
    /// pictures, and the palettes as their bytes (`start, count, triples`).
    logo: Picture,
    logo_pal: Vec<u8>,
    menu_pic: Picture,
    menu_pal: Vec<u8>,
    story_pic: Picture,
    story_pal: Vec<u8>,
    /// Slot 12's resource 0: entry 0 the palette, the rest one column
    /// each (`0x3BA3`–`0x3BF3`).
    wave_palette: Vec<u8>,
    wave_columns: Vec<Picture>,
    /// Slot 12's resource 1, the masks: for each of the 320 columns of
    /// the surface its span (`0x3C27`, `0x3C41`).
    wave_masks: Vec<Vec<Span>>,
    /// `CMBMOU.DAT`: resource 0 the bank the blank writes again
    /// (`0x2928`), resource 1 the pointer (`0x0C9D`, the descriptor at
    /// `DS:0C8h` that `0x0C88` copies into the slot), resource 15 the font
    /// (`0x0A3D`, `+3Ch`).
    bank: Vec<u8>,
    pointer: Picture,
    font: Font,
    /// `DS:0826h`, read at `0x387B`: the three lines.
    menu: Vec<Vec<u8>>,
    /// `DS:095Ah`, read at `0x3403`: the pages, each a row of lines.
    story: Vec<Vec<Vec<u8>>>,
    /// The pointer's limits, `DS:019Eh`–`01A4h` (`XP01` `DS:030Ah`), and
    /// where it starts, `DS:027Ah`/`027Ch` (`XP01` `DS:03E6h`).
    limits: [i32; 4],
    start: (i32, i32),
}

/// A palette fade, `0x3290`: from one level to another by `step`, one
/// level a retrace, then the last one once more.
#[derive(Debug, Clone, Copy)]
struct Fade {
    palette: Pal,
    level: i32,
    step: i32,
    to: i32,
    /// Whether the loop is over and only the final write is left.
    last: bool,
}

/// Which palette a fade writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pal {
    Logo,
    Menu,
    Story,
    Wave,
}

/// Where the program is.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// `0x3B25`: the blank, the file, the screen on; then `0x3700`.
    Begin,
    /// `0x3631`: a logo, unless the intro is being skipped.
    Logo,
    LogoIn(Fade),
    LogoHold {
        since: u64,
    },
    LogoOut(Fade),
    /// `0x374B`: the wave, unless the intro is being skipped.
    WaveCheck,
    /// `0x3D62`–`0x3D73`: the logo slides in from the right.
    Slide {
        since: u64,
    },
    /// `0x3DA5`–`0x3E5D`: the eight masks, one every four hundredths:
    /// drawn, then their level written at the next retrace.
    Masks {
        since: u64,
        next: usize,
        drawn: Option<usize>,
    },
    /// `0x3E60`–`0x3EEC`: the palette on to full.
    Ramp {
        level: i32,
    },
    /// `0x3EEF`–`0x3F14`: held a second.
    WaveHold {
        since: u64,
    },
    /// `0x3F16`–`0x3F98`: faded out two levels a retrace.
    WaveOut {
        level: i32,
    },
    /// `0x3F9A` and `0x3773`: the wave's and the intro's ends.
    IntroEnd,
    /// `0x3785`: the menu built and faded in.
    MenuBegin,
    MenuIn(Fade),
    /// `0x385C`–`0x3979`: the lines drawn, the pointer up.
    MenuReady,
    /// `0x3983`–`0x3A8F`: one pass a poll.
    Menu,
    MenuOut(Fade, Choice),
    /// `0x336E`: the backstory.
    StoryBegin,
    StoryIn(Fade),
    StoryReady,
    /// `0x34CD`–`0x350C`: a page up, until the next event.
    Page {
        page: usize,
    },
    StoryOut(Fade),
    Done(Choice),
}

/// The title program while it runs.
pub(crate) struct Xp00 {
    data: Data,
    stage: Stage,
    /// Retraces since the program started.
    frames: u64,
    /// Whether the program has waited for this retrace already: a second
    /// wait is for the next one.
    retraced: bool,
    /// `[10A0h]`: whether `SAVEGAME.SAV` was there at the start (`0x5F4`).
    saved: bool,
    /// The screen, `A000h`.
    screen: Framebuffer,
    /// The surface `[13F6h]` the wave and the backstory draw into.
    surface: Framebuffer,
    /// The DAC.
    dac: Palette,
    /// The pointer: where, and whether the library has it drawn
    /// (`[1AAh]` bit 0).
    mouse: (i32, i32),
    shown: bool,
    /// Events not yet taken by a poll.
    events: VecDeque<Event>,
    /// `[1404h]` and `[1402h]`: the intro's skip, latched from every event
    /// its checks see, and the mask over it.
    skip: u16,
    skip_mask: u16,
    /// `[1406h]`: the wave's own.
    wave_skip: u16,
    /// `[1408h]`: the rectangle the wave drew last.
    drawn: Rect,
    /// `[bp-14h]`: which menu lines are lit, and `[bp-4]` the line the
    /// pointer was over at the last pass (−1 for none).
    lit: [bool; MENU_LINES],
    over: i32,
    /// What the program asked the music driver since the game last took
    /// it.
    music: Vec<Request>,
}

impl Xp00 {
    /// Reads the program and its files. Nothing runs yet.
    pub(crate) fn open(dir: &Path) -> Result<Self> {
        let image = Image::open(dir, "XP00.EXE", IMAGE_LEN)?;
        let res = resource_dir(dir);
        let data = Data::read(&image, &res)?;
        Ok(Xp00 {
            data,
            stage: Stage::Begin,
            frames: 0,
            retraced: false,
            saved: false,
            screen: draw::display(),
            surface: draw::display(),
            dac: Palette::default(),
            mouse: (0, 0),
            shown: false,
            events: VecDeque::new(),
            skip: 0,
            skip_mask: 0,
            wave_skip: 0,
            drawn: Rect {
                x: 0,
                y: 0,
                w: SURFACE_W,
                h: SURFACE_H,
            },
            lit: [false; MENU_LINES],
            over: -1,
            music: Vec::new(),
        })
    }

    /// Starts the program, `SAVEGAME.SAV` there or not. The pointer is
    /// where the library's image has it until the host moves it.
    pub(crate) fn begin(&mut self, saved: bool) {
        self.saved = saved;
        self.mouse_move(self.data.start.0, self.data.start.1);
    }

    /// What the program asked the music driver, in order, since the last
    /// time this was asked.
    pub(crate) fn take_music(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.music)
    }

    /// The timer cycles the program has run for.
    pub(crate) fn cycles(&self) -> u64 {
        pit_cycles(self.frames)
    }

    /// The hundredths the timer has counted.
    fn now(&self) -> u64 {
        hundredths(self.cycles())
    }

    /// The pointer, inside the library's limits (`0x9A98` region; the
    /// clamp is `XP01`'s `0x185B2`, the same module).
    pub(crate) fn mouse_move(&mut self, x: i32, y: i32) {
        let [x0, y0, x1, y1] = self.data.limits;
        self.mouse = (x.clamp(x0, x1), y.clamp(y0, y1));
    }

    /// A button: 5 for the left, 6 for the right.
    pub(crate) fn click(&mut self, left: bool) {
        self.events
            .push_back(Event::Button(if left { 5 } else { 6 }));
    }

    /// A key, as the input module maps it.
    pub(crate) fn key(&mut self, code: u16) {
        self.events.push_back(Event::Key(code));
    }

    /// The DAC.
    pub(crate) fn palette(&self) -> &Palette {
        &self.dac
    }

    /// The screen, the pointer over it while the library draws it.
    pub(crate) fn render(&self) -> Framebuffer {
        let mut fb = self.screen.clone();
        if self.shown {
            draw::blit_masked(
                &mut fb,
                &self.data.pointer,
                self.mouse.0,
                self.mouse.1,
                Some(0),
            );
        }
        fb
    }

    /// Whether the menu is taking input.
    #[cfg(test)]
    pub(crate) fn menu_up(&self) -> bool {
        matches!(self.stage, Stage::Menu)
    }

    /// Whether a backstory page is up, and which.
    #[cfg(test)]
    pub(crate) fn page(&self) -> Option<usize> {
        match self.stage {
            Stage::Page { page } => Some(page),
            _ => None,
        }
    }

    /// Where line `i` of the menu is, as its box: x, y, w, h.
    #[cfg(test)]
    pub(crate) fn menu_box(&self, i: usize) -> (i32, i32, i32, i32) {
        let w = self.width(&self.data.menu[i]) + 2;
        (
            (SURFACE_W - w) >> 1,
            MENU_Y + MENU_ROW * i32::try_from(i).unwrap(),
            w,
            i32::from(self.data.font.line_height),
        )
    }

    /// One retrace. `Some` once the program has ended.
    ///
    /// The picture a retrace shows is the one after everything the
    /// program does before it waits again: a palette write lands in the
    /// blanking interval, and what is drawn right after it is drawn before
    /// the beam reaches it.
    pub(crate) fn tick(&mut self) -> Option<Choice> {
        self.frames += 1;
        self.retraced = false;
        loop {
            match self.run() {
                Flow::Yield => return None,
                Flow::Next => {}
                Flow::Done(c) => return Some(c),
            }
        }
    }

    /// The current stage, as far as it goes before this retrace is used.
    fn run(&mut self) -> Flow {
        match self.stage {
            Stage::Begin => {
                // 0x3B26: the blank; 0x3B2C: the file; 0x3B2F: the screen
                // on. 0x3700: the pointer thawed, the keys flushed, the
                // skip cleared and unmasked.
                self.blank();
                self.flush_keys();
                self.skip = 0;
                self.skip_mask = 0xFFFF;
                self.stage = Stage::Logo;
                Flow::Next
            }
            Stage::Logo => {
                // 0x3639: skipped altogether once the intro is.
                if self.check() {
                    self.stage = Stage::WaveCheck;
                    return Flow::Next;
                }
                // 0x35C4: the picture, then 0x35F1: the fade in.
                draw::blit_masked(&mut self.screen, &self.data.logo, 0, 0, None);
                self.stage = Stage::LogoIn(Fade::new(Pal::Logo, LOGO_STEP));
                Flow::Next
            }
            Stage::LogoIn(f) => self.fade(f, |s, f| match f {
                Some(f) => Stage::LogoIn(f),
                None => Stage::LogoHold { since: s.now() },
            }),
            Stage::LogoHold { since } => {
                // 0x3345: until [13C8h] reaches the hold, or a skip.
                if self.now() - since >= LOGO_HOLD || self.check() {
                    self.stage = Stage::LogoOut(Fade::new(Pal::Logo, -LOGO_STEP));
                    return Flow::Next;
                }
                Flow::Yield
            }
            Stage::LogoOut(f) => self.fade(f, |_, f| match f {
                Some(f) => Stage::LogoOut(f),
                None => Stage::WaveCheck,
            }),
            Stage::WaveCheck => {
                // 0x372C: [94h] is 1 in the image, cleared only by
                // `/FXOFF` (0x5FD, 0x609); 0x374B: skipped once the intro
                // is.
                if self.check() {
                    self.stage = Stage::IntroEnd;
                    return Flow::Next;
                }
                self.wave_begin();
                Flow::Next
            }
            Stage::IntroEnd => {
                // 0x3773–0x377F: the skip cleared and masked, the pointer
                // hidden.
                self.skip = 0;
                self.skip_mask = 0;
                self.shown = false;
                self.stage = Stage::MenuBegin;
                Flow::Next
            }
            Stage::MenuBegin => {
                // 0x378B: hidden, blanked; 0x37C1/0x37E1 the slots;
                // 0x37FE the picture; 0x380A the idle ink; 0x3818 none lit.
                self.shown = false;
                self.blank();
                draw::blit_masked(&mut self.screen, &self.data.menu_pic, 0, 0, None);
                self.lit = [false; MENU_LINES];
                self.stage = Stage::MenuIn(Fade::new(Pal::Menu, LOGO_STEP));
                Flow::Next
            }
            Stage::MenuIn(f) => self.fade(f, |_, f| match f {
                Some(f) => Stage::MenuIn(f),
                None => Stage::MenuReady,
            }),
            Stage::MenuReady => {
                // 0x384C–0x3854: module 0, loaded at 0x37A1, endlessly.
                self.music.push(Request::Play {
                    piece: MENU_SONG,
                    loops: -1,
                });
                self.menu_ready();
                self.stage = Stage::Menu;
                Flow::Next
            }
            Stage::Menu => {
                // Every event waiting is taken this retrace, one a pass.
                loop {
                    if let Some(choice) = self.menu_pass() {
                        // 0x3A92: hidden, the boxes gone; 0x3AA2: the
                        // music faded; 0x3AB2: the picture again, without
                        // the lines.
                        self.shown = false;
                        self.music.push(Request::FadeOut(MENU_FADE));
                        draw::blit_masked(&mut self.screen, &self.data.menu_pic, 0, 0, None);
                        self.stage = Stage::MenuOut(Fade::new(Pal::Menu, -LOGO_STEP), choice);
                        return Flow::Next;
                    }
                    if self.events.is_empty() {
                        return Flow::Yield;
                    }
                }
            }
            Stage::MenuOut(f, choice) => self.menu_out(f, choice),
            Stage::Done(c) => Flow::Done(c),
            Stage::Slide { .. }
            | Stage::Masks { .. }
            | Stage::Ramp { .. }
            | Stage::WaveHold { .. }
            | Stage::WaveOut { .. } => self.wave_stage(),
            Stage::StoryBegin
            | Stage::StoryIn(_)
            | Stage::StoryReady
            | Stage::Page { .. }
            | Stage::StoryOut(_) => self.story_stage(),
        }
    }

    /// `0x3AB2`–`0x3B07`: the menu faded out a level a retrace, then the
    /// music stopped (`0x3B07`), and line 1 goes on to the backstory
    /// (`0x3B42`).
    fn menu_out(&mut self, f: Fade, choice: Choice) -> Flow {
        let flow = self.fade(f, move |_, f| match f {
            Some(f) => Stage::MenuOut(f, choice),
            None if choice == Choice::New => Stage::StoryBegin,
            None => Stage::Done(choice),
        });
        if !matches!(self.stage, Stage::MenuOut(..)) {
            self.music.push(Request::Stop);
        }
        flow
    }

    /// The wave's stages, from the slide in to the fade out
    /// (`0x3D68`–`0x3E4D`).
    fn wave_stage(&mut self) -> Flow {
        match self.stage {
            Stage::Slide { since } => {
                // 0x3D68: from the right edge two columns a hundredth, to
                // where the logo is centered. Past an `i32` of hundredths
                // the slide is long over, and the edge is where it ends.
                let elapsed = i32::try_from(self.now() - since).unwrap_or(SURFACE_W);
                let x = (SURFACE_W - 2 * elapsed).max(self.wave_x());
                self.wave_pass(x, 0);
                if self.wave_skip != 0 || x <= self.wave_x() {
                    self.stage = Stage::Masks {
                        since: self.now(),
                        next: 0,
                        drawn: None,
                    };
                    return Flow::Next;
                }
                Flow::Yield
            }
            Stage::Masks {
                since,
                next,
                drawn: Some(f),
            } => {
                // 0x3DE8–0x3E4D: the mask's level at the retrace.
                if self.retraced {
                    return Flow::Yield;
                }
                self.retraced = true;
                self.set_level(Pal::Wave, num::count(f) * WAVE_LEVEL_STEP + WAVE_LEVEL);
                self.stage = Stage::Masks {
                    since,
                    next,
                    drawn: None,
                };
                Flow::Next
            }
            Stage::Masks {
                since,
                next,
                drawn: None,
            } => {
                let n = self.data.wave_masks.len();
                if self.wave_skip != 0 || next >= n {
                    self.stage = Stage::Ramp {
                        level: num::count(n) * WAVE_LEVEL_STEP + WAVE_LEVEL,
                    };
                    return Flow::Next;
                }
                // 0x3DB0: the mask the hundredths have reached, four each;
                // 0x3E50: the loop goes on from the one after it.
                let f = usize::try_from((self.now() - since) >> 2).map_or(n - 1, |f| f.min(n - 1));
                self.wave_pass(self.wave_x(), f);
                self.stage = Stage::Masks {
                    since,
                    next: f + 1,
                    drawn: Some(f),
                };
                Flow::Next
            }
            Stage::Ramp { level } => {
                if self.wave_skip != 0 || level > FULL {
                    self.stage = Stage::WaveHold { since: self.now() };
                    return Flow::Next;
                }
                if self.retraced {
                    return Flow::Yield;
                }
                self.retraced = true;
                self.set_level(Pal::Wave, level);
                let e = self.poll();
                self.wave_skip |= e;
                self.stage = Stage::Ramp {
                    level: level + WAVE_LEVEL_STEP,
                };
                Flow::Next
            }
            Stage::WaveHold { since } => {
                if self.wave_skip != 0 || self.now() - since >= WAVE_HOLD {
                    self.stage = Stage::WaveOut { level: FULL };
                    return Flow::Next;
                }
                let e = self.poll();
                self.wave_skip |= e;
                Flow::Yield
            }
            Stage::WaveOut { level } => {
                if self.wave_skip != 0 || level < 0 {
                    self.wave_end();
                    self.stage = Stage::IntroEnd;
                    return Flow::Next;
                }
                if self.retraced {
                    return Flow::Yield;
                }
                self.retraced = true;
                self.set_level(Pal::Wave, level);
                let e = self.poll();
                self.wave_skip |= e;
                self.stage = Stage::WaveOut {
                    level: level - WAVE_OUT_STEP,
                };
                Flow::Next
            }
            _ => unreachable!("`run` hands only the wave's stages here"),
        }
    }

    /// The backstory's stages, from its picture to its fade out
    /// (`0x3373`–`0x355B`).
    fn story_stage(&mut self) -> Flow {
        match self.stage {
            Stage::StoryBegin => {
                // 0x3373: hidden, blanked, the slots; 0x33CC the picture.
                self.shown = false;
                self.blank();
                draw::blit_masked(&mut self.screen, &self.data.story_pic, 0, 0, None);
                self.stage = Stage::StoryIn(Fade::new(Pal::Story, LOGO_STEP));
                Flow::Next
            }
            Stage::StoryIn(f) => self.fade(f, |_, f| match f {
                Some(f) => Stage::StoryIn(f),
                None => Stage::StoryReady,
            }),
            Stage::StoryReady => {
                // 0x3486–0x3490: thawed, the keys flushed, one poll.
                self.flush_keys();
                self.poll();
                self.story_page(0);
                self.stage = Stage::Page { page: 0 };
                Flow::Next
            }
            Stage::Page { page } => {
                // 0x34FD: until the poll answers anything at all.
                let mut page = page;
                loop {
                    if self.poll() != 0 {
                        page += 1;
                        if page >= self.data.story.len() {
                            // 0x3529: hidden; 0x355B the picture again,
                            // to the screen, and the fade out.
                            self.shown = false;
                            draw::blit_masked(&mut self.screen, &self.data.story_pic, 0, 0, None);
                            self.stage = Stage::StoryOut(Fade::new(Pal::Story, -LOGO_STEP));
                            return Flow::Next;
                        }
                        self.story_page(page);
                        self.stage = Stage::Page { page };
                    }
                    if self.events.is_empty() {
                        return Flow::Yield;
                    }
                }
            }
            Stage::StoryOut(f) => self.fade(f, |_, f| match f {
                Some(f) => Stage::StoryOut(f),
                None => Stage::Done(Choice::New),
            }),
            _ => unreachable!("`run` hands only the backstory's stages here"),
        }
    }

    /// One step of a fade: a level written at this retrace, then the skip
    /// asked (`0x3290`–`0x3313`). When the loop is done, or skipped, the
    /// last level is written once more at the next retrace.
    fn fade(&mut self, f: Fade, next: impl FnOnce(&Self, Option<Fade>) -> Stage) -> Flow {
        if self.retraced {
            return Flow::Yield;
        }
        self.retraced = true;
        if f.last {
            self.set_level(f.palette, f.to);
            self.stage = next(self, None);
            return Flow::Next;
        }
        self.set_level(f.palette, f.level);
        let skipped = self.check();
        let level = f.level + f.step;
        let over = if f.step > 0 {
            level > f.to
        } else {
            level < f.to
        };
        self.stage = next(
            self,
            Some(Fade {
                level,
                last: skipped || over,
                ..f
            }),
        );
        Flow::Next
    }

    /// `0323h:000Ch` (`0x323C`): a poll, its event latched into the skip
    /// and masked; whether the intro is being skipped.
    fn check(&mut self) -> bool {
        let e = self.poll();
        self.skip = (self.skip | e) & self.skip_mask;
        self.skip != 0
    }

    /// `08E6h:00F0h`: the next event, its word in `[106Ah]` (0 for none)
    /// and a key's code in `[106Eh]`.
    fn poll_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    /// The same, as the word alone.
    fn poll(&mut self) -> u16 {
        self.poll_event().map_or(0, Event::word)
    }
}

/// What a stage did.
enum Flow {
    /// Waits: for the next retrace, or for time or input to pass.
    Yield,
    /// Goes on now.
    Next,
    /// The program is over.
    Done(Choice),
}

impl Fade {
    /// `0323h:00E7h` (`0x3317`): a positive step fades in from 0 to
    /// `100h`, a negative one out from `100h` to 0.
    fn new(palette: Pal, step: i32) -> Self {
        let (from, to) = if step > 0 { (0, FULL) } else { (FULL, 0) };
        Fade {
            palette,
            level: from,
            step,
            to,
            last: false,
        }
    }
}

/// `01F4h:0029h` (`0x1F69`): one channel at a level of `100h`, rounded at
/// half and held below 40h — `mul bx`, `xchg ah, al`, `shl ah, 1` for the
/// rounding bit, `adc`, `cmp ax, 3Fh`. Level 0 is a `rep stosb` of zeros.
/// The routine is `XP01`'s `0563h:002Bh` byte for byte.
fn scale_channel(v: u8, level: i32) -> u8 {
    if level == 0 {
        return 0;
    }
    let p = u32::from(v) * num::unsigned32(level);
    num::low8(num::low_word(((p >> 8) + ((p >> 7) & 1)).min(0x3F)))
}

impl Xp00 {
    /// `0254h:03A4h` (`0x28E4`): the screen switched off, filled with
    /// `0FEh`, every DAC entry black, `CMBMOU.DAT`'s bank written again
    /// (`0x2930`), the screen switched on again by the caller before the
    /// next retrace — so only the fill and the DAC are seen.
    fn blank(&mut self) {
        self.screen.fill(BLANK_FILL);
        self.dac = Palette::default();
        self.dac.apply(&self.data.bank);
    }

    /// `08E6h:065Dh`: the keys waiting are dropped.
    fn flush_keys(&mut self) {
        self.events.retain(|e| !matches!(e, Event::Key(_)));
    }

    /// `0x3258`/`0x3F35`: the palette at `level`, written from DAC entry
    /// 0 (`0072h:0991h(buf, 0, count)`), `count` the palette's second
    /// byte, 0 for 256 (`0x33D9`–`0x33EC`).
    fn set_level(&mut self, pal: Pal, level: i32) {
        let bytes = match pal {
            Pal::Logo => &self.data.logo_pal,
            Pal::Menu => &self.data.menu_pal,
            Pal::Story => &self.data.story_pal,
            Pal::Wave => &self.data.wave_palette,
        };
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

    /// `1601h:1465h`'s twin `0546h:01A3h`: a line's width.
    fn width(&self, text: &[u8]) -> i32 {
        i32::from(self.data.font.measure(text))
    }

    /// `0254h:0497h` (`10B8h:059Bh` in `XP01`): a line centered on `cx`.
    fn line(fb: &mut Framebuffer, font: &Font, text: &[u8], cx: i32, y: i32, ink: u8) {
        let x = cx - (i32::from(font.measure(text)) >> 1);
        draw::text::line(fb, font, text, x, y, ink);
    }

    /// Where the wave's logo stands at rest: `(140h − columns) >> 1`
    /// (`0x3D48`–`0x3D50`, `sar`).
    fn wave_x(&self) -> i32 {
        (SURFACE_W - num::count(self.data.wave_columns.len())) >> 1
    }

    /// `03B5h:0003h` up to the slide (`0x3B53`–`0x3D62`).
    fn wave_begin(&mut self) {
        // 0x3B67: hidden; 0x3B6C: blanked. 0x3CA9: the surface cleared.
        self.shown = false;
        self.blank();
        self.surface.fill(0);
        // 0x3CD5: the palette at 70h, without waiting for a retrace.
        self.set_level(Pal::Wave, WAVE_LEVEL);
        // 0x3D12–0x3D42: the wave's skip cleared, the last rectangle the
        // whole surface.
        self.wave_skip = 0;
        self.drawn = Rect {
            x: 0,
            y: 0,
            w: SURFACE_W,
            h: SURFACE_H,
        };
        // 0x3D53: thawed, the keys flushed, one poll.
        self.flush_keys();
        self.poll();
        self.stage = Stage::Slide { since: self.now() };
    }

    /// `0x3F9A`: hidden, blanked, the screen on.
    fn wave_end(&mut self) {
        self.shown = false;
        self.blank();
    }

    /// `03B5h:06CDh` (`0x421D`): one pass — a poll into the wave's skip,
    /// mask `f` drawn with the logo at `x` into the surface, and the part
    /// of the surface this draw and the last one covered copied to the
    /// screen; the surface cleared again.
    fn wave_pass(&mut self, x: i32, f: usize) {
        let e = self.poll();
        self.wave_skip |= e;
        let Some(cur) = self.wave_draw(x, f) else {
            return;
        };
        // 0x42D1: the union, signed.
        let a = cur;
        let b = self.drawn;
        let x0 = a.x.min(b.x);
        let y0 = a.y.min(b.y);
        let x1 = (a.x + a.w).max(b.x + b.w);
        let y1 = (a.y + a.h).max(b.y + b.h);
        // 0A35h:00E8h: the rectangle from the surface to the screen.
        for y in y0..y1 {
            for x in x0..x1 {
                if let Some(p) = self.surface.get(x, y) {
                    self.screen.set(x, y, p);
                }
            }
        }
        // 0x42AE: the whole surface cleared; 0x42B6: this rectangle kept.
        self.surface.fill(0);
        self.drawn = cur;
    }

    /// `03B5h:0584h` (`0x40D4`): the logo's columns from `x` on, each
    /// stretched over the span the mask has at its own column of the
    /// surface; the rectangle they cover, or `None` when none is on it.
    fn wave_draw(&mut self, x: i32, f: usize) -> Option<Rect> {
        let cols = num::count(self.data.wave_columns.len());
        // 0x40F0: a logo left of the surface skips its first columns;
        // otherwise the masks are entered at x.
        let (first_col, first_span, cx, n) = if x < 0 {
            let n = cols + x;
            if n <= 0 {
                return None;
            }
            (-x, 0, 0, n.min(SURFACE_W))
        } else {
            let n = SURFACE_W - x;
            if n <= 0 {
                return None;
            }
            (0, x, x, n.min(cols))
        };
        let mut top = SURFACE_H;
        let mut bottom = 0;
        for k in 0..n {
            let col = &self.data.wave_columns[num::index(first_col + k)];
            let span = self.data.wave_masks[f][num::index(first_span + k)];
            if span.count <= 0 {
                continue;
            }
            // 0x41C6 calls 089Eh:0430h: the column, `1 × h`, to
            // `1 × count` at the span's top in the span's own column.
            // That entry hands its arguments to the scaler core at
            // 089Eh:0038h (0x8E3E), which is `XP01`'s byte for byte (see
            // `scale`), so its rows are `scale::rows`.
            let rows = scale::rows(col.height, num::small(span.count));
            let sx = first_span + k;
            for (dy, &sy) in rows.iter().enumerate() {
                let p = col.pixels[usize::from(sy) * usize::from(col.width)];
                self.surface.set(sx, span.y + num::count(dy), p);
            }
            top = top.min(span.y);
            bottom = bottom.max(span.y + span.count);
        }
        Some(Rect {
            x: cx,
            y: top,
            w: n,
            h: bottom - top,
        })
    }

    /// `0x385C`–`0x3979`: the menu put up once it has faded in.
    fn menu_ready(&mut self) {
        // 0254h:0A37h: the keys flushed, the mouse's state taken afresh
        // (08E6h:1241h, so a press before now is not one), the pointer
        // drawn; then a poll and the pointer hidden again.
        self.events.clear();
        // 0x3870–0x3946: each line in the idle ink.
        for (i, text) in self.data.menu.iter().enumerate() {
            Self::line(
                &mut self.screen,
                &self.data.font,
                text,
                CX,
                MENU_Y + MENU_ROW * num::count(i),
                INK_IDLE,
            );
        }
        // 0x3949: without a saved game the second line again, gray, and
        // its box off (`[bp-34h]`, the record's `+4`).
        if !self.saved {
            Self::line(
                &mut self.screen,
                &self.data.font,
                &self.data.menu[1],
                CX,
                MENU_Y + MENU_ROW,
                INK_GREY,
            );
        }
        // 0x3974: the pointer drawn; 0x3979: no line yet.
        self.shown = true;
        self.over = -1;
    }

    /// The line under the pointer, `-1` for none: the box list's walk
    /// (`0x99EC`–`0x9A8A`), `x <= px < x + w`, `y <= py < y + h`, a box
    /// whose `+4` bit 0 is clear passed over.
    fn menu_hover(&self) -> i32 {
        let (px, py) = self.mouse;
        let h = i32::from(self.data.font.line_height);
        for (i, text) in self.data.menu.iter().enumerate() {
            if i == 1 && !self.saved {
                continue;
            }
            let w = self.width(text) + 2;
            let x = (SURFACE_W - w) >> 1;
            let y = MENU_Y + MENU_ROW * num::count(i);
            if (x..x + w).contains(&px) && (y..y + h).contains(&py) {
                return num::count(i);
            }
        }
        -1
    }

    /// One pass of the menu's loop (`0x3983`–`0x3A8F`): a poll, the
    /// lines lit and unlit as the pointer moved, and an event taken.
    fn menu_pass(&mut self) -> Option<Choice> {
        let event = self.poll_event();
        let si = self.menu_hover();
        if si != self.over {
            // 0x39AD: from the last line up, each line that is lit and not
            // under the pointer in the idle ink, the one under it in the
            // hovered — text over text, nothing filled.
            for di in (0..MENU_LINES).rev() {
                let ink = if num::count(di) != si && self.lit[di] {
                    self.lit[di] = false;
                    INK_IDLE
                } else if num::count(di) == si && !self.lit[di] {
                    self.lit[di] = true;
                    INK_HOVER
                } else {
                    continue;
                };
                Self::line(
                    &mut self.screen,
                    &self.data.font,
                    &self.data.menu[di],
                    CX,
                    MENU_Y + MENU_ROW * num::count(di),
                    ink,
                );
            }
            self.over = si;
        }
        // 0x3A5B: a key is Escape or nothing; buttons 5 and 6 take the
        // line under the pointer, `+10h` of its box, 1 to 3.
        match event {
            Some(Event::Key(KEY_ESCAPE)) => Some(Choice::Quit),
            Some(Event::Button(5 | 6)) if si >= 0 => Some(match si {
                0 => Choice::New,
                1 => Choice::Continue,
                _ => Choice::Quit,
            }),
            _ => None,
        }
    }

    /// `0x3497`–`0x34CB`: a page — the picture's band copied into the
    /// surface, the lines laid on it, the band copied to the screen.
    fn story_page(&mut self, page: usize) {
        let pic = &self.data.story_pic;
        for y in STORY_Y..STORY_Y + STORY_H {
            for x in 0..SURFACE_W {
                let p = pic.pixels[num::index(y * i32::from(pic.width) + x)];
                self.surface.set(x, y, p);
            }
        }
        for (i, text) in self.data.story[page].iter().enumerate() {
            Self::line(
                &mut self.surface,
                &self.data.font,
                text,
                CX,
                STORY_Y + STORY_ROW * num::count(i),
                INK_STORY,
            );
        }
        for y in STORY_Y..STORY_Y + STORY_H {
            for x in 0..SURFACE_W {
                if let Some(p) = self.surface.get(x, y) {
                    self.screen.set(x, y, p);
                }
            }
        }
    }
}

/// `mov word [bp-4], imm` — the backstory's row (`0x3403`).
const MOV_BP4_IMM: [u8; 3] = [0xC7, 0x46, 0xFC];
const STORY_ROW_AT: usize = 0x3403;
/// `push word [bx+imm]` — the low word of a menu line's far pointer
/// (`0x387B`).
const PUSH_BX_MEM: [u8; 2] = [0xFF, 0xB7];
const MENU_ROW_AT: usize = 0x387B;
/// `DS:019Eh`: the pointer's four limits; `DS:027Ah`: its position.
const LIMITS: usize = 0x019E;
const POSITION: usize = 0x027A;
/// `CMBMOU.DAT`'s bank, pointer and font.
const BANK_RES: usize = 0;
const POINTER_RES: usize = 1;
const FONT_RES: usize = 15;

impl Data {
    fn read(image: &Image, res: &Path) -> Result<Self> {
        let menu_at = image.imm16(MENU_ROW_AT, &PUSH_BX_MEM, "the menu row")?;
        let menu = (0..MENU_LINES)
            .map(|i| {
                let s = image.c_string(image.far_ptr(image.at(usize::from(menu_at) + 4 * i))?)?;
                Ok(to_cp437(&s))
            })
            .collect::<Result<Vec<_>>>()?;
        let story_at = image.imm16(STORY_ROW_AT, &MOV_BP4_IMM, "the backstory's row")?;
        let mut story = Vec::new();
        let mut at = image.at(usize::from(story_at));
        while image.word(at)? != 0 || image.word(at + 2)? != 0 {
            let lines = image.ptr_list(image.far_ptr(at)?)?;
            story.push(lines.iter().map(|l| to_cp437(l)).collect());
            at += 4;
        }
        let mut limits = [0i32; 4];
        for (k, l) in limits.iter_mut().enumerate() {
            *l = i32::from(num::signed(image.word(image.at(LIMITS + 2 * k))?));
        }
        let start = (
            i32::from(num::signed(image.word(image.at(POSITION))?)),
            i32::from(num::signed(image.word(image.at(POSITION + 2))?)),
        );

        let path = find_ci(res, "INTROS.DAT").ok_or("INTROS.DAT not found")?;
        let Dat::Fgroup(group) = open_dat(&std::fs::read(path)?)? else {
            return Err("INTROS.DAT is not an FGROUP".into());
        };
        let slot = |i: usize| -> Result<&Slot> {
            group
                .slots
                .get(i)
                .ok_or_else(|| format!("INTROS.DAT has no slot {i}").into())
        };
        let picture = |i: usize| -> Result<Picture> {
            match slot(i)? {
                Slot::Rib(r) => Ok(Picture::parse(&r.decoded)?),
                _ => Err(format!("INTROS.DAT slot {i} is not a picture").into()),
            }
        };
        let palette = |i: usize| -> Result<Vec<u8>> {
            match slot(i)? {
                Slot::Raw(b) => Ok(b.clone()),
                Slot::Rib(r) => Ok(r.decoded.clone()),
                Slot::Null => Err(format!("INTROS.DAT slot {i} is empty").into()),
            }
        };
        let Slot::Rib(wave) = slot(WAVE_SLOT)? else {
            return Err("INTROS.DAT slot 12 is not the wave".into());
        };
        let entries = |blob: &[u8]| -> Result<Vec<Vec<u8>>> {
            let dir = motionvm_parsec_formats::rib::directory(blob, "a wave resource")?
                .ok_or("a wave resource has no directory")?;
            Ok(dir.into_iter().map(|r| blob[r].to_vec()).collect())
        };
        let logo_set = entries(wave.get(0).ok_or("the wave has no resource 0")?)?;
        let (wave_palette, columns) = logo_set
            .split_first()
            .ok_or("the wave's resource 0 is empty")?;
        let wave_columns = columns
            .iter()
            .map(|c| Ok(Picture::parse(c)?))
            .collect::<Result<Vec<_>>>()?;
        let wave_masks = entries(wave.get(1).ok_or("the wave has no resource 1")?)?
            .iter()
            .map(|m| Ok(spans(&Picture::parse(&Rib::parse(m)?.decoded)?)))
            .collect::<Result<Vec<_>>>()?;

        let path = find_ci(res, "CMBMOU.DAT").ok_or("CMBMOU.DAT not found")?;
        let cmb = Rib::parse(&std::fs::read(path)?)?;
        let get = |i: usize| -> Result<&[u8]> {
            cmb.get(i)
                .ok_or_else(|| format!("CMBMOU.DAT has no resource {i}").into())
        };
        Ok(Data {
            logo: picture(LOGO_SLOT)?,
            logo_pal: palette(LOGO_SLOT + 1)?,
            menu_pic: picture(MENU_PIC)?,
            menu_pal: palette(MENU_PAL)?,
            story_pic: picture(STORY_PIC)?,
            story_pal: palette(STORY_PAL)?,
            wave_palette: wave_palette.clone(),
            wave_columns,
            wave_masks,
            bank: get(BANK_RES)?.to_vec(),
            pointer: Picture::parse(get(POINTER_RES)?)?,
            font: Font::parse(get(FONT_RES)?)?,
            menu,
            story,
            limits,
            start,
        })
    }
}

/// `03B5h:04AEh` (`0x3FFE`): for every column of a mask, the first row
/// that is not 0 and how many rows that are not 0 follow it.
fn spans(mask: &Picture) -> Vec<Span> {
    let (w, h) = (usize::from(mask.width), usize::from(mask.height));
    (0..w)
        .map(|x| {
            let at = |y: usize| mask.pixels[y * w + x];
            let mut y = 0;
            while y < h && at(y) == 0 {
                y += 1;
            }
            let top = y;
            while y < h && at(y) != 0 {
                y += 1;
            }
            Span {
                y: num::count(top),
                count: num::count(y - top),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The program of the install, begun, or `None` without one.
    fn begun(saved: bool) -> Option<Xp00> {
        let dir = crate::titles::ds1::gamedata_or_skip()?;
        let mut x = Xp00::open(&dir).expect("XP00 opens");
        x.begin(saved);
        Some(x)
    }

    /// Ticks until the menu takes input; how many it took.
    fn to_menu(x: &mut Xp00) -> u32 {
        let mut n = 0;
        while !x.menu_up() {
            assert!(x.tick().is_none());
            n += 1;
            assert!(n < 2000, "the menu never came up");
        }
        n
    }

    /// The pixels of row band `y0..y1` in the inks `ink` and `ink + 1`.
    fn inked(x: &Xp00, y0: i32, y1: i32, ink: u8) -> usize {
        let fb = x.render();
        (y0..y1)
            .flat_map(|y| (0..320).map(move |x| (x, y)))
            .filter(|&(px, py)| {
                fb.get(px, py)
                    .is_some_and(|p| p == ink || p == ink.wrapping_add(1))
            })
            .count()
    }

    /// Every line comes out of the image: three in the menu (`DS:0826h`),
    /// six pages of two in the backstory (`DS:095Ah`).
    #[test]
    fn the_lines_and_pages_are_the_programs() {
        let Some(x) = begun(false) else { return };
        let menu: Vec<String> = x
            .data
            .menu
            .iter()
            .map(|l| motionvm_parsec_formats::cp437_to_string(l))
            .collect();
        assert_eq!(
            menu,
            [
                "Neues Spiel beginnen",
                "Altes Spiel fortsetzen",
                "Spiel sofort beenden"
            ]
        );
        assert_eq!(x.data.story.len(), 6);
        assert!(x.data.story.iter().all(|p| p.len() == 2));
        assert_eq!(
            motionvm_parsec_formats::cp437_to_string(&x.data.story[5][0]),
            "Aber laß Dich nicht unterkriegen!"
        );
        assert_eq!(x.data.limits, [0, 0, 314, 194], "XP01's limits");
        assert_eq!(x.mouse, (160, 100), "XP01's start");
    }

    /// No line the program measures — the menu's three, the backstory's
    /// twelve — holds a byte below `20h`, so `0546h:01A3h` meets a control
    /// byte in none of them.
    #[test]
    fn the_measured_lines_hold_no_control_byte() {
        let Some(x) = begun(false) else { return };
        let lines: Vec<&Vec<u8>> = x
            .data
            .menu
            .iter()
            .chain(x.data.story.iter().flatten())
            .collect();
        assert_eq!(lines.len(), 15);
        for l in lines {
            assert!(
                !l.is_empty() && l.iter().all(|&b| b >= 0x20),
                "{:?}",
                motionvm_parsec_formats::cp437_to_string(l)
            );
        }
    }

    /// The wave's logo is 287 columns of 123 rows, at rest at column 16;
    /// eight masks give each of the surface's 320 columns a span.
    #[test]
    fn the_wave_is_287_columns_through_eight_masks() {
        let Some(x) = begun(false) else { return };
        assert_eq!(x.data.wave_columns.len(), 287);
        assert!(
            x.data
                .wave_columns
                .iter()
                .all(|c| (c.width, c.height) == (1, 123))
        );
        assert_eq!(x.wave_x(), 16);
        assert_eq!(x.data.wave_masks.len(), 8);
        assert!(x.data.wave_masks.iter().all(|m| m.len() == 320));
        assert_eq!(x.data.wave_palette[1], 0xC0, "192 colors");
    }

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

    /// A fade is one level a retrace — level 0 at the first, 4 at the
    /// second — and its last level is written twice, the second time at
    /// the 66th retrace.
    #[test]
    fn a_fade_writes_one_level_a_retrace() {
        let Some(mut x) = begun(false) else { return };
        let pal = x.data.logo_pal.clone();
        let entry = |x: &Xp00, i: usize| x.dac.raw[i];
        // The brightest channel of the logo's palette, to see a step in.
        let i = (0..usize::from(pal[1]) * 3)
            .max_by_key(|&i| pal[2 + i])
            .unwrap();
        x.tick();
        assert_eq!(entry(&x, i), 0);
        x.tick();
        assert_eq!(entry(&x, i), scale_channel(pal[2 + i], 4));
        for _ in 2..65 {
            x.tick();
        }
        assert_eq!(entry(&x, i), pal[2 + i], "level 100h at the 65th");
    }

    /// Without input the logo stands at full brightness for 107 retraces:
    /// the loop's last level, its repeat, 150 hundredths (`0x3718`) —
    /// 105.1 retraces — and the fade out's first level, which is 100h
    /// again. The reference capture shows it for 107 frames too.
    #[test]
    fn the_logo_is_held_a_second_and_a_half() {
        let Some(mut x) = begun(false) else { return };
        let full: Vec<u8> = x.data.logo_pal[2..].to_vec();
        let mut at_full = 0;
        for _ in 0..300 {
            x.tick();
            if x.dac.raw[..full.len()] == full[..] {
                at_full += 1;
            }
        }
        assert_eq!(at_full, 107);
    }

    /// Without input the intro plays to the menu: the first logo, the
    /// wave, the menu's fade — 656 retraces on this clock.
    #[test]
    fn the_intro_plays_through_to_the_menu() {
        let Some(mut x) = begun(false) else { return };
        assert_eq!(to_menu(&mut x), 656);
        assert!(x.shown, "the pointer is drawn");
    }

    /// Any event skips the rest of the intro (`0x323C`): the fade it hit
    /// writes its last level, the logo's hold ends, the fade out jumps to
    /// black, the wave is not shown — and the menu's own fade cannot be
    /// skipped (`[1402h]` is 0 by then, `0x3779`).
    #[test]
    fn a_click_skips_the_intro() {
        let Some(mut x) = begun(false) else { return };
        for _ in 0..10 {
            x.tick();
        }
        x.click(true);
        let n = to_menu(&mut x);
        assert_eq!(
            n,
            4 + 66,
            "the step that saw it, the fade's last, the fade out's first and \
             last; the menu's fade"
        );
    }

    /// Without a saved game "Altes Spiel fortsetzen" is gray (`0F0h`,
    /// `0x3954`) and its box is off: a click on it chooses nothing, and
    /// the pointer over it lights nothing.
    #[test]
    fn continue_is_gray_and_closed_without_a_saved_game() {
        let Some(mut x) = begun(false) else { return };
        x.click(true);
        to_menu(&mut x);
        let (bx, by, bw, bh) = x.menu_box(1);
        assert!(inked(&x, by, by + bh, INK_GREY) > 0, "gray");
        x.mouse_move(bx + bw / 2, by + bh / 2);
        x.tick();
        assert_eq!(inked(&x, by, by + bh, INK_HOVER), 0, "not lit");
        x.click(true);
        x.tick();
        assert!(x.menu_up(), "nothing chosen");
    }

    /// With a saved game the line is in the idle ink, lights under the
    /// pointer and is chosen by a click: the picture fades out and the
    /// program ends with "continue".
    #[test]
    fn continue_is_chosen_with_a_saved_game() {
        let Some(mut x) = begun(true) else { return };
        x.click(true);
        to_menu(&mut x);
        let (bx, by, bw, bh) = x.menu_box(1);
        assert_eq!(inked(&x, by, by + bh, INK_GREY), 0);
        x.mouse_move(bx + bw / 2, by + bh / 2);
        x.tick();
        assert!(inked(&x, by, by + bh, INK_HOVER) > 0, "lit");
        x.click(false);
        let mut end = None;
        for t in 1..200 {
            if let Some(c) = x.tick() {
                end = Some((t, c));
                break;
            }
        }
        assert_eq!(end, Some((66, Choice::Continue)));
    }

    /// The pointer over a line lights it in `0F4h`, and off it the line
    /// goes back to `0FEh` — text over text (`0x39AD`).
    #[test]
    fn the_line_under_the_pointer_is_lit() {
        let Some(mut x) = begun(false) else { return };
        x.click(true);
        to_menu(&mut x);
        let (bx, by, bw, bh) = x.menu_box(0);
        assert_eq!(inked(&x, by, by + bh, INK_HOVER), 0);
        x.mouse_move(bx, by);
        x.tick();
        assert!(
            inked(&x, by, by + bh, INK_HOVER) > 0,
            "lit at the box's corner"
        );
        x.mouse_move(bx + bw, by);
        x.tick();
        assert_eq!(inked(&x, by, by + bh, INK_HOVER), 0, "w is past the box");
    }

    /// Escape is the third line (`0x3A80`).
    #[test]
    fn escape_quits() {
        let Some(mut x) = begun(true) else { return };
        x.click(true);
        to_menu(&mut x);
        x.key(KEY_ESCAPE);
        let end = (0..200).find_map(|_| x.tick());
        assert_eq!(end, Some(Choice::Quit));
    }

    /// A new game goes through the backstory: six pages, each up until an
    /// event, then the picture fades out and the program ends.
    #[test]
    fn a_new_game_shows_six_pages() {
        let Some(mut x) = begun(true) else { return };
        x.click(true);
        to_menu(&mut x);
        let (bx, by, _, _) = x.menu_box(0);
        x.mouse_move(bx + 1, by + 1);
        x.click(true);
        let mut t = 0;
        while x.page().is_none() {
            assert!(x.tick().is_none());
            t += 1;
        }
        assert_eq!(t, 66 + 66, "the menu's fade out, the backstory's in");
        assert!(!x.shown, "no pointer over the backstory");
        for page in 0..6 {
            assert_eq!(x.page(), Some(page));
            for _ in 0..50 {
                x.tick();
            }
            assert_eq!(x.page(), Some(page), "a page waits");
            x.key(u16::from(b' '));
            x.tick();
        }
        // The last page's event came in at a retrace the fade out's first
        // level was written at: 65 more.
        let end = (1..100).find_map(|t| x.tick().map(|c| (t, c)));
        assert_eq!(end, Some((65, Choice::New)));
    }
}
