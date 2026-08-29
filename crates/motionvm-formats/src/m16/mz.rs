//! `ENVIRO.EXE` as a file: the MZ header, the load image behind it, and the
//! kernel word tables in that image.
//!
//! The 16-bit engine is a plain real-mode DOS executable. Its header says how
//! long it is in paragraphs — 800, so the load image begins at file offset
//! `0x3200` — and a far pointer `segment:offset` inside the image is file
//! offset `0x3200 + segment * 16 + offset`. That is how the two kernel
//! tables are read: each is a NUL-terminated array of 8-byte entries
//!
//! ```text
//! u16 name offset, u16 name segment, u16 handler offset, u16 handler segment
//! ```
//!
//! terminated by an entry whose name points at an empty string and whose
//! handler is `0000:0000`. Measured on ENVIRO.EXE (167 430 bytes): the table
//! of 151 domain words is at file `0x1f9f6`, the table of 82 core words at
//! `0x2064e`, their name strings together at `0x1fec5`–`0x20aae`.
//!
//! The tables are found by scanning, the way the 32-bit engine's are
//! ([`crate::m32::le::kernel_words`]): a run of at least six entries whose
//! name pointer resolves to a printable string and whose handler pointer lies
//! inside the image is a table. No other run in the file passes that test.
//!
//! **Ordinals.** A kernel cell is `0x8000 | ordinal`, and the ordinal is the
//! entry's index plus a per-table base: 1 for the core table, 105 for the
//! domain table — in file order the domain table comes first, so table 0
//! is 105-based and table 1 is 1-based. The bases are measured from the
//! modules, not read from the binary: every one of the 24 282 kernel cells in
//! the 65 modules of Die Enviro-Kids greifen ein names an entry under this rule,
//! `##` is cell `0x8001`
//! at the end of every colon definition, and `TOGFX` is `0x8069` where `RUN`
//! enters graphics mode. Ordinals 83–104 fall between the tables and occur
//! in no module.

use crate::error::{Error, Result};
use crate::kernel::{Binding, Inline, KernelWord};
use crate::u16le;

/// The bit that marks a cell as a kernel word.
pub const KERNEL_BIT: u16 = 0x8000;
/// What is left of a kernel cell once the bit is taken off.
pub const ORDINAL_MASK: u16 = 0x7fff;
/// The ordinal of `##`, the word that returns from a colon definition.
pub const RETURN_ORDINAL: u32 = 1;

/// Ordinal of the first entry of each table, indexed by table number in
/// file order: the domain table first, the core table second.
const TABLE_BASE: [u32; 2] = [105, 1];

/// The executable, read whole.
pub struct Image {
    data: Vec<u8>,
    header_len: usize,
}

impl Image {
    /// Reads an MZ executable from disk.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Self::parse(std::fs::read(path)?)
    }

    /// The same from memory.
    pub fn parse(data: Vec<u8>) -> Result<Self> {
        if data.len() < 0x1c {
            return Err(Error::Truncated {
                off: 0,
                need: 0x1c,
                have: data.len(),
            });
        }
        if &data[..2] != b"MZ" && &data[..2] != b"ZM" {
            return Err(Error::Corrupt {
                what: "MZ executable",
                detail: format!("signature is {:02x?}, expected MZ", &data[..2]),
            });
        }
        let header_len = u16le(&data, 8)? as usize * 16;
        if header_len > data.len() {
            return Err(Error::Corrupt {
                what: "MZ executable",
                detail: format!("header of {header_len} bytes in a {}-byte file", data.len()),
            });
        }
        Ok(Self { data, header_len })
    }

    /// Bytes of header before the load image — `0x3200` in ENVIRO.EXE.
    pub fn header_len(&self) -> usize {
        self.header_len
    }

    /// The whole file.
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    /// The load image: everything after the header.
    pub fn load_image(&self) -> &[u8] {
        &self.data[self.header_len..]
    }

    /// The file offset a far pointer names.
    pub fn file_offset(&self, segment: u16, offset: u16) -> usize {
        self.header_len + segment as usize * 16 + offset as usize
    }

    /// The NUL-terminated string at a file offset, if it is one of at most
    /// `max` printable ASCII bytes.
    fn cstr_at(&self, off: usize, max: usize) -> Option<&str> {
        let bytes = self.data.get(off..)?;
        let end = bytes.iter().take(max + 1).position(|&b| b == 0)?;
        let s = &bytes[..end];
        if s.is_empty() || !s.iter().all(|&b| (0x21..0x7f).contains(&b)) {
            return None;
        }
        std::str::from_utf8(s).ok()
    }
}

/// Finds the kernel's word tables in the executable.
///
/// The tables are not aligned and not pointed at by anything the loader
/// reads, so the scan walks the image byte by byte. `table` in the answer is
/// the table's number in file order, `index` the entry's position in it,
/// `handler` and `entry` file offsets.
pub fn kernel_words(img: &Image) -> Vec<KernelWord> {
    const ENTRY: usize = 8;
    const MIN_RUN: usize = 6;
    let data = img.bytes();
    let plausible = |at: usize| -> Option<(String, u32)> {
        let name_off = u16le(data, at).ok()?;
        let name_seg = u16le(data, at + 2).ok()?;
        let fn_off = u16le(data, at + 4).ok()?;
        let fn_seg = u16le(data, at + 6).ok()?;
        let name = img.cstr_at(img.file_offset(name_seg, name_off), 24)?;
        let handler = img.file_offset(fn_seg, fn_off);
        (img.header_len()..data.len())
            .contains(&handler)
            .then(|| (name.to_string(), handler as u32))
    };

    let mut words = Vec::new();
    let mut table = 0;
    let mut at = img.header_len();
    while at + ENTRY <= data.len() {
        if plausible(at).is_none() {
            at += 1;
            continue;
        }
        let start = at;
        let mut run = Vec::new();
        while at + ENTRY <= data.len() {
            let Some((name, handler)) = plausible(at) else {
                break;
            };
            run.push((at, name, handler));
            at += ENTRY;
        }
        if run.len() >= MIN_RUN {
            for (index, (entry, name, handler)) in run.into_iter().enumerate() {
                words.push(KernelWord {
                    name,
                    handler,
                    entry: entry as u32,
                    table,
                    index,
                });
            }
            table += 1;
        } else {
            at = start + 1;
        }
    }
    words
}

/// The bytecode ordinal for a kernel word: its index plus its table's base.
pub fn ordinal_of(word: &KernelWord) -> Option<u32> {
    TABLE_BASE
        .get(word.table)
        .map(|base| base + word.index as u32)
}

/// The binding of a 16-bit kernel: every word at its ordinal, and the inline
/// set read off the names. Fails if the tables do not name one of the inline
/// words — which is what a scan that found the wrong runs looks like.
pub fn binding_of(words: &[KernelWord]) -> Result<Binding> {
    let mut bound: Vec<(u32, String)> = words
        .iter()
        .filter_map(|w| ordinal_of(w).map(|o| (o, w.name.clone())))
        .collect();
    bound.sort_by_key(|&(o, _)| o);
    let inline = Inline::by_name(&bound).ok_or_else(|| Error::Corrupt {
        what: "kernel table",
        detail: format!(
            "{} words found, but not every inline word among them",
            bound.len()
        ),
    })?;
    Ok(Binding {
        words: bound,
        inline,
    })
}
