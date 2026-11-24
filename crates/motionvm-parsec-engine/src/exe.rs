//! A program image as a data source: addresses in, text and tables out.
//!
//! A PARSEC game keeps a good deal of what it says inside its own
//! `XP00.EXE` / `XP01.EXE` rather than in `RESOURCE/` — object names,
//! examine paragraphs, verb fail lines, menu rows, and the room
//! destinations a door writes. Transcribing those into Rust literals is
//! the unfaithful option: a literal cannot be checked against the
//! original, while a read at a cited address fails loudly when it drifts.
//! So the images are unpacked at load and read. Nothing in one is
//! executed.
//!
//! What lives here is the *mechanism*. Which address holds what is a fact
//! about the game and belongs in its own module; how to turn an address
//! into a string, a far pointer or a verified instruction immediate is the
//! same for both programs, because they were built by one compiler for
//! one memory model.
//!
//! Address arithmetic. The load image is laid out for base segment 0, and
//! far pointers stored in it are already relocated for that base, so a
//! stored `(off, seg)` pair names image offset `seg*16 + off`. A `DS:xxxx`
//! address adds to the program's own `DGROUP`, which
//! [`motionvm_parsec_formats::Unpacked::dgroup`] reads out of the Turbo-C
//! prologue — see [`Image::at`].

use std::path::Path;

use motionvm_parsec_formats::{Unpacked, cp437_to_string, find_ci};

use crate::error::Result;

/// One unpacked program image together with the data segment it was
/// linked for.
///
/// The pair travels together because neither half means anything without
/// the other: an address is written `DS:xxxx`, and which byte of the
/// image that is depends on the program.
pub(crate) struct Image {
    /// What the program is called, for error messages.
    name: String,
    /// The load image, laid out for base segment 0.
    bytes: Vec<u8>,
    /// `DGROUP` as an image offset — what a `DS:xxxx` address adds to.
    ds: usize,
}

impl Image {
    /// Unpacks `name` from the install at `dir` and reads its data
    /// segment.
    ///
    /// `expect` is the length the caller's addresses were measured on. An
    /// image of another length is refused rather than read: every offset a
    /// game's module carries belongs to one shipped build, and running
    /// them against another would return plausible garbage with no error.
    pub(crate) fn open(dir: &Path, name: &str, expect: usize) -> Result<Self> {
        let path = find_ci(dir, name).ok_or_else(|| format!("{name} not found in install dir"))?;
        let exe = Unpacked::parse(&std::fs::read(&path)?)?;
        if exe.image.len() != expect {
            return Err(format!(
                "{name} unpacks to {} bytes, expected {expect} — not the build these \
                 offsets describe",
                exe.image.len()
            )
            .into());
        }
        let ds = exe.dgroup()?;
        Ok(Image {
            name: name.to_owned(),
            bytes: exe.image,
            ds,
        })
    }

    /// The same, without insisting on a length.
    ///
    /// [`Self::open`] refuses an image of the wrong size because the game's
    /// offsets belong to one shipped build. A reader that only looks — and
    /// finds what it looks for by shape rather than by address — has no
    /// such offsets to protect, and refusing it would mean not being able
    /// to look at a build nobody has measured.
    pub(crate) fn open_any(dir: &Path, name: &str) -> Result<Self> {
        let path = find_ci(dir, name).ok_or_else(|| format!("{name} not found in install dir"))?;
        let exe = Unpacked::parse(&std::fs::read(&path)?)?;
        let ds = exe.dgroup()?;
        Ok(Image {
            name: name.to_owned(),
            bytes: exe.image,
            ds,
        })
    }

    /// The image offset of `DS:off`.
    pub(crate) fn at(&self, off: usize) -> usize {
        self.ds + off
    }

    /// The image itself, for a caller reading a shape this module has no
    /// name for.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Writes the byte at image offset `at`, as the running program writes
    /// its own tables (the location table among them). Outside the image,
    /// nothing.
    pub(crate) fn poke(&mut self, at: usize, v: u8) {
        if let Some(b) = self.bytes.get_mut(at) {
            *b = v;
        }
    }

    /// The little-endian word at image offset `at`.
    pub(crate) fn word(&self, at: usize) -> Result<u16> {
        let b = self
            .bytes
            .get(at..at + 2)
            .ok_or_else(|| format!("{}: word at {at:#x} out of range", self.name))?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    /// The far pointer stored at image offset `at`, as an image offset.
    pub(crate) fn far_ptr(&self, at: usize) -> Result<usize> {
        let off = usize::from(self.word(at)?);
        let seg = usize::from(self.word(at + 2)?);
        Ok(seg * 16 + off)
    }

    /// The NUL-terminated CP437 string at image offset `at`.
    pub(crate) fn c_string(&self, at: usize) -> Result<String> {
        let tail = self
            .bytes
            .get(at..)
            .ok_or_else(|| format!("{}: string at {at:#x} out of range", self.name))?;
        let end = tail
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| format!("{}: string at {at:#x} runs to end of image", self.name))?;
        Ok(cp437_to_string(&tail[..end]))
    }

    /// Strings behind a far-pointer row that ends in a null pointer.
    ///
    /// The original counts such a row itself — `1EBDh:0000h` in Dunkle
    /// Schatten scans words for a zero and stops on two in a row, which is
    /// a null far pointer — so a reader that took a fixed count would be
    /// inventing one. A row that runs to the end of the image is an error
    /// rather than a very long list.
    pub(crate) fn ptr_list(&self, at: usize) -> Result<Vec<String>> {
        let mut out = Vec::new();
        let mut cur = at;
        loop {
            if self.word(cur)? == 0 && self.word(cur + 2)? == 0 {
                return Ok(out);
            }
            out.push(self.c_string(self.far_ptr(cur)?)?);
            cur += 4;
            if out.len() > 4096 {
                return Err(format!("{}: pointer row at {at:#x} has no null", self.name).into());
            }
        }
    }

    /// `n` strings behind the far-pointer row at image offset `at`.
    pub(crate) fn ptr_table(&self, at: usize, n: usize) -> Result<Vec<String>> {
        (0..n)
            .map(|i| self.c_string(self.far_ptr(at + 4 * i)?))
            .collect()
    }

    /// The immediate of the instruction at image offset `at`, after
    /// checking that the instruction is the one the caller named.
    ///
    /// This is how a table that the original does not keep as a table is
    /// read. A room destination, for instance, is not a row anywhere: it
    /// is the `imm` of a `mov word [2F9Ah], imm` compiled into a room
    /// handler. The caller carries the instruction's address and the bytes
    /// that open it; a drifted address then fails the load instead of
    /// yielding a plausible wrong number.
    ///
    /// `head` is the opcode and its operand bytes, and the immediate is
    /// the two bytes that follow them.
    pub(crate) fn imm16(&self, at: usize, head: &[u8], what: &str) -> Result<u16> {
        let n = head.len();
        let b = self
            .bytes
            .get(at..at + n + 2)
            .ok_or_else(|| format!("{}: {what} at {at:#x} out of range", self.name))?;
        if &b[..n] != head {
            return Err(format!(
                "{}: {at:#x} is not {what} (expected {head:02x?}, found {:02x?})",
                self.name,
                &b[..n]
            )
            .into());
        }
        Ok(u16::from_le_bytes([b[n], b[n + 1]]))
    }
}

/// The opcodes, with their operand bytes, that open the instructions whose
/// immediates the titles read ([`Image::imm16`]).
pub(crate) mod op {
    /// `push imm16`.
    pub(crate) const PUSH_IMM16: [u8; 1] = [0x68];
    /// `push word [bx+imm16]`.
    pub(crate) const PUSH_BX_MEM: [u8; 2] = [0xFF, 0xB7];
    /// `mov bx, [imm16]`.
    pub(crate) const MOV_BX_MEM: [u8; 2] = [0x8B, 0x1E];
}
