//! Damaged copies of the shipped files are refused, never crashed on.
//!
//! `malformed.rs` hands the readers inputs broken in ways somebody thought
//! of. This walks the real files and breaks them in ways nobody did: every
//! container, every engine binary and a sample of every kind of item, cut
//! short at every length through its header and at cuts spaced over the rest,
//! and with single bytes flipped at places a seeded generator picks. What is
//! asserted about each is only that the reader *answers* — an `Ok` or an
//! `Err`, never an unwind — because that is the whole promise a reader makes
//! about a file it did not write: a truncated download or a bad sector is
//! reported by name, not as a crash report about this workspace.
//!
//! The generator is the machines' own linear congruential recurrence, seeded
//! from the file's name, so a failure names a seed and a byte that any other
//! machine with the game reproduces exactly. Nothing here reads a clock.
//!
//! What a mutation cannot check is whether a *wrong* answer came back — a
//! flipped byte in a sprite's pixels decodes to a different picture and no
//! reader can tell. That is the digests' business, over the undamaged files.
//!
//! Needs the games' files and skips, game by game, without them. The games
//! this file drives are all six: Dunkle Schatten 2 (MOTION 32-bit) and Die
//! Enviro-Kids greifen ein, Jeff Jet, Hilfe für Amajambere, Victor Loomes and
//! Falsches Spiel mit Eddie M. (MOTION 16-bit).

use motionvm_motion_formats::font::FontRefTable;
use motionvm_motion_formats::m16::{self, Container, GfxInf, Segment, mz, psm::Plx};
use motionvm_motion_formats::m32::{self, DriverArchive, InstrumentBank, Kind, Song, rsc};
use motionvm_motion_formats::{find_ci, m32::rsc::Bank};
use motionvm_motion_testutil::{
    game_file, gamedata_ds2, gamedata_eddiem, gamedata_enviro, gamedata_hfa, gamedata_jeffjet,
    gamedata_vloomes,
};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

/// The recurrence both machines draw `RANDOM` from, so that "seed 7, flip 12"
/// means the same byte on every machine.
struct Lcg(u32);

impl Lcg {
    /// A generator for `what`: the name folded into a seed, so that a file
    /// keeps its sequence whatever else the suite walks before it.
    fn for_name(what: &str) -> Self {
        Self(what.bytes().fold(0x1234_5678u32, |s, b| {
            s.wrapping_mul(31).wrapping_add(u32::from(b))
        }))
    }

    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }

    /// A number in `0..n`, taken from the high half, which is the half an
    /// LCG's low bits do not spoil.
    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() >> 8).unwrap() % n.max(1)
    }
}

/// How many bytes to flip in one input, and how many cuts to spread over the
/// part of it past the header. Small enough that the whole walk over five
/// games takes seconds; the header is walked byte by byte regardless, because
/// that is where a reader trusts what it reads.
const FLIPS: usize = 32;
const SPACED_CUTS: usize = 16;

/// The lengths `bytes` is cut down to: every length up to `head`, then
/// `SPACED_CUTS` more spread evenly over the rest, then one byte short of
/// whole — the cut a download most often makes.
fn cuts(len: usize, head: usize) -> Vec<usize> {
    let mut out: Vec<usize> = (0..=head.min(len)).collect();
    if len > head {
        let span = len - head;
        out.extend((1..=SPACED_CUTS).map(|i| head + span * i / (SPACED_CUTS + 1)));
        out.push(len - 1);
    }
    out.dedup();
    out
}

/// Every damaged version of one input that made a reader unwind.
#[derive(Default)]
struct Report {
    failures: Vec<String>,
}

impl Report {
    /// Cuts `bytes` at every length [`cuts`] names and flips [`FLIPS`] bytes
    /// of it one at a time, handing each version to `parse`, and records the
    /// versions that panicked. `head` is how far the header reaches — the
    /// region cut byte by byte.
    fn walk(&mut self, what: &str, bytes: &[u8], head: usize, mut parse: impl FnMut(&[u8])) {
        let mut buf = bytes.to_vec();
        for cut in cuts(bytes.len(), head) {
            if catch_unwind(AssertUnwindSafe(|| parse(&bytes[..cut]))).is_err() {
                self.failures.push(format!("{what}: cut to {cut} bytes"));
            }
        }
        if bytes.is_empty() {
            return;
        }
        let mut rng = Lcg::for_name(what);
        for flip in 0..FLIPS {
            let at = rng.below(bytes.len());
            let [to, ..] = (rng.next() >> 16).to_le_bytes();
            let was = buf[at];
            buf[at] = to;
            if catch_unwind(AssertUnwindSafe(|| parse(&buf))).is_err() {
                self.failures.push(format!(
                    "{what}: flip {flip}, byte {at} {was:#04x} -> {to:#04x}"
                ));
            }
            buf[at] = was;
        }
    }

    /// Fails the test with every panicking version listed, so one run says
    /// all of what is wrong rather than the first thing.
    fn assert_clean(&self, game: &str) {
        assert!(
            self.failures.is_empty(),
            "{game}: {} damaged inputs made a reader panic:\n  {}",
            self.failures.len(),
            self.failures.join("\n  ")
        );
    }
}

/// How many items of one kind to walk, spread over the ids present rather
/// than the first few, so the sample crosses the corpus.
const SAMPLE: usize = 6;

fn sample(ids: Vec<usize>) -> Vec<usize> {
    if ids.len() <= SAMPLE {
        return ids;
    }
    (0..SAMPLE).map(|i| ids[i * ids.len() / SAMPLE]).collect()
}

/// A loose file the game ships, if this copy has it: the sound drivers, the
/// banks and the font reference table are needed to play, not to open.
fn loose(dir: &Path, name: &str) -> Option<Vec<u8>> {
    find_ci(dir, name).map(|p| std::fs::read(p).expect("a shipped file reads"))
}

#[test]
fn dunkle_schatten_2_damaged_is_refused_not_crashed_on() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut report = Report::default();

    // The containers, header by header.
    for entry in std::fs::read_dir(&dir)
        .expect("the directory lists")
        .flatten()
    {
        let path = entry.path();
        if !rsc::is_container_name(&path) {
            continue;
        }
        let bytes = std::fs::read(&path).expect("a container reads");
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        report.walk(&name, &bytes, 0x40, |b| {
            let _ = rsc::Rsc::from_bytes(b.to_vec(), name.to_string());
        });
    }

    // A sample of every kind of item, through the reader that decodes it.
    let bank = Bank::open_dir(&dir).expect("the containers open");
    let ids = |kind: Kind| sample(bank.present(kind).into_iter().map(|(_, id)| id).collect());
    let item = |kind: Kind, id: usize| {
        bank.item(kind, id)
            .expect("the index reads")
            .expect("a present slot has bytes")
    };
    for id in ids(Kind::Gfx8) {
        report.walk(&format!("gfx8 {id}"), item(Kind::Gfx8, id), 0x320, |b| {
            let _ = m32::Sprite::parse(b);
        });
    }
    for id in ids(Kind::Text) {
        report.walk(&format!("text {id}"), item(Kind::Text, id), 0x40, |b| {
            let _ = m32::text::parse(b);
        });
    }
    for id in ids(Kind::Block) {
        report.walk(&format!("block {id}"), item(Kind::Block, id), 0x80, |b| {
            let _ = Song::parse(b);
        });
    }
    for id in ids(Kind::Font) {
        report.walk(&format!("font {id}"), item(Kind::Font, id), 0x40, |b| {
            let _ = m32::font::parse(b);
        });
    }
    for id in ids(Kind::Script) {
        report.walk(&format!("script {id}"), item(Kind::Script, id), 0x60, |b| {
            let _ = m32::ScrModule::parse(b);
        });
    }

    // The engine binary, whose kernel table is read out of the relocated
    // image, and the loose files the sound and the text need.
    let exe = std::fs::read(game_file(&dir, "ENGINE.EXE")).expect("ENGINE.EXE reads");
    report.walk("ENGINE.EXE", &exe, 0x100, |b| {
        if let Ok(img) = m32::le::Image::parse(b) {
            let _ = m32::le::kernel_words(&img);
        }
    });
    if let Some(bytes) = loose(&dir, "HMIMDRV.386") {
        report.walk("HMIMDRV.386", &bytes, 0x40, |b| {
            let _ = DriverArchive::parse(b);
        });
    }
    for bank_name in ["MELODIC.BNK", "DRUM.BNK"] {
        if let Some(bytes) = loose(&dir, bank_name) {
            report.walk(bank_name, &bytes, 0x40, |b| {
                let _ = InstrumentBank::parse(b);
            });
        }
    }
    if let Some(bytes) = loose(&dir, "000.FRT") {
        report.walk("000.FRT", &bytes, 0x20, |b| {
            let _ = FontRefTable::parse(b);
        });
    }
    report.assert_clean("Dunkle Schatten 2");
}

/// The 16-bit walk, the same for the five games: the volumes, a sample of
/// every segment's items, and the player binary the kernel is read out of.
fn walk_m16(game: &str, dir: &Path, exe: &str) {
    let mut report = Report::default();

    // The container's volumes. Damage goes into one volume at a time while
    // the others stay whole, which is what a bad copy of one floppy is.
    let mut volumes = Vec::new();
    for n in 1.. {
        let Some(path) = find_ci(dir, &format!("DATA.-{n}-")) else {
            break;
        };
        volumes.push(std::fs::read(path).expect("a volume reads"));
    }
    for v in 0..volumes.len() {
        let whole = volumes.clone();
        report.walk(&format!("DATA.-{}-", v + 1), &volumes[v], 0x40, |b| {
            let mut damaged = whole.clone();
            damaged[v] = b.to_vec();
            let _ = Container::from_volumes(damaged, game.to_string());
        });
    }

    let c = Container::open_dir(dir).expect("the container opens");
    let ids = |seg: Segment| sample(c.present(seg));
    let item = |seg: Segment, id: usize| {
        c.item(seg, id)
            .expect("the index reads")
            .expect("a present slot has bytes")
    };
    for id in ids(Segment::Gfx) {
        report.walk(&format!("gfx {id}"), item(Segment::Gfx, id), 0x10, |b| {
            let _ = m16::Sprite::parse(b);
        });
    }
    for id in ids(Segment::Blk) {
        report.walk(&format!("blk {id}"), item(Segment::Blk, id), 0x40, |b| {
            let _ = Plx::parse(b);
        });
    }
    for id in ids(Segment::Scr) {
        report.walk(&format!("scr {id}"), item(Segment::Scr, id), 0x40, |b| {
            let _ = m16::ScrModule::parse(b);
        });
    }
    for id in ids(Segment::Fnt) {
        report.walk(&format!("fnt {id}"), item(Segment::Fnt, id), 0x20, |b| {
            let _ = m16::font::parse(b);
        });
    }
    for id in ids(Segment::Frt) {
        report.walk(&format!("frt {id}"), item(Segment::Frt, id), 0x20, |b| {
            let _ = FontRefTable::parse(b);
        });
    }
    for id in ids(Segment::Txt) {
        report.walk(&format!("txt {id}"), item(Segment::Txt, id), 0x40, |b| {
            let _ = m16::text::parse(b);
        });
    }

    let bytes = std::fs::read(game_file(dir, exe)).expect("the player reads");
    report.walk(exe, &bytes, 0x40, |b| {
        if let Ok(img) = mz::Image::parse(b.to_vec()) {
            let words = mz::kernel_words(&img);
            let _ = mz::binding_of(&img, &words);
            let _ = mz::skips_empty_areas(&img, &words);
            let _ = mz::croute_defaults_shrink(&img, &words);
            let _ = mz::newsetdesc_capped(&img, &words);
            let _ = mz::croute_smooths_headings(&img, &words);
        }
    });
    if let Some(bytes) = loose(dir, "GFX.INF") {
        report.walk("GFX.INF", &bytes, 0x10, |b| {
            let _ = GfxInf::parse(b);
        });
    }
    report.assert_clean(game);
}

#[test]
fn die_enviro_kids_damaged_is_refused_not_crashed_on() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    walk_m16("Die Enviro-Kids greifen ein", &dir, "ENVIRO.EXE");
}

#[test]
fn jeff_jet_damaged_is_refused_not_crashed_on() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    walk_m16("Jeff Jet", &dir, "HPPLAY.EXE");
}

#[test]
fn hilfe_fuer_amajambere_damaged_is_refused_not_crashed_on() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    walk_m16("Hilfe für Amajambere", &dir, "BMZ.EXE");
}

#[test]
fn victor_loomes_damaged_is_refused_not_crashed_on() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    walk_m16("Victor Loomes", &dir, "LL.EXE");
}

#[test]
fn falsches_spiel_mit_eddie_m_damaged_is_refused_not_crashed_on() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    walk_m16("Falsches Spiel mit Eddie M.", &dir, "STERN.EXE");
}

/// The walk itself is checked on a reader that is known to panic, so that a
/// clean report above means the readers held and not that nothing was asked.
#[test]
fn the_walk_reports_a_reader_that_panics() {
    let mut report = Report::default();
    report.walk("a panicking reader", &[1, 2, 3, 4], 2, |b| {
        assert!(b.len() != 2, "the reader chokes on two bytes");
    });
    assert_eq!(report.failures, vec!["a panicking reader: cut to 2 bytes"]);
    let mut quiet = Report::default();
    quiet.walk("a reader that answers", &[1, 2, 3, 4], 2, |_| {});
    quiet.assert_clean("nothing");
}
