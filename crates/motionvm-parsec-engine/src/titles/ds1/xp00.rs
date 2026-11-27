//! `XP00`, *Dunkle Schatten*'s title program.
//!
//! `DS1.BAT` and `SCHATTEN.BAT` run `xp00.exe`, and `xp01.exe` only when it
//! left errorlevel 0. This module is the program's own part, read from the
//! unpacked `XP00.EXE` (64 176 bytes, `DGROUP` `0E3Ah`, image offsets
//! below); what its library does — the machine, the fades, the intro, the
//! menu routine — is the generation's ([`crate::titles::parsec1994::title`]).
//!
//! `_main` (`005Ch:0008h`, `0x5C8`) probes `SAVEGAME.SAV` (`0x5EC`, byte
//! `[10A0h]`) and calls the title routine `0323h:08F5h` (`0x3B25`), which
//! runs, in order:
//!
//! 1. the blank `0254h:03A4h`, `INTROS.DAT` opened, the screen on;
//! 2. the intro `0x3700`: the logo of slots 4/5 (`0x3631`), then the
//!    *freelance* logo drawn through eight wave masks (`03B5h:0003h`,
//!    `0x3B53`, slot 12);
//! 3. the menu `0x3785` until it answers, with the picture of slots 8/9:
//!    three lines, 1 a new game, 2 the saved one, 3 or Escape quitting;
//! 4. for "Neues Spiel beginnen", the backstory `0x336E`, slots 10/11, six
//!    pages of two lines;
//! 5. back in `_main`: a new game deletes `SAVEGAME.SAV` when it was there
//!    (`0x625`–`0x649`), quitting sets the exit code −1 (`0x64E`).
//!
//! **Music.** The title plays module 0 of `PSMP2L.DAT` from the menu on:
//! loaded as the menu is built (`04DAh:0291h(0, …)` at `0x37A1`), started
//! endlessly once the menu has faded in (`0072h:0658h(…, −1)` at `0x3854`),
//! faded over 2 000 ms when a line is chosen (`0x3AA2`) and stopped once
//! the menu has faded out (`0x3B07`, `0x3B0C`). No call of the program
//! plays an effect: its play routine (`0072h:08CBh`) has no caller.

use std::path::Path;

use motionvm_parsec_formats::detect::resource_dir;
use motionvm_parsec_formats::{Picture, to_cp437};

use crate::draw;
use crate::error::Result;
use crate::exe::Image;
use crate::exe::op::PUSH_BX_MEM;
use crate::num;
use crate::titles::parsec1994::KeyMap;
use crate::titles::parsec1994::title::host::TitleProgram;
use crate::titles::parsec1994::title::intro::Intro;
use crate::titles::parsec1994::title::menu::{Menu, MenuSpec};
use crate::titles::parsec1994::title::{
    Choice, Common, Fade, Faded, Lib, Pal, SURFACE_W, Slots, Step, TitleKeys,
};

/// The build these addresses were read on.
const IMAGE_LEN: usize = 64_176;

/// `0x3795`–`0x37A1`: `push 0` six times, module 0 of the music library
/// with no buffer of the caller's.
const MENU_SONG: u16 = 0;
/// `0x3B79`: `push 0Ch`, the wave's slot.
const WAVE_SLOT: usize = 0x0C;
/// `0x37B7`/`0x37D7`: the menu's picture and palette.
const MENU_PIC: usize = 8;
const MENU_PAL: usize = 9;
/// `0x3385`/`0x33A5`: the backstory's picture and palette.
const STORY_PIC: usize = 10;
const STORY_PAL: usize = 11;
/// The menu (`0x386B`: `mov word [bp-2], 0A0h`; `0x393C`: `add [bp-2],
/// 0Dh`), three lines (`0x3941`: `cmp di, 3`); its fade's step, 4; line 2
/// gray without a saved game (`0x3949`); Escape the third line (`0x3A88`).
const MENU_Y: i32 = 0xA0;
const MENU_ROW: i32 = 0x0D;
const MENU_LINES: usize = 3;
const MENU_STEP: i32 = 4;
const MENU_GREY: usize = 1;
const MENU_ESCAPE: u16 = 3;
/// The backstory's fades: by 4, in and out.
const STORY_STEP: i32 = 4;
/// The backstory's lines `0F4h` (`0x3481`), centered on `0A0h`.
const INK_STORY: u8 = 0xF4;
const CX: i32 = 0xA0;
/// The backstory's band: rows `0A7h`, `21h` of them (`0x3497`–`0x34A7`),
/// its lines `0Fh` apart (`0x34CB`).
const STORY_Y: i32 = 0xA7;
const STORY_H: i32 = 0x21;
const STORY_ROW: i32 = 0x0F;

/// `mov word [bp-4], imm` — the backstory's row (`0x3403`).
const MOV_BP4_IMM: [u8; 3] = [0xC7, 0x46, 0xFC];
const STORY_ROW_AT: usize = 0x3403;
const MENU_ROW_AT: usize = 0x387B;

/// Where the title routine is.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// `0x3B25`: the blank, the file, the screen on.
    Begin,
    Intro,
    Menu,
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
    lib: Lib,
    intro: Intro,
    menu: Menu,
    /// `DS:095Ah`, read at `0x3403`: the backstory's pages, each a row of
    /// lines.
    story: Vec<Vec<Vec<u8>>>,
    story_pic: Picture,
    story_pal: Pal,
    stage: Stage,
}

/// The pointer library's keyboard in `XP00` (`crate::titles::parsec1994::keyboard`),
/// the same code as `XP01`'s at `08E6h`.
const KEYS: TitleKeys = TitleKeys {
    map: KeyMap {
        // `0x0CDF`–`0x0CE3`: `push ds ; push 0BCh ; call far 08E6h:1393h`.
        list: 0x00BC,
        // `cs:12C2h`, `0xA256`: `cmp byte [cs:12C2h], 0`.
        switch: 0x08E60 + 0x12C2,
        // `0xA28A`: `mov di, 1084h`.
        speeds: 0x1084,
        // `0xA283`, `0xA295`: `[25Ch]`, `[25Eh]`, `[260h]`.
        limits: 0x025C,
        // `0x989C`: `mul word [26Ch]`.
        acceleration: 0x026C,
        // `0x9949`: `idiv word [26Eh]`; `0x9909`: `idiv word [270h]`.
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
        let menu_at = image.imm16(MENU_ROW_AT, &PUSH_BX_MEM, "the menu row")?;
        let menu_lines = (0..MENU_LINES)
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
        let mut lib = Lib::new(Common::read(&image, &res, KEYS)?);
        let intros = Slots::open(&res, "INTROS.DAT")?;
        let intro = Intro::read(&mut lib, &intros, WAVE_SLOT)?;
        let menu_pal = lib.add_palette(intros.palette(MENU_PAL)?);
        let menu = Menu::new(
            MenuSpec {
                lines: menu_lines,
                y: MENU_Y,
                row: MENU_ROW,
                grey: MENU_GREY,
                song: Some(MENU_SONG),
                step: MENU_STEP,
                wait_dim: None,
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
        let story_pal = lib.add_palette(intros.palette(STORY_PAL)?);
        Ok(Xp00 {
            lib,
            intro,
            menu,
            story,
            story_pic: intros.picture(STORY_PIC)?,
            story_pal,
            stage: Stage::Begin,
        })
    }

    /// Whether the menu is taking input.
    #[cfg(test)]
    pub(crate) fn menu_up(&self) -> bool {
        matches!(self.stage, Stage::Menu) && self.menu.up()
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
        self.menu.line_box(&self.lib, i)
    }

    /// The title routine, as far as it goes before this retrace is used.
    fn routine(&mut self) -> Step<Choice> {
        match self.stage {
            Stage::Begin => {
                // 0x3B26: the blank; 0x3B2C: the file; 0x3B2F: the screen
                // on. Then the intro, 0x3700.
                self.lib.blank();
                self.stage = Stage::Intro;
                Step::Next
            }
            Stage::Intro => match self.intro.step(&mut self.lib) {
                Step::Yield => Step::Yield,
                Step::Next => Step::Next,
                Step::Done(()) => {
                    self.stage = Stage::Menu;
                    Step::Next
                }
            },
            Stage::Menu => match self.menu.step(&mut self.lib) {
                Step::Yield => Step::Yield,
                Step::Next => Step::Next,
                // 0x3B42: line 1 goes on to the backstory.
                Step::Done(1) => {
                    self.stage = Stage::StoryBegin;
                    Step::Next
                }
                Step::Done(2) => {
                    self.stage = Stage::Done(Choice::Continue);
                    Step::Next
                }
                Step::Done(_) => {
                    self.stage = Stage::Done(Choice::Quit);
                    Step::Next
                }
            },
            Stage::Done(c) => Step::Done(c),
            _ => self.story_stage(),
        }
    }

    /// The backstory's stages, from its picture to its fade out
    /// (`0x3373`–`0x355B`).
    fn story_stage(&mut self) -> Step<Choice> {
        match self.stage {
            Stage::StoryBegin => {
                // 0x3373: hidden, blanked, the slots; 0x33CC the picture.
                self.lib.shown = false;
                self.lib.blank();
                draw::blit_masked(&mut self.lib.screen, &self.story_pic, 0, 0, None);
                self.stage = Stage::StoryIn(Fade::full(self.story_pal, STORY_STEP));
                Step::Next
            }
            Stage::StoryIn(f) => match self.lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::StoryIn(f);
                    Step::Next
                }
                Faded::Done => {
                    self.stage = Stage::StoryReady;
                    Step::Next
                }
            },
            Stage::StoryReady => {
                // 0x3486–0x3490: thawed, the keys flushed, one poll.
                self.lib.flush_keys();
                self.lib.poll();
                self.story_page(0);
                self.stage = Stage::Page { page: 0 };
                Step::Next
            }
            Stage::Page { page } => {
                // 0x34FD: until the poll answers anything at all.
                let mut page = page;
                loop {
                    if self.lib.poll() != 0 {
                        page += 1;
                        if page >= self.story.len() {
                            // 0x3529: hidden; 0x355B the picture again,
                            // to the screen, and the fade out.
                            self.lib.shown = false;
                            draw::blit_masked(&mut self.lib.screen, &self.story_pic, 0, 0, None);
                            self.stage = Stage::StoryOut(Fade::full(self.story_pal, -STORY_STEP));
                            return Step::Next;
                        }
                        self.story_page(page);
                        self.stage = Stage::Page { page };
                    }
                    if self.lib.idle() {
                        return Step::Yield;
                    }
                }
            }
            Stage::StoryOut(f) => match self.lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::StoryOut(f);
                    Step::Next
                }
                Faded::Done => {
                    self.stage = Stage::Done(Choice::New);
                    Step::Next
                }
            },
            _ => unreachable!("`run` hands only the backstory's stages here"),
        }
    }

    /// `0x3497`–`0x34CB`: a page — the picture's band copied into the
    /// surface, the lines laid on it, the band copied to the screen.
    fn story_page(&mut self, page: usize) {
        let pic = &self.story_pic;
        let lib = &mut self.lib;
        for y in STORY_Y..STORY_Y + STORY_H {
            for x in 0..SURFACE_W {
                let p = pic.pixels[num::index(y * i32::from(pic.width) + x)];
                lib.surface.set(x, y, p);
            }
        }
        for (i, text) in self.story[page].iter().enumerate() {
            Lib::line(
                &mut lib.surface,
                &lib.common.font,
                text,
                CX,
                STORY_Y + STORY_ROW * num::count(i),
                INK_STORY,
            );
        }
        for y in STORY_Y..STORY_Y + STORY_H {
            for x in 0..SURFACE_W {
                if let Some(p) = lib.surface.get(x, y) {
                    lib.screen.set(x, y, p);
                }
            }
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
    use crate::titles::parsec1994::title::menu::{INK_GREY, INK_HOVER};
    use crate::titles::parsec1994::title::{KEY_ESCAPE, scale_channel};

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
            .menu
            .line_texts()
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
        assert_eq!(x.story.len(), 6);
        assert!(x.story.iter().all(|p| p.len() == 2));
        assert_eq!(
            motionvm_parsec_formats::cp437_to_string(&x.story[5][0]),
            "Aber laß Dich nicht unterkriegen!"
        );
        assert_eq!(x.lib.common.limits, [0, 0, 314, 194], "XP01's limits");
        assert_eq!(x.lib.mouse, (160, 100), "XP01's start");
    }

    /// No line the program measures — the menu's three, the backstory's
    /// twelve — holds a byte below `20h`, so `0546h:01A3h` meets a control
    /// byte in none of them.
    #[test]
    fn the_measured_lines_hold_no_control_byte() {
        let Some(x) = begun(false) else { return };
        let lines: Vec<&Vec<u8>> = x
            .menu
            .line_texts()
            .iter()
            .chain(x.story.iter().flatten())
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
        let (columns, masks, pal) = x.intro.wave_parts();
        assert_eq!(columns.len(), 287);
        assert!(columns.iter().all(|c| (c.width, c.height) == (1, 123)));
        assert_eq!((SURFACE_W - 287) >> 1, 16);
        assert_eq!(masks.len(), 8);
        assert!(masks.iter().all(|m| m.len() == 320));
        assert_eq!(x.lib.palettes[pal][1], 0xC0, "192 colors");
    }

    /// `01F4h:0029h`: rounded at half a step, held at 3Fh, level 0 black.
    #[test]
    fn a_channel_is_scaled_and_rounded_at_half() {
        assert_eq!(scale_channel(63, 0x100), 63);
        assert_eq!(scale_channel(63, 4), 1, "252 / 256 rounds up");
    }

    /// A fade is one level a retrace — level 0 at the first, 4 at the
    /// second — and its last level is written twice, the second time at
    /// the 66th retrace.
    #[test]
    fn a_fade_writes_one_level_a_retrace() {
        let Some(mut x) = begun(false) else { return };
        let pal = x.lib.palettes[x.intro.logo_pal()].clone();
        let entry = |x: &Xp00, i: usize| x.lib.dac.raw[i];
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
        let full: Vec<u8> = x.lib.palettes[x.intro.logo_pal()][2..].to_vec();
        let mut at_full = 0;
        for _ in 0..300 {
            x.tick();
            if x.lib.dac.raw[..full.len()] == full[..] {
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
        assert!(x.lib.shown, "the pointer is drawn");
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
        assert!(!x.lib.shown, "no pointer over the backstory");
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
