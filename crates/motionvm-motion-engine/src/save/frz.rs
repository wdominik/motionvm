//! `.FRZ`: every resident module's memory, the file `=>PUTAS` writes and
//! `=>GETAS` reads back.

use super::Layout;
use super::codec::{Error, Kind, Reader, Result, Writer, lay_out, open_body};
use motionvm_motion_formats::Generation;
use motionvm_motion_forth::cell;
use std::collections::BTreeMap;

/// One module's memory, as a `.FRZ` carries it: the bytes of the module's
/// image, cells of four on the 32-bit machine, of two on the 16-bit.
pub(crate) struct ModuleImage {
    pub(crate) module: u32,
    pub(crate) bytes: Vec<u8>,
}

/// Lays out `.FRZ`: the resident modules, in the order the original has them.
///
/// ```text
/// "DS2FRZ\0\0"  u32 version  u32 count
/// per module:   u32 number   u32 cells   u32[cells]
///
/// "ENVFRZ\0\0"  u32 version  u32 count
/// per module:   u32 number   u32 bytes   u8[bytes]
/// ```
///
/// The 32-bit original's record carries a copy of the module descriptor
/// between the number and the memory — forty-eight bytes holding, among other
/// things, two live DOS4GW heap addresses that its own reader skips over
/// without looking. There is nothing here for those to describe, so they are
/// left out rather than invented. The 16-bit original (`=>PUTAS` at
/// `ENVIRO.EXE` `12c8:1169`, file `0x16fe9`) writes one run of its arena,
/// from the first resident module's header to the last module's final
/// `##`, as it stands — addresses and all — which a rebuild that places
/// modules elsewhere cannot take back; see the savegame departure.
pub(crate) fn write_frz(images: &[ModuleImage], layout: Generation, slug: &str) -> Vec<u8> {
    let mut chunks = Writer::default();
    chunks.chunk(b"MODS", |w| write_modules(w, images, layout));
    lay_out(layout.frz_magic(), slug, &chunks)
}

/// The `MODS` section: every resident module's memory.
fn write_modules(w: &mut Writer, images: &[ModuleImage], layout: Generation) {
    w.u32(cell::narrow(images.len()));
    for image in images {
        w.u32(image.module);
        match layout {
            Generation::Motion32 => {
                w.u32(cell::narrow(image.bytes.len() / 4));
                for cell in image.bytes.as_chunks::<4>().0 {
                    w.u32(u32::from_le_bytes(*cell));
                }
            }
            Generation::Motion16 => {
                w.u32(cell::narrow(image.bytes.len()));
                for &b in &image.bytes {
                    w.u8(b);
                }
            }
        }
    }
}

/// Reads a `.FRZ` whole, before anything is applied.
///
/// Deliberately two steps. Parsing to a value and only then writing it into the
/// machine is what keeps a damaged savegame from landing half-applied — and a
/// half-applied one is the kind of fault whose symptoms nobody ever traces back
/// to its cause.
pub(crate) fn read_frz(
    bytes: &[u8],
    what: &str,
    layout: Generation,
    slug: &'static str,
) -> Result<Vec<ModuleImage>> {
    let mut r = Reader::new(bytes, what);
    r.magic(layout.frz_magic())?;
    r.version()?;
    let chunks = open_body(&mut r, what, slug)?;
    let mut mods = chunks.take(b"MODS")?;
    let images = read_modules(&mut mods, what, layout)?;
    mods.finish()?;
    Ok(images)
}

/// The module records inside `MODS`.
fn read_modules(r: &mut Reader<'_>, what: &str, layout: Generation) -> Result<Vec<ModuleImage>> {
    let count = cell::index(r.u32()?);
    let mut images = Vec::with_capacity(count.min(64));
    let mut seen = BTreeMap::new();
    for i in 0..count {
        let module = r.u32()?;
        let n = cell::index(r.u32()?);
        let image = match layout {
            Generation::Motion32 => {
                let mut image = Vec::with_capacity(n.saturating_mul(4).min(1 << 22));
                for _ in 0..n {
                    image.extend_from_slice(&r.u32()?.to_le_bytes());
                }
                image
            }
            Generation::Motion16 => r.take(n)?.to_vec(),
        };
        if let Some(first) = seen.insert(module, i) {
            return Err(Error::new(
                what,
                Kind::DuplicateModule {
                    module,
                    first,
                    second: i,
                },
            ));
        }
        images.push(ModuleImage {
            module,
            bytes: image,
        });
    }
    Ok(images)
}
