//! What PARSEC draws with: the screen it draws on, the original's scaled
//! sampling, its pointer blit, its raw plane copies and its text.
//!
//! The framebuffer underneath is the neutral one in [`motionvm_render`],
//! which knows no engine. Everything here cites an address in a PARSEC
//! program, so it is the family's own and lives with the engine — as free
//! functions over the framebuffer rather than methods on it, because the
//! type is not this crate's.

pub(crate) mod scale;
pub(crate) mod text;

use crate::num;
use motionvm_parsec_formats::Picture;
use motionvm_render::{Framebuffer, Rect, TRANSPARENT};

/// Width of the VGA mode the game asks for.
pub(crate) const DISPLAY_W: u16 = 320;
/// Height of the same.
pub(crate) const DISPLAY_H: u16 = 200;

/// The 320×200 VGA surface the game draws into, cleared to index 0.
pub(crate) fn display() -> Framebuffer {
    Framebuffer::new(DISPLAY_W, DISPLAY_H)
}

/// Mouse pointer. The pointer lives in the high DAC banks (208–255);
/// pixels below 208 are the empty cells of the cel box (0 in the cursor
/// file) and are not drawn. Drawing
/// them is a black rectangle the size of the cel.
pub(crate) fn mouse(fb: &mut Framebuffer, src: &Picture, x: i32, y: i32) {
    for sy in 0..i32::from(src.height) {
        let dy = y + sy;
        if dy < 0 || dy >= i32::from(fb.height) {
            continue;
        }
        let row = num::index(sy) * usize::from(src.width);
        for sx in 0..i32::from(src.width) {
            let p = src.pixels[row + num::index(sx)];
            if p < 208 {
                continue;
            }
            fb.set(x + sx, dy, p);
        }
    }
}

/// Scaled blit, index 0 transparent, optionally mirrored horizontally.
///
/// The box comes from [`scale::scaled_extent`] and the sampling from
/// [`scale::columns`] / [`scale::rows`] — the scaler, read. The
/// mirror is the walker's `+2` bit 40h: direction 3 draws direction
/// block 1 flipped, and the mirrored table entries in the original
/// walk the destination backwards over the same source order.
///
/// One scale for both axes: the record's `+0Eh` and `+10h` are separate,
/// and the snap and the walk write the node's scale into both
/// (`0x0B783`, `0x0B787`, `0x0B593`).
pub(crate) fn scaled(
    fb: &mut Framebuffer,
    src: &Picture,
    x: i32,
    y: i32,
    scale: u16,
    mirror: bool,
) {
    let dw = scale::scaled_extent(src.width, scale);
    let dh = scale::scaled_extent(src.height, scale);
    scaled_xy(fb, src, x, y, (dw, dh), mirror);
}

/// Scaled blit to a box of `extent`, index 0 transparent, optionally
/// mirrored horizontally — the object blitter's shape
/// (`0BB6h:000Ah`), which scales each axis by its own factor
/// (`0x0BBCF`, `0x0BC0D`) and hands the box to the same scaler
/// [`scaled`] samples with. An extent of zero draws nothing.
pub(crate) fn scaled_xy(
    fb: &mut Framebuffer,
    src: &Picture,
    x: i32,
    y: i32,
    extent: (u16, u16),
    mirror: bool,
) {
    let (dw, dh) = extent;
    if dw == 0 || dh == 0 {
        return;
    }
    let cols = scale::columns(src.width, dw);
    let rows = scale::rows(src.height, dh);
    let sw = usize::from(src.width);
    for (dy, &sy) in rows.iter().enumerate() {
        let row = usize::from(sy) * sw;
        for (dx, &sx) in cols.iter().enumerate() {
            let Some(&p) = src.pixels.get(row + usize::from(sx)) else {
                continue;
            };
            if p == TRANSPARENT {
                continue;
            }
            let dx = if mirror {
                i32::from(dw) - 1 - num::count(dx)
            } else {
                num::count(dx)
            };
            fb.set(x + dx, y + num::count(dy), p);
        }
    }
}

/// A raw plane of `sw`×`sh` pixels onto the frame, index 0 left as it is.
///
/// A layer plane carries 0 where it has nothing to say, and leaving those
/// pixels as they are lets the layer below show through.
pub(crate) fn raw_masked(fb: &mut Framebuffer, pixels: &[u8], sw: u16, sh: u16, x: i32, y: i32) {
    for sy in 0..i32::from(sh) {
        let dy = y + sy;
        if dy < 0 || dy >= i32::from(fb.height) {
            continue;
        }
        let row = num::index(sy) * usize::from(sw);
        for sx in 0..i32::from(sw) {
            match pixels.get(row + num::index(sx)) {
                Some(&p) if p != TRANSPARENT => fb.set(x + sx, dy, p),
                _ => {}
            }
        }
    }
}

/// Fills a rectangle given as PARSEC's routines take one — a corner and a
/// signed size — clipped to the frame. A width or a height of zero or less
/// fills nothing; one wider than a frame can be is the whole of it.
pub(crate) fn fill_rect(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, index: u8) {
    let side = |n: i32| u16::try_from(n.clamp(0, i32::from(u16::MAX))).unwrap_or(u16::MAX);
    fb.fill_rect(
        Rect {
            x,
            y,
            w: side(w),
            h: side(h),
        },
        index,
    );
}

/// Copies a decoded cel onto the frame at `(x, y)`, skipping the
/// `transparent` index, or none for an opaque copy — the room playfields
/// take that path, where index 0 is a color and not a hole.
///
/// The neutral blit takes the neutral picture; the family's readers hand
/// back their own, and every cel a room, a strip or a font holds stays in
/// the shape it was decoded in rather than being copied once more.
pub(crate) fn blit_masked(
    fb: &mut Framebuffer,
    src: &Picture,
    x: i32,
    y: i32,
    transparent: Option<u8>,
) {
    for sy in 0..i32::from(src.height) {
        let dy = y + sy;
        if dy < 0 || dy >= i32::from(fb.height) {
            continue;
        }
        let row = num::index(sy) * usize::from(src.width);
        for sx in 0..i32::from(src.width) {
            let p = src.pixels[row + num::index(sx)];
            if Some(p) == transparent {
                continue;
            }
            fb.set(x + sx, dy, p);
        }
    }
}
