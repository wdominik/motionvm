//! The intro both title programs play: a logo faded in and out, then the
//! *freelance* logo drawn through eight wave masks (Dunkle Schatten's
//! `0323h:04D0h`, `0x3700`, with the wave `03B5h:0003h`; Captain Zins's
//! `0323h:049Bh`, `0x36CB`, with `043Dh:0006h` — the same code but for
//! its addresses and the wave's slot).
//!
//! With `/FXOFF` on the command line the original shows a plain logo
//! instead of the wave; no host passes it, so the wave is what plays.
//! Any event the checks see skips the rest of the intro.
//!
//! *Bi-Fi 2*'s intro (`0x3723`–`0x3767`) is two stills through the same
//! logo routine (`0311h:02C1h`, `0x33D1`, Dunkle Schatten's `0x3631`), slots
//! 0 and 2, no wave, and its end masks the skip without hiding the pointer
//! ([`Intro::logos`]).

use motionvm_parsec_formats::{Picture, Rib};

use super::{Fade, Faded, Lib, Pal, SURFACE_H, SURFACE_W, Slots, Step};
use crate::draw::{self, scale};
use crate::error::Result;
use crate::num;

/// Dunkle Schatten's `0x3716`: `push 4`, the fade's step; `0x3718`: `push
/// 96h`, the logo's hold in hundredths.
const LOGO_STEP: i32 = 4;
const LOGO_HOLD: u64 = 0x96;
/// `0x371B`: `push 4`, the first logo's picture slot; its palette is the
/// next (`0x3666`: `inc si`). Captain Zins's `0x36F1` pushes the same.
const LOGO_SLOT: usize = 4;
/// Dunkle Schatten's `0x3CB1`: `push 70h`, the wave's first palette level.
const WAVE_LEVEL: i32 = 0x70;
/// `0x3DEA`/`0x3DED`: four levels a mask from 70h (`shl di, 2`, `add di,
/// 70h`), and `0x3EDC`: `add di, 4` on to full.
const WAVE_LEVEL_STEP: i32 = 4;
/// `0x3F8C`: `sub di, 2` — the wave fades out at half the pace.
const WAVE_OUT_STEP: i32 = 2;
/// `0x3F0F`: `cmp word [13C8h], 64h` — the full logo held a second.
const WAVE_HOLD: u64 = 0x64;

/// One column's span in a wave mask, as Dunkle Schatten's `0x3FFE` records
/// it: the first non-zero row from the top and how many non-zero rows
/// follow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Span {
    pub y: i32,
    pub count: i32,
}

/// A rectangle as the wave keeps it (Dunkle Schatten's `[1408h]`,
/// `[1410h]`): x, y, w, h.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Where the intro is.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// The intro's own start: the pointer thawed, the keys flushed, the
    /// skip cleared and unmasked (Dunkle Schatten's `0x3700`).
    Begin,
    /// `0x3631`: logo `i`, unless the intro is being skipped.
    Logo(usize),
    LogoIn(usize, Fade),
    LogoHold {
        i: usize,
        since: u64,
    },
    LogoOut(usize, Fade),
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
    End,
}

/// The intro while it plays.
pub(crate) struct Intro {
    /// The logos, each a picture and its palette, in the order they play.
    logos: Vec<(Picture, Pal)>,
    /// Whether the wave plays after them; without one its parts are empty.
    has_wave: bool,
    /// Whether the end hides the pointer (Dunkle Schatten's `0x377F`).
    end_hides: bool,
    /// The wave slot's resource 0: entry 0 the palette, the rest one
    /// column each (Dunkle Schatten's `0x3BA3`–`0x3BF3`).
    wave_pal: Pal,
    wave_columns: Vec<Picture>,
    /// Resource 1, the masks: for each of the 320 columns of the surface
    /// its span (`0x3C27`, `0x3C41`).
    wave_masks: Vec<Vec<Span>>,
    stage: Stage,
    /// Dunkle Schatten's `[1406h]`: the wave's own skip.
    wave_skip: u16,
    /// `[1408h]`: the rectangle the wave drew last.
    drawn: Rect,
}

impl Intro {
    /// The intro's pictures out of `INTROS.DAT`: the logo's slot and the
    /// one after it, and the wave's slot (Dunkle Schatten's `0x3B79`: `push
    /// 0Ch`; Captain Zins's `0x43FC`: `push 0Dh`).
    pub(crate) fn read(lib: &mut Lib, intros: &Slots, wave_slot: usize) -> Result<Self> {
        let logo = intros.picture(LOGO_SLOT)?;
        let logo_pal = lib.add_palette(intros.palette(LOGO_SLOT + 1)?);
        let logos = vec![(logo, logo_pal)];
        let wave = intros.resources(wave_slot)?;
        let entries = |blob: &[u8]| -> Result<Vec<Vec<u8>>> {
            let dir = motionvm_parsec_formats::rib::directory(blob, "a wave resource")?
                .ok_or("a wave resource has no directory")?;
            Ok(dir.into_iter().map(|r| blob[r].to_vec()).collect())
        };
        let logo_set = entries(wave.first().ok_or("the wave has no resource 0")?)?;
        let (wave_palette, columns) = logo_set
            .split_first()
            .ok_or("the wave's resource 0 is empty")?;
        let wave_columns = columns
            .iter()
            .map(|c| column(c))
            .collect::<Result<Vec<_>>>()?;
        let wave_masks = entries(wave.get(1).ok_or("the wave has no resource 1")?)?
            .iter()
            .map(|m| Ok(spans(&Picture::parse(&Rib::parse(m)?.decoded)?)))
            .collect::<Result<Vec<_>>>()?;
        let wave_pal = lib.add_palette(wave_palette.clone());
        Ok(Intro {
            logos,
            has_wave: true,
            end_hides: true,
            wave_pal,
            wave_columns,
            wave_masks,
            stage: Stage::Begin,
            wave_skip: 0,
            drawn: Rect {
                x: 0,
                y: 0,
                w: SURFACE_W,
                h: SURFACE_H,
            },
        })
    }

    /// Logos alone, each picture slot with its palette the slot after it,
    /// and no wave: *Bi-Fi 2*'s `0x3739`–`0x375B` push slots 0 and 2.
    /// The end masks the skip and leaves the pointer as it is (`0x3761`).
    pub(crate) fn logos(lib: &mut Lib, intros: &Slots, slots: &[usize]) -> Result<Self> {
        let mut logos = Vec::with_capacity(slots.len());
        for &s in slots {
            let picture = intros.picture(s)?;
            let pal = lib.add_palette(intros.palette(s + 1)?);
            logos.push((picture, pal));
        }
        let first = logos.first().ok_or("an intro of no logos")?.1;
        Ok(Intro {
            logos,
            has_wave: false,
            end_hides: false,
            wave_pal: first,
            wave_columns: Vec::new(),
            wave_masks: Vec::new(),
            stage: Stage::Begin,
            wave_skip: 0,
            drawn: Rect {
                x: 0,
                y: 0,
                w: SURFACE_W,
                h: SURFACE_H,
            },
        })
    }

    /// The first logo's palette, for a test to compare the DAC with.
    #[cfg(test)]
    pub(crate) fn logo_pal(&self) -> Pal {
        self.logos[0].1
    }

    /// The wave's columns, masks and palette, for a test.
    #[cfg(test)]
    pub(crate) fn wave_parts(&self) -> (&[Picture], &[Vec<Span>], Pal) {
        (&self.wave_columns, &self.wave_masks, self.wave_pal)
    }

    /// As far as the intro goes before this retrace is used.
    pub(crate) fn step(&mut self, lib: &mut Lib) -> Step<()> {
        match self.stage {
            Stage::Begin => {
                lib.flush_keys();
                lib.skip = 0;
                lib.skip_mask = 0xFFFF;
                self.stage = Stage::Logo(0);
                Step::Next
            }
            Stage::Logo(i) => {
                // 0x3639: skipped altogether once the intro is.
                let Some(&(_, pal)) = self.logos.get(i) else {
                    self.stage = Stage::WaveCheck;
                    return Step::Next;
                };
                if lib.check() {
                    self.stage = Stage::Logo(i + 1);
                    return Step::Next;
                }
                // 0x35C4: the picture, then 0x35F1: the fade in.
                draw::blit_masked(&mut lib.screen, &self.logos[i].0, 0, 0, None);
                self.stage = Stage::LogoIn(i, Fade::full(pal, LOGO_STEP));
                Step::Next
            }
            Stage::LogoIn(i, f) => match lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::LogoIn(i, f);
                    Step::Next
                }
                Faded::Done => {
                    self.stage = Stage::LogoHold {
                        i,
                        since: lib.now(),
                    };
                    Step::Next
                }
            },
            Stage::LogoHold { i, since } => {
                // 0x3345: until the clock reaches the hold, or a skip.
                if lib.now() - since >= LOGO_HOLD || lib.check() {
                    let pal = self.logos[i].1;
                    self.stage = Stage::LogoOut(i, Fade::full(pal, -LOGO_STEP));
                    return Step::Next;
                }
                Step::Yield
            }
            Stage::LogoOut(i, f) => match lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::LogoOut(i, f);
                    Step::Next
                }
                Faded::Done => {
                    self.stage = Stage::Logo(i + 1);
                    Step::Next
                }
            },
            Stage::WaveCheck => {
                // 0x372C: [94h] is 1 in the image, cleared only by
                // `/FXOFF` (0x5FD, 0x609); 0x374B: skipped once the intro
                // is. A title without a wave goes on to the end.
                if !self.has_wave || lib.check() {
                    self.stage = Stage::End;
                    return Step::Next;
                }
                self.wave_begin(lib);
                Step::Next
            }
            Stage::End => {
                // 0x3773–0x377F: the skip cleared and masked, the pointer
                // hidden.
                lib.skip = 0;
                lib.skip_mask = 0;
                if self.end_hides {
                    lib.shown = false;
                }
                Step::Done(())
            }
            Stage::Slide { .. }
            | Stage::Masks { .. }
            | Stage::Ramp { .. }
            | Stage::WaveHold { .. }
            | Stage::WaveOut { .. } => self.wave_stage(lib),
        }
    }

    /// The wave's stages, from the slide in to the fade out
    /// (`0x3D68`–`0x3E4D`).
    fn wave_stage(&mut self, lib: &mut Lib) -> Step<()> {
        match self.stage {
            Stage::Slide { since } => {
                // 0x3D68: from the right edge two columns a hundredth, to
                // where the logo is centered. Past an `i32` of hundredths
                // the slide is long over, and the edge is where it ends.
                let elapsed = i32::try_from(lib.now() - since).unwrap_or(SURFACE_W);
                let x = (SURFACE_W - 2 * elapsed).max(self.wave_x());
                self.wave_pass(lib, x, 0);
                if self.wave_skip != 0 || x <= self.wave_x() {
                    self.stage = Stage::Masks {
                        since: lib.now(),
                        next: 0,
                        drawn: None,
                    };
                    return Step::Next;
                }
                Step::Yield
            }
            Stage::Masks {
                since,
                next,
                drawn: Some(f),
            } => {
                // 0x3DE8–0x3E4D: the mask's level at the retrace.
                if !lib.wait_retrace() {
                    return Step::Yield;
                }
                lib.set_level(self.wave_pal, num::count(f) * WAVE_LEVEL_STEP + WAVE_LEVEL);
                self.stage = Stage::Masks {
                    since,
                    next,
                    drawn: None,
                };
                Step::Next
            }
            Stage::Masks {
                since,
                next,
                drawn: None,
            } => {
                let n = self.wave_masks.len();
                if self.wave_skip != 0 || next >= n {
                    self.stage = Stage::Ramp {
                        level: num::count(n) * WAVE_LEVEL_STEP + WAVE_LEVEL,
                    };
                    return Step::Next;
                }
                // 0x3DB0: the mask the hundredths have reached, four each;
                // 0x3E50: the loop goes on from the one after it.
                let f = usize::try_from((lib.now() - since) >> 2).map_or(n - 1, |f| f.min(n - 1));
                self.wave_pass(lib, self.wave_x(), f);
                self.stage = Stage::Masks {
                    since,
                    next: f + 1,
                    drawn: Some(f),
                };
                Step::Next
            }
            Stage::Ramp { level } => {
                if self.wave_skip != 0 || level > super::FULL {
                    self.stage = Stage::WaveHold { since: lib.now() };
                    return Step::Next;
                }
                if !lib.wait_retrace() {
                    return Step::Yield;
                }
                lib.set_level(self.wave_pal, level);
                self.wave_skip |= lib.poll();
                self.stage = Stage::Ramp {
                    level: level + WAVE_LEVEL_STEP,
                };
                Step::Next
            }
            Stage::WaveHold { since } => {
                if self.wave_skip != 0 || lib.now() - since >= WAVE_HOLD {
                    self.stage = Stage::WaveOut { level: super::FULL };
                    return Step::Next;
                }
                self.wave_skip |= lib.poll();
                Step::Yield
            }
            Stage::WaveOut { level } => {
                if self.wave_skip != 0 || level < 0 {
                    // 0x3F9A: hidden, blanked, the screen on.
                    lib.shown = false;
                    lib.blank();
                    self.stage = Stage::End;
                    return Step::Next;
                }
                if !lib.wait_retrace() {
                    return Step::Yield;
                }
                lib.set_level(self.wave_pal, level);
                self.wave_skip |= lib.poll();
                self.stage = Stage::WaveOut {
                    level: level - WAVE_OUT_STEP,
                };
                Step::Next
            }
            _ => unreachable!("`step` hands only the wave's stages here"),
        }
    }

    /// Where the wave's logo stands at rest: `(140h − columns) >> 1`
    /// (`0x3D48`–`0x3D50`, `sar`).
    fn wave_x(&self) -> i32 {
        (SURFACE_W - num::count(self.wave_columns.len())) >> 1
    }

    /// Dunkle Schatten's `03B5h:0003h` up to the slide
    /// (`0x3B53`–`0x3D62`).
    fn wave_begin(&mut self, lib: &mut Lib) {
        // 0x3B67: hidden; 0x3B6C: blanked. 0x3CA9: the surface cleared.
        lib.shown = false;
        lib.blank();
        lib.surface.fill(0);
        // 0x3CD5: the palette at 70h, without waiting for a retrace.
        lib.set_level(self.wave_pal, WAVE_LEVEL);
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
        lib.flush_keys();
        lib.poll();
        self.stage = Stage::Slide { since: lib.now() };
    }

    /// Dunkle Schatten's `03B5h:06CDh` (`0x421D`): one pass — a poll into
    /// the wave's skip, mask `f` drawn with the logo at `x` into the
    /// surface, and the part of the surface this draw and the last one
    /// covered copied to the screen; the surface cleared again.
    fn wave_pass(&mut self, lib: &mut Lib, x: i32, f: usize) {
        self.wave_skip |= lib.poll();
        let Some(cur) = self.wave_draw(lib, x, f) else {
            return;
        };
        let union = union(cur, self.drawn);
        // 0A35h:00E8h: the rectangle from the surface to the screen.
        copy_rect(&lib.surface, &mut lib.screen, union);
        // 0x42AE: the whole surface cleared; 0x42B6: this rectangle kept.
        lib.surface.fill(0);
        self.drawn = cur;
    }

    /// Dunkle Schatten's `03B5h:0584h` (`0x40D4`): the logo's columns from
    /// `x` on, each stretched over the span the mask has at its own column
    /// of the surface; the rectangle they cover, or `None` when none is on
    /// it.
    fn wave_draw(&self, lib: &mut Lib, x: i32, f: usize) -> Option<Rect> {
        draw_columns(&mut lib.surface, &self.wave_columns, &self.wave_masks[f], x)
    }
}

/// Dunkle Schatten's `0x42D1`: the union of two rectangles, signed.
pub(crate) fn union(a: Rect, b: Rect) -> Rect {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.w).max(b.x + b.w);
    let y1 = (a.y + a.h).max(b.y + b.h);
    Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    }
}

/// A rectangle of one framebuffer copied into another (`0A35h:00E8h`).
pub(crate) fn copy_rect(
    from: &motionvm_render::Framebuffer,
    to: &mut motionvm_render::Framebuffer,
    r: Rect,
) {
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            if let Some(p) = from.get(x, y) {
                to.set(x, y, p);
            }
        }
    }
}

/// A column as the column routine hands it to the scaler: the resource
/// four bytes in, past its width and height only (Dunkle Schatten's
/// `XP00` `0x41BF`, Captain Zins's `XP01` `0x0E1D5`: `add ax, 4`). The
/// wave's columns are pictures, whose pixels begin there. Captain Zins's
/// portraits are cels, whose header goes on with a flags word and a
/// reserved one — four zeros, seen through — so each of those columns is
/// drawn with them above its pixels and without its last four.
pub(crate) fn column(c: &[u8]) -> Result<Picture> {
    let p = Picture::parse(c)?;
    let n = usize::from(p.width) * usize::from(p.height);
    let pixels = c
        .get(4..4 + n)
        .ok_or("a column is shorter than its size")?
        .to_vec();
    Ok(Picture { pixels, ..p })
}

/// Dunkle Schatten's `03B5h:0584h` (`0x40D4`): picture columns from `x` on,
/// each stretched over the span the mask has at its own column of the
/// surface; the rectangle they cover, or `None` when none is on it.
pub(crate) fn draw_columns(
    surface: &mut motionvm_render::Framebuffer,
    columns: &[Picture],
    mask: &[Span],
    x: i32,
) -> Option<Rect> {
    let cols = num::count(columns.len());
    // 0x40F0: a picture left of the surface skips its first columns;
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
        let col = &columns[num::index(first_col + k)];
        // A mask is a span a screen column; one narrower than the screen
        // ends the columns where it ends.
        let Some(&span) = mask.get(num::index(first_span + k)) else {
            break;
        };
        if span.count <= 0 {
            continue;
        }
        // 0x41C6 calls 089Eh:0430h: the column, `1 × h`, to `1 × count`
        // at the span's top in the span's own column. That entry hands its
        // arguments to the scaler core at 089Eh:0038h (0x8E3E), which is
        // `XP01`'s byte for byte (see `scale`), so its rows are
        // `scale::rows`.
        let rows = scale::rows(col.height, num::small(span.count));
        let sx = first_span + k;
        for (dy, &sy) in rows.iter().enumerate() {
            let p = col.pixels[usize::from(sy) * usize::from(col.width)];
            surface.set(sx, span.y + num::count(dy), p);
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

/// Dunkle Schatten's `03B5h:04AEh` (`0x3FFE`): for every column of a mask,
/// the first row that is not 0 and how many rows that are not 0 follow it.
pub(crate) fn spans(mask: &Picture) -> Vec<Span> {
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
