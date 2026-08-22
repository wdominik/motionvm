//! Where the pictures, fonts, texts and songs come from.
//!
//! Everything the game draws with is a numbered item in a resource container,
//! loaded on first use and kept. The engine holds one bank over all the
//! `NNN.RSC` files it found and answers by kind and id.
//!
//! The module-slot bookkeeping lives here too. `=>GET` and `=>ERASE` move no
//! memory in this rebuild — every module is resident from the start — but they
//! keep the slot table, and the *order* of that table is what a savegame's
//! module images are written in.

use crate::{Descriptor, Engine};
use motionvm_formats::font::{Font, FontRefTable};
use motionvm_formats::{Kind, Sprite, TextTable, rsc::Bank};

impl Engine {
    /// Attaches the game's resources so sprites and fonts can be loaded.
    pub(crate) fn with_resources(mut self, dir: &std::path::Path) -> Self {
        self.dir = Some(dir.to_path_buf());
        self.bank = Bank::open_dir(dir).ok();
        // Through `find_ci` rather than `join`: the shipped names are upper
        // case but a copied install is often not, and on a case-sensitive
        // filesystem an exact join silently finds nothing — which here would
        // mean a game that draws no text rather than one that says why.
        self.font_refs = motionvm_formats::find_ci(dir, "000.FRT")
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|d| FontRefTable::parse(&d).ok());
        self.system_font = motionvm_formats::find_ci(dir, "000.FNT")
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|d| Font::parse(&d).ok());
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
        let bank = self.bank.as_ref()?;
        let item = bank.item(Kind::Gfx8, id as usize).ok()??;
        let sprite = Sprite::parse(item).ok()?;
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
            let bank = self.bank.as_ref()?;
            let item = bank.item(Kind::Text, id as usize).ok()??;
            let table = TextTable::parse(item).ok()?;
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
    /// Worth stating why that was wrong before: entry 109 happened to hold
    /// intro-looking text, which read like a confirmation and was not one. The
    /// tell was that the following text came out as dialogue from the next
    /// scene.
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
        let item = self
            .bank
            .as_ref()?
            .item(Kind::Palette, id.max(0) as usize)
            .ok()??;
        let p = motionvm_formats::Palette::from_6bit(item);
        self.palettes.insert(id, p.clone());
        Some(p)
    }

    pub(crate) fn load_font(&mut self, number: i32) -> Option<i32> {
        let bank = self.bank.as_ref()?;
        let item = bank.item(Kind::Font, number.max(0) as usize).ok()??;
        let font = Font::parse(item).ok()?;
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
        let song = self
            .bank
            .as_ref()
            .and_then(|b| b.item(Kind::Block, tune.max(0) as usize).ok().flatten())
            .map(<[u8]>::to_vec);
        let Some(song) = song else { return 0 };
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
