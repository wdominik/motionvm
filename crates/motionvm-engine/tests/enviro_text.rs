//! The 16-bit text drawer against Die Enviro-Kids greifen ein's own scenes.
//!
//! The rules under test are read from `ENVIRO.EXE` — the drawer at
//! `016a:0aac`, the run drawer at `14ee:11cf` — and the scene is the game's
//! own: the living-room arrival, whose first spoken line goes up without any
//! input. Needs the game's files; point `MOTIONVM_GAMEDATA_ENVIRO` at the
//! directory with `DATA.-1-`, or the test skips itself.

use motionvm_engine::{Game, titles};
use motionvm_forth::m16::Vm;
use motionvm_testutil::gamedata_enviro;
use std::path::Path;

/// `RUN` up to the first frame of `CTRL`, the intro clicked through — the
/// same walk `enviro_locations.rs` takes.
fn into_the_game(dir: &Path) -> Game<Vm> {
    let mut game = titles::enviro::open(dir).expect("opens");
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to the intro's ANIMPLAY") {}
    let mut left_intro = false;
    for frame in 1..=6000 {
        let click = frame % 200 == 0;
        game.set_input(160, 100, click, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        if !game.engine.main_loop() {
            left_intro = true;
        } else if left_intro {
            return game;
        }
    }
    panic!("6000 frames and the intro never gave way to CTRL");
}

/// A spoken line's outline sits one pixel around the letters on every side.
///
/// The shadow font's glyphs are the face's dilated by one — two wider, two
/// taller, measured over all 116 glyphs — and the drawer starts the shadow
/// pass one pixel up and left of the face: re-centered with its own metrics
/// on a centered axis, shifted by the template's `-1`/`-1` on an edge
/// (`016a:0e04`, `016a:0e2b`). Both together put the silhouette's bounding
/// box exactly one pixel outside the letters', which is what a recording of
/// the original shows and what this pins: drawing both passes at the same
/// point — the bug this test was written against — doubles the ring on the
/// right and bottom and bares the top left.
#[test]
fn a_spoken_line_wears_its_outline_all_around() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no ENVIRO gamedata");
        return;
    };
    let mut game = into_the_game(&dir);
    // The text is told from the scenery by difference: what the band shows
    // eight frames before the line goes up is the room, and the letters are
    // the light pixels that were not light then. Classification codes per
    // band pixel: 1 = light, 2 = the outline's dark blue, 0 = neither.
    let mut history: Vec<Vec<u8>> = Vec::new();
    for frame in 0..600 {
        game.set_input(10, 10, false, false, 0).expect("input");
        game.step().expect("a frame under CTRL");
        let fb = game.render();
        let pal = game.palette().clone();
        let w = fb.width as usize;
        let mut codes = vec![0u8; w * 40];
        for y in 8..48 {
            for x in 0..w {
                let [r, g, b] = pal.rgb8(fb.pixels[y * w + x]);
                codes[(y - 8) * w + x] = if r > 170 && g > 170 && b > 170 {
                    1
                } else if r < 90 && g < 90 && b > 90 {
                    2
                } else {
                    0
                };
            }
        }
        history.push(codes);
        if history.len() < 9 {
            continue;
        }
        let before = &history[history.len() - 9];
        let settled = &history[history.len() - 2];
        let now = history.last().unwrap();
        if now != settled {
            continue;
        }
        // The letters are what appeared **next to outline color** — a
        // letter pixel always has its ring within two pixels, which is what
        // tells it from a highlight the scene put up in the same frames.
        // The ring is then every outline-colored pixel inside and around
        // the letters' box, appeared or not, because part of it can stand
        // where the room already was that color.
        let near_ring = |i: usize| {
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            (-2..=2i32).any(|dy| {
                (-2..=2i32).any(|dx| {
                    let (nx, ny) = (x + dx, y + dy);
                    nx >= 0
                        && (nx as usize) < w
                        && (0..40).contains(&ny)
                        && now[ny as usize * w + nx as usize] == 2
                })
            })
        };
        let mut face = Vec::new();
        for (i, (&n, &b)) in now.iter().zip(before.iter()).enumerate() {
            if n == 1 && b != 1 && near_ring(i) {
                face.push(((i % w) as i32, (i / w) as i32 + 8));
            }
        }
        if face.len() < 300 {
            continue;
        }
        let fx = (
            face.iter().map(|p| p.0).min().unwrap(),
            face.iter().map(|p| p.0).max().unwrap(),
            face.iter().map(|p| p.1).min().unwrap(),
            face.iter().map(|p| p.1).max().unwrap(),
        );
        let mut ring = Vec::new();
        for (i, &n) in now.iter().enumerate() {
            let (x, y) = ((i % w) as i32, (i / w) as i32 + 8);
            if n == 2 && x >= fx.0 - 1 && x <= fx.1 + 1 && y >= fx.2 - 1 && y <= fx.3 + 1 {
                ring.push((x, y));
            }
        }
        if ring.len() < 200 {
            continue;
        }
        let bbox = |set: &[(i32, i32)]| {
            let xs: Vec<i32> = set.iter().map(|p| p.0).collect();
            let ys: Vec<i32> = set.iter().map(|p| p.1).collect();
            (
                *xs.iter().min().unwrap(),
                *xs.iter().max().unwrap(),
                *ys.iter().min().unwrap(),
                *ys.iter().max().unwrap(),
            )
        };
        let f = bbox(&face);
        let r = bbox(&ring);
        assert_eq!(
            (r.0, r.1, r.2, r.3),
            (f.0 - 1, f.1 + 1, f.2 - 1, f.3 + 1),
            "frame {frame}: the outline's box {r:?} does not ring the letters' {f:?}"
        );
        return;
    }
    panic!("600 frames under CTRL and no spoken line came up");
}
