//! A roll of lines through a window, the library's credits: Dunkle
//! Schatten's `XP01` plays it at the game's end (`12D3h:010Ah` lays it
//! out, `12D3h:025Ah` keeps its cursor, `12D3h:02D2h` draws it), Captain
//! Zins's `XP00` under its title (`0323h:0C78h`, `0323h:0DC8h`,
//! `0323h:0E40h`), *Bi-Fi 2*'s `XP01` at the game's end (`0284h:015Eh`,
//! `0284h:0001h`, `0284h:0079h`). The programs' routines are the same
//! instructions but for their numbers: where the roll starts, how far the
//! set kinds of line and `*` go on, the window and the inks — which is what
//! [`RollSpec`] holds.
//!
//! Each line's first character says what it is. `c`, `n` and `p` are set
//! lines, centered in the window in an ink of their own; `*` goes on by
//! the spec's own step, `/` by 8, `.` by 4, `+` and `-` by nothing,
//! anything else by `10h`, and those are not drawn. The cursor's routine
//! also sums weights of `*` (`50h`), `+` (`28h`) and `-` (`14h`) it passes
//! ([`Cursor`]): Dunkle Schatten's and Captain Zins's callers throw the sum
//! away, *Bi-Fi 2*'s waits four hundredths for each.

use motionvm_parsec_formats::Font;
use motionvm_render::Framebuffer;

use super::intro::Rect;
use crate::draw;

/// The inks of the set kinds in Dunkle Schatten's `XP01` (`0x13064`,
/// `0x1304E`, `0x13059`) and Captain Zins's `XP00` (`0x40D2`, `0x40BC`,
/// `0x40C7`), in the `CMBMOU` bank, which no fade of the pictures touches.
pub(crate) const CMBMOU_INKS: [u8; 3] = [0xE6, 0xF6, 0xF4];
/// How far a `*` line goes on in Dunkle Schatten's `XP01` (`0x12ECB`:
/// `add si, 50h`) and Captain Zins's `XP00`.
pub(crate) const STAR_50: i32 = 0x50;

/// What one program's roll is laid out with.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RollSpec {
    /// Where the first line is in the roll.
    pub start: i32,
    /// How far a `c`, an `n` and a `p` line go on.
    pub advance: [i32; 3],
    /// How far a `*` line goes on.
    pub star: i32,
    /// The inks of a `c`, an `n` and a `p` line.
    pub inks: [u8; 3],
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
    inks: [u8; 3],
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
                Some(b'*') => (spec.star, false),
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
            inks: spec.inks,
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
        self.paint(fb, font, first, pos);
    }

    /// The lines from the cursor's on drawn at `pos`, as *Bi-Fi 2*'s
    /// `0284h:0079h` draws them (`0x028EF`: from `[1D2Ah]`, each at its
    /// `y − [1D28h] + 11h`), clipped as [`Self::draw`] clips.
    pub(crate) fn draw_from(&self, fb: &mut Framebuffer, font: &Font, cursor: Cursor, pos: i32) {
        self.paint(fb, font, cursor.at.min(self.lines.len()), pos);
    }

    /// The lines from `first` on drawn at `pos` into `fb`.
    fn paint(&self, fb: &mut Framebuffer, font: &Font, first: usize, pos: i32) {
        let win = self.window;
        let mut clip = draw::display();
        for line in &self.lines[first..] {
            let y = line.y - pos + win.y;
            if y >= win.y + win.h {
                break;
            }
            let Some(x) = line.x else { continue };
            let [ink_c, ink_n, ink_p] = self.inks;
            let ink = match line.text.first() {
                Some(b'n') => ink_n,
                Some(b'p') => ink_p,
                _ => ink_c,
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

/// The cursor's routine (Dunkle Schatten's `12D3h:025Ah`, `0x12F8A`;
/// *Bi-Fi 2*'s `0284h:0001h`, `0x02841`, the same instructions): the line
/// the roll is drawn from, kept between steps.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Cursor {
    /// The line it stands on; the roll's length for the closing record.
    at: usize,
}

impl Cursor {
    /// The cursor taken to `pos`, and the weights of the lines it passed.
    /// When its own line begins below `pos` it goes back to the first and
    /// passes nothing (`0x0284E`: `cmp [es:bx+2], si ; jng`, `0x02862`:
    /// `jmp 0x028B0`); else it moves on while its line is not the closing
    /// record and the next begins at or above `pos` (`0x0289C`–`0x028AE`),
    /// a `*` passed weighing `50h`, a `+` `28h`, a `-` `14h`
    /// (`0x0287B`–`0x02894`).
    pub(crate) fn advance(&mut self, roll: &Roll, pos: i32) -> i32 {
        let y = |i: usize| roll.lines.get(i).map_or(roll.end, |l| l.y);
        if y(self.at) > pos {
            self.at = 0;
            return 0;
        }
        let mut weight = 0;
        while let Some(line) = roll.lines.get(self.at) {
            if y(self.at + 1) > pos {
                break;
            }
            weight += match line.text.first() {
                Some(b'*') => 0x50,
                Some(b'+') => 0x28,
                Some(b'-') => 0x14,
                _ => 0,
            };
            self.at += 1;
        }
        weight
    }
}
