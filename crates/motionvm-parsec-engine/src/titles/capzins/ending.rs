//! What *Captain Zins*'s `XP01` shows after its main loop, before it exits
//! with code 0: only for location −1 (`_main`, `0x7B2`; any other location
//! exits at once), the parade of `0E01h:068Bh` (`0x0E69B`) out of
//! `RESOURCE\EXTROS.DAT` (`DS:0A3Eh`, opened at `0x0E240`).
//!
//! The game's last picture fades out, the water (slot 0) fades in to
//! module 3, and then, pass after pass until an event, one of the
//! portraits of slots 6 on rides across it, column by column stretched
//! over one of the eleven masks of slot 5 — from the left, then from the
//! right — behind the pipes of slots 2 and 3, a line of the program's
//! under it. An event ends the pass and the parade, and the screen fades
//! out.
//!
//! The columns, the masks' spans and the rectangles are the library's,
//! the same routines as the title program's wave
//! ([`crate::titles::parsec1994::title::intro`]): `0E01h:0004h` the spans,
//! `0E01h:00DAh` the columns, `0E01h:0AC4h` the union.
//!
//! Its fades are its own loops, not the library's: every DAC entry scaled
//! from the shadow, two levels a retrace from `100h` to 0 or 0 to `100h`
//! and no last write after, no event looked at. The ride waits for no
//! retrace: it draws as often as the machine lets it, where the hundredths
//! since the pass began put the portrait — here once a retrace.

use motionvm_parsec_formats::{Picture, Rib};
use motionvm_render::{Framebuffer, Palette};

use crate::dac::Apply as _;
use crate::draw;
use crate::error::Result;
use crate::exe::Image;
use crate::num;
use crate::titles::parsec1994::Request;
use crate::titles::parsec1994::title::host::{Ending, Exit};
use crate::titles::parsec1994::title::intro::{
    Rect, Span, column, copy_rect, draw_columns, spans, union,
};
use crate::titles::parsec1994::title::menu::MUSIC_FADE;
use crate::titles::parsec1994::title::{
    FULL, Lib, SURFACE_W, Slots, Step, far_lines, picture_fb, scale_channel,
};

/// `0x7B2`: `cmp word [1A5Ch], 0FFFFh` — the one location that has it.
const END: i16 = -1;
/// The slots (`0x0E25B`–`0x0E2BE`, `0x0E337`, `0x0E396`): the water, its
/// bank, the left and right pipes, the masks, the shared bank; the
/// portraits from 6 on (`0x0E883`: `add ax, 6`), as many as the group has
/// slots past them (`0x0E2E2`: `sub ax, 6`).
const WATER: usize = 0;
const WATER_BANK: usize = 1;
const PIPES_LEFT: usize = 2;
const PIPES_RIGHT: usize = 3;
const SHARED_BANK: usize = 4;
const MASKS: usize = 5;
const PORTRAITS: usize = 6;
/// `0x0E312`: `push 3`, the module, played endlessly at once (`0x0E323`).
const SONG: u16 = 3;
/// `0x0E6F0`, `0x0E774`, `0x0EAC5`: the fades' two levels a retrace.
const FADE_STEP: i32 = 2;
/// `0x0E947`: `push 0B4h`, the line's row; `0x0E79D`: its ink.
const TEXT_Y: i32 = 0xB4;
const INK: u8 = 0xE8;
/// `0x0EA16`: `cmp ax, 4` — the next line every four passes.
const PASSES_A_LINE: usize = 4;
/// `0x0E794`: `mov word [bp-12h], 0A52h`, the lines.
const LINES_AT: usize = 0x0E794;
const MOV_BP12_IMM: [u8; 3] = [0xC7, 0x46, 0xEE];

/// The sequence `_main` plays for the location word its loop ended on.
pub(super) fn open(exit: &Exit<'_>) -> Result<Option<Box<dyn Ending>>> {
    if exit.location != END {
        return Ok(None);
    }
    Ok(Some(Box::new(Parade::open(exit)?)))
}

/// One of the portraits: its palette, entry 0 of the slot, and its
/// columns, the entries after it, each `1 × h`.
struct Portrait {
    palette: Vec<u8>,
    columns: Vec<Picture>,
}

/// Where the parade is.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// `0x0E6BB`–`0x0E6F5`: the game's last picture faded out.
    Out {
        level: i32,
    },
    /// `0x0E6F7`–`0x0E73B`: the blank and the parade set up, the screen
    /// on with the DAC black.
    Setup,
    /// `0x0E740`–`0x0E77B`: the water faded in.
    In {
        level: i32,
    },
    /// `0x0E7B6`–`0x0E8D8`: a pass's mask, portrait and palette.
    Pass,
    /// `0x0E8DD`: the palette written at the retrace, the screen built
    /// whole, the ride begun.
    PassShow,
    /// `0x0E9C0`–`0x0EA07`: the ride, from `since`.
    Ride {
        since: u64,
    },
    /// `0x0EA6D`–`0x0EACA`: the music faded, the screen faded out.
    End {
        level: i32,
    },
    Over,
}

/// The parade while it plays.
struct Parade {
    lib: Lib,
    /// The water, which the back buffer is restored from.
    water: Framebuffer,
    water_bank: Vec<u8>,
    shared_bank: Vec<u8>,
    pipes: (Picture, Picture),
    /// For each mask the span of each of its columns.
    masks: Vec<Vec<Span>>,
    portraits: Vec<Portrait>,
    lines: Vec<Vec<u8>>,
    /// `[bp-312h]`: the shadow the fade scales.
    shadow: Palette,
    /// The surface the columns are drawn into (`[1918h]`, over the mask's
    /// buffer, which is cleared once its spans are taken).
    columns: Framebuffer,
    /// `[18E6h]`: every event the ride's polls saw.
    latch: u16,
    /// `si`, `di`: the pass's mask and portrait; `[bp-4]`, `[bp-12h]`:
    /// the passes on the line and the line; `[bp-6]`: whether the ride
    /// goes from the left.
    mask: usize,
    portrait: usize,
    passes: usize,
    line: usize,
    from_left: bool,
    /// `[1908h]`, `[1900h]`: the rectangle the columns covered, and the
    /// one before, kept from pass to pass; 0 in the image.
    drawn: Rect,
    before: Rect,
    stage: Stage,
}

impl Parade {
    fn open(exit: &Exit<'_>) -> Result<Self> {
        let lib = Lib::for_ending(exit)?;
        let extros = Slots::open(exit.res, "EXTROS.DAT")?;
        let water = picture_fb(&extros.picture(WATER)?);
        let cel = |i: usize| -> Result<Picture> {
            match extros.slot(i)? {
                motionvm_parsec_formats::Slot::Rib(r) => Ok(Picture::parse_prefix(&r.decoded)?),
                _ => Err(format!("EXTROS.DAT slot {i} is not a cel").into()),
            }
        };
        let masks = extros
            .resources(MASKS)?
            .iter()
            .map(|m| Ok(spans(&Picture::parse(&Rib::parse(m)?.decoded)?)))
            .collect::<Result<Vec<_>>>()?;
        let portraits = (PORTRAITS..extros.len())
            .map(|i| {
                let entries = extros.resources(i)?;
                let (palette, columns) = entries
                    .split_first()
                    .ok_or_else(|| format!("EXTROS.DAT slot {i} is empty"))?;
                Ok(Portrait {
                    palette: palette.clone(),
                    columns: columns.iter().map(|c| column(c)).collect::<Result<_>>()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if masks.is_empty() || portraits.is_empty() {
            return Err("EXTROS.DAT has no masks or no portraits".into());
        }
        let shadow = exit.dac.clone();
        Ok(Parade {
            water,
            water_bank: extros.palette(WATER_BANK)?,
            shared_bank: extros.palette(SHARED_BANK)?,
            pipes: (cel(PIPES_LEFT)?, cel(PIPES_RIGHT)?),
            masks,
            portraits,
            lines: lines(exit.image)?,
            shadow,
            columns: draw::display(),
            latch: 0,
            mask: 0,
            portrait: 0,
            passes: 0,
            line: 0,
            from_left: false,
            drawn: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            before: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            stage: Stage::Out { level: FULL },
            lib,
        })
        .map(|mut p| {
            // 0x0E6A1: hidden, before the fade.
            p.lib.hide();
            p
        })
    }

    /// The shadow at `level`, every entry: `00D8h:0029h` over 256, then
    /// `pal_set` from 0 (`0x0E6D0`–`0x0E6E8`).
    fn set_level(&mut self, level: i32) {
        let rgb: Vec<u8> = self
            .shadow
            .raw
            .iter()
            .map(|&v| scale_channel(v, level))
            .collect();
        self.lib.dac.load(0, &rgb);
    }

    /// The sequence, as far as it goes before this retrace is used.
    fn sequence(&mut self) -> Step<()> {
        match self.stage {
            Stage::Out { level } | Stage::End { level } => {
                if level < 0 {
                    self.stage = match self.stage {
                        Stage::Out { .. } => Stage::Setup,
                        _ => {
                            // 0x0E490: the song stopped.
                            self.lib.music.push(Request::Stop);
                            Stage::Over
                        }
                    };
                    return Step::Next;
                }
                if !self.lib.wait_retrace() {
                    return Step::Yield;
                }
                self.set_level(level);
                self.stage = match self.stage {
                    Stage::Out { .. } => Stage::Out {
                        level: level - FADE_STEP,
                    },
                    _ => Stage::End {
                        level: level - FADE_STEP,
                    },
                };
                Step::Next
            }
            Stage::Setup => {
                // 0x0E6F7: the blank; 0x0E323: module 3; 0x0E378, 0x0E3D7:
                // the two banks; 0x0E3F9: the water onto the screen.
                self.lib.blank();
                self.lib.music.push(Request::Play {
                    piece: SONG,
                    loops: -1,
                });
                self.lib.dac.apply(&self.water_bank);
                self.lib.dac.apply(&self.shared_bank);
                self.lib.screen = self.water.clone();
                // 0x0E70D: the shadow kept; 0x0E733: the DAC black.
                self.shadow = self.lib.dac.clone();
                self.lib.dac = Palette::default();
                self.stage = Stage::In { level: 0 };
                Step::Next
            }
            Stage::In { level } => {
                if level > FULL {
                    // 0x0E77D: the latch cleared.
                    self.latch = 0;
                    self.stage = Stage::Pass;
                    return Step::Next;
                }
                if !self.lib.wait_retrace() {
                    return Step::Yield;
                }
                self.set_level(level);
                self.stage = Stage::In {
                    level: level + FADE_STEP,
                };
                Step::Next
            }
            Stage::Pass => {
                // 0x0E7B6: the direction turned; 0x0E7BA–0x0E7C8: round.
                self.from_left = !self.from_left;
                if self.mask >= self.masks.len() {
                    self.mask = 0;
                }
                if self.portrait >= self.portraits.len() {
                    self.portrait = 0;
                }
                self.stage = Stage::PassShow;
                Step::Next
            }
            Stage::PassShow => {
                if !self.lib.wait_retrace() {
                    return Step::Yield;
                }
                // 0x0E8CB: the portrait's first colour black; 0x0E90B: its
                // bank, without a fade.
                let mut bank = self.portraits[self.portrait].palette.clone();
                if let Some(rgb) = bank.get_mut(2..5) {
                    rgb.fill(0);
                }
                self.lib.dac.apply(&bank);
                // 0x0E937–0x0E978: the water and the line into the back
                // buffer, the whole of it onto the screen.
                self.lib.surface = self.water.clone();
                self.draw_line();
                self.lib.screen = self.lib.surface.clone();
                // 0x0E9AB–0x0E9BA: thawed, the keys flushed, a poll whose
                // event is dropped, the clock set to 0.
                self.lib.thaw();
                self.lib.flush_keys();
                self.lib.poll();
                self.stage = Stage::Ride {
                    since: self.lib.now(),
                };
                Self::went_on(self.ride(0))
            }
            Stage::Ride { since } => {
                let elapsed = i32::try_from(self.lib.now() - since).unwrap_or(i32::MAX / 2);
                Self::went_on(self.ride(elapsed))
            }
            Stage::Over => Step::Done(()),
        }
    }

    /// A picture of the ride as a step: the pass over goes on at once.
    fn went_on(pass_over: bool) -> Step<()> {
        if pass_over { Step::Next } else { Step::Yield }
    }

    /// One picture of the ride `elapsed` hundredths in, and whether the
    /// pass ended with it (`0x0E9C6`–`0x0EA07`): from the left the
    /// portrait's last column comes in first, `elapsed − columns`, until
    /// it is past the right edge; from the right `320 − elapsed`, until it
    /// is past the left.
    fn ride(&mut self, elapsed: i32) -> bool {
        let cols = num::count(self.portraits[self.portrait].columns.len());
        let (pos, on) = if self.from_left {
            let pos = elapsed - cols;
            (pos, pos <= SURFACE_W)
        } else {
            let pos = SURFACE_W - elapsed;
            (pos, pos >= -cols)
        };
        self.frame(pos);
        if self.latch == 0 && on {
            return false;
        }
        // 0x0EA09: hidden; 0x0EA0E–0x0EA41: on to the next mask, the next
        // portrait, every fourth pass the next line, the last kept.
        self.lib.hide();
        self.mask += 1;
        self.portrait += 1;
        self.passes += 1;
        if self.passes >= PASSES_A_LINE {
            self.passes = 0;
            if self.line + 1 < self.lines.len() {
                self.line += 1;
            }
        }
        if self.latch != 0 {
            // 0x0EA6D: hidden; 0x0EA75: the music faded over 2 000 ms;
            // 0x0EA88: the shadow kept.
            self.lib.hide();
            self.lib.music.push(Request::FadeOut(MUSIC_FADE));
            self.shadow = self.lib.dac.clone();
            self.stage = Stage::End { level: FULL };
        } else {
            self.stage = Stage::Pass;
        }
        true
    }

    /// `0E01h:050Bh` (`0x0E51B`): a poll into the latch; the portrait's
    /// columns at `pos` into the cleared surface over the pass's mask;
    /// where they are, the surface into the back buffer with its 0 seen
    /// through (`1995h:03EFh`), the pipes over it at both edges, the line;
    /// the part this picture and the last covered onto the screen; the
    /// back buffer's water restored where the columns were.
    fn frame(&mut self, pos: i32) {
        self.latch |= self.lib.poll();
        let p = &self.portraits[self.portrait];
        let Some(cur) = draw_columns(&mut self.columns, &p.columns, &self.masks[self.mask], pos)
        else {
            return;
        };
        self.drawn = cur;
        let shown = union(self.drawn, self.before);
        for y in cur.y..cur.y + cur.h {
            for x in cur.x..cur.x + cur.w {
                if let Some(px) = self.columns.get(x, y).filter(|&px| px != 0) {
                    self.lib.surface.set(x, y, px);
                }
            }
        }
        let (left, right) = &self.pipes;
        draw::blit_masked(&mut self.lib.surface, left, 0, 0, Some(0));
        draw::blit_masked(
            &mut self.lib.surface,
            right,
            SURFACE_W - i32::from(right.width),
            0,
            Some(0),
        );
        self.draw_line();
        copy_rect(&self.lib.surface, &mut self.lib.screen, shown);
        copy_rect(&self.water, &mut self.lib.surface, cur);
        self.columns.fill(0);
        self.before = self.drawn;
    }

    /// The pass's line, centered at `0B4h` (`0x0E952`–`0x0E962`).
    fn draw_line(&mut self) {
        if let Some(text) = self.lines.get(self.line) {
            let w = self.lib.width(text);
            draw::text::line(
                &mut self.lib.surface,
                &self.lib.common.font,
                text,
                (SURFACE_W - w) >> 1,
                TEXT_Y,
                INK,
            );
        }
    }
}

/// The lines: the far pointers at the offset `0x0E794` stores, up to the
/// null one.
fn lines(image: &Image) -> Result<Vec<Vec<u8>>> {
    far_lines(image, LINES_AT, &MOV_BP12_IMM, "the parade's lines")
}

impl Ending for Parade {
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
    use crate::titles::capzins::build::BUILD;
    use motionvm_parsec_formats::to_cp437;

    fn opened() -> Option<Parade> {
        let dir = motionvm_parsec_testutil::gamedata_capzins()?;
        let image = Image::open(&dir, BUILD.program.0, BUILD.program.1).unwrap();
        let res = motionvm_parsec_formats::detect::resource_dir(&dir);
        let mut dac = Palette::default();
        dac.raw.fill(0x3F);
        let exit = Exit {
            res: &res,
            image: &image,
            pointer: BUILD.pointer,
            location: END,
            screen: draw::display(),
            dac,
        };
        Some(Parade::open(&exit).unwrap())
    }

    /// What the program and its group hold: eleven masks, eighteen
    /// portraits, five lines.
    #[test]
    fn the_parade_is_read_from_the_program_and_its_group() {
        let Some(x) = opened() else {
            return;
        };
        assert_eq!((x.masks.len(), x.portraits.len()), (11, 18));
        let widths: Vec<usize> = x.portraits.iter().map(|p| p.columns.len()).collect();
        assert_eq!(widths[..3], [113, 87, 105]);
        assert_eq!(x.lines.len(), 5);
        assert_eq!(x.lines[0], to_cp437("BRAVO - SIE HABEN DIE WELT GERETTET"));
        assert_eq!(
            (x.pipes.0.width, x.pipes.1.width, x.pipes.0.height),
            (47, 46, 200)
        );
    }

    /// Out of the last picture in 129 retraces, the water in in 129, the
    /// module started between; a portrait rides in from the left a column
    /// each hundredth, and an event ends the parade: the music faded, the
    /// screen out in 129, the song stopped.
    #[test]
    fn the_parade_rides_until_an_event() {
        let Some(mut x) = opened() else {
            return;
        };
        for _ in 0..129 {
            assert!(x.tick());
        }
        assert!(x.lib.dac.raw.iter().all(|&v| v == 0), "faded out");
        // The setup goes on in the same retrace: no wait before it.
        assert!(matches!(x.stage, Stage::In { level: 0 }));
        for _ in 0..129 {
            assert!(x.tick());
        }
        assert_eq!(
            x.take_music(),
            [Request::Play {
                piece: SONG,
                loops: -1
            }]
        );
        assert_eq!(x.lib.dac.raw, x.shadow.raw, "faded in");
        assert_eq!(x.lib.screen.pixels, x.water.pixels);
        // The first pass, from the left: after a second's hundredths the
        // portrait's right edge is 100 columns in.
        x.tick();
        let Stage::Ride { since } = x.stage else {
            panic!("the ride");
        };
        assert!(x.from_left);
        while x.lib.now() - since < 100 {
            assert!(x.tick());
        }
        let elapsed = i32::try_from(x.lib.now() - since).unwrap();
        assert_eq!(x.drawn.x, 0);
        assert!(x.drawn.w > 0 && x.drawn.w <= elapsed);
        assert_ne!(x.lib.screen.pixels, x.water.pixels, "the portrait");
        // An event: the pass and the parade end.
        x.click(true);
        x.tick();
        assert!(matches!(x.stage, Stage::End { .. }));
        let n = (1..=200).find(|_| !x.tick()).expect("over");
        assert_eq!(n, 129 - 1);
        assert_eq!(
            x.take_music(),
            [Request::FadeOut(MUSIC_FADE), Request::Stop]
        );
        assert!(x.lib.dac.raw.iter().all(|&v| v == 0));
    }

    /// Without an event the passes go on: the next from the right, the
    /// next mask and portrait.
    #[test]
    fn the_passes_take_turns_and_go_on() {
        let Some(mut x) = opened() else {
            return;
        };
        for _ in 0..(129 + 129 + 1) {
            x.tick();
        }
        assert!(x.from_left);
        // The first pass is 320 + 113 hundredths long.
        let mut ticks = 0u64;
        while x.portrait == 0 {
            assert!(x.tick());
            ticks += 1;
            assert!(ticks < 1000);
        }
        assert!(!x.from_left || matches!(x.stage, Stage::Pass));
        x.tick();
        assert!(!x.from_left, "the next from the right");
        assert_eq!((x.mask, x.portrait, x.line), (1, 1, 0));
        let ms = ticks * crate::titles::parsec1994::title::frame_nanos() / 1_000_000;
        assert!((4_310..=4_360).contains(&ms), "{ms} ms");
    }

    /// Clicks while the pointer is hidden are not sampled: two in the
    /// fade-out leave nothing for the ride (`0x0E6A1`, the thaw at
    /// `0x0E9AB`), and the first pass rides on.
    #[test]
    fn clicks_in_the_fades_are_not_seen() {
        let Some(mut x) = opened() else {
            return;
        };
        x.click(true);
        x.click(true);
        for _ in 0..(129 + 129 + 1 + 10) {
            assert!(x.tick());
        }
        assert!(matches!(x.stage, Stage::Ride { .. }), "the ride goes on");
    }
}
