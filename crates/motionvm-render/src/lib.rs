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
//!
//! It depends on nothing. The PNG writer is a hundred lines in `png.rs`
//! rather than an encoder crate, because the file this writes has one shape
//! and every field of it is decided here — that module says what the choice
//! costs, which is a screenshot ten times the size and no compression at all.
//! The reader beside it is wider, by as much as a capture of the original
//! needs, and brings its own inflate in `inflate.rs` for the same reason.

mod inflate;
mod pal;
mod png;
pub use inflate::BadStream;
pub use pal::Palette;
pub use png::{Decoded, Pixels, Unreadable, crc32};

/// A coordinate the caller has clipped to the surface, as an index into it.
///
/// Every blit tests `x < 0 || y < 0` before it indexes, so nothing negative
/// reaches here. A cast rather than `try_from`, because the fallback
/// `try_from` would want is a value that cannot occur, and the blits are
/// checked pixel for pixel against the original with this arithmetic.
#[expect(
    clippy::as_conversions,
    reason = "a coordinate already clipped to the surface, so never negative"
)]
fn at(coord: i32) -> usize {
    coord as usize
}

/// A picture dimension as an index. Lossless on every target this workspace
/// builds for, all of which have a `usize` at least 32 bits wide.
#[expect(
    clippy::as_conversions,
    reason = "a widening on every target this builds for; `usize` has no `From<u32>`"
)]
pub(crate) fn wide(n: u32) -> usize {
    n as usize
}

/// A count of pixels in the arithmetic the scaled blit does in 64 bits.
#[expect(
    clippy::as_conversions,
    reason = "a widening on every target this builds for; `u64` has no `From<usize>`"
)]
fn long(n: usize) -> u64 {
    n as u64
}

/// A count of pixels as a coordinate. A picture wider than `i32::MAX` cannot
/// be addressed by one, and saturating is what keeps the arithmetic finite if
/// a caller ever asks for it.
fn coord(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// What can go wrong writing a PNG, or reading one back.
///
/// Three cases, and they want different answers from a caller: a file that
/// could not be created, written or opened is the caller's business — a
/// directory that is not there, a disk that is full — while an encoder that
/// refused is this crate's, and means the pixels and the palette handed in
/// did not agree with each other; and a file that could not be read is the
/// file's, which the refusal names.
///
/// A named type rather than a boxed one, because this crate is the neutral
/// layer and a box there would make every caller's error type a box too. The
/// one boxed error in the workspace is the one the contract asks for.
#[derive(Debug)]
pub enum PngError {
    /// The file could not be created, written or read.
    Io(std::io::Error),
    /// The picture and the palette handed in did not agree with each other.
    Refused(Refusal),
    /// A file to read was not a PNG this crate reads; see [`read_png`].
    Unreadable(Unreadable),
}

/// What can be wrong with a picture before it is written.
///
/// Four things, and they are the four the format cannot express rather than
/// a general validation: an index has eight bits, a palette entry has three
/// bytes, `IHDR` has no way to say "no pixels", and a chunk's length has
/// thirty-two bits. Everything else about an indexed PNG is decided by this
/// crate and cannot come out wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// `pixels` is not `width × height` bytes.
    WrongSize {
        /// How many the size asks for.
        want: usize,
        /// How many were handed in.
        got: usize,
    },
    /// A palette that is not whole RGB triples, or reaches past the 256
    /// entries an eight-bit index can name.
    BadPalette {
        /// How many bytes it holds.
        bytes: usize,
    },
    /// A width or a height of zero, which `IHDR` cannot carry.
    Empty {
        /// The width asked for.
        width: u32,
        /// The height asked for.
        height: u32,
    },
    /// More pixels than one `IDAT` chunk can carry: a chunk's length is
    /// thirty-two bits, and the picture goes into one chunk with a filter
    /// byte a row and five bytes of framing per 65 535.
    TooLarge {
        /// How many bytes of pixels were handed in.
        bytes: usize,
    },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongSize { want, got } => {
                write!(f, "the size asks for {want} pixels and {got} were given")
            }
            Self::BadPalette { bytes } => write!(
                f,
                "a palette of {bytes} bytes is not whole RGB triples, or holds \
                 more than the 256 entries an index can reach"
            ),
            Self::Empty { width, height } => write!(f, "a picture {width} by {height}"),
            Self::TooLarge { bytes } => {
                write!(
                    f,
                    "a picture of {bytes} bytes is more than one IDAT can carry"
                )
            }
        }
    }
}

impl std::fmt::Display for PngError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Refused(why) => write!(f, "{why}"),
            Self::Unreadable(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for PngError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Refused(_) | Self::Unreadable(_) => None,
        }
    }
}

impl From<std::io::Error> for PngError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// One decoded picture, ready to blit: one byte per pixel, `width * height` of
/// them, top row first, each an index into whatever palette is installed.
///
/// Sprite formats differ in more than their encoding — one carries a color
/// table beside its pixels, another relies on whatever palette its game has
/// installed — but what putting one on a surface needs is the same three
/// things in every case. So this is what the blits take, and turning a
/// reader's output into one is the caller's business. Nothing here has to
/// know which container the pixels came out of.
///
/// **The same three fields as [`Framebuffer`], and deliberately not the same
/// type.** A picture is a *source* and a framebuffer is a *target*; nothing in
/// the workspace ever converts one into the other, and no behavior is shared,
/// because a picture has no methods at all. What the split buys is that a
/// surface cannot be blitted as if it were a sprite and a sprite cannot be
/// returned where a frame is promised — the same argument [`PixelAspect`] is
/// two named fields rather than a pair. Folding them together would remove ten
/// lines of declaration and one compiler check.
///
/// [`PixelAspect`]: https://docs.rs/motionvm-playable
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

/// One frame ready to show: the picture, and the palette its indices mean.
///
/// The two travel together because they are only meaningful together — an
/// indexed frame under the wrong palette is not a worse picture, it is a
/// different one — and because handing both over as one borrow is what lets
/// the picture be borrowed at all: returned on its own, a frame would have to
/// be owned, since the palette would be a second borrow behind it. Owned, a
/// frame is 307 200 bytes, up to a hundred times a second — the largest
/// allocation anywhere on the path.
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    /// The picture: one palette index per pixel.
    pub pixels: &'a Framebuffer,
    /// What those indices mean.
    pub palette: &'a Palette,
}

/// A surface being drawn on: one byte per pixel, top row first.
///
/// The default is a frame of no size, which is what a save-under buffer holds
/// before the drawer has ever filled it.
///
/// The *target* half of the pair [`Picture`] explains: same three fields,
/// different role, and everything that draws is here rather than there.
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
            pixels: vec![0; usize::from(width) * usize::from(height)],
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
        for row in y.max(0)..(y + h).min(i32::from(self.height)) {
            for col in x.max(0)..(x + w).min(i32::from(self.width)) {
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
        if x < 0 || y < 0 || x >= i32::from(self.width) || y >= i32::from(self.height) {
            return None;
        }
        Some(at(y) * usize::from(self.width) + at(x))
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
        for sy in 0..sprite.height {
            let dy = y + i32::from(sy);
            if dy < 0 || dy >= i32::from(self.height) {
                continue;
            }
            let row = usize::from(sy) * usize::from(sprite.width);
            for sx in 0..sprite.width {
                let p = sprite.pixels[row + usize::from(sx)];
                if Some(p) == transparent {
                    continue;
                }
                self.set(x + i32::from(sx), dy, p);
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
        // The per-mille arithmetic runs in 64 bits, where it cannot overflow.
        // A scaled size past `usize` is not a size; saturating one lands the
        // loops on their `>=` checks at the first step rather than anywhere
        // else.
        let scaled = |pixels: u16, mille: u32| {
            usize::try_from(u64::from(pixels) * u64::from(mille) / 1000).unwrap_or(usize::MAX)
        };
        let source = |dest: usize, mille: u32| {
            usize::try_from(long(dest) * 1000 / u64::from(mille)).unwrap_or(usize::MAX)
        };
        let (w, h) = (
            scaled(sprite.width, h_mille),
            scaled(sprite.height, v_mille),
        );
        for dy in 0..h {
            let sy = source(dy, v_mille);
            if sy >= usize::from(sprite.height) {
                break;
            }
            let row = sy * usize::from(sprite.width);
            for dx in 0..w {
                let sx = source(dx, h_mille);
                if sx >= usize::from(sprite.width) {
                    break;
                }
                let p = sprite.pixels[row + sx];
                if p != TRANSPARENT {
                    self.set(x + coord(dx), y + coord(dy), p);
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
            let hi = i32::from(area.w)
                .min(i32::from(src.width) - area.x)
                .min(i32::from(self.width) - dx);
            lo..hi
        };
        let rows = {
            let lo = 0.max(-area.y).max(-dy);
            let hi = i32::from(area.h)
                .min(i32::from(src.height) - area.y)
                .min(i32::from(self.height) - dy);
            lo..hi
        };
        if cols.is_empty() || rows.is_empty() {
            return;
        }
        let run = at(cols.end - cols.start);
        for row in rows {
            let s = at(area.y + row) * usize::from(src.width) + at(area.x + cols.start);
            let d = at(dy + row) * usize::from(self.width) + at(dx + cols.start);
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
    pub fn write_png(&self, path: &std::path::Path, palette: &Palette) -> Result<(), PngError> {
        write_indexed_png(
            path,
            u32::from(self.width),
            u32::from(self.height),
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
) -> Result<(), PngError> {
    if width == 0 || height == 0 {
        return Err(PngError::Refused(Refusal::Empty { width, height }));
    }
    let want = wide(width) * wide(height);
    if pixels.len() != want {
        return Err(PngError::Refused(Refusal::WrongSize {
            want,
            got: pixels.len(),
        }));
    }
    if !palette.len().is_multiple_of(3) || palette.len() > 256 * 3 {
        return Err(PngError::Refused(Refusal::BadPalette {
            bytes: palette.len(),
        }));
    }
    // The picture plus a filter byte a row, held under half a chunk's
    // thirty-two-bit length, which leaves the framing more room than it
    // takes; the writer counts on this having been asked.
    if pixels.len() + wide(height) > wide(u32::MAX / 2) {
        return Err(PngError::Refused(Refusal::TooLarge {
            bytes: pixels.len(),
        }));
    }
    // The whole file is built before anything is opened, so a picture that
    // cannot be written leaves no file at all rather than a truncated one.
    let bytes = png::encode(width, height, pixels, &palette, transparent);
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Reads a PNG back: the indexed kind this crate writes, or the true-color
/// kind a capture of the original arrives as.
///
/// This is the other half of a comparison. F12 writes the frame as indices
/// and a palette; the original's side is DOSBox-X's own capture, indexed when
/// the emulator was in a 256-color mode and RGB when it was a screenshot of a
/// window. Both come back through here, and the tool that compares them
/// decides what to make of the colors — see `motionvm-motion-tools
/// compare-frame`.
///
/// Eight bits a sample, not interlaced, any filter, any compression: the
/// readers under [`Unreadable`] say what is refused and why.
pub fn read_png(path: &std::path::Path) -> Result<Decoded, PngError> {
    let file = std::fs::read(path)?;
    png::decode(&file).map_err(PngError::Unreadable)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite(w: u16, h: u16, fill: u8) -> Picture {
        Picture {
            width: w,
            height: h,
            pixels: vec![fill; usize::from(w) * usize::from(h)],
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
