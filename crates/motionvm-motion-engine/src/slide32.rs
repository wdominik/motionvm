//! The 32-bit `->SCRX` and `->SCRY`: the view slides to a new scroll
//! position over a strip that holds the old picture and the new one side by
//! side.
//!
//! Read from `ENGINE.EXE` V0.04.15/R78, `->SCRX` at `0x60690` (`->SCRY` at
//! `0x60990` is the same handler turned through ninety degrees; R109 carries
//! both at `0x75130` and on, and Dunkle Schatten 2 never calls them). The
//! handler pops the target, and when it differs from the register at `+0x24`:
//!
//! 1. allocates a strip the view's height and the view's width plus the
//!    distance wide (`0x161a0`), and copies the view's rectangle of the
//!    software surface — what the display shows — into it (`0x21500`), at the
//!    left when sliding right and at the distance when sliding left;
//! 2. writes the target into the register, marks the screen for a whole
//!    redraw (`orb $0xd0` on its flags, `0x58db0`) and draws it at once
//!    (`0x57710`), then copies the view's rectangle again into the other half
//!    of the strip — so the strip is old and new, joined where they overlap;
//! 3. blits the strip through the view, one window per vertical retrace
//!    (`0x216e0`, then `0x6cdf4` polls port `0x3da` for the retrace and
//!    `0x143c0` puts the pointer back), moving the window by the step at
//!    `0xba510` — **4** in both builds (`0xdb4c8` in R109) — from the old
//!    half toward the new one, for as long as the window has not passed the
//!    distance (`0x607a6`) or gone below zero (`0x60907`);
//! 4. frees the strip, writes the target once more and marks the redraw
//!    again (`0x60968`), so the frame after the slide is drawn normally.
//!
//! The window's positions are therefore 0, 4, 8, … up to the last one not
//! past the distance, and a distance that is not a multiple of four ends with
//! a jump of the remainder into the final redraw — which is what the original
//! shows too. A target equal to the register skips the slide and still marks
//! the redraw.
//!
//! Run here as a transition: the interpreter is held while the strip moves,
//! as the handler holds it inside its own loop, and each window is one frame
//! of the display, a refresh long. The refresh is the mode's: 640×480×256 is
//! VESA `0x101`, at the 60 Hz a VGA-class card and its emulators run that
//! resolution at, which is seventeen of the master counter's ticks to the
//! digit — see [`crate::Engine::step_raw_ticks`].

use crate::Engine;
use motionvm_motion_forth::cell;
use motionvm_render::{Framebuffer, Rect};

/// A slide in flight.
#[derive(Debug, Clone)]
pub(crate) struct Slide {
    /// The screen whose view moves.
    screen: u32,
    /// `->SCRY` rather than `->SCRX`.
    vertical: bool,
    /// Old and new picture joined along the axis of the slide: the old one
    /// at the start when sliding toward larger coordinates, else the new.
    strip: Framebuffer,
    /// Where the view sits on the display (`+0x2c`/`+0x2e`).
    at: (i32, i32),
    /// The view's size (`+0x1c`/`+0x1e`).
    view: (u16, u16),
    /// How far the register moves.
    distance: i32,
    /// The window's position in the strip for the next frame.
    offset: i32,
    /// Toward larger coordinates: the window starts at zero and grows.
    forward: bool,
}

impl Slide {
    /// Pixels the window moves per refresh: the constant at `0xba510`
    /// (R78) and `0xdb4c8` (R109), read as 4 in both.
    pub(crate) const STEP: i32 = 4;

    /// The refresh rate the slide's frames go by, in Hz: VESA mode `0x101`'s.
    pub(crate) const REFRESH_HZ: u32 = 60;

    /// Whether the window has run past the strip — the loop's own exit.
    fn over(&self) -> bool {
        if self.forward {
            self.offset > self.distance
        } else {
            self.offset < 0
        }
    }
}

impl Engine {
    /// `->SCRX`/`->SCRY` on the active screen: builds the strip and queues
    /// the slide, or — for a target the register already holds — marks the
    /// redraw and nothing more, as the handler does.
    pub(crate) fn start_slide(&mut self, vertical: bool, target: i32) {
        let Some(s) = self.display.current_mut() else {
            return;
        };
        let handle = s.handle;
        let old = i32::from(if vertical { s.pos.1 } else { s.pos.0 });
        let target = i32::from(cell::short(target));
        if old == target {
            self.redraw_view(handle);
            return;
        }
        let (view, at) = (s.view, (i32::from(s.view_pos.0), i32::from(s.view_pos.1)));
        let (w, h) = (i32::from(view.0), i32::from(view.1));
        let distance = (target - old).abs();
        let forward = target > old;
        // The strip, and the old picture into its first or second half.
        let (sw, sh) = if vertical {
            (w, h + distance)
        } else {
            (w + distance, h)
        };
        let mut strip = Framebuffer::new(cell::low16(sw), cell::low16(sh));
        let shown = Rect {
            x: at.0,
            y: at.1,
            w: view.0,
            h: view.1,
        };
        let (old_at, new_at) = if forward {
            (0, distance)
        } else {
            (distance, 0)
        };
        let place = |strip: &mut Framebuffer, src: &Framebuffer, along: i32| {
            let (dx, dy) = if vertical { (0, along) } else { (along, 0) };
            strip.copy_from(src, shown, dx, dy);
        };
        place(&mut strip, &self.video, old_at);
        // The register moves, the screen is drawn again under the moved
        // view, and the new picture goes into the other half.
        if let Some(s) = self.display.screen_mut(handle) {
            if vertical {
                s.pos.1 = cell::short(target);
            } else {
                s.pos.0 = cell::short(target);
            }
        }
        self.redraw_view(handle);
        self.draw_screen(handle);
        let fresh = self.display.compose();
        place(&mut strip, &fresh, new_at);
        self.transitions.slide = Some(Slide {
            screen: handle,
            vertical,
            strip,
            at,
            view,
            distance,
            offset: if forward { 0 } else { distance },
            forward,
        });
    }

    /// One frame of a running slide: the window at its current offset onto
    /// the display, then the step. Drops the slide when the window has run
    /// past the strip, which leaves the frame after it to the drawer.
    pub(crate) fn advance_slide(&mut self) {
        let Some(mut slide) = self.transitions.slide.take() else {
            return;
        };
        let (dx, dy) = if slide.vertical {
            (0, slide.offset)
        } else {
            (slide.offset, 0)
        };
        let window = Rect {
            x: dx,
            y: dy,
            w: slide.view.0,
            h: slide.view.1,
        };
        self.video
            .copy_from(&slide.strip, window, slide.at.0, slide.at.1);
        slide.offset += if slide.forward {
            Slide::STEP
        } else {
            -Slide::STEP
        };
        if slide.over() {
            // The last thing the handler does: the register is already at
            // the target, and the screen is marked once more so the next
            // frame draws it whole.
            let handle = slide.screen;
            self.redraw_view(handle);
        } else {
            self.transitions.slide = Some(slide);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Slide;
    use crate::{Descriptor, Engine, Profile, Shows};
    use motionvm_render::Picture;

    /// A display with one full-size screen, its surface twice the width of
    /// its view and showing a picture in which every column tells its own
    /// position — as a descriptor, so that the drawer's repaint after the
    /// slide puts the same columns back.
    fn wide_screen() -> Engine {
        let mut e = Engine::new(Profile::motion32());
        e.select_mode(crate::video::MODE_640X480X256);
        e.enter_graphics().expect("the mode");
        let handle = e.display.new_screen();
        let s = e.display.screen_mut(handle).unwrap();
        s.set_size(1280, 480);
        s.set_view(640, 480);
        s.full_view = (640, 480);
        s.active = true;
        let pixels = (0..480i32)
            .flat_map(|_| (0..1280i32).map(crate::cell_of))
            .collect();
        e.scene.sprites.insert(
            1,
            Picture {
                width: 1280,
                height: 480,
                pixels,
            },
        );
        e.scene.descriptors.push(Descriptor {
            handle: 1,
            screen: handle,
            shows: Shows::Sprite(1),
            active: true,
            dirty: true,
            ..Default::default()
        });
        e.draw();
        e.present();
        e
    }

    /// The window moves four columns a frame from the old view toward the
    /// new one, and the frame after the last one shows the target view
    /// whole.
    #[test]
    fn the_view_slides_four_columns_a_frame_and_lands_on_the_target() {
        let mut e = wide_screen();
        e.start_slide(false, 10);
        let mut frames = 0;
        while e.transitions.slide.is_some() {
            e.advance_slide();
            frames += 1;
            let shown = e.video.get(0, 0).unwrap();
            assert_eq!(
                shown,
                crate::cell_of(((frames - 1) * Slide::STEP).min(10)),
                "frame {frames}"
            );
        }
        // Offsets 0, 4, 8: three frames, then the loop exits at 12 > 10.
        assert_eq!(frames, 3);
        e.draw();
        e.present();
        assert_eq!(
            e.video.get(0, 0),
            Some(crate::cell_of(10)),
            "the target view"
        );
        assert_eq!(e.display.screens[0].pos, (10, 0));
    }

    /// Sliding back runs the window from the far end of the strip to zero.
    #[test]
    fn a_slide_back_starts_at_the_old_view() {
        let mut e = wide_screen();
        e.display.screens[0].pos = (12, 0);
        e.present();
        e.start_slide(false, 0);
        let mut seen = Vec::new();
        while e.transitions.slide.is_some() {
            e.advance_slide();
            seen.push(e.video.get(0, 0).unwrap());
        }
        assert_eq!(
            seen,
            [12, 8, 4, 0].map(crate::cell_of),
            "offsets 12, 8, 4, 0, then the loop exits below zero"
        );
    }

    /// A target the register already holds slides nothing.
    #[test]
    fn a_target_already_reached_is_no_slide() {
        let mut e = wide_screen();
        e.start_slide(false, 0);
        assert!(e.transitions.slide.is_none());
    }
}
