//! Raw 8-bit sprites (the GFX segment).
//!
//! ```text
//! +0   u16  width
//! +2   u16  height
//! +4   u16  zero in every occupied slot; not interpreted
//! +6   u8[width * height]  palette indices, row-major, top row first
//! ```
//!
//! No magic, no compression, no palette: a pixel is an index into whatever
//! palette `SETPAL` last installed. Measured over all 1586 sprites of
//! Die Enviro-Kids greifen ein,
//! the third field is 0 and the item is exactly `6 + width * height` bytes
//! long — which is the one check a reader can make, and does.

use crate::error::{Error, Result};
use crate::past_end;

/// Bytes before the pixels.
pub const HEADER_LEN: usize = 6;

#[derive(Debug, Clone)]
/// One decoded picture: indices into a palette the sprite does not carry.
pub struct Sprite {
    /// Pixels per row.
    pub width: u16,
    /// Rows.
    pub height: u16,
    /// One byte per pixel, `width * height` of them, top row first.
    pub pixels: Vec<u8>,
}

impl Sprite {
    /// Reads one GFX item.
    pub fn parse(item: &[u8]) -> Result<Self> {
        let (head, pixels) = item
            .split_first_chunk::<HEADER_LEN>()
            .ok_or_else(|| past_end(item, 0, HEADER_LEN))?;
        let width = u16::from_le_bytes([head[0], head[1]]);
        let height = u16::from_le_bytes([head[2], head[3]]);
        if usize::from(width).checked_mul(usize::from(height)) != Some(pixels.len()) {
            return Err(Error::Corrupt {
                what: "sprite",
                detail: format!(
                    "a {width}x{height} sprite, and the item holds {} bytes after its header",
                    pixels.len()
                ),
            });
        }
        Ok(Self {
            width,
            height,
            pixels: pixels.to_vec(),
        })
    }
}
