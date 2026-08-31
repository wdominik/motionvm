//! Software rendering: an 8-bit indexed framebuffer, the palettes its indices
//! mean, and the blits between them.
//!
//! This crate knows no engine. Pixels, rectangles, palette lookup and PNG
//! writing hold for whatever draws on top of them; everything an engine
//! measured — its screens, its text, its transitions — lives with that
//! engine, not here.
//!
//! Everything stays paletted until the very last step. That is not nostalgia —
//! the engines this serves animate their palettes and draw through a
//! transparent index, so converting to true color early would throw away
//! exactly the information their drawing operates on.

mod pal;
pub use pal::Palette;

/// One decoded picture, ready to blit: one byte per pixel, `width * height` of
/// them, top row first, each an index into whatever palette is installed.
///
/// Sprite formats differ in more than their encoding — one carries a color
/// table beside its pixels, another relies on whatever palette its game has
/// installed — but what putting one on a surface needs is the same three
/// things in every case. So this is what the blits take, and turning a
/// reader's output into one is the caller's business. Nothing here has to
/// know which container the pixels came out of.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Picture {
    /// Pixels per row.
    pub width: u16,
    /// Rows.
    pub height: u16,
    /// One byte per pixel, `width * height` of them, top row first.
    pub pixels: Vec<u8>,
}

/// The palette index the keyed blits skip by default: see-through rather than
/// a color.
pub const TRANSPARENT: u8 = 0;

/// An indexed image. One byte per pixel, top row first.
///
/// The default is a frame of no size, which is what a save-under buffer holds
/// before the drawer has ever filled it.
#[derive(Debug, Clone, Default)]
pub struct Framebuffer {
    /// Pixels per row.
    pub width: u16,
    /// Rows.
    pub height: u16,
    /// `width * height` palette indices, row-major, no padding between rows.
    pub pixels: Vec<u8>,
}

impl Framebuffer {
    /// A frame of `width` by `height`, cleared to index 0.
    ///
    /// Index 0 is [`TRANSPARENT`], so a fresh buffer is one nothing has been
    /// drawn on rather than one painted black — the two differ the moment it is
    /// composited over something.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize],
        }
    }

    /// Sets every pixel to one index.
    pub fn fill(&mut self, index: u8) {
        self.pixels.fill(index);
    }

    /// Copies one rectangle of another frame of the same size over this one.
    ///
    /// What keeps a rebuild inside its own rectangle: the whole screen is
    /// painted afresh and only the places that asked for it are taken from the
    /// result. See `Engine::draw_screens`.
    pub fn paste_rect(&mut self, src: &Self, x: i32, y: i32, w: i32, h: i32) {
        for row in y.max(0)..(y + h).min(self.height as i32) {
            for col in x.max(0)..(x + w).min(self.width as i32) {
                if let Some(p) = src.get(col, row) {
                    self.set(col, row, p);
                }
            }
        }
    }

    /// The index at `(x, y)`, or `None` when that is off the frame.
    pub fn get(&self, x: i32, y: i32) -> Option<u8> {
        self.offset(x, y).map(|o| self.pixels[o])
    }

    /// Writes one pixel, ignoring the write when `(x, y)` is off the frame.
    ///
    /// Silent rather than checked because every caller here is a clipped loop
    /// that has already decided the pixel belongs on screen.
    pub fn set(&mut self, x: i32, y: i32, index: u8) {
        if let Some(o) = self.offset(x, y) {
            self.pixels[o] = index;
        }
    }

    fn offset(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    /// Draws a sprite with its top-left corner at `(x, y)`.
    ///
    /// Pixels of index [`TRANSPARENT`] are skipped, and anything outside the
    /// target is clipped rather than wrapped.
    pub fn blit(&mut self, sprite: &Picture, x: i32, y: i32) {
        self.blit_masked(sprite, x, y, Some(TRANSPARENT));
    }

    /// Like [`blit`](Self::blit) but with the transparent index chosen, or
    /// `None` for an opaque copy.
    pub fn blit_masked(&mut self, sprite: &Picture, x: i32, y: i32, transparent: Option<u8>) {
        for sy in 0..sprite.height as i32 {
            let dy = y + sy;
            if dy < 0 || dy >= self.height as i32 {
                continue;
            }
            let row = sy as usize * sprite.width as usize;
            for sx in 0..sprite.width as i32 {
                let p = sprite.pixels[row + sx as usize];
                if Some(p) == transparent {
                    continue;
                }
                self.set(x + sx, dy, p);
            }
        }
    }

    /// Draws a sprite scaled by per-mille factors, nearest neighbor.
    ///
    /// The factors are in thousandths of the source size, one per axis, which
    /// is how the engines above this pass their scaling. Nearest-neighbor
    /// because the scaling works on palette indices, where interpolating
    /// between two entries is meaningless.
    pub fn blit_scaled(&mut self, sprite: &Picture, x: i32, y: i32, h_mille: u32, v_mille: u32) {
        if h_mille == 0 || v_mille == 0 {
            return;
        }
        let w = (sprite.width as u64 * h_mille as u64 / 1000) as i32;
        let h = (sprite.height as u64 * v_mille as u64 / 1000) as i32;
        for dy in 0..h {
            let sy = (dy as u64 * 1000 / v_mille as u64) as usize;
            if sy >= sprite.height as usize {
                break;
            }
            let row = sy * sprite.width as usize;
            for dx in 0..w {
                let sx = (dx as u64 * 1000 / h_mille as u64) as usize;
                if sx >= sprite.width as usize {
                    break;
                }
                let p = sprite.pixels[row + sx];
                if p != TRANSPARENT {
                    self.set(x + dx, y + dy, p);
                }
            }
        }
    }

    /// Copies a rectangle of `src` into this buffer at `(dx, dy)`.
    ///
    /// Used for compositing screens, where nothing is transparent: a screen
    /// covers its whole area.
    pub fn copy_from(&mut self, src: &Framebuffer, area: Rect, dx: i32, dy: i32) {
        // A row at a time rather than a pixel at a time. Both are the same
        // copy — a pixel outside either surface was skipped, and the clipped
        // ranges below are exactly the pixels that were not skipped — but the
        // per-pixel form spent two bounds checks and an `Option` on each of
        // the 307 200 pixels of a full-screen compose, which measured at
        // 294 µs a frame, a third of the whole frame.
        //
        // The clip is intersected on three counts per axis: the requested
        // area, what is actually inside `src`, and what is inside `self`.
        let cols = {
            let lo = 0.max(-area.x).max(-dx);
            let hi = (area.w as i32)
                .min(src.width as i32 - area.x)
                .min(self.width as i32 - dx);
            lo..hi
        };
        let rows = {
            let lo = 0.max(-area.y).max(-dy);
            let hi = (area.h as i32)
                .min(src.height as i32 - area.y)
                .min(self.height as i32 - dy);
            lo..hi
        };
        if cols.is_empty() || rows.is_empty() {
            return;
        }
        let run = (cols.end - cols.start) as usize;
        for row in rows {
            let s = (area.y + row) as usize * src.width as usize + (area.x + cols.start) as usize;
            let d = (dy + row) as usize * self.width as usize + (dx + cols.start) as usize;
            self.pixels[d..d + run].copy_from_slice(&src.pixels[s..s + run]);
        }
    }

    /// Writes the frame as an indexed PNG, palette and all.
    ///
    /// Indexed on purpose: a comparison against the original has to be made on
    /// palette indices, not on colors. A screen capture goes through the
    /// display's color profile and comes back with every channel off by one,
    /// so an RGB diff reports seventy percent of the picture as different and
    /// buries a real two-pixel shift in the noise.
    pub fn write_png(
        &self,
        path: &std::path::Path,
        palette: &Palette,
    ) -> Result<(), Box<dyn std::error::Error>> {
        write_indexed_png(
            path,
            self.width as u32,
            self.height as u32,
            &self.pixels,
            palette.to_rgb8(),
            None,
        )
    }

    /// Expands to straight RGBA through `palette`.
    pub fn to_rgba(&self, palette: &Palette) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for &p in &self.pixels {
            let [r, g, b] = palette.rgb8(p);
            out.extend_from_slice(&[r, g, b, 255]);
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// An area of a surface: a corner and a size, in pixels.
pub struct Rect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: u16,
    /// Height.
    pub h: u16,
}

/// Writes an 8-bit indexed PNG.
///
/// Indexed rather than RGB throughout, because the index *is* the information:
/// the engine animates the palette and treats index 0 as transparent, so a
/// picture written as true color has thrown away what the comparison against
/// the original is about. Every image this workspace writes goes through here —
/// the frame F12 writes and the pictures the tools extract — so there is one
/// place where the encoder's settings are decided and
/// one answer to what a written PNG contains.
///
/// `transparent`, when given, is the index written into the PNG's `tRNS` chunk
/// with alpha 0. Sprites want index 0 there; a synthesized sheet wants nothing.
pub fn write_indexed_png(
    path: &std::path::Path,
    width: u32,
    height: u32,
    pixels: &[u8],
    palette: Vec<u8>,
    transparent: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, width, height);
    enc.set_color(png::ColorType::Indexed);
    enc.set_depth(png::BitDepth::Eight);
    let entries = palette.len() / 3;
    enc.set_palette(palette);
    if let Some(index) = transparent {
        let mut alpha = vec![255u8; entries];
        if let Some(a) = alpha.get_mut(index as usize) {
            *a = 0;
        }
        enc.set_trns(alpha);
    }
    enc.write_header()?.write_image_data(pixels)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite(w: u16, h: u16, fill: u8) -> Picture {
        Picture {
            width: w,
            height: h,
            pixels: vec![fill; w as usize * h as usize],
        }
    }

    #[test]
    fn blit_skips_the_transparent_index() {
        let mut fb = Framebuffer::new(4, 2);
        fb.fill(9);
        fb.blit(&sprite(2, 2, TRANSPARENT), 0, 0);
        assert!(
            fb.pixels.iter().all(|&p| p == 9),
            "transparent pixels overwrote the target"
        );

        fb.blit(&sprite(2, 2, 5), 1, 0);
        assert_eq!(fb.get(0, 0), Some(9));
        assert_eq!(fb.get(1, 0), Some(5));
        assert_eq!(fb.get(2, 1), Some(5));
        assert_eq!(fb.get(3, 1), Some(9));
    }

    #[test]
    fn blit_clips_instead_of_wrapping() {
        let mut fb = Framebuffer::new(4, 4);
        fb.blit(&sprite(4, 4, 7), -2, -2);
        assert_eq!(fb.get(0, 0), Some(7));
        assert_eq!(fb.get(2, 2), Some(0), "should not have wrapped around");

        let mut fb = Framebuffer::new(4, 4);
        fb.blit(&sprite(4, 4, 7), 3, 3);
        assert_eq!(fb.get(3, 3), Some(7));
        assert_eq!(fb.get(0, 0), Some(0));
    }

    #[test]
    fn scaling_doubles_a_sprite() {
        let mut fb = Framebuffer::new(8, 8);
        let mut s = sprite(2, 2, 5);
        s.pixels[3] = 6;
        fb.blit_scaled(&s, 0, 0, 2000, 2000);
        // Each source pixel becomes a 2x2 block.
        assert_eq!(fb.get(0, 0), Some(5));
        assert_eq!(fb.get(1, 1), Some(5));
        assert_eq!(fb.get(2, 2), Some(6));
        assert_eq!(fb.get(3, 3), Some(6));
        assert_eq!(fb.get(4, 4), Some(0), "nothing beyond the scaled size");
    }
}
