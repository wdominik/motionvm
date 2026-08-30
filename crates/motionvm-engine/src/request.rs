//! `REQUEST`: the system's own message box, and the only word in either
//! generation that stops the game to ask something.
//!
//! The handler (`0104:35a0` in `LL.EXE`) pops a box, a message and up to five
//! button captions, hands them to the drawer at `0104:7332` and waits there
//! until a click, Enter or Escape answers it. Nothing here can wait like
//! that: a frame has to end for the pointer to move. So the word is asked
//! again every frame until it has an answer — see [`Request`] and
//! [`motionvm_forth::m16::Vm::repeat_word`].
//!
//! Geometry, read from the drawer rather than guessed. The box is `w` by `h`
//! at `x, y`, filled in the color `SYSBC` set and framed in `SYSFC`
//! (`0104:73cc`, `0104:73d4`). The message is drawn at `5` down the box. Each
//! button is `(w - 10 - (n - 1) * 4) / n` wide (`0104:7344`), the first
//! starts `5` in (`0104:7433`) and each next one `4` past the last
//! (`0104:758d`), and they run from `h - 18` to `h - 6` down the box
//! (`0104:7441`). The one the caller named as its default is framed a second
//! time, one pixel out on every side (`0104:746f`).
//!
//! The hit test is a pixel wider than the frame on each side: `h - 19` to
//! `h - 5`, and `x + start` to `x + start + width` (`0104:7550` to
//! `0104:7585`). It answers the button's **one-based** index; Escape answers
//! 0 and Enter the default (`0x75c1`, `0x75cf`).

/// A request the game is waiting on.
#[derive(Debug, Clone)]
pub(crate) struct Request {
    /// Where the box sits and how big it is.
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// What it asks.
    pub message: String,
    /// What the buttons say, left to right.
    pub captions: Vec<String>,
    /// Which button Enter answers, as the caller numbered them.
    pub default: i32,
    /// The colors `SYSFC` and `SYSBC` last set.
    pub fg: i32,
    pub bg: i32,
    /// Set once something answers it; the word hands this back and the box
    /// goes away.
    pub answer: Option<i32>,
    /// Whether the pointer was down on the frame before this one, so that
    /// holding the button does not answer twice.
    pub was_down: bool,
}

impl Request {
    /// Bytes of box that the buttons leave for themselves.
    const MARGIN: i32 = 10;
    /// Bytes between one button and the next.
    const GAP: i32 = 4;
    /// The button row, as offsets down the box: frame, then hit test.
    const FRAME_TOP: i32 = 18;
    const FRAME_BOTTOM: i32 = 6;
    const HIT_TOP: i32 = 19;
    const HIT_BOTTOM: i32 = 5;
    /// Where the first button starts, in from the box's left edge.
    const FIRST: i32 = 5;
    /// The row a caption centers on, as an offset up from the box's bottom
    /// (`0104:74b8`: `di - 12`, with `di` the box's height).
    const CAPTION_MIDDLE: i32 = 12;

    /// How wide one button is.
    pub fn button_width(&self) -> i32 {
        let n = self.captions.len().max(1) as i32;
        (self.w - Self::MARGIN - (n - 1) * Self::GAP) / n
    }

    /// Where button `i` starts, in from the box's left edge.
    pub fn button_left(&self, i: usize) -> i32 {
        Self::FIRST + i as i32 * (self.button_width() + Self::GAP)
    }

    /// The button's frame in display coordinates: left, top, right, bottom.
    pub fn button_frame(&self, i: usize) -> (i32, i32, i32, i32) {
        let left = self.x + self.button_left(i);
        (
            left,
            self.y + self.h - Self::FRAME_TOP,
            left + self.button_width() - 1,
            self.y + self.h - Self::FRAME_BOTTOM,
        )
    }

    /// Which button a point is on, one-based, or `None`.
    ///
    /// Inclusive on every edge, and a pixel outside the frame on each — the
    /// original tests the row once and then each button's span.
    pub fn button_at(&self, px: i32, py: i32) -> Option<i32> {
        if py < self.y + self.h - Self::HIT_TOP || py > self.y + self.h - Self::HIT_BOTTOM {
            return None;
        }
        let width = self.button_width();
        (0..self.captions.len()).find_map(|i| {
            let left = self.x + self.button_left(i);
            (px >= left && px <= left + width).then_some(i as i32 + 1)
        })
    }
}

impl crate::Engine {
    /// Reads the pointer and the key for the open request, once a frame.
    ///
    /// The original polls inside its own loop: a click inside a button
    /// answers that button, Escape answers 0 and Enter the default
    /// (`0104:7587`, `0x75cf`, `0x75c1`). It reads a press, so the button has
    /// to have come up first — a request opened by a click that is still held
    /// would otherwise answer itself with whatever is under the pointer.
    pub(crate) fn poll_request(&mut self) {
        let (x, y, down, key) = (self.mouse.x, self.mouse.y, self.mouse.left != 0, self.key);
        let Some(r) = self.request.as_mut() else {
            return;
        };
        if r.answer.is_some() {
            return;
        }
        match key {
            0x1b => r.answer = Some(0),
            0x0d => r.answer = Some(r.default),
            _ => {}
        }
        if down
            && !r.was_down
            && let Some(hit) = r.button_at(x, y)
        {
            r.answer = Some(hit);
        }
        r.was_down = down;
    }

    /// Draws the open request over the frame.
    ///
    /// An overlay rather than something composed into the screens, because
    /// that is what it is in the original: the drawer saves the background
    /// under the box, blits, and puts the background back when it is
    /// answered (`0104:73aa`), so nothing of the box survives it.
    pub(crate) fn draw_request(&self, frame: &mut motionvm_render::Framebuffer) {
        let Some(r) = self.request.as_ref() else {
            return;
        };
        let (fg, bg) = (r.fg as u8, r.bg as u8);
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                frame.set(x, y, bg);
            }
        }
        outline(frame, r.x, r.y, r.x + r.w - 1, r.y + r.h - 1, fg);

        let (Some(font), Some(refs)) = (self.system_font.as_ref(), self.font_refs.as_ref()) else {
            return;
        };
        // Both texts go through the engine's own run drawer (`0d06:1107`),
        // which takes no spacing of its own: it measures with the **resting
        // glyph gap** at `ds:0x13c4` and advances by it per glyph
        // (`0d06:12d1` and `0d06:11fb`), and the line gap at `ds:0x13c6`
        // between lines (`0d06:11d2`). Both rest at 1 in this build's data
        // segment, and nothing on the request path writes either. Drawing at
        // 0 instead ran the glyphs together.
        let gap = motionvm_render::SPACING;
        let width = |s: &str| motionvm_render::Framebuffer::text_width_spaced(font, refs, s, gap);

        // The message: `0104:7407` hands the drawer y = 5 and x = half the
        // box, in mode **1** — bit 0 only, so it centers on x and takes y as
        // the top (`0d06:1128`, `0d06:118d`). The centering is the drawer's
        // own `x − width/2` (`0d06:1142`), which is not `(w − width)/2` when
        // the two disagree on parity.
        for (n, line) in r.message.lines().enumerate() {
            let at = r.x + r.w / 2 - width(line) / 2;
            let row = r.y + 5 + n as i32 * (font.height as i32 + gap);
            frame.draw_text_line16(font, refs, line, at, row, fg, gap, &[]);
        }

        // A caption: `0104:74aa` hands it mode **5** — bits 0 and 2, so both
        // axes center — on the button's middle, x = its left plus half its
        // width and y = `h − 12` down the box (`0104:74b8`, `0104:74be`).
        for (i, caption) in r.captions.iter().enumerate() {
            let (left, top, right, bottom) = r.button_frame(i);
            outline(frame, left, top, right, bottom, fg);
            if i as i32 + 1 == r.default {
                outline(frame, left - 1, top - 1, right + 1, bottom + 1, fg);
            }
            let at = r.x + r.button_left(i) + r.button_width() / 2 - width(caption) / 2;
            let row = r.y + r.h - Request::CAPTION_MIDDLE - font.height as i32 / 2;
            frame.draw_text_line16(font, refs, caption, at, row, fg, gap, &[]);
        }
    }
}

/// A one-pixel rectangle, corners included.
fn outline(
    frame: &mut motionvm_render::Framebuffer,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    color: u8,
) {
    for x in left..=right {
        frame.set(x, top, color);
        frame.set(x, bottom, color);
    }
    for y in top..=bottom {
        frame.set(left, y, color);
        frame.set(right, y, color);
    }
}
