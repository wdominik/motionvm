//! Where the pictures, fonts, texts and songs come from.
//!
//! Everything the game draws with is a numbered item in a resource container,
//! loaded on first use and kept. Which container depends on the engine
//! generation — the 32-bit engine's game ships `NNN.RSC` banks, the 16-bit
//! engine's one `DATA.-1-` — and [`Resources`] is the engine's one handle on
//! either: a reader reads one layout, and which reader answers is decided
//! here, once, by which container the game directory held.
//!
//! The module-slot bookkeeping lives here too. `=>GET` and `=>ERASE` move no
//! memory in the 32-bit rebuild — every module is resident from the start —
//! but they keep the slot table, and the *order* of that table is what a
//! savegame's module images are written in.

use crate::{Descriptor, Engine};
use motionvm_formats::font::{Font, FontRefTable};
use motionvm_formats::m16::{Container, Segment};
use motionvm_formats::m32::{Kind, Sprite, rsc::Bank};
use motionvm_formats::{Palette, TextTable};

/// The game's resources, read through the reader of its engine generation.
///
/// Every arm reads one layout and nothing else. What comes out is the same
/// for both — a decoded picture, a palette, a font, a text table, a block of
/// bytes — which is what lets the words and the drawer be written once.
pub(crate) enum Resources {
    /// The 32-bit engine's `NNN.RSC` banks, merged into one id space.
    Motion32(Bank),
    /// The 16-bit engine's `DATA.-n-` volumes, opened as one container.
    ///
    /// Boxed because the 16-bit container carries a table per volume and the
    /// buffer its packed items were unfolded into, which would otherwise make
    /// every `Resources` — including the 32-bit one, which holds a handful of
    /// pointers — as wide as the widest.
    Motion16(Box<Container>),
}

impl Resources {
    /// A decoded sprite.
    ///
    /// The picture type is the 32-bit sprite's, because that is the type the
    /// renderer blits. A 16-bit sprite carries no palette of its own, so its
    /// palette half is left all zero; nothing reads it for a 16-bit game,
    /// whose `RUN` installs a palette with `SETPAL` before anything is drawn.
    pub(crate) fn sprite(&self, id: u32) -> Option<Sprite> {
        match self {
            Resources::Motion32(bank) => {
                let item = bank.item(Kind::Gfx8, id as usize).ok()??;
                Sprite::parse(item).ok()
            }
            Resources::Motion16(c) => {
                let item = c.item(Segment::Gfx, id as usize).ok()??;
                let raw = motionvm_formats::m16::gfx::Sprite::parse(item).ok()?;
                Some(Sprite {
                    width: raw.width,
                    height: raw.height,
                    palette: Palette::from_6bit(&[0; Palette::BYTES]),
                    pixels: raw.pixels,
                })
            }
        }
    }

    /// A text table by id.
    pub(crate) fn text(&self, id: i32) -> Option<TextTable> {
        let id = usize::try_from(id).ok()?;
        match self {
            Resources::Motion32(bank) => {
                let item = bank.item(Kind::Text, id).ok()??;
                motionvm_formats::m32::text::parse(item).ok()
            }
            Resources::Motion16(c) => {
                let item = c.item(Segment::Txt, id).ok()??;
                motionvm_formats::m16::text::parse(item).ok()
            }
        }
    }

    /// A palette by id — the same 768 bytes in both generations.
    pub(crate) fn palette(&self, id: i32) -> Option<Palette> {
        let id = usize::try_from(id).ok()?;
        let item = match self {
            Resources::Motion32(bank) => bank.item(Kind::Palette, id).ok()??,
            Resources::Motion16(c) => c.item(Segment::Pal, id).ok()??,
        };
        (item.len() == Palette::BYTES).then(|| Palette::from_6bit(item))
    }

    /// A font by id, decoded.
    pub(crate) fn font(&self, id: i32) -> Option<Font> {
        let id = usize::try_from(id).ok()?;
        match self {
            Resources::Motion32(bank) => {
                let item = bank.item(Kind::Font, id).ok()??;
                motionvm_formats::m32::font::parse(item).ok()
            }
            Resources::Motion16(c) => {
                let item = c.item(Segment::Fnt, id).ok()??;
                motionvm_formats::m16::font::parse(item).ok()
            }
        }
    }

    /// A block's bytes — a song, a table, whatever the game keeps there.
    pub(crate) fn block(&self, id: i32) -> Option<Vec<u8>> {
        let id = usize::try_from(id).ok()?;
        let item = match self {
            Resources::Motion32(bank) => bank.item(Kind::Block, id).ok()??,
            Resources::Motion16(c) => c.item(Segment::Blk, id).ok()??,
        };
        Some(item.to_vec())
    }

    /// A script module's raw item, for a machine that loads them on demand.
    pub(crate) fn script(&self, number: u32) -> Option<Vec<u8>> {
        let item = match self {
            Resources::Motion32(bank) => bank.item(Kind::Script, number as usize).ok()??,
            Resources::Motion16(c) => c.item(Segment::Scr, number as usize).ok()??,
        };
        Some(item.to_vec())
    }

    /// The character-to-glyph table: the loose `000.FRT` beside the 32-bit
    /// banks, FRT slot 0 inside the 16-bit container — the same 516 bytes.
    pub(crate) fn font_refs(&self, dir: &std::path::Path) -> Option<FontRefTable> {
        match self {
            // Through `find_ci` rather than `join`: the shipped names are
            // upper case but a copied install is often not, and on a
            // case-sensitive filesystem an exact join silently finds nothing —
            // which here would mean a game that draws no text rather than one
            // that says why.
            Resources::Motion32(_) => motionvm_formats::find_ci(dir, "000.FRT")
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|d| FontRefTable::parse(&d).ok()),
            Resources::Motion16(c) => {
                let item = c.item(Segment::Frt, 0).ok()??;
                FontRefTable::parse(item).ok()
            }
        }
    }

    /// The font a text draws in when nothing chose one: the loose `000.FNT`
    /// beside the 32-bit banks; for a 16-bit game font 0 of the container —
    /// the text face, which is what `0 SDFNT` at eight of its sites reads as.
    /// Read now, both ends: `SDFNT` (`05f1:1375`) stores a bare table index
    /// at `+0x2b`, and `NEWANIM` (`05f1:0021`) runs `0 +FONT` first thing —
    /// the container's font 0 is the first entry of that table, the face a
    /// fresh descriptor's zero picks.
    pub(crate) fn system_font(&self, dir: &std::path::Path) -> Option<Font> {
        match self {
            Resources::Motion32(_) => motionvm_formats::find_ci(dir, "000.FNT")
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|d| motionvm_formats::m32::font::parse(&d).ok()),
            Resources::Motion16(_) => self.font(0),
        }
    }
}

impl Engine {
    /// Attaches a 32-bit game's resources so sprites and fonts can be loaded.
    pub(crate) fn with_bank(self, dir: &std::path::Path, bank: Bank) -> Self {
        self.with_resources(dir, Resources::Motion32(bank))
    }

    /// Attaches a 16-bit game's resources.
    pub(crate) fn with_container(mut self, dir: &std::path::Path, container: Container) -> Self {
        // The 16-bit drawer copies a block whole, and its descriptors are
        // numbered per screen; see [`Engine::opaque_blocks`] and
        // [`Engine::per_screen_descriptors`].
        self.opaque_blocks = true;
        self.per_screen_descriptors = true;
        self.text16 = true;
        self.level_chain = true;
        // The pointer starts unshown on this machine — see
        // [`Engine::pointer_shows`].
        self.sd_marks_always = true;
        self.callbacks_need_active = true;
        self.pointer_counted = true;
        self.pointer_visible = false;
        self.save_layout = crate::save::Layout::Motion16;
        self.with_resources(dir, Resources::Motion16(Box::new(container)))
    }

    fn with_resources(mut self, dir: &std::path::Path, resources: Resources) -> Self {
        self.dir = Some(dir.to_path_buf());
        self.font_refs = resources.font_refs(dir);
        self.system_font = resources.system_font(dir);
        self.resources = Some(resources);
        self
    }

    /// Puts a module in the first free descriptor slot, as `=>GET` does.
    ///
    /// `=>GET` (0x64999) scans for the first slot whose memory pointer is null
    /// and takes it, which together with `=>ERASE` freeing slots in place is
    /// what makes the order reproducible at all.
    ///
    /// A module that is already resident keeps its slot. The original treats
    /// that as an error unless a flag is set (0x64a0d), but nothing in the game
    /// does it: `INCLLOC` erases before it gets, and `START` takes each module
    /// once.
    pub(crate) fn mark_resident(&mut self, module: u32) {
        if self.slots.contains(&Some(module)) {
            return;
        }
        if let Some(slot) = self.slots.iter_mut().skip(1).find(|s| s.is_none()) {
            *slot = Some(module);
        }
    }

    /// Frees a module's slot, as `=>ERASE` does. Unknown modules are ignored —
    /// the first `INCLLOC` after boot asks for 100, 200 and 300, because
    /// `_ACTLOC` is still zero, and no such modules exist.
    pub(crate) fn mark_gone(&mut self, module: u32) {
        for slot in self.slots.iter_mut() {
            if *slot == Some(module) {
                *slot = None;
            }
        }
    }

    /// The resident modules in slot order — the set `=>PUTAS` writes.
    pub fn resident(&self) -> Vec<u32> {
        self.slots.iter().flatten().copied().collect()
    }

    /// A sprite to **draw**, which is also what may decide the palette.
    ///
    /// The first sprite drawn brings its own 256 colors with it, as it does in
    /// the original — every sprite carries a palette and the first one on the
    /// screen sets it. That belongs to drawing and not to loading, which is why
    /// [`Engine::load_sprite`] is separate: measuring a descriptor has to be
    /// able to reach the picture's size without changing what the screen looks
    /// like, and the incremental drawer measures constantly — every mark on the
    /// damage map needs a rectangle.
    pub(crate) fn sprite(&mut self, id: u32) -> Option<Sprite> {
        let fresh = !self.sprites.contains_key(&id);
        let sprite = self.load_sprite(id)?;
        if fresh && self.display.palette.raw.iter().all(|&v| v == 0) {
            self.display.palette = sprite.palette.clone();
        }
        Some(sprite)
    }

    /// A sprite, without the palette that comes with drawing one.
    pub(crate) fn load_sprite(&mut self, id: u32) -> Option<Sprite> {
        if let Some(s) = self.sprites.get(&id) {
            return Some(s.clone());
        }
        let sprite = self.resources.as_ref()?.sprite(id)?;
        self.sprites.insert(id, sprite.clone());
        Some(sprite)
    }

    /// A text table by resource id.
    ///
    /// `SDTB` names the table and `SDTXT` the entry in it — established by
    /// following the intro: it sets table 6, entry 109, and entry 109 of text
    /// resource 6 is the backstory paragraph the intro shows.
    pub(crate) fn text_table(&mut self, id: i32) -> Option<&TextTable> {
        if !self.texts.contains_key(&id) {
            let table = self.resources.as_ref()?.text(id)?;
            self.texts.insert(id, table);
        }
        self.texts.get(&id)
    }

    /// The string a text descriptor stands for, or `None` where it names none.
    ///
    /// Text numbers are one-based, so `SDTXT n` is the table's entry n - 1.
    /// Asked of the original engine rather than worked out: with table 6
    /// selected, `109 SDTXT GDTXTLEN` comes back as 79 and `110 SDTXT` as 380 —
    /// the lengths of entries 108 and 109, not 109 and 110.
    ///
    /// Worth stating the trap: entry 109 happens to hold intro-looking
    /// text, so an off-by-one reads like a confirmation and is not one. The
    /// tell is the following text — off by one, it comes out as dialogue
    /// from the next scene.
    ///
    /// `None` and `Some("")` are different answers and both occur. A descriptor
    /// naming no table has no text at all; one naming an empty entry has a text
    /// that happens to be empty, and the original still measures it and still
    /// places it — it only declines to draw anything.
    pub fn descriptor_text(&mut self, d: &Descriptor) -> Option<String> {
        let (table, entry) = (d.table?, d.text?);
        let index = entry.checked_sub(1).filter(|i| *i >= 0)?;
        self.text_table(table)?.strings.get(index as usize).cloned()
    }

    pub(crate) fn load_palette(&mut self, id: i32) -> Option<motionvm_formats::Palette> {
        if let Some(p) = self.palettes.get(&id) {
            return Some(p.clone());
        }
        let p = self.resources.as_ref()?.palette(id)?;
        self.palettes.insert(id, p.clone());
        Some(p)
    }

    pub(crate) fn load_font(&mut self, number: i32) -> Option<i32> {
        let font = self.resources.as_ref()?.font(number)?;
        let handle = self.next_font;
        self.next_font += 1;
        self.fonts.insert(handle, font);
        Some(handle)
    }

    /// `STARTTUNE`'s body: fetch the block, hand it over, answer a handle.
    ///
    /// The original formats the tune number as `%03d.blk` (`0xD5908`) and opens
    /// it through the resource layer, which is the same BLOCK segment `GET`
    /// reads — songs are block resources that the sound code sees as virtual
    /// files. A tune that is not there fails silently and answers 0, which is
    /// what the missing-block arm does here.
    pub(crate) fn start_tune(&mut self, tune: i32, looping: i32) -> i32 {
        if self.music.is_none() {
            return 0;
        }
        let Some(song) = self.resources.as_ref().and_then(|r| r.block(tune)) else {
            return 0;
        };
        let handle = self.next_tune;
        self.next_tune += 1;
        if let Some(music) = self.music.as_mut() {
            music.start(handle, tune, looping != 0, &song);
        }
        handle
    }
}

/// An absolute, symlink-free path for a directory that need not exist yet.
///
/// `canonicalize` is the only thing that resolves symlinks, and it refuses a
/// path that is not there — which is no use for a save directory that is about
/// to be created. So the deepest part that does exist is canonicalized and the
/// rest appended. That is enough for the one question asked of it: a `..` or a
/// symlink can only appear in the part that exists, because the part that does
/// not is a name nobody has created yet.
pub(crate) fn resolve(dir: &std::path::Path) -> std::result::Result<std::path::PathBuf, String> {
    let absolute = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("no working directory: {e}"))?
            .join(dir)
    };
    let mut here = absolute.as_path();
    let mut rest = Vec::new();
    while !here.exists() {
        match (here.file_name(), here.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name.to_owned());
                here = parent;
            }
            _ => break,
        }
    }
    let mut out = here
        .canonicalize()
        .map_err(|e| format!("{}: {e}", here.display()))?;
    out.extend(rest.into_iter().rev());
    Ok(out)
}
