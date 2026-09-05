//! A position in a byte slice that moves as fields are read.
//!
//! The readers that walk a file field by field — an event stream, a fixup
//! table, a dictionary — kept a `usize` and added to it after every read,
//! which is a few hundred places where a damaged file could carry the sum
//! past the end, or past `usize`, before anything checked. A [`Cursor`]
//! does the bounds check and the move in one step: a read that runs past the
//! end refuses with the offset it was at, and one that succeeds is what moves
//! the position, so the position never gets ahead of the data.

use crate::error::Result;
use crate::{bytes, records, slice, u8at, wide};

/// A read position inside a byte slice.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    /// A cursor over `data`, standing at `at` — which may be past the end;
    /// the first read then says so.
    pub(crate) fn new(data: &'a [u8], at: usize) -> Self {
        Self { data, at }
    }

    /// Where the cursor stands, as an offset into the data.
    pub(crate) fn position(&self) -> usize {
        self.at
    }

    /// Moves the cursor to `at`.
    pub(crate) fn seek(&mut self, at: usize) {
        self.at = at;
    }

    /// Everything from the cursor on — nothing, when it stands at or past
    /// the end.
    pub(crate) fn remaining(&self) -> &'a [u8] {
        self.data.get(self.at..).unwrap_or_default()
    }

    /// The byte under the cursor, without moving.
    pub(crate) fn peek(&self) -> Result<u8> {
        u8at(self.data, self.at)
    }

    /// `n` bytes from the cursor, which then stands past them.
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let taken = slice(self.data, self.at, n)?;
        self.advance(n);
        Ok(taken)
    }

    /// `N` bytes from the cursor, as an array.
    pub(crate) fn array<const N: usize>(&mut self) -> Result<&'a [u8; N]> {
        let taken = bytes(self.data, self.at)?;
        self.advance(N);
        Ok(taken)
    }

    /// `count` records of `N` bytes each from the cursor.
    pub(crate) fn records<const N: usize>(&mut self, count: usize) -> Result<&'a [[u8; N]]> {
        let taken = records(self.data, self.at, count)?;
        self.advance(taken.as_flattened().len());
        Ok(taken)
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(*self.array()?))
    }

    pub(crate) fn i16(&mut self) -> Result<i16> {
        Ok(i16::from_le_bytes(*self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(*self.array()?))
    }

    /// A `u32` field as what it is used for next: an offset into, or a count
    /// of things in, the file it came out of.
    pub(crate) fn u32at(&mut self) -> Result<usize> {
        self.u32().map(wide)
    }

    /// Moves past `n` bytes that a read has just taken, which bounded the sum
    /// by the data's length.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bounded by the read that succeeded just before"
    )]
    fn advance(&mut self, n: usize) {
        self.at += n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    #[test]
    fn a_read_moves_the_cursor_and_a_short_one_leaves_it_where_it_was() {
        let data = [1u8, 2, 3, 4, 5];
        let mut c = Cursor::new(&data, 1);
        assert_eq!(c.u16().unwrap(), 0x0302);
        assert_eq!(c.position(), 3);
        assert!(matches!(
            c.u32(),
            Err(Error::Truncated {
                off: 3,
                need: 4,
                have: 5
            })
        ));
        assert_eq!(c.position(), 3, "a refused read moves nothing");
        assert_eq!(c.take(2).unwrap(), &[4, 5]);
        assert!(c.remaining().is_empty());
    }

    #[test]
    fn records_take_whole_records_and_a_cursor_past_the_end_reads_nothing() {
        let data = [1u8, 2, 3, 4, 5];
        let mut c = Cursor::new(&data, 0);
        assert_eq!(c.records::<2>(2).unwrap(), &[[1, 2], [3, 4]]);
        assert_eq!(c.position(), 4);
        assert!(c.records::<2>(1).is_err(), "one byte is not a record");
        let mut far = Cursor::new(&data, 9);
        assert!(far.peek().is_err());
        assert!(far.remaining().is_empty());
        assert!(far.u8().is_err());
        assert_eq!(far.position(), 9);
    }
}
