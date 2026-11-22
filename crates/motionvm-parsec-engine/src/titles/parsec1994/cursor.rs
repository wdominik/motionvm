//! The mouse pointer: the library at `179Eh` (`0x179E0`).
//!
//! The library keeps its state in `DGROUP`, and every word below is named
//! by the address it lives at. A pointer is a 32-byte *descriptor*:
//!
//! | offset | field | read at |
//! |---|---|---|
//! | `+0` | far pointer to the cel table (`[195Eh]`, the pointer file from resource 1) | `0x189DB` |
//! | `+4`, `+6` | signed offset from the mouse to the cel's corner | `0x188A3`, `0x188AF` |
//! | `+8` | flags: bits 0–1 the phase mapper, 2–3 the duration function, 10h one-shot, 20h taken at once, 80h no resume, 100h static, 200h hold the last phase | `0x189C1` |
//! | `+0Ah` | the first cel, an index into the table | `0x189D3` |
//! | `+0Ch` | a duration | `0x188DE` |
//! | `+0Eh` | the number of phases | `0x18826` |
//! | `+10h` | a second duration, a modulus, or one byte per phase | `0x188E7`, `0x1890B`, `0x188F1` |
//!
//! A *selector* names the descriptor shown: below `80h` it is `DS:0334h`
//! plus the selector (`0x189B5`) — 0 is the copy the mode setter fills,
//! `40h` the one the timeout shows — and from `80h` on it is a mouse
//! region's shape, `[3B4h]` plus the selector less `80h` (`0x1897F`).
//!
//! The timer calls `179Eh:0A90h` once a mouse unit (two hundredths) with
//! the number of units since the last call. The call takes the mouse's
//! movement, counts the timeout down, asks the regions which shape they
//! want and steps the animation (`0x18492`–`0x188D5`). What it draws, it
//! draws at the mouse plus the descriptor's offset.
//!
//! **What is not modelled.** The driver's mickeys and their acceleration
//! (`0x184D9`–`0x185FA`): the position is the host's, clamped to the
//! limits, and a change sets the same bits 0 and 1 of `[2F6h]`. The
//! buttons' click timing and pressed shapes (`0x18733`–`0x187C2`): the
//! only shaped region the game registers, the menu's, names the same shape for
//! hover and press (`0101h`). Regions whose shape word is 0 — the room's
//! hotspots, the panel's widgets — are not told apart from one another or
//! from no region: what the hit test does with them differs only when a
//! region's selector of `80h` or more is shown away from its region,
//! which the timeout's parking (`0x18997`) alone can bring about. The
//! save-under in video memory (`0x17C50`, `0x17D5F`): the cel is drawn
//! over the finished frame.

use crate::num;
use motionvm_parsec_formats::Picture;

/// Where one build keeps the pointer library's state, and what its loader
/// leaves there. Every address is a `DGROUP` offset unless it says image.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PointerMap {
    /// The four descriptors the mode setter copies from, by mode 1–4.
    pub modes: [u16; 4],
    /// The descriptor they are copied into, selector 0.
    pub slot: u16,
    /// What `[3B4h]` holds: the descriptor of region shape 1.
    pub regions: u16,
    /// The descriptors the loader centres on a cel, `-(w >> 1)`,
    /// `-(h >> 1)` (`sar` then `neg`), with the cel's index in the table.
    pub centered: [(u16, usize); 4],
    /// The mouse library's own generator, `[1864h]`.
    pub seed: u16,
    /// `[3C2h]`, the timeout's reload; `[3C4h]`, the countdown, follows.
    pub timeout: u16,
    /// `[314h]`, the timer interval in hundredths the loader registers.
    pub interval: u16,
    /// `[30Ah]`: the first of the four limits, x min, y min, x max, y max.
    pub limits: u16,
    /// `[3E6h]`: the position, x then y.
    pub position: u16,
    /// `[1962h]`, the mode the setter keeps.
    pub mode: u16,
    /// `[2F6h]`: what changed since the pointer was last drawn.
    pub dirty: u16,
    /// `[316h]`: the first of eleven words in a row — drawn and frozen,
    /// backlog, elapsed, selector, flags, time left, phase drawn, phase,
    /// request, last selector, resume selector (`[316h]`–`[32Ah]`).
    pub state: u16,
    /// `[3EEh]`: where the cel is drawn, x then y.
    pub drawn: u16,
    /// Image offset of the menu's mouse region, the one region with a
    /// shape word. `+4` flags, `+6` shape, `+8` … `+0Eh` the rectangle.
    pub menu_region: usize,
}

impl PointerMap {
    /// Dunkle Schatten's `XP01.EXE`.
    pub(crate) const DS1: PointerMap = PointerMap {
        // `0590h:064Dh`'s table at `cs:06A1h`: `push 234h` (0x05F71),
        // `274h` (0x05F83), `2B4h` (0x05F8D), `2D4h` (0x05F97), each
        // copied to `push 334h` by `179Eh:0960h`.
        modes: [0x0234, 0x0274, 0x02B4, 0x02D4],
        slot: 0x0334,
        // `0x05EC4`: `mov [3B6h], ds ; mov word [3B4h], 254h`.
        regions: 0x0254,
        // `0x05DFA` (`[195Eh]+4`), `0x05E47` (`+14h`), `0x05E6E`
        // (`+18h`), `0x05E95` (`+8`).
        centered: [(0x0254, 1), (0x02B4, 5), (0x02D4, 6), (0x0374, 2)],
        // `0x188FD`: `mul word [1864h]`.
        seed: 0x1864,
        // `0x17ABE`: `mov ax, [3C2h] ; xchg [3C4h], ax`.
        timeout: 0x03C2,
        // `0x05EE0`: `push word [314h]`.
        interval: 0x0314,
        // `0x185B2`, `0x185BA`, `0x185F1`, `0x185F9`.
        limits: 0x030A,
        position: 0x03E6,
        // `0x05F5A`: `mov [1962h], ax`.
        mode: 0x1962,
        // `0x18A0F`, `0x18A01`, `0x188CB`.
        dirty: 0x02F6,
        state: 0x0316,
        drawn: 0x03EE,
        // `0x0DC8B`: `push 2139h ; push 2Ah ; call far 179Eh:0210h`.
        menu_region: 0x2139 * 16 + 0x2A,
    };
}

/// One 32-byte descriptor, as it stands in `DGROUP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Descriptor {
    /// `+4`, `+6`.
    pub offset: (i16, i16),
    /// `+8`.
    pub flags: u16,
    /// `+0Ah`.
    pub base: u16,
    /// `+0Ch`.
    pub duration: u16,
    /// `+0Eh`.
    pub phases: u16,
    /// `+10h` … `+1Fh`.
    pub tail: [u8; 16],
}

impl Descriptor {
    fn read(ds: &[u8], at: u16) -> Self {
        let w = |o: u16| {
            let a = usize::from(at.wrapping_add(o));
            u16::from_le_bytes([ds[a], ds[a + 1]])
        };
        let mut tail = [0; 16];
        let a = usize::from(at) + 0x10;
        tail.copy_from_slice(&ds[a..a + 16]);
        Descriptor {
            offset: (num::signed(w(4)), num::signed(w(6))),
            flags: w(8),
            base: w(0x0A),
            duration: w(0x0C),
            phases: w(0x0E),
            tail,
        }
    }

    /// The word at `+10h`.
    fn word_10(&self) -> u16 {
        u16::from_le_bytes([self.tail[0], self.tail[1]])
    }
}

/// Which region the hit test found last, `[3FAh]`/`[3FCh]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    /// The far pointer is null: no region, or one this model does not tell
    /// apart from the next (every region but the menu's has shape 0).
    Elsewhere,
    /// The menu's region.
    Menu,
}

/// The menu's region record, read from the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Region {
    flags: u16,
    shape: u16,
    rect: (u16, u16, u16, u16),
}

/// The timer's slot for the callback: `+4` countdown, `+6` calls not yet
/// made, `+8` reload (`0x1A801`–`0x1A805`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slot {
    countdown: u16,
    pending: u16,
    reload: u16,
}

impl Slot {
    /// `0x1A226`–`0x1A250`: `hundredths` increments of the scheduler's
    /// own counter. Answers the count the callback is handed, if it is
    /// called.
    fn advance(&mut self, hundredths: u16) -> Option<u16> {
        if self.reload == 0 {
            return None;
        }
        let mut count = self.pending;
        let (left, borrow) = self.countdown.overflowing_sub(hundredths);
        self.countdown = left;
        if borrow {
            loop {
                count = count.wrapping_add(1);
                let (left, carry) = self.countdown.overflowing_add(self.reload);
                self.countdown = left;
                if carry {
                    break;
                }
            }
        } else if count == 0 {
            return None;
        }
        self.pending = 0;
        Some(count)
    }
}

/// The library's state, word by word.
#[derive(Debug, Clone)]
pub(crate) struct Cursor {
    map: PointerMap,
    /// Every descriptor the library can name, by its `DGROUP` address.
    descriptors: Vec<(u16, Descriptor)>,
    /// `[316h]`: bit 0 drawn, bit 15 frozen.
    pub state: u16,
    /// `[318h]`: units counted while the update did not run.
    pub backlog: u16,
    /// `[31Ah]`: the units this update steps by.
    elapsed: u16,
    /// `[31Ch]`: the selector shown.
    pub current: u16,
    /// `[31Eh]`: the shown descriptor's flags, as the animation keeps them.
    flags: u16,
    /// `[320h]`: units left in this phase.
    pub left: u16,
    /// `[322h]`: the phase drawn, after the mapper.
    pub shown: u16,
    /// `[324h]`: the phase counter.
    pub phase: u16,
    /// `[326h]`: the selector asked for.
    pub request: u16,
    /// `[328h]`: the last selector other than the timeout's.
    pub last: u16,
    /// `[32Ah]`: the selector an `80h` descriptor goes back to.
    resume: u16,
    /// `[32Ch]`: the cel drawn, an index into the table.
    pub cel: u16,
    /// `[330h]`: the address of the shown descriptor.
    desc: u16,
    /// `[3C2h]`, `[3C4h]`.
    reload: u16,
    pub timeout: u16,
    /// `[2F6h]`: what changed since the pointer was last drawn.
    dirty: u16,
    /// `[3E6h]`, `[3E8h]`: where the library has the mouse.
    pub pos: (i16, i16),
    /// `[3EEh]`, `[3F0h]`: where the cel is drawn.
    pub drawn: (i16, i16),
    /// `[30Ah]`–`[310h]`.
    limits: [i16; 4],
    /// `[3FAh]`.
    hit: Hit,
    /// `[1864h]`.
    pub seed: u16,
    /// `[1962h]`.
    pub mode: u16,
    /// Whether the menu's region is in the list (`179Eh:0210h`, `0730h`).
    pub menu_registered: bool,
    menu: Region,
    slot: Slot,
}

fn word(ds: &[u8], at: u16) -> u16 {
    let a = usize::from(at);
    u16::from_le_bytes([ds[a], ds[a + 1]])
}

impl Cursor {
    /// The state the loader leaves (`0x05D45`–`0x05F0A`): descriptors read
    /// from `ds` and centered on the cels, `0234h` copied to the slot, the
    /// library initialised (`179Eh:0A00h`), the pointer hidden
    /// (`0x05EFB`), the input polled (`0x05F00`) and mode 1 noted
    /// (`0x05F0A`). `cels` is the table: resource 1 of the pointer file
    /// first.
    pub(crate) fn load(map: PointerMap, ds: &[u8], image: &[u8], cels: &[Option<Picture>]) -> Self {
        let rw = |o: usize| u16::from_le_bytes([image[o], image[o + 1]]);
        let r = map.menu_region;
        let menu = Region {
            flags: rw(r + 4),
            shape: rw(r + 6),
            rect: (rw(r + 8), rw(r + 0x0A), rw(r + 0x0C), rw(r + 0x0E)),
        };
        let mut addrs: Vec<u16> = map.modes.to_vec();
        addrs.push(map.slot);
        addrs.push(map.slot + 0x40);
        // The region shapes the menu's record names, hover and pressed:
        // shape n is `[3B4h] + (n - 1) * 20h` (`0x186BD`).
        for n in menu.shape.to_le_bytes() {
            if n != 0 {
                addrs.push(map.regions + ((u16::from(n) - 1) << 5));
            }
        }
        addrs.sort_unstable();
        addrs.dedup();
        let mut descriptors: Vec<(u16, Descriptor)> = addrs
            .into_iter()
            .map(|a| (a, Descriptor::read(ds, a)))
            .collect();
        for &(at, cel) in &map.centered {
            let Some(Some(p)) = cels.get(cel) else {
                continue;
            };
            if let Some((_, d)) = descriptors.iter_mut().find(|(a, _)| *a == at) {
                d.offset = (-(num::signed(p.width) >> 1), -(num::signed(p.height) >> 1));
            }
        }
        // `0x05EB2`–`0x05EBC`: `0234h` copied to `0334h`, 20h bytes.
        let first = descriptors
            .iter()
            .find(|(a, _)| *a == map.modes[0])
            .map(|(_, d)| *d)
            .unwrap_or_default();
        if let Some((_, d)) = descriptors.iter_mut().find(|(a, _)| *a == map.slot) {
            *d = first;
        }
        let interval = word(ds, map.interval);
        let pos = (
            num::signed(word(ds, map.position)),
            num::signed(word(ds, map.position + 2)),
        );
        let st = |k: u16| word(ds, map.state + 2 * k);
        let mut c = Cursor {
            map,
            descriptors,
            state: st(0),
            backlog: st(1),
            elapsed: st(2),
            current: st(3),
            flags: st(4),
            left: st(5),
            shown: st(6),
            phase: st(7),
            request: st(8),
            last: st(9),
            resume: st(10),
            cel: 0,
            desc: map.slot,
            reload: word(ds, map.timeout),
            timeout: word(ds, map.timeout + 2),
            dirty: word(ds, map.dirty),
            pos,
            drawn: (
                num::signed(word(ds, map.drawn)),
                num::signed(word(ds, map.drawn + 2)),
            ),
            limits: [0, 2, 4, 6].map(|k| num::signed(word(ds, map.limits + k))),
            hit: Hit::Elsewhere,
            seed: word(ds, map.seed),
            mode: word(ds, map.mode),
            menu_registered: false,
            menu,
            // `0x1A801`, `0x1A805`: the interval into `+8` and `+4`.
            slot: Slot {
                countdown: interval,
                pending: 0,
                reload: interval,
            },
        };
        c.init();
        c.hide();
        c.poll();
        c.mode = 1;
        c
    }

    fn descriptor(&self, at: u16) -> Descriptor {
        self.descriptors
            .iter()
            .find(|(a, _)| *a == at)
            .map(|(_, d)| *d)
            .unwrap_or_default()
    }

    fn shown_descriptor(&self) -> Descriptor {
        self.descriptor(self.desc)
    }

    /// Whether the pointer is on the screen.
    pub(crate) fn visible(&self) -> bool {
        self.state & 1 != 0
    }

    /// The cel to draw, its index in the table, and where.
    pub(crate) fn sprite(&self) -> Option<(usize, i32, i32)> {
        self.visible().then_some((
            usize::from(self.cel),
            i32::from(self.drawn.0),
            i32::from(self.drawn.1),
        ))
    }

    /// The position clamped to the limits, as `0x185B2`–`0x185C1` and
    /// `0x185F1`–`0x18602` clamp it.
    pub(crate) fn clamp(&self, x: i32, y: i32) -> (i32, i32) {
        let [x0, y0, x1, y1] = self.limits.map(i32::from);
        let c = |v: i32, lo: i32, hi: i32| {
            if v < lo {
                lo
            } else if v > hi {
                hi
            } else {
                v
            }
        };
        (c(x, x0, x1), c(y, y0, y1))
    }

    /// `179Eh:0A00h`: the library started with the slot's descriptor.
    ///
    /// With `ax` zero the pointer is neither drawn nor frozen (`[316h]`,
    /// `0x18419`), the phase counter `[324h]` is cleared, and selector 0 is
    /// asked for (`[326h]`, `0x1841F`) and taken (`179Eh:0F78h`, `0x18422`);
    /// then everything is marked changed (`[2F6h] = FFFFh`, `0x18428`) and the
    /// shape worked out (`179Eh:0C1Bh`, `0x1842C`).
    fn init(&mut self) {
        self.state = 0;
        self.phase = 0;
        self.request = 0;
        self.select();
        self.dirty = 0xFFFF;
        let mut si = 0xFFFF;
        self.shape(&mut si);
    }

    /// `179Eh:0F78h`: the selector asked for taken, unless the timeout's
    /// is shown, which only notes it.
    ///
    /// The ask is `[326h]` (`0x1895C`), the shown selector `[31Ch]`
    /// (`0x18960`). While the timeout's 40h is shown (`0x18963`), asking for it
    /// again does nothing (`0x1898E`), a negative ask goes back to the one
    /// remembered in `[328h]` (`0x18993`), and any other is noted there for
    /// later (`0x18997`) and the ask withdrawn (`0x1899B`). Otherwise a negative
    /// ask withdraws (`0x18968`), and any other is shown (`0x1896C`) and
    /// remembered unless it is 40h (`0x18975`). The descriptor is `DS:0334h`
    /// plus the selector below 80h (`0x189B5`), else `[3B4h]` plus the selector
    /// less 80h (`0x1897F`); its flags go to `[31Eh]` (`0x189C5`), the selector
    /// 80h comes back to into `[32Ah]` (`0x189D0`), its first cel into `[32Ch]`
    /// (`0x189E3`), and the phase starts from nothing (`[320h]`, `0x189ED`).
    fn select(&mut self) {
        let mut bx = self.request;
        let ax = self.current;
        if ax == 0x40 {
            if bx == 0x40 {
                return;
            }
            if bx & 0x8000 == 0 {
                self.last = bx;
                self.request = ax;
                return;
            }
            bx = self.last;
            self.request = bx;
            self.current = bx;
        } else {
            if bx & 0x8000 != 0 {
                self.request = ax;
                return;
            }
            self.current = bx;
            if bx != 0x40 {
                self.last = bx;
            }
        }
        self.desc = if bx >= 0x80 {
            self.map.regions.wrapping_add(bx - 0x80)
        } else {
            self.map.slot.wrapping_add(bx)
        };
        let d = self.shown_descriptor();
        self.flags = d.flags;
        if d.flags & 0x80 == 0 {
            self.resume = self.current;
        }
        self.cel = d.base;
        self.left = 0;
        self.shown = 0;
        self.phase = 0;
    }

    /// `179Eh:12C7h`: the timeout reloaded, a timed-out pointer given
    /// back, the counters cleared.
    ///
    /// The countdown `[3C4h]` is reloaded from `[3C2h]` (`0x18CA8`–`0x18CAB`);
    /// when it had run out (`0x18CAF`), the shown selector is asked for again
    /// (`0x18CBB`) or, for the timeout's own, the ask withdrawn (`0x18CC0`) and
    /// taken (`179Eh:0F78h`, `0x18CC9`). Everything is marked changed
    /// (`[2F6h]`, `0x18CCD`), and the backlog `[318h]` and the step `[31Ah]`
    /// cleared (`0x18CD5`, `0x18CDB`).
    fn reset(&mut self) {
        self.timed_out_returns();
        self.dirty = 0xFFFF;
        self.backlog = 0;
        self.elapsed = 0;
    }

    /// The reload `0x17ABE` and `0x18CA8` share, and what follows a
    /// countdown that had reached zero: `179Eh:00C0h` (`0x17AA0`, called
    /// at `0x17AC9`) and its copy in line at `0x18CB3`–`0x18CC9`. The
    /// shown selector `[31Ch]` is asked for again (`0x17AA8`, `0x18CBB`),
    /// or, when it is the timeout's own, 40h, the ask is withdrawn and
    /// taken (`179Eh:0F78h`, `0x17AB8`, `0x18CC9`).
    fn timed_out_returns(&mut self) {
        let old = std::mem::replace(&mut self.timeout, self.reload);
        if old != 0 {
            return;
        }
        if self.current != 0x40 {
            self.request = self.current;
        } else {
            self.request = 0xFFFF;
            self.dirty = 0xFFFF;
            self.select();
        }
    }

    /// `179Eh:00DBh`, inside `179Eh:01B0h`: the input polled. Only the
    /// timeout's part is the pointer's; the event record is not built.
    ///
    /// The countdown `[3C4h]` is reloaded from `[3C2h]` (`0x17ABE`–`0x17AC1`)
    /// and, when it had run out (`0x17AC5`), `179Eh:00C0h` gives a
    /// timed-out pointer back (`0x17AC9`). On the timeout's own pointer
    /// that routine also clears `[3D4h]` (`0x17AAE`), the buttons word the
    /// event record is built from (`0x17B24`).
    pub(crate) fn poll(&mut self) {
        self.timed_out_returns();
    }

    /// `179Eh:0A90h(n)`: the timer's call, `n` mouse units.
    ///
    /// `n` is added to the backlog `[318h]` (`0x18477`–`0x1847A`); a frozen
    /// pointer, `[316h]` negative (`0x18484`, `0x18488`), only counts, and
    /// otherwise `179Eh:0AB2h` steps with the zero flag saying whether it is
    /// drawn (`0x1848A`).
    pub(crate) fn callback(&mut self, n: u16, mouse: (i32, i32)) {
        self.backlog = self.backlog.wrapping_add(n);
        if self.state & 0x8000 != 0 {
            return;
        }
        self.update(self.state != 0, mouse);
    }

    /// `179Eh:0AB2h`: the units taken out of the backlog and stepped by.
    /// `draw` is the caller's zero flag inverted.
    ///
    /// The backlog `[318h]` is taken whole into `[31Ah]` (`0x18492`,
    /// `0x18496`). Not drawn (`0x18499`), it runs the movement and the shape and
    /// clears `[31Ah]`; drawn, it runs the movement (`179Eh:0AF9h`, `0x184A3`)
    /// and the shape (`179Eh:0C1Bh`, `0x184A6`), draws nothing when nothing
    /// changed (`0x184A9`), and after drawing clears what is pending, `[2F6h]`
    /// and `[31Ah]` (`0x184BF`–`0x184C4`).
    fn update(&mut self, draw: bool, mouse: (i32, i32)) {
        self.elapsed = std::mem::take(&mut self.backlog);
        let mut si = self.motion(mouse);
        self.shape(&mut si);
        if draw {
            if si != 0 {
                self.dirty = 0;
                self.elapsed = 0;
            }
        } else {
            self.elapsed = 0;
        }
    }

    /// `179Eh:0AF9h`, its effect: the position taken and clamped
    /// (`0x185CD`–`0x185E9` for x, bit 0; `0x1858C`–`0x185AA` for y, bit
    /// 1). The original takes the driver's mickeys; here the host's
    /// position stands in for them.
    fn motion(&mut self, mouse: (i32, i32)) -> u16 {
        let (x, y) = self.clamp(mouse.0, mouse.1);
        let mut si = self.dirty;
        if num::short(y) != self.pos.1 {
            self.pos.1 = num::short(y);
            si |= 2;
        }
        if num::short(x) != self.pos.0 {
            self.pos.0 = num::short(x);
            si |= 1;
        }
        self.dirty = si;
        si
    }

    /// `179Eh:0C1Bh`: the timeout, the regions, the animation, the place.
    /// `si` is the caller's change mask, and comes back with 4 when the
    /// cel changed.
    fn shape(&mut self, si: &mut u16) {
        // 0x185FB: the countdown `[3C4h]` loses the elapsed units and runs
        // out only on a borrow, strictly below zero (`0x185FE`). Unless the
        // timeout's selector 40h is already shown (`0x1860A`), it is asked
        // for (`0x18611`) and taken at once when its descriptor's flags
        // carry 20h (`0x1861D`).
        let (left, borrow) = self.timeout.overflowing_sub(self.elapsed);
        self.timeout = left;
        if borrow {
            self.timeout = 0;
            if self.current != 0x40 {
                self.request = 0x40;
                if self.descriptor(self.map.slot + 0x40).flags & 0x20 != 0 {
                    self.select();
                    *si |= 4;
                }
            }
        }
        // 0x1862E: the regions are asked only when the mouse moved.
        if *si & 3 != 0 {
            self.dirty = *si;
            self.regions();
            *si = self.dirty;
        }
        self.animate(si);
        // 0x18897: the place, when anything changed: the shown
        // descriptor's offset `+4`/`+6` plus the position `[3E6h]`/`[3E8h]`
        // (`0x1889F`–`0x188B3`).
        self.dirty = *si;
        if *si != 0 {
            let d = self.shown_descriptor();
            self.drawn = (
                d.offset.0.wrapping_add(self.pos.0),
                d.offset.1.wrapping_add(self.pos.1),
            );
        }
    }

    /// `0x18657`–`0x1872F`: the region under the mouse, and the shape it
    /// wants.
    ///
    /// A region holds the mouse when `x <= mouse x < x + w` (`+8`, `+0Ch`,
    /// `0x1866A`, `0x18672`) and the same for y (`+0Ah`, `0x1867A`); the same
    /// region as before (`[3FAh]`) changes nothing (`0x1868A`). Its shape word
    /// `+6` (`0x186A0`) gives the hover shape in its low byte and, while a
    /// button is down (`[3D5h]` bits 0–1, `0x186AC`), the high byte; shape `n`
    /// is the descriptor `[3B4h] + (n − 1) · 20h` (`0x186BD`), asked as
    /// selector `80h + (n − 1) · 20h` (`0x186C8`) and taken at once when the
    /// region's `+8` carries 20h (`0x186CE`). Under no region, a region's shape
    /// (`[31Ch]` 80h and up, `0x1870E`) goes back to selector 0 (`0x18716`).
    ///
    /// Only the menu's region has a shape word; the buttons' high byte
    /// equals its low byte (`0101h`), so the press path is not taken apart.
    fn regions(&mut self) {
        let m = self.menu;
        let (x, y) = (num::unsigned(self.pos.0), num::unsigned(self.pos.1));
        let inside = |v: u16, lo: u16, len: u16| lo <= v && v < lo.wrapping_add(len);
        let hit = if self.menu_registered
            && m.flags & 1 != 0
            && m.flags & 2 == 0
            && inside(x, m.rect.0, m.rect.2)
            && inside(y, m.rect.1, m.rect.3)
        {
            Hit::Menu
        } else {
            Hit::Elsewhere
        };
        if hit == Hit::Menu && self.hit == Hit::Menu {
            return;
        }
        self.hit = hit;
        let shape = if hit == Hit::Menu {
            num::low8(m.shape)
        } else {
            0
        };
        if shape != 0 {
            let off = (u16::from(shape) - 1) << 5;
            self.request = off + 0x80;
            if self.descriptor(self.map.regions + off).flags & 0x20 != 0 {
                self.select();
                self.dirty |= 4;
            }
            return;
        }
        // 0x1870E.
        if self.current >= 0x80 {
            self.request = 0;
            if self.descriptor(self.map.slot).flags & 0x20 != 0 {
                self.select();
                self.dirty |= 4;
            }
        }
    }

    /// The duration function `cs:[0EF6h + (flags & 0Ch) >> 1]` for phase
    /// `p` (`0x18802`, `0x18847`).
    ///
    /// Four functions: `0EFEh` answers `+0Ch` (`0x188DE`); `0F03h` `+10h` for
    /// phase 0, else `+0Ch` (`0x188E3`); `0F0Ch` `+0Ch` for phase 0, else the
    /// byte at `+10h + phase − 1` (`0x188EC`, `0x188F1`); `0F18h` `+0Ch` plus a
    /// new seed modulo `+10h` (`0x188FA`).
    fn duration(&mut self, flags: u16, p: u16, d: &Descriptor) -> u16 {
        match (flags & 0x0C) >> 2 {
            0 => d.duration,
            1 => {
                if p != 0 {
                    d.duration
                } else {
                    d.word_10()
                }
            }
            2 => match p.checked_sub(1) {
                Some(k) if num::signed(k) >= 0 => u16::from(d.tail[usize::from(k) & 0x0F]),
                _ => d.duration,
            },
            _ => {
                let r = self.next_seed();
                // `div word es:[bx+10h]` with dx = 0: the remainder.
                let m = d.word_10();
                (if m == 0 { 0 } else { r % m }).wrapping_add(d.duration)
            }
        }
    }

    /// `0x188FA`, `0x18930`: `seed = lo16(seed * 3E5h) + 3F1h + hi16`,
    /// with the carry of the first add.
    ///
    /// The seed is `[1864h]`, multiplied by `3E5h` (`0x188FD`) and stored back
    /// after the add of `3F1h` and the high word with carry (`0x18901`–`0x18906`).
    pub(crate) fn next_seed(&mut self) -> u16 {
        self.seed = next_seed(self.seed);
        self.seed
    }

    /// The phase mapper `cs:[0F37h + (flags & 3) * 2]` (`0x18855`).
    ///
    /// Four mappers: `0F70h` keeps the phase as counted (`0x18950`); `0F71h`
    /// counts backwards, `count − 1 − phase` (`0x18951`, `0x18955`); `0F3Fh`
    /// counts up to half the count `+0Eh` and back down as `count − phase`
    /// (`0x1891F`, `0x1892B`); `0F4Fh` draws a new seed (`0x18930`) and, when
    /// its low byte's parity is even (`0x1893F`, `0x18942`), keeps the phase,
    /// else steps two back (`0x18944`), wrapping below 0 to the last phase
    /// (`0x18948`) and putting the counter `[324h]` there too (`0x1894D`).
    fn map_phase(&mut self, flags: u16, p: u16, d: &Descriptor) -> u16 {
        match flags & 3 {
            0 => p,
            1 => !p.wrapping_sub(d.phases),
            2 => {
                if p <= d.phases >> 1 {
                    p
                } else {
                    d.phases.wrapping_sub(p)
                }
            }
            _ => {
                let s = self.next_seed();
                if num::low8(s).count_ones().is_multiple_of(2) {
                    return p;
                }
                let back = p.wrapping_sub(2);
                let q = if back & 0x8000 == 0 {
                    back
                } else {
                    d.phases.wrapping_sub(1)
                };
                self.phase = q;
                q
            }
        }
    }

    /// `0x187C6`–`0x18895`: the animation stepped by `[31Ah]` units.
    ///
    /// A static descriptor (flags bit 8, `0x187CD`) only takes a new selector
    /// that is asked for (`0x187D6`). Otherwise the time `[320h]` loses the
    /// units (`0x1880C`) and, strictly below 0, steps the phase `[324h]`
    /// (`0x18822`); past the last (`+0Eh`, `0x18826`) a selector asked for is
    /// taken (`0x1882C`), a one-shot (flags bit 4) ends (`0x18837`), and any
    /// other goes round to 0 (`0x1883D`). Each step adds its duration
    /// (`0x1884C`) until the time is no longer below 0 (`0x18850`); then the
    /// phase is mapped once (`0x18855`) and added to the first cel `+0Ah`
    /// (`0x1885A`).
    fn animate(&mut self, si: &mut u16) {
        let cx = self.flags;
        let ax = self.elapsed;
        if cx & 0x100 != 0 {
            if self.current == self.request {
                return;
            }
            self.take_new(si);
            return;
        }
        let (left, borrow) = self.left.overflowing_sub(ax);
        self.left = left;
        if !borrow {
            return;
        }
        let d = self.shown_descriptor();
        loop {
            let mut p = self.phase.wrapping_add(1);
            if p >= d.phases {
                if self.request != self.current {
                    self.take_new(si);
                    return;
                }
                p = 0;
                if cx & 0x10 != 0 {
                    // 0x18880: the one-shot over.
                    self.phase = 0;
                    let cx = cx | 0x100;
                    self.flags = cx;
                    if cx & 0x80 != 0 {
                        self.request = self.resume;
                        self.take_new(si);
                        return;
                    }
                    if cx & 0x200 != 0 {
                        self.show_phase(si);
                    }
                    return;
                }
            }
            self.phase = p;
            let add = self.duration(cx, p, &d);
            let (left, carry) = self.left.overflowing_add(add);
            self.left = left;
            if carry {
                break;
            }
        }
        self.show_phase(si);
    }

    /// `0x187DF`: the selector asked for taken, its first duration and its
    /// first phase.
    ///
    /// The shape is marked changed (`si` bit 2, `0x187DF`), the ask taken
    /// (`179Eh:0F78h`, `0x187E2`), its flags read (`[31Eh]`, `0x187E5`), and
    /// phase 0's duration (`0x187F9`, `0x18802`) put into `[320h]` (`0x18807`).
    fn take_new(&mut self, si: &mut u16) {
        *si |= 4;
        self.select();
        let cx = self.flags;
        let d = self.shown_descriptor();
        self.left = self.duration(cx, 0, &d);
        self.show_phase(si);
    }

    /// `0x18852`: the counted phase mapped and its cel taken.
    fn show_phase(&mut self, si: &mut u16) {
        let d = self.shown_descriptor();
        let cx = self.flags;
        let p = self.phase;
        let shown = self.map_phase(cx, p, &d);
        self.shown = shown;
        self.cel = shown.wrapping_add(d.base);
        *si |= 4;
    }

    /// `179Eh:0882h`: off the screen, and frozen.
    ///
    /// A drawn pointer (`[316h]`, `0x18265`) gets its background back
    /// (`0x18293`); the drawn bit is cleared and the frozen bit 15 set
    /// (`0x18299`, `0x1829C`).
    pub(crate) fn hide(&mut self) {
        self.state = (self.state & !1) | 0x8000;
    }

    /// `179Eh:091Ah`: frozen no more; the backlog stepped by, or dropped
    /// when it reaches the timeout.
    ///
    /// A backlog `[318h]` (`0x18305`) that is not below the countdown `[3C4h]`,
    /// signed (`0x1830D`, `0x18311`), is dropped by `179Eh:12C7h` (`0x18313`);
    /// otherwise it is stepped by with `179Eh:0AB2h` (`0x1831D`), the zero flag
    /// saying whether it is drawn (`0x18318`). The frozen bit comes down
    /// (`0x18326`).
    pub(crate) fn unfreeze(&mut self, mouse: (i32, i32)) {
        self.catch_up(false, mouse);
        self.state &= 0x7FFF;
    }

    /// `179Eh:0830h`: on the screen and not frozen. The backlog is
    /// dropped when twice it reaches the timeout (`0x18224`, `shl bx, 1`).
    ///
    /// The backlog `[318h]` (`0x1821C`), doubled (`0x18224`), is compared with
    /// the countdown `[3C4h]` (`0x18226`) and dropped by `179Eh:12C7h`
    /// (`0x1822C`), or stepped by with `179Eh:0AB2h` (`0x18256`); a pointer not
    /// drawn is drawn now (`0x18268`) and its drawn bit set (`0x18274`).
    pub(crate) fn show(&mut self, mouse: (i32, i32)) {
        self.catch_up(true, mouse);
        self.state = (self.state & 0x7FFF) | 1;
    }

    fn catch_up(&mut self, doubled: bool, mouse: (i32, i32)) {
        if self.state & 0x8000 == 0 || self.backlog == 0 {
            return;
        }
        let b = if doubled {
            self.backlog << 1
        } else {
            self.backlog
        };
        if num::signed(b) >= num::signed(self.timeout) {
            self.reset();
        }
        self.update(self.state & 1 != 0, mouse);
    }

    /// `179Eh:1017h`: the shape taken again — the shown selector asked for
    /// anew (the timeout's withdrawn), then `12C7h`.
    ///
    /// Unless the timeout's 40h is shown (`[31Ch]`, `0x18A04`–`0x18A07`), the
    /// shown selector is withdrawn — `FFFFh` into `[2F6h]` and the ask `[326h]`
    /// (`0x18A0C`–`0x18A12`) — and taken (`179Eh:0F78h`, `0x18A15`), then
    /// `179Eh:12C7h` runs (`0x18A18`).
    pub(crate) fn resync(&mut self) {
        let mut ax = self.current;
        if ax == 0x40 {
            ax = 0xFFFF;
            self.dirty = 0xFFFF;
        }
        self.request = ax;
        self.select();
        self.reset();
    }

    /// `0590h:064Dh(mode)`: the mode noted, and modes 1–4 copied into the
    /// slot by `179Eh:0960h`. Answers the mode before.
    ///
    /// The old mode is read from `[1962h]` and the new one stored there
    /// (`0x05F51`, `0x05F5A`); only modes 1 to 4 (`0x05F60`) go on through a
    /// jump table (`0x05F67`) to `179Eh:0960h(source, 334h)` (`0x05F74`).
    pub(crate) fn set_mode(&mut self, mode: u16) -> u16 {
        let old = std::mem::replace(&mut self.mode, mode);
        if let Some(&src) = mode
            .checked_sub(1)
            .and_then(|k| self.map.modes.get(usize::from(k)))
        {
            self.copy(src, self.map.slot);
        }
        old
    }

    /// `179Eh:0960h(src, dst)`: a descriptor copied over another; when it
    /// is the one shown, the shape is asked again from nothing.
    ///
    /// It runs `179Eh:12C7h` (`0x18346`), marks the pointer not drawn
    /// (`[316h] = FFFFh`, `0x1834C`) and copies the 20h bytes (`0x18380`).
    /// When the target is the shown descriptor (`[330h]`, `0x18390`), no
    /// acceleration is left (`[3DEh]`) and the shape is worked out with no time
    /// (`si = 7`, `179Eh:0C1Bh`, `0x18386`–`0x18389`); then the ask is taken
    /// (`179Eh:0F78h`, `0x1838C`).
    fn copy(&mut self, src: u16, dst: u16) {
        self.reset();
        let saved = self.state;
        self.state = 0xFFFF;
        let d = self.descriptor(src);
        if let Some((_, slot)) = self.descriptors.iter_mut().find(|(a, _)| *a == dst) {
            *slot = d;
        }
        if dst == self.desc {
            let mut si = 7;
            self.shape(&mut si);
        }
        self.select();
        self.state = saved;
    }

    /// `179Eh:0000h(x, y)`: the mouse put at `(x, y)`, inside the limits.
    ///
    /// The timeout and counters are reset (`179Eh:12C7h`, `0x179E6`) and x and
    /// y held in the limits (`[30Ah]`, `0x179EF`). Not drawn (`[316h]` bit 0,
    /// `0x17A0F`–`0x17A12`), the place is taken (`[3E6h]`, `0x17A7C`) and the
    /// shape asked as after a move (`si = 3`, `0x17A90`); drawn, a different
    /// place (`0x17A28`) runs the same and draws there (`0x17A5B`), leaving
    /// nothing pending (`[2F6h] = 0`, `0x17A70`).
    ///
    /// Answers the position as the library holds it.
    pub(crate) fn set_position(&mut self, x: i32, y: i32) -> (i32, i32) {
        self.reset();
        let (x, y) = self.clamp(x, y);
        let to = (num::short(x), num::short(y));
        if self.state & 1 == 0 {
            self.pos = to;
            let mut si = self.dirty | 3;
            self.shape(&mut si);
        } else if self.pos != to {
            self.pos = to;
            let mut si = self.dirty | 3;
            self.shape(&mut si);
            self.dirty = 0;
        }
        (x, y)
    }

    /// `179Eh:0210h` for the menu's region (`0x0DC91`): put first in the
    /// list and the regions asked again on the next call
    /// (`0x17C25`, `mov word [2F6h], 0FFFFh`).
    pub(crate) fn menu_region_add(&mut self) {
        self.menu_registered = true;
        self.dirty = 0xFFFF;
    }

    /// `179Eh:0730h` for the menu's region (`0x0D55F`, `0x0DED4`):
    /// out of the list, and the regions asked again (`0x1811D`).
    pub(crate) fn menu_region_remove(&mut self) {
        self.menu_registered = false;
        self.dirty = 0xFFFF;
    }

    /// The timer: `hundredths` increments of the scheduler's counter in one
    /// interrupt, and the callback when its slot runs out.
    pub(crate) fn timer(&mut self, hundredths: u16, mouse: (i32, i32)) {
        if let Some(n) = self.slot.advance(hundredths) {
            self.callback(n, mouse);
        }
    }

    /// The selector a test pins, and the address of the descriptor shown.
    #[cfg(test)]
    pub(crate) fn shown_descriptor_for_test(&self) -> (u16, Descriptor) {
        (self.desc, self.shown_descriptor())
    }
}

/// `0x188FA`–`0x18906`, without the store.
pub(crate) fn next_seed(seed: u16) -> u16 {
    let p = u32::from(seed) * 0x3E5;
    let (lo, hi) = (num::low_word(p), num::high_word(p));
    let (sum, carry) = lo.overflowing_add(0x3F1);
    sum.wrapping_add(hi).wrapping_add(u16::from(carry))
}

#[cfg(test)]
mod tests {
    //! The numbers pinned here are measured: they are what the original
    //! routines of `XP01.EXE` — `179Eh:0A90h`, `00DBh`, `0882h`, `091Ah`,
    //! `0830h`, `1017h`, `0210h`, `0730h` and `0590h:064Dh` — give, run
    //! under an 8086 emulator on the state the loader leaves, with fifteen
    //! 16×14 cels in the table and the same inputs as below. The host
    //! mouse is written into `[3E6h]`/`[3E8h]` with bits 1/2 of `[2F6h]`
    //! where `0AF9h` would apply mickeys, as [`Cursor::motion`] does.
    use super::*;

    /// The DGROUP words and the menu region as `XP01.EXE`'s image ships
    /// them; [`fixture_is_the_shipped_image`] checks them against an
    /// install.
    fn fixture() -> (Vec<u8>, Vec<u8>) {
        let mut ds = vec![0u8; 0x10000];
        let mut put = |at: u16, ws: &[u16]| {
            for (k, w) in ws.iter().enumerate() {
                let a = usize::from(at) + 2 * k;
                ds[a..a + 2].copy_from_slice(&w.to_le_bytes());
            }
        };
        put(0x0234, &[0, 0, 0, 0, 0x0120, 0, 1, 1]);
        put(0x0254, &[0, 0, 0, 0, 0x0120, 1, 1, 1]);
        put(0x0274, &[0, 0, 0xFFFB, 0xFFFF, 0x0120, 3, 1, 1]);
        put(0x0294, &[0, 0, 0xFFFA, 0xFFFA, 0x0120, 4, 1, 1]);
        put(0x02B4, &[0, 0, 0, 0, 0x0120, 5, 1, 1]);
        put(0x02D4, &[0, 0, 0, 0, 0x002F, 6, 3, 8, 7]);
        put(0x0374, &[0, 0, 0, 0, 0x0120, 2, 1, 1]);
        put(0x02F6, &[0xFFFF]);
        put(0x030A, &[0, 0, 0x013A, 0x00C2]);
        put(0x0314, &[2]);
        put(0x03C2, &[0x14, 0x14]);
        put(0x03E6, &[0xA0, 0x64]);
        put(0x03EE, &[0xA0, 0x64]);
        put(0x1864, &[0x0457]);
        let mut image = vec![0u8; PointerMap::DS1.menu_region + 0x12];
        for (k, w) in [0u16, 0, 1, 0x0101, 0, 0x84, 0x140, 0x44]
            .iter()
            .enumerate()
        {
            let a = PointerMap::DS1.menu_region + 2 * k;
            image[a..a + 2].copy_from_slice(&w.to_le_bytes());
        }
        (ds, image)
    }

    fn cels() -> Vec<Option<Picture>> {
        (0..15)
            .map(|_| {
                Some(Picture {
                    width: 16,
                    height: 14,
                    pixels: vec![0xE0; 16 * 14],
                })
            })
            .collect()
    }

    fn loaded() -> Cursor {
        let (ds, image) = fixture();
        Cursor::load(PointerMap::DS1, &ds, &image, &cels())
    }

    fn loaded_with(patch: impl FnOnce(&mut Vec<u8>)) -> Cursor {
        let (mut ds, image) = fixture();
        patch(&mut ds);
        Cursor::load(PointerMap::DS1, &ds, &image, &cels())
    }

    /// `0x188FA` from the shipped seed `0457h`: the seeds and the
    /// durations `3 + seed % 7` the original's `0F18h` answered on
    /// descriptor `02D4h`, twelve calls.
    #[test]
    fn mouse_generator_matches_the_original() {
        let measured: [(u16, u16); 12] = [
            (0xEAD4, 3),
            (0x9327, 7),
            (0x1D11, 3),
            (0x3797, 3),
            (0x83DC, 5),
            (0x8DBE, 8),
            (0x0B0F, 6),
            (0x1587, 5),
            (0xDB07, 4),
            (0x0989, 8),
            (0x26A3, 3),
            (0x7D56, 8),
        ];
        let mut c = loaded();
        let d = c.descriptor(0x02D4);
        for (seed, dur) in measured {
            assert_eq!(c.duration(d.flags, 1, &d), dur);
            assert_eq!(c.seed, seed);
        }
    }

    /// The step counts the comparison feeds the callback with.
    fn delta(i: usize) -> u16 {
        match (i * 7) % 11 {
            0 => 1,
            k if k > 6 => 1,
            k => u16::try_from(k).unwrap(),
        }
    }

    /// Mode 4 set on an unfrozen pointer, then forty callbacks with the
    /// poll before each: phase, phase drawn, time left, seed and cel after
    /// each, as the original left `[324h]`, `[322h]`, `[320h]`, `[1864h]`
    /// and `[32Ch]`.
    #[test]
    fn mode_4_random_walk_matches_the_original() {
        #[rustfmt::skip]
        let measured: [(u16, u16, u16, u16, u16); 41] = [
            (0, 0, 0, 0x0457, 6), (1, 1, 2, 0x9327, 7), (0, 0, 2, 0x3797, 6),
            (0, 0, 1, 0x3797, 6), (1, 1, 0, 0x8DBE, 7), (2, 2, 4, 0x1587, 8),
            (2, 2, 3, 0x1587, 8), (1, 1, 2, 0x0989, 7), (1, 1, 1, 0x0989, 7),
            (1, 1, 0, 0x0989, 7), (1, 1, 7, 0x25C7, 7), (1, 1, 6, 0x25C7, 7),
            (1, 1, 5, 0x25C7, 7), (1, 1, 2, 0x25C7, 7), (1, 1, 1, 0x25C7, 7),
            (2, 2, 4, 0x4642, 8), (2, 2, 2, 0x4642, 8), (2, 2, 1, 0x4642, 8),
            (3, 3, 2, 0xE92B, 9), (3, 3, 1, 0xE92B, 9), (3, 3, 0, 0xE92B, 9),
            (2, 2, 1, 0xE1A1, 8), (2, 2, 0, 0xE1A1, 8), (3, 3, 5, 0x674E, 9),
            (3, 3, 2, 0x674E, 9), (3, 3, 1, 0x674E, 9), (4, 4, 3, 0xD995, 10),
            (4, 4, 1, 0xD995, 10), (4, 4, 0, 0xD995, 10), (6, 6, 1, 0xA542, 12),
            (6, 6, 0, 0xA542, 12), (7, 7, 5, 0x1118, 13), (7, 7, 1, 0x1118, 13),
            (7, 7, 0, 0x1118, 13), (7, 7, 3, 0xCE32, 13), (7, 7, 0, 0xCE32, 13),
            (0, 0, 2, 0x9174, 6), (7, 7, 3, 0x4F16, 13), (7, 7, 1, 0x4F16, 13),
            (7, 7, 0, 0x4F16, 13), (7, 7, 1, 0x43C7, 13),
        ];
        let mouse = (160, 100);
        let mut c = loaded();
        c.unfreeze(mouse);
        c.set_mode(4);
        let got = |c: &Cursor| (c.phase, c.shown, c.left, c.seed, c.cel);
        assert_eq!(got(&c), measured[0]);
        for (i, m) in measured.iter().enumerate().skip(1) {
            c.poll();
            c.callback(delta(i), mouse);
            assert_eq!(got(&c), *m, "callback {i}");
            assert_eq!(c.current, 0);
        }
    }

    /// FNV-1a over the step counter.
    struct Fnv(u64);
    impl Fnv {
        fn new() -> Self {
            Fnv(0xcbf2_9ce4_8422_2325)
        }
        fn word(&mut self, v: u16) {
            for b in v.to_le_bytes() {
                self.0 ^= u64::from(b);
                self.0 = self.0.wrapping_mul(0x100_0000_01b3);
            }
        }
    }

    /// The same run for 10 000 callbacks, hashed: the original's hash and
    /// last seed.
    #[test]
    fn ten_thousand_mode_4_callbacks_match_the_original() {
        let mouse = (160, 100);
        let mut c = loaded();
        c.unfreeze(mouse);
        c.set_mode(4);
        let mut h = Fnv::new();
        for i in 1..=10_000 {
            c.poll();
            c.callback(delta(i), mouse);
            for v in [c.phase, c.shown, c.left, c.seed, c.cel] {
                h.word(v);
            }
        }
        assert_eq!((h.0, c.seed), (0x187f_3a7a_5c3b_3955, 0x3989));
    }

    /// No poll: the countdown of 20 reaches zero on unit 20 and goes
    /// below it on unit 21, which shows selector 40h, cel 2 (resource 3).
    /// A poll gives back selector 0 at phase 0, which steps on the next
    /// unit.
    #[test]
    fn timeout_matches_the_original() {
        let mouse = (160, 100);
        let mut c = loaded();
        c.unfreeze(mouse);
        c.set_mode(4);
        for unit in 1..=24u16 {
            c.callback(1, mouse);
            if unit <= 20 {
                assert_eq!((c.timeout, c.current), (20 - unit, 0), "unit {unit}");
            } else {
                assert_eq!(
                    (c.timeout, c.current, c.request, c.last),
                    (0, 0x40, 0x40, 0)
                );
                assert_eq!(c.cel, 2);
            }
        }
        let measured_cels = [7, 7, 7, 6, 6, 6, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 7, 7, 7];
        let mut c2 = loaded();
        c2.unfreeze(mouse);
        c2.set_mode(4);
        for (k, &cel) in measured_cels.iter().enumerate() {
            c2.callback(1, mouse);
            assert_eq!(c2.cel, cel, "unit {}", k + 1);
        }
        c.poll();
        assert_eq!(
            (c.timeout, c.current, c.request, c.cel, c.phase),
            (20, 0, 0, 6, 0)
        );
        c.callback(1, mouse);
        assert_eq!(
            (c.timeout, c.current, c.cel, c.phase, c.left),
            (19, 0, 7, 1, 2)
        );
    }

    struct Xs(u32);
    impl Xs {
        fn next(&mut self) -> u32 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.0 = x;
            x
        }
    }

    fn words(c: &Cursor) -> [u16; 21] {
        [
            c.state,
            c.backlog,
            c.current,
            c.flags,
            c.left,
            c.shown,
            c.phase,
            c.request,
            c.last,
            c.resume,
            c.cel,
            c.desc,
            c.timeout,
            c.dirty,
            num::unsigned(c.pos.0),
            num::unsigned(c.pos.1),
            num::unsigned(c.drawn.0),
            num::unsigned(c.drawn.1),
            c.seed,
            c.mode,
            u16::from(c.menu_registered),
        ]
    }

    /// The operation stream the comparison ran: callbacks of 1–3 and of
    /// 1–19 units, polls, hide, unfreeze, show, resync, modes 1–4, the
    /// menu's region in and out, and the mouse moved inside the limits.
    fn ops(mut c: Cursor, n: usize) -> u64 {
        let mut mouse = (160, 100);
        let mut rng = Xs(0x1994_0001);
        let mut h = Fnv::new();
        for _ in 0..n {
            match rng.next() % 32 {
                0..=13 => {
                    let k = 1 + u16::try_from(rng.next() % 3).unwrap();
                    c.callback(k, mouse);
                }
                14 | 15 => {
                    let k = 1 + u16::try_from(rng.next() % 19).unwrap();
                    c.callback(k, mouse);
                }
                16..=18 => c.poll(),
                19 => c.hide(),
                20 => c.unfreeze(mouse),
                21 | 22 => c.show(mouse),
                23 => c.resync(),
                24..=26 => {
                    let k = 1 + u16::try_from(rng.next() % 4).unwrap();
                    c.set_mode(k);
                }
                27 => c.menu_region_add(),
                28 => c.menu_region_remove(),
                _ => {
                    let x = i32::try_from(rng.next() % 315).unwrap();
                    let y = if rng.next().is_multiple_of(2) {
                        i32::try_from(132 + rng.next() % 63).unwrap()
                    } else {
                        i32::try_from(rng.next() % 195).unwrap()
                    };
                    mouse = (x, y);
                }
            }
            for v in words(&c) {
                h.word(v);
            }
        }
        h.0
    }

    /// 20 000 operations on the game's descriptors, every state word hashed
    /// after each: the original's hash.
    #[test]
    fn twenty_thousand_operations_match_the_original() {
        assert_eq!(ops(loaded(), 20_000), 0x4dcb_97d0_eb03_3964);
    }

    /// The paths the game's descriptors do not take — every duration function
    /// with every phase mapper, the one-shot with and without its return
    /// and hold — run through mode 3 with its descriptor `02B4h` given
    /// those flags, `+0Ch` 2, seven phases and `+10h` bytes 5 1 3 2 4 6.
    #[test]
    fn every_duration_and_mapper_matches_the_original() {
        let measured: [(u16, u64); 20] = [
            (0x0000, 0xb9a4_a24c_1b1c_9ee3),
            (0x0001, 0x2521_27fd_55dc_7b4e),
            (0x0002, 0x111f_55fa_1b01_5ead),
            (0x0003, 0x45e2_ce67_962c_2b79),
            (0x0004, 0xee6d_08eb_1688_42ad),
            (0x0005, 0xce65_09b5_fec1_e9b4),
            (0x0006, 0xb7e3_e9a4_6b68_9987),
            (0x0007, 0x09bf_90fb_71fc_6f0e),
            (0x0008, 0xd988_ff7d_74e6_a73e),
            (0x0009, 0xd052_83a6_717e_3133),
            (0x000a, 0xac02_1aae_afca_7fa4),
            (0x000b, 0xb7d7_059d_b17a_a067),
            (0x000c, 0x84f3_91ef_f4c0_8d8d),
            (0x000d, 0xc903_ad08_9780_8094),
            (0x000e, 0x9060_8b37_ac8e_e147),
            (0x000f, 0x7c7f_7a52_699d_4dcc),
            (0x0010, 0x1bc3_2377_adec_d804),
            (0x0090, 0x2e73_e0b8_1112_8870),
            (0x0210, 0xb9bc_3780_55ac_7bda),
            (0x002f, 0x4451_4c87_6b48_8113),
        ];
        for (flags, hash) in measured {
            let c = loaded_with(|ds| {
                let mut put = |o: usize, w: u16| ds[o..o + 2].copy_from_slice(&w.to_le_bytes());
                put(0x2B4 + 8, flags);
                put(0x2B4 + 0x0C, 2);
                put(0x2B4 + 0x0E, 7);
                ds[0x2B4 + 0x10..0x2B4 + 0x16].copy_from_slice(&[5, 1, 3, 2, 4, 6]);
            });
            assert_eq!(ops(c, 20_000), hash, "flags {flags:#06x}");
        }
    }

    /// The same with region shape 1 (`0254h`) and the timeout's
    /// descriptor (`0374h`) given flags 100h, without 20h: a selector they
    /// ask for waits for the end of a cycle (`0x1882C`) or the static path
    /// (`0x187D6`) instead of being taken at once.
    #[test]
    fn a_selector_taken_late_matches_the_original() {
        let measured: [(u16, u64); 20] = [
            (0x0000, 0xd734_7827_fc87_fef9),
            (0x0001, 0x2f1d_741d_70a1_cce9),
            (0x0002, 0x430b_15cb_b70a_0acd),
            (0x0003, 0xccf3_5d30_9ca3_40dd),
            (0x0004, 0xd402_48ea_5e26_9291),
            (0x0005, 0x2a07_6bd5_9f19_7e6d),
            (0x0006, 0xf4f5_ca9e_4131_2d6f),
            (0x0007, 0x7bb2_bc21_1bdd_f8ad),
            (0x0008, 0x9c9e_d674_3031_fa87),
            (0x0009, 0xdd57_5b1f_d50c_92af),
            (0x000a, 0x5197_3121_cd45_559b),
            (0x000b, 0x2f4a_314b_7c3f_ec2b),
            (0x000c, 0x759a_6cff_2662_cb4a),
            (0x000d, 0xa55d_9031_b82e_ccce),
            (0x000e, 0x1297_46b0_3451_bb22),
            (0x000f, 0x5569_4377_699f_6a1b),
            (0x0010, 0xc595_4ab5_8ef3_c3e2),
            (0x0090, 0x7b88_d471_9164_5160),
            (0x0210, 0xab4f_5718_7a3c_84b8),
            (0x002f, 0x2279_8112_bfca_2f9f),
        ];
        for (flags, hash) in measured {
            let c = loaded_with(|ds| {
                let mut put = |o: usize, w: u16| ds[o..o + 2].copy_from_slice(&w.to_le_bytes());
                put(0x2B4 + 8, flags);
                put(0x2B4 + 0x0C, 2);
                put(0x2B4 + 0x0E, 7);
                put(0x254 + 8, 0x0100);
                put(0x374 + 8, 0x0100);
                ds[0x2B4 + 0x10..0x2B4 + 0x16].copy_from_slice(&[5, 1, 3, 2, 4, 6]);
            });
            assert_eq!(ops(c, 20_000), hash, "flags {flags:#06x}");
        }
    }

    /// Duration functions 1 and 3 with `+10h` = 3, so that their cycles
    /// reach the last phase (`0x18826`) as often as the shorter ones do;
    /// otherwise the stream above.
    #[test]
    fn short_durations_with_every_mapper_match_the_original() {
        let measured: [(u16, u64); 8] = [
            (0x0004, 0x9f79_d6a9_3308_be78),
            (0x0005, 0x7457_a400_e73d_7345),
            (0x0006, 0x5102_fb6c_ae32_2de8),
            (0x0007, 0xd097_08e2_11c6_cd9c),
            (0x000c, 0x6b4e_1d1c_4796_860a),
            (0x000d, 0x7f49_c585_58f7_ff9f),
            (0x000e, 0x7370_a0b2_f8c2_aba2),
            (0x000f, 0x3b0b_b053_b5b2_166c),
        ];
        for (flags, hash) in measured {
            let c = loaded_with(|ds| {
                let mut put = |o: usize, w: u16| ds[o..o + 2].copy_from_slice(&w.to_le_bytes());
                put(0x2B4 + 8, flags);
                put(0x2B4 + 0x0C, 2);
                put(0x2B4 + 0x0E, 7);
                put(0x2B4 + 0x10, 3);
            });
            assert_eq!(ops(c, 20_000), hash, "flags {flags:#06x}");
        }
    }

    /// A second stream with `179Eh:0000h(x, y)` in it, positions also
    /// outside the limits, the host mouse following what the library
    /// holds: the original's hash.
    #[test]
    fn setting_the_position_matches_the_original() {
        let mut c = loaded();
        let mut mouse = (160, 100);
        let mut rng = Xs(0x1994_0002);
        let mut h = Fnv::new();
        for _ in 0..20_000 {
            match rng.next() % 32 {
                0..=13 => {
                    let k = 1 + u16::try_from(rng.next() % 3).unwrap();
                    c.callback(k, mouse);
                }
                14..=17 => c.poll(),
                18 => c.hide(),
                19 => c.unfreeze(mouse),
                20 | 21 => c.show(mouse),
                22..=24 => {
                    let k = 1 + u16::try_from(rng.next() % 4).unwrap();
                    c.set_mode(k);
                }
                25 => c.menu_region_add(),
                26 => c.menu_region_remove(),
                27..=29 => {
                    let x = i32::try_from(rng.next() % 360).unwrap() - 20;
                    let y = i32::try_from(rng.next() % 240).unwrap() - 20;
                    mouse = c.set_position(x, y);
                }
                _ => {
                    mouse = (
                        i32::try_from(rng.next() % 315).unwrap(),
                        i32::try_from(rng.next() % 195).unwrap(),
                    );
                }
            }
            for v in words(&c) {
                h.word(v);
            }
        }
        assert_eq!(h.0, 0x5354_0506_a2ec_1068);
    }

    /// The timer's slot: registered with 2, the first call on the third
    /// hundredth and then every second, one unit each.
    #[test]
    fn the_first_callback_comes_on_the_third_hundredth() {
        let mut s = Slot {
            countdown: 2,
            pending: 0,
            reload: 2,
        };
        let calls: Vec<Option<u16>> = (0..7).map(|_| s.advance(1)).collect();
        assert_eq!(calls, [None, None, Some(1), None, Some(1), None, Some(1)]);
        // Calls the busy path deferred (`0x1A1D0`) go out as one count.
        s.pending = 3;
        assert_eq!(s.advance(0), Some(3));
        assert_eq!(s.advance(0), None);
    }

    /// The fixture is what the shipped image holds.
    #[test]
    fn fixture_is_the_shipped_image() {
        let Some(dir) = crate::titles::ds1::gamedata_or_skip() else {
            return;
        };
        let image = crate::exe::Image::open_any(&dir, "XP01.EXE").unwrap();
        let bytes = image.bytes();
        let at = image.at(0);
        let (ds, img) = fixture();
        for (lo, hi) in [
            (0x0234usize, 0x02F4usize),
            (0x02F6, 0x02F8),
            (0x030A, 0x0316),
            (0x0316, 0x032C),
            (0x0374, 0x0394),
            (0x03C2, 0x03C6),
            (0x03E6, 0x03EA),
            (0x03EE, 0x03F2),
            (0x1864, 0x1866),
        ] {
            assert_eq!(&bytes[at + lo..at + hi], &ds[lo..hi], "DS:{lo:04X}");
        }
        let r = PointerMap::DS1.menu_region;
        assert_eq!(&bytes[r..r + 0x10], &img[r..r + 0x10]);
    }
}
