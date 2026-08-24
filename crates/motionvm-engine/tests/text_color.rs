//! What color a text is drawn in when nothing chose one.
//!
//! The help pages are the case: `SHOW_DOC` (module 4, 0x002e8) contains no
//! `SDCOL` at all, and the descriptors it uses — `_ANT1`…`_ANT10`, built by
//! `XYLTITEM.` (module 2, 0x01364) — are never given a color by any script in
//! the game. Exactly one place ever writes one on any of them:
//! `223:LTMANAGER` sets `_ANT1` white for the title text and back to **0** in
//! its last phase. Everything else runs on the default.
//!
//! In the original that default is not a choice, it is the allocator. A
//! descriptor has no text record until `SDTXT` (0x71b4f) or `SDTB` (0x71d45)
//! makes one, and both allocate through 0x203F3 → 0x823E0:
//!
//! ```text
//! 000823e5  call 0x000A1764
//! 000823ee  xor  %al,%al
//! 000823f0  rep  stos %al,(%edi)
//! ```
//!
//! `SDTB` then zeroes +0x00, +0x04, +0x08, +0x10 and +0x18 by hand and leaves
//! **+0x0C**, the color, to that. The drawer reads it back masked
//! (`and $0xFF` at 0x6a21c) and writes the byte straight into the picture. So
//! an untouched text draws in index 0 — black in 54 of the 60 shipped
//! palettes.
//!
//! The game data this file drives is Dunkle Schatten 2's (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_testutil::gamedata_ds2;

/// A game standing in the title with the intro's fades run out.
fn settled(dir: &std::path::Path) -> Game {
    let mut game = Game::open(dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    for _ in 0..2000 {
        game.set_input(300, 440, false, false, 0).expect("input");
        game.step().expect("a frame");
        if game.get_var(2, "_IMX") == Some(300) && !game.engine.in_transition() {
            return game;
        }
    }
    panic!("the game never settled");
}

fn shown(game: &mut Game) -> motionvm_render::Framebuffer {
    game.engine.draw();
    game.engine.present();
    game.render()
}

/// A text nobody gave a color is drawn in index 0, and `GDCOL` agrees.
///
/// Built the way `STARTUP` builds the help pages' descriptors — through the
/// game's own `XYLTITEM.`, which sets a table, an entry and a font and no
/// color — so the default under test is the one the pages actually run on.
///
/// **The two halves matter together**, because two readers of one field can
/// disagree without either of them failing: a drawer that takes 255 for an
/// unset color and a `GDCOL` that takes 0 both look reasonable alone. 255 is
/// magenta in 42 of the 60 shipped palettes, so the disagreement shows up as
/// magenta help text and nowhere else.
#[test]
fn a_text_with_no_color_is_drawn_in_index_zero() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = settled(&dir);

    // `0 0 129 6 XYLTITEM.` — the shape module 3 uses, on text table 6.
    game.call(2, "XYLTITEM.", &[40, 200, 129, 6])
        .expect("2:XYLTITEM.");
    let handle = game
        .engine
        .descriptors()
        .last()
        .expect("a descriptor")
        .handle;
    let d = game
        .engine
        .descriptors_mut()
        .iter_mut()
        .find(|d| d.handle == handle)
        .expect("just made");
    assert_eq!(
        d.color, 0,
        "XYLTITEM. gives no color, so it is the zeroed default"
    );
    // Entry 40 of table 6 is one of the help pages' own paragraphs.
    d.text = Some(40);
    d.screen = 2;
    d.active = true;

    let frame = shown(&mut game);
    let inked: Vec<u8> = (200..240)
        .flat_map(|y| (40..620).map(move |x| (x, y)))
        .filter_map(|(x, y)| frame.get(x, y))
        .collect();
    let zeros = inked.iter().filter(|&&p| p == 0).count();
    let magenta = inked.iter().filter(|&&p| p == 255).count();
    assert!(
        zeros > 100,
        "the text should be there in index 0, got {zeros} such pixels"
    );
    assert_eq!(magenta, 0, "{magenta} pixels came out in index 255");
}

/// A help page is black on white, through the branch the game itself runs.
///
/// `_INVMODE` is *not* set by hand here, and that is the point. Case 1005 of
/// `DO_INVSEL` (module 4, 0x01bc4) is what a click on "Info" reaches — through
/// `CALLMENU`, which arms it as a descriptor callback with `SDWORD`/`SDWAIT` —
/// and besides setting the mode it activates `_ANL1` and `_ANL2`. Those are
/// the page: module 3 builds them as `0 0 128 60 XYLBITEM.` and
/// `320 0 128 60 XYLBITEM.`, two halves of **image 60**, which is a plain
/// 320x400 field of index 2. Setting `_INVMODE` directly skips them and leaves
/// the text standing on nothing — which is how a first look at this made the
/// page appear to have no paper at all.
///
/// Sprite 61, the one `SHOW_DOC` does place, is 600x3 pixels of index 9: a
/// rule under the heading, not a background.
#[test]
fn a_help_page_is_black_text_on_the_pages_own_paper() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = settled(&dir);

    game.set_var(2, "_MENCZW", 1005)
        .expect("the documents button");
    game.call(4, "DO_INVSEL", &[]).expect("4:DO_INVSEL");
    assert_eq!(game.get_var(2, "_INVMODE"), Some(5), "the viewer is open");
    assert_eq!(game.get_var(2, "_DOC"), Some(1), "on its first page");

    // The paper, before anything is asked about the text.
    let paper: Vec<u32> = game
        .engine
        .descriptors()
        .iter()
        .filter(|d| d.active && d.block == Some(60))
        .map(|d| d.handle)
        .collect();
    assert_eq!(
        paper.len(),
        2,
        "both halves of image 60 are showing: {paper:?}"
    );

    let frame = shown(&mut game);
    let page: Vec<u8> = (0..400)
        .flat_map(|y| (0..640).map(move |x| (x, y)))
        .filter_map(|(x, y)| frame.get(x, y))
        .collect();
    let magenta = page.iter().filter(|&&p| p == 255).count();
    assert_eq!(
        magenta, 0,
        "{magenta} pixels of the help page are index 255"
    );
    let white = page.iter().filter(|&&p| p == 2).count();
    assert!(
        white > 200_000,
        "the page should be mostly paper, got {white} pixels of index 2"
    );
    let black = page.iter().filter(|&&p| p == 0).count();
    assert!(black > 500, "and carry text in index 0, got {black} pixels");
}

/// A color that *was* set is left exactly as it was given.
///
/// `STARTUP` builds the info line with `274 SDCOL` — 18 with the composite bit
/// that asks for a darkened backing. The low byte is the pen (`and $0xFF` at
/// 0x6a21c) and the value above 256 is what the drawer tests at 0x69f50, so
/// both halves have to survive. A fix that pushed every text to index 0 would
/// break this and nothing else would notice.
#[test]
fn a_color_that_was_set_survives_whole() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = settled(&dir);

    game.call(2, "XYLTITEM.", &[40, 300, 129, 6])
        .expect("2:XYLTITEM.");
    let handle = game
        .engine
        .descriptors()
        .last()
        .expect("a descriptor")
        .handle;
    let d = game
        .engine
        .descriptors_mut()
        .iter_mut()
        .find(|d| d.handle == handle)
        .expect("just made");
    d.text = Some(40);
    d.screen = 2;
    d.active = true;
    d.color = 274;

    let frame = shown(&mut game);
    let inked: Vec<u8> = (300..340)
        .flat_map(|y| (40..620).map(move |x| (x, y)))
        .filter_map(|(x, y)| frame.get(x, y))
        .collect();
    assert!(
        inked.contains(&18),
        "the pen is the low byte of 274, so index 18"
    );
}
