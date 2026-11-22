//! The game options, `11A0h:0451h` (`XP01.EXE` image `0x11E51`): eight
//! rows over the playfield — music, mouse, text speed, walk speed, save,
//! load, quit, back to the game — and the confirmation box `11A0h:00D2h`
//! three of them ask first.
//!
//! The original runs the menu as a loop of its own inside the call: the
//! screen is built, the menu dissolves in, and each pass of the loop polls
//! the input once, follows the pointer and takes at most one event until a
//! row closes it. Here the loop is turned inside out the same way the
//! effects are: [`Adventure::options_open`] builds the screen and the rest
//! runs one pass per tick in [`Adventure::options_tick`]. While it is open
//! nothing else of the game runs; the clock and the pointer go on.
//!
//! The rows are the program's records in `DGROUP` (`DS:1454h`, eight of ten
//! bytes, a far-pointer row to them at `DS:14A4h`): ink, ground, row, the
//! current index, and a null-terminated row of far pointers to the texts.
//! The index lives in the record, so a setting shown is what the record
//! says and stays so between openings. Every string comes out of the image.
//!
//! The menu draws into the back buffer and presents it; here the back
//! buffer is [`Options::surface`] and the screen [`Options::screen`].

use crate::num;
use motionvm_render::Framebuffer;

use crate::titles::caption::Caption;

use super::savegame::SaveMap;
use super::{Adventure, Api, Far};

/// Where one build keeps the menu's records and the settings they set.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OptionsMap {
    /// The far-pointer row of the eight records, null-terminated.
    pub rows: u16,
    /// The heading.
    pub title: u16,
    /// The two DAC entries written at the menu's colors, and where.
    pub colors: (u16, u8),
    /// The three rows of four text-speed words, slow to fast.
    pub text_speeds: u16,
    /// The four bytes of walk speed, slow to `****`.
    pub walk_speeds: u16,
    /// The four confirmations: save, load, quit, save before quitting.
    pub confirms: [u16; 4],
    /// `[212h]`: the music switch.
    pub music: u16,
    /// `[1EDCh]`: the developer's switches the command line sets.
    pub switches: u16,
    /// Where the same build keeps the settings and the saved game.
    pub save: SaveMap,
}

impl OptionsMap {
    /// *Dunkle Schatten*'s `XP01.EXE`.
    pub(crate) const DS1: OptionsMap = OptionsMap {
        // 0x11A65: `les bx, [bx+14A4h]`.
        rows: 0x14A4,
        // 0x11A51: `push 0E60h`.
        title: 0x0E60,
        // 0x11EE0: `push 2 ; push 0C0h ; push ds ; push 14C8h`.
        colors: (0x14C8, 0xC0),
        // 0x125E5: `mov ax, [bx+14CEh]`, the row `8 · [146Ch]`.
        text_speeds: 0x14CE,
        // 0x1261D: `mov al, [bx+14E6h]`.
        walk_speeds: 0x14E6,
        // 0x12393, 0x123B7, 0x123DE, 0x123F0.
        confirms: [0x1424, 0x1430, 0x143C, 0x1448],
        // 0x12304: `xor word [212h], 1`.
        music: 0x0212,
        // 0x12379: `test word [1EDCh], 10h`.
        switches: 0x1EDC,
        save: SaveMap::DS1,
    };
}

/// The menu's own inks and grounds, all immediates of the routines.
const HEAD_INK: u8 = 0xF4; // 0x11A4B
const HOVER_INK: u8 = 0xFE; // 0x12214
const BACK_TOP: u8 = 0xC1; // 0x11A1E
const BACK_MID: u8 = 0xC0; // 0x11A33
/// The playfield's extent the menu covers (`0x11A0C`, `push 80h ; push
/// 140h`).
const W: i32 = 320;
const H: i32 = 128;
/// The middle row's band, `(0, 15, 320, 98)` (`0x11A36`–`0x11A3D`).
const MID_Y: i32 = 15;
const MID_H: i32 = 98;
/// The heading's row (`0x11A54`, `push 2`) and the column every text is
/// centered on (`push 0A0h`).
const HEAD_Y: i32 = 2;
const CX: i32 = 160;
/// `0x120CE`: `cmp word [1C82h], 64h` — the dissolve lasts a hundred
/// hundredths.
const FADE_HUNDREDTHS: u16 = 100;
/// `0x1208C`: `push 5000h ; push 0`, the draw, and `0x120AB` its mirror
/// `A000h − 1 − n`.
const FADE_SPAN: u16 = 0x5000;
/// The dissolve's draws on the reference machine, DOSBox-X with `core =
/// normal` and `cycles = fixed 25000`: 158 213 passes of the loop, each one
/// draw from the game's generator.
///
/// The loop draws until `[1C82h]` reaches 100 (`0x1208C`–`0x120D3`), so the
/// number is how many passes the machine fits into a second and is not in
/// the program. It is measured on the original on that machine: the
/// generator's seed (`1EB5h:0006h`) stands at `0000:0000` before the loop
/// of the options `XP01` opens at its start (`11A0h:0686h`) and at
/// `7698h:E605h` after it (`11A0h:06D5h`), which is 158 213 steps of the
/// generator. Every
/// dissolve takes that many here, the one at the start and the ones in
/// game; on the original a dissolve's count varies with the machine and
/// with where the timer stands when it begins.
pub(crate) const REFERENCE_DISSOLVE_DRAWS: u32 = 158_213;

/// The confirmation's box (`0x11B3E`–`0x11B58`): text width plus 16, 60
/// rows, centered on the playfield; its rows 20 apart.
const BOX_H: i32 = 60;
const BOX_ROW: i32 = 20;
const BOX_PAD: i32 = 16;
const BOX_OUTER: u8 = 0xFE; // 0x11C46
const BOX_INNER: u8 = 0xFF; // 0x11C6C
const BOX_FRAME: u8 = 0xEE; // 0x11D23
const YES_INK: u8 = 0xF8; // 0x11D5D
const NO_INK: u8 = 0xFA; // 0x11D83

/// The input event `[184Eh]` the loop takes: `-1` a key with its mapped
/// code in `[1852h]`, `5`/`6`/`7` a button (`cs:0C86h`, `cs:043Fh`: all
/// three the same).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Input {
    Key(u16),
    Click,
}

/// `100h`, Escape, and `105h`, Tab (`179Eh:0611h`, `0x18041`, `0x1805A`).
pub(crate) const KEY_ESCAPE: u16 = 0x100;
pub(crate) const KEY_TAB: u16 = 0x105;

/// What the menu was opened from, and so what follows its close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Opened {
    /// `_main`'s call before the main loop (`0x4F87`).
    Start,
    /// The gate `0EB9h:0489h` (`0x0F02E`).
    Gate,
}

/// The confirmation a row asked, and what its answer does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ask {
    Save,
    Load,
    Quit,
    SaveBeforeQuit,
}

impl Ask {
    /// Where this confirmation's table sits in the map's row of them: the
    /// variant's position, which is what a fieldless enum's discriminant
    /// is.
    #[expect(
        clippy::as_conversions,
        reason = "a fieldless enum's discriminant is its position in the row"
    )]
    fn index(self) -> usize {
        self as usize
    }
}

/// One confirmation box up.
#[derive(Debug, Clone, Copy)]
struct Confirm {
    ask: Ask,
    x0: i32,
    y0: i32,
    w: i32,
    /// `[bp-8]`: the box last hovered, `-1` before the first pass.
    alt: i16,
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Fade,
    Menu,
    Confirm(Confirm),
}

/// The menu while it is open.
#[derive(Debug, Clone)]
pub(crate) struct Options {
    phase: Phase,
    /// The back buffer `[1CB0h]`, where everything is drawn.
    pub surface: Framebuffer,
    /// The screen `A000h`, which the dissolve and the presents fill.
    pub screen: Framebuffer,
    /// `[bp-4]`: the row hovered last pass, 1-based, 0 for none.
    hover: u16,
    /// `[bp-6]`: the pointer mode to go back to.
    old_mode: u16,
    /// The dissolve's draws taken so far.
    drawn: u32,
    /// Events not yet taken, one a pass.
    queue: Vec<Input>,
    pub opened: Opened,
    /// The strip's colors and the DAC as the menu found them, for a host's
    /// jump to put back.
    before: (
        Option<crate::titles::palette1994::PanelColors>,
        u16,
        motionvm_render::Palette,
    ),
    /// Whether the last pass closed the menu.
    closed: bool,
}

/// One row's record as the program holds it.
#[derive(Debug, Clone, Copy)]
struct Row {
    at: u16,
    ink: u8,
    ground: u8,
    y: i32,
    index: u16,
    texts: Far,
}

impl Adventure {
    fn omap(&self) -> OptionsMap {
        self.options_map
            .expect("a title opens the options without its OptionsMap")
    }

    /// The records, by the far-pointer row (`0x11AC0`: up to the null).
    fn option_rows(&self) -> Vec<Row> {
        let m = self.omap();
        let mut rows = Vec::new();
        for i in 0u16.. {
            let at = m.rows.wrapping_add(4 * i);
            let (off, seg) = (self.word(at), self.word(at.wrapping_add(2)));
            if off == 0 && seg == 0 {
                break;
            }
            rows.push(Row {
                at: off,
                ink: self.byte(off),
                ground: self.byte(off.wrapping_add(1)),
                y: i32::from(num::signed(self.word(off.wrapping_add(2)))),
                index: self.word(off.wrapping_add(4)),
                texts: Far {
                    off: self.word(off.wrapping_add(6)),
                    seg: self.word(off.wrapping_add(8)),
                },
            });
        }
        rows
    }

    /// How many far pointers a row of texts holds (`1EBDh:0000h`).
    fn option_count(&self, texts: Far) -> u16 {
        let mut n = 0u16;
        loop {
            let at = self.dgroup_off(texts).wrapping_add(4 * n);
            if self.word(at) == 0 && self.word(at.wrapping_add(2)) == 0 {
                return n;
            }
            n += 1;
        }
    }

    /// A far pointer into `DGROUP` as an offset in it.
    fn dgroup_off(&self, p: Far) -> u16 {
        num::low_word(
            (u32::from(p.seg) * 16 + u32::from(p.off)).wrapping_sub(u32::from(self.dgroup) * 16),
        )
    }

    /// The string a far pointer names, as the program's bytes.
    fn far_text(&self, p: Far) -> Vec<u8> {
        let at = usize::from(p.seg) * 16 + usize::from(p.off);
        let b = self.image.bytes();
        let end = b[at..]
            .iter()
            .position(|&c| c == 0)
            .map_or(b.len(), |e| at + e);
        b[at..end].to_vec()
    }

    /// Entry `i` of a row of far pointers to texts.
    fn option_text(&self, texts: Far, i: u16) -> Vec<u8> {
        let at = self.dgroup_off(texts).wrapping_add(4 * i);
        self.far_text(Far {
            off: self.word(at),
            seg: self.word(at.wrapping_add(2)),
        })
    }

    /// `1601h:1465h`: a line's width.
    fn option_width(&self, text: &[u8]) -> i32 {
        i32::from(self.font.measure(text))
    }

    /// `10B8h:059Bh(cx, y, t)`: a line centered across on `cx`.
    fn option_line(&self, fb: &mut Framebuffer, text: &[u8], y: i32, ink: u8) {
        let x = CX - (self.option_width(text) >> 1);
        crate::draw::text::line(fb, &self.font, text, x, y, ink);
    }

    /// `10B8h:05C7h(cx, cy, t)`: centered both ways, the font's height
    /// halved (`0x11154`, `shr ax, 1`).
    fn option_line_mid(&self, fb: &mut Framebuffer, text: &[u8], cy: i32, ink: u8) {
        let y = cy - (i32::from(self.font.line_height) >> 1);
        self.option_line(fb, text, y, ink);
    }

    /// `11A0h:0002h` (`0x11A02`): the menu drawn without a highlight.
    fn option_draw(&self, fb: &mut Framebuffer) {
        // 0x11A16: the playfield flattened first, then covered whole.
        Caption::flatten_rect(fb, 0, 0, W, H, self.flatten);
        crate::draw::fill_rect(fb, 0, 0, W, H, BACK_TOP);
        crate::draw::fill_rect(fb, 0, MID_Y, W, MID_H, BACK_MID);
        let title = self.far_text(Far {
            off: self.omap().title,
            seg: self.dgroup,
        });
        self.option_line(fb, &title, HEAD_Y, HEAD_INK);
        for r in self.option_rows() {
            let text = self.option_text(r.texts, r.index);
            self.option_line(fb, &text, r.y, r.ink);
        }
    }

    /// Whether the options are up.
    pub(crate) fn options_up(&self) -> bool {
        self.options.is_some()
    }

    /// Whether they are still dissolving in, and so not yet taking input.
    #[cfg(test)]
    pub(crate) fn options_fading(&self) -> bool {
        self.options
            .as_ref()
            .is_some_and(|o| matches!(o.phase, Phase::Fade))
    }

    /// `11A0h:0451h`, from its head to the dissolve's start
    /// (`0x11E51`–`0x12086`).
    pub(crate) fn options_open(&mut self, opened: Opened) {
        let m = self.omap();
        let before = (self.panel_colors.clone(), self.status_dim, self.dac.clone());
        // 0x11E57, 0x11E5C: the pointer off, the gray ramp again.
        self.pointer_off();
        self.gray_ramp();
        // 0x11EAE–0x11EBB: the band held dark and the strip frozen.
        self.panel_tint(2);
        self.panel_dim(2);
        // 0x11ED3: the back buffer's playfield flattened, and all of it
        // presented (`1BBDh:1FC4h`, then `10B8h:0009h`).
        let mut surface = self.render_scene();
        Caption::flatten_rect(&mut surface, 0, 0, W, H, self.flatten);
        let screen = surface.clone();
        // 0x11EE9: the menu's two colors.
        let rgb: Vec<u8> = (0..6).map(|i| self.byte(m.colors.0 + i)).collect();
        self.dac.load(m.colors.1, &rgb);
        // 0x11EF1–0x11F3D: the indices taken from the settings. The music
        // row's is not: nothing but the row itself writes it.
        let save = self.omap().save;
        let rows = self.option_rows();
        if let [_, mouse, text, walk, ..] = rows[..] {
            self.set_word(mouse.at.wrapping_add(4), self.word(save.mouse));
            let normal = num::signed(self.word(m.text_speeds.wrapping_add(8)));
            let base = num::signed(self.word(save.text));
            let t = match base.cmp(&normal) {
                std::cmp::Ordering::Less => 2,
                std::cmp::Ordering::Greater => 0,
                std::cmp::Ordering::Equal => 1,
            };
            self.set_word(text.at.wrapping_add(4), t);
            let normal = self.byte(m.walk_speeds.wrapping_add(1));
            let speed = self.byte(save.speed);
            let s = match speed.cmp(&normal) {
                std::cmp::Ordering::Less => 2,
                std::cmp::Ordering::Greater => 0,
                std::cmp::Ordering::Equal => 1,
            };
            self.set_word(walk.at.wrapping_add(4), s);
        }
        // 0x11F63: an index past its row of texts is 0.
        for r in self.option_rows() {
            if self.option_count(r.texts) <= r.index {
                self.set_word(r.at.wrapping_add(4), 0);
            }
        }
        let mut o = Options {
            phase: Phase::Fade,
            surface,
            screen,
            hover: 0,
            old_mode: self.cursor.as_ref().map_or(1, |c| c.mode),
            drawn: 0,
            queue: Vec::new(),
            opened,
            before,
            closed: false,
        };
        self.option_draw(&mut o.surface);
        // 0x12086: the dissolve's clock from zero.
        self.clock.set_1c82(0);
        self.options = Some(o);
    }

    /// An event for the menu's next pass.
    pub(crate) fn options_input(&mut self, input: Input) {
        if let Some(o) = &mut self.options {
            o.queue.push(input);
        }
    }

    /// One tick of the open menu. Answers how it was opened when it closed
    /// on this tick, so that the caller runs what follows that call.
    pub(crate) fn options_tick(&mut self) -> Option<Opened> {
        let mut o = self.options.take()?;
        match o.phase {
            Phase::Fade => self.option_fade(&mut o),
            Phase::Menu => self.option_pass(&mut o),
            Phase::Confirm(c) => self.confirm_pass(&mut o, c),
        }
        if o.closed {
            self.option_close(&o);
            return Some(o.opened);
        }
        self.options = Some(o);
        None
    }

    /// The menu taken down without its close, for a host's jump — which
    /// the original has no road for: the strip's colors, the DAC and the
    /// pointer's mode as the menu found them. The settings are left as
    /// they stand.
    pub(crate) fn options_drop(&mut self) {
        if let Some(o) = self.options.take() {
            (self.panel_colors, self.status_dim, self.dac) = o.before;
            self.pointer_mode(o.old_mode);
        }
    }

    /// `0x1208C`–`0x1210B`: bytes `n` and `9FFFh − n` copied from the back
    /// buffer to the screen until the hundred hundredths are over, then
    /// all of it; then the pointer on in the menu's mode, the mouse
    /// divisors set and the keys flushed.
    ///
    /// Each draw is one from the game's generator, `rnd(0, 5000h)`. The
    /// machine's count of them, [`Adventure::dissolve_draws`], is spread
    /// over the hundredths as they pass, so that all of them are drawn by
    /// the tick the dissolve ends; the loop runs at least once.
    fn option_fade(&mut self, o: &mut Options) {
        let total = num::index(W * H);
        let all = self.dissolve_draws.max(1);
        let now = self.clock.hundredths_1c82();
        let over = now >= FADE_HUNDREDTHS;
        let due = if over {
            all
        } else {
            let part = u64::from(all) * u64::from(now) / u64::from(FADE_HUNDREDTHS);
            u32::try_from(part).unwrap_or(all).max(1)
        };
        while o.drawn < due {
            let n = usize::from(self.roll(0, FADE_SPAN));
            for at in [n, total - 1 - n] {
                o.screen.pixels[at] = o.surface.pixels[at];
            }
            o.drawn += 1;
        }
        if !over {
            return;
        }
        o.screen.pixels[..total].copy_from_slice(&o.surface.pixels[..total]);
        self.pointer_on();
        self.pointer_mode(2);
        self.mouse_divisors_05dc(&self.omap().save);
        // 0x12106: `179Eh:071Dh`, the keys typed so far are gone.
        o.queue.retain(|i| *i == Input::Click);
        o.phase = Phase::Menu;
    }

    /// The row under the pointer, 1-based; 0 is the box over the whole
    /// screen the menu registers first (`0x11E6E`). The rows do not
    /// overlap, so the order the boxes are asked in does not matter.
    fn option_hover(&self, rows: &[Row]) -> u16 {
        let (_, y) = self.mouse;
        let h = i32::from(self.font.line_height);
        rows.iter()
            .position(|r| (r.y..r.y + h).contains(&y))
            .map_or(0, |i| num::word(i) + 1)
    }

    /// `179Eh:0000h(x, y)`: the pointer put somewhere.
    fn option_warp(&mut self, x: i32, y: i32) {
        if let Some(c) = &mut self.cursor {
            self.mouse = c.set_position(x, y);
        } else {
            self.mouse = (x, y);
        }
    }

    /// One pass of the menu's loop (`0x1210B`–`0x125D4`).
    fn option_pass(&mut self, o: &mut Options) {
        self.pointer_poll();
        let rows = self.option_rows();
        let hover = self.option_hover(&rows);
        if hover != o.hover {
            // 0x1213E: the row left, in its own ink — text over text.
            if o.hover > 0 {
                let r = rows[usize::from(o.hover) - 1];
                let text = self.option_text(r.texts, r.index);
                self.option_line(&mut o.surface, &text, r.y, r.ink);
            }
            o.hover = hover;
            // 0x12210: the row entered, in FEh.
            if hover > 0 {
                let r = rows[usize::from(hover) - 1];
                let text = self.option_text(r.texts, r.index);
                self.option_line(&mut o.surface, &text, r.y, HOVER_INK);
            }
        }
        let input = if o.queue.is_empty() {
            None
        } else {
            Some(o.queue.remove(0))
        };
        let h = i32::from(self.font.line_height);
        // `di`: the row to step on, 0 for none.
        let mut step = 0u16;
        match (hover, input) {
            // 0x12591: nothing under the pointer — keys alone.
            (0, Some(Input::Key(KEY_ESCAPE))) => o.closed = true,
            (0, Some(Input::Key(KEY_TAB))) => {
                let last = rows[rows.len() - 1];
                self.option_warp(CX, last.y + (h >> 1));
            }
            (0, _) | (_, None) => {}
            // 0x12420.
            (_, Some(Input::Key(KEY_ESCAPE))) => o.closed = true,
            (_, Some(Input::Key(KEY_TAB))) => {
                let next = if usize::from(hover) < rows.len() {
                    hover
                } else {
                    0
                };
                let r = rows[usize::from(next)];
                self.option_warp(CX, r.y + (h >> 1));
            }
            (_, Some(Input::Key(_))) => {}
            // 0x122EA: a click on a row.
            (id, Some(Input::Click)) => match id {
                1 => {
                    self.option_music();
                    step = 1;
                }
                2 | 3 => {
                    self.set_word(self.omap().save.cfg_dirty, 1);
                    step = id;
                }
                4 => {
                    let r = rows[3];
                    let fast = self.word(self.omap().switches) & 0x10 != 0;
                    if r.index == 2 && !fast {
                        self.set_word(r.at.wrapping_add(4), 3);
                    }
                    step = 4;
                }
                5 => {
                    o.hover = 0;
                    self.confirm_open(o, Ask::Save);
                    return;
                }
                6 => {
                    o.hover = 0;
                    self.confirm_open(o, Ask::Load);
                    return;
                }
                7 => {
                    o.hover = 0;
                    self.confirm_open(o, Ask::Quit);
                    return;
                }
                _ => o.closed = true,
            },
        }
        if !o.closed && step != 0 {
            self.option_step(o, step);
        }
        // 0x125C4: presented.
        o.screen = o.surface.clone();
    }

    /// `0x12304`: the music switched, and the driver told — the kept song
    /// stopped or started again.
    fn option_music(&mut self) {
        let m = self.omap();
        let v = self.word(m.music) ^ 1;
        self.set_word(m.music, v);
        self.music_switched();
    }

    /// `0x12482`–`0x1258F`: a row stepped to its next text, and the mouse
    /// setting taken at once.
    fn option_step(&mut self, o: &mut Options, id: u16) {
        let rows = self.option_rows();
        let r = rows[usize::from(id) - 1];
        let ink = if o.hover == id { HOVER_INK } else { r.ink };
        let h = i32::from(self.font.line_height);
        crate::draw::fill_rect(&mut o.surface, 0, r.y, W, h, r.ground);
        let mut index = r.index.wrapping_add(1);
        if index >= self.option_count(r.texts) {
            index = 0;
        }
        self.set_word(r.at.wrapping_add(4), index);
        let text = self.option_text(r.texts, index);
        self.option_line(&mut o.surface, &text, r.y, ink);
        // 0x1257B: the mouse row's index is the setting.
        let save = self.omap().save;
        let mouse = self.word(rows[1].at.wrapping_add(4));
        if self.word(save.mouse) != mouse {
            self.set_word(save.mouse, mouse);
            self.mouse_divisors_05dc(&save);
        }
    }

    /// `11A0h:00D2h(p)`'s build (`0x11AD2`–`0x11CC7`).
    fn confirm_open(&self, o: &mut Options, ask: Ask) {
        let table = self.omap().confirms[ask.index()];
        let texts = self.confirm_texts(table);
        // 0x11AD9: the menu again, without a highlight.
        self.option_draw(&mut o.surface);
        let w = texts
            .iter()
            .map(|t| self.option_width(t))
            .max()
            .unwrap_or(0)
            + BOX_PAD;
        let x0 = (W - w) >> 1;
        let y0 = (H - BOX_H) >> 1;
        // 0x11C3E: the menu behind flattened, then the box.
        Caption::flatten_rect(&mut o.surface, 0, 0, W, H, self.flatten);
        crate::draw::fill_rect(&mut o.surface, x0 - 2, y0 - 2, w + 4, BOX_H + 4, BOX_OUTER);
        crate::draw::fill_rect(&mut o.surface, x0 - 1, y0 - 1, w + 2, BOX_H + 2, BOX_INNER);
        crate::draw::fill_rect(&mut o.surface, x0, y0, w, BOX_ROW, BACK_TOP);
        self.option_line_mid(&mut o.surface, &texts[0], y0 + 10, HOVER_INK);
        o.phase = Phase::Confirm(Confirm {
            ask,
            x0,
            y0,
            w,
            alt: -1,
        });
        o.screen = o.surface.clone();
    }

    /// The question and the two answers a table's three far pointers name.
    fn confirm_texts(&self, table: u16) -> [Vec<u8>; 3] {
        let t = |i: u16| {
            self.far_text(Far {
                off: self.word(table.wrapping_add(4 * i)),
                seg: self.word(table.wrapping_add(4 * i + 2)),
            })
        };
        [t(0), t(1), t(2)]
    }

    /// One pass of the confirmation's loop (`0x11CD1`–`0x11E0D`).
    fn confirm_pass(&mut self, o: &mut Options, mut c: Confirm) {
        self.pointer_poll();
        let table = self.omap().confirms[c.ask.index()];
        let texts = self.confirm_texts(table);
        let (x, y) = self.mouse;
        let inside = |row: i32| {
            (c.x0..c.x0 + c.w).contains(&x)
                && (c.y0 + row * BOX_ROW..c.y0 + (row + 1) * BOX_ROW).contains(&y)
        };
        let hover: i16 = if inside(1) {
            1
        } else if inside(2) {
            2
        } else {
            0
        };
        if hover != c.alt {
            crate::draw::fill_rect(
                &mut o.surface,
                c.x0,
                c.y0 + BOX_ROW,
                c.w,
                2 * BOX_ROW,
                BACK_MID,
            );
            c.alt = hover;
            if hover > 0 {
                let fy = c.y0 + 3 + i32::from(hover) * BOX_ROW;
                crate::draw::text::frame_rect(&mut o.surface, c.x0 + 3, fy, c.w - 6, 14, BOX_FRAME);
            }
            self.option_line_mid(&mut o.surface, &texts[1], c.y0 + 30, YES_INK);
            self.option_line_mid(&mut o.surface, &texts[2], c.y0 + 50, NO_INK);
            o.screen = o.surface.clone();
        }
        let input = if o.queue.is_empty() {
            None
        } else {
            Some(o.queue.remove(0))
        };
        let mut done = None;
        match input {
            // 0x11DD4: a click answers with the box hovered; only the two
            // answers end the loop.
            Some(Input::Click) if c.alt > 0 => done = Some(c.alt == 1),
            // 0x11DD9: Tab to the other answer, or to yes.
            Some(Input::Key(KEY_TAB)) => {
                let row = if c.alt == 1 { 2 } else { 1 };
                self.option_warp(CX, c.y0 + row * BOX_ROW + BOX_ROW / 2);
            }
            _ => {}
        }
        o.phase = Phase::Confirm(c);
        let Some(yes) = done else {
            return;
        };
        // 0x11E10: the menu again, without a highlight.
        self.option_draw(&mut o.surface);
        o.phase = Phase::Menu;
        self.confirm_answered(o, c.ask, yes);
        o.screen = o.surface.clone();
    }

    /// What an answer does (`0x1239D`–`0x12417`).
    fn confirm_answered(&mut self, o: &mut Options, ask: Ask, yes: bool) {
        let save = self.omap().save;
        match (ask, yes) {
            (Ask::Save, true) => {
                self.option_save();
                o.closed = true;
            }
            (Ask::Load, true) => {
                self.load_0518(&save);
                o.closed = true;
            }
            (Ask::Quit, true) => {
                o.hover = 0;
                self.confirm_open(o, Ask::SaveBeforeQuit);
            }
            (Ask::SaveBeforeQuit, yes) => {
                if yes {
                    self.option_save();
                }
                let m = self.map();
                self.set_word(m.location, 0xFFFE);
                self.set_word(m.entry, 0xFFFF);
                o.closed = true;
            }
            // 0x123D5: no to quitting steps the quit row, which has one
            // text and stays.
            (Ask::Quit, false) => self.option_step(o, 7),
            (Ask::Save | Ask::Load, false) => {}
        }
    }

    /// `147Ah:044Eh`, with the file's bytes left for the host to write.
    fn option_save(&mut self) {
        let tick = self.bios.count();
        if let Some(bytes) = self.save_044e(&self.omap().save, tick) {
            self.save_written = Some(bytes);
        }
    }

    /// The close (`0x125D7`–`0x12677`): the text speed from its row, the
    /// walk speed from its, the boxes gone, the gray ramp and the strip
    /// back, the pointer's mode as it was.
    fn option_close(&mut self, o: &Options) {
        let m = self.omap();
        let save = self.omap().save;
        let rows = self.option_rows();
        if let [_, _, text, walk, ..] = rows[..] {
            // 0x125D7: always, changed or not.
            let at = m
                .text_speeds
                .wrapping_add(8 * self.word(text.at.wrapping_add(4)));
            for i in 0..4u16 {
                let v = self.word(at.wrapping_add(2 * i));
                self.set_word(save.text.wrapping_add(2 * i), v);
            }
            // 0x12619: the walk speed, and the settings dirty if it moved.
            let b = self.byte(
                m.walk_speeds
                    .wrapping_add(self.word(walk.at.wrapping_add(4))),
            );
            if b != self.byte(save.speed) {
                self.set_byte(save.speed, b);
                self.set_word(save.cfg_dirty, 1);
            }
        }
        self.pointer_poll();
        self.gray_ramp();
        self.panel_release(2);
        self.pointer_mode(o.old_mode);
    }

    /// `0590h:18EFh`: the game's gray ramp written again.
    fn gray_ramp(&mut self) {
        if let Some((start, rgb)) = self.entry_ramp.clone() {
            self.effects.dac(&mut self.dac, start, &rgb);
        }
    }

    /// The screen while the menu is up, the pointer over it.
    pub(crate) fn options_render(&self) -> Option<Framebuffer> {
        let o = self.options.as_ref()?;
        let mut fb = o.screen.clone();
        self.draw_pointer(&mut fb);
        Some(fb)
    }
}

#[cfg(test)]
mod tests {
    use super::Far;

    /// Every text the options measure — the title, each row's every
    /// choice, the question and the two answers of each confirmation — and
    /// every message a file routine ends the program with holds no byte
    /// below `20h`, so `1601h:1465h` meets a control byte in none of them.
    #[test]
    fn the_measured_texts_hold_no_control_byte() {
        let Some(a) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        let m = a.omap();
        let mut texts = vec![a.far_text(Far {
            off: m.title,
            seg: a.dgroup,
        })];
        for r in a.option_rows() {
            for i in 0..a.option_count(r.texts) {
                texts.push(a.option_text(r.texts, i));
            }
        }
        for table in m.confirms {
            texts.extend(a.confirm_texts(table));
        }
        for at in [m.save.msg_create, m.save.msg_write, m.save.msg_invalid] {
            texts.push(motionvm_parsec_formats::to_cp437(&a.save_message(at, "")));
        }
        assert_eq!(texts.len(), 34, "the texts measured");
        for t in &texts {
            assert!(!t.is_empty(), "an empty text");
            assert!(
                t.iter().all(|&b| b >= 0x20),
                "{:?}",
                motionvm_parsec_formats::cp437_to_string(t)
            );
        }
    }
}
