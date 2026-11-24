//! What *Dunkle Schatten*'s `XP01` shows after its main loop, before it
//! exits with code 0 (`_main`, `0x4FF0`–`0x501B`), out of
//! `RESOURCE\EXTROS.DAT` (`DS:0AC2h`, opened at `0x1309E`):
//!
//! - **The game's end**, location −1 (`12D3h:0636h`, `0x13366`): the
//!   campaign's banner (slots 2/3) faded in, the credits rolled over it
//!   to module 4 until an event, the banner faded out; then the campaign's
//!   picture (slots 0/1) faded in, held until an event, faded out.
//! - **The quit**, any other location below zero but −4, which the
//!   options' quit leaves (`12D3h:03A7h`, `0x130D7`): the campaign's
//!   picture, then the banner, each held until an event.
//! - Location −4, a developer's key (`0x0FD00`), shows nothing.
//!
//! Each fade is `12D3h:00DCh`'s: the slot's palette, written from DAC
//! entry 0, a level a retrace, by 4 or 8; the checks between its steps see
//! an event only once the mask is set (`[1E82h]`), and an event latched
//! there (`[1E84h]`) stays for the wait after it. The pointer is hidden
//! throughout: the thaw at each wait (`179Eh:091Ah`) draws it only when it
//! was shown, and `179Eh:0882h` took that away.

use motionvm_parsec_formats::Picture;
use motionvm_render::Framebuffer;

use crate::draw;
use crate::error::Result;
use crate::exe::Image;
use crate::exe::op::PUSH_IMM16;
use crate::titles::parsec1994::Request;
use crate::titles::parsec1994::title::host::{Ending, Exit};
use crate::titles::parsec1994::title::intro::{Rect, copy_rect};
use crate::titles::parsec1994::title::menu::MUSIC_FADE;
use crate::titles::parsec1994::title::roll::{Roll, RollSpec};
use crate::titles::parsec1994::title::{
    Fade, Lib, Pal, Slots, Step, fade_stage, far_lines, picture_fb,
};

/// The location words `_main` tells apart (`0x4FF8`, `0x4FFD`).
const END: i16 = -1;
const NOTHING: i16 = -4;
/// The slots (`0x13398`, `0x133B8`; `0x135CA`, `0x135EA`).
const STILL_PIC: usize = 0;
const STILL_PAL: usize = 1;
const BANNER_PIC: usize = 2;
const BANNER_PAL: usize = 3;
/// `0x133D2`: `push 4`, the credits' module, played endlessly (`0x1349E`).
const SONG: u16 = 4;
/// `0x133F7`, `0x1355E`: the banner faded by 4; `0x1369A`, `0x136E6`: the
/// picture by 8, and the quit's both (`0x131D7`, `0x13223`, `0x132DA`,
/// `0x13310`).
const BANNER_STEP: i32 = 4;
const STILL_STEP: i32 = 8;
/// The window the credits roll through (`0x13456`–`0x1346B`: the clip
/// `3Ch, 14h`–`104h, 0B4h`; `0x134BA`–`0x134CC` and `0x134DA`–`0x134E6`:
/// the band restored and shown, `0C8h` by `0A0h`).
const WINDOW: Rect = Rect {
    x: 0x3C,
    y: 0x14,
    w: 0xC8,
    h: 0xA0,
};
/// The layout (`12D3h:010Ah`): the first line at `0A0h` (`0x12E73`), a
/// `c` on by `0Eh`, an `n` by `0Fh`, a `p` by `10h` (`0x12F0D`, `0x12EDC`,
/// `0x12F08`), centered in `0C8h` from `3Ch` (`0x12EF5`–`0x12EFC`); drawn
/// from the window's top (`0x13020`: `add ax, 14h`).
const ROLL: RollSpec = RollSpec {
    start: 0xA0,
    advance: [0x0E, 0x0F, 0x10],
    window: WINDOW,
};
/// `0x12E41`: `push 0AD6h`, the credits' lines.
const LINES_AT: usize = 0x12E41;
/// `0x13502`: `cmp word [1C82h], 3` — a line a step each three hundredths
/// at most, the clock set to 0 before the step is drawn (`0x134AE`).
const ROLL_STEP: u64 = 3;
/// The mask that lets the checks see an event (`0x136C7`: `mov word
/// [1E82h], 0FFFFh`).
const UNMASKED: u16 = 0xFFFF;

/// The sequence `_main` plays for the location word its loop ended on.
pub(super) fn open(exit: &Exit<'_>) -> Result<Option<Box<dyn Ending>>> {
    let first = match exit.location {
        NOTHING => return Ok(None),
        END => Stage::End,
        _ => Stage::Quit,
    };
    Ok(Some(Box::new(Extros::open(exit, first)?)))
}

/// Where the sequence is.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// `0x1336C`–`0x133F2`: the mask off, hidden, the blank, the banner
    /// onto the screen.
    End,
    BannerIn(Fade),
    /// `0x13494`–`0x134A6`: the roll begun.
    RollBegin,
    /// A step of the roll drawn at `pos`, shown from `since`.
    Roll {
        pos: i32,
        since: u64,
    },
    BannerOut(Fade),
    /// `0x1357F`–`0x13584`: the music stopped.
    BannerGone,
    /// `0x135B3`: the blank, the picture.
    Still,
    StillIn(Fade),
    /// `0x136B7`–`0x136C7`: the mask on for the wait.
    StillThaw,
    StillWait,
    StillOut(Fade),
    /// `0x130DB`–`0x131CF`: the mask off, hidden, the blank, the picture.
    Quit,
    QuitStillIn(Fade),
    QuitStillThaw,
    QuitStillWait,
    QuitStillOut(Fade),
    /// `0x13264`–`0x132D4`: the banner, and the mask on before its fade.
    QuitBanner,
    QuitBannerIn(Fade),
    QuitBannerWait,
    QuitBannerOut(Fade),
    Over,
}

/// The sequence while it plays.
struct Extros {
    lib: Lib,
    still: Picture,
    still_pal: Pal,
    /// The banner as the back buffer holds it, which the roll's window is
    /// restored from.
    banner: Framebuffer,
    banner_pal: Pal,
    roll: Roll,
    stage: Stage,
}

impl Extros {
    fn open(exit: &Exit<'_>, first: Stage) -> Result<Self> {
        let mut lib = Lib::for_ending(exit)?;
        let extros = Slots::open(exit.res, "EXTROS.DAT")?;
        let still_pal = lib.add_palette(extros.palette(STILL_PAL)?);
        let banner_pal = lib.add_palette(extros.palette(BANNER_PAL)?);
        let roll = Roll::layout(&lines(exit.image)?, &lib.common.font, ROLL);
        Ok(Extros {
            still: extros.picture(STILL_PIC)?,
            still_pal,
            banner: picture_fb(&extros.picture(BANNER_PIC)?),
            banner_pal,
            roll,
            stage: first,
            lib,
        })
    }

    /// A fade step, the stage it goes on in made by `on` and the one after
    /// it `then`.
    fn fade(&mut self, f: Fade, on: impl FnOnce(Fade) -> Stage, then: Stage) -> Step<()> {
        fade_stage(&mut self.lib, &mut self.stage, f, on, then)
    }

    /// The waits for an event (`0x136CD`, `0x1320A`, `0x132F7`): until a
    /// check sees one, then the mask off, hidden (`0x136D5`–`0x136E1`).
    fn wait(&mut self, then: Stage) -> Step<()> {
        if !self.lib.check() {
            return Step::Yield;
        }
        self.lib.skip = 0;
        self.lib.skip_mask = 0;
        self.lib.hide();
        self.stage = then;
        Step::Next
    }

    /// Before a wait: the pointer thawed, the keys flushed, the event
    /// cleared and the mask on (`0x136B7`–`0x136C7`).
    fn unmask(&mut self) {
        self.lib.thaw();
        self.lib.flush_keys();
        self.lib.skip = 0;
        self.lib.skip_mask = UNMASKED;
    }

    /// The first steps of both sequences: the mask off, hidden, the blank.
    fn begin(&mut self) {
        self.lib.skip = 0;
        self.lib.skip_mask = 0;
        self.lib.hide();
        self.lib.blank();
    }

    /// A picture into the back buffer and the whole of it onto the screen
    /// (`0x1367C`, `0x13692`; `0x131B9`, `0x131CF`; `0x132A6`, `0x132BC`).
    fn show(&mut self, still: bool) {
        if still {
            draw::blit_masked(&mut self.lib.surface, &self.still, 0, 0, None);
        } else {
            self.lib.surface = self.banner.clone();
        }
        self.lib.screen = self.lib.surface.clone();
    }

    /// The sequence, as far as it goes before this retrace is used.
    fn sequence(&mut self) -> Step<()> {
        match self.stage {
            Stage::End => {
                // 0x133EA: the banner straight onto the screen, which is
                // off; 0x133F2: on; 0x1340E: faded in by 4.
                self.begin();
                self.lib.screen = self.banner.clone();
                self.stage = Stage::BannerIn(Fade::full(self.banner_pal, BANNER_STEP));
                Step::Next
            }
            Stage::BannerIn(f) => self.fade(f, Stage::BannerIn, Stage::RollBegin),
            Stage::RollBegin => {
                // 0x13494: thawed; 0x13499: the keys flushed; 0x134A6:
                // module 4, endlessly; 0x134AE: the first step.
                self.lib.thaw();
                self.lib.flush_keys();
                self.lib.music.push(Request::Play {
                    piece: SONG,
                    loops: -1,
                });
                self.roll_step(0);
                self.stage = Stage::Roll {
                    pos: 0,
                    since: self.lib.now(),
                };
                Step::Next
            }
            Stage::Roll { pos, since } => self.roll(pos, since),
            Stage::BannerOut(f) => self.fade(f, Stage::BannerOut, Stage::BannerGone),
            Stage::BannerGone => {
                // 0x1357F, 0x13584: the song stopped, `StopAll`, which cuts
                // the music's fade short.
                self.lib.music.push(Request::Stop);
                self.lib.music.push(Request::StopEffects);
                self.stage = Stage::Still;
                Step::Next
            }
            Stage::Still => {
                // 0x135B3: the blank; 0x136B1: the picture in by 8, the
                // mask still off.
                self.lib.blank();
                self.show(true);
                self.stage = Stage::StillIn(Fade::full(self.still_pal, STILL_STEP));
                Step::Next
            }
            Stage::StillIn(f) => self.fade(f, Stage::StillIn, Stage::StillThaw),
            Stage::StillThaw => {
                self.unmask();
                self.stage = Stage::StillWait;
                Step::Next
            }
            Stage::StillWait => {
                let out = Stage::StillOut(Fade::full(self.still_pal, -STILL_STEP));
                self.wait(out)
            }
            Stage::StillOut(f) => self.fade(f, Stage::StillOut, Stage::Over),
            Stage::Quit => {
                // 0x131ED: the picture in by 8, the mask off.
                self.begin();
                self.show(true);
                self.stage = Stage::QuitStillIn(Fade::full(self.still_pal, STILL_STEP));
                Step::Next
            }
            Stage::QuitStillIn(f) => self.fade(f, Stage::QuitStillIn, Stage::QuitStillThaw),
            Stage::QuitStillThaw => {
                // 0x131F4–0x13204.
                self.unmask();
                self.stage = Stage::QuitStillWait;
                Step::Next
            }
            Stage::QuitStillWait => {
                let out = Stage::QuitStillOut(Fade::full(self.still_pal, -STILL_STEP));
                self.wait(out)
            }
            Stage::QuitStillOut(f) => self.fade(f, Stage::QuitStillOut, Stage::QuitBanner),
            Stage::QuitBanner => {
                // 0x132A6, 0x132BC: the banner shown with the DAC still
                // black, no blank between; 0x132C4–0x132D4: the mask on
                // before its fade, so an event cuts the fade short and is
                // still latched for the wait after it.
                self.show(false);
                self.unmask();
                self.stage = Stage::QuitBannerIn(Fade::full(self.banner_pal, STILL_STEP));
                Step::Next
            }
            Stage::QuitBannerIn(f) => self.fade(f, Stage::QuitBannerIn, Stage::QuitBannerWait),
            Stage::QuitBannerWait => {
                let out = Stage::QuitBannerOut(Fade::full(self.banner_pal, -STILL_STEP));
                self.wait(out)
            }
            Stage::QuitBannerOut(f) => self.fade(f, Stage::QuitBannerOut, Stage::Over),
            Stage::Over => Step::Done(()),
        }
    }

    /// The roll's wait after the step at `pos`, drawn from `since`
    /// (`0x134F4`–`0x1350B`): polled until three hundredths have passed or
    /// an event came, which ends the roll at once (`0x13517`), no other
    /// step drawn.
    fn roll(&mut self, pos: i32, since: u64) -> Step<()> {
        let mut event = 0;
        while event == 0 && self.lib.now() - since < ROLL_STEP {
            event = self.lib.poll();
            if event == 0 && self.lib.idle() {
                return Step::Yield;
            }
        }
        if event != 0 {
            // 0x1351B: hidden; 0x1354D: the banner without the roll;
            // 0x13558: the music faded over 2 000 ms; 0x13575: the banner
            // out by 4.
            self.lib.hide();
            self.lib.screen = self.banner.clone();
            self.lib.music.push(Request::FadeOut(MUSIC_FADE));
            self.stage = Stage::BannerOut(Fade::full(self.banner_pal, -BANNER_STEP));
            return Step::Next;
        }
        // 0x1350D–0x13515: on a line, round at the roll's end. The clock is
        // set to 0 at once after the hundredth that ended the wait, so a
        // step is three hundredths to the count.
        let next = if pos + 1 >= self.roll.end() {
            0
        } else {
            pos + 1
        };
        self.roll_step(next);
        self.stage = Stage::Roll {
            pos: next,
            since: since + ROLL_STEP,
        };
        Step::Yield
    }

    /// One step of the roll at `pos` (`0x134AE`–`0x134E8`): the window's
    /// band of the banner into the back buffer, the lines in view over it,
    /// the window onto the screen.
    fn roll_step(&mut self, pos: i32) {
        let lib = &mut self.lib;
        copy_rect(&self.banner, &mut lib.surface, WINDOW);
        self.roll.draw(&mut lib.surface, &lib.common.font, pos);
        copy_rect(&lib.surface, &mut lib.screen, WINDOW);
    }
}

/// The credits' lines: the far pointers `push imm16` names at `0x12E41`,
/// up to the null one (`1EBDh:0000h` counts them).
fn lines(image: &Image) -> Result<Vec<Vec<u8>>> {
    far_lines(image, LINES_AT, &PUSH_IMM16, "the credits' lines")
}

impl Ending for Extros {
    fn lib(&self) -> &Lib {
        &self.lib
    }

    fn lib_mut(&mut self) -> &mut Lib {
        &mut self.lib
    }

    fn run(&mut self) -> Step<()> {
        self.sequence()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::titles::ds1::build::BUILD;
    use motionvm_render::Palette;

    /// The sequence for `location`, as the main loop of the install would
    /// leave it, or `None` without one.
    fn opened(location: i16) -> Option<Extros> {
        let dir = motionvm_parsec_testutil::gamedata_ds1()?;
        let image = Image::open(&dir, BUILD.program.0, BUILD.program.1).unwrap();
        let res = motionvm_parsec_formats::detect::resource_dir(&dir);
        let exit = Exit {
            res: &res,
            image: &image,
            pointer: BUILD.pointer,
            location,
            screen: draw::display(),
            dac: Palette::default(),
        };
        let first = match location {
            END => Stage::End,
            _ => Stage::Quit,
        };
        Some(Extros::open(&exit, first).unwrap())
    }

    /// `n` retraces, every one going on.
    fn ticks(x: &mut Extros, n: usize) {
        for _ in 0..n {
            assert!(x.tick(), "the sequence goes on");
        }
    }

    /// Retraces until the sequence is over; how many it took.
    fn to_the_end(x: &mut Extros) -> usize {
        (1..=1000).find(|_| !x.tick()).expect("the sequence ends")
    }

    /// The DAC at full: the slot's 128 colours from entry 0.
    fn full(x: &Extros, pal: Pal) -> bool {
        let bytes = &x.lib.palettes[pal];
        x.lib.dac.raw[..0x180] == bytes[2..2 + 0x180]
    }

    /// The quit (`12D3h:03A7h`): the picture faded in by 8 — 33 levels and
    /// the last once more — and held; an event, and it fades out and the
    /// banner fades in, held as well; an event, and it is over.
    #[test]
    fn the_quit_holds_the_picture_then_the_banner() {
        let Some(mut x) = opened(-2) else {
            return;
        };
        ticks(&mut x, 34);
        assert!(full(&x, x.still_pal));
        assert_eq!(x.lib.screen.pixels, picture_fb(&x.still).pixels);
        ticks(&mut x, 500);
        assert!(matches!(x.stage, Stage::QuitStillWait), "held");
        x.click(true);
        ticks(&mut x, 34 + 34);
        assert!(full(&x, x.banner_pal));
        assert_eq!(x.lib.screen.pixels, x.banner.pixels);
        ticks(&mut x, 500);
        // A key the library's hook leaves to the BIOS: `a`.
        x.key(0x61);
        assert_eq!(to_the_end(&mut x), 34, "faded out by 8");
        assert!(x.take_music().is_empty(), "the quit leaves the music be");
    }

    /// The banner's fade is the one a check sees an event in: the event
    /// cuts it short, with the last level written, and stays latched, so
    /// the wait after it is over at once.
    #[test]
    fn an_event_in_the_quits_banner_fade_ends_its_wait_too() {
        let Some(mut x) = opened(-2) else {
            return;
        };
        ticks(&mut x, 40);
        x.click(true);
        ticks(&mut x, 34 + 5);
        x.click(true);
        // The step that sees it, then the last level, and the wait passes.
        ticks(&mut x, 2);
        assert!(full(&x, x.banner_pal), "cut short");
        assert!(matches!(x.stage, Stage::QuitBannerOut(_)));
        assert_eq!(to_the_end(&mut x), 34);
    }

    /// `12D3h:010Ah` lays the 55 lines of `DS:0AD6h` out over 1443
    /// positions.
    #[test]
    fn the_credits_are_laid_out_from_the_program() {
        let Some(x) = opened(END) else {
            return;
        };
        assert_eq!((x.roll.len(), x.roll.end()), (55, 1443));
    }

    /// The end (`12D3h:0636h`): the banner faded in by 4, module 4 then,
    /// the roll a line each three hundredths until an event; the banner
    /// out by 4, the song stopped, the picture in by 8 and held; an
    /// event, and it is over.
    #[test]
    fn the_end_rolls_the_credits_until_an_event() {
        let Some(mut x) = opened(END) else {
            return;
        };
        ticks(&mut x, 66);
        assert!(full(&x, x.banner_pal));
        assert_eq!(
            x.take_music(),
            [Request::Play {
                piece: SONG,
                loops: -1
            }]
        );
        let Stage::Roll { since, .. } = x.stage else {
            panic!("the roll");
        };
        ticks(&mut x, 700);
        let Stage::Roll { pos, .. } = x.stage else {
            panic!("the roll goes on by itself");
        };
        let lines = (x.lib.now() - since) / ROLL_STEP;
        assert_eq!(i32::try_from(lines).unwrap(), pos);
        // The window shows the roll at `pos` over the banner, the rest of
        // the screen the banner alone.
        let mut want = x.banner.clone();
        x.roll.draw(&mut want, &x.lib.common.font, pos);
        assert_eq!(x.lib.screen.pixels, want.pixels);
        assert_ne!(want.pixels, x.banner.pixels, "a line is in view");

        x.click(true);
        ticks(&mut x, 1);
        assert_eq!(
            x.lib.screen.pixels, x.banner.pixels,
            "the banner without the roll"
        );
        ticks(&mut x, 65 + 34);
        assert_eq!(
            x.take_music(),
            [
                Request::FadeOut(MUSIC_FADE),
                Request::Stop,
                Request::StopEffects
            ]
        );
        assert!(full(&x, x.still_pal));
        assert_eq!(x.lib.screen.pixels, picture_fb(&x.still).pixels);
        ticks(&mut x, 500);
        x.click(false);
        assert_eq!(to_the_end(&mut x), 34);
    }

    /// A click while the banner fades in, the pointer hidden, is not
    /// sampled (`0x13378`, the thaw at `0x13494`): the roll goes on.
    #[test]
    fn a_click_in_the_banners_fade_is_not_seen() {
        let Some(mut x) = opened(END) else {
            return;
        };
        ticks(&mut x, 10);
        x.click(true);
        ticks(&mut x, 66 + 10);
        assert!(
            matches!(x.stage, Stage::Roll { pos, .. } if pos > 0),
            "the roll goes on"
        );
    }

    /// The buttons are flags: two presses before a poll are one event, a
    /// button comes before a waiting key, and a hidden pointer samples
    /// nothing (the library's, `0x184FC`, `0x17B39`, `0x17B53`).
    #[test]
    fn buttons_are_latched_not_queued() {
        let Some(x) = opened(END) else {
            return;
        };
        let mut lib = x.lib;
        lib.key(0x61);
        lib.click(true);
        lib.click(true);
        assert_eq!(lib.poll(), 5);
        assert_eq!(lib.poll(), 0xFFFF);
        assert_eq!(lib.poll(), 0);
        lib.click(true);
        lib.click(false);
        assert_eq!(lib.poll(), 7);
        lib.hide();
        lib.click(true);
        lib.thaw();
        assert!(lib.idle());
    }
}
