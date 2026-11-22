//! The screen effects of segment `10B8h`: the transition out of a
//! location, the entry into one, and the reveal some locations play.
//!
//! The three are synchronous in the original: the program draws straight
//! onto the screen (`A000h`) and waits in a loop, and nothing else runs —
//! no frame pass, no tasks, no input — while the hundredths counter
//! `[1C82h]` goes on. Here a call queues the effect with the picture it
//! starts from, and the runtime plays the queue one tick at a time: while
//! it is not empty the visit's frame loop waits and only the clock and the
//! effect move, so the logic finds the counter where the original leaves
//! it.
//!
//! What follows the effects in the original's own order — the entry's
//! gray ramp, `0590:18EF` after `10B8:0CAB`, and a music call such as the
//! stop after the transition out (`0x0ED55`) — is queued behind them, and
//! the rest of the entry (the pointer, the tasks' first tick) runs when
//! the queue is empty.

use super::music::Call;
use crate::num;
use std::collections::VecDeque;

use motionvm_render::Framebuffer;

/// Playfield rows the effects touch; the band and the strip below stay.
const ROWS: usize = 128;
const WIDTH: usize = 320;

/// The VGA's vertical retrace in mode 13h: 25.175 MHz over 800 × 449
/// clocks, 70.086 Hz. A property of the hardware, not of the program; the
/// effects wait on it through two copies of the same loop on port `3DAh`:
/// the far `1AFA:00F0`, which segment `10B8h` calls (`0x10EAE`,
/// `0x1163D`), and the near `1373:000E`, which `1373:028F` and
/// `1373:02F0` call (`0x139DB`, `0x13A36`) for the slide of `10B8:0CAB`
/// (`0x1188D`, `0x1192C`). Counted against
/// the PIT's 1 193 182 Hz in integers: a retrace every
/// `1193182 · 359200 / 25175000` PIT cycles.
pub(crate) const RETRACE_NUM: u64 = 1_193_182 * 800 * 449;
pub(crate) const RETRACE_DEN: u64 = 25_175_000;

/// One queued piece.
#[derive(Debug, Clone)]
enum Fx {
    /// `10B8:0210(n)`: the picture tinted by mask `n`, flattened onto the
    /// ramp 192..207, cut into 4×4 blocks, and grown onto the screen in 32
    /// rings from the middle while the ramp darkens.
    Exit { mask: u8, picture: Vec<u8> },
    /// `10B8:0CAB`: the new picture slides in, even rows from the left and
    /// odd rows from the right, 32 steps of ten columns.
    Entry { picture: Vec<u8> },
    /// `10B8:03BA`: the new picture rolls down in eight-row bands behind
    /// black edges, 24 steps.
    Reveal { picture: Vec<u8> },
    /// A DAC write that follows the effects in the original's order.
    Dac { start: u8, rgb: Vec<u8> },
    /// A music routine called after the effects in the original's order,
    /// handed back to the runtime when its turn comes.
    Music(Call),
}

/// Where a playing effect is in its loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Waiting for `n` more retraces before the next draw.
    Retrace(u8),
    /// Waiting for `[1C82h]` to reach the step's deadline.
    Deadline,
}

/// The effect queue and the screen the effects draw on.
#[derive(Debug, Clone, Default)]
pub(crate) struct Effects {
    queue: VecDeque<Fx>,
    /// The screen while effects play: what was presented when the first of
    /// them was queued, drawn over.
    pub screen: Option<Framebuffer>,
    /// The playing effect's state: its step, its deadline, its phase, and
    /// for the exit its flattened mosaic and its ramp.
    started: bool,
    step: u16,
    deadline: u16,
    phase: Option<Phase>,
    mosaic: Vec<u8>,
    ramp: Vec<u8>,
    /// PIT cycles times `RETRACE_DEN`, towards the next retrace.
    retrace_acc: u64,
    /// The music calls whose turn came this tick, for the runtime to run.
    due: Vec<Call>,
}

impl Effects {
    /// Whether an effect is queued or playing.
    pub(crate) fn busy(&self) -> bool {
        !self.queue.is_empty()
    }

    /// The effects are called where the original draws them at once, so
    /// from the call on the screen is theirs: what was presented last, which
    /// they draw over. A frame shown before the first of them takes its
    /// step shows that screen too, not the picture the caller has gone on
    /// to build.
    fn hold(&mut self, presented: &Framebuffer) {
        if self.screen.is_none() {
            self.screen = Some(presented.clone());
        }
    }

    pub(crate) fn exit(&mut self, mask: u8, picture: &Framebuffer, presented: &Framebuffer) {
        self.hold(presented);
        self.queue.push_back(Fx::Exit {
            mask,
            picture: playfield(picture),
        });
    }

    pub(crate) fn entry(&mut self, picture: &Framebuffer, presented: &Framebuffer) {
        self.hold(presented);
        self.queue.push_back(Fx::Entry {
            picture: playfield(picture),
        });
    }

    pub(crate) fn reveal(&mut self, picture: &Framebuffer, presented: &Framebuffer) {
        self.hold(presented);
        self.queue.push_back(Fx::Reveal {
            picture: playfield(picture),
        });
    }

    /// A DAC write: queued behind a playing effect, else made at once.
    pub(crate) fn dac(&mut self, dac: &mut motionvm_render::Palette, start: u8, rgb: &[u8]) {
        if self.busy() {
            self.queue.push_back(Fx::Dac {
                start,
                rgb: rgb.to_vec(),
            });
        } else {
            dac.load(start, rgb);
        }
    }

    /// A music routine called while effects play: queued behind them.
    pub(crate) fn music(&mut self, call: Call) {
        self.queue.push_back(Fx::Music(call));
    }

    /// The music calls whose turn has come, in order.
    pub(crate) fn take_due(&mut self) -> Vec<Call> {
        std::mem::take(&mut self.due)
    }

    /// One tick of `pit` PIT cycles while effects play; `hundredths` is
    /// `[1C82h]`.
    pub(crate) fn tick(
        &mut self,
        pit: u32,
        dac: &mut motionvm_render::Palette,
        hundredths: &mut u16,
    ) {
        // The retraces this tick brings.
        self.retrace_acc += u64::from(pit) * RETRACE_DEN;
        let mut retraces = u8::try_from(self.retrace_acc / RETRACE_NUM).unwrap_or(u8::MAX);
        self.retrace_acc %= RETRACE_NUM;
        loop {
            let Some(fx) = self.queue.front().cloned() else {
                self.screen = None;
                return;
            };
            if let Fx::Dac { start, rgb } = &fx {
                dac.load(*start, rgb);
                self.queue.pop_front();
                continue;
            }
            if let Fx::Music(call) = fx {
                self.due.push(call);
                self.queue.pop_front();
                continue;
            }
            if !self.started {
                self.start(&fx, dac, hundredths);
            }
            match self.phase {
                Some(Phase::Retrace(n)) => {
                    if retraces == 0 {
                        return;
                    }
                    retraces -= 1;
                    if n > 1 {
                        self.phase = Some(Phase::Retrace(n - 1));
                        // The entry draws a half step after each retrace.
                        if matches!(fx, Fx::Entry { .. }) {
                            self.draw(&fx, dac, 0);
                        }
                        continue;
                    }
                    let half = u8::from(matches!(fx, Fx::Entry { .. }));
                    self.draw(&fx, dac, half);
                    self.phase = Some(Phase::Deadline);
                }
                Some(Phase::Deadline) => {
                    if *hundredths < self.deadline {
                        return;
                    }
                    self.step += 1;
                    if self.step >= steps(&fx) {
                        self.finish();
                        continue;
                    }
                    self.next_step(&fx, dac);
                }
                None => unreachable!("a started effect has a phase"),
            }
        }
    }

    /// The effect's prologue: the counter zeroed, the exit's ramp written
    /// and its mosaic made.
    fn start(&mut self, fx: &Fx, dac: &mut motionvm_render::Palette, hundredths: &mut u16) {
        self.started = true;
        self.step = 0;
        self.deadline = 0;
        *hundredths = 0;
        if let Fx::Exit { mask, picture } = fx {
            // 0x10DB7: no mask is gray. 0x10DD6..0x10E15: bit 0 red, bit 1
            // green, bit 2 blue; 0x10E2C: the ramp 0, 4, …, 60 to 192..207.
            let mask = if *mask == 0 { 7 } else { *mask };
            let mut ramp = vec![0u8; 48];
            for i in 0..16 {
                for c in 0..3 {
                    if mask & (1 << c) != 0 {
                        ramp[i * 3 + c] = num::byte(4 * i);
                    }
                }
            }
            dac.load(192, &ramp);
            self.ramp = ramp;
            // 1373:015E over the shadow just written: 192 + ((5r+9g+2b)>>6).
            let lut: Vec<u8> = (0..256)
                .map(|i| {
                    let [r, g, b] = [0, 1, 2].map(|c| u32::from(dac.raw[i * 3 + c]));
                    192 + num::low8(num::low_word((5 * r + 9 * g + 2 * b) >> 6))
                })
                .collect();
            let flat: Vec<u8> = picture.iter().map(|&p| lut[usize::from(p)]).collect();
            // 1373:01BD(4, 4): each 4×4 block the floor of its mean.
            let mut mosaic = flat.clone();
            for by in (0..ROWS).step_by(4) {
                for bx in (0..WIDTH).step_by(4) {
                    let mut sum = 0u32;
                    for y in by..by + 4 {
                        for x in bx..bx + 4 {
                            sum += u32::from(flat[y * WIDTH + x]);
                        }
                    }
                    let v = num::low8(num::low_word(sum / 16));
                    for y in by..by + 4 {
                        for x in bx..bx + 4 {
                            mosaic[y * WIDTH + x] = v;
                        }
                    }
                }
            }
            self.mosaic = mosaic;
        }
        self.next_step(fx, dac);
    }

    /// The next step's deadline and what it waits for first.
    fn next_step(&mut self, fx: &Fx, dac: &mut motionvm_render::Palette) {
        match fx {
            Fx::Exit { .. } => {
                self.deadline = self.deadline.wrapping_add(3);
                self.phase = Some(Phase::Retrace(1));
            }
            Fx::Entry { .. } => {
                self.deadline = self.deadline.wrapping_add(2);
                self.phase = Some(Phase::Retrace(2));
            }
            Fx::Reveal { .. } => {
                self.deadline = self.deadline.wrapping_add(4);
                self.draw(fx, dac, 0);
                self.phase = Some(Phase::Deadline);
            }
            Fx::Dac { .. } | Fx::Music(_) => {}
        }
    }

    fn finish(&mut self) {
        self.queue.pop_front();
        self.started = false;
        self.phase = None;
        self.mosaic.clear();
    }

    /// Draws step `self.step` (for the entry, its half `half`).
    fn draw(&mut self, fx: &Fx, dac: &mut motionvm_render::Palette, half: u8) {
        let k = usize::from(self.step);
        let Some(screen) = self.screen.as_mut() else {
            return;
        };
        let px = &mut screen.pixels;
        match fx {
            Fx::Exit { .. } => {
                // 0x10E9x: the ramp scaled by f = 252 - 4k, after the vsync.
                let f = 252 - 4 * num::narrow(k);
                let tmp: Vec<u8> = self
                    .ramp
                    .iter()
                    .map(|&v| num::low8(num::low_word(((u32::from(v) * f + 128) >> 8).min(63))))
                    .collect();
                dac.load(192, &tmp);
                // 10B8:0181: the ring x∈[155-5k, 165+5k), y∈[62-2k, 66+2k).
                let (x0, y0) = (155 - 5 * k, 62 - 2 * k);
                let (w, h) = (10 + 10 * k, 4 + 4 * k);
                let mut copy = |x: usize, y: usize, cw: usize, ch: usize| {
                    for yy in y..y + ch {
                        let a = yy * WIDTH + x;
                        px[a..a + cw].copy_from_slice(&self.mosaic[a..a + cw]);
                    }
                };
                copy(x0, y0, w, 2);
                copy(x0, y0, 5, h);
                copy(x0, y0 + h - 2, w, 2);
                copy(x0 + w - 5, y0, 5, h);
            }
            Fx::Entry { picture } => {
                // 0x11892 / 0x118A7: n = 10(k+1) columns.
                let n = 10 * (k + 1);
                for y in (usize::from(half)..ROWS).step_by(2) {
                    let row = y * WIDTH;
                    if half == 0 {
                        // Even rows: S(x, y) = B(x + 320 - n, y), x < n.
                        px[row..row + n].copy_from_slice(&picture[row + WIDTH - n..row + WIDTH]);
                    } else {
                        // Odd rows: S(x, y) = B(x - (320 - n), y).
                        px[row + WIDTH - n..row + WIDTH].copy_from_slice(&picture[row..row + n]);
                    }
                }
                if half == 1 && k == 31 {
                    // 0x118E6: the whole playfield once more.
                    px[..ROWS * WIDTH].copy_from_slice(&picture[..ROWS * WIDTH]);
                }
            }
            Fx::Reveal { picture } => {
                let base = -64 + 8 * num::count(k);
                for j in 0..9i32 {
                    let r = base + 8 * j;
                    if r >= num::count(ROWS) {
                        break;
                    }
                    if r < 0 {
                        continue;
                    }
                    let d = 160 - 20 * j;
                    let mut hline = |x: i32, y: i32, len: i32| {
                        if len > 0 && (0..num::count(ROWS)).contains(&y) {
                            let a = num::index(y) * WIDTH + num::index(x);
                            px[a..a + num::index(len)].fill(0);
                        }
                    };
                    if j >= 1 {
                        hline(160 - d, r + 8 - j, 2 * d);
                    }
                    if j >= 2 {
                        hline(160 - d / 2, r + 9 - j, d);
                    }
                    if j >= 3 {
                        hline(160 - d / 4, r + 9 - j, d / 2);
                    }
                    if j <= 7 {
                        let (src, dst) = (num::index(r + j) * WIDTH, num::index(r) * WIDTH);
                        let n = WIDTH * (8 - num::index(j));
                        let n = n.min(ROWS * WIDTH - src).min(ROWS * WIDTH - dst);
                        px[dst..dst + n].copy_from_slice(&picture[src..src + n]);
                    }
                }
            }
            Fx::Dac { .. } | Fx::Music(_) => {}
        }
    }
}

/// How many steps an effect runs.
fn steps(fx: &Fx) -> u16 {
    match fx {
        Fx::Exit { .. } | Fx::Entry { .. } => 32,
        Fx::Reveal { .. } => 24,
        Fx::Dac { .. } | Fx::Music(_) => 0,
    }
}

/// The playfield rows of a picture.
fn playfield(fb: &Framebuffer) -> Vec<u8> {
    fb.pixels[..ROWS * WIDTH].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use motionvm_render::Palette;

    /// A picture whose every playfield pixel says where it is.
    fn picture() -> Framebuffer {
        let mut fb = crate::draw::display();
        for y in 0..200 {
            for x in 0..320 {
                fb.pixels[y * 320 + x] = u8::try_from((7 * x + 13 * y) % 251 + 1).unwrap();
            }
        }
        fb
    }

    /// Plays the queue to its end with a clock that runs as the INT 8
    /// handler's does: a hundredth every 2E9Ah PIT cycles. Answers the
    /// ticks it took and the counter it left.
    fn play(fx: &mut Effects, dac: &mut Palette) -> (u32, u16) {
        let mut hundredths = 0u16;
        let mut pit = 0u32;
        let mut ticks = 0;
        while fx.busy() {
            pit += crate::titles::parsec1994::clock::TICK_PERIOD;
            while pit >= 0x2E9A {
                pit -= 0x2E9A;
                hundredths += 1;
            }
            fx.tick(
                crate::titles::parsec1994::clock::TICK_PERIOD,
                dac,
                &mut hundredths,
            );
            ticks += 1;
            assert!(ticks < 10_000);
        }
        (ticks, hundredths)
    }

    #[test]
    fn the_reveal_ends_on_the_new_picture_after_96_hundredths() {
        let (old, new) = (crate::draw::display(), picture());
        let mut fx = Effects::default();
        let mut dac = Palette::default();
        fx.reveal(&new, &old);
        // The screen as it stands at the end is the last one drawn; take
        // it before the queue empties by playing all but the final wait.
        let mut hundredths = 0u16;
        let mut pit = 0u32;
        let mut last = None;
        while fx.busy() {
            pit += crate::titles::parsec1994::clock::TICK_PERIOD;
            while pit >= 0x2E9A {
                pit -= 0x2E9A;
                hundredths += 1;
            }
            fx.tick(
                crate::titles::parsec1994::clock::TICK_PERIOD,
                &mut dac,
                &mut hundredths,
            );
            if let Some(s) = &fx.screen {
                last = Some(s.clone());
            }
        }
        let last = last.expect("the effect drew");
        assert_eq!(last.pixels[..ROWS * WIDTH], new.pixels[..ROWS * WIDTH]);
        assert_eq!(last.pixels[ROWS * WIDTH..], old.pixels[ROWS * WIDTH..]);
        assert!(hundredths >= 96, "24 deadlines of four: {hundredths}");
    }

    #[test]
    fn the_entry_slides_in_over_64_retraces() {
        let (old, new) = (crate::draw::display(), picture());
        let mut fx = Effects::default();
        let mut dac = Palette::default();
        fx.entry(&new, &old);
        let (ticks, hundredths) = play(&mut fx, &mut dac);
        // 64 retraces at 70.086 Hz are 0.913 s, about 110 ticks of 120 Hz;
        // the deadline of two hundredths a step never binds.
        assert!((108..=112).contains(&ticks), "{ticks} ticks");
        assert!(hundredths >= 64);
    }

    #[test]
    fn the_exit_darkens_the_tinted_ramp_to_half() {
        let (old, new) = (crate::draw::display(), picture());
        let mut fx = Effects::default();
        let mut dac = Palette::default();
        // Mask 3, yellow: red and green, as the reference run rolled.
        fx.exit(3, &new, &old);
        let (_, hundredths) = play(&mut fx, &mut dac);
        assert!(hundredths >= 96);
        // The last step scales by 128/256: the ramp 4i becomes 2i.
        for i in 0..16 {
            let rgb = &dac.raw[(192 + i) * 3..(192 + i) * 3 + 3];
            assert_eq!(
                rgb,
                [
                    2 * u8::try_from(i).unwrap(),
                    2 * u8::try_from(i).unwrap(),
                    0
                ],
                "entry {i}"
            );
        }
    }

    #[test]
    fn the_exit_mosaic_is_the_floor_of_each_flattened_block() {
        let mut fx = Effects::default();
        let mut dac = Palette::default();
        // A picture in one color whose DAC entry is mid-gray: every block
        // flattens to one value.
        dac.load(5, &[32, 32, 32]);
        let mut pic = crate::draw::display();
        pic.pixels[..ROWS * WIDTH].fill(5);
        fx.exit(1, &pic, &pic);
        let mut h = 0;
        fx.tick(
            crate::titles::parsec1994::clock::TICK_PERIOD,
            &mut dac,
            &mut h,
        );
        // 192 + ((5·32 + 9·32 + 2·32) >> 6) = 192 + 8.
        assert!(fx.mosaic.iter().all(|&v| v == 200), "{:?}", &fx.mosaic[..4]);
    }
}
