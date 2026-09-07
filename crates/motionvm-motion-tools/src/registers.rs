//! `registers`: a song's OPL register stream, rendered offline out of the
//! game's own files, one write a line.
//!
//! This is our side of `compare-dro`. The recording on the other side is
//! DOSBox-X's, made of the original driver playing the same block; what is
//! rendered here is the rebuilt driver playing it — the sequencer and the FM
//! driver of `motionvm-motion-audio`, which are the crates the player runs,
//! fed the block straight out of the container. No engine is involved: a
//! song is started by number, the way `STARTTUNE` starts one, and everything
//! after that is the audio layer's alone.
//!
//! The 32-bit game plays through `HMIMDRV.386`'s OPL3 driver with its two
//! instrument banks, the 16-bit ones through `MUSADL.DRV` — an OPL2 driver,
//! which is why a 16-bit recording is one of an OPL2. A 16-bit song is
//! started to loop, as `-1 N STARTTUNE` starts nearly every one the games
//! play; a 32-bit song loops or does not by its own data.

use crate::compare::write_line;
use motionvm_motion_audio::m32::Fm;
use motionvm_motion_formats::Generation;
use motionvm_motion_formats::m16::{Container, Segment, psm};
use motionvm_motion_formats::m32::{DriverArchive, Kind, bnk, hmi, rsc};
use std::path::Path;

/// A file of the game's, found whatever case a copy left its name in.
fn game_file(dir: &Path, name: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let path = motionvm_motion_formats::find_ci(dir, name)
        .ok_or_else(|| format!("{}: no {name}", dir.display()))?;
    Ok(std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?)
}

/// Renders block `id` for `ticks` ticks of the driver's clock and writes the
/// stream to `out`.
pub(crate) fn registers(
    dir: &Path,
    id: usize,
    ticks: u32,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let generation = crate::generation(dir)?;
    let mut text = format!(
        "# block {id} of {}, {}: every OPL register write of {ticks} ticks, as\n\
         # `tick register value`; tick 0 carries the driver's switch-on, the\n\
         # song starts on tick 1. Compare with `motionvm-motion-tools compare-dro`.\n",
        dir.display(),
        match generation {
            Generation::Motion32 => "MOTION 32-bit, through the OPL3 driver of HMIMDRV.386",
            Generation::Motion16 => "MOTION 16-bit, through MUSADL.DRV",
        }
    );
    let count = match generation {
        Generation::Motion32 => render32(dir, id, ticks, &mut text)?,
        Generation::Motion16 => render16(dir, id, ticks, &mut text)?,
    };
    std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{count:>7} writes over {ticks} ticks -> {}", out.display());
    Ok(())
}

fn render32(
    dir: &Path,
    id: usize,
    ticks: u32,
    text: &mut String,
) -> Result<usize, Box<dyn std::error::Error>> {
    let bank = rsc::Bank::open_dir(dir)?;
    let block = bank
        .item(Kind::Block, id)?
        .ok_or_else(|| format!("no block {id}"))?;
    let song = hmi::Song::parse(block).map_err(|e| format!("block {id} is not a song: {e}"))?;
    let archive = game_file(dir, "HMIMDRV.386")?;
    let archive = DriverArchive::parse(&archive)?;
    let device = archive
        .device(Fm::DEVICE)
        .ok_or("HMIMDRV.386 holds no OPL3 driver")?;
    let melodic = bnk::Bank::parse(&game_file(dir, "MELODIC.BNK")?)?;
    let drums = bnk::Bank::parse(&game_file(dir, "DRUM.BNK")?)?;
    let mut fm = Fm::new(device, &melodic, &drums)?;
    let mut seq = motionvm_motion_audio::m32::Sequencer::new(
        song,
        Fm::DEVICE,
        motionvm_motion_audio::m32::Sequencer::FULL_VOLUME,
    );
    let mut writes = Vec::new();
    let mut messages = Vec::new();
    let mut count = 0;
    fm.take_into(&mut writes);
    for tick in 0..=ticks {
        if tick > 0 {
            messages.clear();
            seq.tick(&mut messages);
            for m in &messages {
                fm.send(*m);
            }
            fm.take_into(&mut writes);
        }
        for w in writes.drain(..) {
            write_line(text, tick, w.address(), w.value);
            count += 1;
        }
    }
    Ok(count)
}

fn render16(
    dir: &Path,
    id: usize,
    ticks: u32,
    text: &mut String,
) -> Result<usize, Box<dyn std::error::Error>> {
    let container = Container::open_dir(dir)?;
    let block = container
        .item(Segment::Blk, id)?
        .ok_or_else(|| format!("no block {id}"))?;
    if !psm::is_song(block) {
        return Err(format!("block {id} is not a PSM 2 song").into());
    }
    let song = psm::Plx::parse(block)?;
    let driver = motionvm_motion_audio::m16::Driver::parse(&game_file(dir, "MUSADL.DRV")?)?;
    let mut writes = motionvm_motion_audio::m16::Sequencer::install_writes(&driver);
    let mut seq = motionvm_motion_audio::m16::Sequencer::new(&driver, song, -1);
    let mut count = 0;
    for tick in 0..=ticks {
        if tick > 0 {
            seq.tick(&mut writes);
        }
        for w in writes.drain(..) {
            write_line(text, tick, w.address(), w.value);
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::read_registers;

    /// The opening tune of Dunkle Schatten 2, rendered for two hundred ticks:
    /// the file begins with the switch-on at tick 0, holds the song from tick
    /// 1 on, and reads back as what was written.
    #[test]
    fn the_opening_tune_renders_to_a_file_that_reads_back() {
        let Some(dir) = motionvm_motion_testutil::gamedata_ds2() else {
            eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
            return;
        };
        let out =
            std::env::temp_dir().join(format!("motionvm-registers-{}.txt", std::process::id()));
        registers(&dir, 25, 200, &out).expect("the render");
        let text = std::fs::read_to_string(&out).expect("the file");
        let _ = std::fs::remove_file(&out);
        let writes = read_registers(&text).expect("the file reads back");
        assert!(writes.len() > 100, "{} writes", writes.len());
        assert!(
            writes.iter().any(|&(t, _, _)| t == 0),
            "the switch-on at tick 0"
        );
        assert!(
            writes
                .iter()
                .any(|&(t, r, _)| t > 0 && r & 0xff >= 0xb0 && r & 0xff <= 0xb8),
            "a key-on later"
        );
        assert_eq!(writes.last().map(|w| w.0 <= 200), Some(true));
    }
}
