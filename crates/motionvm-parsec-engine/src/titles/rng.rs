//! The generator, `1EB5h:0028h` in *Dunkle Schatten* (image `0x1EB78`).
//!
//! A 16×16→32 multiply by `660Dh` and `19h`, then `3619_1149h` added with
//! the carry, the result kept in the routine's own code segment, and the
//! answer taken as `low + (hi mod (high-low+1))`. The multipliers are
//! loaded at `0x1EB8E` and `0x1EB91`, the addend added at
//! `0x1EBAB`–`0x1EBAF`, the seed stored at `cs:[8]` (`0x1EBB2`), and the
//! inclusive span divided into at `0x1EBC1`–`0x1EBC6`.
//!
//! A fresh program starts with the seed at zero, because that is what the
//! image ships: the two words at `cs:6` and `cs:8` are zero.

use crate::num;

/// The seed, which the original keeps in the routine's own code segment.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LcRng {
    /// `cs:[6]`.
    pub lo: u16,
    /// `cs:[8]`.
    pub hi: u16,
}

impl LcRng {
    /// A roll in `low..=high`. `low >= high` answers `low` without
    /// advancing the seed, which is the bound the routine tests first
    /// (`0x1EB80`).
    pub(crate) fn inclusive(&mut self, low: u16, high: u16) -> u16 {
        if low >= high {
            return low;
        }
        let mul_hi = u32::from(self.hi) * 0x660D;
        let mut di = num::low_word(mul_hi);
        di = di.wrapping_add(num::low_word(u32::from(self.lo) * 0x19));
        let mul_lo = u32::from(self.lo) * 0x660D;
        let dx = num::high_word(mul_lo);
        let ax = dx.wrapping_add(di);
        let (bx, carry) = num::low_word(mul_lo).overflowing_add(0x1149);
        self.hi = ax.wrapping_add(0x3619).wrapping_add(u16::from(carry));
        self.lo = bx;
        let span = u32::from(high.wrapping_sub(low).wrapping_add(1));
        low.wrapping_add(num::low_word(u32::from(self.hi) % span))
    }
}
