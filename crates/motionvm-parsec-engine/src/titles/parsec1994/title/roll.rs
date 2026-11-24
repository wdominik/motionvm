//! A roll of lines through a window, the library's credits: Dunkle
//! Schatten's `XP01` plays it at the game's end (`12D3h:010Ah` lays it
//! out, `12D3h:025Ah` keeps its cursor, `12D3h:02D2h` draws it), Captain
//! Zins's `XP00` under its title (`0323h:0C78h`, `0323h:0DC8h`,
//! `0323h:0E40h`). The two programs' routines are the same instructions
//! but for their numbers: where the roll starts, how far the three set
//! kinds of line go on, and the window — which is what [`RollSpec`] holds.
//!
//! Each line's first character says what it is. `c`, `n` and `p` are set
//! lines, centered in the window in an ink of their own; `*` goes on by
//! `50h`, `/` by 8, `.` by 4, `+` and `-` by nothing, anything else by
//! `10h`, and those are not drawn. The cursor's routine also sums weights
//! of `*`, `+` and `-` it passes, and both callers throw the sum away.

use motionvm_parsec_formats::Font;
use motionvm_render::Framebuffer;

use super::intro::Rect;
use crate::draw;

/// The inks of the set kinds (Dunkle Schatten's `0x13064`, `0x1304E`,
/// `0x13059`; Captain Zins's `XP00` `0x40D2`, `0x40BC`, `0x40C7`), in the
/// `CMBMOU` bank, which no fade of the pictures touches.
const INK_C: u8 = 0xE6;
const INK_N: u8 = 0xF6;
const INK_P: u8 = 0xF4;

/// What one program's roll is laid out with.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RollSpec {
    /// Where the first line is in the roll.
    pub start: i32,
    /// How far a `c`, an `n` and a `p` line go on.
    pub advance: [i32; 3],
    /// The window the roll is seen through: a line is centered in its
    /// width, and drawn at its `y` less the roll's position from the top.
    pub window: Rect,
}

/// A line as the layout records it: where it is in the roll, where it
/// starts across (`None` for one that is not drawn), and its text with
/// the first character that typed it.
#[derive(Debug, Clone)]
struct Line {
    y: i32,
    x: Option<i32>,
    text: Vec<u8>,
}

/// The roll laid out.
#[derive(Debug, Clone)]
pub(crate) struct Roll {
    lines: Vec<Line>,
    /// Where the last line leaves off: the closing record, which has no
    /// text, stands there, and the roll begins again at this position.
    end: i32,
    window: Rect,
}

impl Roll {
    /// The layout (Dunkle Schatten's `0x12E3A`–`0x12F58`): each line at the
    /// position so far, which its kind then moves on; a set line at
    /// `((width − text) sar 1) + left`, the text measured without its
    /// first character.
    pub(crate) fn layout(texts: &[Vec<u8>], font: &Font, spec: RollSpec) -> Self {
        let [c, n, p] = spec.advance;
        let mut y = spec.start;
        let mut lines = Vec::with_capacity(texts.len());
        for text in texts {
            let (step, set) = match text.first() {
                Some(b'c') => (c, true),
                Some(b'n') => (n, true),
                Some(b'p') => (p, true),
                Some(b'*') => (0x50, false),
                Some(b'/') => (8, false),
                Some(b'.') => (4, false),
                Some(b'+' | b'-') => (0, false),
                _ => (0x10, false),
            };
            let x = set.then(|| {
                ((spec.window.w - i32::from(font.measure(&text[1..]))) >> 1) + spec.window.x
            });
            lines.push(Line {
                y,
                x,
                text: text.clone(),
            });
            y += step;
        }
        Roll {
            lines,
            end: y,
            window: spec.window,
        }
    }

    /// Where the roll begins again.
    pub(crate) fn end(&self) -> i32 {
        self.end
    }

    /// How many lines it has, the closing record not counted.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.lines.len()
    }

    /// The lines in view at `pos` drawn into `fb`, clipped to the window.
    ///
    /// The cursor (Dunkle Schatten's `0x12F8A`) moves on while its line is
    /// not the closing record and the next begins at or above `pos`; it
    /// goes back to the first line when it is below `pos`, which is where
    /// the roll begins again, and the lines before then are above `pos`
    /// anyway — so it is found from the first line here each time. From it
    /// the lines are drawn (`0x13002`) at `y − pos + top` until one is at
    /// the window's bottom or the closing record is reached; the text
    /// routine clips at the window, its own edges exclusive.
    pub(crate) fn draw(&self, fb: &mut Framebuffer, font: &Font, pos: i32) {
        let mut first = 0;
        while first < self.lines.len() {
            let next = self.lines.get(first + 1).map_or(self.end, |l| l.y);
            if next > pos {
                break;
            }
            first += 1;
        }
        let win = self.window;
        let mut clip = draw::display();
        for line in &self.lines[first..] {
            let y = line.y - pos + win.y;
            if y >= win.y + win.h {
                break;
            }
            let Some(x) = line.x else { continue };
            let ink = match line.text.first() {
                Some(b'n') => INK_N,
                Some(b'p') => INK_P,
                _ => INK_C,
            };
            draw::text::line(&mut clip, font, &line.text[1..], x, y, ink);
        }
        for yy in win.y..win.y + win.h {
            for xx in win.x..win.x + win.w {
                if let Some(p) = clip.get(xx, yy).filter(|&p| p != 0) {
                    fb.set(xx, yy, p);
                }
            }
        }
    }
}
