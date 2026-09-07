//! Where the pictures, fonts, texts and songs come from.
//!
//! Everything the game draws with is a numbered item in a resource container,
//! loaded on first use and kept. Which container depends on the engine
//! generation — the 32-bit engine's game ships `NNN.RSC` banks, the 16-bit
//! engine's one `DATA.-1-` — and [`Resources`] is the engine's one handle on
//! either: a reader reads one layout, and which reader answers is decided
//! here, once, by which container the game directory held.
//!
//! The module-slot bookkeeping lives here too. `=>GET` loads a module into
//! the machine and takes it a slot, `=>ERASE` gives both back, and the
//! *order* of the slot table is what a savegame's module images are written
//! in.

use crate::{Descriptor, Engine};
use motionvm_motion_formats::TextTable;
use motionvm_motion_formats::font::{Font, FontRefTable};
use motionvm_motion_formats::m16::{Container, Segment};
use motionvm_motion_formats::m32::scr::ScrModule;
use motionvm_motion_formats::m32::{Kind, Sprite, rsc::Bank};
use motionvm_motion_forth::cell;
use motionvm_motion_forth::m32;
use motionvm_render::Palette;
use motionvm_render::Picture;

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
    /// A decoded sprite: the picture to blit, and the palette the file carried
    /// where its format has one.
    ///
    /// The 32-bit container gives every sprite a full 256-color table, which is
    /// how the first picture on a screen sets the palette. The 16-bit one gives
    /// none, and needs none: that game's `RUN` installs a palette with `SETPAL`
    /// before anything is drawn. So the palette is optional and the picture is
    /// not — which is also why the renderer takes the picture alone.
    pub(crate) fn sprite(&self, id: u32) -> Option<(Picture, Option<Palette>)> {
        match self {
            Resources::Motion32(bank) => {
                let item = bank.item(Kind::Gfx8, cell::index(id)).ok()??;
                let raw = Sprite::parse(item).ok()?;
                let picture = Picture {
                    width: raw.width,
                    height: raw.height,
                    pixels: raw.pixels,
                };
                Some((picture, Some(Palette::from_6bit(&raw.palette))))
            }
            Resources::Motion16(c) => {
                let item = c.item(Segment::Gfx, cell::index(id)).ok()??;
                let raw = motionvm_motion_formats::m16::gfx::Sprite::parse(item).ok()?;
                let picture = Picture {
                    width: raw.width,
                    height: raw.height,
                    pixels: raw.pixels,
                };
                Some((picture, None))
            }
        }
    }

    /// A text table by id.
    pub(crate) fn text(&self, id: i32) -> Option<TextTable> {
        let id = usize::try_from(id).ok()?;
        match self {
            Resources::Motion32(bank) => {
                let item = bank.item(Kind::Text, id).ok()??;
                motionvm_motion_formats::m32::text::parse(item).ok()
            }
            Resources::Motion16(c) => {
                let item = c.item(Segment::Txt, id).ok()??;
                motionvm_motion_formats::m16::text::parse(item).ok()
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
                motionvm_motion_formats::m32::font::parse(item).ok()
            }
            Resources::Motion16(c) => {
                let item = c.item(Segment::Fnt, id).ok()??;
                motionvm_motion_formats::m16::font::parse(item).ok()
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
            Resources::Motion32(bank) => bank.item(Kind::Script, cell::index(number)).ok()??,
            Resources::Motion16(c) => c.item(Segment::Scr, cell::index(number)).ok()??,
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
            Resources::Motion32(_) => motionvm_motion_formats::find_ci(dir, "000.FRT")
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|d| FontRefTable::parse(&d).ok()),
            Resources::Motion16(c) => {
                let item = c.item(Segment::Frt, 0).ok()??;
                FontRefTable::parse(item).ok()
            }
        }
    }

    /// Font reference table `n` of a 16-bit container — what `SFT n` installs.
    /// The 32-bit engine has no such word, and its one table is the loose
    /// `000.FRT`.
    pub(crate) fn font_ref_table(&self, n: u32) -> Option<FontRefTable> {
        match self {
            Resources::Motion32(_) => None,
            Resources::Motion16(c) => {
                let item = c.item(Segment::Frt, cell::index(n)).ok()??;
                FontRefTable::parse(item).ok()
            }
        }
    }

    /// The font a text draws in when nothing chose one: font 0, wherever
    /// the game keeps it. The 32-bit engine opens `000.fnt` by name through
    /// its resource layer (R78 `0x21b05`–`0x21b41`), which resolves the name
    /// to font 0 of whatever holds it (`0x44ad0`–`0x44b2b`) — item 0 of the
    /// engine's own container, `ENGINE.RSC`, in Checker 2000, and the loose
    /// file of that name in Dunkle Schatten 2, whose `001.RSC` carries the
    /// same bytes as item 0 besides. Which of the two the layer prefers when
    /// both exist is not observable in either game and is not read; the
    /// container is asked first here. For a 16-bit game font 0 of the
    /// container — the text face, which is what `0 SDFNT` at eight of its
    /// sites reads as. Read now, both ends: `SDFNT` (`05f1:1375`) stores a
    /// bare table index at `+0x2b`, and `NEWANIM` (`05f1:0021`) runs `0
    /// +FONT` first thing — the container's font 0 is the first entry of
    /// that table, the face a fresh descriptor's zero picks.
    pub(crate) fn system_font(&self, dir: &std::path::Path) -> Option<Font> {
        match self {
            Resources::Motion32(_) => self.font(0).or_else(|| {
                motionvm_motion_formats::find_ci(dir, "000.FNT")
                    .and_then(|p| std::fs::read(p).ok())
                    .and_then(|d| motionvm_motion_formats::m32::font::parse(&d).ok())
            }),
            Resources::Motion16(_) => self.font(0),
        }
    }

    /// The palette the 32-bit `TOGFX` installs on entering graphics, and the
    /// one the engine's arrow pointer takes its colors from: `000.pal`,
    /// opened by name right after `000.fnt` (R78 `0x21b4c`–`0x21b72`) and
    /// resolved the same way — palette 0 of `ENGINE.RSC` in Checker 2000,
    /// the loose file in Dunkle Schatten 2, whose `001.RSC` holds the same
    /// 768 bytes as palette 0 besides. The 16-bit `TOGFX` installs none.
    pub(crate) fn system_palette(&self, dir: &std::path::Path) -> Option<Palette> {
        match self {
            Resources::Motion32(_) => self.palette(0).or_else(|| {
                motionvm_motion_formats::find_ci(dir, "000.PAL")
                    .and_then(|p| std::fs::read(p).ok())
                    .filter(|d| d.len() == Palette::BYTES)
                    .map(|d| Palette::from_6bit(&d))
            }),
            Resources::Motion16(_) => None,
        }
    }
}

impl Engine {
    /// Attaches a 32-bit game's resources so sprites and fonts can be loaded.
    pub(crate) fn with_bank(self, dir: &std::path::Path, bank: Bank) -> Self {
        self.with_resources(dir, Resources::Motion32(bank))
    }

    /// Attaches a 16-bit game's resources.
    pub(crate) fn with_container(self, dir: &std::path::Path, container: Container) -> Self {
        self.with_resources(dir, Resources::Motion16(Box::new(container)))
    }

    fn with_resources(mut self, dir: &std::path::Path, resources: Resources) -> Self {
        self.dir = Some(dir.to_path_buf());
        self.scene.font_refs = resources.font_refs(dir);
        self.scene.system_font = resources.system_font(dir);
        self.scene.system_palette = resources.system_palette(dir);
        self.sample_dir = sample_dir(dir);
        self.resources = Some(resources);
        self
    }

    /// A speech file by the name a script passes to `->STARTSAMPLE`, out of
    /// the directory `SMPPATH` names — or `None` where there is no such
    /// directory or no such file, which is where the original asks for the
    /// CD forever.
    pub(crate) fn sample_file(&self, name: &str) -> Option<Vec<u8>> {
        let dir = self.sample_dir.as_ref()?;
        let path = motionvm_motion_formats::find_ci(dir, name)?;
        std::fs::read(path).ok()
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
        if self.persistence.slots.contains(&Some(module)) {
            return;
        }
        if let Some(slot) = self
            .persistence
            .slots
            .iter_mut()
            .skip(1)
            .find(|s| s.is_none())
        {
            *slot = Some(module);
        }
    }

    /// Loads module `n` out of the container into the machine and takes it a
    /// slot, as `=>GET` (0x64999) does — a fresh copy every time it is asked,
    /// which is what starts a location's variables over on every entry. A
    /// module the container does not hold is refused by name: the original
    /// opens `%03d.SCR` and stops when it cannot.
    pub(crate) fn get_module(
        &mut self,
        mem: &mut m32::Memory,
        n: u32,
    ) -> motionvm_motion_forth::Result<()> {
        let item = self
            .resources
            .as_ref()
            .and_then(|r| r.script(n))
            .ok_or_else(|| motionvm_motion_forth::Error::MissingResource {
                kind: "module",
                id: cell::signed(n),
                word: "=>GET",
                at: None,
            })?;
        let parsed = ScrModule::parse(&item)
            .map_err(|e| motionvm_motion_forth::Error::Unsupported(format!("=>GET {n}: {e}")))?;
        mem.insert(m32::Module::load(&item, &parsed));
        self.mark_resident(n);
        Ok(())
    }

    /// Frees a module's slot, as `=>ERASE` does; the machine gives the memory
    /// back beside it. Unknown modules are ignored — the first `INCLLOC` after
    /// boot erases 100, 200 and 300, because `_ACTLOC` is still zero, and no
    /// such modules exist.
    pub(crate) fn mark_gone(&mut self, module: u32) {
        for slot in self.persistence.slots.iter_mut() {
            if *slot == Some(module) {
                *slot = None;
            }
        }
    }

    /// The resident modules in slot order — the set `=>PUTAS` writes.
    pub fn resident(&self) -> Vec<u32> {
        self.persistence.slots.iter().flatten().copied().collect()
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
    pub(crate) fn sprite(&mut self, id: u32) -> Option<Picture> {
        if self.scene.sprites.contains_key(&id) {
            return self.load_sprite(id);
        }
        let (picture, palette) = self.resources.as_ref()?.sprite(id)?;
        if let Some(palette) = palette
            && self.display.palette.raw.iter().all(|&v| v == 0)
        {
            self.display.palette = palette;
        }
        self.scene.sprites.insert(id, picture.clone());
        Some(picture)
    }

    /// A sprite, without the palette that comes with drawing one.
    pub(crate) fn load_sprite(&mut self, id: u32) -> Option<Picture> {
        if let Some(s) = self.scene.sprites.get(&id) {
            return Some(s.clone());
        }
        let (picture, _) = self.resources.as_ref()?.sprite(id)?;
        self.scene.sprites.insert(id, picture.clone());
        Some(picture)
    }

    /// A text table by resource id.
    ///
    /// `SDTB` names the table and `SDTXT` the entry in it — established by
    /// following the intro: it sets table 6, entry 109, and entry 109 of text
    /// resource 6 is the backstory paragraph the intro shows.
    pub(crate) fn text_table(&mut self, id: i32) -> Option<&TextTable> {
        if !self.scene.texts.contains_key(&id) {
            let table = self.resources.as_ref()?.text(id)?;
            self.scene.texts.insert(id, table);
        }
        self.scene.texts.get(&id)
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
        let (table, entry) = (d.shows.table()?, d.text?);
        let index = entry.checked_sub(1).filter(|i| *i >= 0)?;
        let index = cell::at(index)?;
        self.text_table(table)?.strings.get(index).cloned()
    }

    pub(crate) fn load_palette(&mut self, id: i32) -> Option<Palette> {
        if let Some(p) = self.scene.palettes.get(&id) {
            return Some(p.clone());
        }
        let p = self.resources.as_ref()?.palette(id)?;
        self.scene.palettes.insert(id, p.clone());
        Some(p)
    }

    pub(crate) fn load_font(&mut self, number: i32) -> Option<i32> {
        let font = self.resources.as_ref()?.font(number)?;
        let handle = self.scene.next_font;
        self.scene.next_font += 1;
        self.scene.fonts.insert(handle, font);
        Some(handle)
    }

    /// `STARTTUNE`'s body: fetch the block, hand it over, answer a handle.
    ///
    /// The original formats the tune number as `%03d.blk` (`0xD5908`) and opens
    /// it through the resource layer, which is the same BLOCK segment `GET`
    /// reads — songs are block resources that the sound code sees as virtual
    /// files. The block goes over the [`crate::MusicSink`] whole, exactly as the
    /// original hands the file to its MIDI layer (`0x853B0`). A tune that is
    /// not there fails silently and answers 0 — as the original's words do
    /// when sound never initialized, which is what a missing sink reproduces.
    pub(crate) fn start_tune(&mut self, tune: i32, looping: i32) -> i32 {
        if self.sound.sink.is_none() {
            return 0;
        }
        let Some(song) = self.resources.as_ref().and_then(|r| r.block(tune)) else {
            return 0;
        };
        let handle = self.sound.next_handle;
        self.sound.next_handle += 1;
        if let Some(music) = self.sound.sink.as_mut() {
            music.start(handle, tune, looping != 0, &song);
        }
        self.sound.playing = true;
        handle
    }

    /// `PLAYSAMPLE`'s second half (`STERN.EXE` `15e5:03b8`): the block,
    /// spelled through the `#F0R3i.blk` template and loaded whole, copied
    /// into the digital driver's buffer and handed to the driver's play
    /// entry — here, over the [`crate::MusicSink`] whole, the way a tune
    /// goes. A block that is not there plays nothing, as a tune that is not
    /// there does; the sink is silent where no sound card is.
    pub(crate) fn play_sample(&mut self, block: i32) {
        let Some(sample) = self.resources.as_ref().and_then(|r| r.block(block)) else {
            return;
        };
        if let Some(music) = self.sound.sink.as_mut() {
            music.sample(block, &sample);
        }
    }
}

/// The directory the game's speech files are in, from the game's own files.
///
/// `SMPPATH` names it as the 1996 installation knew it — `C:\Checker\wavs`
/// — and `SYSTEM.RSC`, the boot script, names the installation's root the
/// same way in its first line, `" C:\checker\"`, before `3 ->RSCPATH`. The
/// directory a copy sits in stands for that root, so the sample directory is
/// what `SMPPATH` names under the root the boot script names, resolved under
/// the game directory: `WAVS`. A copy whose boot script names no root, or
/// whose `SMPPATH` lies outside it, keeps the last component of the path,
/// which is what an installer would have laid beside the game. Case is not
/// trusted on either side: the two shipped files spell the root `checker`
/// and `Checker`.
fn sample_dir(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    use motionvm_motion_formats::m32::smppath;
    let named = motionvm_motion_formats::find_ci(dir, "SMPPATH")
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|text| smppath::parse(&text))?;
    let mut path = smppath::components(&named);
    // The root the boot script names, if it names one: a quoted string at
    // the start of the file.
    let root = motionvm_motion_formats::find_ci(dir, "SYSTEM.RSC")
        .and_then(|p| std::fs::read(p).ok())
        .map(|text| motionvm_motion_formats::cp437_to_string(&text))
        .and_then(|text| {
            let quoted = text.trim_start().strip_prefix('"')?;
            let (inside, _) = quoted.split_once('"')?;
            Some(smppath::components(inside.trim()))
        })
        .unwrap_or_default();
    let under_root = !root.is_empty()
        && root.len() < path.len()
        && root
            .iter()
            .zip(path.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));
    if under_root {
        path.drain(..root.len());
    } else if let Some(last) = path.pop() {
        path = vec![last];
    }
    let mut out = dir.to_path_buf();
    for part in path {
        // Each step case-insensitively, so a lower-cased copy resolves.
        out = motionvm_motion_formats::find_ci(&out, &part).unwrap_or_else(|| out.join(part));
    }
    out.is_dir().then_some(out)
}

/// An absolute, symlink-free path for a directory that need not exist yet.
///
/// `canonicalize` is the only thing that resolves symlinks, and it refuses a
/// path that is not there — which is no use for a save directory that is about
/// to be created. So the deepest part that does exist is canonicalized and the
/// rest appended. That is enough for the one question asked of it: a `..` or a
/// symlink can only appear in the part that exists, because the part that does
/// not is a name nobody has created yet.
pub(crate) fn resolve(dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
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
