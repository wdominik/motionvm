//! *Bi-Fi 2*'s end of the game, `0D84h:0008h` (`0x0D848`–`0x0DE46`): the
//! sequence the Kaffee's handler calls for its entry 6 (`0x10942`) before
//! it returns with the location word at `FFFFh`. `_main` shows nothing
//! after its main loop (`0x007AD`–`0x008C3`): it writes the settings and
//! exits with code 0. Nothing but the Kaffee's row leaves the location
//! word at `FFFFh`, so the runtime plays the sequence as the build's
//! ending for location −1.
//!
//! On the screen the Kaffee left:
//!
//! 1. the status band blacked; the strip below it closes in on the middle
//!    a column a step, two hundredths a step at least, its colors
//!    (`A0h`–`BFh`) dimmed with it, to sample 0 of table A
//!    (`0x0D8F3`–`0x0D9B4`);
//! 2. the strip blacked, sample 1; the room's view shrinks into the window
//!    at `67h, 11h` in 64 hundredths, sample 1 again a quarter of the way
//!    (`0x0DAAB`–`0x0DB8D`);
//! 3. module 11 endlessly; slot 6 of `INTROS.DAT` with the shrunk view
//!    pasted into its window, its colors (slot 7, from `20h`) faded in a
//!    level a retrace (`0x0DBB9`–`0x0DCEE`);
//! 4. the credits rolled through the window a line a step, four
//!    hundredths a step and the weight of the pauses passed, until an
//!    event (`0x0DD47`–`0x0DDD8`);
//! 5. the music faded, the whole DAC faded to black, the music and the
//!    effects stopped (`0x0DDDB`–`0x0DE46`).
//!
//! Nothing skips the first three; the pointer is hidden from the start
//! (`1819h:0B02h`, `0x0D85F`) and sampled again from the roll on
//! (`1819h:0B9Ah`, `0x0DD3D`).
//!
//! The routines are the library's, Dunkle Schatten's by their
//! instructions: `1B72h` its drawing (`1AFAh`), `0104h` its music and
//! palette (`0590h`), `0284h` its roll (`12D3h:010Ah`/`025Ah`/`02D2h`),
//! `17C7h` its scaler (`1750h`, byte for byte), `00D7h:002Bh` its
//! palette scale (`0563h:002Bh`). The sequence's draws go to the screen
//! until the roll, which draws into the back buffer and copies the window
//! onto the screen.

use motionvm_parsec_formats::Picture;
use motionvm_render::{Framebuffer, Palette};

use crate::draw;
use crate::error::Result;
use crate::exe::Image;
use crate::exe::op::PUSH_IMM16;
use crate::titles::parsec1994::Request;
use crate::titles::parsec1994::title::host::{Ending, Exit};
use crate::titles::parsec1994::title::intro::{Rect, copy_rect};
use crate::titles::parsec1994::title::roll::{Cursor, Roll, RollSpec};
use crate::titles::parsec1994::title::{FULL, Lib, Slots, Step, far_lines, scale_channel};

/// The location word the sequence belongs to: the Kaffee's row at
/// `DS:182Ch` names `FFFFh` for entry 6 (`0x108C5`).
const END: i16 = -1;

/// `0x0D88B`–`0x0D897`: `1B72h:00D2h(0, 80h, 140h, 0Ch, 0)`, the status
/// band filled with 0.
const BAND: Rect = Rect {
    x: 0,
    y: 0x80,
    w: 0x140,
    h: 0x0C,
};
/// `0x0D8A5`–`0x0D8AF`: `1B72h:0008h(0, 8Ch, 140h, 3Ch, …)`, the strip
/// taken off the screen; `0x0D9B7`–`0x0D9C3` filled with 0 once it has
/// closed.
const STRIP: Rect = Rect {
    x: 0,
    y: 0x8C,
    w: 0x140,
    h: 0x3C,
};
/// `0x0D8F3`: `mov di, 0A0h`, the strip's half; each half closes in on the
/// column `0A0h` (`0x0D93F`, `0x0D997`).
const HALF: i32 = 0xA0;
/// `0x0D8B7`, `0x0D8C0`: `push 60h`, `push 1BC8h` — the 20h colors from
/// DAC entry `0A0h` (`DS:19E8h + 3·0A0h`, the shadow) kept, and written
/// back from `0A0h` (`0x0D922`) at the level `di + 60h` (`0x0D901`).
const DIM_FIRST: u8 = 0xA0;
const DIM_COUNT: usize = 0x20;
const DIM_BASE: i32 = 0x60;
/// `0x0D9A9`: `cmp word [1CF6h], 2` — a step of the strip two hundredths
/// at least.
const STRIP_STEP: u64 = 2;
/// The rows of table A the samples are (`13ECh:0210h(n)`'s twin
/// `0E5Ch:0210h`: `push 0` at `0x0D8D5`, `push 1` at `0x0D9E8`).
const STRIP_SAMPLE: u16 = 0;
const SHRINK_SAMPLE: u16 = 1;
/// `0x0DA78`–`0x0DA84`: the room's view taken off the screen, 140h by 80h.
const VIEW: Rect = Rect {
    x: 0,
    y: 0,
    w: 0x140,
    h: 0x80,
};
/// The window the view shrinks into and the roll goes through: the two
/// records `0DE4h:004Eh` steps between (`0x0D9F8`–`0x0DA52`: from `0, 0`
/// and `140h, 80h` to `67h, 11h` and `72h, 35h`), the rectangle
/// `0x0DC15`–`0x0DC1D` takes and `0x0DD7A`–`0x0DD8C` restores, and the
/// clip the roll is drawn in (`0x028D8`–`0x028E9`: `67h, 11h`–`0D9h, 46h`).
const WINDOW: Rect = Rect {
    x: 0x67,
    y: 0x11,
    w: 0x72,
    h: 0x35,
};
/// `0x0DABA`: `shl di, 2` — four levels a hundredth; `0x0DAC7`: `cmp di,
/// 20h` — the second sample's level.
const SHRINK_RATE: u64 = 4;
const SHRINK_SAMPLE_AT: i32 = 0x20;
/// `0x0D86E`: `push 0Bh`, the module `0E5Ch:028Fh` loads; `0x0DBB1`:
/// `push 0FFFFh`, played endlessly (`0x0DBB9`).
const SONG: u16 = 0x0B;
/// `0x0DBDF`, `0x0DBF9`: `push 6`, `push 7` — the picture and its colors
/// in `INTROS.DAT` (`DS:0B7Eh`, opened at `0x0DBC9`).
const PICTURE: usize = 6;
const COLORS: usize = 7;
/// `0x0DC81`, `0x0DCB1`: `push 20h`, the picture's colors written.
const PICTURE_COLORS: usize = 0x20;
/// The roll's layout (`0284h:015Eh`): the first line at `35h` (`0x029D7`),
/// the window's height, a `c` on by `0Eh`, an `n` by `10h`, a `p` by `14h`
/// (`0x02A6E`, `0x02A3D`, `0x02A69`), a `*` by nothing (`0x02A31`); the
/// inks `66h`, `76h`, `74h` (`0x02951`, `0x0293B`, `0x02946`). The
/// Vorführraum's roll is laid out by the same routine
/// ([`super::vorfuehrraum`]).
pub(super) const ROLL: RollSpec = RollSpec {
    start: 0x35,
    advance: [0x0E, 0x10, 0x14],
    star: 0,
    inks: [0x66, 0x76, 0x74],
    window: WINDOW,
};
/// `0x029A5`: `push 0BCEh`, the credits' lines.
const LINES_AT: usize = 0x029A5;
/// `0x0DD54`: `shl ax, 2` — four hundredths for each weight the cursor
/// passed; `0x0DDBF`: `cmp word [1CF6h], 4` — four more after the step.
const PAUSE_RATE: u64 = 4;
const ROLL_STEP: u64 = 4;
/// `0x0DDE5`: `push 7D0h`, the music's fade.
const MUSIC_FADE_MS: u16 = 0x07D0;
/// `0B7Eh:0A70h`: from `100h` (`0x0C252`) by 4 (`0x0C295`) while above
/// `80h` (`0x0C298`), the shadow scaled in place each time, written in
/// four runs of 40h a retrace (`0x0C277`, `0x0C28C`).
const OUT_FROM: i32 = 0x100;
const OUT_STEP: i32 = 4;
const OUT_ABOVE: i32 = 0x80;
const OUT_RUN: usize = 0x40;
const OUT_RUNS: usize = 4;

/// The sequence for the location word the main loop ended on.
pub(super) fn open(exit: &Exit<'_>) -> Result<Option<Box<dyn Ending>>> {
    if exit.location != END {
        return Ok(None);
    }
    Ok(Some(Box::new(Credits::open(exit)?)))
}

/// Where the sequence is.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// `0x0D84E`–`0x0D8F3`.
    Begin,
    /// `0x0D8F9`: a step of the strip begun, `di` columns of each half
    /// left.
    StripStep {
        di: i32,
    },
    /// `0x0D91B`: the retrace before the dimmed colors.
    StripDim {
        di: i32,
        since: u64,
    },
    /// `0x0D933`: the retrace before the strip is drawn.
    StripDraw {
        di: i32,
        since: u64,
    },
    /// `0x0D9A4`–`0x0D9AE`: the rest of the step's two hundredths.
    StripWait {
        di: i32,
        since: u64,
    },
    /// `0x0D9B7`–`0x0DAAB`.
    StripGone,
    /// `0x0DAB1`–`0x0DB65`: a step of the view, from `since`.
    Shrink {
        since: u64,
        played: bool,
    },
    /// `0x0DB70`: the retrace before the step at level `t` is shown.
    ShrinkShow {
        since: u64,
        played: bool,
        t: i32,
    },
    /// `0x0DB90`–`0x0DCAC`.
    Picture,
    /// `0x0DCB0`–`0x0DCEE`: the picture's colors at `level`.
    PictureIn {
        level: i32,
    },
    /// `0x0DCF0`–`0x0DD42`.
    RollBegin,
    /// `0x0DD47`–`0x0DD57`: the step at `pos` begun.
    RollStep {
        pos: i32,
        since: u64,
    },
    /// `0x0DD5A`–`0x0DD72`: its pause.
    RollPause {
        pos: i32,
        since: u64,
        pause: u64,
    },
    /// `0x0DDB1`–`0x0DDC8`: the hold after it is drawn.
    RollHold {
        pos: i32,
        since: u64,
    },
    /// `0x0DDDB`–`0x0DDEE`.
    Out,
    /// `0x0C257`–`0x0C26D`: the shadow scaled to `level`.
    OutScale {
        level: i32,
    },
    /// `0x0C272`–`0x0C293`: run `run` of the shadow written at a retrace.
    OutRun {
        level: i32,
        run: usize,
    },
    /// `0x0C29E`–`0x0C2C9`, then `0x0DDF3`–`0x0DE46`.
    Black,
    Over,
}

/// The sequence while it plays.
struct Credits {
    lib: Lib,
    /// The strip as `0x0D8AF` took it off the screen.
    strip: Framebuffer,
    /// The 20h colors from `0A0h` as the shadow held them (`[bp-0E4h]`).
    dim: Vec<u8>,
    /// The view as `0x0DA84` took it off the screen.
    view: Picture,
    /// Slot 6 with the shrunk view pasted into its window (`0x0DC5B`).
    picture: Framebuffer,
    /// Slot 7: its first register and its colors.
    colors: Vec<u8>,
    roll: Roll,
    cursor: Cursor,
    /// The shadow the end's fade scales (`DS:19E8h`).
    shadow: Palette,
    stage: Stage,
}

impl Credits {
    fn open(exit: &Exit<'_>) -> Result<Self> {
        let lib = Lib::for_ending(exit)?;
        let intros = Slots::open(exit.res, "INTROS.DAT")?;
        let picture = intros.picture(PICTURE)?;
        let colors = intros.palette(COLORS)?;
        if colors.len() < 2 + 3 * PICTURE_COLORS {
            return Err("INTROS.DAT slot 7 holds fewer than 20h colors".into());
        }
        let roll = Roll::layout(&lines(exit.image)?, &lib.common.font, ROLL);
        Ok(Credits {
            strip: draw::display(),
            dim: Vec::new(),
            view: Picture {
                width: 0,
                height: 0,
                pixels: Vec::new(),
            },
            picture: picture_screen(&picture),
            colors,
            roll,
            cursor: Cursor::default(),
            shadow: lib.dac.clone(),
            stage: Stage::Begin,
            lib,
        })
    }

    /// The sequence, as far as it goes before this retrace is used.
    fn sequence(&mut self) -> Step<()> {
        match self.stage {
            Stage::Begin
            | Stage::StripStep { .. }
            | Stage::StripDim { .. }
            | Stage::StripDraw { .. }
            | Stage::StripWait { .. }
            | Stage::StripGone
            | Stage::Shrink { .. }
            | Stage::ShrinkShow { .. } => self.closing(),
            Stage::Picture
            | Stage::PictureIn { .. }
            | Stage::RollBegin
            | Stage::RollStep { .. }
            | Stage::RollPause { .. }
            | Stage::RollHold { .. } => self.credits(),
            Stage::Out
            | Stage::OutScale { .. }
            | Stage::OutRun { .. }
            | Stage::Black
            | Stage::Over => self.fade_out(),
        }
    }

    /// Steps 1 and 2: the strip closed, the view shrunk.
    fn closing(&mut self) -> Step<()> {
        match self.stage {
            Stage::Begin => {
                // 0x0D852, 0x0D85A: the screen the target, the clip whole;
                // 0x0D85F: the pointer hidden; 0x0D870: module 11 loaded.
                self.lib.hide();
                fill(&mut self.lib.screen, BAND); // 0x0D897
                self.strip = self.lib.screen.clone(); // 0x0D8AF
                let at = usize::from(DIM_FIRST) * 3;
                self.dim = self.lib.dac.raw[at..at + 3 * DIM_COUNT].to_vec(); // 0x0D8C3
                // 0x0D8EB: `0104h:080Bh(sample, 0, 4)`. The driver's
                // entry 17 is handed 4, four passes more (0x01864: `push
                // [bp+0Ch]`), until the StopAll at 0x0D9CB cuts them.
                self.effect(STRIP_SAMPLE, 4);
                self.stage = Stage::StripStep { di: HALF };
                Step::Next
            }
            Stage::StripStep { di } => {
                // 0x0D8F9: `mov word [1CF6h], 0`.
                let since = self.lib.now();
                self.stage = Stage::StripDim { di, since };
                Step::Next
            }
            Stage::StripDim { di, since } => {
                if !self.lib.wait_retrace() {
                    return Step::Yield; // 0x0D91B
                }
                // 0x0D913, 0x0D92B: the kept colors at `di + 60h`, from
                // register 0A0h.
                let rgb = scaled(&self.dim, di + DIM_BASE);
                self.set_dac(DIM_FIRST, &rgb);
                self.stage = Stage::StripDraw { di, since };
                Step::Next
            }
            Stage::StripDraw { di, since } => {
                if !self.lib.wait_retrace() {
                    return Step::Yield; // 0x0D933
                }
                self.strip_step(di);
                self.stage = Stage::StripWait { di: di - 1, since };
                Step::Next
            }
            Stage::StripWait { di, since } => {
                if self.lib.now() - since < STRIP_STEP {
                    return Step::Yield; // 0x0D9A9
                }
                // 0x0D9B0: `or di, di ; jng`.
                self.stage = if di > 0 {
                    Stage::StripStep { di }
                } else {
                    Stage::StripGone
                };
                Step::Next
            }
            Stage::StripGone => {
                fill(&mut self.lib.screen, STRIP); // 0x0D9C3
                self.lib.music.push(Request::StopEffects); // 0x0D9CB, 0104h:0837h
                self.effect(SHRINK_SAMPLE, 0); // 0x0DA5E, 0104h:080Bh(sample, 0, 0)
                self.view = grab(&self.lib.screen, VIEW); // 0x0DA84
                // 0x0DAAB: `mov word [1CF6h], 0`.
                self.stage = Stage::Shrink {
                    since: self.lib.now(),
                    played: false,
                };
                Step::Next
            }
            Stage::Shrink { since, played } => {
                // 0x0DAB6–0x0DAC4: four levels a hundredth, at most 100h.
                let t = i32::try_from((self.lib.now() - since) * SHRINK_RATE)
                    .unwrap_or(FULL)
                    .min(FULL);
                let mut played = played;
                if t >= SHRINK_SAMPLE_AT && !played {
                    self.effect(SHRINK_SAMPLE, 0); // 0x0DADC
                    played = true;
                }
                self.stage = Stage::ShrinkShow { since, played, t };
                Step::Next
            }
            Stage::ShrinkShow { since, played, t } => {
                if !self.lib.wait_retrace() {
                    return Step::Yield; // 0x0DB70
                }
                self.shrink_step(t);
                // 0x0DB87: `cmp di, 100h ; jnl`.
                self.stage = if t < FULL {
                    Stage::Shrink { since, played }
                } else {
                    Stage::Picture
                };
                Step::Next
            }
            _ => Step::Next,
        }
    }

    /// Steps 3 and 4: the picture faded in, the credits rolled.
    fn credits(&mut self) -> Step<()> {
        match self.stage {
            Stage::Picture => {
                self.lib.music.push(Request::StopEffects); // 0x0DB90
                // 0x0DBB9: `0104h:069Ch(song, −1)`.
                self.lib.music.push(Request::Play {
                    piece: SONG,
                    loops: -1,
                });
                // 0x0DC1D–0x0DC5B: the window as it stands into the
                // picture, at the same place.
                copy_rect(&self.lib.screen, &mut self.picture, WINDOW);
                // 0x0DC79, 0x0DC92: the picture's colors black.
                let black = vec![0; 3 * PICTURE_COLORS];
                self.set_dac(self.colors[0], &black);
                self.lib.screen = self.picture.clone(); // 0x0DCA4
                self.stage = Stage::PictureIn { level: 0 };
                Step::Next
            }
            Stage::PictureIn { level } => {
                if !self.lib.wait_retrace() {
                    return Step::Yield; // 0x0DCCB
                }
                // 0x0DCC3, 0x0DCE1: slot 7's colors at `level`, from its
                // first register.
                let rgb = scaled(&self.colors[2..2 + 3 * PICTURE_COLORS], level);
                self.set_dac(self.colors[0], &rgb);
                // 0x0DCE9–0x0DCEE: on to 100h, the last included.
                self.stage = if level < FULL {
                    Stage::PictureIn { level: level + 1 }
                } else {
                    Stage::RollBegin
                };
                Step::Next
            }
            Stage::RollBegin => {
                // 0x0DCF0–0x0DD31: the back buffer the target, and the
                // clip's; 0x0DD35: the roll laid out (at the open).
                self.lib.thaw(); // 0x0DD3D, 1819h:0B9Ah
                self.lib.flush_keys(); // 0x0DD42, 1819h:099Dh
                self.stage = Stage::RollStep {
                    pos: 0,
                    since: self.lib.now(),
                };
                Step::Next
            }
            Stage::RollStep { pos, since } => {
                // 0x0DD4E: the cursor to `pos`, the weights it passed.
                let weight = self.cursor.advance(&self.roll, pos);
                let pause = u64::try_from(weight).unwrap_or(0) * PAUSE_RATE;
                self.stage = Stage::RollPause { pos, since, pause };
                Step::Next
            }
            Stage::RollPause { pos, since, pause } => {
                let Some(event) = self.wait_event(since, pause) else {
                    return Step::Yield;
                };
                // 0x0DD74: the clock set to 0 at the hundredth that ended
                // the pause; 0x0DD8C–0x0DDA5: the step drawn either way.
                self.roll_step(pos);
                self.stage = if event {
                    Stage::Out
                } else {
                    Stage::RollHold {
                        pos,
                        since: since + pause,
                    }
                };
                Step::Next
            }
            Stage::RollHold { pos, since } => {
                let Some(event) = self.wait_event(since, ROLL_STEP) else {
                    return Step::Yield;
                };
                // 0x0DDCA–0x0DDD2: on a line, round at the roll's end;
                // 0x0DDD4: an event ends the roll.
                let next = if pos + 1 >= self.roll.end() {
                    0
                } else {
                    pos + 1
                };
                self.stage = if event {
                    Stage::Out
                } else {
                    Stage::RollStep {
                        pos: next,
                        since: since + ROLL_STEP,
                    }
                };
                Step::Next
            }
            _ => Step::Next,
        }
    }

    /// Step 5: the music faded, the DAC faded to black, the sound stopped.
    fn fade_out(&mut self) -> Step<()> {
        match self.stage {
            Stage::Out => {
                self.lib.hide(); // 0x0DDDB
                // 0x0DDE0: the roll let go; 0x0DDE8: the music faded.
                self.lib.music.push(Request::FadeOut(MUSIC_FADE_MS));
                self.shadow = self.lib.dac.clone();
                self.stage = Stage::OutScale { level: OUT_FROM };
                Step::Next
            }
            Stage::OutScale { level } => {
                // 0x0C257: a poll, its event not looked at; 0x0C268: the
                // shadow scaled in place.
                self.lib.poll();
                let rgb = scaled(&self.shadow.raw, level);
                self.shadow.load(0, &rgb);
                self.stage = Stage::OutRun { level, run: 0 };
                Step::Next
            }
            Stage::OutRun { level, run } => {
                if !self.lib.wait_retrace() {
                    return Step::Yield; // 0x0C272
                }
                let at = run * OUT_RUN * 3;
                let rgb = self.shadow.raw[at..at + OUT_RUN * 3].to_vec();
                self.lib
                    .dac
                    .load(u8::try_from(run * OUT_RUN).unwrap_or(0), &rgb); // 0x0C284
                self.stage = if run + 1 < OUT_RUNS {
                    Stage::OutRun {
                        level,
                        run: run + 1,
                    }
                } else if level - OUT_STEP > OUT_ABOVE {
                    Stage::OutScale {
                        level: level - OUT_STEP,
                    }
                } else {
                    Stage::Black
                };
                Step::Next
            }
            Stage::Black => {
                // 0x0C29E–0x0C2C9: the screen off, the shadow at 0 and
                // written whole, the screen on.
                self.shadow = Palette::default();
                self.lib.dac = Palette::default();
                // 0x0DDF3: the music stopped; 0x0DDF8: every effect cut.
                self.lib.music.push(Request::Stop);
                self.lib.music.push(Request::StopEffects);
                self.stage = Stage::Over;
                Step::Next
            }
            _ => Step::Done(()),
        }
    }

    /// `0104h:080Bh(sample, 0, repeats)`: the row of table A played, then
    /// `repeats` times more.
    fn effect(&mut self, row: u16, repeats: u16) {
        self.lib.music.push(Request::Effect { row, repeats });
    }

    /// `0104h:08D1h(rgb, start, n)`: colors written from `start`; the
    /// shadow with them (`0x01942`).
    fn set_dac(&mut self, start: u8, rgb: &[u8]) {
        self.lib.dac.load(start, rgb);
    }

    /// The polls of a wait from `since` (`0x0DD5A`–`0x0DD72`,
    /// `0x0DDB1`–`0x0DDC8`): a poll at least, then on until `n` hundredths
    /// have passed or an event came. `None` while it waits; whether an
    /// event ended it.
    fn wait_event(&mut self, since: u64, n: u64) -> Option<bool> {
        loop {
            let event = self.lib.poll(); // 1819h:01B0h, [19A6h]
            if event != 0 {
                return Some(true);
            }
            if self.lib.now() - since >= n {
                return Some(false);
            }
            if self.lib.idle() {
                return None;
            }
        }
    }

    /// One step of the strip with `di` columns of each half on the screen
    /// (`0x0D938`–`0x0D99F`): the column left of the left half and the one
    /// right of the right half drawn in 0, then each half one column
    /// narrower, the left one's left part ending at `0A0h`, the right
    /// one's right part starting there.
    fn strip_step(&mut self, di: i32) {
        let screen = &mut self.lib.screen;
        vline(screen, HALF - di, STRIP.y, STRIP.h); // 0x0D945
        vline(screen, di + HALF - 1, STRIP.y, STRIP.h); // 0x0D95A
        let w = di - 1;
        if w <= 0 {
            return; // 0x0D963
        }
        // 0x0D97B: columns 0 to w of the strip at `0A0h − w`; 0x0D99A:
        // its last w columns at `0A0h`.
        blit(
            &self.strip,
            0,
            STRIP.y,
            screen,
            HALF - w,
            STRIP.y,
            w,
            STRIP.h,
        );
        blit(
            &self.strip,
            STRIP.w - w,
            STRIP.y,
            screen,
            HALF,
            STRIP.y,
            w,
            STRIP.h,
        );
    }

    /// One step of the view at level `t` (`0x0DAEE`–`0x0DB7F`): its place
    /// and size stepped between the view and the window, the back
    /// buffer's `140h × 80h` cleared, the view scaled into it there, and
    /// the whole of it onto the screen at `0, 0`.
    fn shrink_step(&mut self, t: i32) {
        let x = lerp(VIEW.x, WINDOW.x, t);
        let y = lerp(VIEW.y, WINDOW.y, t);
        let w = lerp(VIEW.w, WINDOW.w, t);
        let h = lerp(VIEW.h, WINDOW.h, t);
        let mut back = draw::display();
        let extent = (u16::try_from(w).unwrap_or(0), u16::try_from(h).unwrap_or(0));
        // 0x0DB21: `206Fh:0000h`, the buffer set to 0; 0x0DB68: the
        // scaler, which writes every pixel of the box.
        draw::scaled_xy(&mut back, &self.view, x, y, extent, false);
        blit(&back, 0, 0, &mut self.lib.screen, 0, 0, VIEW.w, VIEW.h); // 0x0DB7F
    }

    /// One step of the roll at `pos` (`0x0DD8C`–`0x0DDA5`): the window of
    /// the picture into the back buffer, the lines in view over it, the
    /// window onto the screen.
    fn roll_step(&mut self, pos: i32) {
        let lib = &mut self.lib;
        copy_rect(&self.picture, &mut lib.surface, WINDOW);
        self.roll.draw(&mut lib.surface, &lib.common.font, pos);
        copy_rect(&lib.surface, &mut lib.screen, WINDOW);
    }
}

/// `0DE4h:004Eh`'s step (`0x0DE5C`): `a` to `b` at level `t` of `100h`,
/// the distance's product rounded at its bit 7 (`shl dl, 1 ; sbb dx, dx ;
/// sub ax, dx`); `a` at 0, `b` from `100h` on.
fn lerp(a: i32, b: i32, t: i32) -> i32 {
    if t >= FULL {
        return b;
    }
    if t == 0 {
        return a;
    }
    let d = ((b - a).abs() * t + 0x80) >> 8;
    if a > b { a - d } else { a + d }
}

/// `00D7h:002Bh(src, dst, n, level)`: every channel at `level`.
fn scaled(rgb: &[u8], level: i32) -> Vec<u8> {
    rgb.iter().map(|&v| scale_channel(v, level)).collect()
}

/// `1B72h:00D2h(x, y, w, h, 0)`: a rectangle filled with 0.
fn fill(fb: &mut Framebuffer, r: Rect) {
    draw::fill_rect(fb, r.x, r.y, r.w, r.h, 0);
}

/// `1B72h:0170h(x, y, n, 0)`: `n` pixels down from `x, y` in 0.
fn vline(fb: &mut Framebuffer, x: i32, y: i32, n: i32) {
    for yy in y..y + n {
        fb.set(x, yy, 0);
    }
}

/// `1B72h:0082h(x, y, pic, sx, sy, w, h)`: a rectangle of a picture onto
/// the target, every pixel.
#[expect(clippy::too_many_arguments, reason = "the routine's own seven")]
fn blit(
    from: &Framebuffer,
    sx: i32,
    sy: i32,
    to: &mut Framebuffer,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    for row in 0..h {
        for col in 0..w {
            if let Some(p) = from.get(sx + col, sy + row) {
                to.set(x + col, y + row, p);
            }
        }
    }
}

/// `1B72h:0008h(x, y, w, h, pic)`: a rectangle taken off the target as a
/// picture.
fn grab(from: &Framebuffer, r: Rect) -> Picture {
    let mut pixels = Vec::with_capacity(usize::try_from(r.w * r.h).unwrap_or(0));
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            pixels.push(from.get(x, y).unwrap_or(0));
        }
    }
    Picture {
        width: u16::try_from(r.w).unwrap_or(0),
        height: u16::try_from(r.h).unwrap_or(0),
        pixels,
    }
}

/// `1B72h:0048h(0, 0, pic)`: a picture onto a screen, every pixel.
fn picture_screen(p: &Picture) -> Framebuffer {
    let mut fb = draw::display();
    let w = i32::from(p.width);
    for (i, &px) in p.pixels.iter().enumerate() {
        let i = i32::try_from(i).unwrap_or(i32::MAX);
        fb.set(i % w, i / w, px);
    }
    fb
}

/// The credits' lines: the far pointers `push imm16` names at `0x029A5`,
/// up to the null one (`2044h:000Ch` counts them).
pub(super) fn lines(image: &Image) -> Result<Vec<Vec<u8>>> {
    far_lines(image, LINES_AT, &PUSH_IMM16, "the credits' lines")
}

impl Ending for Credits {
    fn lib(&self) -> &Lib {
        &self.lib
    }

    fn lib_mut(&mut self) -> &mut Lib {
        &mut self.lib
    }

    fn run(&mut self) -> Step<()> {
        self.sequence()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::titles::bifi2::build::BUILD;

    /// The sequence as the Kaffee's end would leave it, or `None` without
    /// the install.
    fn opened() -> Option<Credits> {
        let dir = motionvm_parsec_testutil::gamedata_bifi2()?;
        let image = Image::open(&dir, BUILD.program.0, BUILD.program.1).unwrap();
        let res = motionvm_parsec_formats::detect::resource_dir(&dir);
        let mut screen = draw::display();
        screen.fill(0x11);
        let exit = Exit {
            res: &res,
            image: &image,
            pointer: BUILD.pointer,
            location: END,
            screen,
            dac: Palette::default(),
        };
        Some(Credits::open(&exit).unwrap())
    }

    /// `0DE4h:004Eh`'s step: the ends exact, the middle rounded at half.
    #[test]
    fn the_step_rounds_at_half() {
        assert_eq!(lerp(0x140, 0x72, 0), 0x140);
        assert_eq!(lerp(0x140, 0x72, 0x100), 0x72);
        // (140h − 72h) · 80h = 6700h: 67h exactly.
        assert_eq!(lerp(0x140, 0x72, 0x80), 0x140 - 0x67);
        // 67h · 1 = 67h, bit 7 clear: 0; 67h · 2 = 0CEh: 1.
        assert_eq!(lerp(0, 0x67, 1), 0);
        assert_eq!(lerp(0, 0x67, 2), 1);
    }

    /// Only the location the Kaffee's row names has the sequence.
    #[test]
    fn only_the_end_has_it() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_bifi2() else {
            return;
        };
        let image = Image::open(&dir, BUILD.program.0, BUILD.program.1).unwrap();
        let res = motionvm_parsec_formats::detect::resource_dir(&dir);
        let exit = Exit {
            res: &res,
            image: &image,
            pointer: BUILD.pointer,
            location: -2,
            screen: draw::display(),
            dac: Palette::default(),
        };
        assert!(open(&exit).unwrap().is_none());
    }

    /// The strip closes, the view shrinks, the picture fades in, and the
    /// roll goes until an event; the samples and the music in the
    /// program's order, and 32 levels of four runs a retrace after it.
    #[test]
    fn the_end_plays_through_to_an_event() {
        let Some(mut x) = opened() else {
            return;
        };
        let mut n = 0;
        while !matches!(x.stage, Stage::RollPause { .. } | Stage::RollHold { .. }) {
            assert!(x.tick(), "the sequence goes on");
            n += 1;
            assert!(n < 5000, "the roll is reached");
        }
        assert_eq!(
            x.take_music(),
            [
                Request::Effect { row: 0, repeats: 4 },
                Request::StopEffects,
                Request::Effect { row: 1, repeats: 0 },
                Request::Effect { row: 1, repeats: 0 },
                Request::StopEffects,
                Request::Play {
                    piece: SONG,
                    loops: -1
                },
            ]
        );
        // The picture on the screen, the shrunk view in its window: the
        // view was all 11h, and at the roll's first position no line is
        // in the window yet.
        assert_eq!(x.lib.screen.get(0, 0x80), x.picture.get(0, 0x80));
        assert_eq!(x.lib.screen.get(0, 0xC7), x.picture.get(0, 0xC7));
        assert_eq!(x.lib.screen.get(WINDOW.x, WINDOW.y), Some(0x11));
        assert_eq!(
            x.lib
                .screen
                .get(WINDOW.x + WINDOW.w - 1, WINDOW.y + WINDOW.h - 1),
            Some(0x11)
        );
        for _ in 0..300 {
            assert!(x.tick());
        }
        x.click(true);
        let mut m = 0;
        while x.tick() {
            m += 1;
            assert!(m < 1000, "the sequence ends");
        }
        assert_eq!(
            x.take_music(),
            [
                Request::FadeOut(MUSIC_FADE_MS),
                Request::Stop,
                Request::StopEffects
            ]
        );
        // 32 levels, four runs each.
        assert_eq!(m + 1, 32 * 4);
        assert_eq!(x.lib.dac.raw, Palette::default().raw);
    }

    /// The credits open with a `+` at `35h` and an empty line on the same
    /// place: the cursor passes the `+` once the roll stands at `35h`, and
    /// the step waits `28h · 4` hundredths before it is drawn.
    #[test]
    fn the_roll_pauses_where_the_lines_say() {
        let Some(x) = opened() else {
            return;
        };
        let mut cursor = Cursor::default();
        for p in 0..0x35 {
            assert_eq!(cursor.advance(&x.roll, p), 0, "{p}");
        }
        assert_eq!(cursor.advance(&x.roll, 0x35), 0x28);
        assert_eq!(cursor.advance(&x.roll, 0x36), 0);
    }
}
