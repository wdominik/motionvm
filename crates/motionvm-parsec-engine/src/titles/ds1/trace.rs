//! The location logic, run case by case against a stand-in engine and
//! digested.
//!
//! A case is one entry point of the original logic and one set of inputs:
//! the location, the kind of entry (`h` handler, `k` hook, `a` animation
//! hook, `r` rule handler), the routine's address, and what the data
//! segment and the engine hold when it runs. The cases are in
//! `tests/ds1_logic_cases.txt`, one head line each; they are inputs, not
//! data of the game. For each, [`Recording`] runs the logic against an
//! [`Api`] that writes down every call (target, argument words) in the
//! notation below and keeps the data segment as a plain 64 KiB image taken
//! from the program's `DGROUP`, and the case's digest folds in what the
//! logic did: the calls in order, the bytes it changed, and how it ended.
//! The digests are held against `tests/digests/ds1-logic.txt`, where every
//! line is anchored to the original routine run on the same inputs and
//! traced the same way.
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

use crate::num;
use crate::titles::parsec1994::{Api, Far};
use std::collections::BTreeMap;

/// Routines that answer with a record in `DX:AX`. The stand-in answers a
/// lookup of id `n` with `(4000h + 4n):0000`, so a record's fields land in
/// a window of their own; `ret=…:0` makes it null.
const RECORD_RETURNS: &[&str] = &[
    "0bb6:052b",
    "0bb6:05b0",
    "14dd:084a",
    "14dd:096b",
    "14dd:0a06",
    "0883:20b3",
];

/// One case: its entry and its inputs.
#[derive(Debug, Clone)]
pub(crate) struct Case {
    entry: String,
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
}

impl Recording {
    /// A stand-in over the program's data segment, with the case's inputs
    /// applied.
    pub(crate) fn new(image: &[u8], dgroup: u16, case: &Case) -> Self {
        let at = usize::from(dgroup) * 16;
        let mut ds = vec![0u8; 0x10000];
        let n = image.len().saturating_sub(at).min(0x10000);
        ds[..n].copy_from_slice(&image[at..at + n]);
        let mut r = Recording {
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
            None if RECORD_RETURNS.contains(&name) => Far {
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
        self.call("0eb9:0001", &[]);
    }
    fn visit_enter(&mut self, loc: u16, entry: u16) {
        self.call("0eb9:0207", &[loc, entry]);
    }
    fn visit_finish_entry(&mut self) {
        self.call("0eb9:00b8", &[]);
    }
    fn dispatch(&mut self) {
        self.call("0eb9:0fce", &[]);
    }
    fn frame_pass(&mut self) {
        self.call("0eb9:01de", &[]);
    }
    fn visit_leave(&mut self) {
        self.call("0eb9:017a", &[]);
    }
    fn slot_unload(&mut self) {
        self.call("0766:0d34", &[]);
    }
    fn hover_on(&mut self) {
        self.call("0eb9:1917", &[]);
    }
    fn input_poll(&mut self) {
        self.call("179e:01b0", &[]);
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
            self.poke(0x1F48, 0x00FF);
        }
    }
    fn pointer_hide(&mut self) {
        self.call("179e:0882", &[]);
    }
    fn pointer_unfreeze(&mut self) {
        self.call("179e:091a", &[]);
    }
    fn key_flush(&mut self) {
        self.call("179e:071d", &[]);
    }
    fn scene_play(&mut self, n: i16) -> u16 {
        self.call("0cfd:172f", &[num::unsigned(n)])
    }
    fn task_arm(&mut self, i: u16) {
        self.call("0cfd:1a2e", &[i]);
    }
    fn task_start(&mut self, i: u16) {
        self.call("0cfd:1a3f", &[i]);
    }
    fn scene_sequence(&mut self, counter: Far, scenes: &[i16]) {
        let mut args = vec![counter.off, counter.seg];
        args.extend(scenes.iter().map(|&n| num::unsigned(n)));
        args.push(0xFFFF);
        self.call("0eb9:1691", &args);
    }
    fn slot_set_object(&mut self, slot: u16, code: u16, rec: Far) {
        self.call("0cfd:1a8c", &[slot, code, rec.off, rec.seg]);
    }
    fn slot_set_figure(&mut self, slot: u16, code: u16, rec: Far) {
        self.call("0cfd:1ad0", &[slot, code, rec.off, rec.seg]);
    }
    fn obj_find(&mut self, id: u16) -> Far {
        self.record("0bb6:052b", id, &[id])
    }
    fn obj_wake(&mut self, id: u16) -> Far {
        self.record("0883:20b3", id, &[id])
    }
    fn obj_show(&mut self, id: u16) -> Far {
        self.record("14dd:084a", id, &[id])
    }
    fn obj_anim(&mut self, rec: Far, k: i16) {
        self.call("0883:1f82", &[rec.off, rec.seg, num::unsigned(k)]);
    }
    fn obj_anim_id(&mut self, id: u16, k: i16) {
        self.call("0883:2099", &[id, num::unsigned(k)]);
    }
    fn obj_hide(&mut self, id: u16) -> Far {
        self.record("14dd:096b", id, &[id])
    }
    fn obj_state(&mut self, id: u16) -> i16 {
        num::signed(self.call("14dd:07e7", &[id]))
    }
    fn objs_save_state(&mut self, out: Far) -> u16 {
        self.call("14dd:068a", &[out.off, out.seg])
    }
    fn walk_to_node(&mut self, node: u16) -> i16 {
        num::signed(self.call("14dd:0a56", &[node]))
    }
    fn anim_pass(&mut self, delta: i16) {
        self.call("0883:0305", &[num::unsigned(delta)]);
    }
    fn room_redraw(&mut self) {
        self.call("0bb6:027a", &[]);
    }
    fn walker_snap(&mut self, rec: Far, node: u16, facing: i16) -> i16 {
        num::signed(self.call(
            "0a9f:0c59",
            &[rec.off, rec.seg, node, num::unsigned(facing)],
        ))
    }
    fn walker_detach(&mut self, rec: Far) {
        self.call("0a9f:0f42", &[rec.off, rec.seg]);
    }
    fn input_lock(&mut self) {
        self.call("0eb9:188a", &[]);
    }
    fn slot_set_raw(&mut self, slot: u16, code: u16, w1: u16, w2: u16) {
        self.call("0cfd:1b14", &[slot, code, w1, w2]);
    }
    fn obj_show_frame(&mut self, id: u16, k: i16) -> Far {
        self.record("14dd:0a06", id, &[id, num::unsigned(k)])
    }
    fn movmem(&mut self, src: Far, dst: Far, n: u16) {
        self.call("1ee7:0022", &[src.off, src.seg, dst.off, dst.seg, n]);
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
        self.call("1ee7:0004", &[dst.off, dst.seg, val, n]);
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
        self.call_text(format!("1ee7:0022(stk,stk,{})", a.join(",")));
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
        self.call("1ef8:0720", &[p.off, p.seg])
    }
    fn mem_free(&mut self, p: Far) -> u16 {
        self.call("1ef8:0840", &[p.off, p.seg])
    }
    fn entry_door_close(&mut self) {
        self.call("0eb9:149d", &[]);
    }
    fn input_unlock(&mut self) {
        self.call("0eb9:18a4", &[]);
    }
    fn face(&mut self, dir: i16) {
        self.call("14dd:0a77", &[num::unsigned(dir)]);
    }
    fn pointer_shape_resync(&mut self) {
        self.call("179e:1017", &[]);
    }
    fn mover_find(&mut self, id: u16) -> Far {
        self.record("0bb6:05b0", id, &[id])
    }
    fn walker_bind_graph(&mut self, rec: Far, graph: u16, node: u16, facing: i16) -> i16 {
        num::signed(self.call(
            "0a9f:0ea6",
            &[rec.off, rec.seg, graph, node, num::unsigned(facing)],
        ))
    }
    fn walker_unlink(&mut self, rec: Far) {
        self.call("0a9f:0e7f", &[rec.off, rec.seg]);
    }
    fn scene_enqueue(&mut self, n: i16) -> u16 {
        self.call("0cfd:1781", &[num::unsigned(n)])
    }
    fn ambient_play(&mut self, lo: i16, hi: i16, flags: u16) -> u16 {
        self.call("0cfd:19a4", &[num::unsigned(lo), num::unsigned(hi), flags])
    }
    fn pal_set(&mut self, rgb: Far, start: u16, count: u16) {
        self.call("0590:09e2", &[rgb.off, rgb.seg, start, count]);
    }
    fn song_load(&mut self, n: u16) -> Far {
        let ax = self.call("1197:000d", &[n]);
        match self.rets.get("1197:000d") {
            Some(&v) => Far { off: v, seg: 0 },
            None => Far { off: ax, seg: 0 },
        }
    }
    fn music_play(&mut self, res: Far, a: i16) {
        self.call("0590:06a9", &[res.off, res.seg, num::unsigned(a)]);
    }
    fn music_play_fade(&mut self, res: Far, a: i16, b: u16) {
        self.call("0590:07a5", &[res.off, res.seg, num::unsigned(a), b]);
    }
    fn music_fade(&mut self, n: i16) {
        self.call("0590:08a9", &[num::unsigned(n)]);
    }
    fn music_stop(&mut self) {
        self.call("0590:08fa", &[]);
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
        self.call("0eb9:18f4", &[]);
    }
    fn play_sample(&mut self, k: u16) {
        self.call("0eb9:1949", &[k]);
    }
    fn walker_walk_to(&mut self, rec: Far, node: u16, facing: i16) -> i16 {
        num::signed(self.call(
            "0a9f:0617",
            &[rec.off, rec.seg, node, num::unsigned(facing)],
        ))
    }
    fn inv_take(&mut self, id: u16) -> i16 {
        num::signed(self.call("0c3f:09e1", &[id]))
    }
    fn farptr_count(&mut self, row: Far) -> u16 {
        self.call("1ebd:0000", &[row.off, row.seg])
    }
    fn inv_has(&mut self, id: u16) -> i16 {
        num::signed(self.call("0c3f:0b06", &[id]))
    }
    fn random(&mut self, lo: u16, hi: u16) -> u16 {
        self.call("1eb5:0028", &[lo, hi])
    }
    fn script_exit(&mut self, k: u16) {
        self.call("1586:05e1", &[k]);
    }
    fn inv_give(&mut self, id: u16) -> i16 {
        num::signed(self.call("0c3f:095c", &[id]))
    }
    fn panel_refresh(&mut self) {
        self.call("0c3f:005c", &[]);
    }
    fn fx_reveal(&mut self) {
        self.call("10b8:03ba", &[]);
    }
}

/// Runs one case. `None` for an entry the logic has no routine for: a
/// kind other than the four, or an animation hook or rule handler the
/// location does not have.
pub(crate) fn run_case(image: &[u8], dgroup: u16, case: &Case) -> Option<Outcome> {
    let mut api = Recording::new(image, dgroup, case);
    let parts: Vec<&str> = case.entry.split('_').collect();
    let head = parts.first()?;
    let (loc, kind) = head.strip_prefix('L')?.split_at(head.len().checked_sub(2)?);
    let logic = super::logic::logic(loc.parse().ok()?)?;
    let off = hex(parts.get(2)?);
    let args: Vec<u16> = case
        .params
        .get("args")
        .map(|a| a.split(',').map(hex).collect())
        .unwrap_or_default();
    let arg = |i: usize| args.get(i).copied().unwrap_or(0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use motionvm_parsec_testutil::digest::Digest;

    /// The cases, in the order the file lists them.
    fn cases() -> Vec<Case> {
        parse(include_str!("../../../tests/ds1_logic_cases.txt"))
    }

    /// Every case has a name of its own: two heads with one name would
    /// share a line of the table.
    #[test]
    fn every_case_has_a_name_of_its_own() {
        let cases = cases();
        let names: std::collections::BTreeSet<String> = cases.iter().map(Case::name).collect();
        assert_eq!(names.len(), cases.len());
        assert_eq!(cases.len(), 1298);
    }

    /// The location logic does what the original routine does in every
    /// case: the same calls with the same arguments in the same order, the
    /// same bytes changed and the same end, held as one digest per case
    /// against the table, whose lines are anchored to the original.
    ///
    /// Needs the program's `DGROUP`, for the bytes the logic reads, and so
    /// the game's files — `MOTIONVM_GAMEDATA_DS1` — and skips without them.
    #[test]
    fn the_location_logic_does_what_the_original_does_in_every_case() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let exe = motionvm_parsec_testutil::game_file(&dir, "XP01.EXE");
        let unpacked =
            motionvm_parsec_formats::Unpacked::parse(&std::fs::read(exe).expect("XP01.EXE reads"))
                .expect("XP01.EXE unpacks");
        let dgroup = u16::try_from(unpacked.dgroup().expect("a DGROUP") / 16).unwrap();
        let table = motionvm_parsec_testutil::Digests::of(env!("CARGO_MANIFEST_DIR"), "ds1-logic");
        for case in &cases() {
            let outcome = run_case(&unpacked.image, dgroup, case)
                .unwrap_or_else(|| panic!("{}: the logic has no routine for it", case.entry));
            let digest = Digest::new().bytes(outcome.text().as_bytes()).value();
            table.check(&case.name(), digest);
        }
    }
}
