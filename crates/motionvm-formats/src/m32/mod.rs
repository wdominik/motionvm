//! Readers for the formats of the 32-bit MOTION engine (`ENGINE.EXE`
//! V0.06.06/R109, as shipped with "Im Netzwerk gefangen - Dunkle Schatten 2").
//!
//! Everything under this module reads a layout that exists in the 32-bit
//! generation only, or exists in both generations with a different shape: the
//! `NNN.RSC` containers, GFX8 sprites, the 32-bit text table, the 32-bit
//! script modules, HMI songs, Ad Lib banks, driver archives and the LE binary
//! the kernel table is lifted from. What both generations share — palettes,
//! the font reference table, the decoded font and text-table structures, the
//! GFXCRUNCH LZW codec both containers pack their items with, the error type
//! and the byte helpers — lives at the crate root.

pub mod bnk;
pub mod disasm;
pub mod drv;
pub mod font;
pub mod gfx;
pub mod hmi;
pub mod le;
pub mod rsc;
pub mod scr;
pub mod text;

pub use bnk::Bank as InstrumentBank;
pub use drv::{Driver, DriverArchive};
pub use gfx::Sprite;
pub use hmi::Song;
pub use le::{Image, KernelWord};
pub use rsc::{Bank, Kind, Rsc};
pub use scr::{Entry, ScrModule};
