//! Software rendering: an 8-bit indexed framebuffer, sprite and text blitting,
//! and the compositing of the game's screens.
//!
//! Everything stays paletted until the very last step. That is not nostalgia —
//! the engine animates the palette (`FADEIN`, `FADEOUT`, `SETPAL`) and treats
//! index 0 as transparent, so converting to true color early would throw away
//! exactly the information those words operate on.

/// The gap the engine leaves after every glyph, across and down. See
/// [`Framebuffer::draw_text`] for how it was measured.
pub const SPACING: i32 = 1;

use motionvm_formats::font::{Font, FontRefTable};
use motionvm_formats::{Palette, Sprite};

/// Width of the display the game asks for with `640x480x256 SETRES`.
pub const DISPLAY_W: u16 = 640;
/// Height of the same. Screens are composited onto a surface of this size.
pub const DISPLAY_H: u16 = 480;

/// Palette index the engine treats as see-through.
pub const TRANSPARENT: u8 = 0;

/// An indexed image. One byte per pixel, top row first.
#[derive(Debug, Clone)]
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
    pub fn blit(&mut self, sprite: &Sprite, x: i32, y: i32) {
        self.blit_masked(sprite, x, y, Some(TRANSPARENT));
    }

    /// Like [`blit`](Self::blit) but with the transparent index chosen, or
    /// `None` for an opaque copy.
    pub fn blit_masked(&mut self, sprite: &Sprite, x: i32, y: i32, transparent: Option<u8>) {
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

    /// Draws a sprite scaled by a per-mille factor, nearest neighbor.
    ///
    /// The engine's `SD%SHR` and its horizontal and vertical variants carry a
    /// factor in thousandths: the title logo is a 320x200 sprite drawn with
    /// 2000, which is exactly the 640x400 the main screen is. Scaling is
    /// nearest-neighbor because the original works on palette indices, where
    /// interpolating between two entries is meaningless.
    pub fn blit_scaled(&mut self, sprite: &Sprite, x: i32, y: i32, h_mille: u32, v_mille: u32) {
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

    /// Draws one line of text, returning how far the pen advanced.
    ///
    /// Glyphs are 1-bit masks, so only set bits are written; the gaps stay as
    /// they were.
    ///
    /// One pixel of spacing follows each glyph. That is measured, not assumed:
    /// the intro's lines come to 409, 437 and 354 pixels of ink in the original,
    /// where the bare glyph widths of the font it uses add up to 361, 382 and
    /// 307 over 49, 56 and 48 characters. Where the pixel comes from the format
    /// does not say — neither the font header nor the reference table carries a
    /// spacing value — so it lives in the engine's drawing code. What is
    /// established is the result, not the mechanism.
    pub fn draw_text(
        &mut self,
        font: &Font,
        refs: &FontRefTable,
        text: &str,
        x: i32,
        y: i32,
        color: u8,
    ) -> i32 {
        self.draw_text_spaced(font, refs, text, x, y, color, SPACING)
    }

    /// The same, with the gap between glyphs given rather than assumed.
    ///
    /// The gap is a global in the original (0xd66ee, with the line gap at
    /// 0xd66ec), and the text drawer sets it per pass: the outline pass takes
    /// the value from the template, the normal pass puts back 1. See
    /// `Engine::draw_text_descriptor`.
    // Font, reference table, string, position, color and spacing: six things
    // the caller genuinely decides per call, with no subset that travels
    // together often enough to earn a struct.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_spaced(
        &mut self,
        font: &Font,
        refs: &FontRefTable,
        text: &str,
        x: i32,
        y: i32,
        color: u8,
        spacing: i32,
    ) -> i32 {
        let mut pen = x;
        for ch in text.chars() {
            let Some(byte) = cp437_byte(ch) else { continue };
            let Some(index) = refs.glyph_for(byte) else {
                continue;
            };
            let Some(glyph) = font.glyphs.get(index as usize) else {
                continue;
            };
            for gy in 0..glyph.height {
                for gx in 0..glyph.width {
                    if glyph.pixel(gx, gy) {
                        self.set(pen + gx as i32, y + gy as i32, color);
                    }
                }
            }
            pen += glyph.width as i32 + spacing;
        }
        // The last glyph's spacing is never drawn, so it is not counted either.
        (pen - x - spacing).max(0)
    }

    /// How wide `text` would be, without drawing it.
    ///
    /// Needed because the game centers text on a point (`SDCEN`, `SDVCEN`)
    /// rather than giving a corner, so the left edge cannot be known until the
    /// width is.
    pub fn text_width(font: &Font, refs: &FontRefTable, text: &str) -> i32 {
        Self::text_width_spaced(font, refs, text, SPACING)
    }

    /// How wide `text` would be at a given glyph gap.
    pub fn text_width_spaced(font: &Font, refs: &FontRefTable, text: &str, spacing: i32) -> i32 {
        let total: i32 = text
            .chars()
            .filter_map(cp437_byte)
            .filter_map(|b| refs.glyph_for(b))
            .filter_map(|i| font.glyphs.get(i as usize))
            .map(|g| g.width as i32 + spacing)
            .sum();
        (total - spacing).max(0)
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

/// Maps a `char` back to the CP437 byte the font tables are indexed by.
fn cp437_byte(ch: char) -> Option<u8> {
    if (ch as u32) < 0x80 {
        return Some(ch as u8);
    }
    (0x80u8..=0xff).find(|&b| motionvm_formats::cp437_char(b) == ch)
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

/// One of the game's screens: a drawing surface with a size, a window onto it,
/// and a place on the display.
///
/// The five words that configure a screen are recorded verbatim. How they map
/// onto compositing is a *hypothesis*, spelled out at [`Display::compose`] —
/// the values the game passes are known exactly, their interpretation is what
/// the reference screenshots have to settle.
#[derive(Debug, Clone)]
pub struct Screen {
    /// What `NEWSCREEN` handed back and every other `SCR*` word names it by.
    pub handle: u32,
    /// `SCRSIZE`: the drawing surface.
    pub size: (u16, u16),
    /// `SCRFVSIZE`: the full view size.
    pub full_view: (u16, u16),
    /// `SCRVSIZE`: the visible window onto the surface.
    pub view: (u16, u16),
    /// `SCRVPOS`.
    pub view_pos: (i16, i16),
    /// `SCRPOS`.
    pub pos: (i16, i16),
    /// `SCRX` writes the first of these and `GSCRX` reads it back; `GSCRY`
    /// reads the second. They live at +0x24 and +0x26 of a record hanging off
    /// the screen, not of the screen itself, which is why they are a pair of
    /// their own and not `view_pos`. What they shift is not established — the
    /// game only ever sets them to zero — so nothing composites with them yet.
    pub origin: (i16, i16),
    /// `FREEZESCR` sets bit 2 of the screen's flag byte at +0x13 and
    /// `UNFREEZESCR` clears it again.
    pub frozen: bool,
    /// Whether the screen is composited at all. `SCRACTIVE` and `SCRINACTIVE`
    /// switch it; an inactive screen keeps its surface and its configuration.
    pub active: bool,
    /// The drawing surface itself, `size` big.
    pub buffer: Framebuffer,
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

/// How much the text backing takes off each color channel.
///
/// Read from the table builder at 0x14740: it walks the palette, subtracts 20
/// from red, green and blue, clamps each at zero and asks 0x1ee26 for the
/// nearest entry to the result. The answers become a 256-byte row, and the
/// backing sends every pixel under the text through it.
pub const BACKING_DARKEN: i16 = 0x14;

impl Framebuffer {
    /// Darkens a rectangle the way the original's text backing does.
    ///
    /// Not a fill: at 0x186d5 the routine reads each pixel, looks it up in the
    /// remap row and writes the answer back, so what shows through is the
    /// picture underneath, darkened — which is how a 256-color game makes a
    /// half-transparent panel. A solid rectangle in some color would be a
    /// different thing entirely and look it.
    ///
    /// Takes the remap row rather than the palette it comes from, because
    /// building one is [`backing_map`]'s 196 000 operations and the answer
    /// depends on nothing else. The original builds its row once per palette
    /// too — the table builder at 0x14740 runs when the palette is set, not
    /// when a panel is drawn.
    pub fn darken_rect(&mut self, map: &[u8; 256], x: i32, y: i32, w: i32, h: i32) {
        for row in y.max(0)..(y + h).min(self.height as i32) {
            for col in x.max(0)..(x + w).min(self.width as i32) {
                let i = row as usize * self.width as usize + col as usize;
                self.pixels[i] = map[self.pixels[i] as usize];
            }
        }
    }
}

/// The 256-byte remap row the backing uses: every palette entry darkened by
/// [`BACKING_DARKEN`] and brought back to the nearest entry there is.
pub fn backing_map(palette: &Palette) -> [u8; 256] {
    let mut map = [0u8; 256];
    for (i, slot) in map.iter_mut().enumerate() {
        // The subtraction is on the palette as stored — six-bit values, 0 to 63
        // — so twenty is about a third of full scale, not a twelfth. Reading it
        // as an eight-bit step would make the backing nearly invisible.
        let want: [i16; 3] =
            std::array::from_fn(|k| (palette.raw[i * 3 + k] as i16 - BACKING_DARKEN).max(0));
        let mut best = (i32::MAX, 0u8);
        for j in 0..Palette::COLORS {
            let d: i32 = (0..3)
                .map(|k| {
                    let e = want[k] as i32 - palette.raw[j * 3 + k] as i32;
                    e * e
                })
                .sum();
            if d < best.0 {
                best = (d, j as u8);
            }
        }
        *slot = best.1;
    }
    map
}

impl Screen {
    /// An empty screen under `handle`, with everything at zero.
    ///
    /// The game configures a screen with the `SCR*` words immediately after
    /// making one, so nothing here is a default anyone relies on.
    pub fn new(handle: u32) -> Self {
        Self {
            handle,
            size: (0, 0),
            full_view: (0, 0),
            view: (0, 0),
            view_pos: (0, 0),
            pos: (0, 0),
            origin: (0, 0),
            frozen: false,
            active: true,
            buffer: Framebuffer::new(0, 0),
        }
    }

    /// Resizes the drawing surface, keeping nothing.
    pub fn set_size(&mut self, w: u16, h: u16) {
        self.size = (w, h);
        self.buffer = Framebuffer::new(w, h);
    }
}

/// The set of screens plus the palette in force.
pub struct Display {
    /// Every screen the game has made, in the order it made them — which is
    /// also the order they are composited in, before `level` is considered.
    pub screens: Vec<Screen>,
    /// The palette in force. `SETPAL` replaces it wholesale, so a frame's
    /// meaning depends on this as much as on its indices.
    pub palette: Palette,
    /// The screen `SCR*` words configure and new descriptors attach to.
    /// `NEWSCREEN` makes its screen current; `ACTSCR` selects an existing one.
    pub current: Option<u32>,
    next_handle: u32,
}

impl Default for Display {
    fn default() -> Self {
        Self::new()
    }
}

impl Display {
    /// Whether a screen is frozen, which is what stops the per-frame walk from
    /// reaching the descriptors on it.
    pub fn frozen(&self, handle: u32) -> bool {
        self.screens.iter().any(|s| s.handle == handle && s.frozen)
    }

    /// A display with no screens and an all-black palette.
    pub fn new() -> Self {
        Self {
            screens: Vec::new(),
            palette: Palette::from_6bit(&[0; Palette::BYTES]),
            current: None,
            next_handle: 1,
        }
    }

    /// `NEWSCREEN`: creates a screen and hands back its handle.
    ///
    /// Handles start at 1, which is what the original returns for the first
    /// screen of a run.
    pub fn new_screen(&mut self) -> u32 {
        let handle = self.next_handle;
        self.next_handle += 1;
        self.screens.push(Screen::new(handle));
        self.current = Some(handle);
        handle
    }

    /// The screen under `handle`, to be changed.
    pub fn screen_mut(&mut self, handle: u32) -> Option<&mut Screen> {
        self.screens.iter_mut().find(|s| s.handle == handle)
    }

    /// The screen the `SCR*` words configure.
    pub fn current_mut(&mut self) -> Option<&mut Screen> {
        let h = self.current?;
        self.screen_mut(h)
    }

    /// `ACTSCR`: makes an existing screen current.
    pub fn set_current(&mut self, handle: u32) {
        if self.screens.iter().any(|s| s.handle == handle) {
            self.current = Some(handle);
        }
    }

    /// Flattens the screens into one 640x480 image.
    ///
    /// **Hypothesis, to be checked against the reference screenshots.** The
    /// game lays out three screens: the main picture 640x400 with `SCRVPOS`
    /// (0,0), a status bar 640x80 with `SCRVPOS` (0,400), and a dialogue
    /// surface 384x480 with `SCRVPOS` (640,0). The first two tile the display
    /// exactly and the third falls outside it, which is what suggests `SCRVPOS`
    /// is the screen's place on the display and `SCRPOS` the scroll offset
    /// inside it. Every value here comes from the game; only this reading of
    /// them is provisional.
    ///
    /// **A running curtain needs no exception here**, although the flags
    /// suggest one: `FADEOUT` clears the active flag and `FADEIN` only sets it
    /// again at the end, so the screen a fade is revealing is invisible to this
    /// filter for the whole fade. It never has to be visible, because a curtain
    /// writes its bands straight into what is on screen
    /// (`Engine::advance_curtain`) — which is what the original does too — and
    /// nothing composes while one runs.
    pub fn compose(&self) -> Framebuffer {
        let mut out = Framebuffer::new(DISPLAY_W, DISPLAY_H);
        for s in self.screens.iter().filter(|s| s.active) {
            out.copy_from(
                &s.buffer,
                self.window(s),
                s.view_pos.0 as i32,
                s.view_pos.1 as i32,
            );
        }
        out
    }

    /// The part of a screen's surface that shows, in surface coordinates.
    ///
    /// `SCRPOS` scrolls the window over the surface and `SCRVSIZE` sizes it;
    /// a curtain has to map its bands the same way, so the rule lives here
    /// rather than twice.
    pub fn window(&self, s: &Screen) -> Rect {
        Rect {
            x: s.pos.0 as i32,
            y: s.pos.1 as i32,
            w: s.view.0.min(s.size.0),
            h: s.view.1.min(s.size.1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite(w: u16, h: u16, fill: u8) -> Sprite {
        Sprite {
            width: w,
            height: h,
            palette: Palette::from_6bit(&[0; Palette::BYTES]),
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

    /// The layout the game actually asks for: a main picture and a status bar
    /// that together tile the display, plus a third screen parked outside it.
    #[test]
    fn screens_tile_the_display() {
        let mut d = Display::new();

        let main = d.new_screen();
        let s = d.screen_mut(main).unwrap();
        s.set_size(640, 400);
        s.view = (640, 400);
        s.view_pos = (0, 0);
        s.buffer.fill(1);

        let status = d.new_screen();
        let s = d.screen_mut(status).unwrap();
        s.set_size(640, 80);
        s.view = (640, 80);
        s.view_pos = (0, 400);
        s.buffer.fill(2);

        let dialogue = d.new_screen();
        let s = d.screen_mut(dialogue).unwrap();
        s.set_size(384, 480);
        s.view = (384, 480);
        s.view_pos = (640, 0);
        s.buffer.fill(3);

        let out = d.compose();
        assert_eq!(out.get(0, 0), Some(1), "main picture at the top");
        assert_eq!(out.get(639, 399), Some(1));
        assert_eq!(out.get(0, 400), Some(2), "status bar below it");
        assert_eq!(out.get(639, 479), Some(2));
        assert!(
            !out.pixels.contains(&3),
            "the third screen sits outside the display and must be clipped away"
        );
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

    #[test]
    fn handles_start_at_one() {
        let mut d = Display::new();
        assert_eq!(d.new_screen(), 1);
        assert_eq!(d.new_screen(), 2);
    }
}
