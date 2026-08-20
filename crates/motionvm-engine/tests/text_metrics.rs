//! Checks the text renderer's measurements against the original engine's.
//!
//! The one pixel of spacing between glyphs was first fitted to a screenshot,
//! which is a weak way to establish something the whole game depends on. It is
//! now read out of the engine's own code, and the numbers below come from
//! asking the 1996 engine to measure text for itself.
//!
//! ## What the engine does
//!
//! The routine at `0x25df6` measures one line:
//!
//! ```text
//! per glyph:  total += glyph_width + [0xd66ee]
//! at the end: total -= [0xd66ee]
//! ```
//!
//! and `0x2619c` builds the block from it:
//!
//! ```text
//! height = lines * font_height + (lines - 1) * [0xd66ec]
//! width  = max over lines
//! ```
//!
//! Both gaps are **globals**, not font fields: `0xd66ec` and `0xd66ee`, each
//! holding 1. Nothing the game can call writes them — the two setters that do
//! are internal and no kernel word reaches them — so the spacing is the same
//! for every font, which is exactly what was in doubt.
//!
//! `SDTXT` then stores the measured width **plus four** into the descriptor,
//! and that padded value is what `GDWIDTH` reports. Hence the four below.

use motionvm_engine::Game;
use motionvm_formats::{Kind, TextTable, rsc::Bank};
use motionvm_render::Framebuffer;
use motionvm_testutil::gamedata;

/// What `GDWIDTH` came back with in the original, for entries of text table 6
/// with no font chosen — so the system font — as `(SDTXT number, width)`.
///
/// Measured with fonts 5, 6 and 8 each registered in turn. All three gave these
/// same numbers, which is how it became clear that `+FONT` registers a font
/// without selecting it.
const ORIGINAL_WIDTHS: [(usize, i32); 3] = [(1, 43), (2, 91), (3, 191)];

/// The four `SDTXT` adds to the measured width before storing it.
const DESCRIPTOR_PADDING: i32 = 4;

#[test]
fn text_measures_the_same_as_in_the_original() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let game = Game::open(&dir).expect("game opens");
    let bank = Bank::open_dir(&dir).expect("resources");
    let item = bank.item(Kind::Text, 6).expect("read").expect("table 6");
    let table = TextTable::parse(item).expect("table parses");
    let refs = game.engine.font_refs().expect("000.FRT");
    let font = game.engine.system_font().expect("000.FNT");

    for (number, engine_width) in ORIGINAL_WIDTHS {
        // Text numbers are one-based.
        let text = &table.strings[number - 1];
        let ours = Framebuffer::text_width(font, refs, text);
        assert_eq!(
            ours + DESCRIPTOR_PADDING,
            engine_width,
            "SDTXT {number} ({text:?}): the original measured {engine_width}, we get {ours} + {DESCRIPTOR_PADDING}"
        );
    }
}

/// A line of one glyph has no gap in it, two glyphs have exactly one.
///
/// The trailing gap is subtracted by the engine, and getting that wrong is
/// invisible on a single line and off by one on every line of a paragraph.
#[test]
fn the_gap_falls_between_glyphs_and_not_after_the_last() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let game = Game::open(&dir).expect("game opens");
    let refs = game.engine.font_refs().expect("000.FRT");
    let font = game.engine.system_font().expect("000.FNT");

    let glyph = refs
        .glyph_for(b'M')
        .and_then(|i| font.glyphs.get(i as usize))
        .expect("an M");
    let w = glyph.width as i32;
    assert_eq!(Framebuffer::text_width(font, refs, "M"), w);
    assert_eq!(Framebuffer::text_width(font, refs, "MM"), 2 * w + 1);
    assert_eq!(Framebuffer::text_width(font, refs, "MMMM"), 4 * w + 3);
    assert_eq!(Framebuffer::text_width(font, refs, ""), 0);
}

/// A text descriptor whose entry is empty draws nothing at all — not even the
/// backing its color asks for.
///
/// The park keeps two of them: handle 53 on table 1 and handle 59 on table 8,
/// both parked on entry 1, which is empty. That is the resting state, and the
/// original keeps them invisible in the drawer rather than by deactivating
/// them: with the backing flag already set (0x69f5d) it runs the layout, tests
/// its +0x18 against 1 (`cmpw $1` at 0x69f75) and clears the flag again, so the
/// backing block is jumped over at 0x69f9e. That field is `strlen` of the
/// layout's own text pointer (0x6cdd2-0x6cde2 through 0x11d0e).
///
/// Without it each one painted a 16 x 34 rectangle of darkened background —
/// four wide and one line tall once `SDTDT` has added its 4, plus the 12 that
/// `backing_rect` puts around it — standing in the picture with nothing in it.
///
/// Rendering twice, once with the empty descriptor switched off, is the whole
/// assertion: if an empty text contributes nothing, the two frames are equal.
#[test]
fn an_empty_text_draws_nothing() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(2).expect("the park");

    let empty: Vec<u32> = game
        .engine
        .descriptors()
        .to_vec()
        .iter()
        .filter(|d| d.active && d.kind == motionvm_engine::DescriptorKind::Text)
        .filter(|d| game.engine.descriptor_text(d).is_some_and(|t| t.is_empty()))
        .map(|d| d.handle)
        .collect();
    assert!(
        !empty.is_empty(),
        "the park should be holding an empty text descriptor"
    );

    // Draw and present around each render, or the two frames are the same
    // frame. `render` hands back what is on screen; only the drawer puts a
    // picture in the buffers and only the presenter (0x1457D) makes it
    // visible. Without both, switching a descriptor off changes nothing that
    // could be seen and the comparison below passes whatever the drawer does —
    // checked by putting the backing back and watching this test stay green.
    game.engine.draw();
    game.engine.present();
    let with = game.render();
    for h in &empty {
        let d = game
            .engine
            .descriptors_mut()
            .iter_mut()
            .find(|d| d.handle == *h)
            .expect("just found");
        d.active = false;
    }
    game.engine.draw();
    game.engine.present();
    let without = game.render();
    let differing = with
        .pixels
        .iter()
        .zip(&without.pixels)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        differing, 0,
        "the empty text descriptors {empty:?} put {differing} pixels on screen"
    );
}
