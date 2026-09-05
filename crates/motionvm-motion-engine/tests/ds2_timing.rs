//! Where a frame's time goes. Not a test — a measurement rig.
//!
//! It asserts nothing, because a wall-clock number is not a property of the
//! code but of the machine it ran on. It exists so that any figure quoted
//! about where a frame's time goes can be reproduced and argued with rather
//! than believed, and so that an optimization is chosen after a measurement
//! rather than before one. Plausible arguments about frame cost are routinely
//! wrong in both directions: the obvious candidate turns out to cost nothing
//! measurable, and the real hot spot sits in a routine nobody suspected.
//!
//! `#[ignore]`d, so `just check` skips it; `just bench` runs it, together with
//! `throughput.rs`, which measures the other thing a frame is made of — how
//! fast the machine that produced the scene ran.
//!
//! Release only. A debug build measures the optimizer, not the code.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_motion_engine::Game;
use motionvm_motion_forth::m32::Vm;
use motionvm_motion_testutil::gamedata_ds2;
use motionvm_render::Framebuffer;
use std::time::Instant;

/// Frames of nothing in particular, to get past startup into a real scene.
fn play(game: &mut Game<Vm>, frames: usize) {
    for _ in 0..frames {
        game.set_input(0, 0, false, false, 0).unwrap();
        game.step().unwrap();
    }
}

#[test]
#[ignore]
fn where_the_frame_time_goes() {
    let Some(dir) = gamedata_ds2() else { return };
    let mut game = Game::open(&dir).unwrap();
    game.start().unwrap();
    while game.pump().unwrap() {}
    game.set_var(2, "_NEXTLOC", 1).unwrap();
    play(&mut game, 400);

    const N: u32 = 2_000;
    let us = |d: std::time::Duration| d.as_secs_f64() * 1e6 / f64::from(N);
    let time = |f: &mut dyn FnMut()| {
        let t = Instant::now();
        for _ in 0..N {
            f();
        }
        t.elapsed()
    };

    // The three stages of a frame, each on its own and over the same state.
    let draw = time(&mut || game.engine.draw());
    let present = time(&mut || game.engine.present());
    // The two ways a frame leaves the engine, because they differ by a copy:
    // `frame` composes into the buffer the engine keeps and lends it out,
    // which is what a window does; `render` composes and hands the picture
    // over owned, which is what a test that keeps one does.
    let frame = time(&mut || {
        std::hint::black_box(game.engine.frame().pixels.width);
    });
    let render = time(&mut || {
        std::hint::black_box(game.engine.render());
    });
    let step = {
        let t = Instant::now();
        play(&mut game, usize::try_from(N).unwrap());
        t.elapsed()
    };

    println!("\n--- a frame, microseconds per call ---");
    println!("draw()      {:8.1}", us(draw));
    println!("present()   {:8.1}", us(present));
    println!(
        "frame()     {:8.1}   (borrowed, what a window takes)",
        us(frame)
    );
    println!("render()    {:8.1}   (owned, one copy more)", us(render));
    println!("step()      {:8.1}   (the whole frame)", us(step));

    // Each piece measured on its own, so its share of the frame is arguable
    // rather than asserted.
    let palette = game.engine.palette().clone();
    println!("\n--- the pieces ---");
    println!(
        "backing_map()        {:8.1}   (the translucency table)",
        us(time(&mut || {
            std::hint::black_box(motionvm_motion_engine::text::backing_map(&palette));
        }))
    );
    println!(
        "compose()            {:8.1}   (layering the screens)",
        us(time(&mut || {
            std::hint::black_box(game.engine.compose());
        }))
    );

    let ids: Vec<u32> = game
        .engine
        .descriptors()
        .iter()
        .filter(|d| d.active)
        .filter_map(|d| d.shows.graphic())
        .collect();
    let sprites: Vec<_> = ids
        .iter()
        .filter_map(|id| game.engine.cached_sprite(*id).cloned())
        .collect();
    let pixels: usize = sprites
        .iter()
        .map(|s| usize::from(s.width) * usize::from(s.height))
        .sum();
    let mut scratch = Framebuffer::new(640, 480);
    println!(
        "blit_scaled x{:<3}     {:8.1}   {pixels} sprite pixels",
        sprites.len(),
        us(time(&mut || {
            for s in &sprites {
                scratch.blit_scaled(s, 0, 0, 1000, 1000);
            }
        }))
    );

    println!("\n--- what a frame copies ---");
    println!(
        "sprite clone x{:<3}    {:8.1}",
        ids.len(),
        us(time(&mut || {
            for id in &ids {
                std::hint::black_box(game.engine.cached_sprite(*id).cloned());
            }
        }))
    );
    println!(
        "descriptor clone x{:<3}{:8.1}",
        game.engine.descriptors().len(),
        us(time(&mut || {
            for d in game.engine.descriptors() {
                std::hint::black_box(d.clone());
            }
        }))
    );
    let font = game.engine.system_font().cloned().unwrap();
    println!(
        "font clone           {:8.1}   (twice per text descriptor)",
        us(time(&mut || {
            std::hint::black_box(font.clone());
        }))
    );

    // What the scene actually contains, so the shares above can be scaled.
    let texts: Vec<_> = game
        .engine
        .descriptors()
        .iter()
        .filter(|d| d.active && d.is_text())
        .cloned()
        .collect();
    let backed = texts
        .iter()
        .filter(|d| d.color >= 0x100)
        .filter(|d| {
            game.engine
                .descriptor_text(d)
                .is_some_and(|t| !t.is_empty())
        })
        .count();
    println!(
        "\nscene: {} descriptors, {} active, {} screens, {} texts of which {backed} backed",
        game.engine.descriptors().len(),
        game.engine
            .descriptors()
            .iter()
            .filter(|d| d.active)
            .count(),
        game.engine.screens().len(),
        texts.len(),
    );
}
