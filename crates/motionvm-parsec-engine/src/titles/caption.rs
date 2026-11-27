//! The speech caption a dialogue of the parsec1994 generation is shown in.
//!
//! A task's page of text becomes a box: measured line by line, framed,
//! anchored to whoever is speaking and kept on the playfield
//! (`1269h:0082h`, `1269h:0008h`), then painted (`1269h:01E0h`): one of
//! two backgrounds — a flat panel for one style value
//! (`1AFAh:0092h`, `0x128D5`) or every pixel under the box translated
//! through the flattening table at `[1CB0h]` (`0724h:0221h`, `0x128F5`) —
//! and then the glyphs.

use crate::num;
use motionvm_parsec_formats::Font;
use motionvm_render::Framebuffer;

use crate::titles::xp1994::{Flatten, Style};

/// A page of caption text, measured and placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Caption {
    /// Left edge in playfield pixels, after placing and clamping.
    pub x: i32,
    /// Top edge, likewise.
    pub y: i32,
    /// Width, the widest line plus the eight the compositor adds.
    pub w: i32,
    /// Height, one line height per line plus six.
    pub h: i32,
    /// The lines, in CP437 and with their controls still in them.
    pub lines: Vec<Vec<u8>>,
}

/// Where the page starting at `from` ends: the first byte below `20h` that
/// is neither a line break nor the argument of a `01` or `03` control —
/// the walk `13A7h:0196h` makes (`0x13C13`: below `20h` stops, `01` and
/// `03` take the byte after them) and `0CFDh:071Dh` repeats line after
/// line while the byte that stopped it is `0Ah` (`0x0D719`).
pub(crate) fn page_end(text: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < text.len() {
        match text[i] {
            1 | 3 => i += 2,
            0x0A => i += 1,
            b if b < 0x20 => return i,
            _ => i += 1,
        }
    }
    text.len()
}

/// The lines of a page, split at `0Ah` — a `0Ah` that is the argument of a
/// `01` or `03` control is not a break.
pub(crate) fn split_lines(page: &[u8]) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    let mut line = Vec::new();
    let mut i = 0;
    while i < page.len() {
        match page[i] {
            1 | 3 => {
                line.extend_from_slice(&page[i..(i + 2).min(page.len())]);
                i += 2;
            }
            0x0A => {
                lines.push(std::mem::take(&mut line));
                i += 1;
            }
            b if b < 0x20 => break,
            b => {
                line.push(b);
                i += 1;
            }
        }
    }
    lines.push(line);
    lines
}

/// The playfield a caption is kept inside: `(0, 0, 320, 128)`, which is
/// what `0x0D524` hands the clamp.
const PLAYFIELD: (i32, i32) = (320, 128);

/// What the compositor adds to the text's own extent (`0x127C3`).
const FRAME: (i32, i32) = (8, 6);

/// How far below an anchor a box goes when it does not fit above
/// (`0x127F7`).
const BELOW: i32 = 10;

impl Caption {
    /// Measures one page.
    ///
    /// A page is lines separated by `\n`; the walk of each line is the
    /// font's own, with the parsec1994 generation's control set. The widest
    /// line wins and the heights add up, and then the frame goes round the
    /// outside — the order `1269h:0082h` does it in.
    pub(crate) fn measure(font: &Font, page: &[u8]) -> Self {
        let lines = split_lines(page);
        let w = lines
            .iter()
            .map(|l| i32::from(font.measure_task_text(l).1))
            .max()
            .unwrap_or(0);
        let h = i32::from(font.line_height) * num::count(lines.len());
        Caption {
            x: 0,
            y: 0,
            w: w + FRAME.0,
            h: h + FRAME.1,
            lines,
        }
    }

    /// `1269h:0082h` and `1269h:0008h`: a page measured, framed, placed
    /// against its anchor by the style word, and kept on the playfield.
    ///
    /// The anchor is where the subject's slot resolved to. The room starts
    /// at three line heights (`0x12729`); each line adds its characters,
    /// the bytes `13A7h:0196h` walked, and one for the line to the count
    /// (`0x1277C`, `0x127A5`), and takes a line height off the room while
    /// one is left (`0x127B1`). The frame adds 8 to the width and 6 to the
    /// height (`0x127C3`, `0x127C8`).
    ///
    /// Bits 0–1 of the style pick the placement through the jump table at
    /// `0x127DF`: 0 ten above the anchor when the box and what is left of
    /// the room fit above it — a box of at least three lines — and ten
    /// below otherwise (`0x127E4`–`0x1280C`); 1 ten above (`0x12800`); 2 ten
    /// below (`0x1280E`); 3 centered on the anchor (`0x12810`). Bits 2–3 then
    /// place it across (`0x12824`): 0 centered on `x` (`0x12837`), 4 the
    /// left edge on `x` (`0x1284B`), 8 the right edge on `x` (`0x12853`).
    /// The count less one is the answer (`0x12861`).
    ///
    /// Answers with the box and that count: the characters of every line
    /// plus the number of lines, less one. A page of one empty line
    /// answers 0, which the caller takes for no page at all (`0x0D800`).
    pub(crate) fn compose(font: &Font, page: &[u8], anchor: (i32, i32), style: u16) -> (Self, i32) {
        let lh = i32::from(font.line_height);
        let mut room = lh * 3;
        let mut count = 0;
        let mut me = Self::measure(font, page);
        for line in &me.lines {
            count += num::count(line.len()) + 1;
            if room >= lh {
                room -= lh;
            }
        }
        let (ax, ay) = anchor;
        me.y = match style & 3 {
            0 if me.h + room <= ay => ay - me.h - BELOW,
            0 | 2 => ay + BELOW,
            1 => ay - me.h - BELOW,
            _ => ay - (me.h >> 1),
        };
        me.x = match style & 0xC {
            0 => ax - (me.w >> 1),
            4 => ax,
            8 => ax - me.w,
            _ => me.x,
        };
        me.clamp();
        (me, count - 1)
    }

    /// `1269h:0008h`: kept inside `(0, 0, 320, 128)`, one axis after the
    /// other: an edge before the near bound is put on it (`0x126C9`,
    /// `0x126EA`), and only otherwise a far edge past the far bound is
    /// pulled back to it (`0x126DA`, `0x126FE`) — so a box larger than the
    /// playfield ends with its near edge on the near bound when it began
    /// before it, and with its far edge on the far bound otherwise.
    fn clamp(&mut self) {
        let fit = |at: i32, size: i32, far: i32| {
            if at < 0 {
                0
            } else if at + size > far {
                far - size
            } else {
                at
            }
        };
        self.x = fit(self.x, self.w, PLAYFIELD.0);
        self.y = fit(self.y, self.h, PLAYFIELD.1);
    }

    /// Draws the box.
    ///
    /// One of two backgrounds, never both: bits 6 and 7 of the style word
    /// choose a flat panel in the style's own color, and every other
    /// value flattens what is already there instead — `0x128B9` jumps past
    /// the fill to the flattening pass, and `0x128DA` jumps past the
    /// flattening when it has filled. Then the text, inset four across and
    /// three down, in the style's ink.
    pub(crate) fn paint(
        &self,
        fb: &mut Framebuffer,
        font: &Font,
        style_word: u16,
        style: Style,
        flatten: Flatten,
    ) {
        if style_word & 0xC0 == 0x40 {
            crate::draw::fill_rect(fb, self.x, self.y, self.w, self.h, style.panel);
        } else {
            Self::flatten_rect(fb, self.x, self.y, self.w, self.h, flatten);
        }
        // `0x1290F`: the ink starts as the style's first byte and stays
        // across lines; `01 n` inside the text switches it to byte `n` of
        // the same three (`0x129B0`, `les bx, es:[bx+0Ah]` then `+n`), and
        // `03 n` is stepped over (`0x129C6`).
        let mut ink = style.ink;
        let mut y = self.y + 3;
        for line in &self.lines {
            let mut pen = self.line_x(font, line, style_word);
            let mut i = 0;
            while i < line.len() {
                let b = line[i];
                if b < 0x20 {
                    let arg = line.get(i + 1).copied().unwrap_or(0);
                    if b == 1 {
                        ink = style.byte(arg);
                    }
                    i += 2;
                    continue;
                }
                i += 1;
                crate::draw::text::glyph(fb, font, b, pen, y, ink);
                let adv = font.advance(b);
                if adv != 0 {
                    pen += i32::from(adv) + i32::from(motionvm_parsec_formats::font::SPACING);
                }
            }
            y += i32::from(font.line_height);
        }
    }

    /// `0724h:0221h`: every pixel of a rectangle through the flattening
    /// table, including the resource RGB updates at `0724h:01E1h`
    /// once they are enabled.
    pub(crate) fn flatten_rect(
        fb: &mut Framebuffer,
        x0: i32,
        y0: i32,
        w: i32,
        h: i32,
        flatten: Flatten,
    ) {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                if let Some(p) = fb.get(x, y) {
                    fb.set(x, y, flatten.map(p));
                }
            }
        }
    }

    /// Where a line starts inside the box.
    ///
    /// The text is inset four across and three down (`0x128FD`), and a
    /// line is centered in what is left unless the style says otherwise —
    /// `0x12936` reads bits 4 and 5 and only the value `10h` is left
    /// aligned.
    pub(crate) fn line_x(&self, font: &Font, line: &[u8], style: u16) -> i32 {
        let inset = self.x + 4;
        if style & 0x30 == 0x10 {
            return inset;
        }
        let width = i32::from(font.measure_task_text(line).1);
        inset + (self.w - 8 - width) / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A caption is the text's own extent plus a frame of eight by six.
    ///
    /// Measured with Dunkle Schatten's own font against a line the game
    /// really says, so a change to either the walk or the frame shows up
    /// here rather than on screen.
    #[test]
    fn a_page_is_measured_line_by_line_and_then_framed() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        // The font is resource 15 of the pointer file, which is where
        // the `parsec1994` runtime takes it from too.
        let path = motionvm_parsec_formats::find_ci(
            &motionvm_parsec_formats::detect::resource_dir(&dir),
            "CMBMOU.DAT",
        )
        .expect("CMBMOU.DAT");
        let bytes = std::fs::read(path).expect("read CMBMOU.DAT");
        let motionvm_parsec_formats::fgroup::Dat::Rib(rib) =
            motionvm_parsec_formats::fgroup::open_dat(&bytes).expect("CMBMOU.DAT")
        else {
            panic!("CMBMOU.DAT is not a bare RIB");
        };
        let font = Font::parse(rib.get(15).expect("resource 15")).expect("font");
        let font = &font;
        let page = motionvm_parsec_formats::to_cp437("Ich bin kein\nMöbelpacker!");
        let one = Caption::measure(font, &page);
        assert_eq!(one.lines.len(), 2);

        // The frame is the whole difference between the widest line and
        // the box, and between the two line heights and the box.
        let widest = one
            .lines
            .iter()
            .map(|l| i32::from(font.measure_task_text(l).1))
            .max()
            .unwrap();
        assert_eq!(one.w - widest, 8);
        assert_eq!(one.h - 2 * i32::from(font.line_height), 6);

        // `03` takes its argument along in the caption's walk
        // (`13A7h:0196h`); the width the status line and the options take
        // (`1601h:1465h`) steps over `03` alone and measures the argument
        // as a character.
        let inked = motionvm_parsec_formats::to_cp437("\u{3}fIch bin kein");
        assert_eq!(
            font.measure_task_text(&inked).1,
            font.measure_task_text(&motionvm_parsec_formats::to_cp437("Ich bin kein"))
                .1
        );
        assert_eq!(
            font.measure(&inked),
            font.measure(&motionvm_parsec_formats::to_cp437("fIch bin kein")),
            "the other walk measures the argument"
        );
    }

    /// A font of no glyphs: every character advances by nothing, so a
    /// line is as wide as the frame alone and as tall as the font says.
    fn one_glyph_font(line_height: u16) -> Font {
        Font {
            first: 0x20,
            line_height,
            glyphs: Vec::new(),
        }
    }

    /// A box is placed against its anchor by the style word, and kept on
    /// the playfield.
    ///
    /// Style 0 goes ten above when the anchor leaves room for the box and
    /// three lines (`0x127EB`), else ten below (`0x127F7`); style 1 always
    /// ten above (`0x12809`), style 2 always ten below (`0x1280E`), style
    /// 3 centered (`0x12817`). Bits 2–3 place x: centered, left edge on the
    /// anchor, right edge on it (`0x1283E`, `0x1284E`, `0x12859`). Then
    /// `1269h:0008h` keeps the box inside `(0, 0, 320, 128)`. The font is
    /// one glyph of no advance, so a page of `n` lines is `6 + n × 12`
    /// tall and 8 wide.
    #[test]
    fn a_box_is_placed_against_its_anchor_and_kept_on_the_playfield() {
        let font = one_glyph_font(12);
        let (b, count) = Caption::compose(&font, b"a\na", (160, 90), 0);
        assert_eq!(count, 3, "two characters and two lines, less one");
        assert_eq!((b.w, b.h), (8, 30));
        assert_eq!(
            (b.x, b.y),
            (156, 50),
            "centered, ten above: room for the box and one more line"
        );

        let (b, _) = Caption::compose(&font, b"a\na", (160, 40), 0);
        assert_eq!(
            b.y, 50,
            "30 tall plus one line of room is more than 40: ten below"
        );

        let (b, _) = Caption::compose(&font, b"a", (160, 10), 1);
        assert_eq!(b.y, 0, "style 1 goes above regardless, and is clamped");

        let (b, _) = Caption::compose(&font, b"a", (160, 100), 2);
        assert_eq!(
            b.y, 110,
            "style 2 goes below: 100 + 10, and 128 - 18 leaves room"
        );

        let (b, _) = Caption::compose(&font, b"a", (160, 100), 3);
        assert_eq!(b.y, 91, "style 3 is centered: 100 - 18 / 2");

        let (b, _) = Caption::compose(&font, b"a", (5, 90), 0);
        assert_eq!(b.x, 1, "centered on x: 5 less half of 8");
        let (b, _) = Caption::compose(&font, b"a", (3, 90), 0);
        assert_eq!(b.x, 0, "clamped to the left edge");
        let (b, _) = Caption::compose(&font, b"a", (5, 90), 4);
        assert_eq!(b.x, 5, "left edge on the anchor");
        let (b, _) = Caption::compose(&font, b"a", (315, 90), 8);
        assert_eq!(b.x, 307, "right edge on the anchor");
        let (b, _) = Caption::compose(&font, b"a", (318, 90), 4);
        assert_eq!(b.x, 312, "clamped to the right edge less its width");
    }

    /// A caption either fills its box with the style's panel color or
    /// flattens what is under it, and never both.
    ///
    /// Before resource updates the flattening keeps the builder's run — 192 to
    /// 207 in Dunkle Schatten — and turns everything else into the first
    /// of them.
    #[test]
    fn a_box_is_filled_one_way_or_flattened_the_other() {
        let flatten = Flatten::Table(crate::titles::xp1994::FlattenTable {
            to: 192,
            first: 192,
            count: 16,
            table: None,
        });
        let style = Style {
            ink: 248,
            middle: 234,
            panel: 232,
        };
        let font = Font::parse(&[]).ok();
        assert!(font.is_none(), "an empty block is not a font");

        let box_ = Caption {
            x: 10,
            y: 10,
            w: 20,
            h: 10,
            lines: Vec::new(),
        };
        let mut fb = Framebuffer::new(320, 128);
        fb.fill(7);
        fb.set(12, 12, 200);
        let no_font = Font {
            first: 0,
            line_height: 8,
            glyphs: Vec::new(),
        };
        box_.paint(&mut fb, &no_font, 0, style, flatten);
        assert_eq!(fb.get(12, 12), Some(200), "a kept color stays");
        assert_eq!(fb.get(15, 15), Some(192), "everything else flattens");
        assert_eq!(fb.get(5, 5), Some(7), "outside the box is untouched");

        let mut fb = Framebuffer::new(320, 128);
        fb.fill(7);
        box_.paint(&mut fb, &no_font, 0x40, style, flatten);
        assert_eq!(fb.get(15, 15), Some(232), "the style's panel color");
        assert_eq!(fb.get(5, 5), Some(7), "and still nothing outside");
    }
}
