//! What a cell's bits mean at each point a word reads them: the conversions
//! the two machines apply, each named for the rule it is.
//!
//! A cell on either data stack is an `i32`, and the original's handlers read
//! the same bits as an address, an id, a count or a byte without ever asking
//! about a sign — `@` at `0x626fc` validates nothing, and the 16-bit machine
//! pushes every result wrapped to sixteen bits. Every place such a reading
//! happens is one of the functions below, so a conversion at a call site says
//! which rule it follows, and the `#[expect(clippy::as_conversions)]` on each
//! function is the one place in the machines where a bit pattern changes
//! type. The count of those attributes is the count of rules, which is the
//! evidence a bare cast cannot give: a cast that reproduces a truncation the
//! original performs looks exactly like one that merely widens.
//!
//! Widenings are not here. `u32::from(u16)` and its kin say what they are
//! already, and a widening is never a claim about the original.

/// A cell read unsigned — an address, an id, a bit pattern: the same 32 bits
/// the stack holds, the way `Address` and every `GET`-style word take them.
#[expect(
    clippy::as_conversions,
    reason = "a reinterpretation of the same 32 bits, which is what the handlers do"
)]
pub fn unsigned(cell: i32) -> u32 {
    cell as u32
}

/// A machine value onto the data stack: the same 32 bits, read signed, the
/// way `@` pushes a fetched cell.
#[expect(
    clippy::as_conversions,
    reason = "a reinterpretation of the same 32 bits, which is what the handlers do"
)]
pub fn signed(bits: u32) -> i32 {
    bits as i32
}

/// The low 16 bits of a cell as a signed quantity — a 16-bit field of one of
/// the original's records, or the value the 16-bit machine keeps of a result.
#[expect(
    clippy::as_conversions,
    reason = "the truncation the 16-bit engine's stores perform: a cell is two bytes there"
)]
pub fn short(cell: i32) -> i16 {
    cell as i16
}

/// A cell as the 16-bit machine holds it: the low 16 bits, sign-extended into
/// the 32-bit slot the shared stack type uses — what every push on that
/// machine does (`m16::Vm::push`).
pub fn wrap16(cell: i32) -> i32 {
    i32::from(short(cell))
}

/// A 16-bit memory cell read onto the stack, signed — what the 16-bit `@`
/// pushes.
#[expect(
    clippy::as_conversions,
    reason = "a reinterpretation of the same 16 bits; the stack holds them sign-extended"
)]
pub fn sign16(cell: u16) -> i32 {
    i32::from(cell as i16)
}

/// The low 16 bits of a cell: a 16-bit address, or the two bytes a 16-bit
/// store writes.
#[expect(
    clippy::as_conversions,
    reason = "the truncation the 16-bit engine's addresses and stores perform"
)]
pub fn low16(cell: i32) -> u16 {
    cell as u16
}

/// The low byte of a cell — what `C!` stores, and what a color or a byte
/// field is once the script has computed it.
#[expect(
    clippy::as_conversions,
    reason = "the truncation `C!` performs: a byte store keeps the low eight bits"
)]
pub fn low8(cell: i32) -> u8 {
    cell as u8
}

/// A 16-bit memory cell as a signed quantity of its own width — a loop index
/// on the 16-bit return stack, compared and stepped as the handler does.
pub fn signed16(cell: u16) -> i16 {
    i16::from_le_bytes(cell.to_le_bytes())
}

/// The reverse: a signed 16-bit quantity back into the cell that holds it.
pub fn unsigned16(v: i16) -> u16 {
    u16::from_le_bytes(v.to_le_bytes())
}

/// An address inside the 16-bit machine's 64 KiB space, as the machine
/// holds it. Every caller has bounded the value by the space's size first —
/// `=>GET` refuses a module that would not fit before it places one.
#[expect(
    clippy::as_conversions,
    reason = "an address the caller has bounded by the 64 KiB space"
)]
pub fn flat(address: usize) -> u16 {
    address as u16
}

/// A module number as the 16-bit container names it: three digits, and the
/// container's slots are counted in sixteen bits.
#[expect(
    clippy::as_conversions,
    reason = "a module number, three digits in every game, as the container's 16-bit slot"
)]
pub fn module(number: u32) -> u16 {
    number as u16
}

/// A machine value as an index into a table: a module number, an ordinal, a
/// cell offset. Lossless on every target this workspace builds for, all of
/// which have a `usize` at least 32 bits wide.
#[expect(
    clippy::as_conversions,
    reason = "a widening on every target this builds for; `usize` has no `From<u32>`"
)]
pub fn index(n: u32) -> usize {
    n as usize
}

/// A stack value as an index, when it is one: `None` below zero, which is
/// what a script's `-1` for "none" is.
pub fn at(cell: i32) -> Option<usize> {
    usize::try_from(cell).ok()
}

/// A count of things the machine holds — modules, descriptors, bytes of an
/// image — as the machine's word. Every such count is bounded by a module's
/// or a savegame's size, far below what a `u32` holds.
#[expect(
    clippy::as_conversions,
    reason = "a count bounded by a module's size, well inside a `u32`"
)]
pub fn narrow(n: usize) -> u32 {
    n as u32
}

/// A count as the cell a script sees it as — the number of lines in a text,
/// the slot a descriptor sits in. Bounded the way [`narrow`] is.
#[expect(
    clippy::as_conversions,
    reason = "a count bounded by a module's size, well inside an `i32`"
)]
pub fn count(n: usize) -> i32 {
    n as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rules, pinned at the edges where a bare cast and a checked
    /// conversion would part company.
    #[test]
    fn the_reinterpretations_keep_every_bit() {
        assert_eq!(unsigned(-1), u32::MAX);
        assert_eq!(signed(u32::MAX), -1);
        assert_eq!(signed(unsigned(i32::MIN)), i32::MIN);
        assert_eq!(sign16(0xffff), -1);
        assert_eq!(sign16(0x7fff), 0x7fff);
    }

    #[test]
    fn the_truncations_keep_the_low_bits() {
        assert_eq!(low16(0x1_2345), 0x2345);
        assert_eq!(low16(-1), 0xffff);
        assert_eq!(short(0x1_8000), i16::MIN);
        assert_eq!(wrap16(0x1_8000), -0x8000);
        assert_eq!(wrap16(0x7fff), 0x7fff);
        assert_eq!(low8(0x1ff), 0xff);
        assert_eq!(low8(-1), 0xff);
    }

    #[test]
    fn an_index_below_zero_is_none() {
        assert_eq!(at(-1), None);
        assert_eq!(at(0), Some(0));
        assert_eq!(index(7), 7);
    }
}
