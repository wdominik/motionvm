//! End-to-end checks against the files Dunkle Schatten 2 ships.
//!
//! These need the original files. Point `MOTIONVM_GAMEDATA_DS2` at the directory holding
//! `001.RSC` etc.; the tests skip themselves if it is missing, so the crate
//! still builds and tests cleanly without the game.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_formats::m32::{Kind, ScrModule, Sprite, rsc::Bank};
use motionvm_formats::{Palette, font};
use motionvm_testutil::{game_file, gamedata_ds2};

macro_rules! bank_or_skip {
    () => {
        match gamedata_ds2() {
            Some(dir) => Bank::open_dir(&dir).expect("open RSC banks"),
            None => {
                eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
                return;
            }
        }
    };
}

#[test]
fn containers_parse_and_indices_are_consistent() {
    let bank = bank_or_skip!();
    assert_eq!(bank.banks().len(), 3, "expected 001/002/003.RSC");
    for r in bank.banks() {
        assert_eq!(r.slot_count(Kind::Gfx8), 5000);
        assert_eq!(r.slot_count(Kind::Palette), 200);
    }
    let counts: Vec<(Kind, usize)> = Kind::ALL
        .iter()
        .map(|&k| (k, bank.present(k).len()))
        .collect();
    for (k, n) in &counts {
        eprintln!("{:>8}: {n}", k.name());
    }
    assert_eq!(counts[0].1, 1678, "gfx8 item count");
    assert_eq!(
        bank.present(Kind::Gfx16).len(),
        0,
        "no hicolor sprites in this game"
    );
}

/// The decisive test for the LZW codec: every sprite must decompress to exactly
/// the size its own header declares, and the inner width*height must agree.
#[test]
fn every_sprite_decodes_to_its_declared_size() {
    let bank = bank_or_skip!();
    let ids = bank.present(Kind::Gfx8);
    assert!(!ids.is_empty());
    let mut pixels_total = 0usize;
    for (bank_ix, id) in ids {
        let item = bank
            .item(Kind::Gfx8, id)
            .unwrap()
            .expect("present slot has data");
        let sprite = Sprite::parse(item)
            .unwrap_or_else(|e| panic!("sprite {id} (bank {bank_ix}) failed: {e}"));
        assert_eq!(
            sprite.pixels.len(),
            sprite.width as usize * sprite.height as usize,
            "sprite {id} pixel count"
        );
        assert!(
            sprite.width > 0 && sprite.height > 0,
            "sprite {id} has a zero dimension"
        );
        pixels_total += sprite.pixels.len();
    }
    eprintln!("decoded {pixels_total} pixels");
}

#[test]
fn palettes_are_768_bytes_of_6bit_values() {
    let bank = bank_or_skip!();
    for (_, id) in bank.present(Kind::Palette) {
        let item = bank.item(Kind::Palette, id).unwrap().unwrap();
        assert_eq!(item.len(), Palette::BYTES, "palette {id} length");
        assert!(
            item.iter().all(|&v| v <= 0x3f),
            "palette {id} has non-6-bit values"
        );
    }
}

#[test]
fn text_tables_parse_and_contain_german() {
    let bank = bank_or_skip!();
    let mut total = 0usize;
    let mut umlauts = 0usize;
    for (_, id) in bank.present(Kind::Text) {
        let item = bank.item(Kind::Text, id).unwrap().unwrap();
        let t =
            motionvm_formats::m32::text::parse(item).unwrap_or_else(|e| panic!("text {id}: {e}"));
        total += t.strings.len();
        umlauts += t
            .strings
            .iter()
            .filter(|s| s.contains(['ä', 'ö', 'ü', 'Ä', 'Ö', 'Ü', 'ß']))
            .count();
    }
    eprintln!("{total} strings, {umlauts} with umlauts");
    assert!(total > 1000, "expected a few thousand strings, got {total}");
    // If CP437 decoding were wrong the umlauts would come out as box-drawing
    // characters and this would be zero.
    assert!(
        umlauts > 100,
        "CP437 decoding looks wrong: only {umlauts} umlaut strings"
    );
}

#[test]
fn script_modules_parse_and_sizes_add_up() {
    let bank = bank_or_skip!();
    let ids = bank.present(Kind::Script);
    assert!(!ids.is_empty());
    let mut empty = 0;
    let mut past = 0;
    for (_, id) in ids {
        let item = bank.item(Kind::Script, id).unwrap().unwrap();
        let m = ScrModule::parse(item).unwrap_or_else(|e| panic!("script {id}: {e}"));
        assert_eq!(m.module as usize, id, "module number should match its slot");
        assert_eq!(m.second_area_len, 16004, "second region size");
        // The two regions must fit inside the item, and the field at 0x1c must
        // be the authoring capacity rather than a size — that is what makes DP
        // the only usable split point.
        assert!(
            0x30 + m.mem_len() + m.second_area_len <= item.len(),
            "module {id}: {} + {} does not fit in {}",
            m.mem_len(),
            m.second_area_len,
            item.len()
        );
        assert_eq!(
            m.declared_mem_len, 0xfff0,
            "module {id}: 0x1c should be the capacity"
        );
        empty += usize::from(m.entries.is_empty());
        // Every word's body must stay inside the module.
        for e in &m.entries {
            assert!(
                e.body_offset() + e.body.len() * 4 <= item.len(),
                "module {id}: word {} runs past the module",
                e.name
            );
        }
        // Words genuinely do live past 0x50 + 16004: the second region is
        // appended *after* module memory, not carved out of it, so treating its
        // size as the dictionary's hid most of the game's code from the
        // disassembler while the interpreter, which calls by address, ran it
        // all along.
        past += m
            .entries
            .iter()
            .filter(|e| e.body_offset() >= 0x50 + m.second_area_len)
            .count();
        // And no word *ends* past DP either, which is where module memory
        // stops. Checking the end and not just the start is the point: every
        // word begins well inside, so only the *last* word of a module can run
        // on into the second region — and there it picks up cells that are not
        // code. In module 2 that gives `2OVER` ten extra cells, among them a
        // call to `0:0x84`; in module 101 it invents a closing `EXIT` for
        // `LD_TASCHE`, which is a constant and needs none — `_PutConst`
        // (0x62484) returns through 0x611df, where `_PutLit` only steps the
        // instruction pointer on.
        for e in &m.entries {
            assert!(
                e.body_offset() + e.body.len() * 4 <= 0x30 + m.mem_len(),
                "module {id}: {} runs past the end of module memory",
                e.name
            );
        }
    }
    assert!(
        past > 100,
        "only {past} words past the declared area; the walk stops too early"
    );

    // Three modules define no words. Two of them (123 and 130) are entirely
    // empty and come out at exactly the 16084 bytes the original compiler
    // produces for a module with nothing in it — independent confirmation of
    // where the dictionary ends. The third (10) carries only data in its tail.
    assert_eq!(empty, 3, "expected exactly three modules without words");
}

#[test]
fn standalone_files_match_their_in_container_twins() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    // 000.PAL on disk is a plain palette.
    let pal = std::fs::read(game_file(&dir, "000.PAL")).unwrap();
    assert_eq!(pal.len(), Palette::BYTES);
    assert!(pal.iter().all(|&v| v <= 0x3f));

    // 000.FRT maps characters to glyph indices; A-Z must be contiguous.
    let frt =
        font::FontRefTable::parse(&std::fs::read(game_file(&dir, "000.FRT")).unwrap()).unwrap();
    let a = frt.glyph_for(b'A').expect("font has 'A'");
    for (i, ch) in (b'A'..=b'Z').enumerate() {
        assert_eq!(
            frt.glyph_for(ch),
            Some(a + i as u16),
            "glyph run broken at {}",
            ch as char
        );
    }

    // 002.SCR / 011.SCR on disk are the same format as the container's scripts.
    let m = ScrModule::parse(&std::fs::read(game_file(&dir, "011.SCR")).unwrap()).unwrap();
    assert_eq!(m.module, 11);
    let names: Vec<&str> = m.entries.iter().map(|e| e.name.as_str()).collect();
    assert!(
        names.contains(&"STIFT"),
        "inventory words missing: {names:?}"
    );
}

#[test]
fn fonts_decode_and_spell_the_alphabet() {
    let bank = bank_or_skip!();
    let dir = gamedata_ds2().expect("checked by bank_or_skip");
    let refs =
        font::FontRefTable::parse(&std::fs::read(game_file(&dir, "000.FRT")).unwrap()).unwrap();

    let ids = bank.present(Kind::Font);
    assert_eq!(ids.len(), 9, "font count");
    let mut total_glyphs = 0;
    for (_, id) in ids {
        let item = bank.item(Kind::Font, id).unwrap().unwrap();
        let f =
            motionvm_formats::m32::font::parse(item).unwrap_or_else(|e| panic!("font {id}: {e}"));
        assert!(
            f.height > 0 && f.height <= 64,
            "font {id} height {}",
            f.height
        );
        assert!(!f.glyphs.is_empty(), "font {id} has no glyphs");
        for (i, g) in f.glyphs.iter().enumerate() {
            assert_eq!(g.height, f.height, "font {id} glyph {i} height");
            assert_eq!(
                g.bits.len(),
                g.stride() * f.height as usize,
                "font {id} glyph {i} bitmap size"
            );
        }
        total_glyphs += f.glyphs.len();
    }
    eprintln!("{total_glyphs} glyphs across 9 fonts");

    // The decisive check: read the glyphs the reference table assigns to 'A'
    // and 'B' out of the largest font and compare them against the shapes those
    // letters have. A wrong bit order still produces plausible-looking blobs, so
    // structure alone would not catch it — but 'A' has an apex on the top row
    // and two feet on the bottom, and 'I' is a bar, and those do not survive a
    // flip.
    let big = bank.item(Kind::Font, 8).unwrap().unwrap();
    let f = motionvm_formats::m32::font::parse(big).unwrap();
    let glyph = |c: u8| -> &font::Glyph { &f.glyphs[refs.glyph_for(c).expect("mapped") as usize] };

    let a = glyph(b'A');
    let ink_in_row = |g: &font::Glyph, y: u16| (0..g.width).filter(|&x| g.pixel(x, y)).count();
    let first = (0..a.height)
        .find(|&y| ink_in_row(a, y) > 0)
        .expect("'A' has ink");
    let last = (0..a.height)
        .rev()
        .find(|&y| ink_in_row(a, y) > 0)
        .expect("'A' has ink");
    assert!(
        ink_in_row(a, first) < ink_in_row(a, last),
        "'A' should be narrow at the top and wide at the bottom, got {} vs {}",
        ink_in_row(a, first),
        ink_in_row(a, last)
    );

    // 'I' is the narrowest letter in a sans face; 'W' among the widest.
    assert!(
        glyph(b'I').width < glyph(b'W').width,
        "'I' should be narrower than 'W'"
    );

    // A-Z map to consecutive glyphs, so a mis-sized table would break the run.
    for (i, ch) in (b'A'..=b'Z').enumerate() {
        let g = refs
            .glyph_for(ch)
            .unwrap_or_else(|| panic!("no glyph for {}", ch as char));
        assert_eq!(g as usize, i, "letter {} out of order", ch as char);
    }
}

#[test]
fn engine_executable_relocates_and_yields_the_kernel_word_table() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let img =
        motionvm_formats::m32::le::Image::open(game_file(&dir, "ENGINE.EXE")).expect("parse LE");

    // A clean parse of every page's fixup records is the strongest signal that
    // the walk stayed in sync; a desync would overshoot a page boundary and the
    // loader would have returned an error instead.
    assert_eq!(img.pages(), 170, "page count");
    assert_eq!(img.fixups_applied(), 17270, "32-bit fixups applied");
    assert_eq!(img.objects().len(), 4, "object count");
    assert!(
        img.objects()[0].executable(),
        "object 1 should be the code segment"
    );

    let words = motionvm_formats::m32::le::kernel_words(&img);
    assert_eq!(words.len(), 356, "kernel words");

    let by_name = |n: &str| words.iter().find(|w| w.name == n);

    // Table 0 holds the core words. Their order is what the compiled bytecode
    // refers to, so the anchors are worth pinning down exactly.
    let first = &words[0];
    assert_eq!(
        (first.table, first.index, first.name.as_str()),
        (0, 0, "##")
    );
    for (index, name) in [
        (7, "DUP"),
        (10, "DROP"),
        (17, "VAR"),
        (18, "CONST"),
        (19, "CREATE"),
    ] {
        let w = by_name(name).unwrap_or_else(|| panic!("kernel word {name} missing"));
        assert_eq!((w.table, w.index), (0, index), "{name} position");
    }

    // The internal words the compiler emits into threaded code; decoding the
    // bytecode is impossible without them.
    for name in [
        "_PutLit",
        "_PutAdr",
        "_PutConst",
        "_CheckIf",
        "_CheckElse",
        "_LoopStart",
    ] {
        assert!(by_name(name).is_some(), "compiler runtime {name} missing");
    }
    // Domain words live in the last table.
    let newscreen = by_name("NEWSCREEN").expect("NEWSCREEN missing");
    assert_eq!(newscreen.table, 2);

    let mut per_table = [0usize; 3];
    for w in &words {
        per_table[w.table] += 1;
    }
    assert_eq!(per_table, [101, 27, 228], "table sizes");
}

/// The Ad Lib banks the music is actually played with.
///
/// Both are the standard bank layout and the arithmetic closes exactly:
/// `28 + 128 * 12 + 128 * 30 = 5404`, which is their size to the byte. The two
/// counts in the header say **127** while there are 128 of each record, so they
/// are a highest index and not a count — sizing a reader from them loses the
/// last instrument, which in the melodic bank is `GUNSHOT`.
///
/// The third byte of a name entry means different things in the two files, and
/// that is not a guess: in `MELODIC.BNK` it is 1 for all 128 entries, and in
/// `DRUM.BNK` it carries MIDI note numbers — index 36 is note 35, `Kick`.
#[test]
fn the_adlib_banks_parse_and_name_their_instruments() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let melodic = std::fs::read(game_file(&dir, "MELODIC.BNK")).expect("MELODIC.BNK");
    let drum = std::fs::read(game_file(&dir, "DRUM.BNK")).expect("DRUM.BNK");
    assert_eq!(
        melodic.len(),
        5404,
        "the melodic bank is the size the layout implies"
    );
    assert_eq!(drum.len(), 5404);

    let m = motionvm_formats::m32::bnk::Bank::parse(&melodic).expect("melodic bank parses");
    assert_eq!(
        m.names.len(),
        128,
        "128 names, not the 127 the header claims"
    );
    assert_eq!(m.instruments.len(), 128);
    for (slot, want) in [(0usize, "PIANO1"), (48, "STRINGS"), (127, "GUNSHOT")] {
        assert_eq!(m.names[slot].name, want, "melodic slot {slot}");
        assert_eq!(
            m.names[slot].index as usize, slot,
            "the melodic bank is in program order"
        );
        assert_eq!(m.names[slot].key, 1, "every melodic entry carries key 1");
    }

    let d = motionvm_formats::m32::bnk::Bank::parse(&drum).expect("drum bank parses");
    assert_eq!(d.names.len(), 128);
    let kick = d
        .names
        .iter()
        .find(|n| n.name == "Kick")
        .expect("the drum bank names a Kick");
    assert_eq!(
        kick.key, 35,
        "the drum bank's third byte is a MIDI note, not a flag"
    );
    let distinct: std::collections::BTreeSet<_> =
        d.instruments.iter().map(|i| format!("{i:?}")).collect();
    assert_eq!(
        distinct.len(),
        34,
        "34 distinct drum patches fill the 128 slots"
    );

    // The one field a synthesizer cannot do without: every instrument has to
    // resolve through its name entry.
    for slot in 0..128 {
        assert!(m.instrument(slot).is_some(), "melodic slot {slot} resolves");
        assert!(d.instrument(slot).is_some(), "drum slot {slot} resolves");
    }
}

/// Every track of every song decodes to its end and no further.
///
/// The stream is `VLQ delta`, event, … up to `FF 2F`, and the sizes of the
/// events are what the sequencer's dispatcher (`0x98983`) consumes. A single
/// wrong size desynchronizes the walk, so "ends exactly on the end marker"
/// is a strong statement: 199 tracks, none of which may run into the next
/// track's bytes.
#[test]
fn every_song_decodes_to_its_end_of_track() {
    let bank = bank_or_skip!();
    let mut songs = 0;
    let mut tracks = 0;
    let mut notes = 0usize;
    let mut note_offs = 0usize;
    for id in 0..=25 {
        let item = bank
            .item(Kind::Block, id)
            .expect("read block")
            .expect("song present");
        let song = motionvm_formats::m32::hmi::Song::parse(item)
            .unwrap_or_else(|e| panic!("song {id}: {e}"));
        assert_eq!(song.tick_hz, 120, "song {id} asks for 120 ticks a second");
        songs += 1;
        for (t, track) in song.tracks.iter().enumerate() {
            let last = track.events.last().expect("a track has events");
            assert_eq!(
                last.event,
                motionvm_formats::m32::hmi::Event::EndOfTrack,
                "song {id} track {t} does not end on FF 2F"
            );
            for e in &track.events {
                match e.event {
                    motionvm_formats::m32::hmi::Event::NoteOn { .. } => notes += 1,
                    motionvm_formats::m32::hmi::Event::NoteOff { .. } => note_offs += 1,
                    _ => {}
                }
            }
            tracks += 1;
        }
    }
    assert_eq!(songs, 26, "the game ships 26 songs");
    assert_eq!(tracks, 186, "and 186 tracks between them");
    assert!(notes > 30_000, "only {notes} notes decoded");
    // The format has no note-offs at all — a note-on carries its own length.
    // Anything else here would mean the duration was misread as a note-off.
    assert_eq!(
        note_offs, 0,
        "the format carries no note-offs, got {note_offs}"
    );
}

/// Reads note-ons out of a standard MIDI file. Enough for `TEST.MID`.
fn smf_notes(d: &[u8]) -> std::collections::BTreeMap<u16, Vec<(u32, u8, u8)>> {
    fn vlq(d: &[u8], p: &mut usize) -> u32 {
        let mut v = 0;
        loop {
            let b = d[*p];
            *p += 1;
            v = (v << 7) | u32::from(b & 0x7f);
            if b & 0x80 == 0 {
                return v;
            }
        }
    }
    let tracks = u16::from_be_bytes([d[10], d[11]]);
    let mut out: std::collections::BTreeMap<u16, Vec<(u32, u8, u8)>> = Default::default();
    let mut p = 14;
    for _ in 0..tracks {
        let len = u32::from_be_bytes([d[p + 4], d[p + 5], d[p + 6], d[p + 7]]) as usize;
        let end = p + 8 + len;
        let mut q = p + 8;
        let (mut tick, mut status) = (0u32, 0u8);
        while q < end {
            tick += vlq(d, &mut q);
            if d[q] >= 0x80 {
                status = d[q];
                q += 1;
            }
            match status {
                0xff => {
                    q += 1;
                    let n = vlq(d, &mut q) as usize;
                    q += n;
                }
                0xf0 | 0xf7 => {
                    let n = vlq(d, &mut q) as usize;
                    q += n;
                }
                s if s & 0xf0 == 0x90 => {
                    let (note, vel) = (d[q], d[q + 1]);
                    q += 2;
                    if vel != 0 {
                        // 480 ticks a quarter at 126 BPM against the song's
                        // 120 Hz: 120 * (60/126) / 480 = 5/42.
                        out.entry(u16::from(s & 0x0f)).or_default().push((
                            tick * 5 / 42,
                            note,
                            vel,
                        ));
                    }
                }
                s if matches!(s & 0xf0, 0xc0 | 0xd0) => q += 1,
                _ => q += 2,
            }
        }
        p = end;
    }
    out
}

/// `TEST.HMI` and `TEST.MID` are the same music, and the decoder proves it.
///
/// The two files ship side by side for `SNDSETUP.EXE`'s hardware test, and the
/// `.MID` is the source the `.HMI` was converted from — same 480 division, 14
/// tracks against 13. That makes them an oracle for the whole event decoding:
/// a wrong delta, a missed note duration, a wrong size for any event class or a
/// mishandled running status all desynchronize the walk and show up here as a
/// different note.
///
/// Times are compared through 5/42, which is what 480 ticks a quarter at
/// 126 BPM comes to against the song's 120 Hz. That ratio does not divide
/// evenly, so the converter's rounding leaves single-tick differences — hence
/// the ±1. Nothing else is given any slack: the note and velocity sequences
/// must match exactly, channel by channel.
///
/// The channels are worth their own mention. The decoder takes them from the
/// **track header** (`+0x7B`), not from the status byte's low nibble, because
/// that is what the sequencer sends on (`0x98AEF`). That they line up with the
/// `.MID`'s nibbles is an independent check of that reading.
#[test]
fn the_hmi_test_song_matches_its_midi_source() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let song = motionvm_formats::m32::hmi::Song::parse(
        &std::fs::read(game_file(&dir, "TEST.HMI")).expect("TEST.HMI"),
    )
    .expect("TEST.HMI parses");
    assert_eq!((song.division, song.tick_hz), (480, 120));

    let mut ours: std::collections::BTreeMap<u16, Vec<(u32, u8, u8)>> = Default::default();
    for track in &song.tracks {
        for e in &track.events {
            if let motionvm_formats::m32::hmi::Event::NoteOn { note, velocity, .. } = e.event {
                ours.entry(track.channel)
                    .or_default()
                    .push((e.tick, note, velocity));
            }
        }
    }
    let theirs = smf_notes(&std::fs::read(game_file(&dir, "TEST.MID")).expect("TEST.MID"));

    assert_eq!(
        ours.keys().collect::<Vec<_>>(),
        theirs.keys().collect::<Vec<_>>(),
        "the two files use the same channels"
    );
    let total: usize = ours.values().map(Vec::len).sum();
    assert_eq!(total, 1961, "TEST.MID holds 1961 note-ons");

    for (channel, mine) in &ours {
        let other = &theirs[channel];
        assert_eq!(mine.len(), other.len(), "channel {channel}: note count");
        for (i, (a, b)) in mine.iter().zip(other).enumerate() {
            assert_eq!(
                (a.1, a.2),
                (b.1, b.2),
                "channel {channel} note {i}: pitch/velocity"
            );
            assert!(
                a.0.abs_diff(b.0) <= 1,
                "channel {channel} note {i}: tick {} against {}",
                a.0,
                b.0
            );
        }
    }
}

/// The three `.386` archives walk end to end.
///
/// There is no directory in the format: the only way to reach the last driver
/// is `next = here + 0x30 + image`, so a walk that lands anywhere but the last
/// byte of the file means the record header was read wrong. All three shipped
/// archives close exactly, and each yields exactly the count in its header —
/// which is what makes the reader checkable at all.
///
/// It does **not** see a wrong device id or a wrong `mem`; the second half
/// covers the ids the game actually asks for.
#[test]
fn the_driver_archives_walk_to_their_last_byte() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    for (name, count) in [("HMIMDRV.386", 8), ("HMIDET.386", 80), ("HMIDRV.386", 100)] {
        let bytes = std::fs::read(game_file(&dir, name)).unwrap_or_else(|_| panic!("{name}"));
        let arc = motionvm_formats::m32::DriverArchive::parse(&bytes)
            .unwrap_or_else(|e| panic!("{name} does not walk: {e}"));
        assert_eq!(arc.drivers.len(), count, "{name} driver count");
        assert_eq!(arc.name, "3", "{name} archive name");
        let end = arc.drivers.last().map(|d| d.at + d.image.len()).unwrap();
        assert_eq!(end, bytes.len(), "{name} chain ends at the file end");
        for d in &arc.drivers {
            assert!(
                d.mem as usize >= d.image.len(),
                "{name}: {} reserves too little",
                d.name
            );
        }
    }

    // The two ids the engine loads instrument banks for, and nothing else.
    let bytes = std::fs::read(game_file(&dir, "HMIMDRV.386")).expect("HMIMDRV.386");
    let arc = motionvm_formats::m32::DriverArchive::parse(&bytes).expect("walks");
    assert_eq!(
        arc.device(0xA002).map(|d| d.name.as_str()),
        Some("fmmidi.com"),
        "the OPL2 driver"
    );
    assert_eq!(
        arc.device(0xA009).map(|d| d.name.as_str()),
        Some("fmmidi3.com"),
        "the OPL3 driver"
    );
    assert_eq!(arc.device(0xA009).map(|d| d.image.len()), Some(14416));
}
