//! `.anm`: the display state `PUTANIM` writes and `GETANIM` reads back —
//! the descriptors, the screens, the palette and a handful of globals.

use super::Layout;
use super::codec::{Reader, Result, Writer, lay_out, open_body, r_i16, r_u16};
use motionvm_motion_formats::Generation;
use motionvm_motion_forth::cell;

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
    /// What the descriptor shows, as `(tag, value)`: 0 nothing, 1 a sprite,
    /// 2 a picture. One field, because the engine has one — see
    /// [`crate::Shows`].
    pub(crate) shows: (u8, i32),
    pub(crate) text: Option<i32>,
    pub(crate) font: Option<i32>,
    pub(crate) color: Option<i32>,
    pub(crate) template: Option<i32>,
    pub(crate) wait: i32,
    pub(crate) callback: i32,
    pub(crate) x_mode: u8,
    pub(crate) y_mode: u8,
    pub(crate) active: bool,
    pub(crate) auto_buffer: bool,
    pub(crate) fields: Vec<(String, i32)>,
    /// The buffer `SDBUF` attached, 16-bit layout only.
    pub(crate) buffer: Option<i32>,
}

pub(crate) fn write_anm(a: &Anim, layout: Generation, slug: &str) -> Vec<u8> {
    let mut chunks = Writer::default();
    chunks.chunk(b"HEAD", |w| write_globals(w, a));
    chunks.chunk(b"SCRN", |w| write_screens(w, &a.screens));
    chunks.chunk(b"FLIP", |w| write_flips(w, &a.flips));
    chunks.chunk(b"DESC", |w| write_descriptors(w, &a.descriptors, layout));
    if layout == Generation::Motion16 {
        chunks.chunk(b"BUFS", |w| write_buffers(w, a.buffers_on, &a.buffers));
    }
    lay_out(layout.anm_magic(), slug, &chunks)
}

/// `HEAD`: the handful of values that belong to no list.
pub(super) fn write_globals(w: &mut Writer, a: &Anim) {
    w.u32(a.next_descriptor);
    w.option_i32(a.current.map(cell::signed));
    w.option_i32(a.screen.map(cell::signed));
    w.u8(u8::from(a.pointer_visible));
    w.i32(a.dialog_offset);
    w.i32(a.dialog_return);
    for byte in a.palette {
        w.u8(byte);
    }
}

/// `SCRN`: the geometry of every screen.
pub(super) fn write_screens(w: &mut Writer, screens: &[ScreenState]) {
    w.u32(cell::narrow(screens.len()));
    for s in screens {
        w.u32(s.handle);
        for pair in [s.size, s.full_view, s.view] {
            w.i32(i32::from(pair.0));
            w.i32(i32::from(pair.1));
        }
        for pair in [s.view_pos, s.pos, s.origin] {
            w.i32(i32::from(pair.0));
            w.i32(i32::from(pair.1));
        }
    }
}

/// `FLIP`: the recipe for the mirrored sprites, in the order they were asked
/// for.
pub(super) fn write_flips(w: &mut Writer, flips: &[(u32, u32)]) {
    w.u32(cell::narrow(flips.len()));
    for (from, to) in flips {
        w.u32(*from);
        w.u32(*to);
    }
}

/// `DESC`: the scene graph.
pub(super) fn write_descriptors(
    w: &mut Writer,
    descriptors: &[DescriptorState],
    layout: Generation,
) {
    w.u32(cell::narrow(descriptors.len()));
    for d in descriptors {
        w.u32(d.handle);
        w.u32(d.screen);
        w.i32(d.x);
        w.i32(d.y);
        w.i32(d.level);
        w.u8(d.shows.0);
        w.i32(d.shows.1);
        for v in [d.text, d.font, d.color, d.template] {
            w.option_i32(v);
        }
        w.i32(d.wait);
        w.i32(d.callback);
        w.u8(d.x_mode);
        w.u8(d.y_mode);
        w.u8(u8::from(d.active));
        w.u8(u8::from(d.auto_buffer));
        w.u32(cell::narrow(d.fields.len()));
        for (name, value) in &d.fields {
            w.string(name);
            w.i32(*value);
        }
        if layout == Generation::Motion16 {
            w.option_i32(d.buffer);
        }
    }
}

/// The switch and the allocated buffers, as `BUFS` carries them: a buffer is
/// its number and its size.
type Buffers = (bool, Vec<(i32, u16, u16)>);

/// `BUFS`: the 16-bit engine's off-screen buffers. The 32-bit engine has
/// none, so its files carry no such section at all — which is what an
/// optional section is for.
fn write_buffers(w: &mut Writer, on: bool, buffers: &[(i32, u16, u16)]) {
    w.u8(u8::from(on));
    w.u32(cell::narrow(buffers.len()));
    for &(id, width, height) in buffers {
        w.i32(id);
        w.i32(i32::from(width));
        w.i32(i32::from(height));
    }
}

pub(crate) fn read_anm(
    bytes: &[u8],
    what: &str,
    layout: Generation,
    slug: &'static str,
) -> Result<Anim> {
    let mut r = Reader::new(bytes, what);
    r.magic(layout.anm_magic())?;
    r.version()?;
    let chunks = open_body(&mut r, what, slug)?;
    let mut head = chunks.take(b"HEAD")?;
    let globals = read_globals(&mut head)?;
    head.finish()?;

    let mut scrn = chunks.take(b"SCRN")?;
    let screens = read_screens(&mut scrn)?;
    scrn.finish()?;

    let mut flip = chunks.take(b"FLIP")?;
    let flips = read_flips(&mut flip)?;
    flip.finish()?;

    let mut desc = chunks.take(b"DESC")?;
    let descriptors = read_descriptors(&mut desc, layout)?;
    desc.finish()?;

    // Absent from a 32-bit file by design, so its absence is not a fault: no
    // buffers is what a 32-bit engine has.
    let (buffers_on, buffers) = match chunks.option(b"BUFS") {
        Some(mut bufs) => {
            let read = read_buffers(&mut bufs)?;
            bufs.finish()?;
            read
        }
        None => (false, Vec::new()),
    };
    Ok(globals.into_anim(screens, flips, descriptors, buffers_on, buffers))
}

/// What `HEAD` carries, on its way to becoming an [`Anim`].
///
/// A type of its own so that the reader hands one value to the constructor:
/// seven values passed positionally would be seven chances to swap two
/// `i32`s that mean different things.
struct Globals {
    next_descriptor: u32,
    current: Option<u32>,
    screen: Option<u32>,
    pointer_visible: bool,
    dialog_offset: i32,
    dialog_return: i32,
    palette: [u8; 768],
}

impl Globals {
    fn into_anim(
        self,
        screens: Vec<ScreenState>,
        flips: Vec<(u32, u32)>,
        descriptors: Vec<DescriptorState>,
        buffers_on: bool,
        buffers: Vec<(i32, u16, u16)>,
    ) -> Anim {
        Anim {
            next_descriptor: self.next_descriptor,
            current: self.current,
            screen: self.screen,
            pointer_visible: self.pointer_visible,
            dialog_offset: self.dialog_offset,
            dialog_return: self.dialog_return,
            palette: self.palette,
            screens,
            flips,
            descriptors,
            buffers_on,
            buffers,
        }
    }
}

fn read_globals(r: &mut Reader<'_>) -> Result<Globals> {
    let next_descriptor = r.u32()?;
    let current = r.option_i32()?.map(cell::unsigned);
    let screen = r.option_i32()?.map(cell::unsigned);
    let pointer_visible = r.u8()? != 0;
    let dialog_offset = r.i32()?;
    let dialog_return = r.i32()?;
    let mut palette = [0u8; 768];
    palette.copy_from_slice(r.take(768)?);
    Ok(Globals {
        next_descriptor,
        current,
        screen,
        pointer_visible,
        dialog_offset,
        dialog_return,
        palette,
    })
}

fn read_screens(r: &mut Reader<'_>) -> Result<Vec<ScreenState>> {
    let mut screens = Vec::new();
    for _ in 0..r.u32()? {
        let handle = r.u32()?;
        let mut u = || -> Result<(u16, u16)> { Ok((r_u16(r)?, r_u16(r)?)) };
        let (size, full_view, view) = (u()?, u()?, u()?);
        let mut i = || -> Result<(i16, i16)> { Ok((r_i16(r)?, r_i16(r)?)) };
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
    Ok(screens)
}

fn read_flips(r: &mut Reader<'_>) -> Result<Vec<(u32, u32)>> {
    let mut flips = Vec::new();
    for _ in 0..r.u32()? {
        flips.push((r.u32()?, r.u32()?));
    }
    Ok(flips)
}

fn read_descriptors(r: &mut Reader<'_>, layout: Generation) -> Result<Vec<DescriptorState>> {
    let mut descriptors = Vec::new();
    for _ in 0..r.u32()? {
        let handle = r.u32()?;
        let screen = r.u32()?;
        let (x, y, level) = (r.i32()?, r.i32()?, r.i32()?);
        let shows = (r.u8()?, r.i32()?);
        let text = r.option_i32()?;
        let font = r.option_i32()?;
        let color = r.option_i32()?;
        let template = r.option_i32()?;
        let (wait, callback) = (r.i32()?, r.i32()?);
        let (x_mode, y_mode) = (r.u8()?, r.u8()?);
        let active = r.u8()? != 0;
        let auto_buffer = r.u8()? != 0;
        let mut fields = Vec::new();
        for _ in 0..r.u32()? {
            let name = r.string()?;
            fields.push((name, r.i32()?));
        }
        let buffer = match layout {
            Generation::Motion32 => None,
            Generation::Motion16 => r.option_i32()?,
        };
        descriptors.push(DescriptorState {
            handle,
            screen,
            x,
            y,
            level,
            shows,
            text,
            font,
            color,
            template,
            wait,
            callback,
            x_mode,
            y_mode,
            active,
            auto_buffer,
            fields,
            buffer,
        });
    }
    Ok(descriptors)
}

fn read_buffers(r: &mut Reader<'_>) -> Result<Buffers> {
    let on = r.u8()? != 0;
    let mut buffers = Vec::new();
    for _ in 0..r.u32()? {
        let id = r.i32()?;
        let (width, height) = (r_u16(r)?, r_u16(r)?);
        buffers.push((id, width, height));
    }
    Ok((on, buffers))
}
