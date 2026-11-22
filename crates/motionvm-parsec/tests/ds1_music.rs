//! The game's sound through the front door: every song of `PSMP2L.DAT`
//! through the game's own `MUSADL.DRV`, and the music a window opens.
//!
//! These need the original files; point `MOTIONVM_GAMEDATA_DS1` at the
//! directory with `RESOURCE/`, or they skip themselves.
//!
//! The byte-for-byte check of the register stream is not here — it runs
//! against OPL captures of the original under DOSBox-X, outside this
//! repository: module 0 as the title program plays it agrees for all 4296
//! writes the capture holds, and module 1 as the Zimmer plays it for the
//! first 1705, up to the room's exit fade. What is here is what can be
//! asked of the shipped files alone, and the digest of every song's
//! stream, which holds the streams to what they were — and the digests of
//! the DMA bytes `DMABLAST.DRV`'s mixer makes of every effect and of every
//! song's digital voices, anchored to the driver's own routines run on the
//! same plays.
//!
//! The game this file drives is Dunkle Schatten (PARSEC).

use motionvm_parsec::PARSEC;
use motionvm_parsec_formats::detect::resource_dir;
use motionvm_parsec_formats::{P2l, find_ci};
use motionvm_parsec_testutil::{Digests, digest::Digest, gamedata_ds1};
use motionvm_playable::{Button, Family};
use motionvm_psm::mixer::{BLOCK, Irq};
use motionvm_psm::{DmaDriver, Driver, Mdh, Mixer, Music, Plx, Sample, Sequencer};
use std::sync::Arc;

/// This game's table of reference digests.
fn digests() -> Digests {
    Digests::of(env!("CARGO_MANIFEST_DIR"), "ds1")
}

/// A file of the install's `RESOURCE/`.
fn resource(dir: &std::path::Path, name: &str) -> Vec<u8> {
    let res = resource_dir(dir);
    std::fs::read(find_ci(&res, name).unwrap_or_else(|| panic!("{name}"))).expect(name)
}

/// Every module's song, as the Ad Lib path hands it over, played endlessly
/// for 20 000 ticks: notes keyed, nothing written past the OPL2's bank, the
/// song still playing — and the whole stream the same, write for write, as
/// the digest the table holds.
#[test]
fn every_song_plays_notes_and_digests_as_it_did() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let library = P2l::parse(resource(&dir, "PSMP2L.DAT")).expect("PSMP2L.DAT");
    let driver = motionvm_parsec_formats::p2l::ad_lib_driver(&resource(&dir, "PSMDRV.DAT"))
        .expect("MUSADL.DRV");
    let driver = Driver::parse(&driver).expect("the driver's tables");
    for n in 0..library.modules.len() {
        let song = Plx::from_section(library.song(n).expect("a song")).expect("a PLX");
        let mut seq = Sequencer::new(&driver, song, -1);
        let mut writes = Sequencer::install_writes(&driver);
        for _ in 0..20_000 {
            seq.tick(&mut writes);
        }
        assert!(
            writes.iter().all(|w| w.address() < 0x100),
            "module {n} wrote past the OPL2 bank"
        );
        assert!(
            writes
                .iter()
                .any(|w| (0xb0..=0xb8).contains(&w.reg) && w.value & 0x20 != 0),
            "module {n} never keyed a note"
        );
        assert!(seq.playing(), "module {n} stopped although it loops");
        let mut d = Digest::new();
        for w in &writes {
            d.byte(w.bank).byte(w.reg).byte(w.value);
        }
        digests().check(&format!("module_{n:02}"), d.value());
    }
}

/// A window that opens the music hears the title program's menu: module 0
/// sounds once the menu has faded in.
#[test]
fn the_title_menu_is_heard_through_the_front_door() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let rate = 44_100;
    let mut game = PARSEC.open(&dir).expect("open");
    let mut source = game
        .open_music(rate)
        .expect("the music comes up")
        .expect("a game with music");
    game.start().expect("start");
    // A click skips the intro to the menu.
    game.button(Button::Left, true);
    game.button(Button::Left, false);
    let mut buf = vec![0i16; 2 * 4410];
    let mut peak = 0;
    for _ in 0..600 {
        game.step().expect("a step");
        source.fill(&mut buf);
        peak = peak.max(buf.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0));
    }
    assert!(peak > 500, "the menu is all but silent: peak {peak}");
}

/// A copy without the music library still plays, and says why there is no
/// sound.
#[test]
fn a_copy_without_the_music_library_says_why_sound_is_off() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let copy = motionvm_parsec_testutil::saves_dir("ds1-no-music");
    let res = copy.join("RESOURCE");
    std::fs::create_dir_all(&res).expect("a RESOURCE directory");
    for exe in ["XP00.EXE", "XP01.EXE"] {
        let from = find_ci(&dir, exe).expect(exe);
        std::fs::copy(from, copy.join(exe)).expect(exe);
    }
    for entry in std::fs::read_dir(resource_dir(&dir)).expect("RESOURCE/") {
        let path = entry.expect("an entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !name.eq_ignore_ascii_case("PSMP2L.DAT") {
            std::fs::copy(&path, res.join(name)).expect(name);
        }
    }
    let mut game = PARSEC
        .open(&copy)
        .expect("the game opens without its music");
    let Err(e) = game.open_music(44_100) else {
        panic!("music without its library");
    };
    assert_eq!(e.to_string(), "RESOURCE/PSMP2L.DAT: not found");
}

/// `DMABLAST.DRV`'s mixer as the game installs it: six effect channels, the
/// default volume `50h` and then `3Ch` (`0x5C15`, `0x4E85`).
fn mixer(dir: &std::path::Path) -> Mixer {
    let file = motionvm_parsec_formats::p2l::driver(
        &resource(dir, "PSMDRV.DAT"),
        motionvm_parsec_formats::p2l::DMABLAST,
    )
    .expect("DMABLAST.DRV");
    let driver = DmaDriver::parse(&file).expect("the mixer's tables");
    let mut m = Mixer::new(&driver, 6);
    m.set_volume(0x50);
    m.set_volume(0x3C);
    m
}

/// The blocks a start from rest mixed — the first half, then the second
/// (`0x1BC7`–`0x1BD6`) — folded in, if the last entry started the mixer.
fn started(m: &mut Mixer, d: &mut Digest) {
    if m.take_start() {
        two_blocks(m, d);
    }
}

fn two_blocks(m: &Mixer, d: &mut Digest) {
    d.bytes(&m.buffer()[..BLOCK]);
    let h = m.half();
    d.bytes(&m.buffer()[h..h + BLOCK]);
}

/// One interrupt, and the block it mixed folded in.
fn irq(m: &mut Mixer, d: &mut Digest) {
    match m.irq() {
        Irq::Next => {
            let h = m.half();
            d.bytes(&m.buffer()[h..h + BLOCK]);
        }
        Irq::Restart => two_blocks(m, d),
        Irq::End => {}
    }
}

/// Every effect of table A's bound, each played alone into a freshly
/// installed mixer until the transfer ends: the DMA bytes of every block,
/// held as one digest per effect. The lines are anchored to the driver's own
/// routines, run on the same plays in an 8086 sandbox outside this
/// repository.
#[test]
fn every_effect_mixes_as_the_driver_mixes_it() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let library = P2l::parse(resource(&dir, "PSMP2L.DAT")).expect("PSMP2L.DAT");
    assert_eq!(library.bounds[0], 21);
    for row in 0..library.bounds[0] {
        let sample = Arc::new(Sample::parse(&library.sample(row).expect("a sample")).expect("SM8"));
        let mut m = mixer(&dir);
        let mut d = Digest::new();
        assert_eq!(m.play(&sample), Some(1), "effect {row} takes channel 1");
        started(&mut m, &mut d);
        let mut blocks = 0;
        while m.running() {
            irq(&mut m, &mut d);
            blocks += 1;
            assert!(blocks < 10_000, "effect {row} never ends");
        }
        irq(&mut m, &mut d);
        assert!(blocks > 2, "effect {row} is longer than two blocks");
        digests().check(&format!("effect_{row:02}"), d.value());
    }
}

/// Every module's digital voices, as the DAC path assembles the module
/// (`0x14186`) and the play routine hands it over (`0x6080`): started at
/// full, one row stepped at a time on the `MUSADL.DRV` side's countdown, the
/// mixer's interrupts falling in between by the row's length against a
/// block's (the song's head tempo; 128 bytes of 48 µs), until the stream
/// ends and the transfer with it. The DMA bytes of every block are one
/// digest per module, anchored like the effects'.
#[test]
fn every_song_s_digital_voices_mix_as_the_driver_mixes_them() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let library = P2l::parse(resource(&dir, "PSMP2L.DAT")).expect("PSMP2L.DAT");
    for n in 0..library.modules.len() {
        let module = library.module(n).expect("a module");
        let word = |at: usize| u32::from_le_bytes(module[at..at + 4].try_into().unwrap());
        let samples = (0..8)
            .map(|i| match usize::try_from(word(0x14 + 4 * i)).unwrap() {
                0 => None,
                off => Some(Arc::new(Sample::parse(&module[off..]).expect("SM8"))),
            })
            .collect();
        let mdh = Mdh::parse(&module[0x38..]).expect("MDH");
        let music = Arc::new(Music::new(mdh, samples).expect("the digital part"));
        let plx = &module[usize::try_from(word(0x10)).unwrap()..];
        let row_len = u64::from(plx[4])
            * u64::from(u16::from_le_bytes([plx[5], plx[6]]).max(0x200))
            * 1_000_000;
        let block_len = 128 * 48 * 1_193_182u64;
        let mut m = mixer(&dir);
        let mut d = Digest::new();
        m.set_music(Some(music));
        let mut rows = m.music_start(0, 0x100);
        started(&mut m, &mut d);
        let mut clock = 0u64;
        let mut steps = 0;
        loop {
            rows = rows.wrapping_sub(1);
            if rows == 0 {
                rows = m.music_step(0);
                steps += 1;
                if rows == 0 {
                    break;
                }
            }
            clock += row_len;
            while clock >= block_len {
                clock -= block_len;
                irq(&mut m, &mut d);
            }
            assert!(steps < 20_000, "module {n}: the stream never ends");
        }
        let mut blocks = 0;
        while m.running() {
            irq(&mut m, &mut d);
            blocks += 1;
            assert!(blocks < 10_000, "module {n}: the transfer never ends");
        }
        irq(&mut m, &mut d);
        assert!(steps > 10, "module {n}: {steps} steps");
        digests().check(&format!("voices_{n:02}"), d.value());
    }
}
