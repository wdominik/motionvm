//! The title program's menu: its picture faded in, its song started, its
//! lines laid on it, one box a line, the line under the pointer lit, a
//! click on one answering the box's value (Dunkle Schatten's `0323h:0555h`,
//! `0x3785`; Captain Zins's `0323h:088Ah`, `0x3ABA`).
//!
//! The two are the same routine grown apart: Captain Zins's has five lines
//! where Dunkle Schatten's has three, fades in by 8 where the other fades
//! by 4, and between its fade and its lines waits five seconds and dims
//! its picture to half. What a title's menu is, [`MenuSpec`] says; what it
//! answers is the value of the box clicked, which the title takes.

use motionvm_parsec_formats::Picture;

use super::{Event, Fade, Faded, KEY_ESCAPE, Lib, Pal, SURFACE_W, Step};
use crate::draw;
use crate::num;
use crate::titles::parsec1994::Request;

/// The music's fade when a line is chosen: `push 7D0h`, 2000 ms (Dunkle
/// Schatten's `0x3A9F`, Captain Zins's `0x3E25`).
pub(crate) const MUSIC_FADE: u16 = 0x07D0;
/// The inks: Dunkle Schatten's `0x380A` idle `0FEh`, `0x39DA` hovered
/// `0F4h`, `0x3954` the line that cannot be chosen `0F0h` — in the bank
/// `CMBMOU.DAT` gives the blank, so no fade of the picture touches them.
pub(crate) const INK_IDLE: u8 = 0xFE;
pub(crate) const INK_HOVER: u8 = 0xF4;
pub(crate) const INK_GREY: u8 = 0xF0;
/// The column the lines are centered on (Dunkle Schatten's `0x3931`).
const CX: i32 = 0xA0;
/// The fade out's step in Dunkle Schatten and Captain Zins.
pub(crate) const OUT_STEP: i32 = -4;

/// What a title's menu is.
#[derive(Debug, Clone)]
pub(crate) struct MenuSpec {
    /// The lines, each a box whose value is its place and one.
    pub lines: Vec<Vec<u8>>,
    /// Where the first line goes and how far apart they are.
    pub y: i32,
    pub row: i32,
    /// The line drawn gray, its box off, without a saved game: the one
    /// that continues it.
    pub grey: usize,
    /// The song started once the menu has faded in; `None` for a menu
    /// whose song the title started before it.
    pub song: Option<u16>,
    /// The fade in's step.
    pub step: i32,
    /// Captain Zins's wait and dim between the fade and the lines: up to
    /// this many hundredths, or until an event, then the picture dimmed to
    /// this level.
    pub wait_dim: Option<(u64, i32)>,
    /// What Escape answers: the box value of the quitting line.
    pub escape: u16,
    /// The inks: idle, hovered, and the line that cannot be chosen.
    pub inks: [u8; 3],
    /// How far a line's box reaches above the line, and how much taller
    /// than the font's line it is.
    pub box_up: i32,
    pub box_taller: i32,
    /// The fade out's step.
    pub out_step: i32,
    /// Whether the menu blanks the screen before its picture, and whether
    /// it takes the input afresh only after its lines are drawn.
    pub blank: bool,
    pub afresh_after_lines: bool,
}

/// Dunkle Schatten's and Captain Zins's inks ([`INK_IDLE`], [`INK_HOVER`],
/// [`INK_GREY`]).
pub(crate) const INKS: [u8; 3] = [INK_IDLE, INK_HOVER, INK_GREY];

/// Where the menu is.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// Hidden, blanked, the picture drawn.
    Begin,
    In(Fade),
    /// The song started, input taken afresh.
    Ready,
    /// Captain Zins's wait (`0x3B96`–`0x3BB5`).
    Wait {
        since: u64,
    },
    Dim(Fade),
    /// The lines drawn, the pointer up.
    Lines,
    /// One pass a poll.
    Run,
    Out(Fade, u16),
}

/// The menu while it is up.
pub(crate) struct Menu {
    spec: MenuSpec,
    picture: Picture,
    pal: Pal,
    stage: Stage,
    /// Which lines are lit, and the line the pointer was over at the last
    /// pass (−1 for none) — Dunkle Schatten's `[bp-14h]` and `[bp-4]`.
    lit: Vec<bool>,
    over: i32,
}

impl Menu {
    pub(crate) fn new(spec: MenuSpec, picture: Picture, pal: Pal) -> Self {
        let lit = vec![false; spec.lines.len()];
        Menu {
            spec,
            picture,
            pal,
            stage: Stage::Begin,
            lit,
            over: -1,
        }
    }

    /// The lines, for a test.
    #[cfg(test)]
    pub(crate) fn line_texts(&self) -> &[Vec<u8>] {
        &self.spec.lines
    }

    /// Whether the menu is taking input.
    #[cfg(test)]
    pub(crate) fn up(&self) -> bool {
        matches!(self.stage, Stage::Run)
    }

    /// Where line `i` is, as its box: x, y, w, h.
    pub(crate) fn line_box(&self, lib: &Lib, i: usize) -> (i32, i32, i32, i32) {
        let w = lib.width(&self.spec.lines[i]) + 2;
        (
            (SURFACE_W - w) >> 1,
            self.line_y(i) - self.spec.box_up,
            w,
            i32::from(lib.common.font.line_height) + self.spec.box_taller,
        )
    }

    fn line_y(&self, i: usize) -> i32 {
        self.spec.y + self.spec.row * num::count(i)
    }

    /// As far as the menu goes before this retrace is used; once a line is
    /// chosen and the menu has faded out, the chosen box's value.
    pub(crate) fn step(&mut self, lib: &mut Lib) -> Step<u16> {
        match self.stage {
            Stage::Begin => {
                // Dunkle Schatten's 0x378B: hidden, blanked; 0x37C1/0x37E1
                // the slots; 0x37FE the picture; 0x380A the idle ink;
                // 0x3818 none lit.
                lib.shown = false;
                if self.spec.blank {
                    lib.blank();
                }
                draw::blit_masked(&mut lib.screen, &self.picture, 0, 0, None);
                self.lit.iter_mut().for_each(|l| *l = false);
                self.stage = Stage::In(Fade::full(self.pal, self.spec.step));
                Step::Next
            }
            Stage::In(f) => match lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::In(f);
                    Step::Next
                }
                Faded::Done => {
                    self.stage = Stage::Ready;
                    Step::Next
                }
            },
            Stage::Ready => {
                // Dunkle Schatten's 0x384C–0x3854: the song, loaded as the
                // menu was built, endlessly. 0254h:0A37h: the input taken
                // afresh, the pointer drawn.
                if let Some(piece) = self.spec.song {
                    lib.music.push(Request::Play { piece, loops: -1 });
                }
                if !self.spec.afresh_after_lines {
                    lib.input_afresh();
                }
                self.stage = match self.spec.wait_dim {
                    Some(_) => Stage::Wait { since: lib.now() },
                    None => Stage::Lines,
                };
                Step::Next
            }
            Stage::Wait { since } => {
                // Captain Zins's 0x3B96–0x3BB5: the clock from 0; until
                // the poll's word is not 0 or 500 hundredths have passed,
                // the test before each poll — so the word the fade's last
                // check left ends the wait at once.
                let Some((wait, dim)) = self.spec.wait_dim else {
                    self.stage = Stage::Lines;
                    return Step::Next;
                };
                loop {
                    if lib.last_poll != 0 || lib.now() - since >= wait {
                        self.stage = Stage::Dim(Fade::between(self.pal, super::FULL, dim, -4));
                        return Step::Next;
                    }
                    lib.poll();
                    if lib.last_poll == 0 && lib.idle() {
                        return Step::Yield;
                    }
                }
            }
            Stage::Dim(f) => match lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::Dim(f);
                    Step::Next
                }
                Faded::Done => {
                    // 0x3BE9: the pointer hidden for the lines.
                    lib.shown = false;
                    self.stage = Stage::Lines;
                    Step::Next
                }
            },
            Stage::Lines => {
                self.lines(lib);
                if self.spec.afresh_after_lines {
                    // Bi-Fi 2's 0x357F: after the lines.
                    lib.input_afresh();
                }
                self.stage = Stage::Run;
                Step::Next
            }
            Stage::Run => {
                // Every event waiting is taken this retrace, one a pass.
                loop {
                    if let Some(value) = self.pass(lib) {
                        self.chosen(lib, value);
                        return Step::Next;
                    }
                    if lib.idle() {
                        return Step::Yield;
                    }
                }
            }
            Stage::Out(f, value) => match lib.fade(f) {
                Faded::Yield => Step::Yield,
                Faded::On(f) => {
                    self.stage = Stage::Out(f, value);
                    Step::Next
                }
                Faded::Done => {
                    // Dunkle Schatten's 0x3B07/0x3B0C: the song stopped,
                    // the digital driver's `StopAll` behind it.
                    lib.music.push(Request::Stop);
                    lib.music.push(Request::StopEffects);
                    self.stage = Stage::Begin;
                    Step::Done(value)
                }
            },
        }
    }

    /// A line chosen (Dunkle Schatten's `0x3A92`): hidden, the boxes gone;
    /// `0x3AA2`: the music faded; `0x3AB2`: the picture again, without the
    /// lines; then the fade out.
    fn chosen(&mut self, lib: &mut Lib, value: u16) {
        lib.shown = false;
        lib.music.push(Request::FadeOut(MUSIC_FADE));
        draw::blit_masked(&mut lib.screen, &self.picture, 0, 0, None);
        let from = self.spec.wait_dim.map_or(super::FULL, |(_, dim)| dim);
        let step = self.spec.out_step;
        self.stage = Stage::Out(Fade::between(self.pal, from, 0, step), value);
    }

    /// The lines in the idle ink, and the line that continues a saved
    /// game again in gray when there is none (Dunkle Schatten's
    /// `0x3870`–`0x3979`, Captain Zins's `0x3BEE`–`0x3CF7`); the pointer
    /// drawn, no line under it yet.
    fn lines(&mut self, lib: &mut Lib) {
        for (i, text) in self.spec.lines.iter().enumerate() {
            Lib::line(
                &mut lib.screen,
                &lib.common.font,
                text,
                CX,
                self.line_y(i),
                self.spec.inks[0],
            );
        }
        if !lib.saved {
            let g = self.spec.grey;
            Lib::line(
                &mut lib.screen,
                &lib.common.font,
                &self.spec.lines[g],
                CX,
                self.line_y(g),
                self.spec.inks[2],
            );
        }
        lib.shown = true;
        self.over = -1;
    }

    /// The line under the pointer, `-1` for none: the box list's walk
    /// (Dunkle Schatten's `0x99EC`–`0x9A8A`), `x <= px < x + w`, `y <= py <
    /// y + h`, a box whose `+4` bit 0 is clear passed over.
    fn hover(&self, lib: &Lib) -> i32 {
        let (px, py) = lib.mouse;
        for i in 0..self.spec.lines.len() {
            if i == self.spec.grey && !lib.saved {
                continue;
            }
            let (x, y, w, h) = self.line_box(lib, i);
            if (x..x + w).contains(&px) && (y..y + h).contains(&py) {
                return num::count(i);
            }
        }
        -1
    }

    /// One pass of the menu's loop (Dunkle Schatten's `0x3983`–`0x3A8F`): a
    /// poll, the lines lit and unlit as the pointer moved, and an event
    /// taken.
    fn pass(&mut self, lib: &mut Lib) -> Option<u16> {
        let event = lib.poll_event();
        let si = self.hover(lib);
        if si != self.over {
            // 0x39AD: from the last line up, each line that is lit and not
            // under the pointer in the idle ink, the one under it in the
            // hovered — text over text, nothing filled.
            for di in (0..self.lit.len()).rev() {
                let ink = if num::count(di) != si && self.lit[di] {
                    self.lit[di] = false;
                    self.spec.inks[0]
                } else if num::count(di) == si && !self.lit[di] {
                    self.lit[di] = true;
                    self.spec.inks[1]
                } else {
                    continue;
                };
                Lib::line(
                    &mut lib.screen,
                    &lib.common.font,
                    &self.spec.lines[di],
                    CX,
                    self.line_y(di),
                    ink,
                );
            }
            self.over = si;
        }
        // 0x3A5B: a key is Escape or nothing; buttons 5 and 6 take the line
        // under the pointer, `+10h` of its box, its place and one.
        match event {
            Some(Event::Key(KEY_ESCAPE)) => Some(self.spec.escape),
            Some(Event::Button(5 | 6)) if si >= 0 => Some(num::low_word(num::unsigned32(si + 1))),
            _ => None,
        }
    }
}
