//! `XP00`, *Captain Zins*'s title program.
//!
//! `ZINS.BAT` runs `xp00.exe`, and `xp01.exe` only when it left errorlevel
//! 0. This module is the program's own part, read from the unpacked
//! `XP00.EXE` (72 640 bytes, `DGROUP` `0EEBh`, image offsets below); its
//! library — the machine, the fades, the intro, the menu routine — is the
//! generation's ([`crate::titles::parsec1994::title`]), shared with
//! Dunkle Schatten's title program but for its addresses.
//!
//! `_main` (`005Ch:0008h`, `0x5C8`) probes `SAVEGAME.SAV` (`0x5D4`–`0x5F4`,
//! `[2654h]`) and calls the title routine `0323h:1169h` (`0x4399`), which
//! runs, in order:
//!
//! 1. `INTROS.DAT` opened, the music library started (`0323h:040Bh`);
//! 2. an animation, slot 0 (`0323h:0459h`, `0323h:01FFh`): a palette and
//!    39 frames of deltas, ten hundredths each, which nothing skips;
//! 3. the intro (`0323h:049Bh`): the logo of slots 4/5, the wave of slot
//!    13 — Dunkle Schatten's, byte for byte, but for the slot;
//! 4. the credits rolled through a window (`0323h:0EDCh`), until an event;
//! 5. the menu (`0323h:088Ah`) until it answers: five lines, 1 and 2 a
//!    page viewer each (`0323h:0520h`) and back to the menu, 3 a new game,
//!    4 the saved one, 5 or Escape quitting;
//! 6. `INTROS.DAT` closed; back in `_main` the answer 3, 4 or 5 is a new
//!    game (`SAVEGAME.SAV` deleted), a continued one, or exit code −1.
//!
//! **Music.** Module 0 of `PSMP2L.DAT` under the credits, 2 under the
//! menu, 1 under the pages, each started endlessly once its picture has
//! faded in and faded over 2 000 ms as the part ends; the animation and
//! the intro are silent.

use std::path::Path;

use motionvm_parsec_formats::detect::resource_dir;
use motionvm_parsec_formats::{Font, Picture, Slot, to_cp437};
use motionvm_render::Framebuffer;

use crate::draw;
use crate::error::Result;
use crate::exe::Image;
use crate::exe::op::{PUSH_BX_MEM, PUSH_IMM16};
use crate::num;
use crate::titles::parsec1994::KeyMap;
use crate::titles::parsec1994::Request;
use crate::titles::parsec1994::title::host::TitleProgram;
use crate::titles::parsec1994::title::intro::{Intro, Rect, copy_rect};
use crate::titles::parsec1994::title::menu::{MUSIC_FADE, Menu, MenuSpec};
use crate::titles::parsec1994::title::roll::{CMBMOU_INKS, Roll, RollSpec, STAR_50};
use crate::titles::parsec1994::title::{
    Choice, Common, Event, FULL, Fade, Faded, KEY_ESCAPE, Lib, Pal, Slots, Step, TitleKeys,
    fade_stage, picture_fb,
};

/// The build these addresses were read on.
const IMAGE_LEN: usize = 72_640;

/// `0x36A4`: the animation's slot; `0x3447`: `add ax, 0FFFEh` — its first
/// resource the palette, the last never shown.
const ANIM_SLOT: usize = 0;
/// `0x34EF`–`0x34F9`: `cmp word [297Ch], 0Ah` — a frame each ten
/// hundredths; `0x3502`–`0x3512`: one more hundredth after the last.
const ANIM_FRAME: u64 = 10;
const ANIM_TAIL: u64 = 1;
/// `0x345E`: the screen filled with 0 before the fade in; `0x3529`: with
/// `0FEh` after the fade out.
const ANIM_FILL_IN: u8 = 0;
const ANIM_FILL_OUT: u8 = 0xFE;
/// `0x43FC`: `push 0Dh`, the wave's slot.
const WAVE_SLOT: usize = 0x0D;
/// The credits: slot 10 the picture, 11 its palette, 12 the frame cel
/// (`0x413A`, `0x415A`, `0x417A`); module 0 (`0x4194`).
const CREDITS_PIC: usize = 10;
const CREDITS_PAL: usize = 11;
const CREDITS_FRAME: usize = 12;
const CREDITS_SONG: u16 = 0;
/// The window (`0x4210`–`0x4247`): from (9, 7) to the exclusive edges
/// `0CAh` and `0A8h`, 193 by 161.
const WIN: Rect = Rect {
    x: 9,
    y: 7,
    w: 0xC1,
    h: 0xA1,
};
/// The roll (`0323h:0C78h`): its first line `0A1h` below the window's
/// top, which is the window's height, so it comes in from the bottom edge
/// (`0x3EE1`); a `c` line on by `0Eh`, an `n` by `10h`, a `p` by `14h`
/// (`0x3F7B`, `0x3F4A`, `0x3F76`); centered in the window's `0C1h`
/// (`0x3F63`–`0x3F6A`) and drawn from its top (`0x408E`: `add ax, 7`).
const ROLL: RollSpec = RollSpec {
    start: 0xA1,
    advance: [0x0E, 0x10, 0x14],
    star: STAR_50,
    inks: CMBMOU_INKS,
    window: WIN,
};
/// `0x42C8`–`0x42DF`: a line a step each three hundredths.
const ROLL_STEP: u64 = 3;
/// The menu: slots 8/9, module 2 (`0x3AD4`), its fade by 8 (`0x3B55`);
/// five lines (`0x3CC4`: `cmp di, 5`) from `82h` (`0x3BEE`), `0Dh` apart
/// (`0x3CBF`); line 4 gray without a saved game (`0x3CCC`–`0x3CF4`);
/// Escape the fifth (`0x3E0B`); between the fade and the lines up to 500
/// hundredths' wait (`0x3B96`–`0x3BB5`) and a dim to `80h` (`0x3BB7`).
const MENU_PIC: usize = 8;
const MENU_PAL: usize = 9;
const MENU_SONG: u16 = 2;
const MENU_STEP: i32 = 8;
const MENU_LINES: usize = 5;
const MENU_ROW: i32 = 0x0D;
const MENU_GREY: usize = 3;
const MENU_ESCAPE: u16 = 5;
const MENU_WAIT: u64 = 0x1F4;
const MENU_DIM: i32 = 0x80;
/// The page viewer: slot 2 the picture, 3 the palette, 1 the font
/// (`0x3798`, `0x37B8`, `0x37D8`); module 1 (`0x3776`).
const PAGES_PIC: usize = 2;
const PAGES_PAL: usize = 3;
const PAGES_FONT: usize = 1;
const PAGES_SONG: u16 = 1;
/// `0x395C`: the band the page is drawn over, 320 by 144 from the top.
const PAGES_BAND: Rect = Rect {
    x: 0,
    y: 0,
    w: 320,
    h: 0x90,
};
/// `0x397B`/`0x399C`: lines from `y` 9, `0Ah` apart, at `x` `10h`.
const PAGE_X: i32 = 0x10;
const PAGE_Y: i32 = 9;
const PAGE_ROW: i32 = 0x0A;
/// `0323h:000Ch`: each character a fixed 6 pixels on (`0x328A`), a shadow
/// at (+1, +1) in `1Fh` (`0x324A`), the character in 6 (`0x329B`), `%`
/// switching it with 36h (`0x32CE`, `xor 30h`).
const PAGE_ADVANCE: i32 = 6;
const INK_SHADOW: u8 = 0x1F;
const INK_PAGE: u8 = 6;
const INK_SWITCH: u8 = 0x30;
/// `mov word [bp-2], imm`: the first line's row (`0x3BEE`). The low word
/// of a menu line's far pointer is a `push word [bx+imm]` (`0x3BFE`); the
/// page tables (`0x3AA3`, `0x3AAF`), the credits' lines (`0x3EAF`) and the
/// viewer's boxes (`0x385E`, `0x386A`, `0x3876`) are `push imm16`s.
const MOV_BP2_IMM: [u8; 3] = [0xC7, 0x46, 0xFE];
const MENU_ROW_AT: usize = 0x3BFE;
const MENU_Y_AT: usize = 0x3BEE;
const PAGES_AT: [usize; 2] = [0x3AA3, 0x3AAF];
const CREDITS_AT: usize = 0x3EAF;
const BOXES_AT: [usize; 3] = [0x385E, 0x386A, 0x3876];

/// One of the viewer's three boxes (`DS:25A0h`, `25B2h`, `25C4h`): its
/// rectangle and what a click on it answers, 1 on, 2 back, 3 out.
#[derive(Debug, Clone, Copy)]
struct PageBox {
    rect: Rect,
    value: u16,
}

/// Where the title routine is.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// `0323h:0459h`: the pointer hidden, the blank, the screen filled
    /// with 0, the palette faded in.
    Anim,
    AnimIn(Fade),
    /// Frame `k` of the 39 up, shown from `since`.
    AnimFrame {
        k: usize,
        since: u64,
    },
    AnimTail {
        since: u64,
    },
    AnimOut(Fade),
    Intro,
    /// `0323h:0EDCh`: the credits' picture faded in.
    Credits,
    CreditsIn(Fade),
    /// A step of the roll drawn at `pos`, shown from `since`.
    Roll {
        pos: i32,
        since: u64,
    },
    CreditsOut(Fade),
    Menu,
    /// `0323h:0520h` on table `t`.
    Pages(usize),
    PagesBlack(usize, Fade),
    /// The loop: the page up is `last`, the one asked for `page`.
    PageRun(Viewer),
    PageOut(Viewer, Fade),
    PageIn(Viewer, Fade),
    PagesEnd(Fade),
    Done(Choice),
}

/// The viewer's state between its passes.
#[derive(Debug, Clone, Copy)]
struct Viewer {
    table: usize,
    page: usize,
    last: Option<usize>,
    /// `[bp-4]`: whether the palette has been faded in once.
    shown: bool,
}

/// The title program while it runs.
pub(crate) struct Xp00 {
    lib: Lib,
    /// Slot 0: the palette, as a full 256-colour bank, and the deltas.
    anim_pal: Pal,
    frames: Vec<Vec<u8>>,
    intro: Intro,
    credits_pic: Picture,
    credits_pal: Pal,
    /// Slot 12, decoded twice to tell its holes from its colour 0: a pixel
    /// is drawn where the two agree.
    frame: (Picture, Picture),
    roll: Roll,
    menu: Menu,
    pages_pic: Picture,
    pages_pal: Pal,
    pages_font: Font,
    /// `DS:0BE0h` and `DS:0D20h`: the two viewers' pages, each a row of
    /// lines.
    pages: [Vec<Vec<Vec<u8>>>; 2],
    boxes: [PageBox; 3],
    stage: Stage,
}

/// The pointer library's keyboard in `XP00` (`crate::titles::parsec1994::keyboard`),
/// the same code as `XP01`'s at `0970h`.
const KEYS: TitleKeys = TitleKeys {
    map: KeyMap {
        // `0x0CDF`–`0x0CE3`: `push ds ; push 0BCh ; call far 0970h:049Fh`.
        list: 0x00BC,
        // `cs:0402h`, `0x9C02`: `cmp byte [cs:0402h], 0`.
        switch: 0x09700 + 0x0402,
        // `0x9C36`: `mov di, 2618h`.
        speeds: 0x2618,
        // `0x9C2F`, `0x9C41`: `[25Ch]`, `[25Eh]`, `[260h]`.
        limits: 0x025C,
        // `0xA3BC`: `mul word [26Ch]`.
        acceleration: 0x026C,
        // `0xA469`: `idiv word [26Eh]`; `0xA429`: `idiv word [270h]`.
        divisors: 0x026E,
    },
    // `0x0CAF`: `push word [1A8h]`, the interval the slot is registered with.
    interval: 0x01A8,
    // `DS:019Eh`: the pointer's four limits; `DS:027Ah`: its position.
    limits: 0x019E,
    position: 0x027A,
};

impl Xp00 {
    /// Reads the program and its files. Nothing runs yet.
    pub(crate) fn open(dir: &Path) -> Result<Self> {
        let image = Image::open(dir, "XP00.EXE", IMAGE_LEN)?;
        let res = resource_dir(dir);
        let mut lib = Lib::new(Common::read(&image, &res, KEYS)?);
        let intros = Slots::open(&res, "INTROS.DAT")?;

        let anim = intros.resources(ANIM_SLOT)?;
        let (palette, deltas) = anim.split_first().ok_or("the animation is empty")?;
        // 0x346B: `push 100h` — the palette has no `start, count` of its
        // own: 256 colours from entry 0.
        let mut bank = vec![0u8, 0];
        bank.extend_from_slice(palette);
        let anim_pal = lib.add_palette(bank);
        // 0x3447: the resource count less two, frames 1 to 39.
        let shown = deltas.len().saturating_sub(1);
        let frames = deltas[..shown].to_vec();

        let intro = Intro::read(&mut lib, &intros, WAVE_SLOT)?;
        let credits_pal = lib.add_palette(intros.palette(CREDITS_PAL)?);
        let frame = match intros.slot(CREDITS_FRAME)? {
            Slot::Rib(r) => (
                Picture::parse_prefix_filled(&r.decoded, 0)?,
                Picture::parse_prefix_filled(&r.decoded, 0xFF)?,
            ),
            _ => return Err("INTROS.DAT slot 12 is not the frame".into()),
        };
        let lines_at = image.imm16(CREDITS_AT, &PUSH_IMM16, "the credits' lines")?;
        let lines: Vec<Vec<u8>> = image
            .ptr_list(image.at(usize::from(lines_at)))?
            .iter()
            .map(|l| to_cp437(l))
            .collect();
        let roll = Roll::layout(&lines, &lib.common.font, ROLL);

        let menu_at = image.imm16(MENU_ROW_AT, &PUSH_BX_MEM, "the menu's lines")?;
        let menu_y = image.imm16(MENU_Y_AT, &MOV_BP2_IMM, "the menu's first row")?;
        let menu_lines = (0..MENU_LINES)
            .map(|i| {
                let s = image.c_string(image.far_ptr(image.at(usize::from(menu_at) + 4 * i))?)?;
                Ok(to_cp437(&s))
            })
            .collect::<Result<Vec<_>>>()?;
        let menu_pal = lib.add_palette(intros.palette(MENU_PAL)?);
        let menu = Menu::new(
            MenuSpec {
                lines: menu_lines,
                y: i32::from(menu_y),
                row: MENU_ROW,
                grey: MENU_GREY,
                song: Some(MENU_SONG),
                step: MENU_STEP,
                wait_dim: Some((MENU_WAIT, MENU_DIM)),
                escape: MENU_ESCAPE,
                inks: crate::titles::parsec1994::title::menu::INKS,
                box_up: 0,
                box_taller: 0,
                out_step: crate::titles::parsec1994::title::menu::OUT_STEP,
                blank: true,
                afresh_after_lines: false,
            },
            intros.picture(MENU_PIC)?,
            menu_pal,
        );

        let pages_font = match intros.slot(PAGES_FONT)? {
            Slot::Rib(r) => Font::parse(&r.decoded)?,
            _ => return Err("INTROS.DAT slot 1 is not the font".into()),
        };
        let pages = read_pages(&image)?;
        let boxes = read_boxes(&image)?;
        let pages_pal = lib.add_palette(intros.palette(PAGES_PAL)?);
        Ok(Xp00 {
            lib,
            anim_pal,
            frames,
            intro,
            credits_pic: intros.picture(CREDITS_PIC)?,
            credits_pal,
            frame,
            roll,
            menu,
            pages_pic: intros.picture(PAGES_PIC)?,
            pages_pal,
            pages_font,
            pages,
            boxes,
            stage: Stage::Anim,
        })
    }

    /// Whether the menu is taking input.
    #[cfg(test)]
    pub(crate) fn menu_up(&self) -> bool {
        matches!(self.stage, Stage::Menu) && self.menu.up()
    }

    /// Where line `i` of the menu is, as its box.
    #[cfg(test)]
    pub(crate) fn menu_box(&self, i: usize) -> (i32, i32, i32, i32) {
        self.menu.line_box(&self.lib, i)
    }

    /// Whether the credits roll, and which page of which viewer is up.
    #[cfg(test)]
    fn rolling(&self) -> bool {
        matches!(self.stage, Stage::Roll { .. })
    }

    #[cfg(test)]
    fn page(&self) -> Option<(usize, usize)> {
        match self.stage {
            Stage::PageRun(v) => v.last.map(|p| (v.table, p)),
            _ => None,
        }
    }

    /// A fade step, the stage it goes on in made by `on` and the one after
    /// it `then`.
    fn fade(&mut self, f: Fade, on: impl FnOnce(Fade) -> Stage, then: Stage) -> Step<Choice> {
        fade_stage(&mut self.lib, &mut self.stage, f, on, then)
    }

    /// The title routine, as far as it goes before this retrace is used.
    fn routine(&mut self) -> Step<Choice> {
        match self.stage {
            Stage::Anim => {
                // 0x368D: hidden; 0x3434: the blank; 0x345E: the screen
                // filled with 0; 0x3475: the fade in by 4.
                self.lib.shown = false;
                self.lib.blank();
                self.lib.screen.fill(ANIM_FILL_IN);
                self.stage = Stage::AnimIn(Fade::full(self.anim_pal, 4));
                Step::Next
            }
            Stage::AnimIn(f) => self.fade(
                f,
                Stage::AnimIn,
                Stage::AnimFrame {
                    k: 0,
                    since: self.lib.now(),
                },
            ),
            Stage::AnimFrame { k, since } => {
                if k > 0 && self.lib.now() - since < ANIM_FRAME {
                    return Step::Yield;
                }
                // 0x34A1–0x34FB: the clock from 0, the frame's deltas
                // onto the screen, ten hundredths.
                // As the roll's: the clock set to 0 the moment the last
                // wait ended, so a frame is ten hundredths to the count.
                if let Some(d) = self.frames.get(k) {
                    crate::titles::parsec1994::title::anim::apply_delta(&mut self.lib.screen, d);
                    let since = if k == 0 { since } else { since + ANIM_FRAME };
                    self.stage = Stage::AnimFrame { k: k + 1, since };
                    Step::Yield
                } else {
                    self.stage = Stage::AnimTail {
                        since: since + ANIM_FRAME,
                    };
                    Step::Next
                }
            }
            Stage::AnimTail { since } => {
                if self.lib.now() - since < ANIM_TAIL {
                    return Step::Yield;
                }
                self.stage = Stage::AnimOut(Fade::full(self.anim_pal, -4));
                Step::Next
            }
            Stage::AnimOut(f) => match self.lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::AnimOut(f);
                    Step::Next
                }
                Faded::Done => {
                    // 0x3529: filled with 0FEh; 0x352F: the blank.
                    self.lib.screen.fill(ANIM_FILL_OUT);
                    self.lib.blank();
                    self.stage = Stage::Intro;
                    Step::Next
                }
            },
            Stage::Intro => match self.intro.step(&mut self.lib) {
                Step::Yield => Step::Yield,
                Step::Next => Step::Next,
                Step::Done(()) => {
                    self.stage = Stage::Credits;
                    Step::Next
                }
            },
            Stage::Credits | Stage::CreditsIn(_) | Stage::Roll { .. } | Stage::CreditsOut(_) => {
                self.credits()
            }
            Stage::Menu => match self.menu.step(&mut self.lib) {
                Step::Yield => Step::Yield,
                Step::Next => Step::Next,
                Step::Done(v) => {
                    self.stage = match v {
                        1 => Stage::Pages(0),
                        2 => Stage::Pages(1),
                        3 => Stage::Done(Choice::New),
                        4 => Stage::Done(Choice::Continue),
                        _ => Stage::Done(Choice::Quit),
                    };
                    Step::Next
                }
            },
            Stage::Done(c) => Step::Done(c),
            _ => self.viewer(),
        }
    }

    /// `0323h:0EDCh`: the credits.
    fn credits(&mut self) -> Step<Choice> {
        match self.stage {
            Stage::Credits => {
                // 0x4112–0x4123: the skip off, hidden, the blank; 0x41AC:
                // the picture; 0x41D0: its palette faded in by 4.
                self.lib.skip = 0;
                self.lib.skip_mask = 0;
                self.lib.shown = false;
                self.lib.blank();
                draw::blit_masked(&mut self.lib.screen, &self.credits_pic, 0, 0, None);
                self.stage = Stage::CreditsIn(Fade::full(self.credits_pal, 4));
                Step::Next
            }
            Stage::CreditsIn(f) => match self.lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::CreditsIn(f);
                    Step::Next
                }
                Faded::Done => {
                    // 0x425B: the keys flushed; 0x4268: module 0,
                    // endlessly.
                    self.lib.flush_keys();
                    self.lib.music.push(Request::Play {
                        piece: CREDITS_SONG,
                        loops: -1,
                    });
                    self.roll_step(0);
                    self.stage = Stage::Roll {
                        pos: 0,
                        since: self.lib.now(),
                    };
                    Step::Next
                }
            },
            Stage::Roll { pos, since } => {
                // 0x42C8–0x42DF: polled until three hundredths have passed
                // or an event came; 0x42E1: on a line, round at the end;
                // 0x42EB: on while no event came.
                let mut event = 0;
                while event == 0 && self.lib.now() - since < ROLL_STEP {
                    event = self.lib.poll();
                    if event == 0 && self.lib.idle() {
                        return Step::Yield;
                    }
                }
                let next = if pos + 1 >= self.roll.end() {
                    0
                } else {
                    pos + 1
                };
                if event != 0 {
                    // 0x42EF: hidden; 0x4321: the picture again, without
                    // the roll and its frame; 0x432C: the music faded;
                    // 0x4349: the palette out by 4.
                    self.lib.shown = false;
                    draw::blit_masked(&mut self.lib.screen, &self.credits_pic, 0, 0, None);
                    self.lib.music.push(Request::FadeOut(MUSIC_FADE));
                    self.stage = Stage::CreditsOut(Fade::full(self.credits_pal, -4));
                    return Step::Next;
                }
                // The clock is set to 0 (`0x4270`) right after the wait
                // left, which the hundredth that ended it did: a step is
                // three hundredths to the count, wherever the retrace
                // falls.
                self.roll_step(next);
                self.stage = Stage::Roll {
                    pos: next,
                    since: since + ROLL_STEP,
                };
                Step::Yield
            }
            Stage::CreditsOut(f) => match self.lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::CreditsOut(f);
                    Step::Next
                }
                Faded::Done => {
                    // 0x4353, 0x4358: the song stopped, `StopAll`.
                    self.lib.music.push(Request::Stop);
                    self.lib.music.push(Request::StopEffects);
                    self.stage = Stage::Menu;
                    Step::Next
                }
            },
            _ => unreachable!("`run` hands only the credits' stages here"),
        }
    }

    /// One step of the roll at `pos` (`0x4270`–`0x42BC`): the window's band
    /// of the picture into the surface, the lines in view over it, the
    /// frame over them, the window to the screen.
    fn roll_step(&mut self, pos: i32) {
        let lib = &mut self.lib;
        copy_rect(&picture_fb(&self.credits_pic), &mut lib.surface, WIN);
        // 0x4278: the cursor on, its pauses thrown away; 0x4299: the lines
        // in view.
        self.roll.draw(&mut lib.surface, &lib.common.font, pos);
        // 0x42A6: the frame, its holes left.
        let (a, b) = &self.frame;
        for yy in 0..i32::from(a.height) {
            for xx in 0..i32::from(a.width) {
                let i = num::index(yy * i32::from(a.width) + xx);
                if a.pixels[i] == b.pixels[i] {
                    lib.surface.set(WIN.x + xx, WIN.y + yy, a.pixels[i]);
                }
            }
        }
        copy_rect(&lib.surface, &mut lib.screen, WIN);
    }

    /// `0323h:0520h`: a page viewer.
    fn viewer(&mut self) -> Step<Choice> {
        match self.stage {
            Stage::Pages(t) => {
                // 0x3756–0x3767: the skip off, hidden, the blank; 0x382D:
                // the palette from 100h to 0 by −100h.
                self.lib.skip = 0;
                self.lib.skip_mask = 0;
                self.lib.shown = false;
                self.lib.blank();
                self.stage = Stage::PagesBlack(t, Fade::between(self.pages_pal, FULL, 0, -FULL));
                Step::Next
            }
            Stage::PagesBlack(t, f) => match self.lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::PagesBlack(t, f);
                    Step::Next
                }
                Faded::Done => {
                    // 0x3855: the picture; 0x389E: input afresh, the
                    // pointer up.
                    draw::blit_masked(&mut self.lib.screen, &self.pages_pic, 0, 0, None);
                    self.lib.input_afresh();
                    self.stage = Stage::PageRun(Viewer {
                        table: t,
                        page: 0,
                        last: None,
                        shown: false,
                    });
                    Step::Next
                }
            },
            Stage::PageRun(v) => self.viewer_pass(v),
            Stage::PageOut(v, f) => match self.lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::PageOut(v, f);
                    Step::Next
                }
                Faded::Done => {
                    self.page_draw(v);
                    self.stage = Stage::PageIn(v, Fade::full(self.pages_pal, 8));
                    Step::Next
                }
            },
            Stage::PageIn(v, f) => match self.lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::PageIn(v, f);
                    Step::Next
                }
                Faded::Done => {
                    if !v.shown {
                        // 0x39DC: module 1, endlessly, after the first
                        // fade in.
                        self.lib.music.push(Request::Play {
                            piece: PAGES_SONG,
                            loops: -1,
                        });
                    }
                    self.stage = Stage::PageRun(Viewer {
                        last: Some(v.page),
                        shown: true,
                        ..v
                    });
                    Step::Next
                }
            },
            Stage::PagesEnd(f) => match self.lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::PagesEnd(f);
                    Step::Next
                }
                Faded::Done => {
                    // 0x3A86, 0x3A8B: stopped, `StopAll`; the menu again.
                    self.lib.music.push(Request::Stop);
                    self.lib.music.push(Request::StopEffects);
                    self.stage = Stage::Menu;
                    Step::Next
                }
            },
            _ => unreachable!("`run` hands only the viewer's stages here"),
        }
    }

    /// One pass of the viewer's loop (`0x38A3`–`0x3A0F`): a poll, the
    /// boxes and Escape taken, and a page asked for drawn.
    fn viewer_pass(&mut self, v: Viewer) -> Step<Choice> {
        let mut v = v;
        {
            let event = self.lib.poll_event();
            let over = self.boxes.iter().find(|b| {
                let (x, y) = self.lib.mouse;
                (b.rect.x..b.rect.x + b.rect.w).contains(&x)
                    && (b.rect.y..b.rect.y + b.rect.h).contains(&y)
            });
            let count = self.pages[v.table].len();
            let out = match (event, over) {
                (Some(Event::Key(KEY_ESCAPE)), _) => true,
                (Some(Event::Button(5 | 6)), Some(b)) => match b.value {
                    1 => {
                        v.page = (v.page + 1).min(count.saturating_sub(1));
                        false
                    }
                    2 => {
                        v.page = v.page.saturating_sub(1);
                        false
                    }
                    _ => true,
                },
                _ => false,
            };
            if out {
                // 0x3A12: hidden; 0x3A1A: the music faded; 0x3A37: out by
                // 4.
                self.lib.shown = false;
                self.lib.music.push(Request::FadeOut(MUSIC_FADE));
                self.stage = Stage::PagesEnd(Fade::full(self.pages_pal, -4));
                return Step::Next;
            }
            if v.last == Some(v.page) {
                self.stage = Stage::PageRun(v);
                return if self.lib.idle() {
                    Step::Yield
                } else {
                    Step::Next
                };
            }
        }
        // 0x3906: a page asked for that is not up: out by 8 once the
        // palette was shown, then drawn and in — by 4 the first time.
        if v.shown {
            self.stage = Stage::PageOut(v, Fade::full(self.pages_pal, -8));
        } else {
            self.page_draw(v);
            self.stage = Stage::PageIn(v, Fade::full(self.pages_pal, 4));
        }
        Step::Next
    }

    /// `0x392B`–`0x39AC`: the band of the picture again, the page's lines
    /// over it, the pointer up.
    fn page_draw(&mut self, v: Viewer) {
        copy_rect(
            &picture_fb(&self.pages_pic),
            &mut self.lib.screen,
            PAGES_BAND,
        );
        if let Some(lines) = self.pages[v.table].get(v.page) {
            for (i, text) in lines.iter().enumerate() {
                shadowed(
                    &mut self.lib.screen,
                    &self.pages_font,
                    text,
                    PAGE_X,
                    PAGE_Y + PAGE_ROW * num::count(i),
                );
            }
        }
        self.lib.shown = true;
    }
}

/// The two viewers' pages: each table (`push imm16` at `0x3AA3` and
/// `0x3AAF`) a list of far pointers to pages up to a null one, each page a
/// list of lines.
fn read_pages(image: &Image) -> Result<[Vec<Vec<Vec<u8>>>; 2]> {
    let mut pages: [Vec<Vec<Vec<u8>>>; 2] = [Vec::new(), Vec::new()];
    for (t, &at) in PAGES_AT.iter().enumerate() {
        let table = image.imm16(at, &PUSH_IMM16, "a page table")?;
        let mut p = image.at(usize::from(table));
        while image.word(p)? != 0 || image.word(p + 2)? != 0 {
            let lines = image.ptr_list(image.far_ptr(p)?)?;
            pages[t].push(lines.iter().map(|l| to_cp437(l)).collect());
            p += 4;
        }
    }
    Ok(pages)
}

/// The viewer's three boxes, each record named by a `push imm16`
/// (`0x385E`, `0x386A`, `0x3876`): `+8` to `+0Eh` its rectangle, `+10h`
/// what it answers.
fn read_boxes(image: &Image) -> Result<[PageBox; 3]> {
    let mut boxes = [PageBox {
        rect: Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        },
        value: 0,
    }; 3];
    for (b, &at) in boxes.iter_mut().zip(BOXES_AT.iter()) {
        let rec = usize::from(image.imm16(at, &PUSH_IMM16, "a viewer's box")?);
        let w = |k: usize| -> Result<i32> {
            Ok(i32::from(num::signed(image.word(image.at(rec + k))?)))
        };
        *b = PageBox {
            rect: Rect {
                x: w(8)?,
                y: w(0x0A)?,
                w: w(0x0C)?,
                h: w(0x0E)?,
            },
            value: image.word(image.at(rec + 0x10))?,
        };
    }
    Ok(boxes)
}

/// `0323h:000Ch` (`0x323C`): a line a character at a time, 6 pixels on
/// each, first its shadow at (+1, +1), then the line in its ink; `%` takes
/// no place and switches the ink.
fn shadowed(fb: &mut Framebuffer, font: &Font, text: &[u8], x: i32, y: i32) {
    for pass in 0..2 {
        let (dx, mut ink) = if pass == 0 {
            (1, INK_SHADOW)
        } else {
            (0, INK_PAGE)
        };
        let mut cx = x + dx;
        for &c in text {
            if c == b'%' {
                if pass == 1 {
                    ink ^= INK_SWITCH;
                }
                continue;
            }
            draw::text::line(fb, font, &[c], cx, y + dx, ink);
            cx += PAGE_ADVANCE;
        }
    }
}

impl TitleProgram for Xp00 {
    fn lib(&self) -> &Lib {
        &self.lib
    }

    fn lib_mut(&mut self) -> &mut Lib {
        &mut self.lib
    }

    fn run(&mut self) -> Step<Choice> {
        self.routine()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The program of the install, begun, or `None` without one.
    fn begun(saved: bool) -> Option<Xp00> {
        let dir = crate::titles::capzins::gamedata_or_skip()?;
        let mut x = Xp00::open(&dir).expect("XP00 opens");
        x.begin(saved);
        Some(x)
    }

    /// Ticks until `done`; how many it took.
    fn until(x: &mut Xp00, done: impl Fn(&Xp00) -> bool) -> u32 {
        let mut n = 0;
        while !done(x) {
            assert!(x.tick().is_none(), "the program ended early");
            n += 1;
            assert!(n < 20_000, "never got there");
        }
        n
    }

    /// What the program reads: five menu lines from `DS:098Ah`, the first
    /// at `82h`; ten and five pages; 88 lines of credits laid out to 2457
    /// rows (`0323h:0C78h`); the viewer's boxes at (11, 163), (94, 163),
    /// (176, 163), 57 by 20, answering 1, 2, 3; 39 frames of animation.
    #[test]
    fn the_program_s_tables_are_read() {
        let Some(x) = begun(false) else { return };
        let lines: Vec<String> = x
            .menu
            .line_texts()
            .iter()
            .map(|l| motionvm_parsec_formats::cp437_to_string(l))
            .collect();
        assert_eq!(
            lines,
            [
                "Kurze Spielanleitung",
                "Wissenswertes über GriPS",
                "Neues Spiel beginnen",
                "Altes Spiel fortsetzen",
                "Spiel sofort beenden"
            ]
        );
        assert_eq!(x.menu_box(0).1, 0x82);
        assert_eq!((x.pages[0].len(), x.pages[1].len()), (10, 5));
        assert_eq!(x.roll.len(), 88);
        assert_eq!(x.roll.end(), 2457);
        let boxes: Vec<(i32, i32, i32, i32, u16)> = x
            .boxes
            .iter()
            .map(|b| (b.rect.x, b.rect.y, b.rect.w, b.rect.h, b.value))
            .collect();
        assert_eq!(
            boxes,
            [
                (11, 163, 57, 20, 1),
                (94, 163, 57, 20, 2),
                (176, 163, 57, 20, 3)
            ]
        );
        assert_eq!(x.frames.len(), 39);
        assert_eq!(x.lib.palettes[x.anim_pal].len(), 2 + 768);
        assert_eq!((x.frame.0.width, x.frame.0.height), (193, 161));
    }

    /// The animation cannot be skipped: a click during it changes nothing
    /// of its length, the intro after it is what the click skips.
    #[test]
    fn the_animation_plays_to_its_end_and_the_credits_roll() {
        let Some(mut a) = begun(false) else { return };
        let Some(mut b) = begun(false) else { return };
        let to_intro = |x: &Xp00| matches!(x.stage, Stage::Intro);
        let plain = until(&mut a, to_intro);
        for _ in 0..100 {
            b.tick();
        }
        b.click(true);
        let clicked = until(&mut b, to_intro) + 100;
        assert_eq!(plain, clicked, "the animation takes as long either way");
        // Two fades of 66 retraces and 39 frames of ten hundredths.
        assert!((66 + 66 + 270..66 + 66 + 290).contains(&plain), "{plain}");
        until(&mut a, Xp00::rolling);
        assert!(!a.lib.shown, "no pointer over the credits");
    }

    /// The roll steps a line each three hundredths to the count: the
    /// clock is set to 0 the moment the last wait ended (`0x4270`), so no
    /// retrace's lateness adds up — 33⅓ lines a second, as a capture of the
    /// original rolls them.
    #[test]
    fn the_roll_steps_a_line_each_three_hundredths() {
        let Some(mut x) = begun(false) else { return };
        until(&mut x, Xp00::rolling);
        let at = |x: &Xp00| match x.stage {
            Stage::Roll { pos, .. } => pos,
            _ => -1,
        };
        let (p0, t0) = (at(&x), x.lib.now());
        for _ in 0..700 {
            x.tick();
        }
        let (p1, t1) = (at(&x), x.lib.now());
        let want = i32::try_from((t1 - t0) / 3).unwrap();
        assert!(
            (want - 1..=want + 1).contains(&(p1 - p0)),
            "{} lines in {} hundredths",
            p1 - p0,
            t1 - t0
        );
    }

    /// An event ends the credits; the menu fades in by 8, waits, dims,
    /// and takes input.
    #[test]
    fn the_credits_end_on_an_event_and_the_menu_comes_up() {
        let Some(mut x) = begun(false) else { return };
        until(&mut x, Xp00::rolling);
        for _ in 0..200 {
            x.tick();
        }
        assert!(x.rolling(), "the roll goes on by itself");
        x.key(u16::from(b' '));
        let n = until(&mut x, Xp00::menu_up);
        // The credits' fade out (66), the menu's in by 8 (34), the wait
        // of 500 hundredths (about 350 retraces) and the dim (34).
        assert!((66 + 34 + 340..66 + 34 + 34 + 360).contains(&n), "{n}");
        assert!(x.lib.shown, "the pointer is drawn");
    }

    /// Line 1 opens the guide: its first page up after the viewer's fade,
    /// the boxes turning the pages, Escape back to the menu.
    #[test]
    fn the_guide_is_paged_and_left_for_the_menu() {
        let Some(mut x) = begun(false) else { return };
        x.click(true);
        until(&mut x, Xp00::rolling);
        x.click(true);
        until(&mut x, Xp00::menu_up);
        let (bx, by, _, _) = x.menu_box(0);
        x.mouse_move(bx + 1, by + 1);
        x.click(true);
        until(&mut x, |x| x.page() == Some((0, 0)));
        let on = x.boxes[0].rect;
        x.mouse_move(on.x + 1, on.y + 1);
        x.click(true);
        until(&mut x, |x| x.page() == Some((0, 1)));
        let back = x.boxes[1].rect;
        x.mouse_move(back.x + 1, back.y + 1);
        x.click(true);
        until(&mut x, |x| x.page() == Some((0, 0)));
        x.key(KEY_ESCAPE);
        until(&mut x, Xp00::menu_up);
    }

    /// Without a saved game line 4 cannot be chosen; Escape quits, line 3
    /// starts a new game.
    #[test]
    fn the_menu_answers_new_continue_and_quit() {
        for (saved, line, want) in [
            (false, None, Choice::Quit),
            (false, Some(2), Choice::New),
            (true, Some(3), Choice::Continue),
        ] {
            let Some(mut x) = begun(saved) else { return };
            x.click(true);
            until(&mut x, Xp00::rolling);
            x.click(true);
            until(&mut x, Xp00::menu_up);
            match line {
                Some(i) => {
                    let (bx, by, _, _) = x.menu_box(i);
                    x.mouse_move(bx + 1, by + 1);
                    x.click(true);
                }
                None => x.key(KEY_ESCAPE),
            }
            let end = (0..400).find_map(|_| x.tick());
            assert_eq!(end, Some(want), "saved {saved}, line {line:?}");
        }
        let Some(mut x) = begun(false) else { return };
        x.click(true);
        until(&mut x, Xp00::rolling);
        x.click(true);
        until(&mut x, Xp00::menu_up);
        let (bx, by, _, _) = x.menu_box(3);
        x.mouse_move(bx + 1, by + 1);
        x.click(true);
        for _ in 0..100 {
            assert!(x.tick().is_none(), "continue is closed without a save");
        }
    }
}
