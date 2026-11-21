//! The 32-bit engine's error box: how a run-time error reaches the player.
//!
//! A handler that meets an error reports it (R109 `0x5294d`, R78 `0x42f50`)
//! and sets the error flag. The report formats the message from the
//! engine's table and puts it up in a box of its own (R109 `0x271a0`, R78
//! `0x21f80`) at 40,40, 200 by 108, titled *Fehler* (R109 `0x532d5`–
//! `0x532e9`, R78 `0x436f2`–`0x4370d`). The box waits until a key comes or
//! the left button goes down, then until the left button is up again, and
//! puts back what was under it (R109 `0x27339`–`0x273db`). Then the flag
//! makes the threaded-code loop abandon the word that was running, and the
//! executor that ran it returns to its caller. Measured in the original,
//! with the click made by script: a division by zero in the frame loop's
//! word skips the rest of that word for the frame and play goes on; one in
//! the boot script ends the engine after the box.
//!
//! The box is drawn by the panel routine (R109 `0x26e8d`, R78 `0x21c30`)
//! and the box routine around it, in display coordinates, every color the
//! palette's nearest match ([`crate::paint32`]) to a six-bit one:
//!
//! - the panel: the whole box in gray (47,47,47), a one-pixel black frame
//!   four in (`0x1888b`), and the title in black on a gray gap in the
//!   frame's top edge, `tw + 8` wide and 10 tall, centered on the box;
//! - the message field: blue (0,0,47) at `x + 8, y + 12`, `w − 16` by
//!   `h − 52`;
//! - the button: green (0,63,0) at `x + 16, y + h − 32`, `w − 32` by 16;
//! - the message at `y + 14` and *OK* at `y + h − 29`, both in yellow
//!   (63,63,0), each line centered on the box's middle (mode 1 of the glyph
//!   output, `0x25c02` → `0x257cf`).
//!
//! Everything is in the system font (`0xd66f4`). Since the box saves what is
//! under it and puts it back, and nothing else draws while it waits, it is
//! drawn here over the presented frame and dropped when it closes.

use crate::Engine;
use crate::text::Pen;
use motionvm_motion_forth::Error;
use motionvm_render::Framebuffer;

/// A report on screen, waiting to be closed.
#[derive(Debug, Clone)]
pub(crate) struct ErrorBox {
    /// The message, as the engine's table words it.
    pub message: &'static str,
    /// Whether the run ends once the box is closed: an error outside the
    /// frame loop's word abandons the boot script, and the engine with it.
    pub ends_run: bool,
    /// A key or the left button has closed it; it goes once the button is
    /// up.
    pub releasing: bool,
}

impl ErrorBox {
    /// Where the report puts the box, and how big.
    const X: i32 = 40;
    const Y: i32 = 40;
    const W: i32 = 200;
    const H: i32 = 108;
    /// The title (R109 `0xd3ac4`, R78 `0xb2cb0`) and the button's caption
    /// (R109 `0xd0358`).
    const TITLE: &'static str = "Fehler";
    const OK: &'static str = "OK";
}

/// Message 0 of both builds' tables (R109 `0xd6e70`, R78 `0xb5f74`).
const DIVISION_BY_ZERO: &str = "Division durch Null!";

impl Engine {
    /// Takes a run-time error the original reports in its box, and answers
    /// whether it did. The box is built for the 32-bit engines — how the
    /// 16-bit ones report is not read — and only a division by zero is
    /// reported this way here; everything else still stops the run.
    pub(crate) fn report_error(&mut self, error: &Error, ends_run: bool) -> bool {
        if self.profile.frame_by_screen || !matches!(error, Error::DivideByZero { .. }) {
            return false;
        }
        // One box at a time: the original's report does not return until
        // the box is closed, so a second cannot come while one is up.
        if self.error_box.is_none() {
            self.error_box = Some(ErrorBox {
                message: DIVISION_BY_ZERO,
                ends_run,
                releasing: false,
            });
        }
        true
    }

    /// Whether a report is on screen.
    pub fn error_box_open(&self) -> bool {
        self.error_box.is_some()
    }

    /// One turn of the box's wait. Answers `Some(ends_run)` once it has
    /// closed.
    ///
    /// The wait takes a key the way `?KEY` does (`0x2379b`) and reads the
    /// left button from the pointer's record (`+8`, `0x27342`); either one
    /// ends the first loop, and the second runs until the left button is up
    /// (`0x27393`).
    pub(crate) fn poll_error_box(&mut self) -> Option<bool> {
        let releasing = self.error_box.as_ref()?.releasing;
        let left = self.input.mouse.left != 0;
        if !releasing && (self.pop_key() != 0 || left) {
            self.error_box.as_mut()?.releasing = true;
        }
        let b = self.error_box.as_ref()?;
        if b.releasing && !left {
            let ends = b.ends_run;
            self.error_box = None;
            return Some(ends);
        }
        None
    }

    /// Draws the open report over the frame.
    pub(crate) fn draw_error_box(&self, frame: &mut Framebuffer) {
        let Some(b) = self.error_box.as_ref() else {
            return;
        };
        let (Some(font), Some(refs)) = (
            self.scene.system_font.as_ref(),
            self.scene.font_refs.as_ref(),
        ) else {
            return;
        };
        let (x, y, w, h) = (ErrorBox::X, ErrorBox::Y, ErrorBox::W, ErrorBox::H);
        let black = self.nearest_color(0, 0, 0);
        let gray = self.nearest_color(47, 47, 47);
        let blue = self.nearest_color(0, 0, 47);
        let green = self.nearest_color(0, 63, 0);
        let yellow = self.nearest_color(63, 63, 0);
        let pen = |color| Pen {
            font,
            refs,
            color,
            spacing: crate::text::SPACING,
        };

        // The panel (`0x26e8d`).
        fill(frame, x, y, w, h, gray);
        outline(frame, x + 4, y + 4, w - 8, h - 8, black);
        let title = pen(black);
        let tw = title.width(ErrorBox::TITLE);
        let tx = x + (w >> 1) - (tw >> 1);
        fill(frame, tx - 4, y, tw + 8, 10, gray);
        crate::text::draw_line(frame, title, ErrorBox::TITLE, tx, y);

        // The box (`0x271a0`).
        fill(frame, x + 8, y + 12, w - 16, h - 52, blue);
        fill(frame, x + 16, y + h - 32, w - 32, 16, green);
        let text = pen(yellow);
        let middle = x + (w >> 1);
        for (line, top) in [(b.message, y + 14), (ErrorBox::OK, y + h - 29)] {
            crate::text::draw_line(frame, text, line, middle - (text.width(line) >> 1), top);
        }
    }
}

/// `0x188fd`'s fill.
fn fill(frame: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, color: u8) {
    for row in y..y + h {
        for col in x..x + w {
            frame.set(col, row, color);
        }
    }
}

/// `0x1888b`'s one-pixel frame.
fn outline(frame: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, color: u8) {
    for col in x..x + w {
        frame.set(col, y, color);
        frame.set(col, y + h - 1, color);
    }
    for row in y..y + h {
        frame.set(x, row, color);
        frame.set(x + w - 1, row, color);
    }
}

#[cfg(test)]
mod tests {
    use crate::{Engine, Profile};
    use motionvm_motion_forth::{Address, Error};

    fn division() -> Error {
        Error::DivideByZero {
            at: Address::new(5, 0),
        }
    }

    /// The box is built for the 32-bit engines; on the 16-bit one the error
    /// still stops the run.
    #[test]
    fn only_the_32_bit_engine_reports_in_a_box() {
        let mut e = Engine::new(Profile::motion16());
        assert!(!e.report_error(&division(), false));
        assert!(!e.error_box_open());
        let mut e = Engine::new(Profile::motion32());
        assert!(e.report_error(&division(), false));
        assert!(e.error_box_open());
    }

    /// An error the box does not cover is left to stop the run.
    #[test]
    fn other_errors_are_not_reported() {
        let mut e = Engine::new(Profile::motion32());
        let underflow = Error::StackUnderflow {
            word: "DROP",
            at: None,
        };
        assert!(!e.report_error(&underflow, false));
    }

    /// The left button closes it on its release, and the box answers
    /// whether the run ends with it.
    #[test]
    fn the_box_closes_when_the_left_button_is_up_again() {
        let mut e = Engine::new(Profile::motion32());
        e.report_error(&division(), true);
        e.input.mouse.left = 1;
        assert_eq!(e.poll_error_box(), None);
        assert_eq!(e.poll_error_box(), None, "held");
        e.input.mouse.left = 0;
        assert_eq!(e.poll_error_box(), Some(true));
        assert!(!e.error_box_open());
    }
}
