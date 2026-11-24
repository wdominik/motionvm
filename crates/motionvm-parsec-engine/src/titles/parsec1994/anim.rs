//! The animator: segment `0883h`, which plays the `AN` blocks.
//!
//! Every frame pass hands the elapsed ticks to `0883h:0305h` after the
//! tasks and the walkers (`0x10202`). It walks the room's layers, and in
//! each the objects and then the chain the walker hangs in, and counts
//! the ticks off every live object whose state is not zero; one whose
//! countdown runs out takes a step (`0883h:012Ah`), and a step for a
//! state of 7 or 8 — an object bound to an entry — is the interpreter
//! `0883h:0642h` over that entry's byte stream. What the stream can say:
//!
//! | opcode | bytes | what |
//! |---|---|---|
//! | `00` | 1 | end: the animation asked for next, or idle |
//! | `01` | 1 | return: pop a frame and go where its opcode says |
//! | `02` | 1 | next: the same without popping a loop's frame |
//! | `03 tt tt` | 3 | push a frame that returns to `tt` |
//! | `04 tt tt` | 3 | call `tt`, skipping a `03` there |
//! | `05` … `09` | 1, 1, 2, n+2, 4n+2 | when the next animation may take over |
//! | `0A t` | 2 | the longest wait once one is asked for |
//! | `0B n tt tt` | 4 | loop `n` times, out at `tt` |
//! | `0C tt tt` | 3 | loop forever, out at `tt` |
//! | `0D tt tt` | 3 | jump |
//! | `0E n (tt tt)×n` | 2n+2 | random jump |
//! | `0F n (tt tt)×n` | 2n+2 | random call |
//! | `10 d`, `11 lo hi` | 2, 3 | the default and the random wait |
//! | `12`–`15` | 3 | move the object by, or to |
//! | `16 dx dy w h` | 9 | the hotspot follows the object |
//! | `17 cc cc` | 3 | report code `cc` |
//! | `18 m` | 2 | mirror off, on, toggled |
//! | `19`–`1C` | 3 | the scale set or moved |
//! | `80`–`85` | 2–4 | one frame, with its wait |
//! | `88`–`8D` | | a sequence forward |
//! | `90`–`95` | | a sequence forward and back |
//! | `98`–`9D` | | a random walk through a sequence |
//! | `A0`–`A5` | | one frame drawn from a list |
//!
//! The table of the first twenty-nine is in the data segment, `DS:0474h`
//! (`0x0902F`, `call word [bx+474h]` with no override), which is what
//! makes them findable; the classes above `80h` are a five-way match on
//! `op & F8h` (`0x08F8A`) and a jump on `op & 7` inside each.
//!
//! An entry's working state lives in the entry's own bytes in the
//! original; here it is [`AniRun`], one per entry of the room's block and
//! of the global one, indexed by [`Adventure::anis`].

use crate::num;
use std::sync::Arc;

use motionvm_parsec_formats::Animations;

use super::objects::{
    HIDDEN, LIVE, MIRROR, STATE_FIGURE_ANIM, STATE_HEAD, STATE_NPC_ANIM, STATE_PART, STATE_SINGLE,
};
use super::{Adventure, Event, Far, scaled};

/// What an entry is bound to: the record the interpreter reads its frame
/// into and counts its waits on (`+70h` of the entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Body {
    /// A room object, by its index in [`Adventure::objects`].
    Object(usize),
    /// The player's record, `DS:1693h`.
    Figure,
    /// A room's 30h-byte record, by its index in [`Adventure::movers`].
    /// No `AN` entry is ever bound to one (`14DDh:0007h` binds only the
    /// 32-byte objects, `0x14DD7`), so it is a body of the chains alone.
    Mover(usize),
}

/// The working state of one `AN` entry — the bytes from `+5` to `+73h`
/// as the animator writes them, named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AniRun {
    /// Whether the entry is the global block's: its `ANIS` is zeroed at
    /// load and its offsets count from `[1ED4h]` (`0x08DBC`).
    pub global: bool,
    /// The entry's offset in its block.
    pub at: usize,
    /// `+0`: the chain, 0 for none.
    pub chain: u16,
    /// `+2`: the object's id, or a figure's negated.
    pub id: i16,
    /// `+4`: how many animations.
    pub count: u8,
    /// `+5`: the one playing.
    pub current: u8,
    /// `+6`: the one asked for, plus one.
    pub next: u8,
    /// `+7`: the phase — 0 idle, 1 fetching, 2 after a frame, 3 forward
    /// through a sequence, 4 forward and 5 back, 6 wandering.
    pub phase: u8,
    /// `+8`: the sequence's mode, its opcode's low three bits.
    pub mode: u8,
    /// `+9`: the head's frame, `FFh` for hidden.
    pub frame: u8,
    /// `+0Ah`: where in the sequence.
    pub index: u8,
    /// `+0Bh`: steps left of a random walk.
    pub repeat: u8,
    /// `+0Ch`, `+0Dh`.
    pub delay_lo: u8,
    /// See [`Self::delay_lo`].
    pub delay_hi: u8,
    /// `+0Eh`.
    pub delay: u8,
    /// `+10h`: when the next animation may take over, bit 7 once a
    /// transition has.
    pub condition: u8,
    /// `+11h`.
    pub cap: u8,
    /// `+12h`: the condition's argument, as an offset into the block.
    pub cond_arg: usize,
    /// `+16h`.
    pub loops: u8,
    /// `+17h`.
    pub frames: u8,
    /// `+18h`: the offsets of the opcodes that pushed.
    pub stack: [u16; 32],
    /// `+58h`.
    pub counters: [u8; 16],
    /// `+68h`: the stream, as an offset.
    pub cursor: usize,
    /// `+6Ch`: the sequence being played.
    pub sequence: usize,
    /// `+70h`: the head.
    pub head: Option<Body>,
}

impl AniRun {
    /// An entry as loaded: its fixed part, and everything else zero.
    pub(crate) fn new(e: &motionvm_parsec_formats::ani::Entry, global: bool) -> Self {
        AniRun {
            global,
            at: e.at,
            chain: e.chain,
            id: e.id,
            count: e.count,
            current: 0,
            next: 0,
            phase: 0,
            mode: 0,
            frame: 0,
            index: 0,
            repeat: 0,
            delay_lo: e.delay_lo,
            delay_hi: e.delay_hi,
            delay: e.delay,
            condition: 0,
            cap: 0,
            cond_arg: 0,
            loops: 0,
            frames: 0,
            stack: [0; 32],
            counters: [0; 16],
            cursor: 0,
            sequence: 0,
            head: None,
        }
    }
}

/// The id the player's record carries at `+13h`, `0FFh` (`DS:1693h+13h`
/// in the image), which the global block's entry names negated.
pub(super) const PLAYER_ID: i16 = 0xFF;
/// The same id as the byte an object record carries it in (`+13h`).
pub(super) const PLAYER_OBJECT: u8 = 0xFF;

/// The wait an idle entry sets before it is looked at again (`0x092DB`,
/// `0x095F2`).
const IDLE_WAIT: i32 = 0xF0;

impl Adventure {
    /// The block an entry's offsets count in.
    pub(crate) fn anim_block(&self, global: bool) -> Option<Arc<Animations>> {
        if global {
            self.global_animations.clone()
        } else {
            self.room_animations.clone()
        }
    }

    /// The runs of the location just entered: the room block's entries
    /// in directory order, then the global block's — none bound yet.
    pub(super) fn rebuild_animations(&mut self) {
        self.room_animations = self.room().and_then(|r| r.animations.clone()).map(Arc::new);
        let mut anis = Vec::new();
        if let Some(block) = &self.room_animations {
            // A forest row's map renames the entries without a chain
            // (`0x82E5`).
            let renamed = self.bound_map.as_ref().map(|m| m.ani_ids.as_slice());
            anis.extend(block.entries().enumerate().map(|(j, e)| {
                let mut run = AniRun::new(&e, false);
                if let Some(id) = renamed.and_then(|r| r.get(j).copied().flatten()) {
                    run.id = id;
                }
                run
            }));
        }
        if let Some(block) = &self.global_animations {
            anis.extend(block.entries().map(|e| AniRun::new(&e, true)));
        }
        self.anis = anis;
        if let Some(w) = &mut self.walker {
            w.state = 0;
        }
    }

    /// `19D2h(lo, hi)`: a wait between the two, `lo` itself when they
    /// are not in order (`0x0A20E`).
    fn wait_between(&mut self, lo: i32, hi: i32) -> i32 {
        if lo >= hi {
            return lo;
        }
        lo + i32::from(self.rng.inclusive(0, num::small(hi - lo)))
    }

    /// `1EB5h:0028h(0, n - 1)` as the list opcodes roll it: an index into
    /// `n` entries.
    fn index_of(&mut self, n: u8) -> usize {
        usize::from(self.rng.inclusive(0, u16::from(n).saturating_sub(1)))
    }

    /// The record an entry plays into: its countdown and frame fields,
    /// through one accessor for both kinds of body.
    fn body_countdown(&self, body: Body) -> i32 {
        match body {
            Body::Object(i) => self.objects.get(i).map_or(0, |o| o.countdown),
            Body::Figure => self.walker.as_ref().map_or(0, |w| w.anim_acc),
            Body::Mover(i) => self.movers.get(i).map_or(0, |m| m.walker.anim_acc),
        }
    }

    fn set_body_countdown(&mut self, body: Body, v: i32) {
        match body {
            Body::Object(i) => {
                if let Some(o) = self.objects.get_mut(i) {
                    o.countdown = v;
                }
            }
            Body::Figure => {
                if let Some(w) = &mut self.walker {
                    w.anim_acc = v;
                }
            }
            Body::Mover(i) => {
                if let Some(m) = self.movers.get_mut(i) {
                    m.walker.anim_acc = v;
                }
            }
        }
    }

    fn body_flags(&self, body: Body) -> u8 {
        match body {
            Body::Object(i) => self.objects.get(i).map_or(0, |o| o.flags),
            // The walker record is live (`DS:1693h+2` is 1 in the image);
            // its mirror bit is the record's own.
            Body::Figure => {
                LIVE | if self.walker.as_ref().is_some_and(|w| w.mirror) {
                    MIRROR
                } else {
                    0
                }
            }
            Body::Mover(i) => self.movers.get(i).map_or(0, |m| m.flags()),
        }
    }

    fn set_body_mirror(&mut self, body: Body, on: bool) {
        match body {
            Body::Object(i) => {
                if let Some(o) = self.objects.get_mut(i) {
                    o.flags = (o.flags & !MIRROR) | if on { MIRROR } else { 0 };
                }
            }
            Body::Figure => {
                if let Some(w) = &mut self.walker {
                    w.mirror = on;
                }
            }
            Body::Mover(i) => {
                if let Some(m) = self.movers.get_mut(i) {
                    m.walker.mirror = on;
                }
            }
        }
    }

    fn body_id(&self, body: Body) -> u8 {
        match body {
            Body::Object(i) => self.objects.get(i).map_or(0, |o| o.id),
            Body::Figure => PLAYER_OBJECT,
            Body::Mover(i) => self.movers.get(i).map_or(0, |m| m.id),
        }
    }

    /// `0883h:0305h(delta)`: the animator's pass, once a frame.
    ///
    /// A live object (`+2` bit 0, `0x08B4D`) with a state (`+12h`,
    /// `0x08B57`) counts the ticks off its countdown `+16h` (`0x08B5E`) and
    /// steps when none are left (`0x08B62`); the chain's kind-8 records step
    /// every tick (`0x08B9D`, the compare with `0Bh`).
    pub(crate) fn animator_tick(&mut self, delta: i32) {
        let layers = self.room().map_or(0, |r| r.room.layers.len());
        for layer in 0..layers {
            for i in 0..self.objects.len() {
                let o = &self.objects[i];
                if usize::from(o.layer) != layer || o.flags & LIVE == 0 || o.state == 0 {
                    continue;
                }
                self.objects[i].countdown -= delta;
                if self.objects[i].countdown <= 0 {
                    self.object_step(Body::Object(i));
                }
            }
            // The layer's chain, from its head (`0x08BAC`–`0x08BF7`).
            let mut at = self.chain_head(layer);
            while let Some(body) = at {
                self.chain_member_tick(body, delta);
                at = self.chain_next(body);
            }
        }
    }

    /// One record of a chain in the animator's pass: one with a state, and
    /// live (`0x08BBC`–`0x08BCB`), counts the ticks off, and steps when
    /// they are gone — a kind-8 record (`0Bh`) every tick (`0x08B9D`).
    /// A walking record's step is the walk's
    /// ([`crate::titles::walker::Walker::animate_walk`]).
    fn chain_member_tick(&mut self, body: Body, delta: i32) {
        match body {
            Body::Figure => {
                let Some((state, acc, walking)) = self
                    .walker
                    .as_ref()
                    .map(|w| (w.state, w.anim_acc, w.walking))
                else {
                    return;
                };
                if state != 0 {
                    if let Some(w) = &mut self.walker {
                        w.anim_acc = acc - delta;
                    }
                    if state == STATE_NPC_ANIM || acc - delta <= 0 {
                        self.object_step(Body::Figure);
                    }
                } else if walking && let Some(w) = &mut self.walker {
                    for k in w.animate_walk(delta) {
                        self.effect_1949(k);
                    }
                }
            }
            Body::Mover(i) => {
                if let Some(m) = self.movers.get_mut(i)
                    && m.flags() & LIVE != 0
                {
                    for k in m.walker.animate_walk(delta) {
                        self.effect_1949(k);
                    }
                }
            }
            Body::Object(_) => {}
        }
    }

    /// `0883h:012Ah(obj)`: one step of a record whose countdown ran out,
    /// for the states the animator owns. The walk cycle's states 1, 2, 3
    /// and 6 are the walker's own and stepped there.
    ///
    /// By state: 7 runs the entry's stream, then its frame (`0x08A71`); 8 the
    /// same, then the chain (`0x08A82`); 9, a part, never steps — its countdown
    /// is set to `7FFFh` (`0x08A8A`); `0Ah` takes the mover's position
    /// (`0A9Fh:0B4Ch`, `0x08A98`) and then runs the stream (`0x08AAF`). The
    /// tail puts the interval back on the countdown (`0x08AF7`) and goes round
    /// again while none is left (`0x08B02`).
    fn object_step(&mut self, body: Body) {
        // The tail loops while the countdown stays at or below zero; every
        // arm the animator owns puts a wait on it or ends the animation
        // with one, so the loop is bounded in practice, and a bound here
        // keeps a malformed stream from spinning.
        for _ in 0..64 {
            let state = match body {
                Body::Object(i) => self.objects[i].state,
                Body::Figure => self.walker.as_ref().map_or(0, |w| w.state),
                // Its walk is [`Walker::animate_walk`]'s; nothing else
                // steps it.
                Body::Mover(_) => return,
            };
            let before = self.body_countdown(body);
            let interval = match state {
                STATE_SINGLE => {
                    self.run_stream_then_frame(body);
                    0
                }
                STATE_HEAD => {
                    self.run_stream(body);
                    if let Some(ai) = self.body_ani(body) {
                        self.apply_chain(ai);
                    }
                    0
                }
                STATE_PART => {
                    self.set_body_countdown(body, 0x7FFF);
                    0
                }
                STATE_FIGURE_ANIM => {
                    // `0A9Fh:0B4Ch` copies the mover's position into the
                    // record, which the walker keeps for itself here.
                    if self.body_countdown(body) > 0 {
                        0
                    } else {
                        self.run_stream_then_frame(body);
                        // `0x08AB8`: a stream that put no wait on the
                        // record leaves the walker's interval to do it.
                        if self.body_countdown(body) == before {
                            self.walker.as_ref().map_or(0, |w| w.gait.speed)
                        } else {
                            0
                        }
                    }
                }
                STATE_NPC_ANIM => {
                    self.run_stream_then_frame(body);
                    0
                }
                _ => return,
            };
            let left = self.body_countdown(body) + interval;
            self.set_body_countdown(body, left);
            if left > 0 {
                return;
            }
        }
    }

    fn body_ani(&self, body: Body) -> Option<usize> {
        match body {
            Body::Object(i) => self.objects.get(i)?.ani,
            Body::Figure => self.walker.as_ref()?.ani,
            Body::Mover(_) => None,
        }
    }

    /// `0883h:0CBBh(obj)`: the stream, then the entry's frame into the
    /// record — hidden for `FFh` (`0x0951E`), else `+9` and `+0Ah`
    /// (`0x09564`).
    fn run_stream_then_frame(&mut self, body: Body) {
        let Some(ai) = self.body_ani(body) else {
            return;
        };
        self.run_stream(body);
        let frame = self.anis[ai].frame;
        match body {
            Body::Object(i) => {
                let o = &mut self.objects[i];
                if frame == 0xFF {
                    o.flags |= HIDDEN;
                } else {
                    o.flags &= !HIDDEN;
                    o.frame = frame;
                    o.cel = o.cel_base.wrapping_add(u16::from(frame));
                }
            }
            Body::Figure => {
                if let Some(w) = &mut self.walker {
                    w.anim_frame = frame;
                    w.cel_held = None;
                }
            }
            Body::Mover(_) => {}
        }
    }

    /// `0883h:0642h(obj)`: the interpreter, from a queued animation's
    /// takeover to the opcodes, until one sets a frame and its wait.
    fn run_stream(&mut self, body: Body) {
        let Some(ai) = self.body_ani(body) else {
            return;
        };
        let Some(block) = self.anim_block(self.anis[ai].global) else {
            return;
        };
        // `0x08EB5`: a queued animation, and whether it may take over.
        if self.anis[ai].next != 0 {
            let r = self.anim_ready(ai, &block);
            if r > 0 {
                self.anim_start(ai, &block, num::low8(num::unsigned(r - 1)));
            } else if r < 0 {
                // `0x08EB5`–`0x08F08`: a transition out of the table.
                self.anis[ai].condition |= 0x80;
                let entry = self.anis[ai].cond_arg + usize::from(num::unsigned(!r)) * 4;
                let off = block.word(entry + 3).unwrap_or(0);
                self.anim_jump_or_end(ai, usize::from(off));
            }
        }
        loop {
            let go_on = match self.anis[ai].phase {
                0 => {
                    // `0x092A8`: idle, looked at again in two seconds.
                    let c = self.body_countdown(body) + IDLE_WAIT;
                    self.set_body_countdown(body, c);
                    false
                }
                1 => self.fetch(ai, &block, body),
                2 => {
                    self.anis[ai].phase = 1;
                    self.fetch(ai, &block, body)
                }
                3 => self.sequence_forward(ai, &block, body, false),
                4 => self.sequence_forward(ai, &block, body, true),
                5 => self.sequence_back(ai, &block, body),
                6 => self.sequence_wander(ai, &block, body),
                _ => false,
            };
            if !go_on {
                break;
            }
        }
        // `0x0943E`: with an animation queued, the wait is capped.
        if self.anis[ai].next != 0 && self.anim_ready(ai, &block) != 0 {
            let cap = i32::from(self.anis[ai].cap);
            if cap < self.body_countdown(body) {
                self.set_body_countdown(body, cap);
            }
        }
    }

    /// `0883h:0486h(ani)`: whether the animation asked for may take over
    /// — its number plus one when it may, a negative index into the
    /// transition table when that says where to go first, 0 for not yet.
    ///
    /// An idle entry (`+7`, `0x08CBD`) may take over at once; one that is
    /// fetching (state 1, `0x08CC7`) may not. Otherwise the condition `+10h`
    /// decides (`0x08CE0`): 2, on the frame `+12h` (`0x08D01`); 3, on a frame
    /// in the list (`0x08D3D`); 4, the transition table's animation or any
    /// (`0x08D7F`), answered as the complement of its index (`0x08D95`).
    fn anim_ready(&self, ai: usize, block: &Animations) -> i16 {
        let a = &self.anis[ai];
        let next = i16::from(a.next);
        match a.phase {
            0 => return next,
            1 => return 0,
            _ => {}
        }
        match a.condition {
            1 => next,
            2 => {
                if block.byte(a.cond_arg) == Some(a.frame) {
                    next
                } else {
                    0
                }
            }
            3 => {
                let n = usize::from(block.byte(a.cond_arg).unwrap_or(0));
                if (1..=n).any(|k| block.byte(a.cond_arg + k) == Some(a.frame)) {
                    next
                } else {
                    0
                }
            }
            4 => {
                let n = usize::from(block.byte(a.cond_arg).unwrap_or(0));
                for k in 0..n {
                    let e = a.cond_arg + k * 4;
                    if block.byte(e + 1) == Some(a.frame)
                        && matches!(block.byte(e + 2), Some(0xFF) | None)
                        || block.byte(e + 1) == Some(a.frame)
                            && block.byte(e + 2) == Some(a.current)
                    {
                        return !num::signed(num::word(k));
                    }
                }
                0
            }
            _ => 0,
        }
    }

    /// `0883h:0582h(ani, k)`: animation `k` started from its first
    /// opcode, the stack and the loops cleared (`0x08DF8`–`0x08E35`).
    /// Answers whether the entry holds it; when not, the entry ends what
    /// it was doing (`17FAh`).
    fn anim_start(&mut self, ai: usize, block: &Animations, k: u8) -> bool {
        let at = self.anis[ai].at;
        let count = self.anis[ai].count;
        let off = if k < count {
            block
                .word(at + motionvm_parsec_formats::ani::ENTRY_OFFSETS + usize::from(k) * 2)
                .unwrap_or(0)
        } else {
            0
        };
        if off == 0 {
            self.anis[ai].next = 0;
            self.anim_end(ai, block);
            return false;
        }
        let a = &mut self.anis[ai];
        a.phase = 1;
        a.current = k;
        a.cursor = usize::from(off);
        a.loops = 0;
        a.frames = 0;
        a.next = 0;
        a.condition &= 0x7F;
        true
    }

    /// `0883h:060Fh(ani, off)`: the stream moved to `off`, or ended when
    /// there is none.
    fn anim_jump_or_end(&mut self, ai: usize, off: usize) {
        if off == 0 {
            self.anis[ai].next = 0;
            if let Some(block) = self.anim_block(self.anis[ai].global) {
                self.anim_end(ai, &block);
            }
            return;
        }
        self.anis[ai].phase = 1;
        self.anis[ai].cursor = off;
    }

    /// `0883h:0D86h(ani)`: the animation over — the one asked for next
    /// starts, or the entry idles and its record is looked at again in
    /// two seconds (`0x095F2`).
    fn anim_end(&mut self, ai: usize, block: &Animations) {
        let next = self.anis[ai].next;
        if next != 0 {
            self.anim_start(ai, block, next - 1);
            return;
        }
        let a = &mut self.anis[ai];
        a.loops = 0;
        a.frames = 0;
        a.condition &= 0x7F;
        a.phase = 0;
        if let Some(body) = a.head {
            self.set_body_countdown(body, IDLE_WAIT);
        }
    }

    /// `19F8h(ani)`: the stream's position pushed — the opcode's own
    /// offset — or refused at thirty-two (`0x0A238`).
    fn push_frame(&mut self, ai: usize) -> bool {
        let a = &mut self.anis[ai];
        let depth = usize::from(a.frames);
        if depth >= 0x20 {
            return false;
        }
        a.stack[depth] = num::word(a.cursor);
        a.frames += 1;
        true
    }

    /// `1822h(ani, frame, wait)` and `183Ch(ani, frame, lo, hi)`: the
    /// entry's frame set and the wait put on the record.
    fn set_frame(&mut self, ai: usize, frame: u8, wait: i32) {
        self.anis[ai].frame = frame;
        if let Some(body) = self.anis[ai].head {
            let c = self.body_countdown(body) + wait;
            self.set_body_countdown(body, c);
        }
    }

    /// `1955h(ani, phase)`: a sequence begun — the phase set, the mode
    /// taken off the opcode, and the data found past the opcode's own
    /// operands (`0x0A1BD`–`0x0A1E5`).
    fn sequence_begin(&mut self, ai: usize, block: &Animations, phase: u8) -> bool {
        let cursor = self.anis[ai].cursor;
        let mode = block.byte(cursor).unwrap_or(0) & 7;
        let a = &mut self.anis[ai];
        a.phase = phase;
        a.mode = mode;
        a.sequence = cursor
            + match mode {
                2 => 2,
                3 => 3,
                0 | 1 | 4 | 5 => 1,
                _ => {
                    self.anim_end(ai, block);
                    return false;
                }
            };
        true
    }

    /// `1862h(ani)`: the wait for the frame the sequence is on, put on
    /// the record, and the width of a sequence entry (`0x0A0B3`–`0x0A175`).
    fn sequence_wait(&mut self, ai: usize, block: &Animations) -> Option<usize> {
        let a = &self.anis[ai];
        let (seq, idx) = (a.sequence, usize::from(a.index));
        let (wait, stride) = match a.mode {
            0 => (i32::from(a.delay), 1),
            1 => (
                self.wait_between(i32::from(a.delay_lo), i32::from(a.delay_hi)),
                1,
            ),
            2 => (i32::from(block.byte(seq.wrapping_sub(1)).unwrap_or(0)), 1),
            3 => {
                let lo = i32::from(block.byte(seq.wrapping_sub(2)).unwrap_or(0));
                let hi = i32::from(block.byte(seq.wrapping_sub(1)).unwrap_or(0));
                (self.wait_between(lo, hi), 1)
            }
            4 => (i32::from(block.byte(seq + idx * 2 + 2).unwrap_or(0)), 2),
            5 => {
                let lo = i32::from(block.byte(seq + idx * 3 + 2).unwrap_or(0));
                let hi = i32::from(block.byte(seq + idx * 3 + 3).unwrap_or(0));
                (self.wait_between(lo, hi), 3)
            }
            _ => {
                self.anim_end(ai, block);
                return None;
            }
        };
        if let Some(body) = self.anis[ai].head {
            let c = self.body_countdown(body) + wait;
            self.set_body_countdown(body, c);
        }
        Some(stride)
    }

    /// Phases 3 and 4 (`0x092B4`, `0x09302`): the next frame of the
    /// sequence; at its end the stream goes on past it, or the way back
    /// begins.
    fn sequence_forward(&mut self, ai: usize, block: &Animations, body: Body, back: bool) -> bool {
        let _ = body;
        let count = block.byte(self.anis[ai].sequence).unwrap_or(0);
        let Some(stride) = self.sequence_wait(ai, block) else {
            return false;
        };
        let a = &mut self.anis[ai];
        let idx = usize::from(a.index);
        a.index = a.index.wrapping_add(1);
        a.frame = block.byte(a.sequence + idx * stride + 1).unwrap_or(0xFF);
        if a.index < count {
            return false;
        }
        if back {
            a.phase = 5;
        } else {
            self.sequence_over(ai, count, stride);
        }
        false
    }

    /// `0x09415`: the sequence over — phase 2, the stream past it.
    fn sequence_over(&mut self, ai: usize, count: u8, stride: usize) {
        let a = &mut self.anis[ai];
        a.phase = 2;
        a.cursor = a.sequence + usize::from(count) * stride + 1;
    }

    /// Phase 5 (`0x09355`): back down the sequence to its second frame.
    fn sequence_back(&mut self, ai: usize, block: &Animations, body: Body) -> bool {
        let _ = body;
        let count = block.byte(self.anis[ai].sequence).unwrap_or(0);
        let Some(stride) = self.sequence_wait(ai, block) else {
            return false;
        };
        let a = &mut self.anis[ai];
        a.index = a.index.wrapping_sub(1);
        let at = usize::from(a.index) * stride;
        a.frame = block.byte(a.sequence + at + 1).unwrap_or(0xFF);
        if at <= stride {
            self.sequence_over(ai, count, stride);
        }
        false
    }

    /// Phase 6 (`0x0939F`): a random walk — one step up or down the
    /// sequence, the step drawn as `19D2h(−15, 31) >> 4 | 1`
    /// (`0x093EB`), the frame shown the one stood on before the step,
    /// for as many steps as the repeat count.
    fn sequence_wander(&mut self, ai: usize, block: &Animations, body: Body) -> bool {
        let _ = body;
        let count = block.byte(self.anis[ai].sequence).unwrap_or(0);
        let Some(stride) = self.sequence_wait(ai, block) else {
            return false;
        };
        let roll = self.wait_between(-15, 31);
        let step = (roll >> 4) | 1;
        let a = &mut self.anis[ai];
        let old = i32::from(a.index);
        let mut next = old + step;
        if next < 0 {
            next = i32::from(count) - 1;
        } else if next >= i32::from(count) {
            next = 0;
        }
        a.index = num::low8(num::low16(next));
        a.frame = block
            .byte(a.sequence + num::index(old) * stride + 1)
            .unwrap_or(0xFF);
        let left = a.repeat;
        a.repeat = a.repeat.wrapping_sub(1);
        if left <= 1 {
            self.sequence_over(ai, count, stride);
        }
        false
    }

    /// Phase 1 (`0x08F40`): one opcode. Answers whether to fetch another
    /// at once.
    fn fetch(&mut self, ai: usize, block: &Animations, body: Body) -> bool {
        let c = self.anis[ai].cursor;
        let Some(op) = block.byte(c) else {
            self.anim_end(ai, block);
            return false;
        };
        if op < 0x1D {
            return self.control(ai, block, body, op);
        }
        match op & 0xF8 {
            0x80 => self.single_frame(ai, block, op & 7),
            0x88 => {
                if self.sequence_begin(ai, block, 3) {
                    self.anis[ai].index = 0;
                    return true;
                }
                false
            }
            0x90 => {
                if self.sequence_begin(ai, block, 4) {
                    self.anis[ai].index = 0;
                    return true;
                }
                false
            }
            0x98 => {
                // `0x0908B`: the walk keeps its place when it has one,
                // and goes on for twice the sequence's length.
                if !self.sequence_begin(ai, block, 6) {
                    return false;
                }
                let n = block.byte(self.anis[ai].sequence).unwrap_or(0);
                let a = &mut self.anis[ai];
                if a.index >= n {
                    a.index = 0;
                }
                a.repeat = if n > 0x7F { 0xFF } else { n * 2 };
                true
            }
            0xA0 => self.listed_frame(ai, block, op & 7),
            _ => {
                self.anim_end(ai, block);
                false
            }
        }
    }

    /// `80h`–`85h` (`0x08FB6`–`0x0904F`): one frame and its wait, then
    /// phase 2.
    fn single_frame(&mut self, ai: usize, block: &Animations, mode: u8) -> bool {
        let c = self.anis[ai].cursor;
        let b = |k: usize| i32::from(block.byte(c + k).unwrap_or(0));
        let frame = block.byte(c + 1).unwrap_or(0xFF);
        let (wait, len) = match mode {
            0 => (i32::from(self.anis[ai].delay), 2),
            1 => {
                let (lo, hi) = (self.anis[ai].delay_lo, self.anis[ai].delay_hi);
                (self.wait_between(i32::from(lo), i32::from(hi)), 2)
            }
            2 | 4 => (b(2), 3),
            3 | 5 => (self.wait_between(b(2), b(3)), 4),
            _ => {
                self.anim_end(ai, block);
                return false;
            }
        };
        self.set_frame(ai, frame, wait);
        let a = &mut self.anis[ai];
        a.cursor += len;
        a.phase = 2;
        false
    }

    /// `A0h`–`A5h` (`0x090B7`–`0x0927B`): one frame drawn from a list,
    /// and its wait, then phase 2.
    fn listed_frame(&mut self, ai: usize, block: &Animations, mode: u8) -> bool {
        let c = self.anis[ai].cursor;
        let b = |k: usize| block.byte(c + k).unwrap_or(0);
        let (frame, wait, len) = match mode {
            0 => {
                let n = b(1);
                let r = self.index_of(n);
                (b(2 + r), i32::from(self.anis[ai].delay), usize::from(n) + 2)
            }
            1 => {
                let n = b(1);
                let r = self.index_of(n);
                let (lo, hi) = (self.anis[ai].delay_lo, self.anis[ai].delay_hi);
                (
                    b(2 + r),
                    self.wait_between(i32::from(lo), i32::from(hi)),
                    usize::from(n) + 2,
                )
            }
            2 => {
                let n = b(2);
                let r = self.index_of(n);
                (b(3 + r), i32::from(b(1)), usize::from(n) + 3)
            }
            3 => {
                let n = b(3);
                let r = self.index_of(n);
                (
                    b(4 + r),
                    self.wait_between(i32::from(b(1)), i32::from(b(2))),
                    usize::from(n) + 4,
                )
            }
            4 => {
                let n = b(1);
                let r = self.index_of(n) * 2;
                (b(2 + r), i32::from(b(3 + r)), usize::from(n) * 2 + 2)
            }
            5 => {
                let n = b(1);
                let r = self.index_of(n) * 3;
                (
                    b(2 + r),
                    self.wait_between(i32::from(b(3 + r)), i32::from(b(4 + r))),
                    usize::from(n) * 3 + 2,
                )
            }
            _ => {
                self.anim_end(ai, block);
                return false;
            }
        };
        self.set_frame(ai, frame, wait);
        let a = &mut self.anis[ai];
        a.cursor += len;
        a.phase = 2;
        false
    }

    /// The twenty-nine opcodes of the table at `DS:0474h`, in its order.
    /// Answers whether the interpreter fetches the next at once.
    fn control(&mut self, ai: usize, block: &Animations, body: Body, op: u8) -> bool {
        let c = self.anis[ai].cursor;
        let b = |k: usize| block.byte(c + k).unwrap_or(0);
        match op {
            0x00..=0x04 | 0x0B..=0x0F => self.control_flow(ai, block, op),
            0x12..=0x1C => self.control_body(ai, block, body, op),
            0x05 => {
                self.anis[ai].condition = 0;
                self.anis[ai].cursor += 1;
                true
            }
            0x06 => {
                self.anis[ai].condition = 1;
                self.anis[ai].cursor += 1;
                true
            }
            0x07 => {
                let a = &mut self.anis[ai];
                a.cond_arg = c + 1;
                a.condition = 2;
                a.cursor += 2;
                true
            }
            0x08 => {
                let a = &mut self.anis[ai];
                a.cond_arg = c + 1;
                a.condition = 3;
                a.cursor += usize::from(b(1)) + 2;
                true
            }
            0x09 => {
                let a = &mut self.anis[ai];
                a.cond_arg = c + 1;
                a.condition = 4;
                a.cursor += usize::from(b(1)) * 4 + 2;
                true
            }
            0x0A => {
                self.anis[ai].cap = b(1);
                self.anis[ai].cursor += 2;
                true
            }
            0x10 => {
                self.anis[ai].delay = b(1);
                self.anis[ai].cursor += 2;
                true
            }
            0x11 => {
                let a = &mut self.anis[ai];
                a.delay_lo = b(1);
                a.delay_hi = b(2);
                a.cursor += 3;
                true
            }
            _ => {
                self.anim_end(ai, block);
                false
            }
        }
    }

    /// The opcodes that move the cursor through the stream: the end, the
    /// returns, the calls, the loops and the jumps.
    fn control_flow(&mut self, ai: usize, block: &Animations, op: u8) -> bool {
        let c = self.anis[ai].cursor;
        let b = |k: usize| block.byte(c + k).unwrap_or(0);
        let w = |k: usize| usize::from(block.word(c + k).unwrap_or(0));
        match op {
            // `0D86h`.
            0x00 => {
                self.anim_end(ai, block);
                false
            }
            // `0DCCh`: return.
            0x01 => self.unwind(ai, block, false),
            // `0EB5h`: next.
            0x02 => self.unwind(ai, block, true),
            // `0FE4h`: a frame that returns to `tt`; with the stack full,
            // straight there.
            0x03 => {
                if self.push_frame(ai) {
                    self.anis[ai].cursor += 3;
                } else {
                    self.anis[ai].cursor = w(1);
                }
                true
            }
            // `101Fh`: call, skipping a `03` at the callee.
            0x04 => {
                if self.push_frame(ai) {
                    let mut target = w(1);
                    if block.byte(target) == Some(3) {
                        target += 3;
                    }
                    self.anis[ai].cursor = target;
                } else {
                    self.anis[ai].cursor += 3;
                }
                true
            }
            // `1152h`: a counted loop.
            0x0B => {
                if !self.push_frame(ai) {
                    self.anis[ai].cursor = w(2);
                    return true;
                }
                let a = &mut self.anis[ai];
                if a.loops < 0x10 {
                    a.counters[usize::from(a.loops)] = b(1);
                    a.loops += 1;
                    a.cursor += 4;
                } else {
                    a.frames -= 1;
                    a.cursor = w(2);
                }
                true
            }
            // `11BBh`: a loop without a count.
            0x0C => {
                if self.push_frame(ai) {
                    self.anis[ai].cursor += 3;
                } else {
                    self.anis[ai].cursor = w(1);
                }
                true
            }
            0x0D => {
                self.anis[ai].cursor = w(1);
                true
            }
            0x0E => {
                let r = self.index_of(b(1));
                self.anis[ai].cursor = w(2 + r * 2);
                true
            }
            0x0F => {
                let n = b(1);
                if self.push_frame(ai) {
                    let r = self.index_of(n);
                    let mut target = w(2 + r * 2);
                    if block.byte(target) == Some(3) {
                        target += 3;
                    }
                    self.anis[ai].cursor = target;
                } else {
                    self.anis[ai].cursor += usize::from(n) * 2 + 2;
                }
                true
            }
            _ => {
                self.anim_end(ai, block);
                false
            }
        }
    }

    /// The opcodes that act on the body the entry drives: its position,
    /// its hotspot, its signal, its mirror and its scale.
    fn control_body(&mut self, ai: usize, block: &Animations, body: Body, op: u8) -> bool {
        let c = self.anis[ai].cursor;
        let b = |k: usize| block.byte(c + k).unwrap_or(0);
        let w = |k: usize| usize::from(block.word(c + k).unwrap_or(0));
        let i = |k: usize| i32::from(block.int(c + k).unwrap_or(0));
        match op {
            0x12..=0x15 => {
                let (x, y) = self.body_point(body);
                let to = match op {
                    0x12 => (x + i(1), y),
                    0x13 => (x, y + i(1)),
                    0x14 => (i(1), y),
                    _ => (x, i(1)),
                };
                self.move_body(ai, body, to);
                self.anis[ai].cursor += 3;
                true
            }
            0x16 => {
                if self.anis[ai].chain != 0 {
                    self.apply_chain(ai);
                }
                self.hotspot_follows_body(body, (i(1), i(3)), (i(5), i(7)));
                self.anis[ai].cursor += 9;
                true
            }
            0x17 => {
                let code = num::word(w(1));
                self.anim_signal(body, code);
                self.anis[ai].cursor += 3;
                true
            }
            0x18 => {
                let want = match b(1) {
                    0 => Some(false),
                    1 => Some(true),
                    2 => Some(self.body_flags(body) & MIRROR == 0),
                    _ => None,
                };
                if let Some(on) = want {
                    self.set_body_mirror(body, on);
                }
                self.anis[ai].cursor += 2;
                true
            }
            0x19..=0x1C => {
                if let Body::Object(oi) = body
                    && let Some(o) = self.objects.get_mut(oi)
                {
                    match op {
                        0x19 => o.scale.0 = num::word(w(1)),
                        0x1A => o.scale.1 = num::word(w(1)),
                        0x1B => o.scale.0 = o.scale.0.wrapping_add(num::low16(i(1))),
                        _ => o.scale.1 = o.scale.1.wrapping_add(num::low16(i(1))),
                    }
                }
                self.anis[ai].cursor += 3;
                true
            }
            _ => {
                self.anim_end(ai, block);
                false
            }
        }
    }

    /// `0DCCh` (return) and `0EB5h` (next): the top frame decides where
    /// the stream goes, by the opcode that pushed it.
    ///
    /// Both dispatch on the pushing opcode through a jump table: the return
    /// at `0x09610` (03 and 0C to the address in the word after the
    /// pushing opcode, 04 three bytes past that opcode, 0B out of the loop
    /// to the address at its `+2`, 0F past its table of words), the next at
    /// `0x0973A` (0B counts down and stays, 0C stays).
    ///
    /// A `next` on a counted loop whose count is not spent goes back into
    /// the body and keeps the frame (`0x0977B`); on a loop without a count
    /// it always does (`0x0977F`). Every other opcode a frame could carry
    /// ends the animation.
    fn unwind(&mut self, ai: usize, block: &Animations, next: bool) -> bool {
        let depth = self.anis[ai].frames;
        if depth == 0 {
            self.anim_end(ai, block);
            return false;
        }
        let top = usize::from(depth) - 1;
        let p = usize::from(self.anis[ai].stack[top]);
        let op = block.byte(p).unwrap_or(0);
        let w = |k: usize| usize::from(block.word(p + k).unwrap_or(0));
        let pop = |a: &mut AniRun| a.frames = num::byte(top);
        match op {
            0x03 | 0x0C if !next || op == 0x03 => {
                self.anis[ai].cursor = w(1);
                pop(&mut self.anis[ai]);
                true
            }
            0x04 => {
                self.anis[ai].cursor = p + 3;
                pop(&mut self.anis[ai]);
                true
            }
            0x0B if !next => {
                let a = &mut self.anis[ai];
                a.loops = a.loops.wrapping_sub(1);
                a.cursor = w(2);
                pop(a);
                true
            }
            0x0B => {
                let a = &mut self.anis[ai];
                let slot = usize::from(a.loops.wrapping_sub(1)) & 0xF;
                a.counters[slot] = a.counters[slot].wrapping_sub(1);
                if a.counters[slot] == 0 {
                    a.loops = a.loops.wrapping_sub(1);
                    a.cursor = w(2);
                    pop(a);
                } else {
                    a.cursor = p + 4;
                }
                true
            }
            0x0C => {
                self.anis[ai].cursor = p + 3;
                true
            }
            0x0F => {
                let n = usize::from(block.byte(p + 1).unwrap_or(0));
                self.anis[ai].cursor = p + 2 + n * 2;
                pop(&mut self.anis[ai]);
                true
            }
            _ => {
                self.anim_end(ai, block);
                false
            }
        }
    }

    fn body_point(&self, body: Body) -> (i32, i32) {
        match body {
            Body::Object(i) => self.objects.get(i).map_or((0, 0), |o| (o.x, o.y)),
            Body::Figure => self.walker.as_ref().map_or((0, 0), |w| (w.x, w.y)),
            Body::Mover(i) => self
                .movers
                .get(i)
                .map_or((0, 0), |m| (m.walker.x, m.walker.y)),
        }
    }

    /// `1A33h(ani, x, y)`: the head moved, and its chain with it
    /// (`0x0A283`–`0x0A2AD`).
    fn move_body(&mut self, ai: usize, body: Body, to: (i32, i32)) {
        match body {
            Body::Object(i) => {
                if let Some(o) = self.objects.get_mut(i) {
                    o.x = to.0;
                    o.y = to.1;
                }
            }
            Body::Figure => {
                if let Some(w) = &mut self.walker {
                    w.x = to.0;
                    w.y = to.1;
                }
            }
            Body::Mover(i) => {
                if let Some(m) = self.movers.get_mut(i) {
                    m.walker.x = to.0;
                    m.walker.y = to.1;
                }
            }
        }
        if self.anis[ai].chain != 0 {
            self.apply_chain(ai);
        }
    }

    /// The cel's box around an object's point as the blitter aligns it
    /// (`0x0BC4B`–`0x0BC77`, and the same arithmetic in `000Fh` and
    /// `1A82h`): the top-left corner and the scaled extent.
    fn object_box(&self, i: usize) -> Option<(i32, i32, i32, i32)> {
        let o = self.objects.get(i)?;
        let (w, h) = self.object_cel_size(o.bundle, o.cel)?;
        let w = scaled(w, o.scale.0);
        let h = scaled(h, o.scale.1);
        let (mut x, mut y) = (o.x, o.y);
        match o.align {
            1 => {
                x -= w >> 1;
                y -= h >> 1;
            }
            2 => {
                x -= w >> 1;
                y -= h;
            }
            _ => {}
        }
        Some((x, y, w, h))
    }

    /// `13F2h`'s tail (`0x09C68`–`0x09DE0`): the hotspot that names the
    /// object put on the object's box, moved by the offsets, sized by
    /// the extents where they are given and by the cel where they are 0.
    fn hotspot_follows_body(&mut self, body: Body, offset: (i32, i32), size: (i32, i32)) {
        let Body::Object(i) = body else {
            return;
        };
        let Some((x, y, w, h)) = self.object_box(i) else {
            return;
        };
        let key = u16::from(self.objects[i].id) | 0x8000;
        let Some(room) = self.room() else {
            return;
        };
        let spot = room
            .hotspots
            .spots
            .iter()
            .position(|hs| hs.verb_mask & 0x80FF == key);
        if let Some(s) = spot
            && let Some(hs) = self.hotspots_rt.get_mut(s)
        {
            hs.x = x + offset.0;
            hs.y = y + offset.1;
            hs.w = if size.0 == 0 { w } else { size.0 };
            hs.h = if size.1 == 0 { h } else { size.1 };
            // `0x09DE0`: the region moved, the pointer's shape again.
            self.pointer_resync();
        }
    }

    /// `0883h:1A82h(ani)`: the head's frame for this frame of the chain,
    /// and every part placed against it.
    ///
    /// The head's node decides its cel: kind 0 takes the node's cel for the
    /// frame (`0x0A3DC`, `0x0A3F1`), kind 100h (`0x0A3E4`) one whose kind-6
    /// frame is a position (`0x0A426`). Every node's form is its high byte
    /// (`0x0A5C9`) and picks the placement from a jump table (`0x0A6E0`), form
    /// 0 skipping the node; a part with a cel is shown there (`+2` bit 1
    /// cleared, `0x0A760`), one with `FFh` is hidden (bit 1 set, `0x0A783`).
    pub(crate) fn apply_chain(&mut self, ai: usize) {
        let Some(block) = self.anim_block(self.anis[ai].global) else {
            return;
        };
        let Some(Body::Object(head)) = self.anis[ai].head else {
            return;
        };
        let frame = self.anis[ai].frame;
        let chain = usize::from(self.anis[ai].chain);
        if chain == 0 {
            // `0x0A30A`.
            let o = &mut self.objects[head];
            if frame == 0xFF {
                o.flags |= HIDDEN;
            } else {
                o.frame = frame;
                o.cel = o.cel_base.wrapping_add(u16::from(frame));
            }
            return;
        }
        if frame == 0xFF {
            // `0x0A34B`: every part hidden.
            let mut at = chain;
            while let Some(node) = block.node(at) {
                if let Some(p) = self.object_index(num::low8(node.id)) {
                    self.objects[p].flags |= HIDDEN;
                }
                at = usize::from(node.next);
            }
            return;
        }
        let stack = self.place_chain_head(&block, head, chain, frame);
        self.place_chain_parts(&block, head, chain, frame, stack);
    }

    /// `0x0A3A3`: the head's cel, and a position when its frame says; the
    /// head placed, and the box its parts stack against.
    fn place_chain_head(
        &mut self,
        block: &Animations,
        head: usize,
        chain: usize,
        frame: u8,
    ) -> Stack {
        // `0x0A3A3`: the head's cel, and a position when its frame says.
        let (mut x, mut y) = (self.objects[head].x, self.objects[head].y);
        let mut cel = 0xFFu8;
        let mut at = chain;
        while let Some(node) = block.node(at) {
            match node.kind {
                0 => {
                    cel = block.node_cel(&node, frame).unwrap_or(0xFF);
                    break;
                }
                0x100 => {
                    let f = block.node_frame(&node, frame).unwrap_or([0xFF; 8]);
                    cel = f[1];
                    if f[0] == 6 {
                        x = i32::from(i16::from_le_bytes([f[2], f[3]]));
                        y = i32::from(i16::from_le_bytes([f[4], f[5]]));
                    }
                    break;
                }
                _ => at = usize::from(node.next),
            }
        }
        if cel < 0xFF {
            let o = &mut self.objects[head];
            o.flags &= !HIDDEN;
            o.x = x;
            o.y = y;
            o.frame = cel;
            o.cel = o.cel_base.wrapping_add(u16::from(cel));
            let (bx, by, _, bh) = self.object_box(head).unwrap_or((x, y, 0, 0));
            Stack {
                left: bx,
                top: by,
                top_of_head: by,
                bottom: by + bh,
                bottom_of_head: by + bh,
            }
        } else {
            // `0x0A552`: the head hidden; its point taken as the box.
            let o = &mut self.objects[head];
            o.flags |= HIDDEN;
            o.x = x;
            o.y = y;
            Stack {
                left: x,
                top: y,
                top_of_head: y,
                bottom: y,
                bottom_of_head: y,
            }
        }
    }

    /// `0x0A58E`: the parts, from the first node, each placed against the
    /// box the head and the parts before it make.
    fn place_chain_parts(
        &mut self,
        block: &Animations,
        head: usize,
        chain: usize,
        frame: u8,
        stack: Stack,
    ) {
        let head_mirror = self.objects[head].flags & MIRROR;
        let Stack {
            left,
            mut top,
            top_of_head,
            mut bottom,
            bottom_of_head,
        } = stack;
        let mut at = chain;
        while let Some(node) = block.node(at) {
            at = usize::from(node.next);
            let Some(p) = self.object_index(num::low8(node.id)) else {
                continue;
            };
            let (placement, part_cel, mut px, mut py, mirror) = match node.kind & 0xFF00 {
                0 => (
                    num::low8(node.kind),
                    block.node_cel(&node, frame).unwrap_or(0xFF),
                    i32::from(node.dx) + left,
                    i32::from(node.dy),
                    head_mirror != 0,
                ),
                0x100 => {
                    let f = block.node_frame(&node, frame).unwrap_or([0xFF; 8]);
                    (
                        f[0],
                        f[1],
                        i32::from(i16::from_le_bytes([f[2], f[3]])) + left,
                        i32::from(i16::from_le_bytes([f[4], f[5]])),
                        f[6] != 0,
                    )
                }
                // A node of another form carries no frame for the part;
                // the original falls through with stale locals, which
                // no shipped chain does.
                _ => continue,
            };
            let new_cel = if part_cel < 0xFF {
                let o = &mut self.objects[p];
                if (o.flags & MIRROR != 0) != mirror {
                    o.flags ^= MIRROR;
                }
                o.cel_base.wrapping_add(u16::from(part_cel))
            } else {
                self.objects[p].cel
            };
            let bundle = self.objects[p].bundle;
            let h = self.object_cel_size(bundle, new_cel).map_or(0, |(_, h)| h);
            match placement {
                0 => continue,
                1 => {
                    py += bottom;
                    bottom += h;
                }
                2 => {
                    py += bottom_of_head;
                    if py + h > bottom {
                        bottom = py + h;
                    }
                }
                3 => {
                    py += top - h;
                    top = py;
                }
                4 => {
                    py += top_of_head - h;
                    if top > py {
                        top = py;
                    }
                }
                5 => py += top_of_head,
                6 => px -= left,
                _ => {}
            }
            let o = &mut self.objects[p];
            if part_cel < 0xFF {
                o.flags &= !HIDDEN;
                o.cel = new_cel;
                o.x = px;
                o.y = py;
            } else {
                o.flags |= HIDDEN;
            }
        }
    }

    /// `0883h:20B3h(id)`: the object shown, and its animation 0 played
    /// (`1F82h(14DDh:084Ah(id, 0), 0)`, `0x0A8EC`–`0x0A8FB`).
    pub(crate) fn object_wake(&mut self, id: u8) {
        if self.object_show_all(id) {
            self.object_anim(id, 0);
        }
    }

    /// `0883h:1F82h(obj, k)`: animation `k` on an object — started when
    /// its entry is idle, asked for otherwise.
    ///
    /// Only states 7 to `0Bh` take it, and `0Ah` does nothing (`0x0A7BE`); a
    /// `k` past the block's count puts the object in state 0 (`0x0A7FE`). An
    /// object that is not live (`0x0A83C`) — a door — comes up hidden, `+2`
    /// bits 0 and 1 set (`0x0A84C`). An idle entry (`+7`, `0x0A862`) starts at
    /// once with its countdown at 0; otherwise the animation is asked for in
    /// `+6` (`0x0A889`) and the wait capped.
    pub(crate) fn object_anim(&mut self, id: u8, k: u8) {
        let Some(i) = self.object_index(id) else {
            return;
        };
        self.events.push(Event::ObjectAnim {
            object: id,
            anim: k,
        });
        let state = self.objects[i].state;
        if !matches!(
            state,
            STATE_SINGLE | STATE_HEAD | STATE_PART | STATE_NPC_ANIM
        ) {
            return;
        }
        // A build without the checks reads whatever the record points at;
        // here an object without a block, or a number past its count, is
        // still not started, but its state is left as it was.
        let checks = self.profile.anim_checks_count;
        let Some(ai) = self.objects[i].ani else {
            if checks {
                self.objects[i].state = 0;
            }
            return;
        };
        if k >= self.anis[ai].count {
            if checks {
                self.objects[i].state = 0;
            }
            return;
        }
        let Some(block) = self.anim_block(self.anis[ai].global) else {
            return;
        };
        if self.objects[i].flags & LIVE == 0 {
            if self
                .profile
                .anim_wakes_only_kind
                .is_some_and(|k| self.objects[i].kind != u16::from(k))
            {
                return;
            }
            self.objects[i].flags |= LIVE | HIDDEN;
            self.objects[i].countdown = 0;
            self.anis[ai].frame = 0xFF;
        }
        self.request_anim(ai, &block, Body::Object(i), k);
    }

    /// The tail `1F82h` and `20D9h` share (`0x0A862`–`0x0A8BD`,
    /// `0x0A98B`–`0x0A9BF`).
    fn request_anim(&mut self, ai: usize, block: &Animations, body: Body, k: u8) {
        if self.anis[ai].phase == 0 {
            self.anim_start(ai, block, k);
            self.set_body_countdown(body, 0);
            return;
        }
        self.anis[ai].next = k.wrapping_add(1);
        if self.anim_ready(ai, block) != 0 {
            let cap = i32::from(self.anis[ai].cap);
            if cap < self.body_countdown(body) {
                self.set_body_countdown(body, cap);
            }
        }
    }

    /// `0883h:20D9h(1693h, k)`: animation `k` on the figure — bound to
    /// its entry first when it has none (`14DDh:0098h`), then as an
    /// object's. Answers `k + 1`, or 1 for a fresh start, or 0 when there
    /// is no entry for the figure at all.
    pub(crate) fn anim_player(&mut self, k: i16) -> i16 {
        self.events.push(Event::FigureAnim(k));
        let Ok(k) = u8::try_from(k) else {
            return 0;
        };
        // `+12h` as one byte (`0x0A91E`): a walk's 6 answers 0.
        let state = self.walker.as_ref().map_or(0, |w| {
            if w.walking && w.state == 0 {
                6
            } else {
                w.state
            }
        });
        let bound = self.walker.as_ref().is_some_and(|w| w.ani.is_some());
        let ok = match state {
            0 => self.bind_figure(),
            STATE_FIGURE_ANIM => true,
            STATE_NPC_ANIM => bound || self.bind_figure(),
            _ => false,
        };
        if !ok {
            return 0;
        }
        let Some(ai) = self.walker.as_ref().and_then(|w| w.ani) else {
            if let Some(w) = &mut self.walker {
                w.state = 0;
            }
            return 0;
        };
        let Some(block) = self.anim_block(self.anis[ai].global) else {
            return 0;
        };
        if self.anis[ai].phase == 0 {
            let started = self.anim_start(ai, &block, k);
            if let Some(w) = &mut self.walker {
                w.anim_acc = 0;
            }
            return i16::from(started);
        }
        self.anis[ai].next = k.wrapping_add(1);
        if self.anim_ready(ai, &block) != 0 {
            let cap = i32::from(self.anis[ai].cap);
            if let Some(w) = &mut self.walker
                && cap < w.anim_acc
            {
                w.anim_acc = cap;
            }
        }
        i16::from(k) + 1
    }

    /// `14DDh:0098h(1693h)`: the figure bound to the entry that names it
    /// negated — the room's block first, the global one after
    /// (`0x14E82`, `0x14ECF`) — and put into state `0Ah`. `+2Eh` keeps
    /// `+0Ch`, then `+0Ch` goes down by the block and `+9` up by it
    /// (`0x14F47`–`0x14F70`): for a record whose `+0Ch` is its block, the
    /// cel base becomes the file's first cel, and the streams' frames are
    /// cels of the player's file counted from it.
    ///
    /// `0x14F4B` copies `+0Ch` into `+2Eh`; the block is `+2Ch` (`0x14F4F`),
    /// or one block when the mirror bit `+2` bit 6 is set (`0x14F53`); it is
    /// subtracted from `+0Ch` (`0x14F66`) and added to `+15h` plus one for `+9`
    /// (`0x14F5A`, `0x14F6E`), and the state set to `0Ah` (`0x14F9A`).
    fn bind_figure(&mut self) -> bool {
        let want = -PLAYER_ID;
        let Some(ai) = self
            .anis
            .iter()
            .position(|a| !a.global && a.id == want)
            .or_else(|| self.anis.iter().position(|a| a.global && a.id == want))
        else {
            return false;
        };
        let Some(w) = &mut self.walker else {
            return false;
        };
        w.final_facing = num::signed(w.base);
        let d = if w.mirror {
            u16::from(w.gait.phases) + 1
        } else {
            num::unsigned(w.block)
        };
        w.base = w.base.wrapping_sub(d);
        let frame = w.phase.wrapping_add(num::low8(d));
        w.anim_frame = frame;
        w.ani = Some(ai);
        w.state = STATE_FIGURE_ANIM;
        self.anis[ai].frame = frame;
        self.anis[ai].head = Some(Body::Figure);
        true
    }

    /// `14DDh:01D4h(1693h)`: a running animation on the figure stopped —
    /// the entry idled, the record back on its standing cel, and the
    /// block what the mirror bit says: mirrored, block 3 and `+0Ch` up by
    /// one block; else block 3 becomes block 1 and `+0Ch` goes up by the
    /// block (`0x14FEE`–`0x15055`).
    ///
    /// It acts only while state `0Ah` runs (`0x14FAB`), idles the entry
    /// (`+7 = 0`, `0x14FF1`), and by the mirror bit (`0x1500D`) either adds one
    /// block to `+0Ch` (`0x1501B`) or turns block 3 into block 1 (`0x15029`) and
    /// adds the block (`0x1503A`); `+9` and `+0Ah` become the standing cel out
    /// of `+15h` (`0x15041`).
    pub(crate) fn stop_player_anim(&mut self) {
        let Some(w) = &mut self.walker else {
            return;
        };
        if w.state != STATE_FIGURE_ANIM {
            return;
        }
        let ai = w.ani;
        w.state = 0;
        w.anim_acc = 0;
        let stride = i16::from(w.gait.phases) + 1;
        if w.mirror {
            w.block = stride.wrapping_mul(3);
            w.base = w.base.wrapping_add(num::unsigned(stride));
        } else {
            if w.block >= stride.wrapping_mul(3) {
                w.block = stride;
            }
            w.base = w.base.wrapping_add(num::unsigned(w.block));
        }
        w.phase = w.gait.phases;
        w.cel_held = None;
        if let Some(ai) = ai
            && self.anis[ai].id == -PLAYER_ID
        {
            self.anis[ai].phase = 0;
        }
    }

    /// `0CFDh:1BA2h(k)`: a page task 0 holds for a click let go —
    /// `k` kept as the code that let it go (`[1E48h]`), the hold bit
    /// cleared and the countdown run out.
    pub(crate) fn page_release(&mut self, k: u16) {
        self.page_release_code = k;
        let hold = self.profile.text_flags.hold;
        let t = &mut self.tasks.t[0];
        if t.flags & hold != 0 {
            t.flags &= !hold;
            t.countdown = 0;
        }
    }

    /// `0EB9h:1D97h(obj, code)`, the callback an `17` opcode reaches
    /// through `[1EE2h]` (`0x09E37`).
    ///
    /// Twelve arms through a jump table (`0x10938`): 1 lets the held page go
    /// (`0CFDh:1BA2h`, `0x10951`); 3 asks for sample 4 (`0x10975`) and 8 for
    /// sample 3 (`0x10961`); 5 arms task 0 (`0CFDh:1A2Eh`, `0x1093D`) and 6
    /// starts it (`0CFDh:1A3Fh`, `0x10947`); 11 and 12 take the pending figure
    /// animation in `[1E50h]` (`0x10988`). The code the machine waits for,
    /// `[28C1h]`, is compared (`0x109A8`), and the location's own hook
    /// `[1E60h]` is called (`0x109BB`).
    fn anim_signal(&mut self, body: Body, code: u16) {
        let object = self.body_id(body);
        self.events.push(Event::AnimCode { object, code });
        match code {
            1 => self.page_release(1),
            3 => self.effect_1949(4),
            5 => self.task_arm(0),
            6 => self.task_start(0),
            8 => self.effect_1949(3),
            11 | 12 => {
                self.script.figure_animation = self.script.pending_animation;
                if self.script.figure_animation >= 0 {
                    let k = self.script.figure_animation;
                    self.anim_player(k);
                }
                self.script.pending_animation = -1;
            }
            _ => {}
        }
        if code == self.anim_wait {
            self.anim_wait = 0;
        }
        // `0x109BB`: and the location's own hook at `[1E60h]`, at once.
        if let Some(table) = self.logic_table {
            table.anim_hook(self, Far::object(u16::from(object)), code);
        }
    }
}

/// The box a chain's parts stack against (`0x0A58E`): the head's own
/// edges, which stay, and the running top and bottom the parts above and
/// below it move.
#[derive(Debug, Clone, Copy)]
struct Stack {
    left: i32,
    top: i32,
    top_of_head: i32,
    bottom: i32,
    bottom_of_head: i32,
}

#[cfg(test)]
mod tests {
    //! The interpreter on streams of its own, with the room's file and
    //! the generator both out of the picture: what each opcode does to
    //! the entry's cursor, stack and frame is the whole of the claim.

    use super::*;
    use crate::titles::parsec1994::ObjectRt;

    /// A block with one entry for object 1 and one animation at `0xA0`
    /// holding `stream`, padded so every jump lands inside the block.
    fn block_with(stream: &[u8]) -> Animations {
        let mut b = vec![0u8; 0x200];
        b[..2].copy_from_slice(motionvm_parsec_formats::ani::MAGIC);
        b[4..8].copy_from_slice(&0x10u32.to_le_bytes());
        b[0x12..0x14].copy_from_slice(&1u16.to_le_bytes());
        b[0x14] = 1;
        b[0x1C] = 10;
        b[0x1D] = 20;
        b[0x1E] = 15;
        b[0x84..0x88].copy_from_slice(motionvm_parsec_formats::ani::ENTRY_MAGIC);
        b[0x88..0x8A].copy_from_slice(&0xA0u16.to_le_bytes());
        b[0xA0..0xA0 + stream.len()].copy_from_slice(stream);
        Animations::parse(&b).expect("a block")
    }

    /// A fresh game's runtime with the block put in place of the room's
    /// and one object of kind 4 at (10, 20) in place of the room's, bound
    /// to the block's entry — or `None` where no install is at hand.
    fn rig(stream: &[u8]) -> Option<Adventure> {
        let mut g = crate::titles::ds1::adventure_for_test()?;
        let block = Arc::new(block_with(stream));
        g.room_animations = Some(block.clone());
        g.anis = block.entries().map(|e| AniRun::new(&e, false)).collect();
        g.objects = vec![ObjectRt {
            kind: 4,
            flags: LIVE,
            align: 0,
            x: 10,
            y: 20,
            bundle: 0,
            frame: 0,
            cel: 5,
            cel_base: 5,
            scale: (0, 0),
            state: STATE_SINGLE,
            id: 1,
            phases: 3,
            countdown: 0,
            ani: Some(0),
            defaults: Default::default(),
            layer: 0,
        }];
        g.anis[0].head = Some(Body::Object(0));
        if let Some(w) = &mut g.walker {
            w.state = 0;
        }
        Some(g)
    }

    /// Ticks until the object shows `frame`, or `None` after `limit`.
    ///
    /// A wait of `d` keeps a frame for `d − 1` passes after the one that
    /// set it: that pass took the countdown to −1 first (`0x08B5E`), and
    /// the wait is added to what is there (`0x0A066`).
    fn ticks_until_frame(g: &mut Adventure, frame: u8, limit: i32) -> Option<i32> {
        for t in 1..=limit {
            g.animator_tick(1);
            if g.objects[0].frame == frame && g.objects[0].flags & HIDDEN == 0 {
                return Some(t);
            }
        }
        None
    }

    #[test]
    fn a_single_frame_opcode_shows_its_frame_for_its_wait() {
        // `82 02 28`: frame 2 for 40 ticks; then `80 01`: frame 1 for the
        // default 15; then end.
        let Some(mut g) = rig(&[0x82, 0x02, 0x28, 0x80, 0x01, 0x00]) else {
            return;
        };
        g.object_anim(1, 0);
        assert_eq!(g.anis[0].phase, 1);
        assert_eq!(ticks_until_frame(&mut g, 2, 10), Some(1), "at once");
        assert_eq!(g.objects[0].cel, 7, "cel base plus the frame");
        assert_eq!(
            g.objects[0].countdown, 39,
            "forty, less the tick that ran out"
        );
        assert_eq!(
            ticks_until_frame(&mut g, 1, 100),
            Some(39),
            "after its wait"
        );
        for _ in 0..15 {
            g.animator_tick(1);
        }
        assert_eq!(g.anis[0].phase, 0, "idle after the end");
        assert_eq!(g.objects[0].countdown, IDLE_WAIT);
    }

    #[test]
    fn a_forward_sequence_steps_through_its_frames_and_goes_on_past_them() {
        // `8A 05 03 01 02 03`: three frames five ticks apart; then
        // `82 09 01` and end.
        let Some(mut g) = rig(&[0x8A, 0x05, 0x03, 0x01, 0x02, 0x03, 0x82, 0x09, 0x01, 0x00]) else {
            return;
        };
        g.object_anim(1, 0);
        let mut seen = Vec::new();
        for t in 1..=25 {
            g.animator_tick(1);
            let f = g.objects[0].frame;
            if seen.last().map(|&(_, f0)| f0) != Some(f) {
                seen.push((t, f));
            }
        }
        assert_eq!(seen, [(1, 1), (5, 2), (10, 3), (15, 9)]);
    }

    #[test]
    fn a_loop_without_a_count_repeats_until_the_next_animation_is_asked_for() {
        // `03 end`, `06`, `0C out`, `80 01`, `80 02`, `02`, out: `00`.
        let stream = [
            0x03, 0xB0, 0x00, 0x06, 0x0C, 0xAF, 0x00, 0x80, 0x01, 0x80, 0x02, 0x02, 0x00,
        ];
        let Some(mut g) = rig(&stream) else {
            return;
        };
        g.object_anim(1, 0);
        let mut frames = Vec::new();
        for _ in 0..8 {
            let before = g.objects[0].frame;
            for _ in 0..15 {
                g.animator_tick(1);
            }
            let f = g.objects[0].frame;
            if f != before {
                frames.push(f);
            }
        }
        // Sampled every fifteen ticks: the frame set on the fifteenth.
        assert_eq!(frames, [2, 1, 2, 1, 2, 1, 2, 1], "round and round");
        assert_eq!(g.anis[0].frames, 2, "the two frames stay on the stack");
    }

    #[test]
    fn a_counted_loop_runs_its_count_and_falls_out() {
        // `0B 02 out out`, `80 01`, `80 02`, `02`, out at 0xA9: `80 03`, `00`.
        let stream = [
            0x0B, 0x02, 0xA9, 0x00, 0x80, 0x01, 0x80, 0x02, 0x02, 0x80, 0x03, 0x00,
        ];
        let Some(mut g) = rig(&stream) else {
            return;
        };
        g.object_anim(1, 0);
        let mut frames = Vec::new();
        for _ in 0..7 {
            let before = g.objects[0].frame;
            for _ in 0..15 {
                g.animator_tick(1);
            }
            let f = g.objects[0].frame;
            if f != before {
                frames.push(f);
            }
        }
        assert_eq!(frames, [2, 1, 2, 3], "twice through, then out");
        assert_eq!(g.anis[0].loops, 0);
        assert_eq!(g.anis[0].frames, 0, "the loop's frame is popped");
    }

    #[test]
    fn a_call_returns_after_the_call_and_skips_the_callees_return_frame() {
        // `04 sub`, `80 03`, `00`; sub at 0xB0: `03 xx xx`, `80 07`, `01`.
        let mut stream = vec![0x04, 0xB0, 0x00, 0x80, 0x03, 0x00];
        stream.resize(0x10, 0);
        stream.extend_from_slice(&[0x03, 0x00, 0x00, 0x80, 0x07, 0x01]);
        let Some(mut g) = rig(&stream) else {
            return;
        };
        g.object_anim(1, 0);
        assert_eq!(
            ticks_until_frame(&mut g, 7, 5),
            Some(1),
            "the callee's frame first"
        );
        assert_eq!(g.anis[0].frames, 1, "one frame: the call's");
        assert_eq!(
            ticks_until_frame(&mut g, 3, 40),
            Some(14),
            "then back after the call"
        );
        assert_eq!(g.anis[0].frames, 0);
    }

    #[test]
    fn a_code_opcode_reaches_the_callback_and_clears_the_wait() {
        let Some(mut g) = rig(&[0x17, 0x02, 0x00, 0x80, 0x01, 0x00]) else {
            return;
        };
        g.anim_wait = 2;
        g.object_anim(1, 0);
        g.animator_tick(1);
        assert_eq!(g.anim_wait, 0, "the machine's wait is over");
        assert!(g.events.contains(&Event::AnimCode { object: 1, code: 2 }));
    }

    #[test]
    fn a_hidden_frame_hides_and_a_door_comes_up_hidden_first() {
        // `8A 0A 03 FF 00 01`: hidden, closed, open, ten ticks each.
        let Some(mut g) = rig(&[0x8A, 0x0A, 0x03, 0xFF, 0x00, 0x01, 0x00]) else {
            return;
        };
        g.objects[0].flags = 0;
        g.object_anim(1, 0);
        assert_eq!(
            g.objects[0].flags & (LIVE | HIDDEN),
            LIVE | HIDDEN,
            "a door not live comes up hidden"
        );
        g.animator_tick(1);
        assert!(g.objects[0].flags & HIDDEN != 0, "the first frame is FFh");
        assert_eq!(ticks_until_frame(&mut g, 0, 20), Some(9));
        assert_eq!(ticks_until_frame(&mut g, 1, 20), Some(10));
        assert!(g.objects[0].visible());
    }

    #[test]
    fn an_animation_asked_for_takes_over_where_the_condition_allows() {
        // Animation 0: `05` (never), `0C out`, `80 01`, `02`. Animation 1
        // at 0xC0: `80 04`, `00`. A block with two animations.
        let mut b = vec![0u8; 0x200];
        b[..2].copy_from_slice(motionvm_parsec_formats::ani::MAGIC);
        b[4..8].copy_from_slice(&0x10u32.to_le_bytes());
        b[0x12..0x14].copy_from_slice(&1u16.to_le_bytes());
        b[0x14] = 2;
        b[0x1E] = 15;
        b[0x84..0x88].copy_from_slice(motionvm_parsec_formats::ani::ENTRY_MAGIC);
        b[0x88..0x8A].copy_from_slice(&0xA0u16.to_le_bytes());
        b[0x8A..0x8C].copy_from_slice(&0xC0u16.to_le_bytes());
        b[0xA0..0xA7].copy_from_slice(&[0x05, 0x0C, 0xB0, 0x00, 0x80, 0x01, 0x02]);
        b[0xC0..0xC3].copy_from_slice(&[0x80, 0x04, 0x00]);
        let block = Arc::new(Animations::parse(&b).expect("block"));
        let Some(mut g) = rig(&[0x00]) else {
            return;
        };
        g.room_animations = Some(block.clone());
        g.anis = block.entries().map(|e| AniRun::new(&e, false)).collect();
        g.anis[0].head = Some(Body::Object(0));
        g.object_anim(1, 0);
        assert_eq!(ticks_until_frame(&mut g, 1, 5), Some(1));
        g.object_anim(1, 1);
        assert_eq!(g.anis[0].next, 2, "asked for");
        for _ in 0..60 {
            g.animator_tick(1);
        }
        assert_eq!(g.objects[0].frame, 1, "condition 0 never lets it in");
        // Condition 1 in front lets it in at the next step.
        b[0xA0] = 0x06;
        let block = Arc::new(Animations::parse(&b).expect("block"));
        g.room_animations = Some(block.clone());
        g.anis = block.entries().map(|e| AniRun::new(&e, false)).collect();
        g.anis[0].head = Some(Body::Object(0));
        g.objects[0].countdown = 0;
        g.object_anim(1, 0);
        assert_eq!(ticks_until_frame(&mut g, 1, 5), Some(1));
        g.object_anim(1, 1);
        assert_eq!(
            ticks_until_frame(&mut g, 4, 20),
            Some(1),
            "the wait is capped to 0"
        );
    }
}
