//! `motionvm-motion-tools` — inspect and extract the resources of a MOTION game, of
//! either engine generation.
//!
//! ```text
//! motionvm-motion-tools info    <gamedata>                        summary of every resource bank
//! motionvm-motion-tools extract <gamedata> [--out DIR] [--pal N]  write sprites, texts, music, scripts
//! motionvm-motion-tools sprite  <gamedata> <id> [--out F] [--pal N]  one sprite as PNG
//! motionvm-motion-tools script  <gamedata> <id>                   module header, symbols, disassembly
//! motionvm-motion-tools registers <gamedata> <block> [--ticks N] [--out F]   a song's OPL register writes
//! motionvm-motion-tools compare-frame <ours.png> <theirs.png> [--crop X,Y,W,H] [--scale N]
//! motionvm-motion-tools compare-dro   <registers.txt> <capture.dro>
//! ```
//!
//! The first four read a game; the last three are the comparison against the
//! original's own output — a frame F12 wrote against a capture, a rendered
//! register stream against a recording — and live in [`compare`] and
//! [`registers`].
//!
//! The directory's files say which generation it is: `NNN.RSC` containers
//! beside `ENGINE.EXE` are a 32-bit game (Dunkle Schatten 2) and go to
//! [`m32`], a `DATA.-1-` is a 16-bit game (Die Enviro-Kids greifen ein, Jeff
//! Jet, Hilfe für Amajambere, Victor Loomes or Falsches Spiel mit Eddie M.,
//! told apart by the engine binary beside it) and goes to [`m16`]. The two command sets write the same kinds
//! of files where the data allows and say where they differ.

mod compare;
mod json;
mod m16;
mod m32;
mod registers;

use motionvm_motion_formats::{Generation, TextTable, font};
use motionvm_render::Palette;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(e) = run() {
        eprintln!("motionvm-motion-tools: {e}");
        std::process::exit(1);
    }
}

/// What the four subcommands take.
///
/// One string, printed to stdout for `--help` and to stderr when a call does
/// not parse. Splitting the two would let them drift, and a usage text that
/// disagrees with the dispatch is worse than none.
const USAGE: &str = "\
usage: motionvm-motion-tools <command> [arguments]

  info    <gamedata>                 every resource bank, with counts and sizes
  extract <gamedata> [--out DIR]     sprites, palettes, fonts, texts, music or
                     [--pal N]       blocks, scripts with their disassembly,
                                     and the kernel usage table
                                     (--out defaults to ./out)
  sprite  <gamedata> <id> [--out F]  one sprite as an indexed PNG
                     [--pal N]       (--out defaults to ./sprite-NNNN.png)
  script  <gamedata> <id>            one script module's header, symbol table
                                     and disassembly, on stdout
  registers <gamedata> <block>       a song's OPL register writes, one a line,
                     [--ticks N]     for N ticks of the driver's clock (6000)
                     [--out F]       (--out defaults to ./registers-NNN.txt)
  compare-frame <ours.png> <theirs.png>
                     [--crop X,Y,W,H]  the frame F12 wrote against a capture of
                     [--scale N]       the original, on the colors the DAC held;
                                       names the first pixel that differs.
                                       --crop takes the picture area out of a
                                       screenshot, --scale every Nth pixel of it
  compare-dro <registers.txt> <capture.dro>
                                     a rendered register stream against a DRO
                                     recording of the original: lined up on the
                                     write the recording resumes with after its
                                     snapshot, and the first write that differs
                                     named. Both comparisons exit 1 on a
                                     difference.

<gamedata> is the directory the game was installed into: 001.RSC and
ENGINE.EXE for Dunkle Schatten 2 (MOTION 32-bit), DATA.-1- and ENVIRO.EXE for
Die Enviro-Kids greifen ein, DATA.-1-, DATA.-2- and HPPLAY.EXE for Jeff Jet,
DATA.-1-, DATA.-2- and BMZ.EXE for Hilfe für Amajambere, DATA.-1- and LL.EXE
for Victor Loomes, DATA.-1-, DATA.-2-, DATA.-3- and STERN.EXE for Falsches
Spiel mit Eddie M. (all five MOTION 16-bit). --pal names the palette a 16-bit
sprite is written through — its sprites carry none — and defaults to 0, the
one the game installs first; a 32-bit sprite carries its own.";

/// A share of the kernel words, for the coverage line: counts of words in a
/// game, far below where `f64` stops counting whole numbers.
#[expect(
    clippy::as_conversions,
    reason = "counts of words in a game, far below 2^53, as a percentage for a report"
)]
pub(crate) fn percent(part: usize, whole: usize) -> f64 {
    100.0 * part as f64 / whole.max(1) as f64
}

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

/// Which engine generation a game directory belongs to, and a refusal that
/// names both containers when it is neither.
///
/// The probe itself is `Generation::of`, in the crate that owns both readers.
/// It answers *which machine wrote these files*, which is a different question
/// from the engine's `titles::detect` — that one answers *which game a player
/// can play* and refuses a MOTION game outside its roster, while these tools
/// read more games than the player plays, Checker 2000 among them. So the two
/// probes stay apart, and the tools must never grow an engine dependency to
/// share one: both crates are the family's, so the boundary test would not
/// object, and the narrowing would be silent.
fn generation(dir: &Path) -> Result<Generation, Box<dyn std::error::Error>> {
    Generation::of(dir).ok_or_else(|| {
        format!(
            "{}: neither a DATA.-1- (MOTION 16-bit) nor NNN.RSC containers (MOTION 32-bit)",
            dir.display()
        )
        .into()
    })
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, rest) = args.split_first().unwrap_or_else(|| usage());
    match cmd.as_str() {
        // Asking for help is not a usage error, so it answers on stdout and
        // exits 0 — otherwise `motionvm-motion-tools --help | less` shows nothing and
        // a shell script that checks the status treats the answer as failure.
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        "info" => {
            let dir = Path::new(rest.first().unwrap_or_else(|| usage()));
            match generation(dir)? {
                Generation::Motion32 => m32::info(dir),
                Generation::Motion16 => m16::info(dir),
            }
        }
        "extract" => {
            let dir = Path::new(rest.first().unwrap_or_else(|| usage()));
            let out = flag(rest, "--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("out"));
            match generation(dir)? {
                Generation::Motion32 => m32::extract(dir, &out),
                Generation::Motion16 => m16::extract(dir, &out, palette_flag(rest)?),
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
                Generation::Motion32 => m32::one_sprite(dir, id, &out),
                Generation::Motion16 => m16::one_sprite(dir, id, &out, palette_flag(rest)?),
            }
        }
        "script" => {
            let dir = Path::new(rest.first().unwrap_or_else(|| usage()));
            let id: usize = rest
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| usage());
            match generation(dir)? {
                Generation::Motion32 => m32::script(dir, id),
                Generation::Motion16 => m16::script(dir, id),
            }
        }
        "registers" => {
            let dir = Path::new(rest.first().unwrap_or_else(|| usage()));
            let id: usize = rest
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| usage());
            let ticks = match flag(rest, "--ticks") {
                Some(n) => n.parse().map_err(|_| format!("--ticks {n}: not a count"))?,
                None => 6000,
            };
            let out = flag(rest, "--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(format!("registers-{id:03}.txt")));
            registers::registers(dir, id, ticks, &out)
        }
        "compare-frame" => {
            let ours = Path::new(rest.first().unwrap_or_else(|| usage()));
            let theirs = Path::new(rest.get(1).unwrap_or_else(|| usage()));
            let crop = flag(rest, "--crop").map(|s| crop_flag(&s)).transpose()?;
            let scale = match flag(rest, "--scale") {
                Some(n) => n
                    .parse()
                    .map_err(|_| format!("--scale {n}: not a factor"))?,
                None => 1,
            };
            finding(compare::frame(ours, theirs, crop, scale)?)
        }
        "compare-dro" => {
            let ours = Path::new(rest.first().unwrap_or_else(|| usage()));
            let theirs = Path::new(rest.get(1).unwrap_or_else(|| usage()));
            finding(compare::dro(ours, theirs)?)
        }
        _ => usage(),
    }
}

/// A comparison's answer as an exit status: a difference is not an error —
/// the report has been printed — but it is a finding, and a script that runs
/// the comparison wants to see it without reading the report. Exit 1, the
/// way `diff` and `cmp` say the same thing.
fn finding(same: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !same {
        std::process::exit(1);
    }
    Ok(())
}

/// `--crop X,Y,W,H`: four numbers, or a usage error that says which.
fn crop_flag(s: &str) -> Result<compare::Crop, Box<dyn std::error::Error>> {
    let n: Vec<u32> = s
        .split(',')
        .map(|v| v.trim().parse())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("--crop {s}: wants four numbers, X,Y,W,H"))?;
    match n[..] {
        [x, y, w, h] => Ok(compare::Crop { x, y, w, h }),
        _ => Err(format!("--crop {s}: wants four numbers, X,Y,W,H").into()),
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
    for index in 0..=u8::MAX {
        let (x, y) = (
            usize::from(index % 16) * CELL,
            usize::from(index / 16) * CELL,
        );
        for row in px[y * SIDE..].chunks_mut(SIDE).take(CELL) {
            row[x..x + CELL].fill(index);
        }
    }
    let side = u32::try_from(SIDE)?;
    motionvm_render::write_indexed_png(path, side, side, &px, pal.to_rgb8(), None)
        .map_err(Into::into)
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
    let cell_w = f
        .glyphs
        .iter()
        .map(|g| usize::from(g.width))
        .max()
        .unwrap_or(1)
        + PAD;
    let cell_h = usize::from(f.height) + PAD;
    let rows = f.glyphs.len().div_ceil(COLS);
    let (w, h) = ((COLS * cell_w).max(1), (rows * cell_h).max(1));

    // 0 = background, 1 = glyph pixel, 2 = cell separator.
    let mut px = vec![0u8; w * h];
    for (i, g) in f.glyphs.iter().enumerate() {
        let (cx, cy) = ((i % COLS) * cell_w, (i / COLS) * cell_h);
        for y in 0..f.height {
            for x in 0..g.width {
                if g.pixel(x, y) {
                    px[(cy + usize::from(y)) * w + cx + usize::from(x)] = 1;
                }
            }
        }
        // A tick in the corner marks where each cell starts, so glyph widths
        // stay readable even where a glyph is blank.
        px[cy * w + cx] = 2;
    }

    // White ground, black ink, a pale tick in each cell's corner.
    let palette = vec![0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0xd0, 0xd8, 0xe8];
    motionvm_render::write_indexed_png(
        path,
        u32::try_from(w)?,
        u32::try_from(h)?,
        &px,
        palette,
        None,
    )
    .map_err(Into::into)
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
                && usize::from(g) < f.glyphs.len()
            {
                let name = motionvm_motion_formats::cp437_char(ch);
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
