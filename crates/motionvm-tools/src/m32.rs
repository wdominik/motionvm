//! The four commands over a 32-bit game — `NNN.RSC` containers beside
//! `ENGINE.EXE`, as Dunkle Schatten 2 ships them.

use motionvm_formats::m32::{Kind, ScrModule, Sprite, disasm::Disassembler, rsc::Bank};
use motionvm_formats::{Palette, font};
use std::io::Write;
use std::path::Path;

/// `ENGINE.EXE` in `dir`, whatever case it is spelled in.
///
/// A copied install often arrives lower-cased, and on a case-sensitive
/// filesystem an exact-case join reports the file as absent while it sits right
/// there. Every shipped file this workspace opens by name goes through
/// `find_ci` for that reason.
fn engine_exe(dir: &Path) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    motionvm_formats::find_ci(dir, "ENGINE.EXE")
        .ok_or_else(|| format!("{}: no ENGINE.EXE", dir.display()).into())
}

pub(crate) fn info(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let bank = Bank::open_dir(dir)?;
    for r in bank.banks() {
        let name = Path::new(r.source())
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let counts: Vec<String> = Kind::ALL
            .iter()
            .filter_map(|&k| {
                let n = r.present(k).len();
                (n > 0).then(|| format!("{} {n}", k.name()))
            })
            .collect();
        println!("{name:>10}  {}", counts.join(", "));
        if r.trailing_slack() > 0 {
            println!(
                "{:>10}  {} bytes of unindexed trailing data",
                "",
                r.trailing_slack()
            );
        }
    }
    println!();
    for &k in &Kind::ALL {
        let n = bank.present(k).len();
        if n > 0 {
            println!("total {:>8}: {n}", k.name());
        }
    }
    Ok(())
}

pub(crate) fn extract(dir: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let bank = Bank::open_dir(dir)?;

    // Sprites.
    let sprite_dir = out.join("sprites");
    std::fs::create_dir_all(&sprite_dir)?;
    let mut n_sprites = 0;
    for (_, id) in bank.present(Kind::Gfx8) {
        let item = bank.item(Kind::Gfx8, id)?.expect("present");
        let sprite = Sprite::parse(item)?;
        write_png(&sprite_dir.join(format!("{id:04}.png")), &sprite)?;
        n_sprites += 1;
    }
    println!("{n_sprites:>6} sprites  -> {}", sprite_dir.display());

    // Palettes: raw 6-bit bytes plus a readable swatch strip.
    let pal_dir = out.join("palettes");
    std::fs::create_dir_all(&pal_dir)?;
    let mut n_pal = 0;
    for (_, id) in bank.present(Kind::Palette) {
        let item = bank.item(Kind::Palette, id)?.expect("present");
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
    for (_, id) in bank.present(Kind::Text) {
        let item = bank.item(Kind::Text, id)?.expect("present");
        let table = motionvm_formats::m32::text::parse(item)?;
        n_strings += table.strings.len();
        crate::write_text_json(&text_dir.join(format!("{id:03}.json")), &table)?;
        n_text += 1;
    }
    println!(
        "{n_text:>6} text tables ({n_strings} strings) -> {}",
        text_dir.display()
    );

    // Music: HMI songs, written out unchanged.
    let music_dir = out.join("music");
    std::fs::create_dir_all(&music_dir)?;
    let mut n_music = 0;
    for (_, id) in bank.present(Kind::Block) {
        let item = bank.item(Kind::Block, id)?.expect("present");
        std::fs::write(music_dir.join(format!("{id:03}.hmi")), item)?;
        n_music += 1;
    }
    println!("{n_music:>6} songs    -> {}", music_dir.display());

    // Fonts: raw file, a contact sheet of every glyph, and the metrics.
    let font_dir = out.join("fonts");
    std::fs::create_dir_all(&font_dir)?;
    // 000.FRT is shared by all fonts and maps characters to glyph indices.
    let refs = motionvm_formats::find_ci(dir, "000.FRT")
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|d| font::FontRefTable::parse(&d).ok());
    let mut n_font = 0;
    let mut n_glyphs = 0;
    for (_, id) in bank.present(Kind::Font) {
        let item = bank.item(Kind::Font, id)?.expect("present");
        std::fs::write(font_dir.join(format!("{id:03}.fnt")), item)?;
        let f = motionvm_formats::m32::font::parse(item)?;
        n_glyphs += f.glyphs.len();
        crate::write_font_sheet(&font_dir.join(format!("{id:03}.png")), &f)?;
        crate::write_font_json(&font_dir.join(format!("{id:03}.json")), &f, refs.as_ref())?;
        n_font += 1;
    }
    println!(
        "{n_font:>6} fonts ({n_glyphs} glyphs) -> {}",
        font_dir.display()
    );

    // Scripts: raw module plus a disassembly of its threaded code.
    let img = motionvm_formats::m32::le::Image::open(engine_exe(dir)?)?;
    let kernel = motionvm_formats::m32::le::kernel_words(&img);
    let mut dis = Disassembler::new(&kernel);
    let scr_dir = out.join("scripts");
    std::fs::create_dir_all(&scr_dir)?;
    let mut modules = Vec::new();
    for (_, id) in bank.present(Kind::Script) {
        let item = bank.item(Kind::Script, id)?.expect("present");
        std::fs::write(scr_dir.join(format!("{id:03}.scr")), item)?;
        let m = ScrModule::parse(item)?;
        dis.learn(&m);
        modules.push(m);
    }
    // Every module has to be known before rendering, so that calls across
    // modules come out as names rather than raw offsets.
    let n_scr = modules.len();
    for m in &modules {
        let mut f = std::io::BufWriter::new(std::fs::File::create(
            scr_dir.join(format!("{:03}.f", m.module)),
        )?);
        write_module(&mut f, m, &dis)?;
    }
    println!("{n_scr:>6} scripts  -> {}", scr_dir.display());

    // Which primitives the game actually reaches for. This is the number that
    // decides how much of the 356-word kernel has to be reimplemented.
    let usage = dis.usage(&modules);
    let mut f = std::io::BufWriter::new(std::fs::File::create(out.join("kernel-usage.txt"))?);
    writeln!(f, "{:<16} {:>8}  VM", "WORD", "USES")?;
    let mut done = 0;
    let mut done_uses = 0;
    let mut total_uses = 0;
    for (name, n) in &usage {
        let have = motionvm_forth::m32::IMPLEMENTED.contains(&name.as_str());
        done += usize::from(have);
        total_uses += n;
        if have {
            done_uses += n;
        }
        writeln!(f, "{name:<16} {n:>8}  {}", if have { "yes" } else { "" })?;
    }
    println!(
        "{:>6} of {} kernel words used; {done} implemented, covering {:.0}% of all uses -> {}",
        usage.len(),
        kernel.len(),
        100.0 * done_uses as f64 / total_uses.max(1) as f64,
        out.join("kernel-usage.txt").display()
    );

    Ok(())
}

pub(crate) fn one_sprite(
    dir: &Path,
    id: usize,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let bank = Bank::open_dir(dir)?;
    let item = bank.item(Kind::Gfx8, id)?.ok_or("no such sprite")?;
    let sprite = Sprite::parse(item)?;
    println!(
        "sprite {id}: {}x{} ({} px)",
        sprite.width,
        sprite.height,
        sprite.pixels.len()
    );
    write_png(out, &sprite)?;
    println!("wrote {}", out.display());
    Ok(())
}

pub(crate) fn script(dir: &Path, id: usize) -> Result<(), Box<dyn std::error::Error>> {
    let bank = Bank::open_dir(dir)?;
    let item = bank
        .item(Kind::Script, id)?
        .ok_or("no such script module")?;
    let m = ScrModule::parse(item)?;
    let img = motionvm_formats::m32::le::Image::open(engine_exe(dir)?)?;
    let kernel = motionvm_formats::m32::le::kernel_words(&img);
    let mut dis = Disassembler::new(&kernel);
    for (_, other) in bank.present(Kind::Script) {
        if let Some(raw) = bank.item(Kind::Script, other)?
            && let Ok(parsed) = ScrModule::parse(raw)
        {
            dis.learn(&parsed);
        }
    }
    let mut stdout = std::io::stdout().lock();
    write_module(&mut stdout, &m, &dis)?;
    Ok(())
}

fn write_module(f: &mut impl Write, m: &ScrModule, dis: &Disassembler) -> std::io::Result<()> {
    write!(f, "{}", dis.module(m))
}

/// Writes an indexed-color PNG, using palette index 0 as transparent.
///
/// Index 0 is the engine's transparency slot for sprites; keeping the image
/// paletted rather than RGBA preserves the original indices for later
/// comparison against the running game.
fn write_png(path: &Path, sprite: &Sprite) -> Result<(), Box<dyn std::error::Error>> {
    motionvm_render::write_indexed_png(
        path,
        sprite.width as u32,
        sprite.height as u32,
        &sprite.pixels,
        sprite.palette.to_rgb8(),
        Some(motionvm_render::TRANSPARENT),
    )
}
