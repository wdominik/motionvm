//! The formats hold on Dunkle Schatten's own files.
//!
//! The readers are handed the shipped `RESOURCE/*.DAT` and the three
//! programs, and asked the questions the engine asks of them: every count
//! and every byte pinned below was read out of the game's files.
//!
//! They need the game's files — `MOTIONVM_GAMEDATA_DS1` — and skip without
//! them.
//!
//! The game this file drives is Dunkle Schatten (PARSEC).

use motionvm_parsec_formats::detect::{Title, identify, resource_dir};
use motionvm_parsec_formats::fgroup::{Dat, Slot, open_dat};
use motionvm_parsec_formats::{
    Bank, Font, HotspotTable, ObjectMap, P2l, Rib, Room, Unpacked, find_ci,
};
use motionvm_parsec_testutil::gamedata_ds1;

/// The install, or a skip.
macro_rules! ds1_or_skip {
    () => {
        match gamedata_ds1() {
            Some(dir) => dir,
            None => {
                eprintln!("skipping: no Dunkle Schatten gamedata directory");
                return;
            }
        }
    };
}

fn dat(dir: &std::path::Path, name: &str) -> Vec<u8> {
    let res = resource_dir(dir);
    std::fs::read(find_ci(&res, name).unwrap_or_else(|| panic!("{name}"))).expect(name)
}

/// Every program says where its own data segment is.
///
/// Turbo-C's entry code opens `mov dx, DGROUP` and twenty bytes later
/// `mov ds, dx`; the immediate is the segment `DS:xxxx` addresses
/// are relative to. The three programs open that way, so the
/// data segment is something to read rather than something to write down.
#[test]
fn every_program_names_its_own_data_segment() {
    // (program, DGROUP as an image offset)
    let want: &[(&str, usize)] = &[
        ("XP00.EXE", 0x0_e3a0),
        ("XP01.EXE", 0x2_1ef0),
        ("XP02.EXE", 0x0_1e20),
    ];
    let dir = ds1_or_skip!();
    for &(prog, ds) in want {
        let Some(path) = find_ci(&dir, prog) else {
            panic!("no {prog}");
        };
        let exe = Unpacked::parse(&std::fs::read(&path).expect(prog)).expect(prog);
        assert_eq!(exe.dgroup().unwrap(), ds, "{prog}");
    }
}

/// The game is recognized by the file its rooms live in.
#[test]
fn the_game_is_told_by_its_room_file() {
    let dir = ds1_or_skip!();
    assert_eq!(identify(&dir), Some(Title::DunkleSchatten));
}

/// Dunkle Schatten's room file is an FGROUP of six-resource RIBs, and each of the
/// six is independently optional.
///
/// 60 slots, the first the `NULLPTR` marker and every other one a RIB of
/// exactly six. The six are `[NOD, AN…, OMP, DLG, hotspots,
/// RDV1.00]`, and a slot fills in only what it needs: resource 1 opens on
/// the two bytes `AN`, resource 2 on `OMP\0`, resource 5 on `RDV1.00\0`,
/// and anything a slot does not use is the empty-slot literal instead.
///
/// Counted per index over the live slots: `[24, 25, 31, 28, 20, 28]`. So
/// **`DSROOM.DAT` is not 59 rooms**: 28 of its slots carry an RDV and the
/// rest are other things the same table holds.
#[test]
fn the_1994_room_files_are_fgroups_of_six_optional_resources() {
    let (file, slots, filled) = ("DSROOM.DAT", 60, [24, 25, 31, 28, 20, 28]);
    let dir = ds1_or_skip!();
    {
        let bytes = dat(&dir, file);
        let Dat::Fgroup(group) = open_dat(&bytes).expect(file) else {
            panic!("{file} is not an FGROUP");
        };
        assert_eq!(group.slots.len(), slots, "{file} slot count");
        assert!(matches!(group.slots[0], Slot::Null), "{file} slot 0");
        let mut counted = [0usize; 6];
        for (i, slot) in group.slots.iter().enumerate().skip(1) {
            let Slot::Rib(rib) = slot else {
                panic!("{file} slot {i} is not a RIB");
            };
            assert_eq!(rib.resources.len(), 6, "{file} slot {i}");
            for (j, blob) in rib.resources.iter().enumerate() {
                if blob.starts_with(motionvm_parsec_formats::NULLPTR) {
                    continue;
                }
                counted[j] += 1;
                match j {
                    1 => assert_eq!(&blob[..2], b"AN", "{file} slot {i} resource 1"),
                    3 => assert!(
                        Bank::looks_like(blob),
                        "{file} slot {i} resource 3 is not a dialogue bank"
                    ),
                    2 => assert_eq!(&blob[..4], b"OMP\0", "{file} slot {i} resource 2"),
                    5 => assert_eq!(
                        &blob[..8],
                        motionvm_parsec_formats::rdv::MAGIC,
                        "{file} slot {i} resource 5"
                    ),
                    _ => {}
                }
            }
        }
        assert_eq!(counted, filled, "{file} filled slots per resource");
    }
}

/// Every room composes a 320×128 playfield, and its hotspot table reads
/// with 46-byte records: 28 rooms.
#[test]
fn every_1994_room_composes_a_playfield_and_a_hotspot_table() {
    let (file, want_rooms) = ("DSROOM.DAT", 28);
    let dir = ds1_or_skip!();
    {
        let bytes = dat(&dir, file);
        let Dat::Fgroup(group) = open_dat(&bytes).expect(file) else {
            panic!("{file} is not an FGROUP");
        };
        let (mut rooms, mut spots) = (0usize, 0usize);
        for (i, slot) in group.slots.iter().enumerate() {
            let Slot::Rib(rib) = slot else { continue };
            let blob = rib.get(5).unwrap();
            if blob.starts_with(motionvm_parsec_formats::NULLPTR) {
                continue;
            }
            let room = Room::parse(blob).unwrap_or_else(|e| panic!("{file} slot {i} room: {e}"));
            assert_eq!(
                room.pixels.len(),
                320 * 128,
                "{file} slot {i} is not 320×128"
            );
            assert!(!room.layers.is_empty(), "{file} slot {i} has no layer");
            let table = HotspotTable::parse(rib.get(4).unwrap())
                .unwrap_or_else(|e| panic!("{file} slot {i} hotspots: {e}"));
            spots += table.spots.len();
            rooms += 1;
        }
        assert_eq!(rooms, want_rooms, "{file} rooms");
        assert!(
            spots > rooms,
            "{file}: only {spots} hotspots over {rooms} rooms"
        );
    }
}

/// Every map cell of every room names a 16×16 tile in its own layer's
/// table, and every room's object table reaches its objects.
///
/// The redraw takes a tile as entry `k − 1` of the layer's table and
/// nothing else, so a cell that named no tile there would draw garbage in
/// the original too. Counted: 89 layers with a map over the 28 rooms, and
/// no cell among them that misses.
#[test]
fn every_map_cell_names_a_tile_in_its_layers_table() {
    let dir = ds1_or_skip!();
    let bytes = dat(&dir, "DSROOM.DAT");
    let Dat::Fgroup(group) = open_dat(&bytes).expect("DSROOM.DAT") else {
        panic!("DSROOM.DAT is not an FGROUP");
    };
    let mut maps = 0usize;
    for (i, slot) in group.slots.iter().enumerate() {
        let Slot::Rib(rib) = slot else { continue };
        let blob = rib.get(5).unwrap();
        if blob.starts_with(motionvm_parsec_formats::NULLPTR) {
            continue;
        }
        let room = Room::parse(blob).unwrap_or_else(|e| panic!("slot {i}: {e}"));
        for (l, layer) in room.layers.iter().enumerate().filter(|(_, l)| l.kind == 1) {
            maps += 1;
            for &k in layer.cells.iter().filter(|&&k| k != 0) {
                let tile = &layer.cels[usize::from(k) - 1];
                assert_eq!(
                    (tile.width, tile.height),
                    (16, 16),
                    "slot {i} layer {l} cell value {k}"
                );
            }
        }
        assert!(
            room.overlays
                .iter()
                .filter(|o| o.bundle == 0)
                .all(|o| usize::from(o.cel_index) < room.object_cels.len()),
            "slot {i}: an object's cel lies past the table"
        );
    }
    assert_eq!(maps, 89, "layers with a map");
}

/// The intros and outros are FGROUPs and the music library a blob that
/// is neither.
///
/// Counted: `INTROS` 13 slots, `EXTROS` 4. `PSMP2L.DAT` is the one shipped
/// `.DAT` that is not a container at all.
#[test]
fn intros_extros_and_the_music_library_are_the_measured_shapes() {
    let dir = ds1_or_skip!();
    for (file, slots) in [("INTROS.DAT", 13), ("EXTROS.DAT", 4)] {
        let bytes = dat(&dir, file);
        let Dat::Fgroup(group) = open_dat(&bytes).unwrap_or_else(|e| panic!("{file}: {e}")) else {
            panic!("{file} is not an FGROUP");
        };
        assert_eq!(group.slots.len(), slots, "{file}");
    }
    let library = dat(&dir, "PSMP2L.DAT");
    assert!(open_dat(&library).is_err(), "PSMP2L is a container");
}

/// `PSMP2L.DAT`'s three tables as the header names them: 60 samples with
/// a bound of 21, 16 songs with a bound of 0, 16 module heads with a bound
/// of 16 — and every song packed, every sample and head stored.
///
/// Every module's song comes out of the Ad Lib path as a bare `PLX` of
/// exactly its row's size, with all nine channel offsets inside it, and
/// is the row of the module's own number; every module assembles to
/// exactly the length its row asks for, its song behind the head where
/// the head's `+10h` says.
#[test]
fn the_music_library_holds_sixteen_songs_and_their_modules() {
    let dir = ds1_or_skip!();
    let p = P2l::parse(dat(&dir, "PSMP2L.DAT")).expect("PSMP2L.DAT");
    assert_eq!(
        (p.samples.len(), p.songs.len(), p.modules.len()),
        (60, 16, 16)
    );
    assert_eq!(p.bounds, [21, 0, 16]);
    assert!(
        p.songs.iter().all(|e| e.stored < e.size),
        "every song packed"
    );
    assert!(p.samples.iter().all(|e| e.stored == e.size));
    assert!(p.modules.iter().all(|m| m.head.stored == m.head.size));
    for (n, row) in p.songs.iter().enumerate() {
        let song = p.song(n).unwrap_or_else(|e| panic!("song {n}: {e}"));
        assert_eq!(song.len(), row.size, "song {n} is its own row");
        assert_eq!(&song[..4], b"PLX\0", "song {n}");
        for c in 0..9 {
            let at = usize::from(u16::from_le_bytes([song[7 + 2 * c], song[8 + 2 * c]]));
            assert!(at < song.len(), "song {n} channel {c} at {at:#x}");
        }
        let m = p.module(n).unwrap_or_else(|e| panic!("module {n}: {e}"));
        assert_eq!(m.len(), p.modules[n].assembled, "module {n}");
        let plx = usize::try_from(u32::from_le_bytes(m[0x10..0x14].try_into().unwrap())).unwrap();
        assert_eq!(&m[plx..plx + song.len()], &song[..], "module {n}");
    }
}

/// `MUSADL.DRV`, the Ad Lib driver the program installs, is resource 5 of
/// `PSMDRV.DAT` (`0x5B7B`), and that resource is a RIB of its own: 4480
/// bytes once unpacked, the `MUS` 1.00 header with fifteen entries and
/// seven imports, the `NS` trailer at `0x1165`.
#[test]
fn the_ad_lib_driver_is_a_rib_inside_the_driver_file() {
    let dir = ds1_or_skip!();
    let Dat::Rib(rib) = open_dat(&dat(&dir, "PSMDRV.DAT")).expect("PSMDRV.DAT") else {
        panic!("PSMDRV.DAT is one RIB");
    };
    let inner = Rib::parse(rib.get(5).expect("resource 5")).expect("a RIB in the RIB");
    assert!(inner.raw, "one object, no table");
    let drv = &motionvm_parsec_formats::p2l::ad_lib_driver(&dat(&dir, "PSMDRV.DAT")).unwrap();
    assert_eq!(drv, &inner.decoded);
    assert_eq!(drv.len(), 4480);
    assert_eq!(&drv[..10], b"MUS\0\x00\x01\x0f\x00\x07\x00");
    assert_eq!(&drv[0x1165..0x1167], b"NS");
}

/// The font is resource 15 of `CMBMOU.DAT`: 194 records from `20h`, 122
/// of them with a bitmap, on a twelve-row pitch. Every record with a
/// bitmap advances the pen and every record without one has an advance
/// of 0, so the advance a record carries and the glyph it carries never
/// part.
#[test]
fn the_font_is_resource_fifteen_of_the_pointer_file() {
    let (file, index, bitmaps, height) = ("CMBMOU.DAT", 15, 122, 12);
    let dir = ds1_or_skip!();
    let rib = Rib::parse(&dat(&dir, file)).unwrap_or_else(|e| panic!("{file}: {e}"));
    let block = rib.get(index).unwrap_or_else(|| panic!("{file}[{index}]"));
    let font = Font::parse(block).unwrap_or_else(|e| panic!("{file}[{index}]: {e}"));
    for (i, glyph) in font.glyphs.iter().enumerate() {
        let rec = &block[0x0A + 8 * i..0x0A + 8 * i + 8];
        let advance = u16::from_le_bytes([rec[2], rec[3]]);
        assert_eq!(glyph.is_some(), advance != 0, "{file} record {i}");
    }
    assert_eq!(font.first, 0x20, "{file}");
    assert_eq!(font.glyphs.len(), 194, "{file}");
    assert_eq!(font.line_height, height, "{file}");
    assert_eq!(
        font.glyphs.iter().filter(|g| g.is_some()).count(),
        bitmaps,
        "{file} glyphs with a bitmap"
    );
}

/// The dialogues are kept in banks, one to a resource.
///
/// `XP01` reaches a dialogue by number through one of three tables and
/// complains `NULLDLG: D%03d_%03d.DCS` when the place is empty. The file
/// side of that is a nested directory whose empty places hold the literal
/// `NULLPTR\r\n` — so the counts below are *places*, and a bank with holes
/// in it is the normal case, not damage.
#[test]
fn dialogues_are_banked() {
    let places = [(34usize, 2usize), (22, 0)];
    let dir = ds1_or_skip!();
    let rib = match open_dat(&dat(&dir, "DIALOG.DAT")).expect("DIALOG.DAT") {
        Dat::Rib(rib) => rib,
        Dat::Fgroup(_) => panic!("DIALOG.DAT is a bare RIB"),
    };
    for (i, (count, holes)) in places.iter().enumerate() {
        let blob = rib.get(i).unwrap_or_else(|| panic!("resource {i}"));
        assert!(Bank::looks_like(blob), "DIALOG.DAT[{i}] is not a bank");
        let bank = Bank::parse(blob).unwrap_or_else(|e| panic!("[{i}]: {e}"));
        assert_eq!(bank.len(), *count, "places in DIALOG.DAT[{i}]");
        assert_eq!(
            bank.entries.iter().filter(|e| e.is_none()).count(),
            *holes,
            "NULLPTR places in DIALOG.DAT[{i}]"
        );
    }
}

/// The bank Dunkle Schatten reaches as number 999 holds the lines the
/// player says when a verb leads nowhere.
///
/// `XP01` loads `RESOURCE\DIALOG.DAT` at image `0x525F` and puts the
/// container's first pointer in `[1EF0h]`, which `0x0D5A9` addresses with
/// the owner number `3E7h`. These are the texts in it.
#[test]
fn dunkle_schattens_bank_999_holds_the_refusals() {
    let dir = ds1_or_skip!();
    let Dat::Rib(rib) = open_dat(&dat(&dir, "DIALOG.DAT")).expect("DIALOG.DAT") else {
        panic!("DIALOG.DAT is a bare RIB");
    };
    let bank = Bank::parse(rib.get(0).expect("resource 0")).expect("bank");

    let line = |i: usize| bank.get(i).expect("entry").lines[0].text.clone();
    // `\x03` and the byte after it are an ink control the original
    // substitutes while painting; the reader leaves them in place, so
    // they are pinned here rather than trimmed away.
    assert_eq!(line(6), "\u{3}fDas funktioniert leider nicht!\r");
    assert_eq!(line(8), "\u{3}fDas nutzt nichts.\r");
    assert_eq!(line(16), "\u{3}fIch kann nichts\nBesonderes erkennen.\r");
    assert!(bank.get(17).is_none(), "17 is a NULLPTR place");
    assert!(bank.get(18).is_none(), "18 is a NULLPTR place");
}

/// A room's own dialogues are resource 3 of its slot, and the first of
/// them is what the game opens with.
///
/// `XP01` loads them at image `0x7B14` out of the room container's fourth
/// pointer and counts them with the same routine bank 999 is counted
/// with. Location 2's handler then arms number **0** of that bank on
/// object 200 — so this text is the first thing Dunkle Schatten says.
#[test]
fn a_room_keeps_its_own_dialogues_and_the_bedroom_opens_the_game() {
    let dir = ds1_or_skip!();
    let Dat::Fgroup(group) = open_dat(&dat(&dir, "DSROOM.DAT")).expect("DSROOM.DAT") else {
        panic!("DSROOM.DAT is not an FGROUP");
    };
    let Slot::Rib(rib) = &group.slots[2] else {
        panic!("slot 2 is not a RIB");
    };
    let bank = Bank::parse(rib.get(3).expect("resource 3")).expect("bank");
    assert_eq!(bank.len(), 10, "places in the bedroom's bank");
    assert_eq!(bank.entries.iter().filter(|e| e.is_none()).count(), 1);
    // `\x01` and `\x03` plus a byte are controls the original acts on while
    // painting, so the line begins with two of them rather than with text.
    assert!(
        bank.get(0).expect("entry 0").lines[0]
            .text
            .starts_with("\u{1}\u{1}* Schnarch! *"),
        "the game opens with a snore"
    );
}

/// The Wald's slot and the 30 of the maze carry an object map, and no
/// other slot does: resource 2 of slots 23 and 30 to 59, 760 bytes each,
/// 26 entries under the flags `3Fh`, nothing past `+0Ch` of an entry. The
/// Wald's moves object `D5h` by (−176, 9) and leaves `D7h` on frame 0.
#[test]
fn the_forest_rows_carry_their_object_maps() {
    let dir = ds1_or_skip!();
    let Dat::Fgroup(group) = open_dat(&dat(&dir, "DSROOM.DAT")).expect("DSROOM.DAT") else {
        panic!("DSROOM.DAT is not an FGROUP");
    };
    let mut carriers = Vec::new();
    for (n, slot) in group.slots.iter().enumerate() {
        let Slot::Rib(rib) = slot else { continue };
        let Some(blob) = rib.get(2).filter(|b| b.starts_with(b"OMP\0")) else {
            continue;
        };
        assert_eq!(blob.len(), 760, "slot {n}");
        assert!(
            blob[0x20..]
                .chunks(0x1C)
                .all(|e| e[0x0C..].iter().all(|&b| b == 0)),
            "slot {n}"
        );
        let map = ObjectMap::parse(blob).expect("an object map");
        assert_eq!((map.flags, map.entries.len()), (0x3F, 26), "slot {n}");
        carriers.push(n);
    }
    let want: Vec<usize> = std::iter::once(23).chain(30..60).collect();
    assert_eq!(carriers, want);
    let Slot::Rib(rib) = &group.slots[23] else {
        panic!("slot 23 is not a RIB");
    };
    let wald = ObjectMap::parse(rib.get(2).expect("resource 2")).expect("the Wald's map");
    let d5 = wald.entries.iter().find(|e| e.id == 0xD5).expect("D5h");
    assert_eq!((d5.new_id, d5.cel, d5.dx, d5.dy), (0xD5, 0, -176, 9));
    let d7 = wald.entries.iter().find(|e| e.id == 0xD7).expect("D7h");
    assert_eq!((d7.new_id, d7.cel, d7.dx, d7.dy), (0xD7, 0, 0, 0));
}

/// Bank 2 is one text per inventory item, in the program's own order.
///
/// `XP01` puts `DIALOG.DAT`'s second pointer in `[1EE6h]` and addresses it
/// as bank **2**. It holds 22 places, and the row of item names at
/// `DS:1000h` that `0x0F5AC` reads with `cmp si, 17h` holds 22 names — the
/// same 22, in the same order, so place `n` describes item `n + 1`.
#[test]
fn dunkle_schattens_bank_2_describes_the_inventory() {
    let dir = ds1_or_skip!();
    let Dat::Rib(rib) = open_dat(&dat(&dir, "DIALOG.DAT")).expect("DIALOG.DAT") else {
        panic!("DIALOG.DAT is a bare RIB");
    };
    let bank = Bank::parse(rib.get(1).expect("resource 1")).expect("bank");
    assert_eq!(bank.len(), 22, "one place per item");

    let line = |i: usize| bank.get(i).expect("entry").lines[0].text.clone();
    assert!(line(0).contains("Werkzeugkasten"), "item 1 is the toolbox");
    assert!(line(4).contains("Schlüsselbund"), "item 5 is the keyring");
    assert!(
        line(12).contains("der in jedem Computerspiel vorkommt"),
        "item 13 is the key, and the game knows it"
    );
}

/// The parsec1994 verb slots are two lower than the file layout suggests.
///
/// A rule's handler names a run-time offset — `+1Eh` for `Benutze`,
/// `+22h` for `Betrachte`, `+26h` for `Sprich mit`, `+2Ah` for `Nimm` —
/// and the run-time record is the file record shifted four bytes on from
/// the verb mask (`0x0F5D9` reads the mask at `+10h` and the name at
/// `+12h`, against `+0Ch` and `+0Eh` in the file). So `texts[0]` is
/// `+16h` there, and the four slots are `[2]`, `[3]`, `[4]`, `[5]`.
///
/// Reading the shipped texts is what settles it: every one of these is
/// the answer its verb deserves, and none would be under the unshifted
/// mapping.
#[test]
fn the_1994_verb_slots_are_two_three_four_and_five() {
    let dir = ds1_or_skip!();
    let Dat::Fgroup(group) = open_dat(&dat(&dir, "DSROOM.DAT")).expect("DSROOM.DAT") else {
        panic!("DSROOM.DAT is not an FGROUP");
    };
    let Slot::Rib(rib) = &group.slots[2] else {
        panic!("slot 2 is not a RIB");
    };
    let table = HotspotTable::parse(rib.get(4).expect("resource 4")).expect("hotspots");
    let spot = |name: &str| {
        table
            .spots
            .iter()
            .find(|h| h.name == name)
            .unwrap_or_else(|| panic!("no {name} in the bedroom"))
    };

    let board = spot("Skateboard");
    assert!(
        board.texts[2].starts_with("Ähm - hier im Zimmer?"),
        "Benutze"
    );
    assert!(
        board.texts[3].starts_with("Mit dem Ding hab ich mir schon"),
        "Betrachte"
    );
    assert!(board.texts[5].starts_with("Ich laß es heute"), "Nimm");

    let shelf = spot("Regal");
    assert!(shelf.texts[3].starts_with("Au, weia!"), "Betrachte");
    assert!(shelf.texts[5].starts_with("Lieber nichts anrühren"), "Nimm");

    // And slot 0 is the name again, not an answer.
    assert_eq!(board.texts[0], board.name);
}

/// Every `AN` block the game ships parses: the room slots' resource 1
/// and `DIALOG.DAT`'s resource 2, the block the player animates from.
///
/// The block is a directory of entries, each checked for its `ANIS` magic
/// at `+74h` the way `14DDh:0293h` checks it (`0x15097`) and for the
/// animation offsets `0883h:0582h` reads at `+78h` (`0x08DED`). Counted:
/// the 25 room blocks hold 107 entries and 414 animations between them,
/// and the global block holds one — the player's, with the negated figure
/// id `−255` that `14DDh:0098h` looks for (`0x14E9D`). `0098h` searches
/// the room's block before the global one (`0x14E82`); an id is never
/// zero.
///
/// The Zimmer is the block to pin: the bed (200) with a
/// snore and a yawn, and the door (100) with three animations, the second
/// of which opens it — `05 | 17 08 00 | 8A 0A 03 FF 00 01 | 17 02 00 | 00`
/// after the `03 tt tt` every stream opens with, which is where the code
/// 2 the door's rule waits for comes from. The Küche's mother (200) is a
/// chain of two parts, the head and the body.
#[test]
fn every_animation_block_parses() {
    use motionvm_parsec_formats::Animations;
    let (file, want_blocks, want_entries, want_animations) = ("DSROOM.DAT", 25, 107, 414);
    let dir = ds1_or_skip!();
    {
        let bytes = dat(&dir, file);
        let Dat::Fgroup(group) = open_dat(&bytes).expect(file) else {
            panic!("{file} is not an FGROUP");
        };
        let (mut blocks, mut entries, mut animations) = (0usize, 0usize, 0usize);
        for (i, slot) in group.slots.iter().enumerate() {
            let Slot::Rib(rib) = slot else { continue };
            let Some(blob) = rib
                .get(1)
                .filter(|b| !b.starts_with(motionvm_parsec_formats::NULLPTR))
            else {
                continue;
            };
            let a = Animations::parse(blob).unwrap_or_else(|e| panic!("{file} slot {i}: {e}"));
            blocks += 1;
            entries += a.len();
            for e in a.entries() {
                assert_ne!(e.id, 0, "{file} slot {i}: an entry names something");
                assert!(e.count > 0, "{file} slot {i}: an entry with no animation");
                animations += usize::from(e.count);
            }
        }
        assert_eq!(
            (blocks, entries, animations),
            (want_blocks, want_entries, want_animations),
            "{file}"
        );

        let dialog = Rib::parse(&dat(&dir, "DIALOG.DAT")).expect("DIALOG.DAT");
        let global = Animations::parse(dialog.get(2).expect("resource 2"))
            .unwrap_or_else(|e| panic!("DIALOG.DAT resource 2: {e}"));
        assert_eq!(global.len(), 1, "one entry, the player's");
        assert_eq!(global.entry(0).unwrap().id, -255, "−(FFh)");
    }

    let bytes = dat(&dir, "DSROOM.DAT");
    let Dat::Fgroup(group) = open_dat(&bytes).expect("DSROOM.DAT") else {
        panic!("DSROOM.DAT is not an FGROUP");
    };
    let block = |slot: usize| {
        let Slot::Rib(rib) = &group.slots[slot] else {
            panic!("slot {slot}");
        };
        Animations::parse(rib.get(1).unwrap()).unwrap()
    };
    let zimmer = block(2);
    let ids: Vec<(i16, u8, u16)> = zimmer.entries().map(|e| (e.id, e.count, e.chain)).collect();
    assert_eq!(ids, [(200, 2, 0), (100, 3, 0)]);
    let open = zimmer.animation(1, 1).expect("the door's second animation");
    assert_eq!(
        &zimmer.bytes()[open..open + 17],
        [
            0x03, 0x1B, 0x01, 0x05, 0x17, 0x08, 0x00, 0x8A, 0x0A, 0x03, 0xFF, 0x00, 0x01, 0x17,
            0x02, 0x00, 0x00
        ]
    );
    let kueche = block(3);
    let ids: Vec<(i16, u8)> = kueche.entries().map(|e| (e.id, e.count)).collect();
    assert_eq!(ids, [(200, 4), (101, 3), (109, 3)]);
    let mother = kueche.entry(0).unwrap();
    let head = kueche.node(usize::from(mother.chain)).expect("the head");
    let body = kueche.node(usize::from(head.next)).expect("the body");
    assert_eq!((head.kind, head.id), (0, 200));
    assert_eq!((body.kind, body.id, body.next), (1, 201, 0));
    assert_eq!(
        (0..12)
            .map(|f| kueche.node_cel(&head, f))
            .collect::<Vec<_>>(),
        (0..12).map(Some).collect::<Vec<_>>(),
        "the head's twelve frames are its twelve cels"
    );
}

/// The people who walk besides the player: one 30h-byte record in nine of
/// the rooms, and those nine are exactly the rooms whose
/// walk-graph resource holds a second graph.
///
/// The record table is the room header's `+14h`, counted by its `+0Eh`
/// (the loader's live-byte loop, `0x7972`–`0x79B1`); the graphs are
/// `2Eh`-byte headers from `+4`, counted by the word at `+0` (`0724h:0000h`,
/// `0x7259`–`0x726A`). Every header carries mode `FFFFh` in the file —
/// only the snap `0A9Fh:0C59h` writes the 0 the walk routines want
/// (`0x0B7D0`).
#[test]
fn nine_rooms_carry_a_person_and_a_second_walk_graph() {
    use motionvm_parsec_formats::rdv::WalkGraph;
    let dir = ds1_or_skip!();
    let Dat::Fgroup(group) = open_dat(&dat(&dir, "DSROOM.DAT")).expect("DSROOM.DAT") else {
        panic!("DSROOM.DAT is not an FGROUP");
    };
    // Slot, id, kind, interval, phases, and graph 1's node ids.
    type Person = (usize, u8, u8, u8, u8, &'static [u16]);
    let want: [Person; 9] = [
        (6, 0x80, 6, 0x0F, 6, &[0x12C, 0x12D, 0x12E]),
        (8, 0xCA, 6, 0x11, 6, &[0xFED4, 0xFFFE, 0x12D, 0x12C]),
        (9, 0x83, 6, 0x0F, 6, &[0x12C, 0x12D, 0x12F, 0x130, 0x12E]),
        (11, 0xCC, 7, 0x11, 6, &[0xC9, 0xCA]),
        (13, 0xC9, 7, 0x0F, 6, &[0x12C, 0x12D, 0x190, 0x191]),
        (24, 0xCA, 6, 0x11, 7, &[0xFED4, 0xFFFE, 0x12D, 0x12C]),
        (25, 0xCA, 6, 0x11, 7, &[0x12D, 0xFFFE, 0x12C]),
        (
            26,
            0xCA,
            6,
            0x11,
            7,
            &[0xFED4, 0xFFFE, 0x12C, 0xFFFD, 0x12D],
        ),
        (
            27,
            0xC8,
            0x11,
            0x0C,
            6,
            &[0x12C, 0xFFFD, 0x12D, 0x190, 0x191, 0xFFFC, 0x192],
        ),
    ];
    let mut seen = Vec::new();
    for (slot, s) in group.slots.iter().enumerate() {
        let Slot::Rib(rib) = s else { continue };
        let Some(rdv) = rib.get(5).filter(|b| Room::looks_like(b)) else {
            continue;
        };
        let room = Room::parse(rdv).unwrap_or_else(|e| panic!("slot {slot}: {e}"));
        let nod = rib.get(0).unwrap_or(&[]);
        let graphs = WalkGraph::parse_all(nod);
        for k in 0..graphs.len() {
            assert_eq!(
                WalkGraph::file_mode(nod, k),
                Some(0xFFFF),
                "slot {slot} graph {k}: mode FFFFh in the file"
            );
        }
        match want.iter().find(|w| w.0 == slot) {
            Some(&(_, id, kind, interval, phases, nodes)) => {
                assert_eq!(room.movers.len(), 1, "slot {slot}: one record");
                let m = room.movers[0];
                assert_eq!(
                    (m.id, m.kind, m.interval, m.phases, m.base, m.bundle),
                    (id, kind, interval, phases, 1, 0),
                    "slot {slot}"
                );
                assert_eq!(graphs.len(), 2, "slot {slot}: two graphs");
                let ids: Vec<u16> = graphs[1].nodes.iter().map(|n| n.id).collect();
                assert_eq!(ids, nodes, "slot {slot}: graph 1");
                seen.push(slot);
            }
            None => {
                assert!(room.movers.is_empty(), "slot {slot}: no record");
                assert!(graphs.len() <= 1, "slot {slot}: at most one graph");
            }
        }
    }
    assert_eq!(seen, [6, 8, 9, 11, 13, 24, 25, 26, 27]);
}

/// Every text a bank holds fits the room `XP01` copies a text into: the
/// copy `13A7h:0023h` is handed `[1E4Ah]`, half of the 2000-byte scratch
/// `[1691h]` (`0x0DF3D`–`0x0DF45`), and stops there.
#[test]
fn every_1994_text_fits_the_room_the_player_copies_it_into() {
    let dir = ds1_or_skip!();
    let mut longest = 0usize;
    let mut scan = |bank: &Bank| {
        for d in bank.entries.iter().flatten() {
            for l in &d.lines {
                longest = longest.max(motionvm_parsec_formats::to_cp437(&l.text).len());
            }
        }
    };
    let Dat::Rib(rib) = open_dat(&dat(&dir, "DIALOG.DAT")).expect("DIALOG.DAT") else {
        panic!("DIALOG.DAT is a bare RIB");
    };
    for i in 0..2 {
        scan(&Bank::parse(rib.get(i).expect("a bank")).expect("bank"));
    }
    let Dat::Fgroup(rooms) = open_dat(&dat(&dir, "DSROOM.DAT")).expect("DSROOM.DAT") else {
        panic!("DSROOM.DAT is an FGROUP");
    };
    for slot in &rooms.slots {
        if let Slot::Rib(r) = slot
            && let Some(b) = r.get(3).filter(|b| Bank::looks_like(b))
        {
            scan(&Bank::parse(b).expect("bank"));
        }
    }
    assert_eq!(longest, 510);
}
