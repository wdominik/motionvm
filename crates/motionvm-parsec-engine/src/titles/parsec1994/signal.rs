//! What an animation's signal does, read off the build's own signal
//! routine — *Dunkle Schatten*'s `0EB9h:1D97h` (`0x10927`), *Captain Zins*'s
//! `0A01h:1D48h` (`0x0BD58`), *Bi-Fi 2*'s `09B6h:1A75h` (`0x0B5D5`).
//!
//! Each routine takes the code in `si` (`mov si, [bp+0Ah]`), dispatches
//! `code − 1` through a jump table in its own segment, and every arm ends
//! at the common tail: the code the mode machine waits for compared, the
//! location's animation hook called. What the arms do differs by build —
//! *Dunkle Schatten*'s plays sample 4 for code 3, *Captain Zins*'s sample 14h
//! for code 9, *Bi-Fi 2*'s a sample by the object's id for codes 2, 3, 8
//! and 9 — so the reader runs every arm, for every object id, over the
//! few instruction forms the three routines are made of, and keeps what
//! each one calls. A form it does not know refuses the build.

use super::profile::LibraryMap;
use crate::Result;
use crate::exe::Image;

/// One act of an arm, in the order the arm does it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignalAct {
    /// The held page let go (`0CFDh:1BA2h(k)`).
    Release(u16),
    /// The task armed (`0CFDh:1A2Eh(k)`).
    TaskArm(u16),
    /// The task started (`0CFDh:1A3Fh(k)`).
    TaskStart(u16),
    /// A sample (`0EB9h:1949h(k)`).
    Sample(u16),
    /// The band let go for an owner, the routine the input's unlock
    /// releases the band with (*Captain Zins*'s `078Ah:0666h(k)`).
    BandRelease(u16),
    /// The pending figure animation taken (`[1E50h]`, `0x10988`).
    FigureAnimation,
}

/// What an arm does, at most three acts.
pub(crate) type Acts = [Option<SignalAct>; 3];

/// One code's arm: what it does for most objects, and the ids it does
/// something else for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SignalArm {
    pub acts: Acts,
    pub by_id: [Option<(u8, Acts)>; 10],
}

/// Every code's arm, code `k` at `k − 1`; a code past `count` does
/// nothing but the tail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SignalArms {
    pub count: u16,
    pub arms: [SignalArm; 12],
}

impl SignalArms {
    /// What code `code` does for an object of id `id`.
    pub(crate) fn acts(&self, code: u16, id: u8) -> impl Iterator<Item = SignalAct> {
        let arm = (1..=self.count)
            .contains(&code)
            .then(|| self.arms[usize::from(code - 1)]);
        let acts = arm.map_or([None; 3], |a| {
            a.by_id
                .iter()
                .flatten()
                .find(|(i, _)| *i == id)
                .map_or(a.acts, |(_, x)| *x)
        });
        acts.into_iter().flatten()
    }

    /// The routine's prologue, `push bp ; mov bp, sp ; push si ; mov si,
    /// [bp+0Ah] ; mov bx, si ; dec bx ; cmp bx, n` (`55 8B EC 56 8B 76 0A
    /// 8B DE 4B 83 FB n`), then either `ja end` (`77 r`, *Dunkle Schatten*,
    /// *Captain Zins*) or `jna +3 ; jmp end` (`76 03 E9 rel`, *Bi-Fi 2*), and
    /// `shl bx, 1 ; jmp [cs:bx+t]` (`D1 E3 2E FF A7 t`).
    pub(crate) fn read(img: &Image, map: &LibraryMap) -> Result<Self> {
        let at = map.anim_signal;
        let no = || super::profile::refused("the animation signal", at);
        let base = usize::from(at.0) * 16;
        let start = base + usize::from(at.1);
        let b = img.bytes();
        let head = b.get(start..start + 13).ok_or_else(no)?;
        if head[..12]
            != [
                0x55, 0x8B, 0xEC, 0x56, 0x8B, 0x76, 0x0A, 0x8B, 0xDE, 0x4B, 0x83, 0xFB,
            ]
        {
            return Err(no());
        }
        let count = u16::from(head[12]) + 1;
        let mut p = start + 13;
        let end = match b.get(p..p + 5).ok_or_else(no)? {
            [0x77, r, ..] => {
                let e = near(base, p + 2, i16::from(i8::from_le_bytes([*r])));
                p += 2;
                e
            }
            [0x76, 0x03, 0xE9, lo, hi] => {
                let e = near(base, p + 5, i16::from_le_bytes([*lo, *hi]));
                p += 5;
                e
            }
            _ => return Err(no()),
        };
        let t = b.get(p..p + 7).ok_or_else(no)?;
        if t[..5] != [0xD1, 0xE3, 0x2E, 0xFF, 0xA7] || count > 12 {
            return Err(no());
        }
        let table = base + usize::from(u16::from_le_bytes([t[5], t[6]]));
        let calls = Calls::read(img, map)?;
        let mut arms = [SignalArm::default(); 12];
        for code in 1..=count {
            let k = usize::from(code - 1);
            let entry = base + usize::from(word(b, table + 2 * k).ok_or_else(no)?);
            let mut runs = Vec::with_capacity(256);
            for id in 0..=255u8 {
                runs.push(run(b, base, (entry, end), code, id, &calls).ok_or_else(no)?);
            }
            // The acts most ids share, and the ids apart from them.
            let common = *runs
                .iter()
                .max_by_key(|r| runs.iter().filter(|s| s == r).count())
                .ok_or_else(no)?;
            let mut arm = SignalArm {
                acts: common,
                ..SignalArm::default()
            };
            let mut apart = (0..=255u8).zip(&runs).filter(|(_, r)| **r != common);
            for slot in &mut arm.by_id {
                *slot = apart.next().map(|(id, r)| (id, *r));
            }
            if apart.next().is_some() {
                return Err(no());
            }
            arms[k] = arm;
        }
        Ok(SignalArms { count, arms })
    }
}

/// The far routines an arm may call, and the near sample routine.
struct Calls {
    release: (u16, u16),
    arm: (u16, u16),
    start: (u16, u16),
    sample: (u16, u16),
    band_release: (u16, u16),
}

impl Calls {
    /// The band's release is the far call the input's unlock makes with
    /// `push 4` (`6A 04 9A off seg`).
    fn read(img: &Image, map: &LibraryMap) -> Result<Self> {
        let at = map.input_unlock;
        let b = img
            .bytes()
            .get(usize::from(at.0) * 16 + usize::from(at.1)..)
            .and_then(|b| b.get(..0x20))
            .ok_or_else(|| super::profile::refused("the input's unlock", at))?;
        let i = b
            .windows(3)
            .position(|w| w == [0x6A, 0x04, 0x9A])
            .ok_or_else(|| super::profile::refused("the input's unlock", at))?;
        let band_release = (
            u16::from_le_bytes([b[i + 5], b[i + 6]]),
            u16::from_le_bytes([b[i + 3], b[i + 4]]),
        );
        Ok(Calls {
            release: map.page_release,
            arm: map.task_arm,
            start: map.task_start,
            sample: map.play_sample,
            band_release,
        })
    }
}

fn word(b: &[u8], i: usize) -> Option<u16> {
    b.get(i..i + 2).map(|w| u16::from_le_bytes([w[0], w[1]]))
}

/// A near target in the segment at `base`, from the instruction that ends
/// at `next`.
fn near(base: usize, next: usize, rel: i16) -> usize {
    let off = u16::try_from(next - base)
        .unwrap_or(0)
        .wrapping_add_signed(rel);
    base + usize::from(off)
}

/// The machine an arm runs on: the registers it reads, its pushes and
/// the acts it has called.
struct Cpu<'a> {
    b: &'a [u8],
    base: usize,
    seg: u16,
    code: u16,
    id: u8,
    calls: &'a Calls,
    ax: u16,
    bx: u16,
    zero: bool,
    stack: Vec<u16>,
    acts: Acts,
    n: usize,
}

fn rel8(r: u8) -> i16 {
    i16::from(i8::from_le_bytes([r]))
}

impl Cpu<'_> {
    fn emit(&mut self, a: SignalAct) -> Option<()> {
        *self.acts.get_mut(self.n)? = Some(a);
        self.n += 1;
        Some(())
    }

    /// A far call by what it calls, with the last push as its argument.
    fn far(&mut self, to: (u16, u16)) -> Option<()> {
        let k = *self.stack.last()?;
        let c = self.calls;
        let act = if to == c.release {
            SignalAct::Release(k)
        } else if to == c.arm {
            SignalAct::TaskArm(k)
        } else if to == c.start {
            SignalAct::TaskStart(k)
        } else if to == c.band_release {
            SignalAct::BandRelease(k)
        } else {
            return None;
        };
        self.emit(act)
    }

    /// The pushes, the pops and the calls: the next instruction's place,
    /// or `None` for a form that is not one of them.
    fn step_calls(&mut self, ip: usize, x: &[u8]) -> Option<usize> {
        Some(match *x {
            [0x90, ..] => ip + 1,
            [0x6A, k, ..] => {
                self.stack.push(rel8(k).cast_unsigned());
                ip + 2
            }
            [0x68, lo, hi, ..] => {
                self.stack.push(u16::from_le_bytes([lo, hi]));
                ip + 3
            }
            [0x56, ..] => {
                self.stack.push(self.code);
                ip + 1
            }
            [0x59, ..] => {
                self.stack.pop()?;
                ip + 1
            }
            [0x0E, 0xE8, lo, hi, ..] => {
                let to = near(self.base, ip + 4, i16::from_le_bytes([lo, hi]));
                if (self.seg, u16::try_from(to - self.base).ok()?) != self.calls.sample {
                    return None;
                }
                self.emit(SignalAct::Sample(*self.stack.last()?))?;
                ip + 4
            }
            [0x9A, o0, o1, s0, s1, _] => {
                self.far((u16::from_le_bytes([s0, s1]), u16::from_le_bytes([o0, o1])))?;
                ip + 5
            }
            _ => return None,
        })
    }

    /// The id read, the registers worked and the jumps taken.
    fn step_flow(&mut self, ip: usize, x: &[u8]) -> Option<usize> {
        let base = self.base;
        Some(match *x {
            [0xEB, r, ..] => near(base, ip + 2, rel8(r)),
            [0xE9, lo, hi, ..] => near(base, ip + 3, i16::from_le_bytes([lo, hi])),
            [0xC4, 0x5E, 0x06, ..] => ip + 3,
            [0x26, 0x8A, 0x47, 0x13, ..] => {
                self.ax = (self.ax & 0xFF00) | u16::from(self.id);
                ip + 4
            }
            [0xB0, k, ..] => {
                self.ax = (self.ax & 0xFF00) | u16::from(k);
                ip + 2
            }
            [0xB4, k, ..] => {
                self.ax = (self.ax & 0x00FF) | (u16::from(k) << 8);
                ip + 2
            }
            [0x2D, lo, hi, ..] => {
                self.ax = self.ax.wrapping_sub(u16::from_le_bytes([lo, hi]));
                ip + 3
            }
            [0x3D, lo, hi, ..] => {
                self.zero = self.ax == u16::from_le_bytes([lo, hi]);
                ip + 3
            }
            [op @ (0x74 | 0x75), r, ..] if (op == 0x74) == self.zero => near(base, ip + 2, rel8(r)),
            [0x74 | 0x75, ..] => ip + 2,
            [0x8B, 0xD8, ..] => {
                self.bx = self.ax;
                ip + 2
            }
            [0x83, 0xFB, k, 0x77, r, _] if self.bx > u16::from(k) => near(base, ip + 5, rel8(r)),
            [0x83, 0xFB, _, 0x77, ..] => ip + 5,
            [0xD1, 0xE3, ..] => {
                self.bx = self.bx.wrapping_shl(1);
                ip + 2
            }
            [0x2E, 0xFF, 0xA7, lo, hi, _] => {
                let t = base + usize::from(u16::from_le_bytes([lo, hi]).wrapping_add(self.bx));
                base + usize::from(word(self.b, t)?)
            }
            _ => return None,
        })
    }
}

/// One arm run for code `code` and an object of id `id`, from `entry`
/// until it reaches the tail `end`: the acts it calls, or `None` for a
/// form not read.
fn run(
    b: &[u8],
    base: usize,
    at: (usize, usize),
    code: u16,
    id: u8,
    calls: &Calls,
) -> Option<Acts> {
    let (mut ip, end) = at;
    let mut cpu = Cpu {
        b,
        base,
        seg: u16::try_from(base / 16).ok()?,
        code,
        id,
        calls,
        ax: 0,
        bx: 0,
        zero: false,
        stack: Vec::new(),
        acts: [None; 3],
        n: 0,
    };
    // *Dunkle Schatten*'s figure animation, `mov ax, [m16] ; mov [m16], ax`
    // (`A1 lo hi A3 lo hi`), runs to the tail on its own words.
    if b.get(ip) == Some(&0xA1) && b.get(ip + 3) == Some(&0xA3) {
        cpu.emit(SignalAct::FigureAnimation)?;
        return Some(cpu.acts);
    }
    for _ in 0..64 {
        if ip == end {
            return Some(cpu.acts);
        }
        let x = b.get(ip..ip + 6)?;
        ip = match cpu.step_calls(ip, x) {
            Some(next) => next,
            None => cpu.step_flow(ip, x)?,
        };
    }
    None
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{SignalAct, SignalArms};

    /// Each code's acts for most objects and the ids apart, for a test
    /// to compare.
    /// A code, what it does for most objects, and the ids apart.
    pub(crate) type Listed = (u16, Vec<SignalAct>, Vec<(u8, Vec<SignalAct>)>);

    pub(crate) fn listed(s: &SignalArms) -> Vec<Listed> {
        (1..=s.count)
            .map(|c| {
                let a = s.arms[usize::from(c - 1)];
                let acts = a.acts.into_iter().flatten().collect();
                let apart = a
                    .by_id
                    .iter()
                    .flatten()
                    .map(|(id, x)| (*id, x.iter().flatten().copied().collect()))
                    .collect();
                (c, acts, apart)
            })
            .collect()
    }
}
