//! Checks the renderer against the game's own artwork.
//!
//! Synthetic fills are not enough to pin a renderer. They exercise the
//! geometry and nothing else: a real sprite has a palette, a transparent
//! index, and a decoder in front of it, and a fault in any of those produces a
//! composed frame that is wrong with no way to tell whether the VM feeding it
//! or the drawing itself is at fault.
//!
//! So these tests take resources straight out of the game and assert that what
//! lands in the framebuffer is exactly what the decoder produced — no offsets,
//! no palette drift, no lost transparency.
//!
//! They need the original files and skip themselves without them.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_formats::Palette;
use motionvm_formats::m32::{Kind, Sprite, rsc::Bank};
use motionvm_render::{Display, Framebuffer, TRANSPARENT};
use motionvm_testutil::gamedata_ds2;

/// A big sprite from the game, plus a small one with transparency.
fn art() -> Option<(Sprite, Sprite)> {
    let bank = Bank::open_dir(gamedata_ds2()?).ok()?;
    let mut big: Option<Sprite> = None;
    let mut masked: Option<Sprite> = None;
    for (_, id) in bank.present(Kind::Gfx8) {
        let Ok(Some(item)) = bank.item(Kind::Gfx8, id) else {
            continue;
        };
        let Ok(s) = Sprite::parse(item) else { continue };
        if big.is_none() && s.width >= 320 && s.height >= 200 {
            big = Some(s.clone());
        }
        if masked.is_none()
            && s.width >= 16
            && s.pixels.contains(&TRANSPARENT)
            && s.pixels.iter().any(|&p| p != TRANSPARENT)
        {
            masked = Some(s);
        }
        if big.is_some() && masked.is_some() {
            break;
        }
    }
    Some((big?, masked?))
}

macro_rules! art_or_skip {
    () => {
        match art() {
            Some(a) => a,
            None => {
                eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
                return;
            }
        }
    };
}

/// A whole background must land in the buffer unchanged.
#[test]
fn a_background_blits_pixel_for_pixel() {
    let (big, _) = art_or_skip!();
    let mut fb = Framebuffer::new(big.width, big.height);
    fb.blit_masked(&big, 0, 0, None);

    assert_eq!(fb.pixels.len(), big.pixels.len());
    let differing = fb
        .pixels
        .iter()
        .zip(&big.pixels)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        differing, 0,
        "{differing} pixels differ from the decoded sprite"
    );
    eprintln!(
        "checked {}x{} = {} pixels",
        big.width,
        big.height,
        big.pixels.len()
    );
}

/// Transparent pixels must leave what was underneath alone, and opaque ones
/// must not.
#[test]
fn transparency_is_honored_on_real_art() {
    let (_, masked) = art_or_skip!();
    const UNDER: u8 = 0xAB;
    let mut fb = Framebuffer::new(masked.width, masked.height);
    fb.fill(UNDER);
    fb.blit(&masked, 0, 0);

    let (mut kept, mut drawn) = (0, 0);
    for y in 0..masked.height {
        for x in 0..masked.width {
            let src = masked.pixels[y as usize * masked.width as usize + x as usize];
            let got = fb.get(x as i32, y as i32).expect("inside the buffer");
            if src == TRANSPARENT {
                assert_eq!(
                    got, UNDER,
                    "transparent pixel at {x},{y} overwrote the background"
                );
                kept += 1;
            } else {
                assert_eq!(got, src, "opaque pixel at {x},{y} came out wrong");
                drawn += 1;
            }
        }
    }
    assert!(
        kept > 0 && drawn > 0,
        "the chosen sprite exercises both cases"
    );
    eprintln!("{drawn} pixels drawn, {kept} left showing through");
}

/// Drawing partly off the edge must clip, never wrap.
#[test]
fn drawing_past_the_edge_clips() {
    let (big, _) = art_or_skip!();
    let mut fb = Framebuffer::new(64, 64);
    fb.blit_masked(&big, -32, -32, None);

    for y in 0..64 {
        for x in 0..64 {
            let want = big.pixels[(y + 32) * big.width as usize + (x + 32)];
            assert_eq!(fb.get(x as i32, y as i32), Some(want), "at {x},{y}");
        }
    }
}

/// The palette chain must reproduce the sprite's own colors.
#[test]
fn palette_expansion_matches_the_sprites_own_colors() {
    let (big, _) = art_or_skip!();
    let mut fb = Framebuffer::new(big.width, big.height);
    fb.blit_masked(&big, 0, 0, None);

    let rgba = fb.to_rgba(&big.palette);
    assert_eq!(rgba.len(), big.pixels.len() * 4);
    for (i, &p) in big.pixels.iter().enumerate().step_by(97) {
        let [r, g, b] = big.palette.rgb8(p);
        assert_eq!(
            &rgba[i * 4..i * 4 + 4],
            &[r, g, b, 255],
            "pixel {i}, index {p}"
        );
    }
    // Six-bit DAC values must reach full scale, or everything comes out dim.
    assert_eq!(
        Palette::from_6bit(&[0x3f; Palette::BYTES]).rgb8(0),
        [255, 255, 255]
    );
}

/// The layout the game builds, drawn with real art on each layer.
#[test]
fn composing_the_games_screen_layout_keeps_each_layer_in_place() {
    let (big, _) = art_or_skip!();
    let mut d = Display::new();

    // Main picture, filling the top 400 rows.
    let main = d.new_screen();
    let s = d.screen_mut(main).expect("just created");
    s.set_size(640, 400);
    s.view = (640, 400);
    s.view_pos = (0, 0);
    s.buffer.blit_masked(&big, 0, 0, None);
    let main_top_left = s.buffer.get(0, 0);

    // Status bar underneath it.
    let status = d.new_screen();
    let s = d.screen_mut(status).expect("just created");
    s.set_size(640, 80);
    s.view = (640, 80);
    s.view_pos = (0, 400);
    s.buffer.fill(0x22);

    let out = d.compose();
    assert_eq!(out.width, 640);
    assert_eq!(out.height, 480);
    assert_eq!(
        out.get(0, 0),
        main_top_left,
        "main picture belongs at the top"
    );
    assert_eq!(
        out.get(0, 399),
        big.pixels.get(399 * big.width as usize).copied()
    );
    assert_eq!(out.get(0, 400), Some(0x22), "status bar starts at row 400");
    assert_eq!(out.get(639, 479), Some(0x22));
}
