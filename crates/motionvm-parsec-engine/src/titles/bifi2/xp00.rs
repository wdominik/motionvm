//! `XP00`, *Bi-Fi 2*'s title program.
//!
//! `BIFI.BAT` runs `xp00.exe`, and `xp01.exe` only when it left errorlevel
//! 0. This module is the program's own part, read from the unpacked
//! `XP00.EXE` (62 592 bytes, `DGROUP` `0DD7h`, image offsets below); what
//! its library does — the machine, the fades, the logo routine, the menu —
//! is the generation's ([`crate::titles::parsec1994::title`]). Its pointer
//! library is Captain Zins's (`0888h` here, `0970h` there, the same entry
//! offsets), its `DGROUP` words `0Ah` below the other two titles'.
//!
//! `_main` (`005Ch:0008h`, `0x5C8`) reads the mouse setting (`0x6A4`),
//! probes `SAVEGAME.SAV` (`0x626`–`0x646`, byte `[10BCh]`), opens the music
//! library (`0x651`) and calls the title routine `0311h:059Dh` (`0x36AD`),
//! which runs, in order:
//!
//! 1. the pointer hidden, `INTROS.DAT` opened;
//! 2. an animation, slot 8 (`0311h:0134h`, `0x3244`): its palette written
//!    at once and 36 frames of deltas, five hundredths each, which nothing
//!    skips;
//! 3. the song started;
//! 4. two stills through the logo routine (`0311h:02C1h`, `0x33D1`, Dunkle
//!    Schatten's `0x3631`): slots 0/1, then 2/3 — no wave;
//! 5. the title picture, slots 4/5, faded in by 2, which nothing skips;
//! 6. the menu (`0311h:0342h`, `0x3452`) until it answers: three lines, 1 a
//!    new game, 2 the saved one, 3 or Escape quitting; the picture faded
//!    out by 8;
//! 7. back in `_main`: 1 deletes `SAVEGAME.SAV` (`0x66F`, without looking
//!    at the probe), 1 and 2 leave exit code 0, 3 leaves −1 (`0x68A`).
//!
//! With `/JUKEBOX` on the command line and a music driver the third line
//! reads "Ab geht's zur Jukebox!" (`0x61C`) and choosing it runs a jukebox
//! (`0311h:0BB5h`) before the exit; no host passes a command line, so the
//! menu is the one above.
//!
//! **Music.** Module 0 of `PSMP2L.DAT`, loaded and started endlessly as the
//! animation ends (`047Fh:028Ah(0, …)` at `0x3709`, `0076h:065Ah(…, −1)` at
//! `0x371B`), under the stills, the title and the menu; faded over 2 000 ms
//! when a line is chosen (`0x3812`) and stopped once the picture has faded
//! out (`0x3860`, `0x3865`).

use std::path::Path;

use motionvm_parsec_formats::detect::resource_dir;
use motionvm_parsec_formats::{Rib, to_cp437};

use crate::error::Result;
use crate::exe::Image;
use crate::exe::op::PUSH_BX_MEM;
use crate::titles::parsec1994::KeyMap;
use crate::titles::parsec1994::Request;
use crate::titles::parsec1994::title::anim::apply_delta;
use crate::titles::parsec1994::title::host::TitleProgram;
use crate::titles::parsec1994::title::intro::Intro;
use crate::titles::parsec1994::title::menu::{Menu, MenuSpec};
use crate::titles::parsec1994::title::{
    Choice, Common, Fade, Faded, Lib, Pal, Slots, Step, TitleKeys,
};

/// The build these addresses were read on.
const IMAGE_LEN: usize = 62_592;

/// `0x36D4`: `push 8`, the animation's slot; `0x325C`: `dec ax` — its
/// first resource the palette, every other one a frame.
const ANIM_SLOT: usize = 8;
/// `0x330B`: `cmp word [13E4h], 5` — a frame each five hundredths;
/// `0x3324`: one more hundredth after the last.
const ANIM_FRAME: u64 = 5;
const ANIM_TAIL: u64 = 1;
/// `0x332B`: `push 0FFFCh`, the fade out's step.
const ANIM_OUT_STEP: i32 = -4;
/// `0x3348`: `push 0`, what the screen is filled with after the fade out.
const ANIM_FILL_OUT: u8 = 0;
/// `0x2834`: `push 0` — the blank fills the screen with 0, where Dunkle
/// Schatten's `0x28F9` pushes `0FEh`.
const BLANK_FILL: u8 = 0;
/// `0x373E` and `0x3752`: `push 0`, `push 2` — the two stills' slots, each
/// followed by its palette (`0x3406`: `inc si`); `0x3739`/`0x373B`: step
/// 4, held `96h` hundredths, as the logo routine's other caller in Dunkle
/// Schatten's.
const STILLS: [usize; 2] = [0, 2];
/// `0x3775`/`0x3793`: the title picture and its palette.
const TITLE_PIC: usize = 4;
const TITLE_PAL: usize = 5;
/// `0x37BB`: `push 2`, the title's fade in; `0x3818`: `push 0FFF8h`, the
/// fade out once a line is chosen.
const TITLE_STEP: i32 = 2;
const TITLE_OUT_STEP: i32 = -8;
/// `0x3705`/`0x3707`: the module's dword pushed as 0.
const SONG: u16 = 0;
/// The menu: first line `9Ah` (`0x3472`), `0Fh` apart (`0x3547`), three
/// lines (`0x354C`: `cmp di, 3`); line 2 gray without a saved game
/// (`0x3554`–`0x357C`); Escape the third (`0x3690`).
const MENU_Y: i32 = 0x9A;
const MENU_ROW: i32 = 0x0F;
const MENU_LINES: usize = 3;
const MENU_GREY: usize = 1;
const MENU_ESCAPE: u16 = 3;
/// The inks: `0x345C` idle `7Eh`, `0x35E5` hovered `74h`, `0x355F` gray
/// `70h` — in `CMBMOU.DAT`'s bank, which here is `60h`–`7Fh`.
const MENU_INKS: [u8; 3] = [0x7E, 0x74, 0x70];
/// A line's box: one row above the line (`0x34D5`: `dec ax`) and two rows
/// taller than the font (`0x3502`: `add ax, 2`).
const MENU_BOX_UP: i32 = 1;
const MENU_BOX_TALLER: i32 = 2;
/// `push word [bx+imm]`: the low word of a menu line's far pointer
/// (`0x3482`).
const MENU_ROW_AT: usize = 0x3482;

/// The pointer library's keyboard in `XP00` (`crate::titles::parsec1994::keyboard`),
/// Captain Zins's code at `0xE80` below its place there.
const KEYS: TitleKeys = TitleKeys {
    map: KeyMap {
        // `0x0D21`–`0x0D25`: `push ds ; push 0B2h ; call far 0888h:049Fh`.
        list: 0x00B2,
        // `cs:0402h`, `0x8D82`: `cmp byte [cs:0402h], 0`.
        switch: 0x08880 + 0x0402,
        // `0x8DB6`: `mov di, 1080h`.
        speeds: 0x1080,
        // `0x8DAF`, `0x8DC1`: `[252h]`, `[254h]`, `[256h]`.
        limits: 0x0252,
        // `0x953C`: `mul word [262h]`.
        acceleration: 0x0262,
        // `0x95E9`: `idiv word [264h]`; `0x95A9`: `idiv word [266h]`.
        divisors: 0x0264,
    },
    // `0x0CF1`: `push word [19Eh]`, the interval the slot is registered with.
    interval: 0x019E,
    // The pointer's limits and position (`0x9601`/`0x9609` `[194h]`/`[198h]`
    // around x `[270h]`, `0x95C2`/`0x95CA` `[196h]`/`[19Ah]` around y
    // `[272h]`): `0Ah` below Dunkle Schatten's `019Eh`/`027Ah`.
    limits: 0x0194,
    position: 0x0270,
};

/// Where the title routine is.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// `0x36B1`: hidden; `0x3249`: the blank, the palette, the screen on.
    Anim,
    /// Frame `k` of the 36, its clock started at hundredth `at`: unpacked
    /// then, drawn at the next retrace.
    AnimFrame {
        k: usize,
        at: u64,
    },
    /// `0x3319`–`0x3329`: until hundredth `until`.
    AnimTail {
        until: u64,
    },
    AnimOut(Fade),
    /// `0x36FD`–`0x3733`: the song, and the intro's start.
    Intro,
    /// `0x376D`–`0x37E4`: the title picture faded in by 2 — the menu's own
    /// `Begin`/`In` with no blank.
    Menu,
    Done(Choice),
}

/// The title program while it runs.
pub(crate) struct Xp00 {
    lib: Lib,
    /// Slot 8: the palette, `start, count, triples`, and the deltas, each
    /// unpacked.
    anim_pal: Pal,
    frames: Vec<Vec<u8>>,
    intro: Intro,
    menu: Menu,
    stage: Stage,
}

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
        let mut lib = Lib::new(Common::read(&image, &res, KEYS)?);
        lib.blank_fill = BLANK_FILL;
        let intros = Slots::open(&res, "INTROS.DAT")?;

        let anim = intros.resources(ANIM_SLOT)?;
        let (palette, deltas) = anim.split_first().ok_or("the animation is empty")?;
        // 0x3285: the palette's own `start, count` header, written from
        // entry 0.
        let anim_pal = lib.add_palette(palette.clone());
        // 0x32E8: `0076h:094Dh` (`0x10AD`) unpacks a frame that begins
        // `RIB\0` (`DS:02A6h`, `0x10BF`) and copies any other.
        let frames = deltas
            .iter()
            .map(|d| {
                Ok(if Rib::looks_like(d) {
                    Rib::parse(d)?.decoded
                } else {
                    d.clone()
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let intro = Intro::logos(&mut lib, &intros, &STILLS)?;
        let title_pal = lib.add_palette(intros.palette(TITLE_PAL)?);
        let menu = Menu::new(
            MenuSpec {
                lines: menu_lines,
                y: MENU_Y,
                row: MENU_ROW,
                grey: MENU_GREY,
                // No song of the menu's own: it plays since 0x371B.
                song: None,
                step: TITLE_STEP,
                wait_dim: None,
                escape: MENU_ESCAPE,
                inks: MENU_INKS,
                box_up: MENU_BOX_UP,
                box_taller: MENU_BOX_TALLER,
                out_step: TITLE_OUT_STEP,
                // No blank before the picture (0x37A9), and the input
                // taken afresh after the lines (0x357F).
                blank: false,
                afresh_after_lines: true,
            },
            intros.picture(TITLE_PIC)?,
            title_pal,
        );
        Ok(Xp00 {
            lib,
            anim_pal,
            frames,
            intro,
            menu,
            stage: Stage::Anim,
        })
    }

    /// Whether the menu is taking input.
    #[cfg(test)]
    pub(crate) fn menu_up(&self) -> bool {
        matches!(self.stage, Stage::Menu) && self.menu.up()
    }

    /// Where line `i` of the menu is, as its box: x, y, w, h.
    #[cfg(test)]
    pub(crate) fn menu_box(&self, i: usize) -> (i32, i32, i32, i32) {
        self.menu.line_box(&self.lib, i)
    }

    /// The title routine, as far as it goes before this retrace is used.
    fn routine(&mut self) -> Step<Choice> {
        match self.stage {
            Stage::Anim => {
                // 0x36B1: hidden — `0888h:0A12h`, the hide that stops the
                // sampling too (Dunkle Schatten's `08E6h:0792h`). 0x3249:
                // the blank; 0x3285: the palette at once, its own count
                // from entry 0; 0x328D: the screen on.
                self.lib.hide();
                self.lib.blank();
                let p = &self.lib.palettes[self.anim_pal];
                let count = usize::from(p.get(1).copied().unwrap_or(0));
                let rgb: Vec<u8> = p.iter().skip(2).take(count * 3).copied().collect();
                self.lib.dac.load(0, &rgb);
                self.stage = Stage::AnimFrame {
                    k: 0,
                    at: self.lib.now(),
                };
                Step::Next
            }
            Stage::AnimFrame { k, at } => {
                // 0x32BA: the clock set to 0 at `at`, the moment the last
                // wait ended (0x330B) — five hundredths to the count.
                if self.lib.now() < at {
                    return Step::Yield;
                }
                let Some(d) = self.frames.get(k) else {
                    // 0x3319: the clock from 0 again, one hundredth more.
                    self.stage = Stage::AnimTail {
                        until: at + ANIM_TAIL,
                    };
                    return Step::Next;
                };
                // 0x32E8: the frame unpacked; 0x21F5: laid on the screen
                // at the start of a retrace.
                if !self.lib.wait_retrace() {
                    return Step::Yield;
                }
                apply_delta(&mut self.lib.screen, d);
                self.stage = Stage::AnimFrame {
                    k: k + 1,
                    at: at + ANIM_FRAME,
                };
                Step::Yield
            }
            Stage::AnimTail { until } => {
                if self.lib.now() < until {
                    return Step::Yield;
                }
                // 0x332B: out by 4. The mask is still the image's 0
                // (`DS:141Ah`), so its checks skip nothing.
                self.stage = Stage::AnimOut(Fade::full(self.anim_pal, ANIM_OUT_STEP));
                Step::Next
            }
            Stage::AnimOut(f) => match self.lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::AnimOut(f);
                    Step::Next
                }
                Faded::Done => {
                    // 0x3348: the screen filled with 0 — no blank.
                    self.lib.screen.fill(ANIM_FILL_OUT);
                    // 0x3709/0x371B: module 0, endlessly.
                    self.lib.music.push(Request::Play {
                        piece: SONG,
                        loops: -1,
                    });
                    // 0x3723: `0888h:0AAAh`, the thaw (Dunkle Schatten's
                    // `08E6h:082Ah`, which its intro calls too).
                    self.lib.thaw();
                    self.stage = Stage::Intro;
                    Step::Next
                }
            },
            Stage::Intro => match self.intro.step(&mut self.lib) {
                // 0x3723–0x3733: thawed, flushed, the skip on; the two
                // stills; 0x3761: the skip off.
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
                // 0x65E–0x68A: 1 a new game, 2 the saved one, else −1.
                Step::Done(v) => {
                    self.stage = Stage::Done(match v {
                        1 => Choice::New,
                        2 => Choice::Continue,
                        _ => Choice::Quit,
                    });
                    Step::Next
                }
            },
            Stage::Done(c) => Step::Done(c),
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
    use crate::titles::parsec1994::title::KEY_ESCAPE;

    /// The program of the install, begun, or `None` without one.
    fn begun(saved: bool) -> Option<Xp00> {
        let Some(dir) = motionvm_parsec_testutil::gamedata_bifi2() else {
            eprintln!("skipping: no Bi-Fi 2 gamedata directory");
            return None;
        };
        let mut x = Xp00::open(&dir).expect("XP00 opens");
        x.begin(saved);
        Some(x)
    }

    /// What the program reads: three menu lines from `DS:09C8h`, 36 frames
    /// after the palette, the pointer's words where this build keeps them.
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
                "Neues Spiel beginnen",
                "Altes Spiel fortsetzen",
                "Spiel sofort beenden"
            ]
        );
        assert_eq!(x.frames.len(), 36);
        assert_eq!(&x.lib.palettes[x.anim_pal][..2], &[0, 0x20]);
        assert_eq!(x.lib.common.limits, [0, 0, 314, 194]);
        assert_eq!(x.lib.mouse, (160, 100));
    }

    /// A line's box is one row above it and two taller than the font
    /// (`0x34D5`, `0x3502`): with the font's 12 rows, 14, from `99h`.
    #[test]
    fn a_line_s_box_frames_it() {
        let Some(x) = begun(false) else { return };
        let (_, y, _, h) = x.menu_box(0);
        assert_eq!(
            (y, h),
            (MENU_Y - 1, i32::from(x.lib.common.font.line_height) + 2)
        );
        let (_, y1, _, _) = x.menu_box(1);
        assert_eq!(y1 - y, MENU_ROW);
    }

    /// Nothing before the stills skips: a click in the animation leaves it
    /// running, and the song starts as it ends.
    #[test]
    fn the_animation_is_not_skipped() {
        let Some(mut x) = begun(false) else { return };
        for _ in 0..10 {
            x.tick();
        }
        x.click(true);
        let mut n = 0;
        while !matches!(x.stage, Stage::Intro) {
            assert!(x.tick().is_none());
            n += 1;
            assert!(n < 1000);
        }
        assert!(x.take_music().iter().any(|r| matches!(
            r,
            Request::Play {
                piece: 0,
                loops: -1
            }
        )));
    }

    /// Escape is the third line.
    #[test]
    fn escape_quits() {
        let Some(mut x) = begun(true) else { return };
        let mut n = 0;
        while !x.menu_up() {
            x.tick();
            n += 1;
            assert!(n < 5000, "the menu never came up");
        }
        x.key(KEY_ESCAPE);
        let end = (0..200).find_map(|_| x.tick());
        assert_eq!(end, Some(Choice::Quit));
    }

    // Tick counts to the menu, and the frames of each part, are left for
    // the comparison with a capture.
}
