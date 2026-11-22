//! What Dunkle Schatten's location logic asks of the engine.
//!
//! The game's logic — a location's handler, its hooks and its own rule
//! handlers — is compiled C that talks to the engine through far calls
//! alone, and it reads and writes globals in `DGROUP`. [`Api`] is that
//! boundary: one method per engine routine — 64 far addresses, as the
//! recording stand-in in `trace.rs` names them — with the routine's own
//! arguments in the original's order and its own return value, and
//! accessors for the data segment, the far bytes the logic reads and the
//! handler's own locals. *Dunkle Schatten*'s program calls three engine
//! routines more from its game code — `0CFDh:18C2h`, `0883h:000Fh` and
//! `14DDh:074Bh` — all of them from global rules that read the hotspot or
//! the object record a choice went through, which this trait does not
//! reach; those rules are read over the runtime itself.
//!
//! The logic is written against this trait and nothing else, so the same
//! code runs against the engine and against a recording stand-in that
//! compares every call with a trace of the original routine (`trace.rs`).
//!
//! Each method carries the routine's address and what it does.

/// A far pointer the logic only passes on: a record the engine found
/// (`0BB6h:052Bh` and friends), or one of its own records in `DGROUP`.
/// The logic never builds one from parts except for `DGROUP` records,
/// which is what [`Api::ds`] is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Far {
    /// Offset word, the one pushed last and found at `[bp+6]`.
    pub off: u16,
    /// Segment word.
    pub seg: u16,
}

impl Far {
    /// The null pointer a lookup answers with when it finds nothing.
    pub(crate) const NULL: Far = Far { off: 0, seg: 0 };
}

/// The engine as the location logic sees it.
pub(crate) trait Api {
    // --- the data segment -------------------------------------------------

    /// The byte at `DS:addr`.
    fn byte(&self, addr: u16) -> u8;
    /// Writes the byte at `DS:addr`.
    fn set_byte(&mut self, addr: u16, v: u8);
    /// The word at `DS:addr`.
    fn word(&self, addr: u16) -> u16;
    /// Writes the word at `DS:addr`.
    fn set_word(&mut self, addr: u16, v: u16);
    /// A far pointer into `DGROUP`, as `push ds ; push off` builds it.
    fn ds(&self, off: u16) -> Far;

    // --- the visit: 0EB9h, 0766h -----------------------------------------

    /// `0EB9h:0001h`: the visit's own setup — the figure's graphics, the
    /// location's slot loaded, the object states taken from its record.
    fn visit_setup(&mut self);
    /// `0EB9h:0207h(loc, entry)`: the shared entry — mode block cleared,
    /// the location's rules, subjects and hooks installed, the entry door
    /// opened, the figure placed on the entry node, the tasks cleared.
    fn visit_enter(&mut self, loc: u16, entry: u16);
    /// `0EB9h:00B8h`: the entry finished — first picture, the entry effect
    /// of the location's kind, ramp, pointer, music, tasks enabled.
    fn visit_finish_entry(&mut self);
    /// `0EB9h:0FCEh`: one frame of input dispatched.
    fn dispatch(&mut self);
    /// `0EB9h:01DEh`: one frame pass — the ticks taken, status, tasks,
    /// walker and animator advanced, the picture drawn.
    fn frame_pass(&mut self);
    /// `0EB9h:017Ah`: leaving — pointer hidden, mode reset, tasks 0 and 1
    /// stopped, the transition effect and music stopped.
    fn visit_leave(&mut self);
    /// `0766h:0D34h`: the slot just left is released.
    fn slot_unload(&mut self);
    /// `0EB9h:18F4h`: hover off, the status refreshed, the pointer hidden.
    fn hover_off(&mut self);
    /// `0EB9h:1949h(k)`: sample `k` of the table at `DS:1EF4h` played.
    fn play_sample(&mut self, k: u16);
    /// `0EB9h:188Ah`: the input taken over by the script — `[289Bh]` bit 1,
    /// the hover forgotten, the band held.
    fn input_lock(&mut self);
    /// `0EB9h:1917h`: hover on again, the status refreshed, the key buffer
    /// flushed, the input polled twice, the pointer shown.
    fn hover_on(&mut self);

    // --- input and pointer: 179Eh -----------------------------------------

    /// `179Eh:01B0h`: mouse and keyboard polled.
    fn input_poll(&mut self);
    /// `179Eh:0882h`: the pointer taken off the screen and frozen.
    fn pointer_hide(&mut self);
    /// `179Eh:091Ah`: the frozen pointer catches up with the mouse.
    fn pointer_unfreeze(&mut self);
    /// `179Eh:071Dh`: the keyboard ring buffer emptied.
    fn key_flush(&mut self);

    // --- scenes, tasks and subject slots: 0CFDh ---------------------------

    /// `0CFDh:172Fh(n)`: scene `n` bound to task 0 and started at once.
    /// Answers word `+2` of the scene blob (its sentence count).
    fn scene_play(&mut self, n: i16) -> u16;
    /// `0CFDh:1A2Eh(i)`: task `i` armed.
    fn task_arm(&mut self, i: u16);
    /// `0CFDh:1A3Fh(i)`: an armed task `i` started.
    fn task_start(&mut self, i: u16);
    /// `0EB9h:1691h(counter, scenes…, -1)`: the next scene of a sequence;
    /// the byte at `counter` steps up to the last one.
    fn scene_sequence(&mut self, counter: Far, scenes: &[i16]);
    /// `0CFDh:1B14h(slot, code, w1, w2)`: a subject slot written as it is;
    /// for a code of kind 0 the two words are a point.
    fn slot_set_raw(&mut self, slot: u16, code: u16, w1: u16, w2: u16);
    /// `0CFDh:1A8Ch(slot, code, rec)`: subject slot `slot` names an object.
    fn slot_set_object(&mut self, slot: u16, code: u16, rec: Far);
    /// `0CFDh:1AD0h(slot, code, rec)`: subject slot `slot` names a walker.
    fn slot_set_figure(&mut self, slot: u16, code: u16, rec: Far);

    // --- objects: 0883h, 0BB6h, 14DDh -------------------------------------

    /// `0BB6h:052Bh(id)`: the object with this id, or null.
    fn obj_find(&mut self, id: u16) -> Far;
    /// `0883h:20B3h(id)`: the object shown and set on animation 0.
    fn obj_wake(&mut self, id: u16) -> Far;
    /// `14DDh:084Ah(id)`: the object shown.
    fn obj_show(&mut self, id: u16) -> Far;
    /// `14DDh:0A06h(id, k)`: the object shown on its frame `k`; a negative
    /// frame hides it.
    fn obj_show_frame(&mut self, id: u16, k: i16) -> Far;
    /// `0883h:1F82h(rec, k)`: the object at `rec` set on animation `k`.
    fn obj_anim(&mut self, rec: Far, k: i16);
    /// `0883h:2099h(id, k)`: the object set on animation `k`.
    fn obj_anim_id(&mut self, id: u16, k: i16);
    /// `14DDh:096Bh(id)`: the object hidden.
    fn obj_hide(&mut self, id: u16) -> Far;
    /// `14DDh:07E7h(id)`: whether the object is live: −1 not found, else a
    /// state the callers test for non-zero.
    fn obj_state(&mut self, id: u16) -> i16;
    /// `14DDh:068Ah(out)`: one state byte per object written to `out`;
    /// answers the count.
    fn objs_save_state(&mut self, out: Far) -> u16;
    /// `14DDh:0A56h(node)`: the player walks to node `node`; answers what
    /// `0A9Fh:0617h` answered.
    fn walk_to_node(&mut self, node: u16) -> i16;
    /// `0883h:0305h(delta)`: one animator pass.
    fn anim_pass(&mut self, delta: i16);
    /// `0BB6h:027Ah`: the room redrawn.
    fn room_redraw(&mut self);

    // --- walker, inventory, panel, effects: 0A9Fh, 0C3Fh, 10B8h ----------

    /// `0A9Fh:0C59h(rec, node, facing)`: the walker set on a node without
    /// walking. Answers 1, or 0 when there is no graph or node.
    fn walker_snap(&mut self, rec: Far, node: u16, facing: i16) -> i16;
    /// `0A9Fh:0617h(rec, node, facing)`: the walker walks to a node.
    /// Answers 2 when a walk started, 1 when a running leg was retargeted,
    /// 0 when there is no way, −1 when the first edge is blocked.
    fn walker_walk_to(&mut self, rec: Far, node: u16, facing: i16) -> i16;
    /// `0A9Fh:0F42h(rec)`: the walker taken out of the draw chain.
    fn walker_detach(&mut self, rec: Far);
    /// `0C3Fh:0B06h(id)`: whether the item is carried, 1 or 0.
    fn inv_has(&mut self, id: u16) -> i16;
    /// `0C3Fh:09E1h(id)`: the item taken away; 1 when it was carried.
    fn inv_take(&mut self, id: u16) -> i16;
    /// `0C3Fh:095Ch(id)`: the item given; 1 when taken in.
    fn inv_give(&mut self, id: u16) -> i16;
    /// `0C3Fh:005Ch`: the panel's own pass.
    fn panel_refresh(&mut self);
    /// `10B8h:03BAh`: the reveal effect over the playfield, synchronous.
    fn fx_reveal(&mut self);

    // --- the rest ----------------------------------------------------------

    /// A word written through a far pointer outside `DGROUP` (`mov es, …`),
    /// as the map's developer keys write into the location table.
    fn set_far_word(&mut self, at: Far, v: u16) {
        let [lo, hi] = v.to_le_bytes();
        self.set_far_byte(at, lo);
        self.set_far_byte(
            Far {
                off: at.off.wrapping_add(1),
                seg: at.seg,
            },
            hi,
        );
    }
    /// A byte written through a far pointer (`mov es, … ; mov es:[…], …`):
    /// into `DGROUP`, or into the program's own tables such as the
    /// location table at `204Bh`.
    fn set_far_byte(&mut self, at: Far, v: u8);
    /// A byte read through a far pointer.
    fn far_byte(&mut self, at: Far) -> u8;
    /// Local `i` of the running handler's own stack frame — a register or
    /// `[bp-n]` the original keeps from its entry across its frame loop.
    /// Each visit starts them at zero.
    fn local(&self, i: usize) -> u16;
    /// Writes local `i`.
    fn set_local(&mut self, i: usize, v: u16);
    /// A word read through a far pointer.
    fn far_word(&mut self, at: Far) -> u16 {
        let lo = self.far_byte(at);
        let hi = self.far_byte(Far {
            off: at.off.wrapping_add(1),
            seg: at.seg,
        });
        u16::from_le_bytes([lo, hi])
    }
    /// `1EE7h:0022h(src, dst, n)`: `n` bytes copied forwards (`movmem`).
    fn movmem(&mut self, src: Far, dst: Far, n: u16);
    /// `1EE7h:0004h(dst, val, n)`: `n` bytes set (`setmem`).
    fn setmem(&mut self, dst: Far, val: u16, n: u16);
    /// `1EE7h:0022h(src, dst, n)` with the source a buffer on the
    /// handler's own stack (`push ss ; lea ax, [bp-n]`): the bytes the logic
    /// built there, copied to `dst`.
    fn movmem_from_local(&mut self, src: &[u8], dst: Far);
    /// `1EF8h:0720h(p)`: a heap routine handed the far pointer `p`
    /// (`lds si, [bp+6]`, `0x1F6A6`); what it does to the block is not read.
    fn heap_0720(&mut self, p: Far) -> u16;
    /// `1EF8h:0840h(p)`: a heap block freed; 0, or 5 for a bad block.
    fn mem_free(&mut self, p: Far) -> u16;

    // --- more of the visit, the walker and the objects ---------------------

    /// `0EB9h:149Dh`: the remembered entry door plays animation 0 and is
    /// forgotten.
    fn entry_door_close(&mut self);
    /// `0EB9h:18A4h`: the input given back (`[289Bh]` bit 1 cleared), the
    /// hover forgotten, the band released.
    fn input_unlock(&mut self);
    /// `14DDh:0A77h(dir)`: the player turns to face `dir`.
    fn face(&mut self, dir: i16);
    /// `179Eh:1017h`: the pointer's shape rebuilt.
    fn pointer_shape_resync(&mut self);
    /// `0BB6h:05B0h(id)`: a record that can walk, by id — a person's —
    /// or null.
    fn mover_find(&mut self, id: u16) -> Far;
    /// `0A9Fh:0EA6h(rec, graph, node, facing)`: the record bound to walk
    /// graph `graph` and set on its node; 1, or 0.
    fn walker_bind_graph(&mut self, rec: Far, graph: u16, node: u16, facing: i16) -> i16;
    /// `0A9Fh:0E7Fh(rec)`: the record's animation stopped and it taken out
    /// of its layer's draw chain.
    fn walker_unlink(&mut self, rec: Far);

    // --- scenes and tasks, more ---------------------------------------------

    /// `0CFDh:1781h(n)`: scene `n` queued behind a busy task 0, or bound
    /// at once. Answers 0 for a full queue, `FFFFh` for queued.
    fn scene_enqueue(&mut self, n: i16) -> u16;
    /// `0CFDh:19A4h(lo, hi, flags)`: one of scenes `lo..=hi` rolled and
    /// bound to task 1, repeated while `flags` bit 0 is set.
    fn ambient_play(&mut self, lo: i16, hi: i16, flags: u16) -> u16;

    // --- palette and music: 0590h, 1197h ------------------------------------

    /// `0590h:09E2h(rgb, start, count)`: `count` DAC entries from `start`.
    fn pal_set(&mut self, rgb: Far, start: u16, count: u16);
    /// `1197h:000Dh(n)`: entry `n` of `PSMP2L.DAT` loaded; null with the
    /// music off.
    fn song_load(&mut self, n: u16) -> Far;
    /// `0590h:06A9h(res, a)`: a music resource started.
    fn music_play(&mut self, res: Far, a: i16);
    /// `0590h:07A5h(res, a, b)`: the same, faded in over `b`.
    fn music_play_fade(&mut self, res: Far, a: i16, b: u16);
    /// `0590h:08A9h(n)`: the music faded out.
    fn music_fade(&mut self, n: i16);
    /// `0590h:08FAh`: the music stopped.
    fn music_stop(&mut self);

    /// `1EB5h:0028h(lo, hi)`: a roll of the shared generator, `lo..=hi`.
    fn random(&mut self, lo: u16, hi: u16) -> u16;
    /// `1EBDh:0000h(row)`: the far pointers in a null-terminated row.
    fn farptr_count(&mut self, row: Far) -> u16;
    /// `1586h:05E1h(k)`: the location left through exit `k`, as the mode
    /// machine's gate does it.
    fn script_exit(&mut self, k: u16);
}

/// The segment word of a handle the engine gives out for an object record.
/// The logic only passes such a pointer on, so the engine is free to say
/// which object it means in the offset word.
const OBJECT_HANDLE: u16 = 0xF0B1;
/// The segment word of a handle for a record that walks besides the
/// player's: one of the room's 30h-byte records, whose place in the table
/// times `30h` is the offset word, so the offset plus a field offset names
/// that field of that record.
const MOVER_HANDLE: u16 = 0xF0B2;
/// The segment word of the handle for the loaded room's header. The engine
/// keeps the room in its own shape, so what lies behind it is not served.
const ROOM_HANDLE: u16 = 0xF0B3;

impl Far {
    /// The handle for the loaded room's header.
    pub(crate) fn room() -> Far {
        Far {
            off: 0,
            seg: ROOM_HANDLE,
        }
    }
}

impl Far {
    /// The handle for object `id`.
    pub(crate) fn object(id: u16) -> Far {
        Far {
            off: id,
            seg: OBJECT_HANDLE,
        }
    }

    /// The object a handle names, if it is one.
    pub(crate) fn object_id(self) -> Option<u16> {
        (self.seg == OBJECT_HANDLE).then_some(self.off)
    }

    /// The handle for the room's 30h-byte record `i`.
    pub(crate) fn mover(i: usize) -> Far {
        Far {
            off: num::word(i * MOVER_RECORD),
            seg: MOVER_HANDLE,
        }
    }

    /// The record and the field offset a pointer into one names.
    pub(crate) fn mover_field(self) -> Option<(usize, u16)> {
        (self.seg == MOVER_HANDLE).then(|| {
            let stride = num::word(MOVER_RECORD);
            (usize::from(self.off / stride), self.off % stride)
        })
    }

    /// The record a handle names, when it is a record's own address.
    pub(crate) fn mover_index(self) -> Option<usize> {
        self.mover_field().filter(|&(_, f)| f == 0).map(|(i, _)| i)
    }
}

use crate::num;
use motionvm_parsec_formats::rdv::MOVER_RECORD;

/// Where one build keeps the globals its location logic shares with the
/// engine. Every other address the logic touches is the logic's own and
/// lives in the runtime's copy of `DGROUP` ([`Adventure::ds`]).
#[derive(Debug, Clone, Copy)]
pub(crate) struct DsMap {
    /// The entry id; non-zero again once an exit is asked for.
    pub entry: u16,
    /// The location the main loop dispatches on.
    pub location: u16,
    /// The location the running handler entered.
    pub here: u16,
    /// The player's record, `30h` bytes: the walker the runtime keeps
    /// (see `player.rs`).
    pub walker: u16,
    /// The scene machine's hundredths counter (clock `+0Eh`).
    pub hundredths: u16,
    /// The ticks the frame pass has not taken yet (clock `+2`).
    pub ticks_taken: u16,
    /// The first of 32 bytes of object states per location.
    pub object_states: u16,
    /// The far pointers a `02 n` text control substitutes.
    pub substitutions: u16,
    /// How many far pointers that row holds.
    pub substitution_count: u16,
    /// The mode block's verb (`[288Dh]` in Dunkle Schatten).
    pub verb: u16,
    /// The mode block's first object (`[2891h]`).
    pub object: u16,
    /// The first object's class (`[2893h]`).
    pub object_class: u16,
    /// The mode block's second object (`[2895h]`).
    pub second: u16,
    /// The second object's class (`[2897h]`).
    pub second_class: u16,
    /// The first of the dialogue's flag bytes, one per flag.
    pub flags: u16,
    /// The input event record the poll fills (`DS:184Eh`): its first word.
    pub input_event: u16,
    /// The gate the mode machine asks, a far pointer: offset, then segment
    /// (`[28BBh]`, `[28BDh]`).
    pub gate: u16,
    /// The loaded room's header, a far pointer (`[2851h]`).
    pub room_header: u16,
    /// The door the player came in by, until it has swung shut (`[1EDEh]`).
    pub entry_door: u16,
    /// The carried list: its count byte, the first visible slot's byte,
    /// and the first byte of the list.
    pub inventory: (u16, u16, u16),
    /// The segment of the far buffer a saved game is held in.
    pub save_buffer: u16,
    /// Where the routines the gate names are linked.
    pub gates: GateMap,
    /// The choice menu's record, its geometry and its inks.
    pub menu: u16,
}

use super::mode::{Gate, GateMap};
use super::{Adventure, Event};
use crate::titles::subject::Subject;

impl Adventure {
    pub(super) fn map(&self) -> DsMap {
        self.ds_map
    }

    /// The location record a pointer into the object-state rows names.
    fn object_state_row(&self, out: Far) -> Option<i32> {
        let m = self.map();
        let d = out.off.checked_sub(m.object_states)?;
        (out.seg == self.dgroup && d % 32 == 0).then_some(i32::from(d / 32))
    }

    /// Substitution `n` is set again from the far pointer the logic left
    /// in its row.
    fn refresh_substitution(&mut self, n: usize) {
        let m = self.map();
        let at = m.substitutions.wrapping_add(num::word(n) * 4);
        let off = usize::from(self.ds_word(at));
        let seg = usize::from(self.ds_word(at.wrapping_add(2)));
        let text = self
            .image
            .c_string(seg * 16 + off)
            .map(|s| motionvm_parsec_formats::to_cp437(&s))
            .unwrap_or_default();
        if self.substitutions.len() <= n {
            self.substitutions.resize(n + 1, Vec::new());
        }
        self.substitutions[n] = text;
    }

    fn ds_word(&self, addr: u16) -> u16 {
        u16::from_le_bytes([
            self.ds[usize::from(addr)],
            self.ds[usize::from(addr.wrapping_add(1))],
        ])
    }
}

impl Api for Adventure {
    fn byte(&self, addr: u16) -> u8 {
        // A byte of a word the engine shares is that word's low or high
        // half; every other byte is the data segment's own, under the
        // records the engine keeps in its own shape.
        if self.is_shared(addr) {
            self.word(addr).to_le_bytes()[0]
        } else if self.is_shared(addr.wrapping_sub(1)) {
            self.word(addr.wrapping_sub(1)).to_le_bytes()[1]
        } else {
            self.mem_byte(addr)
        }
    }

    fn set_byte(&mut self, addr: u16, v: u8) {
        if self.is_shared(addr) {
            let [_, hi] = self.word(addr).to_le_bytes();
            self.set_word(addr, u16::from_le_bytes([v, hi]));
        } else if self.is_shared(addr.wrapping_sub(1)) {
            let at = addr.wrapping_sub(1);
            let [lo, _] = self.word(at).to_le_bytes();
            self.set_word(at, u16::from_le_bytes([lo, v]));
        } else {
            self.set_mem_byte(addr, v);
        }
    }

    fn word(&self, addr: u16) -> u16 {
        let m = self.map();
        if addr == m.entry {
            self.entry_word
        } else if addr == m.location {
            self.location_word
        } else if addr == m.here {
            self.here_word
        } else if addr == m.hundredths {
            self.clock.hundredths_1c82()
        } else if addr == m.ticks_taken {
            self.clock.ticks
        } else if addr == m.verb {
            u16::from(self.verb)
        } else if addr == m.object {
            self.object1
        } else if addr == m.object_class {
            self.object2
        } else if addr == m.second {
            self.second_object.0
        } else if addr == m.second_class {
            self.second_object.1
        } else if addr == m.entry_door {
            num::unsigned(self.entry_door)
        } else if addr == m.gate || addr == m.gate.wrapping_add(2) {
            let (seg, off) = m.gates.far(self.gate);
            if addr == m.gate { off } else { seg }
        } else if addr == m.room_header || addr == m.room_header.wrapping_add(2) {
            let room = Far::room();
            if addr == m.room_header {
                room.off
            } else {
                room.seg
            }
        } else if addr == m.input_event {
            // There is no event record: input reaches the runtime as host
            // events, and the poll leaves nothing here for the logic.
            0
        } else {
            u16::from_le_bytes([self.mem_byte(addr), self.mem_byte(addr.wrapping_add(1))])
        }
    }

    fn set_word(&mut self, addr: u16, v: u16) {
        let m = self.map();
        let a = usize::from(addr);
        if addr == m.entry {
            self.entry_word = v;
        } else if addr == m.location {
            self.location_word = v;
        } else if addr == m.here {
            self.here_word = v;
        } else if addr == m.hundredths {
            self.clock.set_1c82(v);
        } else if addr == m.verb {
            self.verb = num::low8(v);
        } else if addr == m.object {
            self.object1 = v;
        } else if addr == m.object_class {
            self.object2 = v;
        } else if addr == m.second {
            self.second_object.0 = v;
        } else if addr == m.second_class {
            self.second_object.1 = v;
        } else if addr == m.entry_door {
            self.entry_door = num::signed(v);
        } else if addr == m.input_event
            || addr == m.room_header
            || addr == m.room_header.wrapping_add(2)
        {
        } else if addr == m.gate || addr == m.gate.wrapping_add(2) {
            // The two words name the far routine; the engine knows two.
            let [lo, hi] = v.to_le_bytes();
            self.ds[a] = lo;
            self.ds[usize::from(addr.wrapping_add(1))] = hi;
            let off = self.ds_word(m.gate);
            let seg = self.ds_word(m.gate.wrapping_add(2));
            self.gate = m.gates.gate((seg, off)).unwrap_or_else(|| {
                self.events.push(Event::Unbuilt(
                    "gate",
                    (u32::from(seg) << 16) | u32::from(off),
                ));
                Gate::None
            });
        } else if addr == m.ticks_taken {
            if self.effects.busy() {
                self.ticks_after_effect = Some(v);
            } else {
                self.clock.ticks = v;
            }
        } else {
            let [lo, hi] = v.to_le_bytes();
            self.set_mem_byte(addr, lo);
            self.set_mem_byte(addr.wrapping_add(1), hi);
            let subs = m.substitutions;
            let n = usize::from(m.substitution_count);
            if addr >= subs && usize::from(addr - subs) < n * 4 {
                self.refresh_substitution(usize::from(addr - subs) / 4);
            }
        }
    }

    fn ds(&self, off: u16) -> Far {
        Far {
            off,
            seg: self.dgroup,
        }
    }

    fn visit_setup(&mut self) {
        // A location that cannot be loaded leaves the last one up; the
        // original has no such location in its table.
        let _ = self.visit_setup_impl();
    }
    fn visit_enter(&mut self, loc: u16, entry: u16) {
        self.visit_enter_impl(i32::from(num::signed(loc)), num::signed(entry));
    }
    fn visit_finish_entry(&mut self) {
        self.visit_finish_entry_impl();
    }
    fn dispatch(&mut self) {
        Adventure::dispatch(self);
    }
    fn frame_pass(&mut self) {
        // The options opened inside this frame's dispatch: in the original
        // the frame goes on only after they close.
        if !self.options_up() {
            Adventure::frame_pass(self);
        }
    }
    fn visit_leave(&mut self) {
        self.visit_leave_impl();
    }
    fn slot_unload(&mut self) {
        self.slot_release();
    }
    fn hover_off(&mut self) {
        self.hover_forgotten();
    }
    fn hover_on(&mut self) {
        self.hover_reset();
    }
    fn input_lock(&mut self) {
        Adventure::input_locked(self);
    }
    fn play_sample(&mut self, k: u16) {
        self.effect_1949(num::low8(k));
    }

    fn input_poll(&mut self) {
        // A press reaches the runtime latched, at the start of the frame
        // after it (`crate::mouse`); the poll's own work on the pointer is
        // the timeout's reload.
        self.pointer_poll();
    }
    fn pointer_hide(&mut self) {
        self.pointer_off();
    }
    fn pointer_unfreeze(&mut self) {
        self.pointer_thaw();
    }
    fn key_flush(&mut self) {
        // `179Eh:0533h` empties the library's ring at `cs:450h`. Host keys
        // wait in the BIOS's type-ahead buffer in front of the library
        // (`crate::keys`) and reach it one a frame, so the library keeps no
        // ring here and there is nothing of its own to empty; whether the
        // original's flush reaches past its ring into the BIOS's is not
        // read.
    }

    fn scene_play(&mut self, n: i16) -> u16 {
        self.scene_load(n)
    }
    fn scene_sequence(&mut self, counter: Far, scenes: &[i16]) {
        Adventure::scene_sequence(self, usize::from(counter.off), scenes);
    }
    fn task_arm(&mut self, i: u16) {
        Adventure::task_arm(self, usize::from(i));
    }
    fn task_start(&mut self, i: u16) {
        Adventure::task_start(self, usize::from(i));
    }
    fn slot_set_object(&mut self, slot: u16, code: u16, rec: Far) {
        match rec.object_id() {
            Some(id) => self
                .subjects
                .set(usize::from(slot), code, Subject::Object(num::low8(id))),
            None => self.subjects.clear(usize::from(slot), code),
        }
    }
    fn slot_set_raw(&mut self, slot: u16, code: u16, w1: u16, w2: u16) {
        // Only a point (kind 0) is ever written this way by the logic.
        self.subjects
            .set_point(usize::from(slot), code, num::signed(w1), num::signed(w2));
    }
    fn slot_set_figure(&mut self, slot: u16, code: u16, rec: Far) {
        if rec == self.ds(self.map().walker) {
            self.subjects.set(usize::from(slot), code, Subject::Player);
        } else if let Some(i) = rec.mover_index().filter(|&i| i < self.movers.len()) {
            self.subjects
                .set(usize::from(slot), code, Subject::Mover(i));
        } else {
            self.subjects.clear(usize::from(slot), code);
        }
    }

    fn obj_find(&mut self, id: u16) -> Far {
        match self.object_index(num::low8(id)) {
            Some(_) => Far::object(id),
            None => Far::NULL,
        }
    }
    fn obj_wake(&mut self, id: u16) -> Far {
        self.object_wake(num::low8(id));
        self.obj_find(id)
    }
    fn obj_show(&mut self, id: u16) -> Far {
        if self.object_show_all(num::low8(id)) {
            Far::object(id)
        } else {
            Far::NULL
        }
    }
    fn obj_show_frame(&mut self, id: u16, k: i16) -> Far {
        self.object_show_frame(num::low8(id), k);
        self.obj_find(id)
    }
    fn obj_anim(&mut self, rec: Far, k: i16) {
        if let Some(id) = rec.object_id() {
            self.object_anim(num::low8(id), num::low8(num::unsigned(k)));
        }
    }
    fn obj_anim_id(&mut self, id: u16, k: i16) {
        self.object_anim(num::low8(id), num::low8(num::unsigned(k)));
    }
    fn obj_hide(&mut self, id: u16) -> Far {
        let found = self.obj_find(id);
        self.object_hide(num::low8(id));
        found
    }
    fn obj_state(&mut self, id: u16) -> i16 {
        i16::from(self.object_is_live(num::signed(id)))
    }
    fn objs_save_state(&mut self, out: Far) -> u16 {
        if let Some(loc) = self.object_state_row(out) {
            self.store_location_record(loc);
        }
        0
    }
    fn walk_to_node(&mut self, node: u16) -> i16 {
        // `14DDh:0A56h`: the player, facing left as it was, and the walk
        // put on record when one started (`0x15840`).
        let answer = self.walk_to_object(num::signed(node), -1);
        if answer != 0 {
            self.walk_target = num::signed(node);
        }
        answer
    }
    fn anim_pass(&mut self, delta: i16) {
        self.animator_tick(i32::from(delta));
    }
    fn room_redraw(&mut self) {
        // The renderer draws the whole picture every frame.
    }

    fn walker_walk_to(&mut self, rec: Far, node: u16, facing: i16) -> i16 {
        if let Some(i) = rec.mover_index() {
            return self.walk_mover(i, node, facing);
        }
        if rec != self.ds(self.map().walker) {
            return 0;
        }
        self.walk_to_object(num::signed(node), facing)
    }
    fn walker_snap(&mut self, rec: Far, node: u16, facing: i16) -> i16 {
        if let Some(i) = rec.mover_index() {
            return self.snap_mover(i, node, facing);
        }
        if rec != self.ds(self.map().walker) {
            return 0;
        }
        i16::from(self.snap_player(num::signed(node), facing))
    }
    fn walker_detach(&mut self, rec: Far) {
        if let Some(i) = rec.mover_index() {
            // `0x0B935`: nothing without walk graphs.
            if self.room().is_some_and(|r| !r.walks.is_empty()) {
                self.detach_mover(i);
            }
        } else if rec == self.ds(self.map().walker) {
            self.detach_player();
        }
    }
    fn inv_has(&mut self, id: u16) -> i16 {
        i16::from(self.carried(num::low8(id)))
    }
    fn inv_give(&mut self, id: u16) -> i16 {
        let before = self.carried(num::low8(id));
        self.give_item(num::low8(id));
        i16::from(!before && self.carried(num::low8(id)))
    }
    fn inv_take(&mut self, id: u16) -> i16 {
        let before = self.carried(num::low8(id));
        self.take_item(num::low8(id));
        i16::from(before)
    }
    fn panel_refresh(&mut self) {
        // The panel is drawn whole every frame.
    }
    fn fx_reveal(&mut self) {
        // `10B8h:03BAh`: queued with the picture as it stands; it plays
        // before the frame loop goes on, from the counter at zero.
        self.events.push(Event::Reveal);
        let picture = self.render_scene();
        self.effects.reveal(&picture, &self.presented);
    }

    fn random(&mut self, lo: u16, hi: u16) -> u16 {
        self.roll(lo, hi)
    }
    fn farptr_count(&mut self, row: Far) -> u16 {
        let mut n = 0u16;
        while self.ds_word(row.off.wrapping_add(n * 4)) != 0
            || self.ds_word(row.off.wrapping_add(n * 4 + 2)) != 0
        {
            n += 1;
        }
        n
    }
    fn script_exit(&mut self, k: u16) {
        Adventure::script_exit(self, num::low8(k));
    }
    fn movmem(&mut self, src: Far, dst: Far, n: u16) {
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
        for (i, &b) in src.iter().enumerate() {
            self.set_far_byte(
                Far {
                    off: dst.off.wrapping_add(num::word(i)),
                    seg: dst.seg,
                },
                b,
            );
        }
    }
    fn heap_0720(&mut self, p: Far) -> u16 {
        // There is no heap here. A song's handle names no block of this
        // engine's — the song is the library's bytes — so there is nothing
        // to hand back; anything else is recorded.
        if p.seg == super::music::SONG_HANDLE {
            return 0;
        }
        self.events.push(Event::Unbuilt(
            "1EF8h:0720h",
            (u32::from(p.seg) << 16) | u32::from(p.off),
        ));
        0
    }
    fn mem_free(&mut self, _p: Far) -> u16 {
        0
    }
    fn entry_door_close(&mut self) {
        self.entry_door_closes();
    }
    fn input_unlock(&mut self) {
        self.input_unlocked();
    }
    fn face(&mut self, dir: i16) {
        self.face_player(dir);
    }
    fn pointer_shape_resync(&mut self) {
        self.pointer_resync();
    }
    fn mover_find(&mut self, id: u16) -> Far {
        self.find_mover(num::low8(id))
    }
    fn walker_bind_graph(&mut self, rec: Far, graph: u16, node: u16, facing: i16) -> i16 {
        if let Some(i) = rec.mover_index() {
            return self.bind_mover(i, graph, node, facing);
        }
        // The player is put down by the entry, never bound by the logic.
        self.events.push(Event::Unbuilt(
            "0A9Fh:0EA6h",
            (u32::from(rec.off) << 16) | (u32::from(graph) << 8) | u32::from(node & 0xFF),
        ));
        0
    }
    fn walker_unlink(&mut self, rec: Far) {
        if let Some(i) = rec.mover_index() {
            self.detach_mover(i);
        } else if rec == self.ds(self.map().walker) {
            self.detach_player();
        } else {
            self.events
                .push(Event::Unbuilt("0A9Fh:0E7Fh", u32::from(rec.off)));
        }
    }
    fn scene_enqueue(&mut self, n: i16) -> u16 {
        if self.tasks.busy() {
            if self.tasks.enqueue(n) { 0xFFFF } else { 0 }
        } else {
            self.scene_load(n)
        }
    }
    fn ambient_play(&mut self, lo: i16, hi: i16, flags: u16) -> u16 {
        self.ambient_load(lo, hi, flags)
    }
    fn pal_set(&mut self, rgb: Far, start: u16, count: u16) {
        if rgb.seg == self.dgroup {
            let bytes: Vec<u8> = (0..count * 3)
                .map(|i| self.byte(rgb.off.wrapping_add(i)))
                .collect();
            self.dac.load(num::low8(start), &bytes);
        }
    }
    fn song_load(&mut self, n: u16) -> Far {
        self.music.load(n)
    }
    fn music_play(&mut self, res: Far, a: i16) {
        self.music_call(super::music::Call::Play {
            song: res,
            loops: a,
        });
    }
    fn music_play_fade(&mut self, res: Far, a: i16, b: u16) {
        self.music_call(super::music::Call::PlayFade {
            song: res,
            loops: a,
            ms: b,
        });
    }
    fn music_fade(&mut self, n: i16) {
        self.music_call(super::music::Call::Fade(n));
    }
    fn music_stop(&mut self) {
        self.music_call(super::music::Call::Stop);
    }
    fn set_far_byte(&mut self, at: Far, v: u8) {
        if at.seg == self.dgroup {
            self.set_byte(at.off, v);
        } else if at.seg == OBJECT_HANDLE || at.seg == MOVER_HANDLE {
            // A field of a record the engine keeps in its own shape.
            self.events.push(Event::FarWrite {
                seg: at.seg,
                off: at.off,
                value: u16::from(v),
            });
        } else {
            // The program's own memory: the image, which the engine reads
            // its tables from, so a write here is seen as the original sees
            // it.
            let at = usize::from(at.seg) * 16 + usize::from(at.off);
            self.image.poke(at, v);
        }
    }
    fn local(&self, i: usize) -> u16 {
        self.handler_locals.get(i).copied().unwrap_or(0)
    }
    fn set_local(&mut self, i: usize, v: u16) {
        if let Some(l) = self.handler_locals.get_mut(i) {
            *l = v;
        }
    }
    fn far_byte(&mut self, at: Far) -> u8 {
        if let Some(b) = at
            .mover_field()
            .and_then(|(i, field)| self.mover_byte(i, field))
        {
            b
        } else if at.seg == self.dgroup {
            self.byte(at.off)
        } else if at.seg == OBJECT_HANDLE || at.seg == MOVER_HANDLE {
            self.events.push(Event::Unbuilt(
                "far read",
                (u32::from(at.seg) << 16) | u32::from(at.off),
            ));
            0
        } else {
            let at = usize::from(at.seg) * 16 + usize::from(at.off);
            self.image.bytes().get(at).copied().unwrap_or(0)
        }
    }
}

impl Adventure {
    /// Whether the logic shares this word with the engine.
    fn is_shared(&self, addr: u16) -> bool {
        let m = self.map();
        [
            m.entry,
            m.location,
            m.here,
            m.hundredths,
            m.ticks_taken,
            m.verb,
            m.object,
            m.object_class,
            m.second,
            m.second_class,
            m.input_event,
            m.gate,
            m.gate.wrapping_add(2),
            m.room_header,
            m.room_header.wrapping_add(2),
            m.entry_door,
        ]
        .contains(&addr)
    }
}

/// The data segment's bytes that the engine keeps in its own shape.
///
/// Three blocks of `DGROUP` are records the runtime holds as structures,
/// and a read or a write through the data segment reaches the structure:
/// the 32-byte object rows of every location (`DS:1F56h`, the save's
/// `0x14C42` and the restore's `0x14D5A` move them as bytes), the carried
/// list with its count and scroll bytes (`0x14C8B`, `0x14D72`), and the
/// player's 30h-byte record, which is the walker (`0x14C73`, `0x14D0C`;
/// see `player.rs`).
impl Adventure {
    /// Which object row and byte an address names, if it names one.
    fn object_state_at(&self, addr: u16) -> Option<(usize, usize)> {
        let d = usize::from(addr.checked_sub(self.map().object_states)?);
        (d < self.location_records.len() * 32).then_some((d / 32, d % 32))
    }

    /// One byte of the data segment.
    pub(crate) fn mem_byte(&self, addr: u16) -> u8 {
        let m = self.map();
        if let Some((row, i)) = self.object_state_at(addr) {
            return self.location_records[row][i];
        }
        if let Some(i) = self.player_record_at(addr) {
            return self.player_record_byte(i);
        }
        let (count, scroll, list) = m.inventory;
        if addr == count {
            num::byte(self.items.len())
        } else if addr == scroll {
            num::byte(self.scroll)
        } else if let Some(&id) = addr
            .checked_sub(list)
            .and_then(|i| self.items.get(usize::from(i)))
        {
            id
        } else {
            self.ds[usize::from(addr)]
        }
    }

    /// The same, written.
    pub(crate) fn set_mem_byte(&mut self, addr: u16, v: u8) {
        let m = self.map();
        if let Some((row, i)) = self.object_state_at(addr) {
            self.location_records[row][i] = v;
            return;
        }
        if let Some(i) = self.player_record_at(addr) {
            self.set_player_record_byte(i, v);
            return;
        }
        let (count, scroll, list) = m.inventory;
        // The block the save copies from the count on, `7Fh` bytes
        // (`0x14C8B`), is the most the list can reach.
        if (count..count.wrapping_add(0x7F)).contains(&addr) {
            // The list as bytes, the byte written, the list read back:
            // a block copy that writes the count first and the items after
            // it ends on the list it copied.
            self.ds[usize::from(count)] = num::byte(self.items.len());
            self.ds[usize::from(scroll)] = num::byte(self.scroll);
            for (i, &id) in self.items.iter().enumerate() {
                self.ds[usize::from(list) + i] = id;
            }
            self.ds[usize::from(addr)] = v;
            let n = usize::from(self.ds[usize::from(count)]);
            let at = usize::from(list);
            self.items = self.ds[at..at + n].to_vec();
            self.scroll = usize::from(self.ds[usize::from(scroll)]);
            return;
        }
        self.ds[usize::from(addr)] = v;
    }

    /// The far buffer a saved game is held in: a segment of its own below
    /// `DGROUP`, `8B1h` bytes and the padding up to `DGROUP` after them.
    /// It lives where the program has it, in the image.
    pub(crate) fn save_buffer(&self) -> &[u8] {
        let at = usize::from(self.map().save_buffer) * 16;
        let end = self.image.at(0);
        &self.image.bytes()[at..end]
    }

    /// One byte of that buffer, written.
    pub(crate) fn save_buffer_set(&mut self, off: usize, v: u8) {
        let at = usize::from(self.map().save_buffer) * 16 + off;
        self.image.poke(at, v);
    }
}
