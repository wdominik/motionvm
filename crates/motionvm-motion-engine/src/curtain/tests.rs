//! The curtains and the fade words: the bands the handlers compute, what a
//! fade reveals and hides, and how long it takes.

use crate::testing::*;
use crate::*;
use motionvm_motion_forth::m32;

/// The rows the handlers compute, checked against the disassembly.
///
/// `FADEIN` copies one band at `base + i` of height `(h/2 - i) * 2`, so the
/// visible rows are `[i, h - i)`. `FADEOUT` copies two bands of `step` at
/// `base + i` and `base + h - step - i`, leaving `[i + step, h - step - i)`
/// showing. Both run in steps of eight over 480 rows.
#[test]
fn the_curtain_matches_the_handlers() {
    let area = (0, 0, 640, 480);
    let screen = 0;
    let mut open = Curtain {
        opening: true,
        screen,
        area,
        step: 8,
        offset: 240,
        ticks_per_band: 1,
        banked: 0,
        palette_after: None,
    };
    assert_eq!(
        open.visible(),
        (240, 240),
        "closed at the middle to begin with"
    );
    open.advance(1);
    assert_eq!(
        open.visible(),
        (232, 248),
        "one step out is 16 rows around the center"
    );

    let mut shut = Curtain {
        opening: false,
        screen,
        area,
        step: 8,
        offset: 0,
        ticks_per_band: 1,
        banked: 0,
        palette_after: None,
    };
    assert_eq!(
        shut.visible(),
        (8, 472),
        "the first bands eat eight rows off each edge"
    );
    shut.advance(1);
    assert_eq!(shut.visible(), (16, 464));

    // Every intermediate band is centered on the screen's middle.
    for c in [&mut open, &mut shut] {
        while !c.done() {
            let (top, bottom) = c.visible();
            assert_eq!(
                top + bottom,
                480,
                "the band is symmetric at offset {}",
                c.offset
            );
            c.advance(1);
        }
    }

    let mut steps = 0;
    let mut c = Curtain {
        opening: true,
        screen,
        area,
        step: 8,
        offset: 240,
        ticks_per_band: 1,
        banked: 0,
        palette_after: None,
    };
    while !c.done() {
        c.advance(1);
        steps += 1;
    }
    assert_eq!(steps, 31, "480 rows in steps of eight from the middle out");
}

/// Repaints screen 1's descriptor in `color`, ready for a fade to reveal.
fn repaint(e: &mut Engine, color: u8) {
    let sprite = e.scene.sprites.get_mut(&10).expect("the top sprite");
    sprite.pixels.fill(color);
}

fn fade(e: &mut Engine, mem: &mut m32::Memory, name: &str, screen: u32) {
    e.display.set_current(screen);
    let mut stack = vec![1, 50, 8];
    e.plain_word32(name, &mut stack, mem).unwrap();
}

/// A fade in reveals its picture *over* what is on the screen.
///
/// The handler draws the screen once (0x74af9), wipes the update map
/// (0x74afe) and then marks one growing band. Marking is all it does:
/// outside the band the presenter copies nothing, so video memory still
/// holds the previous frame. Blacking that out instead — which is right
/// for `FADEOUT`, whose surface really is filled with color 0 first — is
/// what made every menu page and help page blink through black, because
/// the menu fades in without ever fading out (module 4, 0x01af4, 0x01bb4,
/// 0x03bc0, 0x03be8 …).
#[test]
fn a_fade_in_reveals_its_picture_over_the_old_one() {
    let mut mem = mem();
    let mut e = two_screens(3, 7);
    assert_eq!(
        e.render().get(0, 0),
        Some(3),
        "the old picture is on screen"
    );

    repaint(&mut e, 5);
    fade(&mut e, &mut mem, "FADEIN", 1);

    // The band opens from row 16 outwards in steps of eight. While it is
    // part-way, the middle must be the new picture and both edges the old.
    let mut seen_middle = false;
    while e.in_transition() {
        e.advance_curtain();
        let frame = e.render();
        let (top, bottom) = (frame.get(0, 0), frame.get(0, 31));
        let middle = frame.get(0, 16);
        if middle == Some(5) && top == Some(3) {
            seen_middle = true;
            assert_eq!(
                bottom,
                Some(3),
                "the bottom edge still shows the old picture too"
            );
        }
        assert_eq!(
            frame.get(0, 40),
            Some(7),
            "the other screen is not in this fade"
        );
    }
    assert!(
        seen_middle,
        "the fade never showed new middle and old edges at once"
    );
    assert_eq!(
        e.render().get(0, 0),
        Some(5),
        "and it ends on the new picture"
    );
}

/// A fade out queued before a fade in hides the *old* picture.
///
/// The shape a single nested call produces — `DO_INVSEL`'s documents branch
/// does exactly this, `FADEOUT` on the picture, descriptors swapped,
/// `FADEIN` on the picture, all before the interpreter pauses (module 4,
/// 0x01c6c–0x01ca8). The later `FADEIN` has already drawn the new picture
/// into the screen buffer by the time the earlier `FADEOUT` plays, so a
/// fade out that composed the buffers afresh would hide a picture nobody
/// had seen yet.
#[test]
fn a_queued_fade_out_hides_the_picture_that_was_showing() {
    let mut mem = mem();
    let mut e = two_screens(3, 7);

    fade(&mut e, &mut mem, "FADEOUT", 1);
    repaint(&mut e, 5);
    fade(&mut e, &mut mem, "FADEIN", 1);
    assert_eq!(
        e.transitions.curtains.len(),
        2,
        "both curtains are queued before either plays"
    );

    let mut closing = true;
    while e.in_transition() {
        if closing && e.transitions.curtains.front().map(|c| c.opening) == Some(true) {
            closing = false;
        }
        e.advance_curtain();
        let frame = e.render();
        if closing {
            for row in 0..32 {
                assert_ne!(
                    frame.get(0, row),
                    Some(5),
                    "the new picture showed during the fade out"
                );
            }
        }
    }
    assert_eq!(
        e.render().get(0, 0),
        Some(5),
        "the fade in brings the new picture up"
    );
}

/// The band width comes from the field each handler reads.
///
/// `FADEIN` takes `+0x18` at 0x74b22 — `SCRSIZE`, the whole surface — and
/// `FADEOUT` takes `+0x1C` at 0x74d70, the `SCRVSIZE` window. No screen the
/// game builds sets the two apart, so this is the only place the difference
/// can be seen at all.
#[test]
fn the_two_directions_read_different_width_fields() {
    let mut mem = mem();
    let mut e = two_screens(3, 7);
    e.display.screen_mut(1).expect("screen 1").view = (8, 32);

    fade(&mut e, &mut mem, "FADEIN", 1);
    assert_eq!(
        e.transitions.curtains.front().expect("a curtain").area.2,
        16,
        "a fade in spans SCRSIZE"
    );
    e.transitions.curtains.clear();

    fade(&mut e, &mut mem, "FADEOUT", 1);
    assert_eq!(
        e.transitions.curtains.front().expect("a curtain").area.2,
        8,
        "a fade out spans SCRVSIZE"
    );
}

/// No pointer while a curtain runs.
///
/// Both handlers call `HIDEMOUSE` (0x74ae2) before the band loop and
/// `SHOWMOUSE` (0x74b64) after it.
#[test]
fn the_pointer_stays_away_while_a_curtain_runs() {
    let mut mem = mem();
    let mut e = two_screens(3, 7);
    e.scene.sprites.insert(
        99,
        Picture {
            width: 2,
            height: 2,
            pixels: vec![9; 4],
        },
    );
    e.cursor_state.shape = Some(PointerShape::Sprite {
        id: 99,
        hot_x: 0,
        hot_y: 0,
    });
    e.cursor_state.shows = 1;
    e.input.mouse.x = 0;
    e.input.mouse.y = 0;
    assert_eq!(
        e.render().get(0, 0),
        Some(9),
        "the pointer draws when nothing is fading"
    );

    fade(&mut e, &mut mem, "FADEOUT", 1);
    assert_eq!(
        e.render().get(0, 0),
        Some(3),
        "and not while a curtain is up"
    );
    assert!(
        e.cursor_state.visible(),
        "the script's own state is left alone"
    );
}

/// `FADEIN`'s mode 2 is the curtain with `WHITEBOX`'s box painted
/// first — at 25,122, 452 by 317, the frame two in — and no wait
/// between its bands: one step opens it whole, and the box is on the
/// screen's surface under everything the bands reveal.
#[test]
fn a_mode_two_fade_in_paints_the_box_and_opens_at_once() {
    let mut mem = mem();
    let mut e = Engine::new(Profile::motion32());
    e.select_mode(video::MODE_640X480X256);
    e.enter_graphics().expect("the mode");
    // A palette with a white at 5 and a black at 9 among grays, so the
    // two indices are neither each other nor the surface's own 0.
    let mut raw = [20u8; motionvm_render::Palette::BYTES];
    raw[15..18].copy_from_slice(&[63, 63, 63]);
    raw[27..30].copy_from_slice(&[0, 0, 0]);
    e.display.palette = motionvm_render::Palette { raw };
    let (white, black) = (5, 9);
    let h = e.display.new_screen();
    let s = e.display.screen_mut(h).unwrap();
    s.set_size(640, 480);
    s.set_view(640, 480);
    e.display.set_current(h);
    let mut stack = vec![2, 50, 8];
    e.plain_word32("FADEIN", &mut stack, &mut mem).unwrap();
    assert!(e.in_transition(), "mode 2 is the curtain");
    assert_eq!(e.step_ticks(), 0, "and it waits for nothing");
    e.advance_curtain();
    assert!(!e.in_transition(), "one step opens it whole");
    let picture = e.render();
    let at = |x, y| picture.get(x, y);
    assert_eq!(at(25, 122), Some(white), "the box's corner");
    assert_eq!(at(27, 124), Some(black), "the frame, two in");
    assert_eq!(at(30, 130), Some(white), "white inside the frame");
    assert_eq!(at(476, 438), Some(white), "the box's far corner");
    assert_eq!(at(477, 439), Some(0), "and nothing past it");
}

/// `FADEOUT`'s mode 2 is a different effect — a translucent fade through
/// the darkening tables (0x74eef, color 0x102) — that no shipped call
/// reaches, so it is recorded and skipped. It must not go through
/// `note_no_effect`, whose `debug_assert!` fires for any word
/// `Word::inert` does not admit, and neither fade word is admitted.
#[test]
fn a_mode_two_fade_out_is_recorded_rather_than_fatal() {
    let mut mem = mem();
    let mut e = two_screens(3, 7);
    e.display.set_current(1);
    let mut stack = vec![2, 50, 8];
    e.plain_word32("FADEOUT", &mut stack, &mut mem).unwrap();
    assert!(!e.in_transition(), "mode 2 is not the curtain");
    assert_eq!(
        e.stubbed().get("FADEOUT (mode 2)"),
        Some(&1),
        "and it is reported, not swallowed"
    );
}

/// R78's curtains wait for nothing: with the capability off, a mode-1
/// fade opens in one step of no ticks.
#[test]
fn an_unwaiting_build_opens_its_curtain_in_one_step() {
    let mut mem = mem();
    let mut e = two_screens(3, 7);
    e.profile.curtains_wait = false;
    e.display.set_current(1);
    let mut stack = vec![1, 50, 8];
    e.plain_word32("FADEIN", &mut stack, &mut mem).unwrap();
    assert!(e.in_transition());
    assert_eq!(e.step_ticks(), 0);
    e.advance_curtain();
    assert!(!e.in_transition());
}

/// Every fade's pace, worked out from the handler and checked at three
/// heights — including the one the game only reaches in its title.
///
/// The handler divides twice and the two divisions do different jobs:
///
/// ```text
/// bands = view_height / 16     0x74cd0   only ever the divisor below
/// half  = view_height / 2      0x74ce7
/// delay = duration / bands     0x74cf6   signed, truncating, no floor
/// loop: i = 0 … half step 8    0x74d55   -> half/8 + 1 passes
/// ```
///
/// So the pass count is *not* `bands`, and the total is
/// `(half/8 + 1) * (50 / (h/16))`. Truncation makes the big screen the
/// fast one: 50/30 is 1 where 50/25 is 2, so the 480-row title fades in
/// 31 ticks against the 400-row game's 52. That is the original, and the
/// reason the intro's fades are quicker than everything after them.
#[test]
fn the_pace_of_a_fade_follows_the_two_divisions() {
    // height -> ticks a band, passes, ticks in total
    for (height, per, passes, total) in [(480, 1, 31, 31), (400, 2, 26, 52), (80, 10, 6, 60)] {
        let mut mem = mem();
        let mut e = Engine::new(Profile::motion32());
        let mut s = Screen::new(1);
        s.size = (640, cell::low16(height));
        s.view = (640, cell::low16(height));
        s.buffer = Framebuffer::new(640, cell::low16(height));
        e.display.screens.push(s);
        e.display.set_current(1);

        let mut stack = vec![1, 50, 8];
        e.plain_word32("FADEOUT", &mut stack, &mut mem).unwrap();
        let c = e.transitions.curtains.front().expect("a curtain").clone();
        assert_eq!(c.ticks_per_band, per, "{height} rows: 50 / ({height}/16)");
        assert_eq!(e.step_ticks(), per, "and a step is one band's worth");

        let (mut steps, mut ticks) = (0, 0);
        while e.in_transition() {
            ticks += e.step_ticks();
            e.advance_curtain();
            steps += 1;
            assert!(steps < 500, "{height} rows: the curtain never finished");
        }
        assert_eq!(steps, passes, "{height} rows: half/8 + 1 passes");
        assert_eq!(ticks, total, "{height} rows: one wait a pass");
    }
}
