//! The files a savegame is made of.
//!
//! The original writes three per slot, all named from the id `701 + slot`:
//!
//! | file | written by | holds |
//! |---|---|---|
//! | `NNN.blk` | `PUT` | four bytes, the location number |
//! | `NNN.FRZ` | `=>PUTAS` | every resident module's memory |
//! | `NNN.anm` | `PUTANIM` | the descriptor tree and a handful of globals |
//!
//! The first is written straight from module memory and read straight back, so
//! its bytes here are the original's. The other two are not, and cannot be.
//!
//! ## Why the bytes differ
//!
//! `NEWSCREEN` and `NEWDESC` hand the scripts a raw heap pointer in the
//! original; here they hand out small handles counted from one. Both are just
//! opaque numbers to the bytecode, which only ever passes them back to `ACTSCR`
//! and `ACTDESC` — but they are numbers the scripts *store*, in the module
//! memory a savegame is mostly made of. So a saved module image from the
//! original is full of addresses that mean nothing here, and one of ours is
//! full of handles that would mean nothing there. Savegames cannot be carried
//! between the two in either direction, and no amount of matching the file
//! layout would change that. What is reproduced instead is the mechanism: the
//! same three artifacts, the same ids, the same order, the same semantics — so
//! that the game's own menu drives it, rather than a second path beside it.
//!
//! Two places where these files deliberately differ from the original beyond
//! their layout:
//!
//! * **They are checked.** `=>GETAS` (0x657a4) opens the file without looking
//!   at the handle, reads whatever length it finds, and copies it into module
//!   memory positionally — no magic, no count, no validation anywhere. A
//!   truncated or mismatched file there corrupts the running game silently.
//!   Here every record is checked *before the first byte is written back*, so a
//!   bad file stops the load instead of half-applying it.
//! * **Records are keyed by module number.** The original writes the number
//!   into each record and then ignores it on the way back in, relying on the
//!   loaded set being identical because `INCLLOC` has just rebuilt it. Reading
//!   by name costs nothing and turns a whole class of silent corruption into a
//!   named error.

//!
//! ## What is where
//!
//! This file holds what every savegame file shares: the atomic write, the
//! per-generation magic and the layout version. [`codec`] is the file's
//! grammar — the head with its checksum, the chunked body, the reader that
//! refuses by name and the writer. [`frz`] and [`anm`] are the two files
//! themselves, each a set of chunks over that grammar. The tests at the
//! bottom drive the whole of it through the two files, which is how a change
//! to any part is proved.

// A savegame is the one file the engine reads that a player can hand it, so
// its reader holds the readers' rule: an index has been bounds-checked or is a
// constant, and a sum says what it does at the edge. The module's tests are
// outside the rule, because a test indexes what it built.
#![cfg_attr(
    not(test),
    deny(clippy::indexing_slicing, clippy::arithmetic_side_effects)
)]

use motionvm_motion_formats::Generation;
use std::io::Write;
use std::path::{Path, PathBuf};

mod anm;
mod codec;
mod frz;

pub(crate) use anm::{Anim, DescriptorState, ScreenState, read_anm, write_anm};
pub(crate) use frz::{ModuleImage, read_frz, write_frz};

/// Writes a savegame file so that no reader ever sees half of one.
///
/// The original writes each of the three straight through — `PUT` at 0x668aa
/// opens, writes and closes, and its siblings do the same — so a crash, a
/// full disk or a pulled plug partway through leaves a file that is neither
/// the old save nor the new one, and the old one is already gone. That is not
/// a fault worth reproducing: it costs a player the slot they were saving
/// over, and it costs them silently, since a half-written file still opens.
///
/// Here the bytes go to `<name>.tmp`, are flushed to the device, and only then
/// take the file's name. A rename within a directory is atomic on every
/// filesystem this runs on, so a reader sees either every byte of the new file
/// or every byte of the old one.
///
/// The temporary lands beside its target and not in a system temporary
/// directory, because a rename across mount points is a copy and not a rename.
///
/// The directory itself is deliberately not flushed. Doing so would make the
/// *rename* durable across a power cut as well; without it, a crash can leave
/// the old file in place — which is the older of the two consistent states,
/// and consistent is the whole of what is promised here.
pub(crate) fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    let tmp = PathBuf::from(name);
    let write = |tmp: &Path| -> std::io::Result<()> {
        let mut file = std::fs::File::create(tmp)?;
        file.write_all(bytes)?;
        file.sync_all()
    };
    if let Err(e) = write(&tmp).and_then(|()| std::fs::rename(&tmp, path)) {
        // Whatever went wrong, the half-written temporary is not wanted: it
        // would sit in the save directory under a name nothing reads, and the
        // next attempt would have to overwrite it anyway.
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

/// What `=>PUTAS` and `=>GETAS` put at the head of a `.FRZ`, per generation.
///
/// The two engines keep the same three artifacts, but a 16-bit module image is
/// bytes of a flat arena where a 32-bit one is cells, and a 16-bit descriptor
/// carries a buffer — so each generation writes a magic of its own and neither
/// reads the other's. The magic is the *generation's* and not the game's,
/// which is why the five 16-bit games must not share a save directory: one
/// would open another's slot rather than refuse it.
///
/// Hung off [`Generation`] rather than a `Layout` enum beside it: there were
/// four types saying "which generation" and they agreed by convention. This is
/// the one, with the savegame's answer on it.
pub(crate) trait Layout {
    /// The eight bytes a `.FRZ` of this generation opens with.
    fn frz_magic(self) -> &'static [u8; 8];
    /// The eight bytes an `.anm` of this generation opens with.
    fn anm_magic(self) -> &'static [u8; 8];
}

impl Layout for Generation {
    fn frz_magic(self) -> &'static [u8; 8] {
        match self {
            Generation::Motion32 => b"DS2FRZ\0\0",
            Generation::Motion16 => b"ENVFRZ\0\0",
        }
    }

    fn anm_magic(self) -> &'static [u8; 8] {
        match self {
            Generation::Motion32 => b"DS2ANM\0\0",
            Generation::Motion16 => b"ENVANM\0\0",
        }
    }
}

/// The layout this build writes and reads — the only one.
///
/// A bump means the *file* changed shape, not that the engine did: a field
/// added to a chunk, a chunk whose meaning moved. A new chunk beside the
/// known ones needs no bump, which is the whole reason the body is chunked.
/// A file of any other version is refused by name and left on disk: nothing
/// here deletes a savegame it cannot read, and nothing converts one — one
/// reader, one layout, which is what keeps the reader a single path.
pub(crate) const VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::anm::{write_descriptors, write_flips, write_globals, write_screens};
    use super::codec::{Writer, crc32, lay_out};
    use super::*;
    use motionvm_motion_forth::cell;

    /// A descriptor with every field distinguishable from every other, so a
    /// round trip that drops one or swaps two cannot come back equal.
    fn descriptor(handle: u32, buffer: Option<i32>) -> DescriptorState {
        DescriptorState {
            handle,
            screen: 7,
            x: -11,
            y: 12,
            level: 13,
            shows: (2, 15),
            text: Some(16),
            font: Some(18),
            color: Some(19),
            template: Some(20),
            wait: -21,
            callback: 22,
            x_mode: 1,
            y_mode: 2,
            active: true,
            auto_buffer: true,
            fields: vec![("SDBLK".into(), 23), ("SDTDT".into(), 24)],
            buffer,
        }
    }

    fn anim(layout: Generation) -> Anim {
        Anim {
            next_descriptor: 42,
            current: Some(2),
            screen: Some(1),
            pointer_visible: true,
            dialog_offset: 3,
            dialog_return: 4,
            palette: [5; 768],
            screens: vec![ScreenState {
                handle: 1,
                size: (320, 200),
                full_view: (320, 200),
                view: (320, 165),
                view_pos: (6, 7),
                pos: (8, 9),
                origin: (10, 11),
            }],
            flips: vec![(30, 31)],
            descriptors: vec![
                descriptor(2, (layout == Generation::Motion16).then_some(25)),
                descriptor(3, (layout == Generation::Motion16).then_some(-1)),
            ],
            buffers_on: layout == Generation::Motion16,
            buffers: if layout == Generation::Motion16 {
                vec![(26, 27, 28)]
            } else {
                Vec::new()
            },
        }
    }

    /// What a descriptor shows has to survive a save and a load.
    ///
    /// The suite drives the games end to end and never looks at these fields
    /// afterwards, so a picture lost between `PUTANIM` and `GETANIM` would
    /// only surface as a room that comes back blank — a long way from the
    /// line that dropped it.
    #[track_caller]
    fn round_trip(layout: Generation) {
        let before = anim(layout);
        let bytes = write_anm(&before, layout, SLUG);
        let after = read_anm(&bytes, "a written savegame", layout, SLUG).expect("reads back");

        assert_eq!(after.next_descriptor, before.next_descriptor);
        assert_eq!(after.current, before.current);
        assert_eq!(after.screen, before.screen);
        assert_eq!(after.pointer_visible, before.pointer_visible);
        assert_eq!(
            (after.dialog_offset, after.dialog_return),
            (before.dialog_offset, before.dialog_return)
        );
        assert_eq!(after.palette, before.palette);
        assert_eq!(after.flips, before.flips);
        assert_eq!(after.buffers_on, before.buffers_on);
        assert_eq!(after.buffers, before.buffers);
        assert_eq!(after.screens.len(), before.screens.len());
        for (a, b) in after.screens.iter().zip(&before.screens) {
            assert_eq!(
                (
                    a.handle,
                    a.size,
                    a.full_view,
                    a.view,
                    a.view_pos,
                    a.pos,
                    a.origin
                ),
                (
                    b.handle,
                    b.size,
                    b.full_view,
                    b.view,
                    b.view_pos,
                    b.pos,
                    b.origin
                )
            );
        }

        assert_eq!(after.descriptors.len(), before.descriptors.len());
        for (a, b) in after.descriptors.iter().zip(&before.descriptors) {
            assert_eq!((a.handle, a.screen), (b.handle, b.screen));
            assert_eq!((a.x, a.y, a.level), (b.x, b.y, b.level));
            // The picture and the text, which is what nothing else checks.
            assert_eq!(a.shows, b.shows, "what {} shows", b.handle);
            assert_eq!(a.text, b.text, "text of {}", b.handle);
            assert_eq!((a.font, a.color, a.template), (b.font, b.color, b.template));
            assert_eq!((a.wait, a.callback), (b.wait, b.callback));
            assert_eq!((a.x_mode, a.y_mode), (b.x_mode, b.y_mode));
            assert_eq!((a.active, a.auto_buffer), (b.active, b.auto_buffer));
            assert_eq!(a.fields, b.fields);
            assert_eq!(a.buffer, b.buffer, "buffer of {}", b.handle);
        }
    }

    #[test]
    fn a_32_bit_animation_comes_back_field_for_field() {
        round_trip(Generation::Motion32);
    }

    #[test]
    fn a_16_bit_animation_comes_back_field_for_field() {
        round_trip(Generation::Motion16);
    }

    /// The other generation's file is refused by name, not misread.
    #[test]
    fn an_animation_of_the_other_layout_is_refused() {
        let bytes = write_anm(&anim(Generation::Motion32), Generation::Motion32, SLUG);
        let Err(e) = read_anm(&bytes, "a 32-bit savegame", Generation::Motion16, SLUG) else {
            panic!("the magic does not match, so this should not have read");
        };
        assert!(
            e.to_string().contains("a 32-bit savegame"),
            "should name what it was reading: {e}"
        );
    }

    /// Any version but this one is refused by name — and the file is left
    /// where it is.
    #[test]
    fn a_version_this_build_does_not_read_is_refused_by_name() {
        let layout = Generation::Motion32;
        let good = write_anm(&anim(layout), layout, SLUG);
        for version in [0u32, VERSION + 1, 99] {
            let mut bytes = good.clone();
            bytes[8..12].copy_from_slice(&version.to_le_bytes());
            let Err(e) = read_anm(&bytes, "701.anm", layout, SLUG) else {
                panic!("version {version} should not have read");
            };
            let said = e.to_string();
            assert!(said.contains("701.anm"), "names the file: {said}");
            assert!(said.contains(&version.to_string()), "names it: {said}");
        }
    }

    /// One byte changed anywhere in the body, and the file is refused as
    /// damaged rather than read as if nothing had happened.
    #[test]
    fn a_single_flipped_byte_is_caught() {
        let layout = Generation::Motion16;
        let good = write_anm(&anim(layout), layout, SLUG);
        // Every twentieth byte of the body, so the check is not resting on
        // one lucky offset.
        for at in (HEAD..good.len()).step_by(20) {
            let mut bytes = good.clone();
            bytes[at] ^= 0x01;
            let Err(e) = read_anm(&bytes, "701.anm", layout, SLUG) else {
                panic!("a flipped byte at {at} was read as if it were sound");
            };
            assert!(
                e.to_string().contains("damaged"),
                "byte {at}: should say the file is damaged, said: {e}"
            );
        }
    }

    /// A slot carried by hand into another game's directory is refused as that
    /// other game's, before anything in the body is looked at.
    #[test]
    fn another_games_slot_is_refused_as_that_games() {
        let layout = Generation::Motion16;
        let bytes = write_anm(&anim(layout), layout, "enviro");
        let Err(e) = read_anm(&bytes, "701.anm", layout, "hfa") else {
            panic!("this is not this game's slot");
        };
        let said = e.to_string();
        assert!(said.contains("enviro"), "names the file's game: {said}");
        assert!(said.contains("hfa"), "and the reader's: {said}");
    }

    /// Puts a section this build does not know into an otherwise sound file,
    /// and repairs the length and the checksum around it — which is what a
    /// later build adding a section would produce.
    fn with_a_later_section(file: &[u8]) -> Vec<u8> {
        let mut body = file[HEAD..].to_vec();
        body.extend_from_slice(b"XTRA");
        body.extend_from_slice(&7u32.to_le_bytes());
        body.extend_from_slice(b"unknown");
        let mut out = file[..12].to_vec();
        out.extend_from_slice(&cell::narrow(body.len()).to_le_bytes());
        out.extend_from_slice(&crc32(&body).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    /// The point of the chunking: a build that adds a section does not need a
    /// version bump, because this one steps over what it does not know.
    #[test]
    fn a_section_this_build_does_not_know_is_stepped_over() {
        let layout = Generation::Motion16;
        let before = anim(layout);
        let bytes = with_a_later_section(&write_anm(&before, layout, SLUG));
        let after = read_anm(&bytes, "a later build's slot", layout, SLUG)
            .expect("an unknown section is not a fault");
        assert_eq!(after.descriptors.len(), before.descriptors.len());
        assert_eq!(after.palette, before.palette);
    }

    #[test]
    fn a_writer_and_a_reader_that_disagree_inside_a_section_say_so() {
        // Growth is a new section and never a field on the end of an old one,
        // so bytes left over in a section this build knows are a disagreement
        // and not a newer writer.
        let layout = Generation::Motion32;
        let mut chunks = Writer::default();
        chunks.chunk(b"HEAD", |w| {
            write_globals(w, &anim(layout));
            w.u32(0);
        });
        chunks.chunk(b"SCRN", |w| write_screens(w, &[]));
        chunks.chunk(b"FLIP", |w| write_flips(w, &[]));
        chunks.chunk(b"DESC", |w| write_descriptors(w, &[], layout));
        let bytes = lay_out(layout.anm_magic(), SLUG, &chunks);
        let Err(e) = read_anm(&bytes, "701.anm", layout, SLUG) else {
            panic!("four bytes nobody accounts for should not read");
        };
        assert!(
            e.to_string().contains("left over"),
            "should say what is left: {e}"
        );
    }

    /// A section the layout requires, missing.
    #[test]
    fn a_missing_section_is_named() {
        let layout = Generation::Motion32;
        let mut chunks = Writer::default();
        chunks.chunk(b"HEAD", |w| write_globals(w, &anim(layout)));
        chunks.chunk(b"SCRN", |w| write_screens(w, &[]));
        chunks.chunk(b"FLIP", |w| write_flips(w, &[]));
        let bytes = lay_out(layout.anm_magic(), SLUG, &chunks);
        let Err(e) = read_anm(&bytes, "701.anm", layout, SLUG) else {
            panic!("there is no DESC section");
        };
        assert!(e.to_string().contains("DESC"), "should name it: {e}");
    }

    /// The head is fixed-width, and the tests above index past it.
    const HEAD: usize = 8 + 4 + 4 + 4;

    /// The game these tests pretend to be.
    const SLUG: &str = "enviro";

    /// A replaced file is replaced whole, and nothing is left beside it.
    #[test]
    fn an_atomic_write_leaves_no_temporary_and_no_half_file() {
        let dir = std::env::temp_dir().join("motionvm-save-atomic");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let path = dir.join("701.blk");

        write_atomically(&path, b"the first save").expect("writes");
        assert_eq!(std::fs::read(&path).expect("reads"), b"the first save");

        // Over a shorter one: a plain write that truncated and then failed
        // would leave a prefix of the new bytes; this either replaces the
        // file or leaves it as it was.
        write_atomically(&path, b"a longer second save").expect("writes again");
        assert_eq!(
            std::fs::read(&path).expect("reads"),
            b"a longer second save"
        );

        let left: Vec<_> = std::fs::read_dir(&dir)
            .expect("lists")
            .filter_map(|e| e.ok().map(|e| e.file_name()))
            .collect();
        assert_eq!(left.len(), 1, "the temporary is gone: {left:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
