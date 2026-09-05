//! Whole-number scaling, and the blit that carries it out.
//!
//! Pure integer geometry: how many window pixels one game pixel gets on each
//! axis, which pair the window opens on, and the loop that expands the
//! engine's indexed frame into the buffer softbuffer hands over. No `winit`
//! and no `softbuffer` reach in here, which is why it is its own file — the
//! window loop and the arithmetic it depends on are two subjects, and this is
//! the one the README makes its most specific promises about: "exact at 5×6
//! and its multiples, and the window opens on the largest exact step the
//! screen has room for".
//!
//! The pixel aspect a pair is measured against comes from the game, through
//! `Playable::pixel_aspect`: square where the game's grid already matches its
//! monitor, taller than wide where it did not.

use motionvm_playable::{Framebuffer, PixelAspect};

/// A window dimension as an index. Lossless on every target this workspace
/// builds for, all of which have a `usize` at least 32 bits wide.
#[expect(
    clippy::as_conversions,
    reason = "a widening on every target this builds for; `usize` has no `From<u32>`"
)]
pub(crate) fn wide(n: u32) -> usize {
    n as usize
}

/// How much bigger than the game the window is asked to be, to begin with —
/// per axis, as everywhere here.
///
/// Only whole numbers: the art is hand-drawn pixels and a fractional factor
/// smears them. Nothing else needs telling — [`blit`] works the factors out
/// from the window it is given, and the pointer mapping divides by the same
/// ones, so both follow this on their own. Which is also why this is only
/// the opening size: dragging the window edge moves the factors, and
/// Alt+Enter takes the whole screen at the largest pair that fits.
///
/// The pair is the smallest at or above twice the game whose height does not
/// round the pixel aspect down — the window may open a touch narrow, never
/// squashed. Square pixels get (2, 2): 1280x960 logical points fit under the
/// title bar of a 1080p screen and three times — 1920x1440 — does not.
/// The 16-bit games' 6:5 pixels get (3, 4) — 960×800 — because
/// (2, 2) would show the squash this pair exists to correct.
pub(crate) fn base_pair(aspect: PixelAspect) -> (u32, u32) {
    let mut sx = 2;
    loop {
        let sy = tall(sx, aspect);
        if sy * aspect.width >= sx * aspect.height {
            return (sx, sy);
        }
        sx += 1;
    }
}

/// How many window rows a game pixel `sx` columns wide gets under the pixel
/// aspect: `sx · aspect` to the nearest whole number, never zero.
///
/// Exact where the aspect divides — for 6:5 pixels at sx of 5, 10, 15 — and
/// at most half a row off between, which at those sizes is under five percent
/// of the picture's shape. For square pixels it is `sx` itself.
fn tall(sx: u32, aspect: PixelAspect) -> u32 {
    ((sx * aspect.height + aspect.width / 2) / aspect.width).max(1)
}

/// How many window pixels one game pixel gets, axis by axis: whole numbers,
/// never zero, the pair as close to the pixel aspect as the window allows.
///
/// The one definition, because the picture and the pointer have to agree. Two
/// copies of this arithmetic is two places to drift apart in, and a pointer
/// that disagrees with the picture by one scale step lands every click in the
/// wrong place.
pub(crate) fn scale_pair(
    game: (u32, u32),
    aspect: PixelAspect,
    width: u32,
    height: u32,
) -> (u32, u32) {
    for sx in (1..=(width / game.0).max(1)).rev() {
        let sy = tall(sx, aspect);
        if game.0 * sx <= width && game.1 * sy <= height {
            return (sx, sy);
        }
    }
    // A window too small for the picture even at one: clipped, as before.
    (1, 1)
}

/// The pair the window opens on: like [`scale_pair`], but preferring the
/// largest pair that meets the pixel aspect *exactly*, where one fits.
///
/// Only `resumed` asks — the opening picture should be the true shape when
/// the screen has room for it, and dragging afterwards walks every whole
/// step. For square pixels every pair is exact and this *is* `scale_pair`.
pub(crate) fn opening_pair(
    game: (u32, u32),
    aspect: PixelAspect,
    width: u32,
    height: u32,
) -> (u32, u32) {
    for sx in (1..=(width / game.0).max(1)).rev() {
        if !(sx * aspect.height).is_multiple_of(aspect.width) {
            continue;
        }
        let sy = sx * aspect.height / aspect.width;
        if game.0 * sx <= width && game.1 * sy <= height {
            return (sx, sy);
        }
    }
    scale_pair(game, aspect, width, height)
}

/// Draws the frame into the window buffer, scaled by whole numbers — one per
/// axis, meeting the game's pixel aspect — and centered.
///
/// This walks every physical window pixel — on a Retina display five million
/// of them, sixteen times the game's own 307 200 — so it is the one loop in
/// the frontend where the shape of the code is the cost. So no arithmetic runs
/// per destination pixel: every source row is expanded through the palette
/// once and then repeated with `copy_within`, which is a straight memmove, and
/// there is no whole-buffer clear. Dividing each destination coordinate back to
/// its source instead is the obvious shape and costs a division per pixel.
///
/// Every pixel of `out` is still written every call — the picture over its
/// rectangle, the margins by the strip fills. That is a promise, not a
/// leftover: softbuffer only hands out a freshly zeroed buffer on some
/// platforms; on others it persists with whatever it held, and a pixel left
/// unwritten shows it.
pub(crate) fn blit(
    frame: &Framebuffer,
    colors: &[u32; 256],
    out: &mut [u32],
    width: u32,
    height: u32,
    aspect: PixelAspect,
) {
    let (sx, sy) = scale_pair(
        (u32::from(frame.width), u32::from(frame.height)),
        aspect,
        width,
        height,
    );
    let (dw, dh) = (u32::from(frame.width) * sx, u32::from(frame.height) * sy);
    // Left-over space is split evenly; an odd remainder leaves the extra pixel
    // on the right and bottom, which is invisible and keeps the arithmetic in
    // integers.
    let (ox, oy) = (
        (width.saturating_sub(dw)) / 2,
        (height.saturating_sub(dh)) / 2,
    );
    // How much of the picture the window has room for: all of it, unless the
    // window is smaller than the game — then the pair is already pinned at
    // one and the picture is cut off at the right and bottom.
    let rows = wide(dh.min(height.saturating_sub(oy)));
    let cols = wide(dw.min(width.saturating_sub(ox)));
    let (width, ox, oy) = (wide(width), wide(ox), wide(oy));
    let (sx, sy) = (wide(sx), wide(sy));

    // The margins. Top and bottom are contiguous runs; the side strips only
    // exist when the width is not an exact multiple, and the loop is skipped
    // entirely when they are empty.
    out[..oy * width].fill(0);
    out[(oy + rows) * width..].fill(0);
    if ox > 0 || ox + cols < width {
        for y in oy..oy + rows {
            out[y * width..y * width + ox].fill(0);
            out[y * width + ox + cols..(y + 1) * width].fill(0);
        }
    }

    for row in 0..rows.div_ceil(sy) {
        let y0 = row * sy;
        let base = (oy + y0) * width + ox;
        let src = &frame.pixels[row * usize::from(frame.width)..];
        // The row, expanded once: each source pixel becomes `sx` copies of
        // its color.
        let dst = &mut out[base..base + cols];
        for (chunk, &index) in dst.chunks_exact_mut(sx).zip(src) {
            chunk.fill(colors[usize::from(index)]);
        }
        // A row cut off mid-pixel. `scale_pair` cannot actually produce one —
        // sx above 1 means the window fits the whole width, and at 1 every
        // chunk is a pixel — but the promise above is that every pixel of
        // `out` gets written, and that must not hang on that arithmetic.
        let rem = cols % sx;
        if rem > 0 {
            dst[cols - rem..].fill(colors[usize::from(src[cols / sx])]);
        }
        // And repeated: the other window rows this source row covers are
        // copies of the one just written.
        for r in 1..sy.min(rows - y0) {
            out.copy_within(base..base + cols, base + r * width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The blit as it was first written: one destination pixel at a time, a
    /// divide per coordinate, a full clear up front. Sixteen times slower than
    /// [`blit`] and obviously right, which is exactly what an oracle is for.
    fn blit_reference(
        frame: &Framebuffer,
        colors: &[u32; 256],
        out: &mut [u32],
        width: u32,
        height: u32,
        aspect: PixelAspect,
    ) {
        out.fill(0);
        let (sx, sy) = scale_pair(
            (u32::from(frame.width), u32::from(frame.height)),
            aspect,
            width,
            height,
        );
        let (dw, dh) = (u32::from(frame.width) * sx, u32::from(frame.height) * sy);
        let (ox, oy) = (
            (width.saturating_sub(dw)) / 2,
            (height.saturating_sub(dh)) / 2,
        );
        for y in 0..dh.min(height.saturating_sub(oy)) {
            let src_row = wide(y / sy) * usize::from(frame.width);
            let dst_row = wide(oy + y) * wide(width) + wide(ox);
            for x in 0..dw.min(width.saturating_sub(ox)) {
                let index = frame.pixels[src_row + wide(x / sx)];
                out[dst_row + wide(x)] = colors[usize::from(index)];
            }
        }
    }

    /// A frame with structure in it: every pixel its own mix of position,
    /// so a swapped row or a column off by one cannot cancel out.
    fn patterned(width: u16, height: u16) -> (Framebuffer, [u32; 256]) {
        let mut frame = Framebuffer::new(width, height);
        for (i, p) in frame.pixels.iter_mut().enumerate() {
            *p = u8::try_from(i * 7 % 251).unwrap();
        }
        let mut colors = [0u32; 256];
        for (i, c) in colors.iter_mut().enumerate() {
            *c = u32::try_from(i).unwrap() * 0x0101 + 3;
        }
        (frame, colors)
    }

    fn agree(frame: &Framebuffer, colors: &[u32; 256], cases: &[(u32, u32)], aspect: PixelAspect) {
        for &(w, h) in cases {
            // Prefilled with a color neither blit writes, so a pixel either
            // of them missed cannot pass as agreement.
            let mut fast = vec![0xdead_beefu32; wide(w * h)];
            let mut slow = vec![0xdead_beefu32; wide(w * h)];
            blit(frame, colors, &mut fast, w, h, aspect);
            blit_reference(frame, colors, &mut slow, w, h, aspect);
            assert_eq!(fast, slow, "{w}x{h}");
        }
    }

    #[test]
    fn the_fast_blit_agrees_with_the_slow_one() {
        const WIDTH: u32 = 640;
        const HEIGHT: u32 = 480;
        let (frame, colors) = patterned(
            u16::try_from(WIDTH).unwrap(),
            u16::try_from(HEIGHT).unwrap(),
        );
        agree(
            &frame,
            &colors,
            &[
                (WIDTH * 3, HEIGHT * 3),         // an exact multiple, no margins
                (WIDTH * 3 + 9, HEIGHT * 3 + 5), // odd margins on every side
                (2560, 1920),                    // a Retina window, scale 4
                (WIDTH, HEIGHT),                 // scale 1, exact
                (700, 500),                      // scale 1 with margins
                (500, 400),                      // smaller than the picture: clipped
                (700, 300),                      // clipped in one direction only
                (639, 481),                      // one pixel short, one over
                (1, 1),                          // degenerate
            ],
            SQUARE,
        );
    }

    #[test]
    fn the_fast_blit_agrees_on_tall_pixels_too() {
        let (frame, colors) = patterned(320, 200);
        agree(
            &frame,
            &colors,
            &[
                (960, 800),   // (3, 4) with no margin
                (1600, 1200), // (5, 6), the aspect met exactly
                (1920, 1600), // (6, 7), margin below
                (2007, 1413), // odd margins on every side
                (320, 200),   // (1, 1): shown square, all there is room for
                (300, 180),   // smaller than the picture: clipped
                (1, 1),       // degenerate
            ],
            TALL,
        );
    }

    /// The pairs the table in the docs promises, and that the old single
    /// scalar comes back out for square pixels.
    /// Square pixels, and the 16-bit games' 5:6 ones.
    const SQUARE: PixelAspect = PixelAspect {
        width: 1,
        height: 1,
    };
    const TALL: PixelAspect = PixelAspect {
        width: 5,
        height: 6,
    };

    #[test]
    fn the_scale_pairs_are_the_documented_ones() {
        let game = (320, 200);
        let par = TALL;
        for (w, h, want) in [
            (960, 800, (3, 4)),
            (1280, 1000, (4, 5)),
            (1600, 1200, (5, 6)),
            (1920, 1400, (6, 7)),
            (1920, 1600, (6, 7)),
            (3840, 2160, (8, 10)),
            (640, 400, (2, 2)),
            (100, 80, (1, 1)),
        ] {
            assert_eq!(scale_pair(game, par, w, h), want, "{w}x{h}");
        }
        // Square pixels: the pair is the old `min(w/gw, h/gh).max(1)` twice.
        for (w, h, want) in [(2560, 1920, 4), (700, 500, 1), (1, 1, 1)] {
            assert_eq!(
                scale_pair((640, 480), SQUARE, w, h),
                (want, want),
                "{w}x{h}"
            );
        }
        // The opening snap prefers the exact pair where one fits.
        assert_eq!(opening_pair(game, par, 1920, 1600), (5, 6));
        assert_eq!(opening_pair(game, par, 3840, 2880), (10, 12));
        assert_eq!(opening_pair(game, par, 960, 800), (3, 4)); // none fits
        assert_eq!(opening_pair((640, 480), SQUARE, 2560, 1920), (4, 4));
        // And the window opens unsquashed: (2,2) for square pixels, (3,4)
        // for the 16-bit games' tall ones.
        assert_eq!(base_pair(SQUARE), (2, 2));
        assert_eq!(base_pair(TALL), (3, 4));
    }

    /// [`scale_pair`] against a brute force, over a grid of window sizes.
    ///
    /// The function walks down from the largest horizontal scale that fits and
    /// takes the first pair whose *both* axes fit; the oracle tries every pair
    /// and keeps the largest that fits, which is obviously right and far too
    /// slow to use. They have to agree, and where they can disagree is the
    /// interesting part: `tall` is not monotone in an obvious way for a 5:6
    /// pixel, so a larger `sx` can want a `sy` that no longer fits while a
    /// smaller one does — the walk relies on taking the first that works, and
    /// this is what says that is the largest.
    ///
    /// Both games' sizes and both pixel shapes, over sizes from smaller than
    /// one picture to larger than eight of them.
    #[test]
    fn the_scale_walk_agrees_with_brute_force() {
        fn largest_that_fits(
            game: (u32, u32),
            aspect: PixelAspect,
            width: u32,
            height: u32,
        ) -> (u32, u32) {
            let mut best = (1, 1);
            for sx in 1..=64 {
                let sy = tall(sx, aspect);
                if game.0 * sx <= width && game.1 * sy <= height && sx >= best.0 {
                    best = (sx, sy);
                }
            }
            best
        }

        let square = PixelAspect::default();
        let tall_pixel = PixelAspect {
            width: 5,
            height: 6,
        };
        for game in [(640u32, 480u32), (320, 200)] {
            for aspect in [square, tall_pixel] {
                for width in (100..=2600).step_by(37) {
                    for height in (100..=1800).step_by(41) {
                        assert_eq!(
                            scale_pair(game, aspect, width, height),
                            largest_that_fits(game, aspect, width, height),
                            "game {game:?} aspect {aspect:?} in {width}x{height}"
                        );
                    }
                }
            }
        }
    }

    /// A scale pair never overflows the window it was chosen for, and never
    /// answers zero.
    ///
    /// The two things everything downstream assumes: `blit` walks
    /// `game * scale` pixels and would run off the surface, and a zero scale
    /// would divide by nothing when a click is mapped back.
    #[test]
    fn a_scale_pair_fits_or_is_one() {
        let tall_pixel = PixelAspect {
            width: 5,
            height: 6,
        };
        for game in [(640u32, 480u32), (320, 200)] {
            for aspect in [PixelAspect::default(), tall_pixel] {
                for width in (1..=2000).step_by(29) {
                    for height in (1..=1500).step_by(31) {
                        let (sx, sy) = scale_pair(game, aspect, width, height);
                        assert!(sx >= 1 && sy >= 1, "{sx}x{sy} in {width}x{height}");
                        let fits = game.0 * sx <= width && game.1 * sy <= height;
                        assert!(
                            fits || (sx, sy) == (1, 1),
                            "{sx}x{sy} overflows {width}x{height} without being the \
                             clipped fallback"
                        );
                    }
                }
            }
        }
    }
}
