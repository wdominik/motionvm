//! `motionvm-parsec-tools` — inspect and extract the resources of a PARSEC
//! game: *Dunkle Schatten*, *Captain Zins* or *Bi-Fi 2*, told apart by the
//! room container the install ships.
//!
//! ```text
//! motionvm-parsec-tools info    <gamedata>                 summary of every DAT file
//! motionvm-parsec-tools extract <gamedata> [--out DIR]     rooms and pictures as PNG
//! motionvm-parsec-tools picture <gamedata> <file> [n]      one resource as PNG
//! motionvm-parsec-tools dialogues <gamedata>               every dialogue bank of the game
//! motionvm-parsec-tools rib     <gamedata> <file>          what each resource of a DAT is
//! motionvm-parsec-tools room    <gamedata> <id>            one RDV room: header, hotspots
//! motionvm-parsec-tools registers <gamedata> <module>      one song's OPL register stream
//! ```

use motionvm_parsec_formats::detect::{Title, identify, resource_dir};
use motionvm_parsec_formats::fgroup::{Dat, Slot, open_dat};
use motionvm_parsec_formats::{Bank, Dialogue, Font, Picture, Rib, Room, RoomFile, find_ci, pal};
use motionvm_render::{Palette, write_indexed_png};
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(e) = run() {
        eprintln!("motionvm-parsec-tools: {e}");
        std::process::exit(1);
    }
}

const USAGE: &str = "\
usage: motionvm-parsec-tools <command> [arguments]

  info    <gamedata>                 every DAT file, with RIB counts and sizes
  extract <gamedata> [--out DIR]     every room playfield of the room
                                     container (DSROOM.DAT, CZROOM.DAT,
                                     BFROOM.DAT) as a
                                     PNG with its sizes and hotspot list
                                     beside it, and the pictures of the
                                     strip files (PANEL*.DAT) and CMBMOU.DAT
                                     (--out defaults to ./out)
  picture <gamedata> <file> [n] [--out F]
                                     one RIB resource as an indexed PNG
                                     (n defaults to 0; --out defaults to picture.png)
  dialogues <gamedata>               every dialogue bank: the two global ones
                                     in DIALOG.DAT and each location's own in
                                     the room container, with its texts
  rib     <gamedata> <file>          every resource of one DAT: index, size and
                                     what the readers make of it
  room    <gamedata> <id>            one RDV room's trailer and hotspot table
  registers <gamedata> <module> [--ticks N] [--out F]
                                     module N of PSMP2L.DAT played endlessly
                                     through the game's MUSADL.DRV: every OPL
                                     register write as `tick register value`,
                                     the driver's switch-on on tick 0 and the
                                     song from tick 1 (--ticks defaults to
                                     6000, --out to registers-NN.txt)
<gamedata> is the directory the game was installed into — the one holding
XP01.EXE and RESOURCE/.
";

fn usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2);
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, rest) = args.split_first().unwrap_or_else(|| usage());
    match cmd.as_str() {
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        "info" => {
            let dir = game(rest)?;
            info(dir)
        }
        "extract" => {
            let dir = game(rest)?;
            let out = flag(rest, "--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("out"));
            extract(dir, &out)
        }
        "picture" => {
            let dir = game(rest)?;
            let file = rest.get(1).unwrap_or_else(|| usage());
            let n: usize = rest.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
            let out = flag(rest, "--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("picture.png"));
            one_picture(dir, file, n, &out)
        }
        "dialogues" => {
            let dir = game(rest)?;
            dialogues(dir)
        }
        "rib" => {
            let dir = game(rest)?;
            let file = rest.get(1).unwrap_or_else(|| usage());
            rib_listing(dir, file)
        }
        "room" => {
            let dir = game(rest)?;
            let id: u32 = rest
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| usage());
            room(dir, id)
        }
        "registers" => {
            let dir = game(rest)?;
            let module: usize = rest
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| usage());
            let ticks = match flag(rest, "--ticks") {
                Some(n) => n.parse().map_err(|_| format!("--ticks {n}: not a count"))?,
                None => 6000,
            };
            let out = flag(rest, "--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(format!("registers-{module:02}.txt")));
            registers(dir, module, ticks, &out)
        }
        _ => usage(),
    }
}

/// Module `module`'s song as the Ad Lib path hands it to the driver —
/// the bare `PLX` out of `PSMP2L.DAT` — played endlessly through the
/// game's own `MUSADL.DRV` out of `PSMDRV.DAT`, for `ticks` ticks of the
/// driver's clock; every register write it makes, as `tick register
/// value`. The other side of a comparison is an OPL capture of the
/// original; no engine is involved, a song is started by number.
fn registers(
    dir: &Path,
    module: usize,
    ticks: u32,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fmt::Write as _;
    let res = resource_dir(dir);
    let read = |name: &str| -> Result<Vec<u8>, String> {
        let path = find_ci(&res, name).ok_or_else(|| format!("RESOURCE/{name}: not found"))?;
        std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
    };
    let library = motionvm_parsec_formats::P2l::parse(read("PSMP2L.DAT")?)?;
    let song = motionvm_psm::Plx::from_section(library.song(module)?)?;
    let driver = motionvm_parsec_formats::p2l::ad_lib_driver(&read("PSMDRV.DAT")?)?;
    let driver = motionvm_psm::Driver::parse(&driver)?;
    let mut writes = motionvm_psm::Sequencer::install_writes(&driver);
    let mut seq = motionvm_psm::Sequencer::new(&driver, song, -1);
    let mut text = format!(
        "# module {module} of PSMP2L.DAT in {}, through MUSADL.DRV: every OPL register\n\
         # write of {ticks} ticks, as `tick register value`; tick 0 carries the\n\
         # driver's switch-on, the song starts on tick 1.\n",
        dir.display()
    );
    let mut count = 0;
    for tick in 0..=ticks {
        if tick > 0 {
            seq.tick(&mut writes);
        }
        for w in writes.drain(..) {
            let _ = writeln!(text, "{tick} {:#05x} {:#04x}", w.address(), w.value);
            count += 1;
        }
    }
    std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{count:>7} writes over {ticks} ticks -> {}", out.display());
    Ok(())
}

/// One palette resource as the DAC it leaves: its bank written over black,
/// or black when it is not a bank.
fn dac(bytes: &[u8]) -> Palette {
    let mut p = Palette::default();
    if let Some(bank) = pal::bank(bytes) {
        p.load(bank.start, bank.rgb);
    }
    p
}

/// A room's own bank as the DAC it leaves.
fn room_dac(room: &Room) -> Palette {
    let mut p = Palette::default();
    p.load(room.palette_start, &room.palette_rgb);
    p
}

/// The game directory a command names, refused with a sentence when it
/// holds no game this crate reads. Without the room container a command would reach
/// whichever reader opens a file first and fail on that file's name — or,
/// for `info` and `dialogues`, list nothing and exit 0 as if it had read a
/// game.
fn game(rest: &[String]) -> Result<&Path, Box<dyn std::error::Error>> {
    let dir = Path::new(rest.first().unwrap_or_else(|| usage()));
    match identify(dir) {
        Some(_) => Ok(dir),
        None => Err(refusal(dir).into()),
    }
}

/// What [`game`] says about a directory that holds no game: every game it
/// looked for, and the file that would have told it.
fn refusal(dir: &Path) -> String {
    let looked: Vec<String> = Title::ALL
        .iter()
        .map(|t| format!("{} ({})", t.name(), t.needs()))
        .collect();
    format!(
        "{}: no PARSEC game here — {}",
        dir.display(),
        looked.join(", ")
    )
}

/// The room container of the game at `dir`.
fn room_container(dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let title = identify(dir).ok_or_else(|| refusal(dir))?;
    find_ci(&resource_dir(dir), title.room_file())
        .ok_or_else(|| format!("no {}", title.room_file()).into())
}

fn flag(args: &[String], name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1).cloned()
}

fn info(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(t) = identify(dir) {
        println!("{} ({})", t.name(), t.slug());
    }
    let res = resource_dir(dir);
    let mut names: Vec<_> = std::fs::read_dir(&res)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x.eq_ignore_ascii_case("dat"))
        })
        .collect();
    names.sort();
    for path in names {
        let bytes = std::fs::read(&path)?;
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        match open_dat(&bytes) {
            Ok(Dat::Rib(r)) => {
                println!(
                    "{name}: RIB n={} buf={} stored={} raw={}",
                    r.resources.len(),
                    r.buf_size,
                    r.stored,
                    r.raw
                );
            }
            Ok(Dat::Fgroup(g)) => {
                let ribs = g.slots.iter().filter(|s| matches!(s, Slot::Rib(_))).count();
                let nulls = g.slots.iter().filter(|s| matches!(s, Slot::Null)).count();
                println!(
                    "{name}: FGROUP n={} ribs={ribs} null={nulls}",
                    g.slots.len()
                );
            }
            Err(e) => println!("{name}: {e}"),
        }
    }
    Ok(())
}

fn extract(dir: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(out)?;
    let res = resource_dir(dir);
    extract_rooms(dir, out)?;
    extract_stills(&res, out)?;
    Ok(())
}

fn extract_rooms(dir: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let rooms = out.join("rooms");
    std::fs::create_dir_all(&rooms)?;
    for (id, file) in rooms_of(dir)? {
        let png = rooms.join(format!("room{id}.png"));
        write_indexed_png(
            &png,
            320,
            128,
            &file.room.pixels,
            room_dac(&file.room).to_rgb8(),
            None,
        )?;
        let txt = rooms.join(format!("room{id}.txt"));
        let mut w = std::fs::File::create(txt)?;
        writeln!(w, "room {id}")?;
        writeln!(
            w,
            "view {}x{} tile {}x{} map {}x{}",
            file.room.view.0,
            file.room.view.1,
            file.room.tile.0,
            file.room.tile.1,
            file.room.map.0,
            file.room.map.1
        )?;
        for (i, hs) in file.hotspots.spots.iter().enumerate() {
            writeln!(
                w,
                "  [{i}] ({},{},{},{}) mask={:04X} {}",
                hs.x, hs.y, hs.w, hs.h, hs.verb_mask, hs.name
            )?;
        }
    }
    Ok(())
}

fn extract_stills(res: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let pics = out.join("pictures");
    std::fs::create_dir_all(&pics)?;
    // The strip's files — Dunkle Schatten's one `PANELS.DAT`, Captain
    // Zins's `PANEL0.DAT` and `PANEL1.DAT` — and the pointers'.
    let mut names: Vec<String> = std::fs::read_dir(res)?
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_uppercase))
        .filter(|n| n.starts_with("PANEL") && n.ends_with(".DAT"))
        .collect();
    names.sort();
    names.push("CMBMOU.DAT".to_owned());
    for name in &names {
        let Some(path) = find_ci(res, name) else {
            continue;
        };
        let Ok(rib) = Rib::parse(&std::fs::read(path)?) else {
            continue;
        };
        let pal = rib
            .resources
            .iter()
            .find(|b| {
                b.len() == 96 || b.len() == 386 || (b.len() >= 2 && b[0] == 0 && b[1] == 0x80)
            })
            .map(|b| dac(b))
            .unwrap_or_default();
        let stem = name
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(name)
            .to_ascii_lowercase();
        for (i, blob) in rib.resources.iter().enumerate() {
            if let Ok(pic) = Picture::parse(blob) {
                let png = pics.join(format!("{stem}-{i:02}.png"));
                write_indexed_png(
                    &png,
                    u32::from(pic.width),
                    u32::from(pic.height),
                    &pic.pixels,
                    pal.to_rgb8(),
                    None,
                )?;
            }
        }
    }
    Ok(())
}

fn one_picture(
    dir: &Path,
    file: &str,
    n: usize,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let res = resource_dir(dir);
    let path = find_ci(&res, file).ok_or_else(|| format!("no {file} in {}", res.display()))?;
    let rib = Rib::parse(&std::fs::read(path)?)?;
    let blob = rib
        .get(n)
        .ok_or_else(|| format!("{file} has no resource {n}"))?;
    if Room::looks_like(blob) {
        let room = Room::parse(blob)?;
        write_indexed_png(out, 320, 128, &room.pixels, room_dac(&room).to_rgb8(), None)?;
    } else {
        let pic = Picture::parse(blob)?;
        let pal = rib
            .resources
            .iter()
            .find(|b| b.len() == 96 || b.len() == 386)
            .map(|b| dac(b))
            .unwrap_or_default();
        write_indexed_png(
            out,
            u32::from(pic.width),
            u32::from(pic.height),
            &pic.pixels,
            pal.to_rgb8(),
            None,
        )?;
    }
    println!("wrote {}", out.display());
    Ok(())
}

/// What each resource of one DAT holds, as far as the readers can tell.
///
/// A RIB is a numbered bag of blobs with no type field: which slot is a
/// palette, which a picture and which a font is something the program
/// knew and the file does not say. Reading a title therefore starts by
/// asking every reader what it makes of every slot, and that is what this
/// prints — one line per resource, with the first bytes when nothing
/// claims it.
///
/// A guess here is a starting point for reading the listing, never an
/// answer: two of these shapes overlap, and the program is the authority
/// on which one it meant.
fn rib_listing(dir: &Path, file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let res = resource_dir(dir);
    let path = find_ci(&res, file)
        .or_else(|| find_ci(dir, file))
        .ok_or_else(|| format!("{file} not found"))?;
    let bytes = std::fs::read(&path)?;
    let ribs: Vec<(String, Rib)> = match open_dat(&bytes)? {
        Dat::Rib(rib) => vec![(String::new(), rib)],
        Dat::Fgroup(group) => group
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| match slot {
                Slot::Rib(rib) => Some((format!("slot {i} "), rib.clone())),
                Slot::Null | Slot::Raw(_) => None,
            })
            .collect(),
    };
    for (label, rib) in ribs {
        for (i, blob) in rib.resources.iter().enumerate() {
            println!("{label}[{i}] {} bytes: {}", blob.len(), describe(blob));
        }
    }
    Ok(())
}

/// Every dialogue the game holds, bank by bank.
///
/// `XP01` reaches a dialogue through one of three tables: the location's
/// own — resource 3 of its room slot — and two global banks it numbers 999
/// and 2, which are resources 0 and 1 of `DIALOG.DAT`. This walks all of
/// them.
///
/// The texts are shown with their controls resolved rather than raw: a
/// `02 n` substitution is printed as `«n»` because what it stands for is
/// drawn at run time, `01` and `03` pairs are dropped because they steer
/// the painter, and the page break `\r` is shown as ` / `. What the file
/// holds is the bytes; this is a reading of them.
fn dialogues(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut printed = false;
    for (label, file, n) in [("bank 999", "DIALOG.DAT", 0), ("bank 2", "DIALOG.DAT", 1)] {
        let Some(path) = find_ci(&resource_dir(dir), file) else {
            continue;
        };
        let bytes = std::fs::read(path)?;
        let Dat::Rib(rib) = open_dat(&bytes)? else {
            continue;
        };
        let Some(blob) = rib.get(n) else { continue };
        if !Bank::looks_like(blob) {
            println!("{label}: not a bank");
            return Ok(());
        }
        show_bank(label, &Bank::parse(blob)?);
        printed = true;
    }
    if let Ok(path) = room_container(dir) {
        let bytes = std::fs::read(path)?;
        if let Dat::Fgroup(group) = open_dat(&bytes)? {
            for (i, slot) in group.slots.iter().enumerate() {
                let Slot::Rib(rib) = slot else { continue };
                let Some(blob) = rib.get(3) else { continue };
                if !Bank::looks_like(blob) {
                    continue;
                }
                show_bank(&format!("location {i}"), &Bank::parse(blob)?);
                printed = true;
            }
        }
    }
    if !printed {
        println!("no dialogue banks here");
    }
    Ok(())
}

/// One bank, its places numbered the way a handler names them.
fn show_bank(label: &str, bank: &Bank) {
    println!("\n{label}: {} places", bank.len());
    for (i, entry) in bank.entries.iter().enumerate() {
        match entry {
            None => println!("  {i:3}  —"),
            Some(d) => {
                for (j, line) in d.lines.iter().enumerate() {
                    let head = if j == 0 {
                        format!("  {i:3}  ")
                    } else {
                        "       ".to_owned()
                    };
                    println!("{head}{}", plain(&line.text));
                }
            }
        }
    }
}

/// A dialogue line with its control pairs read rather than shown raw.
fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut it = text.chars();
    while let Some(c) = it.next() {
        match c {
            '\u{1}' | '\u{3}' => {
                it.next();
            }
            '\u{2}' => {
                let n = it.next().map_or(0, u32::from);
                out.push_str(&format!("«{n}»"));
            }
            '\r' => out.push_str(" / "),
            '\n' => out.push(' '),
            c => out.push(c),
        }
    }
    // A line ends on its page break; showing that trailing separator would
    // only add noise to every line.
    out.trim().trim_end_matches('/').trim_end().to_owned()
}

/// One resource in one line: what a reader claims it is, or its head.
///
/// Order matters. A palette is 98 bytes with a `start, count` prefix and
/// nothing else matches it; a picture or a cel is checked exactly, so its
/// header has to account for the whole blob; only then is the 8-byte cel
/// header tried as a *prefix*, which is what a flags-1 opcode stream is.
/// The font check comes last and is fenced, because a font block's header
/// is loose enough to accept a cel if it is asked first.
fn describe(blob: &[u8]) -> String {
    if blob.starts_with(motionvm_parsec_formats::NULLPTR) {
        return "empty slot".into();
    }
    if blob.len() >= 8 && blob[..8] == *motionvm_parsec_formats::rdv::MAGIC {
        return "RDV1.00 room".into();
    }
    if let Some(bank) = pal::bank(blob) {
        return format!(
            "palette, bank at {}, {} colours",
            bank.start,
            bank.rgb.len() / 3
        );
    }
    if let Ok(p) = Picture::parse(blob) {
        let kind = if usize::from(p.width) * usize::from(p.height) + 4 == blob.len() {
            "picture"
        } else {
            "cel, uncompressed"
        };
        return format!("{kind} {}x{}", p.width, p.height);
    }
    if blob.len() >= 8 {
        let (w, h) = (
            u16::from_le_bytes([blob[0], blob[1]]),
            u16::from_le_bytes([blob[2], blob[3]]),
        );
        let flags = u16::from_le_bytes([blob[4], blob[5]]);
        if Picture::parse_prefix(blob).is_ok() {
            return format!("cel {w}x{h}, flags {flags}");
        }
    }
    if let Ok(f) = Font::parse(blob) {
        let n = f.glyphs.iter().filter(|g| g.is_some()).count();
        if n > 0 && (4..=32).contains(&f.line_height) {
            return format!(
                "font, {n} glyphs from {:#04x}, line height {}",
                f.first, f.line_height
            );
        }
    }
    if Bank::looks_like(blob)
        && let Ok(bank) = Bank::parse(blob)
    {
        let holes = bank.entries.iter().filter(|e| e.is_none()).count();
        let empty = if holes == 0 {
            String::new()
        } else {
            format!(", {holes} empty")
        };
        return format!("dialogue bank, {} places{empty}", bank.len());
    }
    if Dialogue::looks_like(blob)
        && let Ok(d) = Dialogue::parse(blob)
    {
        return format!("II dialogue, mode {}, {} lines", d.mode, d.lines.len());
    }
    let head: Vec<String> = blob.iter().take(8).map(|b| format!("{b:02x}")).collect();
    let text: String = blob
        .iter()
        .take(8)
        .map(|&b| {
            if (0x20..0x7f).contains(&b) {
                char::from(b)
            } else {
                '.'
            }
        })
        .collect();
    format!("? {} |{text}|", head.join(" "))
}

/// Every room the install holds: the slots of the room container that
/// carry one, each a six-resource RIB.
fn rooms_of(dir: &Path) -> Result<Vec<(String, RoomFile)>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    let path = room_container(dir)?;
    let bytes = std::fs::read(path)?;
    if let Dat::Fgroup(group) = open_dat(&bytes)? {
        for (i, slot) in group.slots.iter().enumerate() {
            let Slot::Rib(rib) = slot else { continue };
            if let Some(room) = RoomFile::from_1994_slot(rib) {
                out.push((format!("{i:02}"), room?));
            }
        }
    }
    Ok(out)
}

fn room(dir: &Path, id: u32) -> Result<(), Box<dyn std::error::Error>> {
    let want = format!("{id:02}");
    let rooms = rooms_of(dir)?;
    let file = rooms
        .into_iter()
        .find(|(name, _)| *name == want)
        .map(|(_, f)| f)
        .ok_or_else(|| format!("no room {want} in this install"))?;
    println!(
        "room {id:02}: view {}x{} tile {}x{} map {}x{} cels={}",
        file.room.view.0,
        file.room.view.1,
        file.room.tile.0,
        file.room.tile.1,
        file.room.map.0,
        file.room.map.1,
        file.room.cels.len()
    );
    let mut bundles: std::collections::BTreeMap<u8, usize> = std::collections::BTreeMap::new();
    for o in &file.room.overlays {
        *bundles.entry(o.bundle).or_default() += 1;
    }
    println!("  layers: {}", file.room.layers.len());
    for o in &file.room.overlays {
        println!(
            "    obj id {:3} kind {} flags {:#04x} at ({},{}) align {} bundle {} cel {} scale {:?} layer {}",
            o.id, o.kind, o.flags, o.x, o.y, o.align, o.bundle, o.cel_index, o.scale, o.layer
        );
    }
    println!(
        "  overlays: {} — bundles {}",
        file.room.overlays.len(),
        bundles
            .iter()
            .map(|(b, n)| format!("{b}\u{d7}{n}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    match file.walk() {
        Some(g) => {
            println!("  walk: {} nodes, {} edges", g.nodes.len(), g.edges.len());
            for (i, n) in g.nodes.iter().enumerate() {
                println!(
                    "    node {i}: id {} at ({},{}) scale {} layer {}",
                    n.id, n.x, n.y, n.scale, n.layer
                );
            }
        }
        None => println!("  walk: none"),
    }
    for (i, hs) in file.hotspots.spots.iter().enumerate() {
        println!(
            "  [{i}] active={} ({},{},{},{}) mask={:04X} {}",
            hs.active, hs.x, hs.y, hs.w, hs.h, hs.verb_mask, hs.name
        );
        for (j, t) in hs.texts.iter().enumerate() {
            if !t.is_empty() {
                let one = t.replace(['\r', '\n'], " / ");
                // Which verb reaches a slot is read from the rules'
                // handlers.
                let verb = match j {
                    2 => "Benutze  ",
                    3 => "Betrachte",
                    4 => "Sprich   ",
                    5 => "Nimm     ",
                    _ => "         ",
                };
                println!("      [{j}] {verb} {one}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::refusal;
    use std::path::Path;

    #[test]
    fn a_directory_without_the_game_is_refused_by_name() {
        assert_eq!(
            refusal(Path::new("/games/DS2")),
            "/games/DS2: no PARSEC game here — Dunkle Schatten (RESOURCE/DSROOM.DAT beside \
             XP01.EXE), Captain Zins (RESOURCE/CZROOM.DAT beside XP01.EXE), Bi-Fi 2 - \
             Action in Hollywood (RESOURCE/BFROOM.DAT beside XP01.EXE)"
        );
    }
}
