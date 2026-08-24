//! The files a savegame is made of.
//!
//! The original writes three per slot, all named from the id `701 + slot`:
//!
//! | file | written by | holds |
//! |---|---|---|
//! | `NNN.blk` | `PUT` | four bytes, the location number |
//! | `NNN.FRZ` | `=>PUTAS` | every resident module's memory |
//! | `NNN.anm` | `PUTANIM` | the descriptor tree and a handful of globals |
//!
//! The first is written straight from module memory and read straight back, so
//! its bytes here are the original's. The other two are not, and cannot be.
//!
//! ## Why the bytes differ
//!
//! `NEWSCREEN` and `NEWDESC` hand the scripts a raw heap pointer in the
//! original; here they hand out small handles counted from one. Both are just
//! opaque numbers to the bytecode, which only ever passes them back to `ACTSCR`
//! and `ACTDESC` — but they are numbers the scripts *store*, in the module
//! memory a savegame is mostly made of. So a saved module image from the
//! original is full of addresses that mean nothing here, and one of ours is
//! full of handles that would mean nothing there. Savegames cannot be carried
//! between the two in either direction, and no amount of matching the file
//! layout would change that. What is reproduced instead is the mechanism: the
//! same three artifacts, the same ids, the same order, the same semantics — so
//! that the game's own menu drives it, rather than a second path beside it.
//!
//! Two places where these files deliberately differ from the original beyond
//! their layout:
//!
//! * **They are checked.** `=>GETAS` (0x657a4) opens the file without looking
//!   at the handle, reads whatever length it finds, and copies it into module
//!   memory positionally — no magic, no count, no validation anywhere. A
//!   truncated or mismatched file there corrupts the running game silently.
//!   Here every record is checked *before the first byte is written back*, so a
//!   bad file stops the load instead of half-applying it.
//! * **Records are keyed by module number.** The original writes the number
//!   into each record and then ignores it on the way back in, relying on the
//!   loaded set being identical because `INCLLOC` has just rebuilt it. Reading
//!   by name costs nothing and turns a whole class of silent corruption into a
//!   named error.

use std::collections::BTreeMap;

/// What `=>PUTAS` and `=>GETAS` put at the head of a `.FRZ`.
/// Which game's files these are: the two engines keep the same three
/// artifacts, but a 16-bit module image is bytes of a flat arena where a
/// 32-bit one is cells, and a 16-bit descriptor carries a buffer — so each
/// generation has a layout and a magic of its own, and neither reads the
/// other's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Layout {
    /// Dunkle Schatten 2's, `DS2FRZ`/`DS2ANM` — the layout the released
    /// binaries have written since 0.1.
    Motion32,
    /// Die Enviro-Kids greifen ein's, `ENVFRZ`/`ENVANM`.
    Motion16,
}

impl Layout {
    fn frz_magic(self) -> &'static [u8; 8] {
        match self {
            Layout::Motion32 => b"DS2FRZ\0\0",
            Layout::Motion16 => b"ENVFRZ\0\0",
        }
    }

    pub(crate) fn anm_magic(self) -> &'static [u8; 8] {
        match self {
            Layout::Motion32 => b"DS2ANM\0\0",
            Layout::Motion16 => b"ENVANM\0\0",
        }
    }
}

pub(crate) const VERSION: u32 = 1;

pub(crate) type Result<T> = std::result::Result<T, String>;

/// A little-endian reader that refuses to run off the end.
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    what: String,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8], what: impl Into<String>) -> Self {
        Self {
            bytes,
            at: 0,
            what: what.into(),
        }
    }

    pub(crate) fn magic(&mut self, want: &[u8; 8]) -> Result<()> {
        let got = self.take(8)?;
        if got != want {
            return Err(format!("{}: not a savegame file ({got:02x?})", self.what));
        }
        let version = self.u32()?;
        if version != VERSION {
            return Err(format!(
                "{}: savegame version {version}, this build writes {VERSION}",
                self.what
            ));
        }
        Ok(())
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or_else(|| format!("{}: absurd length", self.what))?;
        if end > self.bytes.len() {
            return Err(format!(
                "{}: wanted {n} bytes at {} but the file is {} long",
                self.what,
                self.at,
                self.bytes.len()
            ));
        }
        let out = &self.bytes[self.at..end];
        self.at = end;
        Ok(out)
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }

    pub(crate) fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    /// A length-prefixed string, for the few descriptor fields that carry one.
    pub(crate) fn string(&mut self) -> Result<String> {
        let n = self.u32()? as usize;
        let bytes = self.take(n)?;
        String::from_utf8(bytes.to_vec()).map_err(|e| format!("{}: {e}", self.what))
    }

    /// Whether the file has been consumed exactly. Anything left over means the
    /// writer and the reader disagree, which is worth saying out loud.
    pub(crate) fn finish(&self) -> Result<()> {
        if self.at != self.bytes.len() {
            return Err(format!(
                "{}: {} bytes left over after reading",
                self.what,
                self.bytes.len() - self.at
            ));
        }
        Ok(())
    }
}

/// A little-endian writer, the mirror of [`Reader`].
#[derive(Default)]
pub(crate) struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    pub(crate) fn new(magic: &[u8; 8]) -> Self {
        let mut w = Self::default();
        w.bytes.extend_from_slice(magic);
        w.u32(VERSION);
        w
    }

    pub(crate) fn u32(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn i32(&mut self, v: i32) {
        self.u32(v as u32);
    }

    pub(crate) fn u8(&mut self, v: u8) {
        self.bytes.push(v);
    }

    pub(crate) fn string(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.bytes.extend_from_slice(s.as_bytes());
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// An optional value, written as a present flag and then the value.
impl Writer {
    pub(crate) fn option_i32(&mut self, v: Option<i32>) {
        self.u8(u8::from(v.is_some()));
        self.i32(v.unwrap_or(0));
    }
}

impl Reader<'_> {
    pub(crate) fn option_i32(&mut self) -> Result<Option<i32>> {
        let present = self.u8()? != 0;
        let value = self.i32()?;
        Ok(present.then_some(value))
    }
}

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
pub(crate) fn write_frz(images: &[ModuleImage], layout: Layout) -> Vec<u8> {
    let mut w = Writer::new(layout.frz_magic());
    w.u32(images.len() as u32);
    for image in images {
        w.u32(image.module);
        match layout {
            Layout::Motion32 => {
                w.u32((image.bytes.len() / 4) as u32);
                for cell in image.bytes.as_chunks::<4>().0 {
                    w.u32(u32::from_le_bytes(*cell));
                }
            }
            Layout::Motion16 => {
                w.u32(image.bytes.len() as u32);
                for &b in &image.bytes {
                    w.u8(b);
                }
            }
        }
    }
    w.bytes().to_vec()
}

/// Reads a `.FRZ` whole, before anything is applied.
///
/// Deliberately two steps. Parsing to a value and only then writing it into the
/// machine is what keeps a damaged savegame from landing half-applied — and a
/// half-applied one is the kind of fault whose symptoms nobody ever traces back
/// to its cause.
pub(crate) fn read_frz(bytes: &[u8], what: &str, layout: Layout) -> Result<Vec<ModuleImage>> {
    let mut r = Reader::new(bytes, what);
    r.magic(layout.frz_magic())?;
    let count = r.u32()? as usize;
    let mut images = Vec::with_capacity(count.min(64));
    let mut seen = BTreeMap::new();
    for i in 0..count {
        let module = r.u32()?;
        let n = r.u32()? as usize;
        let image = match layout {
            Layout::Motion32 => {
                let mut image = Vec::with_capacity((n * 4).min(1 << 22));
                for _ in 0..n {
                    image.extend_from_slice(&r.u32()?.to_le_bytes());
                }
                image
            }
            Layout::Motion16 => r.take(n)?.to_vec(),
        };
        if let Some(first) = seen.insert(module, i) {
            return Err(format!(
                "{what}: module {module} appears twice, as record {first} and {i}"
            ));
        }
        images.push(ModuleImage {
            module,
            bytes: image,
        });
    }
    r.finish()?;
    Ok(images)
}

/// The display state a `.anm` carries, in the terms this engine keeps it.
///
/// The original's file is a walk of its descriptor tree: 0x3E-byte structs and
/// per-type payloads, tagged `0x3E9` for "children follow" and so on, with the
/// active screen and four more globals at the end. Its reader rebuilds every
/// pointer from scratch, because the ones in the file are stale — which is the
/// clearest sign that the layout is an artifact of that engine's data
/// structures and not something worth copying into different ones.
///
/// What is copied is the *choice of what to keep*: the descriptors and which
/// one is current, the active screen, and the geometry of the screens. Around
/// that, three things the original has no need for.
///
/// The palette, because `SETPAL` runs in every location macro except 312 and
/// 330, and a savegame made in one of those two would otherwise come back
/// wearing the menu's colors.
///
/// The handle counter, because the original hands out heap addresses that a
/// fresh allocation can never collide with, while these are counted from one:
/// load into a freshly started game without it and the next `NEWSETDESC` gives
/// out a handle a restored descriptor already holds.
///
/// And the recipe for the sprites `GFXVFLIP` makes — the ids, not the pixels.
/// Mirrored sprites are put in the graphics pool under ids no resource file
/// contains, so a reload cannot find them again. Those a location's macro makes
/// come back with `INCLLOC`; the ones a task makes do not, until the task runs
/// again.
///
/// Deliberately *not* here: whether a screen is frozen or inactive. At the
/// moment of a save the picture is frozen, because the menu is open over it —
/// putting that back would load a game that stands still. The load path
/// finishes with `UNFREEZESCR` and `1 _INVMODE !` and sorts it out itself.
pub(crate) struct Anim {
    pub(crate) next_descriptor: u32,
    /// The handle of the current descriptor, not its index: the list is about
    /// to be replaced wholesale, and an index into the old one means nothing.
    pub(crate) current: Option<u32>,
    pub(crate) screen: Option<u32>,
    pub(crate) pointer_visible: bool,
    pub(crate) dialog_offset: i32,
    pub(crate) dialog_return: i32,
    pub(crate) palette: [u8; 768],
    pub(crate) screens: Vec<ScreenState>,
    /// `(from, to)` pairs, in the order `GFXVFLIP` was asked for them.
    pub(crate) flips: Vec<(u32, u32)>,
    pub(crate) descriptors: Vec<DescriptorState>,
    /// The 16-bit engine's off-screen buffers — the switch, and each
    /// allocated buffer's number and size. Only the 16-bit layout carries
    /// them; the 32-bit game has none.
    pub(crate) buffers_on: bool,
    pub(crate) buffers: Vec<(i32, u16, u16)>,
}

/// Everything a screen keeps that no script re-establishes on its own.
pub(crate) struct ScreenState {
    pub(crate) handle: u32,
    pub(crate) size: (u16, u16),
    pub(crate) full_view: (u16, u16),
    pub(crate) view: (u16, u16),
    pub(crate) view_pos: (i16, i16),
    pub(crate) pos: (i16, i16),
    pub(crate) origin: (i16, i16),
}

/// One descriptor, flattened. The named fields travel as names: the map holds
/// `&'static str` keys, so a name that is not one of the known ones cannot be
/// reconstructed at all and has to be an error rather than a silent drop.
pub(crate) struct DescriptorState {
    pub(crate) handle: u32,
    pub(crate) screen: u32,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) level: i32,
    pub(crate) sprite: Option<i32>,
    pub(crate) block: Option<i32>,
    pub(crate) text: Option<i32>,
    pub(crate) table: Option<i32>,
    pub(crate) font: Option<i32>,
    pub(crate) color: Option<i32>,
    pub(crate) template: Option<i32>,
    pub(crate) wait: i32,
    pub(crate) callback: i32,
    pub(crate) x_mode: u8,
    pub(crate) y_mode: u8,
    pub(crate) kind: u8,
    pub(crate) active: bool,
    pub(crate) auto_buffer: bool,
    pub(crate) fields: Vec<(String, i32)>,
    /// The buffer `SDBUF` attached, 16-bit layout only.
    pub(crate) buffer: Option<i32>,
}

pub(crate) fn write_anm(a: &Anim, layout: Layout) -> Vec<u8> {
    let mut w = Writer::new(layout.anm_magic());
    w.u32(a.next_descriptor);
    w.option_i32(a.current.map(|h| h as i32));
    w.option_i32(a.screen.map(|h| h as i32));
    w.u8(u8::from(a.pointer_visible));
    w.i32(a.dialog_offset);
    w.i32(a.dialog_return);
    for byte in a.palette {
        w.u8(byte);
    }
    w.u32(a.screens.len() as u32);
    for s in &a.screens {
        w.u32(s.handle);
        for pair in [s.size, s.full_view, s.view] {
            w.i32(pair.0 as i32);
            w.i32(pair.1 as i32);
        }
        for pair in [s.view_pos, s.pos, s.origin] {
            w.i32(pair.0 as i32);
            w.i32(pair.1 as i32);
        }
    }
    w.u32(a.flips.len() as u32);
    for (from, to) in &a.flips {
        w.u32(*from);
        w.u32(*to);
    }
    w.u32(a.descriptors.len() as u32);
    for d in &a.descriptors {
        w.u32(d.handle);
        w.u32(d.screen);
        w.i32(d.x);
        w.i32(d.y);
        w.i32(d.level);
        for v in [
            d.sprite, d.block, d.text, d.table, d.font, d.color, d.template,
        ] {
            w.option_i32(v);
        }
        w.i32(d.wait);
        w.i32(d.callback);
        w.u8(d.x_mode);
        w.u8(d.y_mode);
        w.u8(d.kind);
        w.u8(u8::from(d.active));
        w.u8(u8::from(d.auto_buffer));
        w.u32(d.fields.len() as u32);
        for (name, value) in &d.fields {
            w.string(name);
            w.i32(*value);
        }
        if layout == Layout::Motion16 {
            w.option_i32(d.buffer);
        }
    }
    if layout == Layout::Motion16 {
        w.u8(u8::from(a.buffers_on));
        w.u32(a.buffers.len() as u32);
        for &(id, width, height) in &a.buffers {
            w.i32(id);
            w.i32(width as i32);
            w.i32(height as i32);
        }
    }
    w.bytes().to_vec()
}

pub(crate) fn read_anm(bytes: &[u8], what: &str, layout: Layout) -> Result<Anim> {
    let mut r = Reader::new(bytes, what);
    r.magic(layout.anm_magic())?;
    let next_descriptor = r.u32()?;
    let current = r.option_i32()?.map(|h| h as u32);
    let screen = r.option_i32()?.map(|h| h as u32);
    let pointer_visible = r.u8()? != 0;
    let dialog_offset = r.i32()?;
    let dialog_return = r.i32()?;
    let mut palette = [0u8; 768];
    palette.copy_from_slice(r.take(768)?);

    let mut screens = Vec::new();
    for _ in 0..r.u32()? {
        let handle = r.u32()?;
        let mut u = || -> Result<(u16, u16)> { Ok((r_u16(&mut r)?, r_u16(&mut r)?)) };
        let (size, full_view, view) = (u()?, u()?, u()?);
        let mut i = || -> Result<(i16, i16)> { Ok((r_i16(&mut r)?, r_i16(&mut r)?)) };
        let (view_pos, pos, origin) = (i()?, i()?, i()?);
        screens.push(ScreenState {
            handle,
            size,
            full_view,
            view,
            view_pos,
            pos,
            origin,
        });
    }

    let mut flips = Vec::new();
    for _ in 0..r.u32()? {
        flips.push((r.u32()?, r.u32()?));
    }

    let mut descriptors = Vec::new();
    for _ in 0..r.u32()? {
        let handle = r.u32()?;
        let screen = r.u32()?;
        let (x, y, level) = (r.i32()?, r.i32()?, r.i32()?);
        let sprite = r.option_i32()?;
        let block = r.option_i32()?;
        let text = r.option_i32()?;
        let table = r.option_i32()?;
        let font = r.option_i32()?;
        let color = r.option_i32()?;
        let template = r.option_i32()?;
        let (wait, callback) = (r.i32()?, r.i32()?);
        let (x_mode, y_mode, kind) = (r.u8()?, r.u8()?, r.u8()?);
        let active = r.u8()? != 0;
        let auto_buffer = r.u8()? != 0;
        let mut fields = Vec::new();
        for _ in 0..r.u32()? {
            let name = r.string()?;
            fields.push((name, r.i32()?));
        }
        let buffer = match layout {
            Layout::Motion32 => None,
            Layout::Motion16 => r.option_i32()?,
        };
        descriptors.push(DescriptorState {
            handle,
            screen,
            x,
            y,
            level,
            sprite,
            block,
            text,
            table,
            font,
            color,
            template,
            wait,
            callback,
            x_mode,
            y_mode,
            kind,
            active,
            auto_buffer,
            fields,
            buffer,
        });
    }
    let (mut buffers_on, mut buffers) = (false, Vec::new());
    if layout == Layout::Motion16 {
        buffers_on = r.u8()? != 0;
        for _ in 0..r.u32()? {
            let id = r.i32()?;
            let (width, height) = (r_u16(&mut r)?, r_u16(&mut r)?);
            buffers.push((id, width, height));
        }
    }
    r.finish()?;
    Ok(Anim {
        next_descriptor,
        current,
        screen,
        pointer_visible,
        dialog_offset,
        dialog_return,
        palette,
        screens,
        flips,
        descriptors,
        buffers_on,
        buffers,
    })
}

fn r_u16(r: &mut Reader<'_>) -> Result<u16> {
    Ok(r.i32()? as u16)
}

fn r_i16(r: &mut Reader<'_>) -> Result<i16> {
    Ok(r.i32()? as i16)
}
