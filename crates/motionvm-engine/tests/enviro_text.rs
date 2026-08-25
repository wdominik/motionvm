//! The 16-bit text drawer against Die Enviro-Kids greifen ein's own scenes.
//!
//! The rules under test are read from `ENVIRO.EXE` — the drawer at
//! `016a:0aac`, the run drawer at `14ee:11cf` — and the scene is the game's
//! own: the living-room arrival, whose first spoken line goes up without any
//! input. Needs the game's files; point `MOTIONVM_GAMEDATA_ENVIRO` at the
//! directory with `DATA.-1-`, or the test skips itself.

use motionvm_engine::{Game, Playable, titles};
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

/// Past the scrapyard arrival, the same way `enviro_locations.rs` gets
/// there: location 1's scene plays itself out and moves on to location 11.
fn settled_in_the_game(dir: &Path) -> Game<Vm> {
    let mut game = into_the_game(dir);
    let ms = game.get_var(601, "_MS").expect("_MS") as u32;
    let walker = game.get_var(601, "_WALKER").expect("_WALKER") as u32;
    for frame in 1..=3000 {
        let active = game
            .engine
            .descriptors()
            .iter()
            .find(|d| d.screen == ms && d.handle == walker)
            .is_some_and(|d| d.active);
        if active
            && game.get_var(601, "ACTLOC") == Some(11)
            && game.get_var(601, "NEXTLOC") == Some(-1)
        {
            return game;
        }
        game.set_input(2, 2, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("arrival frame {frame} stopped: {e}"));
    }
    panic!("the arrival never settled in location 11");
}

/// A morning-scene line on template 2 keeps its ring past the intro's
/// font teardown.
///
/// The intro loads its own shadow font and defines templates 6 and 2 over
/// it (`0 SFT 2 +FONT _SHFONT ! … 38 6 XDEFTDT 16 2 XDEFTDT`, module 610),
/// then frees that font on its way out (`_SHFONT @ -FONT`). `RUN` defines
/// all nine templates afresh over a new handle right after (module 100,
/// past `=>ERASE`) — and that replaces the intro's entries, because
/// `DEFTDT` writes a fixed table indexed by the id (`ds:0x1A0C`, ten bytes
/// an entry). Appending instead kept the intro's entry first in line, its
/// font gone, and every text on templates 2 and 6 lost its outline for
/// the whole game — Eva's lines among them: `SAY_EVA` (module 112) says
/// "Alter Sprücheklopfer!" through `… 52 12 2 DOXINFO`, face color 12 on
/// template 2, whose shadow color is 16 (`16 2 XDEFTDT`).
#[test]
fn a_morning_scene_line_keeps_its_ring_past_the_intros_font_teardown() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no ENVIRO gamedata");
        return;
    };
    let mut game = settled_in_the_game(&dir);
    // The scene sits behind `DOS_EX @ 2 =` in location 12's task
    // (`MYCALCMOVE`, module 112, phase 2): the morning briefing that walks
    // `B_D01` through TXT 2, Eva's entries in face color 12.
    game.set_var(607, "DOS_EX", 2).expect("DOS_EX");
    game.request_location(12).expect("NEXTLOC");
    let mut history: Vec<Vec<u8>> = Vec::new();
    for frame in 1..=3000 {
        game.set_input(2, 2, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        let fb = game.render();
        let w = fb.width as usize;
        // The band the spoken lines stand in: `y 52` centered, three lines
        // at most. Indices, not colors — the face is palette index 12, the
        // ring index 16, straight from the script.
        let band: Vec<u8> = fb.pixels[8 * w..90 * w].to_vec();
        history.push(band);
        if history.len() < 9 {
            continue;
        }
        let before = &history[history.len() - 9];
        let settled = &history[history.len() - 2];
        let now = history.last().unwrap();
        if now != settled {
            continue;
        }
        // The letters are the face-colored pixels that were not there
        // eight frames ago; the scenery's own index-12 pixels cancel out.
        let face: Vec<(i32, i32)> = now
            .iter()
            .zip(before.iter())
            .enumerate()
            .filter(|&(_, (&n, &b))| n == 12 && b != 12)
            .map(|(i, _)| ((i % w) as i32, (i / w) as i32))
            .collect();
        if face.len() < 250 {
            continue;
        }
        let f = (
            face.iter().map(|p| p.0).min().unwrap(),
            face.iter().map(|p| p.0).max().unwrap(),
            face.iter().map(|p| p.1).min().unwrap(),
            face.iter().map(|p| p.1).max().unwrap(),
        );
        let ring: Vec<(i32, i32)> = now
            .iter()
            .enumerate()
            .filter(|&(i, &n)| {
                let (x, y) = ((i % w) as i32, (i / w) as i32);
                n == 16 && x >= f.0 - 1 && x <= f.1 + 1 && y >= f.2 - 1 && y <= f.3 + 1
            })
            .map(|(i, _)| ((i % w) as i32, (i / w) as i32))
            .collect();
        if ring.len() < 100 {
            continue;
        }
        let r = (
            ring.iter().map(|p| p.0).min().unwrap(),
            ring.iter().map(|p| p.0).max().unwrap(),
            ring.iter().map(|p| p.1).min().unwrap(),
            ring.iter().map(|p| p.1).max().unwrap(),
        );
        assert_eq!(
            r,
            (f.0 - 1, f.1 + 1, f.2 - 1, f.3 + 1),
            "frame {frame}: the outline's box does not ring the letters'"
        );
        return;
    }
    panic!("3000 frames in location 12 and no template-2 line wore a ring");
}
