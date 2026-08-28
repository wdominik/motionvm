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

/// Runs `each` over every item of `kind` that the index claims *and* the file
/// really holds, and answers `(written, over-claimed)`.
///
/// `present` reads the offset table and `item` reads the data behind it, and
/// the two do not agree on a damaged container: `Rsc::present` calls a slot
/// filled when its offset differs from the next one's, while `Rsc::item`
/// answers `None` as soon as that pair stops increasing — which is what a
/// truncated or half-copied offset table looks like. Counted and skipped
/// rather than fatal, for the same reason `Game::<Vm>::open` skips such a
/// slot: one bad entry should not stop the rest from coming out, and a person
/// holding a damaged copy is better served by what is there plus a count than
/// by an abort naming nothing they can act on.
fn each_item(
    bank: &Bank,
    kind: Kind,
    mut each: impl FnMut(usize, &[u8]) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let (mut written, mut over) = (0, 0);
    for (_, id) in bank.present(kind) {
        match bank.item(kind, id)? {
            Some(item) => {
                each(id, item)?;
                written += 1;
            }
            None => over += 1,
        }
    }
    Ok((written, over))
}

pub(crate) fn extract(dir: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let bank = Bank::open_dir(dir)?;

    // Sprites.
    let sprite_dir = out.join("sprites");
    std::fs::create_dir_all(&sprite_dir)?;
    let (n_sprites, over) = each_item(&bank, Kind::Gfx8, |id, item| {
        let sprite = Sprite::parse(item)?;
        write_png(&sprite_dir.join(format!("{id:04}.png")), &sprite)
    })?;
    println!(
        "{n_sprites:>6} sprites  -> {}{}",
        sprite_dir.display(),
        crate::over_claimed(over)
    );

    // Palettes: raw 6-bit bytes plus a readable swatch strip.
    let pal_dir = out.join("palettes");
    std::fs::create_dir_all(&pal_dir)?;
    let (n_pal, over) = each_item(&bank, Kind::Palette, |id, item| {
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
    let (n_text, over) = each_item(&bank, Kind::Text, |id, item| {
        let table = motionvm_formats::m32::text::parse(item)?;
        n_strings += table.strings.len();
        crate::write_text_json(&text_dir.join(format!("{id:03}.json")), &table)
    })?;
    println!(
        "{n_text:>6} text tables ({n_strings} strings) -> {}{}",
        text_dir.display(),
        crate::over_claimed(over)
    );

    // Music: HMI songs, written out unchanged.
    let music_dir = out.join("music");
    std::fs::create_dir_all(&music_dir)?;
    let (n_music, over) = each_item(&bank, Kind::Block, |id, item| {
        Ok(std::fs::write(
            music_dir.join(format!("{id:03}.hmi")),
            item,
        )?)
    })?;
    println!(
        "{n_music:>6} songs    -> {}{}",
        music_dir.display(),
        crate::over_claimed(over)
    );

    // Fonts: raw file, a contact sheet of every glyph, and the metrics.
    let font_dir = out.join("fonts");
    std::fs::create_dir_all(&font_dir)?;
    // 000.FRT is shared by all fonts and maps characters to glyph indices.
    let refs = motionvm_formats::find_ci(dir, "000.FRT")
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|d| font::FontRefTable::parse(&d).ok());
    let mut n_glyphs = 0;
    let (n_font, over) = each_item(&bank, Kind::Font, |id, item| {
        std::fs::write(font_dir.join(format!("{id:03}.fnt")), item)?;
        let f = motionvm_formats::m32::font::parse(item)?;
        n_glyphs += f.glyphs.len();
        crate::write_font_sheet(&font_dir.join(format!("{id:03}.png")), &f)?;
        crate::write_font_json(&font_dir.join(format!("{id:03}.json")), &f, refs.as_ref())
    })?;
    println!(
        "{n_font:>6} fonts ({n_glyphs} glyphs) -> {}{}",
        font_dir.display(),
        crate::over_claimed(over)
    );

    // Scripts: raw module plus a disassembly of its threaded code.
    let img = motionvm_formats::m32::le::Image::open(engine_exe(dir)?)?;
    let kernel = motionvm_formats::m32::le::kernel_words(&img);
    let mut dis = Disassembler::new(&kernel);
    let scr_dir = out.join("scripts");
    std::fs::create_dir_all(&scr_dir)?;
    let mut modules = Vec::new();
    let (n_scr, over) = each_item(&bank, Kind::Script, |id, item| {
        std::fs::write(scr_dir.join(format!("{id:03}.scr")), item)?;
        let m = ScrModule::parse(item)?;
        dis.learn(&m);
        modules.push(m);
        Ok(())
    })?;
    // Every module has to be known before rendering, so that calls across
    // modules come out as names rather than raw offsets.
    for m in &modules {
        let mut f = std::io::BufWriter::new(std::fs::File::create(
            scr_dir.join(format!("{:03}.f", m.module)),
        )?);
        write_module(&mut f, m, &dis)?;
    }
    println!(
        "{n_scr:>6} scripts  -> {}{}",
        scr_dir.display(),
        crate::over_claimed(over)
    );

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

#[cfg(test)]
mod tests {
    use super::*;

    /// A container whose index over-claims: three GFX slots, of which the
    /// middle one is listed as filled and has no bytes.
    ///
    /// `Rsc::present` calls a slot filled when its offset differs from the
    /// next one's; `Rsc::item` answers `None` as soon as that pair stops
    /// increasing. A decreasing pair satisfies both, which is what a
    /// truncated or half-written offset table looks like from the outside.
    fn over_claiming_rsc() -> Vec<u8> {
        let counts = [3u32, 1, 1, 1, 1, 1];
        let total = 2 * counts[0] + counts[1..].iter().sum::<u32>();
        let table_end = 0x30 + total * 4;
        let mut offsets = vec![table_end + 12; total as usize];
        offsets[0] = table_end; // slot 0: six bytes
        offsets[1] = table_end + 6; // slot 1: starts after slot 2 ends
        offsets[2] = table_end + 5;
        let mut v = Vec::new();
        for c in counts {
            v.extend_from_slice(&c.to_le_bytes());
        }
        v.resize(0x30, 0);
        for o in &offsets {
            v.extend_from_slice(&o.to_le_bytes());
        }
        v.resize(table_end as usize + 12, 0xaa);
        v
    }

    /// A directory of this test's own under `target/`, wiped before use.
    fn test_dir(name: &str) -> std::path::PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-dirs")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a test directory");
        dir
    }

    #[test]
    fn an_over_claiming_index_is_counted_and_skipped_rather_than_asserted_on() {
        let dir = test_dir("tools_m32_over_claim");
        std::fs::write(dir.join("001.RSC"), over_claiming_rsc()).expect("writing the container");
        let bank = Bank::open_dir(&dir).expect("the container opens");
        assert_eq!(bank.present(Kind::Gfx8).len(), 3, "the index claims three");

        let mut seen = Vec::new();
        let (written, over) = each_item(&bank, Kind::Gfx8, |id, item| {
            seen.push((id, item.len()));
            Ok(())
        })
        .expect("no hard error");
        assert_eq!(seen, [(0, 6), (2, 7)], "the two that are really there");
        assert_eq!((written, over), (2, 1));
    }

    #[test]
    fn an_undamaged_index_over_claims_nothing() {
        // The same container with the decreasing pair repaired: every slot
        // the index claims is handed over, and the second count stays zero,
        // which is what every shipped container answers.
        let dir = test_dir("tools_m32_intact");
        let mut bytes = over_claiming_rsc();
        let at = 0x30 + 4;
        bytes[at..at + 4].copy_from_slice(&(0x30u32 + 11 * 4 + 4).to_le_bytes());
        std::fs::write(dir.join("001.RSC"), bytes).expect("writing the container");
        let bank = Bank::open_dir(&dir).expect("the container opens");

        let mut ids = Vec::new();
        let (written, over) = each_item(&bank, Kind::Gfx8, |id, _| {
            ids.push(id);
            Ok(())
        })
        .expect("no hard error");
        assert_eq!(ids, [0, 1, 2]);
        assert_eq!((written, over), (3, 0));
    }

    #[test]
    fn an_error_from_the_body_is_not_swallowed() {
        // The count is for what the file does not hold. A reader that fails
        // on bytes that *are* there is a different thing and still stops the
        // command, so a corrupt sprite is not quietly reported as a missing
        // one.
        let dir = test_dir("tools_m32_body_error");
        std::fs::write(dir.join("001.RSC"), over_claiming_rsc()).expect("writing the container");
        let bank = Bank::open_dir(&dir).expect("the container opens");
        let e = each_item(&bank, Kind::Gfx8, |_, _| Err("the reader said no".into()))
            .expect_err("the body's error reaches the caller");
        assert_eq!(e.to_string(), "the reader said no");
    }
}
