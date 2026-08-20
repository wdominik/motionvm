//! `motionvm-tools` — inspect and extract the resources of a MOTION game.
//!
//! ```text
//! motionvm-tools info    <gamedata>                 summary of every resource bank
//! motionvm-tools extract <gamedata> [--out DIR]     write sprites, texts, music, scripts
//! motionvm-tools sprite  <gamedata> <id> [--out F]  one sprite as PNG
//! motionvm-tools script  <gamedata> <id>            module header and disassembly
//! ```

mod json;

use motionvm_formats::{
    Kind, Palette, ScrModule, Sprite, TextTable, disasm::Disassembler, font, rsc::Bank,
};
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(e) = run() {
        eprintln!("motionvm-tools: {e}");
        std::process::exit(1);
    }
}

/// What the four subcommands take.
///
/// One string, printed to stdout for `--help` and to stderr when a call does
/// not parse. Splitting the two would let them drift, and a usage text that
/// disagrees with the dispatch is worse than none.
const USAGE: &str = "\
usage: motionvm-tools <command> [arguments]

  info    <gamedata>                 every resource bank, with counts and sizes
  extract <gamedata> [--out DIR]     sprites, palettes, fonts, texts, music,
                                     scripts and the kernel usage table
                                     (--out defaults to ./out)
  sprite  <gamedata> <id> [--out F]  one sprite as an indexed PNG
                                     (--out defaults to ./sprite-NNNN.png)
  script  <gamedata> <id>            one script module's header, symbol table
                                     and disassembly, on stdout

<gamedata> is the directory the game was installed into — the one holding
001.RSC and ENGINE.EXE.";

fn usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2);
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, rest) = args.split_first().unwrap_or_else(|| usage());
    match cmd.as_str() {
        // Asking for help is not a usage error, so it answers on stdout and
        // exits 0 — otherwise `motionvm-tools --help | less` shows nothing and
        // a shell script that checks the status treats the answer as failure.
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        "info" => {
            let dir = rest.first().unwrap_or_else(|| usage());
            info(Path::new(dir))
        }
        "extract" => {
            let dir = rest.first().unwrap_or_else(|| usage());
            let out = flag(rest, "--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("out"));
            extract(Path::new(dir), &out)
        }
        "sprite" => {
            let dir = rest.first().unwrap_or_else(|| usage());
            let id: usize = rest
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| usage());
            let out = flag(rest, "--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(format!("sprite-{id:04}.png")));
            one_sprite(Path::new(dir), id, &out)
        }
        "script" => {
            let dir = rest.first().unwrap_or_else(|| usage());
            let id: usize = rest
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| usage());
            script(Path::new(dir), id)
        }
        _ => usage(),
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1).cloned()
}

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

fn info(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

fn extract(dir: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
        write_palette_png(
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
        let table = TextTable::parse(item)?;
        n_strings += table.strings.len();
        let mut f = std::io::BufWriter::new(std::fs::File::create(
            text_dir.join(format!("{id:03}.json")),
        )?);
        writeln!(f, "[")?;
        for (i, s) in table.strings.iter().enumerate() {
            let comma = if i + 1 == table.strings.len() {
                ""
            } else {
                ","
            };
            writeln!(f, "  {}{comma}", json::quote(s))?;
        }
        writeln!(f, "]")?;
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
        let f = font::Font::parse(item)?;
        n_glyphs += f.glyphs.len();
        write_font_sheet(&font_dir.join(format!("{id:03}.png")), &f)?;
        write_font_json(&font_dir.join(format!("{id:03}.json")), &f, refs.as_ref())?;
        n_font += 1;
    }
    println!(
        "{n_font:>6} fonts ({n_glyphs} glyphs) -> {}",
        font_dir.display()
    );

    // Scripts: raw module plus a disassembly of its threaded code.
    let img = motionvm_formats::le::Image::open(engine_exe(dir)?)?;
    let kernel = motionvm_formats::le::kernel_words(&img);
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
        let have = motionvm_forth::IMPLEMENTED.contains(&name.as_str());
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

fn one_sprite(dir: &Path, id: usize, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

fn script(dir: &Path, id: usize) -> Result<(), Box<dyn std::error::Error>> {
    let bank = Bank::open_dir(dir)?;
    let item = bank
        .item(Kind::Script, id)?
        .ok_or("no such script module")?;
    let m = ScrModule::parse(item)?;
    let img = motionvm_formats::le::Image::open(engine_exe(dir)?)?;
    let kernel = motionvm_formats::le::kernel_words(&img);
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

/// Writes a palette as a 16x16 grid of 8x8 swatches so it can be eyeballed.
fn write_palette_png(path: &Path, pal: &Palette) -> Result<(), Box<dyn std::error::Error>> {
    const CELL: usize = 8;
    const SIDE: usize = 16 * CELL;
    let mut px = vec![0u8; SIDE * SIDE];
    for (i, p) in px.iter_mut().enumerate() {
        let (x, y) = (i % SIDE, i / SIDE);
        *p = ((y / CELL) * 16 + x / CELL) as u8;
    }
    motionvm_render::write_indexed_png(path, SIDE as u32, SIDE as u32, &px, pal.to_rgb8(), None)
}

/// Writes every glyph of a font into one image, laid out in a grid.
///
/// A contact sheet rather than one file per glyph: nine fonts come to some
/// eight hundred glyphs, and the point of the output is to be able to look at a
/// font and see whether it decoded correctly.
fn write_font_sheet(path: &Path, f: &font::Font) -> Result<(), Box<dyn std::error::Error>> {
    const COLS: usize = 16;
    const PAD: usize = 2;
    let cell_w = f.glyphs.iter().map(|g| g.width as usize).max().unwrap_or(1) + PAD;
    let cell_h = f.height as usize + PAD;
    let rows = f.glyphs.len().div_ceil(COLS);
    let (w, h) = ((COLS * cell_w).max(1), (rows * cell_h).max(1));

    // 0 = background, 1 = glyph pixel, 2 = cell separator.
    let mut px = vec![0u8; w * h];
    for (i, g) in f.glyphs.iter().enumerate() {
        let (cx, cy) = ((i % COLS) * cell_w, (i / COLS) * cell_h);
        for y in 0..f.height {
            for x in 0..g.width {
                if g.pixel(x, y) {
                    px[(cy + y as usize) * w + cx + x as usize] = 1;
                }
            }
        }
        // A tick in the corner marks where each cell starts, so glyph widths
        // stay readable even where a glyph is blank.
        px[cy * w + cx] = 2;
    }

    // White ground, black ink, a pale tick in each cell's corner.
    let palette = vec![0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0xd0, 0xd8, 0xe8];
    motionvm_render::write_indexed_png(path, w as u32, h as u32, &px, palette, None)
}

/// Writes a font's metrics and its character mapping.
fn write_font_json(
    path: &Path,
    f: &font::Font,
    refs: Option<&font::FontRefTable>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(out, "{{")?;
    writeln!(out, "  \"height\": {},", f.height)?;
    writeln!(out, "  \"glyph_count\": {},", f.glyphs.len())?;
    let widths: Vec<String> = f.glyphs.iter().map(|g| g.width.to_string()).collect();
    writeln!(out, "  \"widths\": [{}],", widths.join(", "))?;
    writeln!(out, "  \"characters\": {{")?;
    if let Some(r) = refs {
        let mut rows = Vec::new();
        for ch in 0u8..=255 {
            if let Some(g) = r.glyph_for(ch)
                && (g as usize) < f.glyphs.len()
            {
                let name = motionvm_formats::cp437_char(ch);
                rows.push(format!(
                    "    {}: {{\"char\": {}, \"glyph\": {g}}}",
                    json::quote(&ch.to_string()),
                    json::quote(&name.to_string())
                ));
            }
        }
        writeln!(out, "{}", rows.join(",\n"))?;
    }
    writeln!(out, "  }}")?;
    writeln!(out, "}}")?;
    Ok(())
}
