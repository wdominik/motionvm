//! The chip itself — the one piece of this project that was not read out of the
//! game.
//!
//! Everything else here is rebuilt from something: the sequencer from
//! `ENGINE.EXE`, the [FM driver](crate::m32::fm) from `fmmidi3.com`, the formats
//! from the files. The OPL3 is hardware; there is no code in the game that says
//! what a Yamaha YMF262 does with a register, only code that writes to it. So
//! this layer comes from outside, and it is deliberately the thinnest possible
//! wrapper, because it is the seam where a different core would be attached.
//!
//! No trait: while there is exactly one implementation a trait would be
//! ceremony. The day a second one exists it is this same single file either
//! way.
//!
//! ## Why this core
//!
//! [`nuked_opl3`] is a pure-Rust port of Nuked-OPL3, which was reconstructed
//! from photographs of the decapped chip and is the accuracy reference every
//! emulator measures itself against. The port carries the C original as an
//! optional feature and tests itself against it for bit-identical output, so
//! choosing pure Rust costs no accuracy — and it means no C compiler in the
//! build, no `unsafe` boundary, and no trouble cross-compiling.
//!
//! **It is `LGPL-2.1-or-later`**, as every Nuked derivative is. Linked
//! statically into a Rust binary that means anyone handed the binary has to be
//! able to relink it — the object files or the sources have to be on offer. It
//! is the one part of this program that is not ours to license freely.
//!
//! ## Two details that matter
//!
//! **The register address is already right.** [`Write::address`] returns
//! `reg | (bank << 8)`, and that is exactly the 16-bit register number the chip
//! takes: on an OPL3 the second bank *is* the ninth address bit. Nothing is
//! translated here.
//!
//! **Writes go through the buffered path.** The real chip needs time between
//! two register writes, which is what the original driver's string of dummy
//! reads is for (`0x1E60`). `write_register_buffered` models that delay instead
//! of applying everything instantly, which is what the hardware does and what
//! the sound of a dense passage depends on.

/// One write to the chip.
///
/// `bank` is 0 for the base port and 1 for base + 2; on an OPL3 that is the
/// same as the register's ninth bit, so `reg as u16 | (bank << 8)` is the
/// address a DRO recording stores.
///
/// Both generations' drivers answer in these: the register set is the chip's,
/// not the driver's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Write {
    /// 0 for the base port, 1 for base + 2 — the register's ninth address bit.
    pub bank: u8,
    /// The register within the bank.
    pub reg: u8,
    /// The byte written to it.
    pub value: u8,
}

impl Write {
    /// The 9-bit register address, the form a DRO capture uses.
    pub fn address(self) -> u16 {
        self.reg as u16 | ((self.bank as u16) << 8)
    }
}

use nuked_opl3::Opl3Chip;

/// An OPL3 running at some output sample rate.
pub struct Chip {
    chip: Opl3Chip,
    rate: u32,
}

impl Chip {
    /// The rate the chip really runs at. Everything else is resampled from it,
    /// by the core itself — we do not resample.
    pub const NATIVE_RATE: u32 = 49716;

    /// A chip that will be asked for samples at `rate`.
    ///
    /// The core resamples internally from [`NATIVE_RATE`](Self::NATIVE_RATE);
    /// nothing here does.
    pub fn new(rate: u32) -> Self {
        Self {
            chip: Opl3Chip::new(rate),
            rate,
        }
    }

    /// The sample rate it was made with.
    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Sends one register write to the chip.
    ///
    /// Buffered rather than immediate on purpose: the buffered path models the
    /// settling time between writes that a real chip has, and the driver leans
    /// on it — see the note at the top of this module.
    pub fn write(&mut self, w: Write) {
        self.chip.write_register_buffered(w.address(), w.value);
    }

    /// Fills `out` with interleaved stereo frames. `out.len()` must be even.
    pub fn render(&mut self, out: &mut [i16]) {
        if out.len() < 2 {
            out.fill(0);
            return;
        }
        // The only error the core reports is a buffer under two samples, which
        // the line above has already dealt with.
        let _ = self.chip.generate_stream(out);
    }
}
