//! The choice menu: what a dialogue puts up when more than one answer
//! survives its flags.
//!
//! The menu is a record at `DS:04F8h`, shipped with its geometry
//! (`0x223E8`):
//!
//! ```text
//! 04F8  0005        five lines at most
//! 04FA  0000        the highlighted choice, 1-based, 0 for none
//! 04FC  0005        x
//! 04FE  0084        y, 132: the top of the band
//! 0500  0136        the width a line may take, 310
//! 0502  000D        the line height, 13
//! 0504  2139:003C   one word a line: which choice it belongs to
//! 0508  2139:0000   the inks: F8 F8 FA E8 E8 E8 FE F8
//! 050C  21EF:1CD4   one far pointer a line: its text
//! ```
//!
//! The dialogue player builds it in its choice arm (`0x0DB74`): every
//! surviving record's text is substituted into the task's scratch,
//! prepared by `13A7h:01F1h` and cut into lines by `1269h:0504h`; then
//! the task is kind 5 with its countdown parked, and the menu's own mouse
//! object — `2139:002A`, `(0, 132)` 320 by 68, id `200h` — is registered
//! (`179Eh:0210h`, `0x0DC91`). Every frame `0CFDh:0FD9h` turns the
//! pointer's row inside that object into the highlighted choice
//! (`0x0E061`, `[185Eh] / [502h]`) and a click into the choice taken
//! (`0x0E0D5`); the keys `105h` and `106h` move the highlight, `20h` and
//! `101h` take it (`0x0E17C`). `0CFDh:0EFEh` then plays the record chosen.
//!
//! It is painted by `0CFDh:162Fh` over the panel's picture, redrawn whole
//! at row 128 (`0C3Fh:04D2h`, `0x0C909`) with the items on it, one line of
//! glyphs at a time (`1269h:03BEh`): ink `F8`, or `E8` — three bytes on —
//! for the highlighted choice's lines (`0x12AFC`, `add dx, 3`).
//! *Bi-Fi 2*'s strip redraw `0751h:04A2h` (`0x079B2`, called from task
//! 0's menu paint at `0x09614`) also fills the line's rows with color 0
//! (`0x07A62`) and holds the band below dark (`0x07A7C`) before the lines
//! go on ([`super::profile::MenuBand`]).

use crate::num;
use motionvm_parsec_formats::Font;
use motionvm_render::Framebuffer;

use super::tasks::PARKED;
use super::{Adventure, Event, cursor};

/// The menu's geometry, as its record ships it: read out of the program
/// rather than copied from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MenuShape {
    /// `+0`: how many lines there are.
    pub lines: usize,
    /// `+4`: where a line starts.
    pub x: i32,
    /// `+6`: the first line's row, the band's top.
    pub y: i32,
    /// `+8`: the width a line may take.
    pub width: i32,
    /// `+0Ah`: from one line to the next.
    pub line_h: i32,
    /// The ink row the far pointer at `+10h` names (`0x12AE7`). Entry 0 is
    /// a line's ink, entry 3 the highlighted choice's, and a `01 n` in a
    /// text picks entry `n`.
    pub inks: [u8; 8],
}

impl MenuShape {
    /// The record at `DS:record` of the program, `ds` its data segment.
    pub(crate) fn read(
        image: &crate::exe::Image,
        ds: &[u8],
        record: u16,
    ) -> crate::error::Result<Self> {
        let at = usize::from(record);
        let word = |k: usize| -> crate::error::Result<u16> {
            let lo = ds.get(at + k).copied();
            let hi = ds.get(at + k + 1).copied();
            match (lo, hi) {
                (Some(lo), Some(hi)) => Ok(u16::from_le_bytes([lo, hi])),
                _ => Err(
                    format!("the menu record at DS:{record:04X}h runs past the data segment")
                        .into(),
                ),
            }
        };
        let inks_at = usize::from(word(0x12)?) * 16 + usize::from(word(0x10)?);
        let inks = image
            .bytes()
            .get(inks_at..inks_at + 8)
            .and_then(|b| b.first_chunk::<8>().copied())
            .ok_or_else(|| format!("the menu's inks at {inks_at:#x} run past the image"))?;
        Ok(MenuShape {
            lines: usize::from(word(0)?),
            x: i32::from(num::signed(word(4)?)),
            y: i32::from(num::signed(word(6)?)),
            width: i32::from(num::signed(word(8)?)),
            line_h: i32::from(num::signed(word(0x0A)?)),
            inks,
        })
    }
}

/// One line of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MenuLine {
    /// The bytes drawn, CP437 with `01 n` controls left in.
    pub text: Vec<u8>,
    /// The choice the line belongs to, 1-based (`2139:003C`).
    pub choice: u16,
}

/// The record at `DS:04F8h`, the parts of it that change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Menu {
    /// The lines, in order; at most [`MenuShape::lines`].
    pub lines: Vec<MenuLine>,
    /// `[04FAh]`: the highlighted choice, 1-based, 0 for none.
    pub highlight: u16,
    /// Whether the menu's mouse object is registered (`0x0DC91` on,
    /// `0x0DED4` off).
    pub up: bool,
}

/// What a key does to the menu — the arms at `0x0E0FC`, `0x0E114` and
/// `0x0E0D5`, by the mapped codes `105h`, `106h`, `20h` and `101h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuKey {
    /// `105h`: the next choice, round to the first past the last.
    Down,
    /// `106h`: the previous choice, round to the last before the first.
    Up,
    /// `20h` and `101h`: the highlighted choice taken.
    Select,
}

impl Adventure {
    /// The choice arm from `0x0DB74`: the menu built out of the survivors
    /// and the task parked on it.
    ///
    /// The strip is frozen (`0C3Fh:05DAh(1)`, `0x0DB89`), nothing is chosen,
    /// `[1E4Eh] = 0` (`0x0DBD9`), and the lines are cleared (`1269h:037Ch`,
    /// `0x0DBE3`). Each record's text is substituted (`13A7h:0023h`,
    /// `0x0DC14`) and cut into lines (`1269h:0504h`, `0x0DC32`). The task
    /// becomes kind 5 (`0x0DC51`) and is painted fresh, bit 1 of its run
    /// word (`0x0DC5C`); the hook is called with 9, then 3 (`0x0DC6D`), and
    /// the mouse object registered (`179Eh:0210h`, `0x0DC91`).
    pub(super) fn menu_build(&mut self, i: usize, survivors: &[usize]) {
        self.panel_dim(1);
        self.choice_taken = 0;
        self.menu = Menu::default();
        let Some(r) = self.tasks.t[i].dialogue else {
            return;
        };
        for (n, &k) in survivors.iter().enumerate() {
            let Some(text) = self
                .dialogue_of(r)
                .and_then(|d| d.record(k))
                .map(|rec| motionvm_parsec_formats::to_cp437(rec.text))
            else {
                continue;
            };
            let scratch = self.substitute(&text);
            let prepared = prepare_menu_text(&scratch);
            self.menu_add(&prepared, num::word(n + 1));
        }
        let t = &mut self.tasks.t[i];
        t.kind = 5;
        t.run |= 2;
        t.countdown = PARKED;
        self.hook_call(9, -1, i);
        self.hook_call(3, 0, i);
        self.menu.up = true;
        if let Some(c) = &mut self.cursor {
            c.menu_region_add();
        }
        let lines = self
            .menu
            .lines
            .iter()
            .map(|l| (l.choice, motionvm_parsec_formats::cp437_to_string(&l.text)))
            .collect();
        self.events.push(Event::MenuUp { lines });
    }

    /// `1269h:0504h(menu, text)`: one choice's text cut into lines.
    ///
    /// The first choice is 1 (`0x12BC0`) and each next one the line before
    /// it plus one (`0x12BF1`, `0x12BF5`). The text is prepared
    /// (`13A7h:01F1h`, `0x12BFC`), and while lines are left (`0x12C55`)
    /// `05D1h` says how much fits one (`0x12C0F`), the line is marked as
    /// this choice's (`0x12C2F`) and the text cut there (`0x12C45`).
    fn menu_add(&mut self, text: &[u8], choice: u16) {
        let mut rest = text;
        while !rest.is_empty() && self.menu.lines.len() < self.menu_shape.lines {
            let n = self.wrap_line(rest).max(1).min(rest.len());
            self.menu.lines.push(MenuLine {
                text: rest[..n].to_vec(),
                choice,
            });
            rest = &rest[n..];
            if rest.first() == Some(&b' ') {
                rest = &rest[1..];
            }
        }
    }

    /// [`Adventure::menu_add`], for a test to look at.
    #[cfg(test)]
    pub(crate) fn menu_add_for_test(&mut self, text: &[u8], choice: u16) {
        self.menu_add(text, choice);
    }

    /// `1269h:05D1h(text, width)`: how many bytes of the text fit a line
    /// of `width`.
    ///
    /// A space is its advance plus the font's spacing (`1601h:14BAh` at
    /// `0x12C7D`, `+5` at `0x12C8A`). Word by word, controls kept
    /// (`13A7h:0145h`, `0x12CA4`), each word is measured (`13A7h:0196h`,
    /// `0x12CE0`) and taken off the width (`0x12CEB`); with no room left
    /// the line stops before it (`0x12CF0`). A space after a word is taken
    /// too (`0x12D04`), but not counted at the end (`0x12D2A`).
    fn wrap_line(&self, text: &[u8]) -> usize {
        let space =
            i32::from(self.font.advance(b' ')) + i32::from(motionvm_parsec_formats::font::SPACING);
        let mut left = self.menu_shape.width;
        let mut taken = 0usize;
        let mut space_last = false;
        let mut at = 0usize;
        while at < text.len() {
            let word = word_end(&text[at..]);
            let (_, width) = self.font.measure_task_text(&text[at..at + word]);
            left -= i32::from(width);
            if left <= 0 {
                break;
            }
            at += word;
            taken += word;
            if text.get(at) == Some(&b' ') {
                at += 1;
                taken += 1;
                space_last = true;
                left -= space;
                if left <= 0 {
                    break;
                }
            } else {
                space_last = false;
                break;
            }
        }
        if space_last {
            taken -= 1;
        }
        taken
    }

    /// The menu's hover (`0x0E041`): the pointer's row inside the menu's
    /// mouse object is a line, and the line's choice is highlighted; off
    /// the object nothing is.
    pub(crate) fn menu_hover(&mut self) {
        if !self.menu.up {
            return;
        }
        let (x, y) = self.mouse;
        let Some((mx, my, mw, mh)) = self.cursor.as_ref().map(cursor::Cursor::menu_rect) else {
            return;
        };
        let inside = x >= mx && x < mx + mw && y >= my && y < my + mh;
        self.menu.highlight = if inside {
            let line = usize::try_from((y - my) / self.menu_shape.line_h).unwrap_or(usize::MAX);
            self.menu.lines.get(line).map_or(0, |l| l.choice)
        } else {
            0
        };
    }

    /// A click while the menu is up (`0x0E0D5`): the highlighted choice
    /// is the one taken, and none is nothing.
    pub(crate) fn menu_click(&mut self) {
        self.menu_hover();
        self.choice_taken = self.menu.highlight;
        if self.choice_taken != 0 {
            self.choice_take();
        }
    }

    /// A key while the menu is up. Down steps the highlight `[4FAh]` on
    /// (`0x0E0CC`) and past the last, `[1E4Ch]`, back to the first
    /// (`0x0E0D6`); up steps it back (`0x0E117`) and before the first to
    /// the last (`0x0E11D`).
    pub(crate) fn menu_key(&mut self, key: MenuKey) {
        if !self.menu.up {
            return;
        }
        let count = num::word(self.tasks.choices.len());
        match key {
            MenuKey::Down => {
                self.menu.highlight += 1;
                if self.menu.highlight > count {
                    self.menu.highlight = 1;
                }
            }
            MenuKey::Up => {
                if self.menu.highlight <= 1 {
                    self.menu.highlight = count;
                } else {
                    self.menu.highlight -= 1;
                }
            }
            MenuKey::Select => {
                self.choice_taken = self.menu.highlight;
                if self.choice_taken != 0 {
                    self.choice_take();
                }
                return;
            }
        }
        self.menu_pointer_to_highlight();
    }

    /// After a key moved the highlight, the mouse goes to the middle of
    /// the highlighted choice's lines, at the right end of a line
    /// (`0x0E101`–`0x0E15F`). The lines are counted down from `[4F8h]`:
    /// the first line of the choice and the first line after it.
    ///
    /// A line with no choice or a later one marks the end (`0x0E113`,
    /// `0x0E122`), one with an earlier choice the start after it
    /// (`0x0E131`). Then `y = [4FEh] + (start + end) × [502h] / 2`, the line
    /// height `[502h]` being 13 and the halving an arithmetic shift
    /// (`0x0E143`–`0x0E14B`), and `x = [4FCh] + [500h] − 3`
    /// (`0x0E154`–`0x0E15B`), handed to `179Eh:0000h` (`0x0E15F`).
    fn menu_pointer_to_highlight(&mut self) {
        let h = self.menu.highlight;
        let choice = |k: usize| self.menu.lines.get(k).map_or(0, |l| l.choice);
        let mut end = self.menu_shape.lines;
        let mut start = 0;
        let mut dx = self.menu_shape.lines;
        while dx > 0 {
            dx -= 1;
            let c = choice(dx);
            if c == 0 || c > h {
                end = dx;
            }
            if c != 0 && c < h {
                start = dx + 1;
                break;
            }
        }
        let m = self.menu_shape;
        let y = m.y + ((num::count(start + end) * m.line_h) >> 1);
        let x = m.x + m.width - 3;
        if let Some(c) = &mut self.cursor {
            self.mouse = c.set_position(x, y);
        }
    }

    /// `0CFDh:0EFEh`: the choice taken.
    ///
    /// The mouse object goes (`179Eh:0730h`, `0x0DED4`). The choice
    /// `[1E4Eh]`, 1-based (`0x0DEE1`), names a survivor in `DS:1CE6h`
    /// (`0x0DEE7`), which names a record (`0x0DEF3`), which goes into
    /// `[1DE4h]` as the one to play next (`0x0DF0B`). A record flagged
    /// `1FF0h` (`0x0DF13`) is spent by being chosen, bit 15 set
    /// (`0x0DF1A`). The band is held (`0C3Fh:0581h(1)`, `0x0DF1F`) and the
    /// record played as a mode-1 line (`0x0DF2B`, the near call to `09E9h`).
    fn choice_take(&mut self) {
        self.menu.up = false;
        // `0x0DED4`, `0x0DEDC`: the menu's region out, the shape again.
        if let Some(c) = &mut self.cursor {
            c.menu_region_remove();
            c.resync();
        }
        let Some(&rec) = self
            .tasks
            .choices
            .get(usize::from(self.choice_taken).wrapping_sub(1))
        else {
            return;
        };
        let Some(r) = self.tasks.t[0].dialogue else {
            return;
        };
        self.tasks.t[0].next = Some(rec);
        if self.record_flags(r, rec) == 0x1FF0 {
            self.set_record_flags(r, rec, 0x9FF0);
        }
        self.panel_tint(1);
        self.events.push(Event::ChoiceTaken(rec));
        self.play(0, true);
    }

    /// `1269h:03BEh(menu)`: the lines drawn, over the panel's picture.
    ///
    /// The font is `[196Ch]` and its spacing byte `+5` (`0x12A9B`,
    /// `0x12A9F`). The highlighted choice's lines (`0x12ADF`) take the ink
    /// three on (`0x12AFC`); each glyph is drawn (`1601h:13CCh`, `0x12B25`)
    /// and advanced by its width (`1601h:14BAh`, `0x12B31`), a `01 n` sets
    /// ink `n` (`0x12B44`), and each line moves down one row (`0x12B74`).
    pub(crate) fn paint_menu(&self, fb: &mut Framebuffer, font: &Font) {
        if !self.menu.up {
            return;
        }
        let m = self.menu_shape;
        for (i, line) in self.menu.lines.iter().enumerate() {
            let y = m.y + m.line_h * num::count(i);
            let mut ink = if line.choice == self.menu.highlight {
                m.inks[3]
            } else {
                m.inks[0]
            };
            let mut x = m.x;
            let mut k = 0;
            while k < line.text.len() {
                let b = line.text[k];
                if b >= 0x20 {
                    crate::draw::text::glyph(fb, font, b, x, y, ink);
                    let adv = font.advance(b);
                    x += i32::from(adv) + i32::from(motionvm_parsec_formats::font::SPACING);
                    k += 1;
                } else if b == 1 {
                    let n = usize::from(line.text.get(k + 1).copied().unwrap_or(0));
                    ink = m.inks.get(n).copied().unwrap_or(ink);
                    k += 2;
                } else {
                    break;
                }
            }
        }
    }
}

/// `13A7h:0145h`'s cut: the bytes up to the next space, end, `0Ah` or
/// `0Dh`, a `01` or `03` control taken with its argument.
fn word_end(text: &[u8]) -> usize {
    let mut i = 0;
    while i < text.len() {
        match text[i] {
            b' ' | 0 | 0x0A | 0x0D => break,
            1..=3 => i += 2,
            _ => i += 1,
        }
    }
    i.min(text.len())
}

/// `13A7h:01F1h(text)`: a text made fit for the menu, in two passes. The
/// first turns a `03 n` into two spaces (`0x13C86`), keeps `01 n` and
/// `02 n` (`0x13C8A`) and makes `0Ah` and `0Dh` a space each (`0x13C9A`).
/// The second, over the result again from `0x13CA7`, drops leading spaces
/// (`0x13CB1`), makes a run of spaces one space (`0x13CCB`) and drops a
/// trailing one (`0x13CDA`).
pub(crate) fn prepare_menu_text(text: &[u8]) -> Vec<u8> {
    let mut pass1 = Vec::with_capacity(text.len() + 4);
    let mut i = 0;
    while i < text.len() {
        let b = text[i];
        match b {
            0 => break,
            3 => {
                pass1.extend_from_slice(b"  ");
                i += 2;
            }
            1 | 2 => {
                pass1.extend_from_slice(&text[i..(i + 2).min(text.len())]);
                i += 2;
            }
            0x0A | 0x0D => {
                pass1.push(b' ');
                i += 1;
            }
            _ => {
                pass1.push(b);
                i += 1;
            }
        }
    }
    let mut out = Vec::with_capacity(pass1.len());
    let mut i = 0;
    while i < pass1.len() && pass1[i] == b' ' {
        i += 1;
    }
    while i < pass1.len() {
        let b = pass1[i];
        match b {
            1 | 2 => {
                out.extend_from_slice(&pass1[i..(i + 2).min(pass1.len())]);
                i += 2;
            }
            b' ' => {
                while i < pass1.len() && pass1[i] == b' ' {
                    i += 1;
                }
                if i < pass1.len() {
                    out.push(b' ');
                }
            }
            _ => {
                out.push(b);
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text is prepared for the menu: breaks and `03 n` become spaces,
    /// runs of spaces one space, ink controls stay.
    #[test]
    fn a_menu_text_is_flattened_to_one_line_of_single_spaces() {
        let text = b"  Wecker?\rJunge,  wir\n\x03\x0Chaben \x01\x02Mittag. ";
        assert_eq!(
            prepare_menu_text(text),
            b"Wecker? Junge, wir haben \x01\x02Mittag.".to_vec()
        );
    }

    /// A word ends at a space or a break, and a control is two bytes of it.
    #[test]
    fn a_word_ends_at_a_space_and_keeps_its_controls_whole() {
        assert_eq!(word_end(b"Moin, Mutter!"), 5);
        assert_eq!(word_end(b"\x01\x02Moin Mutter"), 6);
        assert_eq!(word_end(b"Ende"), 4);
    }
}
