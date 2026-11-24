//! A title's location logic, run case by case against a stand-in engine
//! and digested.
//!
//! A case is one entry point of the original logic and one set of inputs:
//! the location (`G` a rule of the global table, `H` the shared hook,
//! neither of them a location's), the kind of entry (`h` handler, `k`
//! hook, `a` animation hook, `r` rule handler), the routine's address, and what the data
//! segment and the engine hold when it runs. A title keeps its cases in
//! the crate's `tests/`, one head line each; they are inputs, not data of
//! the game. For each, [`Recording`] runs the logic against an
//! [`Api`] that writes down every call (target, argument words) in the
//! notation below and keeps the data segment as a plain 64 KiB image taken
//! from the program's `DGROUP`, and the case's digest folds in what the
//! logic did: the calls in order, the bytes it changed, and how it ended.
//! The title holds the digests against its table in `tests/digests/`,
//! where every line is anchored to the original routine run on the same
//! inputs and traced the same way. Where the program has each routine is
//! the title's ([`Routines`]).
//!
//! An engine routine answers what the head's `ret=seg:off:v` names for it,
//! and every other answers 0, except a record lookup: id `n` answers
//! `(4000h + 4n):0000`, a window of 64 bytes of its own, so a record's
//! fields land where the diff can name them. The answers follow from the
//! head alone, which is what lets a digest of our run be held against one
//! of the original's.
//!
//! One case and what it digests:
//!
//! ```text
//! case L2h_0089_0079 89:79 frames=2 set=2725:3 wset=1c82:65 ret=14dd:7e7:1 poke=1:1c82:65 args=7,a,0
//!   call 0eb9:0001()
//!   diff 2711:00>01
//!   end returned frames=2
//! ```

use super::logic::Logic;
use super::{Api, Far};
use crate::num;
use std::collections::BTreeMap;

/// Where a title's program has the engine routines the logic calls, as the
/// trace writes them (`seg:off`, lower case), one per [`Api`] method.
pub(crate) struct Routines {
    pub visit_setup: &'static str,
    pub visit_enter: &'static str,
    pub visit_finish_entry: &'static str,
    pub dispatch: &'static str,
    pub frame_pass: &'static str,
    pub visit_leave: &'static str,
    pub slot_unload: &'static str,
    pub hover_on: &'static str,
    pub input_poll: &'static str,
    pub pointer_hide: &'static str,
    pub pointer_unfreeze: &'static str,
    pub key_flush: &'static str,
    pub scene_play: &'static str,
    pub task_arm: &'static str,
    pub task_start: &'static str,
    pub scene_sequence: &'static str,
    pub slot_set_object: &'static str,
    pub slot_set_figure: &'static str,
    pub obj_find: &'static str,
    /// `None` for a program whose library has no such routine.
    pub obj_wake: Option<&'static str>,
    pub obj_show: &'static str,
    pub obj_anim: &'static str,
    pub obj_anim_id: &'static str,
    pub obj_hide: &'static str,
    pub obj_state: &'static str,
    pub objs_save_state: &'static str,
    pub walk_to_node: &'static str,
    pub anim_pass: &'static str,
    pub room_redraw: &'static str,
    pub walker_snap: &'static str,
    pub walker_detach: &'static str,
    pub input_lock: &'static str,
    pub slot_set_raw: &'static str,
    pub obj_show_frame: &'static str,
    pub movmem: &'static str,
    pub setmem: &'static str,
    pub movmem_from_local: &'static str,
    pub heap_0720: &'static str,
    pub mem_free: &'static str,
    pub entry_door_close: &'static str,
    pub input_unlock: &'static str,
    pub face: &'static str,
    pub pointer_shape_resync: &'static str,
    pub mover_find: &'static str,
    pub walker_bind_graph: &'static str,
    pub walker_unlink: &'static str,
    pub scene_enqueue: &'static str,
    pub ambient_play: &'static str,
    pub pal_set: &'static str,
    pub song_load: &'static str,
    pub music_play: &'static str,
    pub music_play_fade: &'static str,
    pub music_fade: &'static str,
    pub music_stop: &'static str,
    pub hover_off: &'static str,
    pub play_sample: &'static str,
    pub walker_walk_to: &'static str,
    pub inv_take: &'static str,
    pub farptr_count: &'static str,
    pub inv_has: &'static str,
    pub random: &'static str,
    pub script_exit: &'static str,
    pub inv_give: &'static str,
    pub panel_refresh: &'static str,
    pub fx_reveal: &'static str,
    pub obj_invalidate: &'static str,
    pub figure_anim: &'static str,
    pub say_text: &'static str,
    pub hotspot_follow: &'static str,
    pub figure_face: &'static str,
    pub figure_turn: &'static str,
    pub task_stop: &'static str,
    pub page_release: &'static str,
    pub band_hold: &'static str,
    pub node_block: &'static str,
    pub node_open: &'static str,
    pub interaction: &'static str,
    /// The word that points at the hotspot the object was chosen
    /// through, whose lines [`Api::hotspot_own_text`] reads.
    pub chosen_hotspot: u16,
    /// The routines that answer with a record in `DX:AX`. The stand-in
    /// answers a lookup of id `n` with `(4000h + 4n):0000`, so a record's
    /// fields land in a window of their own; `ret=…:0` makes it null.
    pub record_returns: &'static [&'static str],
    /// The main loop's entry word, which the stand-in sets to `FFh` when
    /// it asks for an exit, as the engine does after `frames` pictures.
    pub exit_word: u16,
    /// The far pointer the shared hook hands its calls on through
    /// ([`Api::location_hook`]): the stand-in names the call by the
    /// pointer's two words, as the case set them.
    pub location_hook: u16,
    /// The far pointer the visit's callback is called through
    /// ([`Api::callback`]), named the same way; `None` for a program that
    /// keeps none.
    pub callback: Option<u16>,
}

/// What the runner needs of a title: its routines' addresses, which
/// logic each row has, and its global rules read against [`Api`].
pub(crate) struct TraceTitle {
    pub routines: &'static Routines,
    pub logic: fn(u16) -> Option<Logic>,
    pub global_rule: fn(&mut dyn Api, (u16, u16), u16) -> Option<i16>,
    /// The shared hook, `(code, arg, task)`, read against [`Api`].
    pub hook: fn(&mut dyn Api, u16, u16, u16),
}

/// One case: its entry and its inputs.
#[derive(Debug, Clone)]
pub(crate) struct Case {
    pub(crate) entry: String,
    /// The inputs as the head writes them, after the entry and its
    /// address.
    inputs: String,
    params: BTreeMap<String, String>,
}

impl Case {
    /// The case's name in the digest table: the entry and eight hex digits
    /// of an FNV-1a 64 over its inputs, so that two cases of one entry
    /// have two names and a name does not depend on where in the file its
    /// case stands.
    pub(crate) fn name(&self) -> String {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for &b in self.inputs.as_bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        format!("{}.{:08x}", self.entry, h & 0xFFFF_FFFF)
    }
}

/// What a case did: the calls in order, the bytes changed, the end.
pub(crate) struct Outcome {
    calls: Vec<String>,
    diff: String,
    end: String,
}

impl Outcome {
    /// The text the digest is taken over: a line per call, then `diff`
    /// and `end`, as in the module's example.
    pub(crate) fn text(&self) -> String {
        let mut t = String::new();
        for c in &self.calls {
            t.push_str("call ");
            t.push_str(c);
            t.push('\n');
        }
        t.push_str("diff ");
        t.push_str(&self.diff);
        t.push('\n');
        t.push_str("end ");
        t.push_str(&self.end);
        t.push('\n');
        t
    }
}

/// Reads the head lines of a case file; everything else is ignored.
pub(crate) fn parse(text: &str) -> Vec<Case> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("case "))
        .map(|rest| {
            let mut w = rest.splitn(3, ' ');
            let entry = w.next().unwrap_or_default().to_string();
            let _at = w.next();
            let inputs = w.next().unwrap_or_default().trim().to_string();
            let params = inputs
                .split_whitespace()
                .filter_map(|kv| kv.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            Case {
                entry,
                inputs,
                params,
            }
        })
        .collect()
}

fn hex(s: &str) -> u16 {
    u16::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0)
}

/// The stand-in engine.
pub(crate) struct Recording {
    /// Where the title's program has the engine's routines.
    routines: &'static Routines,
    ds: Vec<u8>,
    dgroup: u16,
    /// The program's image, for what a write outside `DGROUP` overwrites.
    image: Vec<u8>,
    /// Words written outside `DGROUP`, by physical address: first value
    /// and last.
    far: BTreeMap<usize, (u8, u8)>,
    /// The handler's own locals (see `Api::local`).
    locals: [u16; 8],
    /// First value of every byte the logic wrote, by address.
    written: BTreeMap<u16, u8>,
    calls: Vec<String>,
    rets: BTreeMap<String, u16>,
    frames: u32,
    frame: u32,
    pokes: Vec<(u32, u16, u16)>,
    /// `absent=seg:off:id`: that record routine answers null for that id.
    absent: Vec<(String, u16)>,
}

impl Recording {
    /// A stand-in over the program's data segment, with the case's inputs
    /// applied.
    pub(crate) fn new(routines: &'static Routines, image: &[u8], dgroup: u16, case: &Case) -> Self {
        let at = usize::from(dgroup) * 16;
        let mut ds = vec![0u8; 0x10000];
        let n = image.len().saturating_sub(at).min(0x10000);
        ds[..n].copy_from_slice(&image[at..at + n]);
        let mut r = Recording {
            routines,
            ds,
            dgroup,
            image: image.to_vec(),
            far: BTreeMap::new(),
            locals: [0; 8],
            written: BTreeMap::new(),
            calls: Vec::new(),
            rets: BTreeMap::new(),
            frames: case
                .params
                .get("frames")
                .map_or(2, |v| v.parse().unwrap_or(2)),
            frame: 0,
            pokes: Vec::new(),
            absent: Vec::new(),
        };
        for (k, v) in &case.params {
            for p in v.split(',') {
                let q: Vec<&str> = p.split(':').collect();
                match k.as_str() {
                    "set" if q.len() == 2 => r.ds[usize::from(hex(q[0]))] = num::low8(hex(q[1])),
                    "wset" if q.len() == 2 => r.poke(hex(q[0]), hex(q[1])),
                    "ret" if q.len() == 3 => {
                        let name = format!("{:04x}:{:04x}", hex(q[0]), hex(q[1]));
                        r.rets.insert(name, hex(q[2]));
                    }
                    // A record field as the oracle presets it, in the
                    // window of its id at physical 40000h + 64 id.
                    "rset" | "rwset" if q.len() == 3 => {
                        let at = 0x40000 + usize::from(hex(q[0])) * 64 + usize::from(hex(q[1]));
                        let n = if k == "rset" { 1 } else { 2 };
                        for (i, b) in hex(q[2]).to_le_bytes().into_iter().take(n).enumerate() {
                            r.far.insert(at + i, (b, b));
                        }
                    }
                    "absent" if q.len() == 3 => {
                        let name = format!("{:04x}:{:04x}", hex(q[0]), hex(q[1]));
                        r.absent.push((name, hex(q[2])));
                    }
                    "poke" if q.len() == 3 => {
                        r.pokes
                            .push((q[0].parse().unwrap_or(0), hex(q[1]), hex(q[2])));
                    }
                    _ => {}
                }
            }
        }
        r
    }

    fn poke(&mut self, addr: u16, v: u16) {
        let [lo, hi] = v.to_le_bytes();
        self.ds[usize::from(addr)] = lo;
        self.ds[usize::from(addr.wrapping_add(1))] = hi;
    }

    fn write(&mut self, addr: u16, v: u8) {
        let old = self.ds[usize::from(addr)];
        self.written.entry(addr).or_insert(old);
        self.ds[usize::from(addr)] = v;
    }

    /// Notes a call and answers it by the head's rule: what `ret=` names
    /// for its target, else 0.
    fn call(&mut self, name: &str, args: &[u16]) -> u16 {
        let a: Vec<String> = args.iter().map(|w| format!("{w:04x}")).collect();
        self.call_text(format!("{name}({})", a.join(",")))
    }

    /// Notes a call already written out and answers it as [`Self::call`].
    fn call_text(&mut self, text: String) -> u16 {
        let name = text.split('(').next().unwrap_or_default();
        let ret = self.rets.get(name).copied().unwrap_or(0);
        self.calls.push(text);
        ret
    }

    /// A record lookup: the stand-in's window for the id, or what `ret=` set.
    fn record(&mut self, name: &str, id: u16, args: &[u16]) -> Far {
        self.call(name, args);
        match self.rets.get(name) {
            Some(&v) => Far { off: v, seg: 0 },
            None if self.absent.iter().any(|(n, i)| n == name && *i == id) => Far::NULL,
            None if self.routines.record_returns.contains(&name) => Far {
                off: 0,
                seg: 0x4000u16.wrapping_add(id.wrapping_mul(4)),
            },
            None => Far::NULL,
        }
    }

    /// The bytes changed, in the notation of the module's example, ordered
    /// by physical address: `DS` offsets for `DGROUP`,
    /// `recN+off` for the window a looked-up record `N` stands in
    /// (`(4000h + 4N):0000`, 64 bytes each), `physNNNNN` elsewhere.
    pub(crate) fn diff(&self) -> String {
        let base = usize::from(self.dgroup) * 16;
        let mut all: Vec<(usize, String)> = self
            .far
            .iter()
            .filter(|(_, (old, new))| old != new)
            .map(|(&a, (old, new))| {
                let name = if (0x40000..0x80000).contains(&a) {
                    format!("rec{:x}+{:x}", (a - 0x40000) / 64, (a - 0x40000) % 64)
                } else {
                    format!("phys{a:05x}")
                };
                (a, format!("{name}:{old:02x}>{new:02x}"))
            })
            .collect();
        all.extend(
            self.written
                .iter()
                .filter(|&(&a, &old)| old != self.ds[usize::from(a)])
                .map(|(&a, &old)| {
                    (
                        base + usize::from(a),
                        format!("{a:04x}:{old:02x}>{:02x}", self.ds[usize::from(a)]),
                    )
                }),
        );
        all.sort_by_key(|&(a, _)| a);
        all.into_iter()
            .map(|(_, t)| t)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Api for Recording {
    fn byte(&self, addr: u16) -> u8 {
        self.ds[usize::from(addr)]
    }
    fn set_byte(&mut self, addr: u16, v: u8) {
        self.write(addr, v);
    }
    fn word(&self, addr: u16) -> u16 {
        u16::from_le_bytes([
            self.ds[usize::from(addr)],
            self.ds[usize::from(addr.wrapping_add(1))],
        ])
    }
    fn set_word(&mut self, addr: u16, v: u16) {
        let [lo, hi] = v.to_le_bytes();
        self.write(addr, lo);
        self.write(addr.wrapping_add(1), hi);
    }
    fn ds(&self, off: u16) -> Far {
        Far {
            off,
            seg: self.dgroup,
        }
    }

    fn visit_setup(&mut self) {
        self.call(self.routines.visit_setup, &[]);
    }
    fn visit_enter(&mut self, loc: u16, entry: u16) {
        self.call(self.routines.visit_enter, &[loc, entry]);
    }
    fn visit_finish_entry(&mut self) {
        self.call(self.routines.visit_finish_entry, &[]);
    }
    fn dispatch(&mut self) {
        self.call(self.routines.dispatch, &[]);
    }
    fn frame_pass(&mut self) {
        self.call(self.routines.frame_pass, &[]);
    }
    fn visit_leave(&mut self) {
        self.call(self.routines.visit_leave, &[]);
    }
    fn slot_unload(&mut self) {
        self.call(self.routines.slot_unload, &[]);
    }
    fn hover_on(&mut self) {
        self.call(self.routines.hover_on, &[]);
    }
    fn input_poll(&mut self) {
        self.call(self.routines.input_poll, &[]);
        // The stand-in's frame clock: the engine writes
        // between two pictures, and asks for an exit after `frames`.
        self.frame += 1;
        let due: Vec<(u16, u16)> = self
            .pokes
            .iter()
            .filter(|p| p.0 == self.frame)
            .map(|p| (p.1, p.2))
            .collect();
        for (a, v) in due {
            self.poke(a, v);
        }
        if self.frame >= self.frames {
            self.poke(self.routines.exit_word, 0x00FF);
        }
    }
    fn pointer_hide(&mut self) {
        self.call(self.routines.pointer_hide, &[]);
    }
    fn pointer_unfreeze(&mut self) {
        self.call(self.routines.pointer_unfreeze, &[]);
    }
    fn key_flush(&mut self) {
        self.call(self.routines.key_flush, &[]);
    }
    fn scene_play(&mut self, n: i16) -> u16 {
        self.call(self.routines.scene_play, &[num::unsigned(n)])
    }
    fn task_arm(&mut self, i: u16) {
        self.call(self.routines.task_arm, &[i]);
    }
    fn task_start(&mut self, i: u16) {
        self.call(self.routines.task_start, &[i]);
    }
    fn scene_sequence(&mut self, counter: Far, scenes: &[i16]) {
        let mut args = vec![counter.off, counter.seg];
        args.extend(scenes.iter().map(|&n| num::unsigned(n)));
        args.push(0xFFFF);
        self.call(self.routines.scene_sequence, &args);
    }
    fn slot_set_object(&mut self, slot: u16, code: u16, rec: Far) {
        self.call(
            self.routines.slot_set_object,
            &[slot, code, rec.off, rec.seg],
        );
    }
    fn slot_set_figure(&mut self, slot: u16, code: u16, rec: Far) {
        self.call(
            self.routines.slot_set_figure,
            &[slot, code, rec.off, rec.seg],
        );
    }
    fn obj_find(&mut self, id: u16) -> Far {
        self.record(self.routines.obj_find, id, &[id])
    }
    fn obj_wake(&mut self, id: u16) -> Far {
        // A program without the routine names none, and no trace of it
        // has the call.
        let name = self.routines.obj_wake.unwrap_or("none");
        self.record(name, id, &[id])
    }
    fn obj_show(&mut self, id: u16) -> Far {
        self.record(self.routines.obj_show, id, &[id])
    }
    fn obj_anim(&mut self, rec: Far, k: i16) {
        self.call(
            self.routines.obj_anim,
            &[rec.off, rec.seg, num::unsigned(k)],
        );
    }
    fn obj_anim_id(&mut self, id: u16, k: i16) {
        self.call(self.routines.obj_anim_id, &[id, num::unsigned(k)]);
    }
    fn obj_hide(&mut self, id: u16) -> Far {
        self.record(self.routines.obj_hide, id, &[id])
    }
    fn obj_state(&mut self, id: u16) -> i16 {
        num::signed(self.call(self.routines.obj_state, &[id]))
    }
    fn objs_save_state(&mut self, out: Far) -> u16 {
        self.call(self.routines.objs_save_state, &[out.off, out.seg])
    }
    fn walk_to_node(&mut self, node: u16) -> i16 {
        num::signed(self.call(self.routines.walk_to_node, &[node]))
    }
    fn anim_pass(&mut self, delta: i16) {
        self.call(self.routines.anim_pass, &[num::unsigned(delta)]);
    }
    fn room_redraw(&mut self) {
        self.call(self.routines.room_redraw, &[]);
    }
    fn walker_snap(&mut self, rec: Far, node: u16, facing: i16) -> i16 {
        num::signed(self.call(
            self.routines.walker_snap,
            &[rec.off, rec.seg, node, num::unsigned(facing)],
        ))
    }
    fn walker_detach(&mut self, rec: Far) {
        self.call(self.routines.walker_detach, &[rec.off, rec.seg]);
    }
    fn input_lock(&mut self) {
        self.call(self.routines.input_lock, &[]);
    }
    fn slot_set_raw(&mut self, slot: u16, code: u16, w1: u16, w2: u16) {
        self.call(self.routines.slot_set_raw, &[slot, code, w1, w2]);
    }
    fn obj_show_frame(&mut self, id: u16, k: i16) -> Far {
        self.record(self.routines.obj_show_frame, id, &[id, num::unsigned(k)])
    }
    fn movmem(&mut self, src: Far, dst: Far, n: u16) {
        self.call(
            self.routines.movmem,
            &[src.off, src.seg, dst.off, dst.seg, n],
        );
        for i in 0..n {
            let b = self.far_byte(Far {
                off: src.off.wrapping_add(i),
                seg: src.seg,
            });
            self.set_far_byte(
                Far {
                    off: dst.off.wrapping_add(i),
                    seg: dst.seg,
                },
                b,
            );
        }
    }
    fn setmem(&mut self, dst: Far, val: u16, n: u16) {
        self.call(self.routines.setmem, &[dst.off, dst.seg, val, n]);
        for i in 0..n {
            self.set_far_byte(
                Far {
                    off: dst.off.wrapping_add(i),
                    seg: dst.seg,
                },
                num::low8(val),
            );
        }
    }
    fn movmem_from_local(&mut self, src: &[u8], dst: Far) {
        // A pointer onto the caller's stack is written `stk`: where the
        // stack is says nothing about the logic.
        let a: Vec<String> = [dst.off, dst.seg, u16::try_from(src.len()).unwrap()]
            .iter()
            .map(|w| format!("{w:04x}"))
            .collect();
        self.call_text(format!(
            "{}(stk,stk,{})",
            self.routines.movmem_from_local,
            a.join(",")
        ));
        for (i, &b) in src.iter().enumerate() {
            self.set_far_byte(
                Far {
                    off: dst.off.wrapping_add(u16::try_from(i).unwrap()),
                    seg: dst.seg,
                },
                b,
            );
        }
    }
    fn heap_0720(&mut self, p: Far) -> u16 {
        self.call(self.routines.heap_0720, &[p.off, p.seg])
    }
    fn mem_free(&mut self, p: Far) -> u16 {
        self.call(self.routines.mem_free, &[p.off, p.seg])
    }
    fn entry_door_close(&mut self) {
        self.call(self.routines.entry_door_close, &[]);
    }
    fn input_unlock(&mut self) {
        self.call(self.routines.input_unlock, &[]);
    }
    fn face(&mut self, dir: i16) {
        self.call(self.routines.face, &[num::unsigned(dir)]);
    }
    fn pointer_shape_resync(&mut self) {
        self.call(self.routines.pointer_shape_resync, &[]);
    }
    fn mover_find(&mut self, id: u16) -> Far {
        self.record(self.routines.mover_find, id, &[id])
    }
    fn walker_bind_graph(&mut self, rec: Far, graph: u16, node: u16, facing: i16) -> i16 {
        num::signed(self.call(
            self.routines.walker_bind_graph,
            &[rec.off, rec.seg, graph, node, num::unsigned(facing)],
        ))
    }
    fn walker_unlink(&mut self, rec: Far) {
        self.call(self.routines.walker_unlink, &[rec.off, rec.seg]);
    }
    fn scene_enqueue(&mut self, n: i16) -> u16 {
        self.call(self.routines.scene_enqueue, &[num::unsigned(n)])
    }
    fn ambient_play(&mut self, lo: i16, hi: i16, flags: u16) -> u16 {
        self.call(
            self.routines.ambient_play,
            &[num::unsigned(lo), num::unsigned(hi), flags],
        )
    }
    fn pal_set(&mut self, rgb: Far, start: u16, count: u16) {
        self.call(self.routines.pal_set, &[rgb.off, rgb.seg, start, count]);
    }
    fn song_load(&mut self, n: u16) -> Far {
        let ax = self.call(self.routines.song_load, &[n]);
        match self.rets.get(self.routines.song_load) {
            Some(&v) => Far { off: v, seg: 0 },
            None => Far { off: ax, seg: 0 },
        }
    }
    fn music_play(&mut self, res: Far, a: i16) {
        self.call(
            self.routines.music_play,
            &[res.off, res.seg, num::unsigned(a)],
        );
    }
    fn music_play_fade(&mut self, res: Far, a: i16, b: u16) {
        self.call(
            self.routines.music_play_fade,
            &[res.off, res.seg, num::unsigned(a), b],
        );
    }
    fn music_fade(&mut self, n: i16) {
        self.call(self.routines.music_fade, &[num::unsigned(n)]);
    }
    fn music_stop(&mut self) {
        self.call(self.routines.music_stop, &[]);
    }
    fn set_far_byte(&mut self, at: Far, v: u8) {
        if at.seg == self.dgroup {
            self.write(at.off, v);
            return;
        }
        let a = usize::from(at.seg) * 16 + usize::from(at.off);
        let old = self.image.get(a).copied().unwrap_or(0);
        self.far.entry(a).or_insert((old, old)).1 = v;
    }
    fn local(&self, i: usize) -> u16 {
        self.locals.get(i).copied().unwrap_or(0)
    }
    fn set_local(&mut self, i: usize, v: u16) {
        if let Some(l) = self.locals.get_mut(i) {
            *l = v;
        }
    }
    fn far_byte(&mut self, at: Far) -> u8 {
        if at.seg == self.dgroup {
            return self.ds[usize::from(at.off)];
        }
        let a = usize::from(at.seg) * 16 + usize::from(at.off);
        match self.far.get(&a) {
            Some(&(_, v)) => v,
            None => self.image.get(a).copied().unwrap_or(0),
        }
    }
    fn hover_off(&mut self) {
        self.call(self.routines.hover_off, &[]);
    }
    fn play_sample(&mut self, k: u16) {
        self.call(self.routines.play_sample, &[k]);
    }
    fn walker_walk_to(&mut self, rec: Far, node: u16, facing: i16) -> i16 {
        num::signed(self.call(
            self.routines.walker_walk_to,
            &[rec.off, rec.seg, node, num::unsigned(facing)],
        ))
    }
    fn inv_take(&mut self, id: u16) -> i16 {
        num::signed(self.call(self.routines.inv_take, &[id]))
    }
    fn farptr_count(&mut self, row: Far) -> u16 {
        self.call(self.routines.farptr_count, &[row.off, row.seg])
    }
    fn inv_has(&mut self, id: u16) -> i16 {
        num::signed(self.call(self.routines.inv_has, &[id]))
    }
    fn random(&mut self, lo: u16, hi: u16) -> u16 {
        self.call(self.routines.random, &[lo, hi])
    }
    fn script_exit(&mut self, k: u16) {
        self.call(self.routines.script_exit, &[k]);
    }
    fn obj_invalidate(&mut self, rec: Far) {
        self.call(self.routines.obj_invalidate, &[rec.off, rec.seg]);
    }
    fn figure_anim(&mut self, rec: Far, k: i16) -> i16 {
        num::signed(self.call(
            self.routines.figure_anim,
            &[rec.off, rec.seg, num::unsigned(k)],
        ))
    }
    fn say_text(&mut self, text: Far) {
        self.call(self.routines.say_text, &[text.off, text.seg]);
    }
    fn hotspot_own_text(&mut self, k: u16) -> Far {
        // The logic reads the record pointer and the line's far pointer
        // itself; nothing is called.
        let at = self.routines.chosen_hotspot;
        let rec = Far {
            off: self.word(at),
            seg: self.word(at.wrapping_add(2)),
        };
        if rec == Far::NULL {
            return Far::NULL;
        }
        let field = |me: &mut Self, d: u16| {
            me.far_word(Far {
                off: rec.off.wrapping_add(0x16 + 4 * k + d),
                seg: rec.seg,
            })
        };
        let off = field(self, 0);
        let seg = field(self, 2);
        Far { off, seg }
    }
    fn hotspot_follow(&mut self, rec: Far) {
        self.call(self.routines.hotspot_follow, &[rec.off, rec.seg]);
    }
    fn figure_face(&mut self, rec: Far, dir: i16) {
        self.call(
            self.routines.figure_face,
            &[rec.off, rec.seg, num::unsigned(dir)],
        );
    }
    fn figure_turn(&mut self, k: i16) -> i16 {
        num::signed(self.call(self.routines.figure_turn, &[num::unsigned(k)]))
    }
    fn location_hook(&mut self, code: u16, arg: u16, task: u16) {
        let at = self.routines.location_hook;
        let target = format!(
            "{:04x}:{:04x}",
            self.word(at.wrapping_add(2)),
            self.word(at)
        );
        self.call(&target, &[code, arg, task]);
    }
    fn callback(&mut self, id: u16) -> i16 {
        // Named by the pointer's two words as the case set them, the way
        // the location's hook is; a program that keeps no callback has no
        // such call.
        let Some(at) = self.routines.callback else {
            return num::signed(self.call("none", &[id]));
        };
        let target = format!(
            "{:04x}:{:04x}",
            self.word(at.wrapping_add(2)),
            self.word(at)
        );
        num::signed(self.call(&target, &[id]))
    }
    fn task_stop(&mut self, i: u16) {
        self.call(self.routines.task_stop, &[i]);
    }
    fn page_release(&mut self, k: u16) {
        self.call(self.routines.page_release, &[k]);
    }
    fn band_hold(&mut self, owner: u16) {
        self.call(self.routines.band_hold, &[owner]);
    }
    fn node_block(&mut self, rec: Far, node: u16) {
        self.call(self.routines.node_block, &[rec.off, rec.seg, node]);
    }
    fn node_open(&mut self, rec: Far, node: u16) {
        self.call(self.routines.node_open, &[rec.off, rec.seg, node]);
    }
    fn interaction(&mut self, id: u16, facing: i16, scene: i16) {
        self.call(
            self.routines.interaction,
            &[id, num::unsigned(facing), num::unsigned(scene)],
        );
    }
    fn inv_give(&mut self, id: u16) -> i16 {
        num::signed(self.call(self.routines.inv_give, &[id]))
    }
    fn panel_refresh(&mut self) {
        self.call(self.routines.panel_refresh, &[]);
    }
    fn fx_reveal(&mut self) {
        self.call(self.routines.fx_reveal, &[]);
    }
}

pub(crate) fn run_case(
    title: &TraceTitle,
    image: &[u8],
    dgroup: u16,
    case: &Case,
) -> Option<Outcome> {
    let mut api = Recording::new(title.routines, image, dgroup, case);
    let parts: Vec<&str> = case.entry.split('_').collect();
    let head = parts.first()?;
    let (loc, kind) = head.strip_prefix('L')?.split_at(head.len().checked_sub(2)?);
    let seg = hex(parts.get(1)?);
    let off = hex(parts.get(2)?);
    let args: Vec<u16> = case
        .params
        .get("args")
        .map(|a| a.split(',').map(hex).collect())
        .unwrap_or_default();
    let arg = |i: usize| args.get(i).copied().unwrap_or(0);
    // `G`: a rule of the global table, which belongs to no location.
    if loc == "G" {
        let ax = (title.global_rule)(&mut api, (seg, off), arg(0))?;
        return Some(Outcome {
            diff: api.diff(),
            calls: api.calls,
            end: format!("returned ax={:04x} frames={}", num::unsigned(ax), api.frame),
        });
    }
    // `H`: the shared hook, which belongs to no location either.
    if loc == "H" {
        (title.hook)(&mut api, arg(0), arg(1), arg(2));
        return Some(Outcome {
            diff: api.diff(),
            calls: api.calls,
            end: format!("returned frames={}", api.frame),
        });
    }
    let logic = (title.logic)(loc.parse().ok()?)?;
    // The hooks and events that poll the input themselves are counted the
    // same way: every poll is a picture.
    let end = match kind {
        "h" => {
            (logic.enter)(&mut api);
            let mut guard = 0;
            while (logic.frame)(&mut api) && guard < 1000 {
                guard += 1;
            }
            (logic.leave)(&mut api);
            format!("returned frames={}", api.frame)
        }
        "k" => {
            (logic.hook)(&mut api, arg(0), arg(1), arg(2));
            format!("returned frames={}", api.frame)
        }
        "a" => {
            (logic.anim_hook?)(
                &mut api,
                Far {
                    off: arg(0),
                    seg: arg(1),
                },
                arg(2),
            );
            format!("returned frames={}", api.frame)
        }
        "r" => {
            let ax = (logic.rule)(&mut api, off, arg(0))?;
            format!("returned ax={:04x} frames={}", num::unsigned(ax), api.frame)
        }
        _ => return None,
    };
    Some(Outcome {
        diff: api.diff(),
        calls: api.calls,
        end,
    })
}
