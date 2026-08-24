//! The four commands over a 16-bit game — one `DATA.-n-` container, as Die
//! Enviro-Kids greifen ein ships it.
//!
//! What comes out mirrors the 32-bit commands where the data allows: sprites
//! as indexed PNGs (through a palette of the caller's choice, because a
//! 16-bit sprite carries none of its own), palettes, fonts, texts, blocks,
//! and scripts both raw and as a `.f` listing read through the kernel table
//! of `ENVIRO.EXE`, with a `kernel-usage.txt` that marks which of the kernel
//! words the 16-bit machine implements.

use motionvm_formats::m16::{
    Container, Segment, disasm::Disassembler, font, gfx, mz, psm, scr, text,
};
use motionvm_formats::{Binding, Palette, font::FontRefTable};
use std::io::Write;
use std::path::Path;

type Res = Result<(), Box<dyn std::error::Error>>;

/// The kernel of the game in `dir`: `ENVIRO.EXE` read and its tables bound.
fn binding(dir: &Path) -> Result<Binding, Box<dyn std::error::Error>> {
    let exe = motionvm_formats::find_ci(dir, "ENVIRO.EXE")
        .ok_or_else(|| format!("{}: no ENVIRO.EXE", dir.display()))?;
    let img = mz::Image::open(exe)?;
    Ok(mz::binding_of(&mz::kernel_words(&img))?)
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
        "{:>10}  boot: module {} word {}; header words {:?}",
        "DATA.-1-",
        boot.module,
        boot.word,
        c.open_fields()
    );
    for &seg in &Segment::ALL {
        let ids = c.present(seg);
        let bytes: usize = ids
            .iter()
            .filter_map(|&id| c.item(seg, id).ok().flatten())
            .map(<[u8]>::len)
            .sum();
        println!(
            "{:>10}  {:>5} of {:>5} slots occupied, {bytes:>9} bytes",
            seg.name(),
            ids.len(),
            c.slot_count(seg)
        );
    }
    let songs = c
        .present(Segment::Blk)
        .into_iter()
        .filter(|&id| {
            c.item(Segment::Blk, id)
                .ok()
                .flatten()
                .is_some_and(psm::is_module)
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

pub(crate) fn extract(dir: &Path, out: &Path, pal: usize) -> Res {
    let c = Container::open_dir(dir)?;
    let palette = palette(&c, pal)?;
    let rgb = palette.to_rgb8();

    // Sprites, every one through the same palette.
    let sprite_dir = out.join("sprites");
    std::fs::create_dir_all(&sprite_dir)?;
    let mut n_sprites = 0;
    for id in c.present(Segment::Gfx) {
        let item = c.item(Segment::Gfx, id)?.expect("present");
        let s = gfx::Sprite::parse(item)?;
        motionvm_render::write_indexed_png(
            &sprite_dir.join(format!("{id:04}.png")),
            s.width as u32,
            s.height as u32,
            &s.pixels,
            rgb.clone(),
            Some(motionvm_render::TRANSPARENT),
        )?;
        n_sprites += 1;
    }
    println!(
        "{n_sprites:>6} sprites (palette {pal}) -> {}",
        sprite_dir.display()
    );

    // Palettes: raw 6-bit bytes plus a swatch grid.
    let pal_dir = out.join("palettes");
    std::fs::create_dir_all(&pal_dir)?;
    let mut n_pal = 0;
    for id in c.present(Segment::Pal) {
        let item = c.item(Segment::Pal, id)?.expect("present");
        std::fs::write(pal_dir.join(format!("{id:03}.pal")), item)?;
        crate::write_palette_png(
            &pal_dir.join(format!("{id:03}.png")),
            &Palette::from_6bit(item),
        )?;
        n_pal += 1;
    }
    println!("{n_pal:>6} palettes -> {}", pal_dir.display());

    // Text tables as JSON.
    let text_dir = out.join("text");
    std::fs::create_dir_all(&text_dir)?;
    let mut n_text = 0;
    let mut n_strings = 0;
    for id in c.present(Segment::Txt) {
        let item = c.item(Segment::Txt, id)?.expect("present");
        let table = text::parse(item)?;
        n_strings += table.strings.len();
        crate::write_text_json(&text_dir.join(format!("{id:03}.json")), &table)?;
        n_text += 1;
    }
    println!(
        "{n_text:>6} text tables ({n_strings} strings) -> {}",
        text_dir.display()
    );

    // Blocks, written out unchanged, with an index that says which are songs.
    let blk_dir = out.join("blocks");
    std::fs::create_dir_all(&blk_dir)?;
    let mut index = std::io::BufWriter::new(std::fs::File::create(blk_dir.join("index.txt"))?);
    writeln!(index, "{:<6} {:>8}  CONTENT", "BLOCK", "BYTES")?;
    let mut n_blk = 0;
    let mut n_songs = 0;
    for id in c.present(Segment::Blk) {
        let item = c.item(Segment::Blk, id)?.expect("present");
        std::fs::write(blk_dir.join(format!("{id:04}.blk")), item)?;
        let what = match psm::tags(item) {
            Some(tags) => {
                n_songs += 1;
                format!(
                    "PSM 2 song (MDH at {}, SM8 at {})",
                    tags.mdh.map_or("-".into(), |o| o.to_string()),
                    tags.sm8.map_or("-".into(), |o| o.to_string())
                )
            }
            None => String::new(),
        };
        writeln!(index, "{id:<6} {:>8}  {what}", item.len())?;
        n_blk += 1;
    }
    println!(
        "{n_blk:>6} blocks ({n_songs} PSM 2 songs) -> {}",
        blk_dir.display()
    );

    // Fonts: raw file, a contact sheet, the metrics; the one FRT maps them.
    let font_dir = out.join("fonts");
    std::fs::create_dir_all(&font_dir)?;
    let refs = c
        .item(Segment::Frt, 0)?
        .and_then(|d| FontRefTable::parse(d).ok());
    let mut n_font = 0;
    let mut n_glyphs = 0;
    for id in c.present(Segment::Fnt) {
        let item = c.item(Segment::Fnt, id)?.expect("present");
        std::fs::write(font_dir.join(format!("{id:03}.fnt")), item)?;
        let f = font::parse(item)?;
        n_glyphs += f.glyphs.len();
        crate::write_font_sheet(&font_dir.join(format!("{id:03}.png")), &f)?;
        crate::write_font_json(&font_dir.join(format!("{id:03}.json")), &f, refs.as_ref())?;
        n_font += 1;
    }
    println!(
        "{n_font:>6} fonts ({n_glyphs} glyphs) -> {}",
        font_dir.display()
    );

    // Scripts: the raw module, a symbol table, and a listing of every word
    // read through the kernel table.
    let binding = binding(dir)?;
    let scr_dir = out.join("scripts");
    std::fs::create_dir_all(&scr_dir)?;
    let mut symbols = std::io::BufWriter::new(std::fs::File::create(scr_dir.join("modules.txt"))?);
    let mut modules = Vec::new();
    for number in c.present(Segment::Scr) {
        let item = c.item(Segment::Scr, number)?.expect("present");
        std::fs::write(scr_dir.join(format!("{number:03}.scr")), item)?;
        let m = scr::ScrModule::parse(item)?;
        write_symbols(&mut symbols, &m, item.len())?;
        modules.push(m);
    }
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
    println!("{:>6} scripts  -> {}", modules.len(), scr_dir.display());

    // Which kernel words the game reaches for, and which the 16-bit machine
    // implements itself; the rest are the engine's.
    let usage = library.usage(&modules);
    let mut f = std::io::BufWriter::new(std::fs::File::create(out.join("kernel-usage.txt"))?);
    writeln!(f, "{:<16} {:>8}  VM", "WORD", "USES")?;
    let mut done = 0;
    let mut done_uses = 0;
    let mut total_uses = 0;
    for (name, n) in &usage {
        let have = motionvm_forth::m16::IMPLEMENTED.contains(&name.as_str());
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
        100.0 * done_uses as f64 / total_uses.max(1) as f64,
        out.join("kernel-usage.txt").display()
    );

    Ok(())
}

/// Whether a module number is one of a location's three — 100+N, 300+N,
/// 500+N for N in 1..=17 — as opposed to the resident library.
fn is_location(module: u16) -> bool {
    matches!(module, 101..=117 | 301..=317 | 501..=517)
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
        s.width as u32,
        s.height as u32,
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
        let same_location = is_location(other as u16)
            && is_location(m.module)
            && other % 100 == m.module as usize % 100;
        if (!is_location(other as u16) || same_location)
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
            "VAR" | "CONST" => e
                .body
                .get(1)
                .map_or(String::new(), |v| format!("  = {}", *v as i16)),
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
