//! The DAC as PARSEC writes it: banks out of its files, laid over the
//! palette the screen is shown through.
//!
//! The reader says which registers a palette blob names and what it puts
//! in them ([`motionvm_parsec_formats::pal::bank`]); the palette that holds the
//! registers is the neutral one. This is the write between the two — a
//! port `3C8h`/`3C9h` run per blob, leaving every other register as it was.

use motionvm_parsec_formats::{RoomFile, pal};
use motionvm_render::Palette;

/// One palette blob written over a palette.
pub(crate) trait Apply {
    /// Writes the bank `bytes` names, and nothing else — nothing at all
    /// when `bytes` is not a bank.
    fn apply(&mut self, bytes: &[u8]);
}

impl Apply for Palette {
    fn apply(&mut self, bytes: &[u8]) {
        if let Some(bank) = pal::bank(bytes) {
            self.load(bank.start, bank.rgb);
        }
    }
}

/// A room's own bank, as a fresh DAC.
pub(crate) fn of_room(room: &RoomFile) -> Palette {
    let mut p = Palette::default();
    p.load(room.room.palette_start, &room.room.palette_rgb);
    p
}
