//! `motionvm-tools` — inspect and extract the resources of a MOTION game, of
//! either engine generation.
//!
//! ```text
//! motionvm-tools info    <gamedata>                        summary of every resource bank
//! motionvm-tools extract <gamedata> [--out DIR] [--pal N]  write sprites, texts, music, scripts
//! motionvm-tools sprite  <gamedata> <id> [--out F] [--pal N]  one sprite as PNG
//! motionvm-tools script  <gamedata> <id>                   module header, symbols, disassembly
//! ```
//!
//! The directory's files say which generation it is: `NNN.RSC` containers
//! beside `ENGINE.EXE` are a 32-bit game (Dunkle Schatten 2) and go to
//! [`m32`], a `DATA.-1-` is a 16-bit game (Die Enviro-Kids greifen ein or
//! Jeff Jet - Abenteuer InfoHighway, told apart by the engine binary beside
//! it) and goes to [`m16`]. The two command sets write the same kinds of
//! files where the data allows and say where they differ.

mod json;
mod m16;
mod m32;

use motionvm_formats::{Palette, TextTable, font};
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
  extract <gamedata> [--out DIR]     sprites, palettes, fonts, texts, music or
                     [--pal N]       blocks, scripts with their disassembly,
                                     and the kernel usage table
                                     (--out defaults to ./out)
  sprite  <gamedata> <id> [--out F]  one sprite as an indexed PNG
                     [--pal N]       (--out defaults to ./sprite-NNNN.png)
  script  <gamedata> <id>            one script module's header, symbol table
                                     and disassembly, on stdout

<gamedata> is the directory the game was installed into: 001.RSC and
ENGINE.EXE for Dunkle Schatten 2 (MOTION 32-bit), DATA.-1- and ENVIRO.EXE for
Die Enviro-Kids greifen ein, DATA.-1-, DATA.-2- and HPPLAY.EXE for Jeff Jet -
Abenteuer InfoHighway (both MOTION 16-bit). --pal names the palette a 16-bit
sprite is written through — its sprites carry none — and defaults to 0, the
one the game installs first; a 32-bit sprite carries its own.";

/// What to append to a count line when the container's index over-claimed.
///
/// Empty for an undamaged container, which is every shipped one. A copy that
/// was truncated in transit, or a download that stopped early, says how many
/// items it is short of beside how many came out — the same shape `info`
/// already uses for the occupancy mismatches and the trailing slack.
pub(crate) fn over_claimed(n: usize) -> String {
    match n {
        0 => String::new(),
        1 => "  (1 more is indexed and not in the file)".into(),
        n => format!("  ({n} more are indexed and not in the file)"),
    }
}

fn usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2);
}

/// Which engine generation a game directory belongs to, told by its files.
enum Generation {
    /// `NNN.RSC` containers and `ENGINE.EXE`.
    M32,
    /// A `DATA.-n-` container, and the engine binary beside it.
    M16,
}

fn generation(dir: &Path) -> Result<Generation, Box<dyn std::error::Error>> {
    if motionvm_formats::find_ci(dir, "DATA.-1-").is_some() {
        return Ok(Generation::M16);
    }
    let has_rsc = std::fs::read_dir(dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                let p = e.path();
                p.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| x.eq_ignore_ascii_case("rsc"))
                    && p.file_stem()
                        .and_then(|s| s.to_str())
                        .is_some_and(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
            })
        })
        .unwrap_or(false);
    if has_rsc {
        return Ok(Generation::M32);
    }
    Err(format!(
        "{}: neither a DATA.-1- (MOTION 16-bit) nor NNN.RSC containers (MOTION 32-bit)",
        dir.display()
    )
    .into())
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
            let dir = Path::new(rest.first().unwrap_or_else(|| usage()));
            match generation(dir)? {
                Generation::M32 => m32::info(dir),
                Generation::M16 => m16::info(dir),
            }
        }
        "extract" => {
            let dir = Path::new(rest.first().unwrap_or_else(|| usage()));
            let out = flag(rest, "--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("out"));
            match generation(dir)? {
                Generation::M32 => m32::extract(dir, &out),
                Generation::M16 => m16::extract(dir, &out, palette_flag(rest)?),
            }
        }
        "sprite" => {
            let dir = Path::new(rest.first().unwrap_or_else(|| usage()));
            let id: usize = rest
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| usage());
            let out = flag(rest, "--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(format!("sprite-{id:04}.png")));
            match generation(dir)? {
                Generation::M32 => m32::one_sprite(dir, id, &out),
                Generation::M16 => m16::one_sprite(dir, id, &out, palette_flag(rest)?),
            }
        }
        "script" => {
            let dir = Path::new(rest.first().unwrap_or_else(|| usage()));
            let id: usize = rest
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| usage());
            match generation(dir)? {
                Generation::M32 => m32::script(dir, id),
                Generation::M16 => m16::script(dir, id),
            }
        }
        _ => usage(),
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1).cloned()
}

/// `--pal N`, or the 16-bit default; a value that is not a number is a usage
/// error rather than silently the default.
fn palette_flag(args: &[String]) -> Result<usize, Box<dyn std::error::Error>> {
    match flag(args, "--pal") {
        None => Ok(m16::DEFAULT_PALETTE),
        Some(s) => s
            .parse()
            .map_err(|_| format!("--pal {s}: not a palette number").into()),
    }
}

/// Writes a text table as a JSON array of strings.
pub(crate) fn write_text_json(
    path: &Path,
    table: &TextTable,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
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
    Ok(())
}

/// Writes a palette as a 16x16 grid of 8x8 swatches so it can be eyeballed.
pub(crate) fn write_palette_png(
    path: &Path,
    pal: &Palette,
) -> Result<(), Box<dyn std::error::Error>> {
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
pub(crate) fn write_font_sheet(
    path: &Path,
    f: &font::Font,
) -> Result<(), Box<dyn std::error::Error>> {
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
pub(crate) fn write_font_json(
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

#[cfg(test)]
mod tests {
    use super::over_claimed;

    #[test]
    fn an_undamaged_container_adds_nothing_to_the_line() {
        assert_eq!(over_claimed(0), "");
    }

    #[test]
    fn a_damaged_one_says_how_many_and_reads_as_a_sentence() {
        assert_eq!(over_claimed(1), "  (1 more is indexed and not in the file)");
        assert_eq!(
            over_claimed(7),
            "  (7 more are indexed and not in the file)"
        );
    }
}
