//! Readers for the formats of the 16-bit MOTION engine — `ENVIRO.EXE` as
//! shipped with "Die Enviro-Kids greifen ein", and its older builds
//! `HPPLAY.EXE`, `BMZ.EXE` and `LL.EXE`.
//!
//! Everything under this module reads a layout that exists in the 16-bit
//! generation only, or exists in both generations with a different shape: the
//! single `DATA.-n-` container, raw sprites, raw fonts, text tables with
//! relative offsets, script modules with 16-bit cells and global word ids, the
//! tags of the PSM 2 music blocks, the MZ executable with the kernel tables in
//! it, the sprite-size sidecar the earlier framing's games ship, and the
//! disassembler that reads a module back through them. What both generations share — palettes,
//! the font reference table, the decoded font and text-table structures, the
//! error type and the byte helpers — lives at the crate root; the 32-bit
//! engine's readers live under [`crate::m32`].
//!
//! Layouts here are measured on the games that ship on this engine, and a
//! reader that states a count states which. Die Enviro-Kids greifen ein is the
//! one most of them were read on: 1586 sprites, 130 blocks, 65 modules, 23
//! palettes, 3 fonts, 96 text tables and one font reference table in a
//! 7 609 296-byte container.

pub mod dat;
pub mod disasm;
pub mod font;
pub mod gfx;
pub mod gfxinf;
pub mod mz;
pub mod psm;
pub mod scr;
pub mod text;

pub use dat::{Boot, Container, Framing, Segment};
pub use gfx::Sprite;
pub use gfxinf::GfxInf;
pub use scr::{Entry, ScrModule};
