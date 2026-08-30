//! Readers for the formats of the 16-bit MOTION engine (`ENVIRO.EXE`, as
//! shipped with "Die Enviro-Kids greifen ein").
//!
//! Everything under this module reads a layout that exists in the 16-bit
//! generation only, or exists in both generations with a different shape: the
//! single `DATA.-n-` container, raw sprites, raw fonts, text tables with
//! relative offsets, script modules with 16-bit cells and global word ids, the
//! tags of the PSM 2 music blocks, the MZ executable with the kernel tables in
//! it, and the disassembler that reads a module back through them. What both generations share — palettes,
//! the font reference table, the decoded font and text-table structures, the
//! error type and the byte helpers — lives at the crate root; the 32-bit
//! engine's readers live under [`crate::m32`].
//!
//! Every layout here was measured on the one game that ships with this
//! engine: 1586 sprites, 130 blocks, 65 modules, 23 palettes, 3 fonts, 96
//! text tables and one font reference table in a 7 609 296-byte container. A
//! reader that states a count states that corpus.

pub mod dat;
pub mod disasm;
pub mod font;
pub mod gfx;
pub mod gfxinf;
pub mod mz;
pub mod psm;
pub mod scr;
pub mod text;

pub use dat::{Boot, Container, Generation, Segment};
pub use gfx::Sprite;
pub use gfxinf::GfxInf;
pub use scr::{Entry, ScrModule};
