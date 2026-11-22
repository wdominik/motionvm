//! What a number's bits mean at each point the engine changes their type:
//! the conversions it applies, each named for the rule it is.
//!
//! The original computes in sixteen-bit registers that carry no sign of
//! their own — `jl` reads a word signed, `jb` the same word unsigned — and
//! the engine holds those words as `u16` or `i16`, and its own arithmetic
//! on them in `i32`. Every place a value crosses between those types is one
//! of the functions below, so a conversion at a call site says which rule
//! it follows, and the `#[expect(clippy::as_conversions)]` on each function
//! is the one place in the engine where a bit pattern changes type. The
//! count of those attributes is the count of rules, which is the evidence a
//! bare cast cannot give: a cast that reproduces a truncation the original
//! performs looks exactly like one that merely narrows a bounded count.
//!
//! Widenings are not here. `i32::from(u16)` and its kin say what they are
//! already, and a widening is never a claim about the original.

/// A word read signed: the same sixteen bits, the way `jl` and `imul` read
/// them.
pub(crate) fn signed(word: u16) -> i16 {
    i16::from_le_bytes(word.to_le_bytes())
}

/// A signed quantity back into the word that holds it — what a `mov` of a
/// register into memory does, which knows nothing of a sign.
pub(crate) fn unsigned(v: i16) -> u16 {
    u16::from_le_bytes(v.to_le_bytes())
}

/// A byte read signed: the operand of `push imm8`, which the processor
/// sign-extends.
pub(crate) fn signed8(byte: u8) -> i8 {
    i8::from_le_bytes(byte.to_le_bytes())
}

/// Thirty-two bits read signed: a code point the keyboard hands over, or a
/// remainder small enough that the sign bit is clear.
pub(crate) fn signed32(bits: u32) -> i32 {
    i32::from_le_bytes(bits.to_le_bytes())
}

/// Thirty-two signed bits read unsigned: a sign-extended word (`cwd`) added
/// into a 32-bit sum, as the savegame's checksum does.
pub(crate) fn unsigned32(v: i32) -> u32 {
    u32::from_le_bytes(v.to_le_bytes())
}

/// The low byte of a word: what a byte register keeps of the register it
/// is half of. `mov dl, cl` at `0x4B97` takes the forest maze's distance
/// `d` out of `CX` this way before `0x4B9D` stores `d + 1` as a byte, and
/// every id, verb and color the original keeps in a byte is taken the same
/// way.
#[expect(
    clippy::as_conversions,
    reason = "the truncation a byte register performs: `mov dl, cl` at `0x4B97` keeps the low eight bits"
)]
pub(crate) fn low8(word: u16) -> u8 {
    word as u8
}

/// The low word of a value the engine computes in `i32` where the original
/// computes in one sixteen-bit register: `imul dx` at `0x0AF10` makes a
/// 32-bit product and the `cwd` at `0x0AF12` keeps only its low word, and
/// every store of a register into a word of the data segment does the same.
#[expect(
    clippy::as_conversions,
    reason = "the truncation a 16-bit register performs: `cwd` at `0x0AF12` keeps the low word of the product"
)]
pub(crate) fn low16(v: i32) -> u16 {
    v as u16
}

/// A quantity the engine computes in `i32` and the original keeps in a
/// word, whose range the word holds by construction — a scale, an extent,
/// a span, a key code. Not a truncation: no caller's value is outside
/// `0..=FFFFh`.
#[expect(
    clippy::as_conversions,
    reason = "a scale, an extent or a key code, bounded well inside a word"
)]
pub(crate) fn small(v: i32) -> u16 {
    v as u16
}

/// The same low word, read signed — what the original's register holds
/// when a signed jump reads it.
pub(crate) fn short(v: i32) -> i16 {
    signed(low16(v))
}

/// The low word of a 32-bit quantity the original keeps as a `DX:AX` pair:
/// `mul si` at `0x1EB96` leaves the low word of the generator's product in
/// `AX`, and `div bx` at `0x1EBC6` its remainder in `DX`.
#[expect(
    clippy::as_conversions,
    reason = "the low half of a `DX:AX` pair: `mul si` at `0x1EB96` leaves it in `AX`"
)]
pub(crate) fn low_word(v: u32) -> u16 {
    v as u16
}

/// The high word of the same pair, `DX`.
pub(crate) fn high_word(v: u32) -> u16 {
    low_word(v >> 16)
}

/// A count or an index the original keeps in a word — an offset into the
/// 64 KiB data segment or into a resource, a slot of one of its tables, a
/// cel number. Every caller's value is bounded by the table it counts, far
/// below what a word holds.
#[expect(
    clippy::as_conversions,
    reason = "a count bounded by a table of the 64 KiB data segment, well inside a word"
)]
pub(crate) fn word(n: usize) -> u16 {
    n as u16
}

/// A count or an index the original keeps in a byte — the carried items,
/// the scroll position, a layer, a flag number. Every caller's value is
/// bounded by the list it counts, which the original sizes in bytes.
#[expect(
    clippy::as_conversions,
    reason = "a count the original keeps in a byte, bounded by the list it counts"
)]
pub(crate) fn byte(n: usize) -> u8 {
    n as u8
}

/// A count of things the engine holds — lines of a text, columns of a
/// picture, steps of a fade — as the `i32` its coordinates are computed
/// in. Every such count is bounded by a screen or a resource.
#[expect(
    clippy::as_conversions,
    reason = "a count bounded by a screen or a resource, well inside an `i32`"
)]
pub(crate) fn count(n: usize) -> i32 {
    n as i32
}

/// A count as the `u32` a fade's arithmetic multiplies in, bounded the way
/// [`count`] is.
#[expect(
    clippy::as_conversions,
    reason = "a count bounded by a fade's steps, well inside a `u32`"
)]
pub(crate) fn narrow(n: usize) -> u32 {
    n as u32
}

/// A position as an index into the buffer it addresses. Every caller has
/// bounded the value by the buffer first; one that is negative becomes an
/// index past every buffer, which a `get` refuses.
#[expect(
    clippy::as_conversions,
    reason = "a position the caller has bounded by the buffer it indexes"
)]
pub(crate) fn index(v: i32) -> usize {
    v as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rules, pinned at the edges where a bare cast and a checked
    /// conversion would part company.
    #[test]
    fn the_reinterpretations_keep_every_bit() {
        assert_eq!(signed(0xFFFF), -1);
        assert_eq!(signed(0x7FFF), 0x7FFF);
        assert_eq!(unsigned(-2), 0xFFFE);
        assert_eq!(signed8(0xFA), -6);
        assert_eq!(signed32(0x41), 0x41);
        assert_eq!(unsigned32(-1), u32::MAX);
    }

    #[test]
    fn the_truncations_keep_the_low_bits() {
        assert_eq!(low8(0x1234), 0x34);
        assert_eq!(low16(0x1_2345), 0x2345);
        assert_eq!(low16(-2), 0xFFFE);
        assert_eq!(short(0x1_8000), i16::MIN);
        assert_eq!(low_word(0x1234_5678), 0x5678);
        assert_eq!(high_word(0x1234_5678), 0x1234);
    }

    #[test]
    fn a_negative_position_is_past_every_buffer() {
        assert!([0u8; 4].get(index(-1)).is_none());
        assert_eq!(index(3), 3);
    }
}
