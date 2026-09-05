//! The four commands over a 16-bit game — its `DATA.-n-` container, on one
//! volume or two.
//!
//! What comes out mirrors the 32-bit commands where the data allows: sprites
//! as indexed PNGs (through a palette of the caller's choice, because a
//! 16-bit sprite carries none of its own), palettes, fonts, texts, blocks,
//! and scripts both raw and as a `.f` listing read through the kernel table of
//! the engine binary beside the container, with a `kernel-usage.txt` that marks
//! which of the kernel words the 16-bit machine implements.

use motionvm_motion_formats::m16::{
    Container, Segment, disasm::Disassembler, font, gfx, mz, psm, scr, text,
};
use motionvm_motion_formats::{Binding, font::FontRefTable};
use motionvm_render::Palette;
use std::io::Write;
use std::path::Path;

type Res = Result<(), Box<dyn std::error::Error>>;

/// The 16-bit engine binaries a game directory may hold, in probe order.
///
/// A second list of the same four names as the engine crate's, on purpose:
/// this tool reads a game's files without opening the game, and does not depend
/// on the engine.
const ENGINES: [&str; 4] = ["ENVIRO.EXE", "HPPLAY.EXE", "BMZ.EXE", "LL.EXE"];

/// The kernel of the game in `dir`: its engine binary read and its tables
/// bound. The binary is read, never run — the word table is what is wanted.
fn binding(dir: &Path) -> Result<Binding, Box<dyn std::error::Error>> {
    let exe = ENGINES
        .iter()
        .find_map(|name| motionvm_motion_formats::find_ci(dir, name))
        .ok_or_else(|| format!("{}: no {}", dir.display(), ENGINES.join(" and no ")))?;
    let img = mz::Image::open(exe)?;
    Ok(mz::binding_of(&img, &mz::kernel_words(&img))?)
}

/// The palette a sprite is written through when the caller names none.
///
/// `RUN` installs palette 0 before anything is drawn, so 0 is what the first
/// picture is seen through; rooms install their own, and `--pal` selects one
/// of those.
pub(crate) const DEFAULT_PALETTE: usize = 0;

pub(crate) fn info(dir: &Path) -> Res {
    let c = Container::open_dir(dir)?;
    let boot = c.boot();
    println!(
        "{:>10}  boot: module {} word {}; {} volume(s), {} spare offset entries",
        "DATA.-1-",
        boot.module,
        boot.word,
        c.volumes(),
        c.spare_offsets()
    );
    for &seg in &Segment::ALL {
        let ids = c.present(seg);
        let bytes: usize = ids
            .iter()
            .filter_map(|&id| c.item(seg, id).ok().flatten())
            .map(<[u8]>::len)
            .sum();
        println!(
            "{:>10}  {:>5} of {:>5} slots occupied, {bytes:>9} bytes{}",
            seg.name(),
            ids.len(),
            c.slot_count(seg),
            if c.packed(seg) { ", packed" } else { "" }
        );
    }
    let songs = c
        .present(Segment::Blk)
        .into_iter()
        .filter(|&id| {
            c.item(Segment::Blk, id)
                .ok()
                .flatten()
                .is_some_and(psm::is_song)
        })
        .count();
    println!("{:>10}  {songs} of the blocks are PSM 2 songs", "");
    let mismatches = c.occupancy_mismatches();
    if !mismatches.is_empty() {
        println!(
            "{:>10}  {} slots whose occupancy word disagrees with the offsets",
            "",
            mismatches.len()
        );
    }
    if c.trailing_slack() > 0 {
        println!(
            "{:>10}  {} bytes of unindexed trailing data",
            "",
            c.trailing_slack()
        );
    }
    Ok(())
}

/// The palette `id` of the container, as 8-bit RGB, or an error naming it.
fn palette(c: &Container, id: usize) -> Result<Palette, Box<dyn std::error::Error>> {
    let item = c
        .item(Segment::Pal, id)?
        .ok_or_else(|| format!("no palette {id} in the container"))?;
    if item.len() != Palette::BYTES {
        return Err(format!(
            "palette {id} is {} bytes, not {}",
            item.len(),
            Palette::BYTES
        )
        .into());
    }
    Ok(Palette::from_6bit(item))
}

/// Runs `each` over every item of `segment` that the index claims *and* the
/// file really holds, and answers `(written, over-claimed)`.
///
/// The same shape as the 32-bit command's helper, and for the same reason: a
/// tool that asserts its way past a damaged container aborts naming nothing
/// the person holding it can act on. Where the two differ is what they can
/// meet. `Container::present` and `Container::item` decide emptiness with one
/// predicate — the span's own length — so on this generation the second count
/// is zero for any file that opened at all, and the occupancy word, which
/// *can* disagree with the offsets, is reported separately by `info`. The
/// count is carried anyway rather than asserted away: it costs a `usize`, and
/// which of two functions in another crate share a predicate is not something
/// this tool should be built on.
fn each_item(
    c: &Container,
    segment: Segment,
    mut each: impl FnMut(usize, &[u8]) -> Res,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let (mut written, mut over) = (0, 0);
    for id in c.present(segment) {
        match c.item(segment, id)? {
            Some(item) => {
                each(id, item)?;
                written += 1;
            }
            None => over += 1,
        }
    }
    Ok((written, over))
}

pub(crate) fn extract(dir: &Path, out: &Path, pal: usize) -> Res {
    let c = Container::open_dir(dir)?;
    let palette = palette(&c, pal)?;
    let rgb = palette.to_rgb8();

    // Sprites, every one through the same palette.
    let sprite_dir = out.join("sprites");
    std::fs::create_dir_all(&sprite_dir)?;
    let (n_sprites, over) = each_item(&c, Segment::Gfx, |id, item| {
        let s = gfx::Sprite::parse(item)?;
        motionvm_render::write_indexed_png(
            &sprite_dir.join(format!("{id:04}.png")),
            u32::from(s.width),
            u32::from(s.height),
            &s.pixels,
            rgb.clone(),
            Some(motionvm_render::TRANSPARENT),
        )
        .map_err(Into::into)
    })?;
    println!(
        "{n_sprites:>6} sprites (palette {pal}) -> {}{}",
        sprite_dir.display(),
        crate::over_claimed(over)
    );

    // Palettes: raw 6-bit bytes plus a swatch grid.
    let pal_dir = out.join("palettes");
    std::fs::create_dir_all(&pal_dir)?;
    let (n_pal, over) = each_item(&c, Segment::Pal, |id, item| {
        std::fs::write(pal_dir.join(format!("{id:03}.pal")), item)?;
        crate::write_palette_png(
            &pal_dir.join(format!("{id:03}.png")),
            &Palette::from_6bit(item),
        )
    })?;
    println!(
        "{n_pal:>6} palettes -> {}{}",
        pal_dir.display(),
        crate::over_claimed(over)
    );

    // Text tables as JSON.
    let text_dir = out.join("text");
    std::fs::create_dir_all(&text_dir)?;
    let mut n_strings = 0;
    let (n_text, over) = each_item(&c, Segment::Txt, |id, item| {
        let table = text::parse(item)?;
        n_strings += table.strings.len();
        crate::write_text_json(&text_dir.join(format!("{id:03}.json")), &table)
    })?;
    println!(
        "{n_text:>6} text tables ({n_strings} strings) -> {}{}",
        text_dir.display(),
        crate::over_claimed(over)
    );

    // Blocks, written out unchanged, with an index that says which are songs.
    let blk_dir = out.join("blocks");
    std::fs::create_dir_all(&blk_dir)?;
    let mut index = std::io::BufWriter::new(std::fs::File::create(blk_dir.join("index.txt"))?);
    writeln!(index, "{:<6} {:>8}  CONTENT", "BLOCK", "BYTES")?;
    let mut n_songs = 0;
    let (n_blk, over) = each_item(&c, Segment::Blk, |id, item| {
        std::fs::write(blk_dir.join(format!("{id:04}.blk")), item)?;
        let what = match psm::tags(item) {
            Some(tags) => {
                n_songs += 1;
                format!(
                    "PSM 2 song (MDH at {}, SM8 at {})",
                    tags.mdh.map_or_else(|| "-".into(), |o| o.to_string()),
                    tags.sm8.map_or_else(|| "-".into(), |o| o.to_string())
                )
            }
            // The earlier games store the Ad Lib section on its own, with no
            // module around it and no sample sections to point at.
            None if psm::is_song(item) => {
                n_songs += 1;
                "PSM 2 song (a bare PLX section)".into()
            }
            None => String::new(),
        };
        Ok(writeln!(index, "{id:<6} {:>8}  {what}", item.len())?)
    })?;
    println!(
        "{n_blk:>6} blocks ({n_songs} PSM 2 songs) -> {}{}",
        blk_dir.display(),
        crate::over_claimed(over)
    );

    // Fonts: raw file, a contact sheet, the metrics; the one FRT maps them.
    let font_dir = out.join("fonts");
    std::fs::create_dir_all(&font_dir)?;
    let refs = c
        .item(Segment::Frt, 0)?
        .and_then(|d| FontRefTable::parse(d).ok());
    let mut n_glyphs = 0;
    let (n_font, over) = each_item(&c, Segment::Fnt, |id, item| {
        std::fs::write(font_dir.join(format!("{id:03}.fnt")), item)?;
        let f = font::parse(item)?;
        n_glyphs += f.glyphs.len();
        crate::write_font_sheet(&font_dir.join(format!("{id:03}.png")), &f)?;
        crate::write_font_json(&font_dir.join(format!("{id:03}.json")), &f, refs.as_ref())
    })?;
    println!(
        "{n_font:>6} fonts ({n_glyphs} glyphs) -> {}{}",
        font_dir.display(),
        crate::over_claimed(over)
    );

    // Scripts: the raw module, a symbol table, and a listing of every word
    // read through the kernel table.
    let binding = binding(dir)?;
    let scr_dir = out.join("scripts");
    std::fs::create_dir_all(&scr_dir)?;
    let mut symbols = std::io::BufWriter::new(std::fs::File::create(scr_dir.join("modules.txt"))?);
    let mut modules = Vec::new();
    let (_, over) = each_item(&c, Segment::Scr, |number, item| {
        std::fs::write(scr_dir.join(format!("{number:03}.scr")), item)?;
        let m = scr::ScrModule::parse(item)?;
        write_symbols(&mut symbols, &m, item.len())?;
        modules.push(m);
        Ok(())
    })?;
    // Ids are reused across modules that are never resident together, so
    // the names a listing resolves calls to are those of the library — the
    // modules RUN keeps — plus, for a location's three modules, each other's.
    let mut library = Disassembler::new(&binding);
    for m in modules.iter().filter(|m| !is_location(m.module)) {
        library.learn(m);
    }
    for m in &modules {
        let mut dis = library.clone();
        if is_location(m.module) {
            let n = m.module % 100;
            for sibling in modules
                .iter()
                .filter(|s| is_location(s.module) && s.module % 100 == n)
            {
                dis.learn(sibling);
            }
        }
        std::fs::write(scr_dir.join(format!("{:03}.f", m.module)), dis.module(m))?;
    }
    println!(
        "{:>6} scripts  -> {}{}",
        modules.len(),
        scr_dir.display(),
        crate::over_claimed(over)
    );

    // Which kernel words the game reaches for, and which the 16-bit machine
    // implements itself; the rest are the engine's, and whether the engine has
    // them is the engine's own suite's to say — these tools know no engine.
    let usage = library.usage(&modules);
    let mut f = std::io::BufWriter::new(std::fs::File::create(out.join("kernel-usage.txt"))?);
    writeln!(f, "{:<16} {:>8}  VM", "WORD", "USES")?;
    let mut done = 0;
    let mut done_uses = 0;
    let mut total_uses = 0;
    for (name, n) in &usage {
        let have = motionvm_motion_forth::m16::IMPLEMENTED.contains(&name.as_str());
        done += usize::from(have);
        total_uses += n;
        if have {
            done_uses += n;
        }
        writeln!(f, "{name:<16} {n:>8}  {}", if have { "yes" } else { "" })?;
    }
    println!(
        "{:>6} of {} kernel words used; {done} implemented by the machine, covering {:.0}% of all uses -> {}",
        usage.len(),
        binding.len(),
        crate::percent(done_uses, total_uses),
        out.join("kernel-usage.txt").display()
    );

    Ok(())
}

/// Whether a module number belongs to a location rather than to the resident
/// library, which starts at 600.
///
/// The later games give a location three modules — 100+N, 300+N, 500+N — and
/// the bound is 20, the most any of them has: Hilfe für Amajambere numbers to
/// 120/320/520, Die Enviro-Kids greifen ein to 117 and Jeff Jet to 113.
/// Victor Loomes gives a location two, 100+N and **20+N**, and numbers to 13,
/// so its macro modules run 21 to 33 — under every window the later games
/// need. Too low a bound is silent rather than loud — a location module
/// counted as library teaches its ids to every other listing, and the names
/// come out wrong with nothing said.
fn is_location(module: u16) -> bool {
    matches!(module, 21..=40 | 101..=120 | 301..=320 | 501..=520)
}

pub(crate) fn one_sprite(dir: &Path, id: usize, out: &Path, pal: usize) -> Res {
    let c = Container::open_dir(dir)?;
    let item = c.item(Segment::Gfx, id)?.ok_or("no such sprite")?;
    let s = gfx::Sprite::parse(item)?;
    println!(
        "sprite {id}: {}x{} ({} px), through palette {pal}",
        s.width,
        s.height,
        s.pixels.len()
    );
    motionvm_render::write_indexed_png(
        out,
        u32::from(s.width),
        u32::from(s.height),
        &s.pixels,
        palette(&c, pal)?.to_rgb8(),
        Some(motionvm_render::TRANSPARENT),
    )?;
    println!("wrote {}", out.display());
    Ok(())
}

pub(crate) fn script(dir: &Path, number: usize) -> Res {
    let c = Container::open_dir(dir)?;
    let item = c
        .item(Segment::Scr, number)?
        .ok_or("no such script module")?;
    let m = scr::ScrModule::parse(item)?;
    let binding = binding(dir)?;
    let mut dis = Disassembler::new(&binding);
    // Names from the library and, for a location module, from its siblings.
    for other in c.present(Segment::Scr) {
        let same_location = u16::try_from(other).is_ok_and(is_location)
            && is_location(m.module)
            && other % 100 == usize::from(m.module) % 100;
        if (!u16::try_from(other).is_ok_and(is_location) || same_location)
            && let Some(raw) = c.item(Segment::Scr, other)?
            && let Ok(parsed) = scr::ScrModule::parse(raw)
        {
            dis.learn(&parsed);
        }
    }
    let mut stdout = std::io::stdout().lock();
    write_symbols(&mut stdout, &m, item.len())?;
    write!(stdout, "{}", dis.module(&m))?;
    Ok(())
}

/// One module's header and symbol table: every word with its id, its kind and
/// its length in cells.
fn write_symbols(f: &mut impl Write, m: &scr::ScrModule, bytes: usize) -> std::io::Result<()> {
    writeln!(
        f,
        "\\ module {}  ids {}-{}  {} words  {bytes} bytes",
        m.module,
        m.first_id,
        m.last_id,
        m.entries.len()
    )?;
    for e in &m.entries {
        let kind = if e.is_variable() {
            "VAR"
        } else if e.is_constant() {
            "CONST"
        } else {
            ":"
        };
        let value = match kind {
            "VAR" | "CONST" => e.body.get(1).map_or(String::new(), |v| {
                format!("  = {}", i16::from_le_bytes(v.to_le_bytes()))
            }),
            _ => String::new(),
        };
        writeln!(
            f,
            "  {:>5}  {:<5} {:<12} {:>5} cells{value}",
            e.id,
            kind,
            e.name,
            e.body.len()
        )?;
    }
    writeln!(f)
}

#[cfg(test)]
mod tests {

    /// The three module series a location owns, against the resident library.
    ///
    /// The bound matters and is silent when it is wrong: a location module
    /// counted as library teaches its ids to every other listing, and the
    /// names come out wrong with nothing said. Twenty is the most locations
    /// any of the games has.
    #[test]
    fn a_locations_modules_are_told_from_the_library() {
        for n in 1..=20 {
            for base in [100, 300, 500] {
                assert!(is_location(base + n), "module {}", base + n);
            }
        }
        // Victor Loomes' second module per location, which no later game has.
        for n in 1..=13 {
            assert!(is_location(20 + n), "module {}", 20 + n);
        }
        for module in [100, 300, 500, 20, 41, 121, 321, 521, 600, 607, 650, 651] {
            assert!(!is_location(module), "module {module} is not a location's");
        }
    }
    use super::*;

    /// A one-volume `DATA.-n-` over three GFX slots: an item, an empty slot,
    /// an item — the shape the container reader's own tests use.
    fn minimal_dat() -> Vec<u8> {
        let mut v = vec![0u8; 0x26];
        v[0..2].copy_from_slice(&100u16.to_le_bytes()); // boot module
        v[2..4].copy_from_slice(&401u16.to_le_bytes()); // boot word
        v[4..6].copy_from_slice(&3u16.to_le_bytes()); // three GFX slots
        v[18..20].copy_from_slice(&1u16.to_le_bytes()); // one volume
        v.extend_from_slice(&[1u16, 0, 1].map(u16::to_le_bytes).concat());
        let first = u32::try_from(v.len() + 3 * 4).unwrap();
        let items: [&[u8]; 2] = [&[1, 0, 1, 0, 0, 0], &[2, 0, 1, 0, 0, 0, 7, 8]];
        let second = first + u32::try_from(items[0].len()).unwrap();
        v.extend_from_slice(&[first, second, second].map(u32::to_le_bytes).concat());
        v.extend_from_slice(items[0]);
        v.extend_from_slice(items[1]);
        v
    }

    #[test]
    fn every_slot_the_index_claims_is_handed_over() {
        let c = Container::from_bytes(minimal_dat(), "t".into()).expect("the container opens");
        assert_eq!(c.present(Segment::Gfx), [0, 2], "slot 1 is empty");

        let mut seen = Vec::new();
        let (written, over) = each_item(&c, Segment::Gfx, |id, item| {
            seen.push((id, item.len()));
            Ok(())
        })
        .expect("no hard error");
        assert_eq!(seen, [(0, 6), (2, 8)]);
        assert_eq!(
            (written, over),
            (2, 0),
            "this generation's present and item share a predicate"
        );
    }

    #[test]
    fn a_slot_flagged_for_a_volume_that_is_not_there_is_not_walked() {
        // The occupancy word can disagree with the offsets — Jeff Jet's
        // shipped container has two such slots — and `info` reports the
        // count. What `extract` walks is the offsets, so a flagged slot with
        // no bytes is simply not among them: what has bytes is what is there.
        let mut bytes = minimal_dat();
        bytes[0x26 + 2..0x26 + 4].copy_from_slice(&1u16.to_le_bytes());
        let c = Container::from_bytes(bytes, "t".into()).expect("the container opens");
        assert_eq!(c.occupancy_mismatches(), [1]);

        let mut ids = Vec::new();
        let (written, over) = each_item(&c, Segment::Gfx, |id, _| {
            ids.push(id);
            Ok(())
        })
        .expect("no hard error");
        assert_eq!(ids, [0, 2]);
        assert_eq!((written, over), (2, 0));
    }

    #[test]
    fn an_error_from_the_body_is_not_swallowed() {
        // A reader that fails on bytes that are really there still stops the
        // command; the second count is for what the file does not hold.
        let c = Container::from_bytes(minimal_dat(), "t".into()).expect("the container opens");
        let e = each_item(&c, Segment::Gfx, |_, _| Err("the reader said no".into()))
            .expect_err("the body's error reaches the caller");
        assert_eq!(e.to_string(), "the reader said no");
    }
}
