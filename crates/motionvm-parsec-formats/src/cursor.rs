//! A position in a byte slice that moves as fields are read.
//!
//! A reader that walks a file field by field — a directory, a relocation
//! table, a stream of tokens — would otherwise keep a `usize` and add to it
//! after every read, and each of those sums is a place where a damaged file
//! could carry the position past the end, or past `usize`, before anything
//! checked. A [`Cursor`] does the bounds check and the move in one step: a
//! read that runs past the end refuses with the offset it was at, and one
//! that succeeds is what moves the position, so the position never gets
//! ahead of the data.

use crate::error::Result;
use crate::{bytes, wide};

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

    /// `N` bytes from the cursor, as an array.
    pub(crate) fn array<const N: usize>(&mut self) -> Result<&'a [u8; N]> {
        let taken = bytes(self.data, self.at)?;
        self.advance(N);
        Ok(taken)
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(*self.array()?))
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
        let data = [1u8, 2, 3, 4, 5, 6, 7];
        let mut c = Cursor::new(&data, 1);
        assert_eq!(c.u16().unwrap(), 0x0302);
        assert!(matches!(c.u32at(), Ok(0x0706_0504)));
        assert!(matches!(
            c.u32(),
            Err(Error::Truncated {
                off: 7,
                need: 4,
                have: 7
            })
        ));
        assert!(
            c.u8().is_err(),
            "a refused read moves nothing, and 7 is the end"
        );
        let mut far = Cursor::new(&data, 9);
        assert!(far.u8().is_err());
    }
}
