//! What a parsec1994 title's location logic asks of the engine.
//!
//! The game's logic — a location's handler, its hooks and its own rule
//! handlers, the global rules and the shared hook — is compiled C that
//! talks to the engine through far calls alone, and it reads and writes
//! globals in `DGROUP`. [`Api`] is that boundary: one method per engine
//! routine, as the recording stand-in in `trace.rs` names them, with the
//! routine's own arguments in the original's order and its own return
//! value, and accessors for the data segment, the far bytes the logic reads
//! and the handler's own locals. Two methods stand for a call through a far
//! pointer rather than to a routine: the shared hook's hand-on to the
//! location's own hook ([`Api::location_hook`]) and the visit's callback
//! ([`Api::callback`]).
//!
//! The logic is written against this trait and nothing else, so the same
//! code runs against the engine and against a recording stand-in that
//! compares every call with a trace of the original routine (`trace.rs`).
//!
//! Each method carries the routine's address in *Dunkle Schatten*'s
//! `XP01.EXE` and what it does; where that program has no such routine, the
//! method says which build has it.

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
    /// `179Eh:0830h`: the pointer put on the screen and unfrozen.
    fn pointer_show(&mut self);
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
    /// `0883h:03EBh(rec, k)`: the object at `rec` on frame `k`, or hidden
    /// for a `k` below zero.
    fn obj_frame(&mut self, rec: Far, k: i16);
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
    /// `0CFDh:194Bh(n, flags)`: scene `n` bound to task 1 as
    /// [`Api::ambient_play`] binds a rolled one, without the roll.
    fn ambient_play_one(&mut self, n: i16, flags: u16) -> u16;
    /// `0CFDh:1B55h`: the speakers of task 1 while it plays (kind 4 or
    /// more), else 0.
    fn ambient_speakers(&mut self) -> u16;

    // --- palette and music: 0590h, 1197h ------------------------------------

    /// `0590h:09E2h(rgb, start, count)`: `count` DAC entries from `start`.
    fn pal_set(&mut self, rgb: Far, start: u16, count: u16);
    /// `0590h:0A1Fh(rec)` (*Bi-Fi 2*'s `0104h:090Eh`): a palette record
    /// loaded — its first DAC register, its count (0 meaning 256) and its
    /// colors — as the build loads a resource's colors: into the
    /// flattening table's lookup, or with the shade's darker copy `80h` on.
    fn pal_load(&mut self, rec: Far);
    /// The same loader handed a record the handler built on its own stack
    /// (`push ss ; lea ax, [bp-n]`).
    fn pal_load_local(&mut self, rec: &[u8]);
    /// `0724h:03D7h(src, dst, r, g, b, n)` (*Bi-Fi 2*'s `02AFh:0217h`): `n`
    /// colors from `src` to `dst`, red through ramp `r`, green through
    /// `g`, blue through `b` (`xlat` from `cs:00A0h + 40h·k`).
    fn pal_ramps(&mut self, src: Far, dst: Far, ramps: [u16; 3], n: u16);
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
    /// The digital driver's entry 18, `StopAll(wait)` (*Dunkle
    /// Schatten*'s `18FDh:0436h`), called with a digital driver installed
    /// (`cmp word [2B0h], 0`, *Bi-Fi 2*'s `0x14046`) whatever the music
    /// switch says. `wait` 1 waits for the block's interrupt; this runtime's
    /// effects stop at once either way.
    fn digital_stop(&mut self, wait: u16);
    /// The digital driver's entry 22 (`18FDh:044Ah`): the effects' volume
    /// set, the old one answered.
    fn effect_volume(&mut self, v: u16) -> u16;

    /// `1EB5h:0028h(lo, hi)`: a roll of the shared generator, `lo..=hi`.
    fn random(&mut self, lo: u16, hi: u16) -> u16;
    /// `1EBDh:0000h(row)`: the far pointers in a null-terminated row.
    fn farptr_count(&mut self, row: Far) -> u16;
    /// `1586h:05E1h(k)`: the location left through exit `k`, as the mode
    /// machine's gate does it.
    fn script_exit(&mut self, k: u16);

    // --- what game code beyond the locations asks: the global rules and
    // the shared hook -------------------------------------------------------

    /// `0883h:000Fh(rec)`: the object's rectangle marked for the drawer.
    /// This engine draws every frame whole and keeps no such marks.
    fn obj_invalidate(&mut self, rec: Far);
    /// `0883h:20D9h(rec, k)`: animation `k` on the figure's record `rec`;
    /// answers `k + 1`, 1 for a fresh start, or 0 without an entry.
    fn figure_anim(&mut self, rec: Far, k: i16) -> i16;
    /// `0CFDh:18C2h(text)`: a hotspot's own line, played by task 0 as
    /// kind 3, over the player.
    fn say_text(&mut self, text: Far);
    /// The far pointer the hotspot the object was chosen through keeps at
    /// its `+16h + 4k` (`[28B3h]` the record), null when there is no such
    /// hotspot or no line in that slot.
    fn hotspot_own_text(&mut self, k: u16) -> Far;
    /// `14DDh:074Bh(rec)`: the hotspot whose mask ends in the object's id
    /// takes the object's live bit as its own.
    fn hotspot_follow(&mut self, rec: Far);
    /// The hotspot the object was chosen through (`[28B3h]` the record),
    /// its active bit, `+4` bit 0, cleared: `and word es:[bx+4], 0FFFEh`.
    /// Answers whether there was one.
    fn chosen_hotspot_off(&mut self) -> bool;
    /// `0A9Fh:0C0Ch(rec, dir)`: the figure faces direction `dir`.
    fn figure_face(&mut self, rec: Far, dir: i16);
    /// `0A9Fh:10F7h(k)`: a turn animation on the figure; answers 1 when
    /// one started.
    fn figure_turn(&mut self, k: i16) -> i16;
    /// The shared hook's hand-on (`call far [1E5Ch]`, *Dunkle Schatten*'s
    /// `0x1085E`): `(code, arg, task)` to the location's own hook, the far
    /// pointer its record keeps at `+2Fh`. Its answer is not read.
    fn location_hook(&mut self, code: u16, arg: u16, task: u16);
    /// The visit's callback asked about object `id`, through the far
    /// pointer the build keeps for it ([`super::Profile::callback`]): the
    /// library's own answer where the visit's setup put it, or the
    /// location's routine it was bent to. −1 for an object that is not
    /// there. *Dunkle Schatten*'s program keeps no callback.
    fn callback(&mut self, id: u16) -> i16;
    /// `0CFDh:1A0Eh(i)`: task `i` stopped — countdown 0, kind 1 asked for.
    fn task_stop(&mut self, i: u16);
    /// `0CFDh:1BA2h(k)`: the page task 0 holds let go, `k` the code that
    /// let it go.
    fn page_release(&mut self, k: u16);
    /// `0C3Fh:0581h(owner)`: the band held for `owner`'s bits.
    fn band_hold(&mut self, owner: u16);
    /// `0C3Fh:0639h(owner)` (*Bi-Fi 2*'s `0751h:0635h`, Captain Zins's
    /// `078Ah:0646h`): the band's redraw held for `owner`'s bits — added
    /// to the holders (`[4F0h]`), its pending release cancelled
    /// (`[4F2h]`), the mask the painter tests the redraw marks `[4E8h]`
    /// against cleared (`[4EAh]`, `0x0CA3B`) and the refresh countdown
    /// stopped (`[4F6h]`). With that mask 0 the painter leaves the band as
    /// it stands (`0x0C4B3`–`0x0C4BC`) until a release sets it again
    /// (`0x0CA0C`); this runtime composes the band whole each frame and
    /// keeps no such mask. The one hold the programs make, the Kaffee's
    /// before the game's end (*Bi-Fi 2*'s `0x1090E`), is followed by a
    /// wait that polls no input (`0x10923`) and then by the end's
    /// sequence, which paints over the band, so the band it composes is
    /// the one that stands.
    fn band_redraw_hold(&mut self, owner: u16);
    /// `0A9Fh:1002h(rec, node)`: every edge of node `node` in the graph
    /// `rec` is bound to blocked (`3FFh`), so that no walk takes it; a
    /// node the graph does not have is left alone.
    fn node_block(&mut self, rec: Far, node: u16);
    /// `0A9Fh:105Ch(rec, node)`: the same edges open again (`0`).
    fn node_open(&mut self, rec: Far, node: u16);
    /// `0EB9h:177Ah(id, facing, scene)`: the script's own interaction with
    /// object `id`, as if the pointer had chosen it with verb 2 — the mode
    /// block set, the player sent to the node of that id facing `facing`
    /// — and scene `scene` played.
    fn interaction(&mut self, id: u16, facing: i16, scene: i16);
    /// `0EB9h:1703h(id, facing, counter, scenes…, -1)`: the same
    /// interaction with the next scene of a sequence; the byte at
    /// `counter` steps up to the count of scenes, one past the last.
    fn interaction_sequence(&mut self, id: u16, facing: i16, counter: Far, scenes: &[i16]);

    // --- records in the logic's own data, the roll, the tile marks -------

    /// `05DCh:0D6Ah(rec)` (*Dunkle Schatten*'s `0A9Fh:0E4Ch`, Captain
    /// Zins's `05F2h:0DF0h`): the record's animation stopped, the record
    /// taken out of any chain and put at the head of the one its `+2Ah`
    /// names. *Bi-Fi 2*'s Vorführraum links a record of its own data
    /// segment with it ([`super::image_records`]).
    fn record_link(&mut self, rec: Far);
    /// `0284h:015Eh` (*Dunkle Schatten*'s `12D3h:010Ah`): the roll laid
    /// out from the far pointers at `row` by `spec`; where it ends. The
    /// routine takes no arguments and lays out the row it names itself
    /// (`push ds ; push 0BCEh`, `0x029A4`), which is `row`. Captain Zins's
    /// `XP01` has no such routine.
    fn roll_layout(&mut self, row: Far, spec: &super::title::roll::RollSpec) -> u16;
    /// `0284h:0001h(pos)` (*Dunkle Schatten*'s `12D3h:025Ah`): the roll's
    /// cursor taken to `pos`; the weights of the lines it passed. Captain
    /// Zins's `XP01` has no such routine.
    fn roll_seek(&mut self, pos: u16) -> u16;
    /// `0284h:0079h` (*Dunkle Schatten*'s `12D3h:02D2h`): the roll drawn
    /// from its cursor at the position last sought, clipped to its
    /// window. Captain Zins's `XP01` has no such routine.
    fn roll_draw(&mut self);
    /// `0284h:029Ah` (*Dunkle Schatten*'s `12D3h:0249h`): the roll's lines
    /// freed. Captain Zins's `XP01` has no such routine.
    fn roll_free(&mut self);
    /// `1C3Ch:2036h(x, y, w, h)` (*Dunkle Schatten*'s `1BBDh:2036h`,
    /// Captain Zins's `1A81h:2036h`): the screen's tiles under the box
    /// marked for the drawer.
    fn tiles_mark(&mut self, x: u16, y: u16, w: u16, h: u16);
    /// `1C3Ch:20C1h(x, y, w, h)` (*Dunkle Schatten*'s `1BBDh:20C1h`,
    /// Captain Zins's `1A81h:20C1h`): whether any tile under the box is
    /// marked.
    fn tiles_dirty(&mut self, x: u16, y: u16, w: u16, h: u16) -> bool;
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
/// keeps the room in its own shape and serves of it what the logic reads:
/// the far pointer at `+1Ch` to the room's cel table
/// ([`ROOM_TABLE_HANDLE`]).
const ROOM_HANDLE: u16 = 0xF0B3;
/// The segment word of the handle for the room's cel table, whose entry
/// 0 names the room's palette record (the resource's offset 8, in front
/// of the cels; [`ROOM_PALETTE_HANDLE`]). The cels behind it are not
/// served.
const ROOM_TABLE_HANDLE: u16 = 0xF0B5;
/// The segment word of the handle for the room's palette record: its
/// first DAC register, its count and its colors, as the file has them.
const ROOM_PALETTE_HANDLE: u16 = 0xF0B6;
/// The segment word of the handle for a line of the chosen hotspot; the
/// offset word is the line's slot.
const TEXT_HANDLE: u16 = 0xF0B4;

impl Far {
    /// Whether the segment word is one of the handles the engine gives
    /// out for the records it keeps in its own shape.
    pub(crate) fn is_handle(self) -> bool {
        (OBJECT_HANDLE..=ROOM_PALETTE_HANDLE).contains(&self.seg)
    }

    /// The handle for the loaded room's header.
    pub(crate) fn room() -> Far {
        Far {
            off: 0,
            seg: ROOM_HANDLE,
        }
    }
}

/// The room a handle leaves for each object's fields: the offset word is
/// the id times this plus the field.
const OBJECT_STRIDE: u16 = 0x40;

impl Far {
    /// The handle for object `id`.
    pub(crate) fn object(id: u16) -> Far {
        Far {
            off: id.wrapping_mul(OBJECT_STRIDE),
            seg: OBJECT_HANDLE,
        }
    }

    /// The object a handle names, when it is an object's own address.
    pub(crate) fn object_id(self) -> Option<u16> {
        self.object_field()
            .filter(|&(_, f)| f == 0)
            .map(|(id, _)| id)
    }

    /// The object and the field offset a pointer into one names.
    pub(crate) fn object_field(self) -> Option<(u16, u16)> {
        (self.seg == OBJECT_HANDLE).then_some((self.off / OBJECT_STRIDE, self.off % OBJECT_STRIDE))
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
    /// The mode machine's mode (`[288Fh]`) and its wait bits (`[289Bh]`).
    pub mode: u16,
    pub bits: u16,
    /// The verb as it was chosen (`[289Dh]`), beside the verb itself.
    pub verb_chosen: u16,
    /// The two-object word (`[2899h]`), which the mode reset clears second
    /// (*Dunkle Schatten*'s `0x0EBD5`, *Bi-Fi 2*'s `0x09BA8`).
    pub two_object: u16,
    /// The walk verb, which the door's and the script exit's gates set.
    pub walk_verb: u8,
    /// The loaded room's header, a far pointer (`[2851h]`).
    pub room_header: u16,
    /// The door the player came in by, until it has swung shut (`[1EDEh]`).
    pub entry_door: u16,
    /// The node the figure walks to, put on record when a walk starts
    /// (`[289Fh]`).
    pub walk_target: u16,
    /// The animation code the mode machine waits for (`[28C1h]`).
    pub anim_wait: u16,
    /// The status line's age, which asks for the line to be composed again
    /// (`[28C3h]`).
    pub status_age: u16,
    /// The items' classes, a byte an item id: what the choice of an item
    /// puts beside it as its class (`[0527h + id]` at `0x0F5C4`).
    pub item_classes: u16,
    /// The carried list: its count byte, the first visible slot's byte,
    /// and the first byte of the list.
    pub inventory: (u16, u16, u16),
    /// The segment of the far buffer a saved game is held in.
    pub save_buffer: u16,
    /// Where the routines the gate names are linked.
    pub gates: GateMap,
    /// The choice menu's record, its geometry and its inks.
    pub menu: u16,
    /// The first of the six words in a row the shared hook keeps the
    /// script's business with the figure in, [`Script`]'s `pending_animation` to
    /// `turn_pending` in address order; `None` for a build whose hook keeps
    /// no such row.
    pub script: Option<u16>,
    /// The word the shared hook sets to ask the dispatcher's head to start
    /// task 0 once the figure is linked ([`Script`]'s `start_task_asked`); `None` for
    /// a build whose hook never asks.
    pub script_start: Option<u16>,
}

use super::mode::{Gate, GateMap};
use super::{Adventure, Body, Event, Script, objects};
use crate::titles::subject::Subject;

impl Adventure {
    /// The walking body a record's address names: the player's record, or
    /// one of the room's walking records.
    fn walking_body(&self, rec: Far) -> Option<Body> {
        if rec == self.ds(self.map().walker) {
            Some(Body::Figure)
        } else {
            rec.mover_index().map(Body::Mover)
        }
    }
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

    pub(super) fn ds_word(&self, addr: u16) -> u16 {
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
        } else if addr == m.mode {
            self.mode
        } else if addr == m.verb_chosen {
            self.verb_chosen
        } else if addr == m.bits {
            self.bits
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
        } else if addr == m.walk_target {
            num::unsigned(self.walk_target)
        } else if addr == m.two_object {
            self.two_object
        } else if addr == m.anim_wait {
            self.anim_wait
        } else if addr == m.status_age {
            self.status_refresh_age
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
        } else if let Some(i) = self.script_word(addr) {
            self.script.word(i)
        } else if let Some(high) = self.callback_word(addr) {
            if high {
                self.callback.seg
            } else {
                self.callback.off
            }
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
        } else if addr == m.verb_chosen {
            self.verb_chosen = v;
        } else if addr == m.mode {
            self.mode = v;
        } else if addr == m.bits {
            self.bits = v;
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
        } else if addr == m.walk_target {
            self.walk_target = num::signed(v);
        } else if addr == m.two_object {
            self.two_object = v;
        } else if addr == m.anim_wait {
            self.anim_wait = v;
        } else if addr == m.status_age {
            self.status_refresh_age = v;
        } else if let Some(i) = self.script_word(addr) {
            self.script.set_word(i, v);
        } else if let Some(high) = self.callback_word(addr) {
            if high {
                self.callback.seg = v;
            } else {
                self.callback.off = v;
            }
        } else if addr == m.input_event
            || addr == m.room_header
            || addr == m.room_header.wrapping_add(2)
        {
        } else if addr == m.gate || addr == m.gate.wrapping_add(2) {
            // The two words name the far routine; the engine knows two.
            let [lo, hi] = v.to_le_bytes();
            self.ds[a] = lo;
            self.ds[usize::from(addr.wrapping_add(1))] = hi;
            // A pair written word by word is unknown between the two
            // writes; whether the gate is one the engine has is asked when
            // the gate is (`Adventure::ask_gate`).
            let off = self.ds_word(m.gate);
            let seg = self.ds_word(m.gate.wrapping_add(2));
            // A far routine none of the three: the location's own.
            self.gate = m.gates.gate((seg, off)).unwrap_or(Gate::Own(seg, off));
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
    fn pointer_show(&mut self) {
        self.pointer_on();
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
        let code = self.slot_code(code);
        match rec.object_id() {
            Some(id) => self
                .subjects
                .set(usize::from(slot), code, Subject::Object(num::low8(id))),
            None => self.subjects.clear(usize::from(slot), code),
        }
    }
    fn slot_set_raw(&mut self, slot: u16, code: u16, w1: u16, w2: u16) {
        let code = self.slot_code(code);
        // Only a point (kind 0) is ever written this way by the logic.
        self.subjects
            .set_point(usize::from(slot), code, num::signed(w1), num::signed(w2));
    }
    fn slot_set_figure(&mut self, slot: u16, code: u16, rec: Far) {
        let code = self.slot_code(code);
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
    fn obj_frame(&mut self, rec: Far, k: i16) {
        if let Some(i) = rec
            .object_id()
            .and_then(|id| self.object_index(num::low8(id)))
        {
            self.object_frame(i, k);
        }
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
        if answer != 0 && self.profile.walk_to_node_remembers {
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
        // `0x0B935`: nothing without walk graphs, for any record.
        if !self.room().is_some_and(|r| !r.walks.is_empty()) {
            return;
        }
        if let Some(i) = rec.mover_index() {
            self.detach_mover(i);
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
    fn obj_invalidate(&mut self, _rec: Far) {}
    fn figure_anim(&mut self, rec: Far, k: i16) -> i16 {
        // `0x0A90E`: a null record answers 0.
        if rec == Far::NULL {
            return 0;
        }
        match self.walking_body(rec) {
            Some(Body::Figure) => self.anim_player(k),
            Some(body) => self.anim_walker(body, k),
            None => {
                self.events
                    .push(Event::Unbuilt("0883h:20D9h", u32::from(rec.off)));
                0
            }
        }
    }
    fn say_text(&mut self, text: Far) {
        if text.seg != TEXT_HANDLE {
            return;
        }
        if let Some(line) = self.own_text(usize::from(text.off)) {
            self.hotspot_text(&line);
        }
    }
    fn hotspot_own_text(&mut self, k: u16) -> Far {
        if self.own_text(usize::from(k)).is_some() {
            Far {
                off: k,
                seg: TEXT_HANDLE,
            }
        } else {
            Far::NULL
        }
    }
    fn chosen_hotspot_off(&mut self) -> bool {
        let Some(h) = self.hover_spot.and_then(|i| self.hotspots_rt.get_mut(i)) else {
            return false;
        };
        h.active = false;
        true
    }
    fn hotspot_follow(&mut self, rec: Far) {
        let Some(i) = rec
            .object_id()
            .and_then(|id| self.object_index(num::low8(id)))
        else {
            return;
        };
        self.hotspot_follows(i);
    }
    fn figure_face(&mut self, rec: Far, dir: i16) {
        // `0x0B608`: a null record is let alone.
        if rec == Far::NULL {
            return;
        }
        match self.walking_body(rec) {
            Some(body) => self.face_walker(body, dir),
            None => self
                .events
                .push(Event::Unbuilt("0A9Fh:0C0Ch", u32::from(rec.off))),
        }
    }
    fn figure_turn(&mut self, k: i16) -> i16 {
        self.turn_player(k)
    }
    fn location_hook(&mut self, code: u16, arg: u16, task: u16) {
        let (arg, task) = (num::signed(arg), usize::from(task));
        self.events.push(Event::LocationHook { code, arg, task });
        // The location's own hook, its record's `+2Fh` in Dunkle Schatten
        // (`call far [1E5Ch]`, `0x1085E`), at once.
        if let Some(table) = self.logic_table {
            table.location_hook(self, code, arg, task);
        }
    }
    fn callback(&mut self, id: u16) -> i16 {
        Adventure::callback(self, id)
    }
    fn task_stop(&mut self, i: u16) {
        if usize::from(i) < self.tasks.t.len() {
            Adventure::task_stop(self, usize::from(i));
        }
    }
    fn page_release(&mut self, k: u16) {
        Adventure::page_release(self, k);
    }
    fn band_hold(&mut self, owner: u16) {
        self.panel_tint(owner);
    }
    fn band_redraw_hold(&mut self, owner: u16) {
        self.panel_redraw_hold(owner);
    }
    fn node_block(&mut self, rec: Far, node: u16) {
        self.set_node_edges(rec, node, 0x03FF);
    }
    fn node_open(&mut self, rec: Far, node: u16) {
        self.set_node_edges(rec, node, 0);
    }
    fn interaction(&mut self, id: u16, facing: i16, scene: i16) {
        Adventure::interaction(self, id, facing, scene);
    }
    fn interaction_sequence(&mut self, id: u16, facing: i16, counter: Far, scenes: &[i16]) {
        Adventure::interaction_sequence(self, id, facing, usize::from(counter.off), scenes);
    }
    fn record_link(&mut self, rec: Far) {
        if self.is_image_record(rec) {
            self.link_image_record(rec);
        } else {
            // The player and the people are linked by the walker's own
            // calls, never by the logic's.
            self.events.push(Event::Unbuilt(
                "05DCh:0D6Ah",
                (u32::from(rec.seg) << 16) | u32::from(rec.off),
            ));
        }
    }
    fn roll_layout(&mut self, row: Far, spec: &super::title::roll::RollSpec) -> u16 {
        self.roll_layout_impl(row, spec)
    }
    fn roll_seek(&mut self, pos: u16) -> u16 {
        self.roll_seek_impl(pos)
    }
    fn roll_draw(&mut self) {
        self.roll_draw_impl();
    }
    fn roll_free(&mut self) {
        self.room_roll = None;
    }
    // The runtime composes the whole picture every frame, so a mark has
    // nothing to ask for and every tile is as good as marked — as with
    // `obj_invalidate` and `room_redraw`.
    fn tiles_mark(&mut self, _x: u16, _y: u16, _w: u16, _h: u16) {}
    fn tiles_dirty(&mut self, _x: u16, _y: u16, _w: u16, _h: u16) -> bool {
        true
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
        } else if self.is_image_record(rec) && self.unlink_image_record(rec) {
            // A record of the logic's own that `05DCh:0D6Ah` linked.
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
    fn ambient_play_one(&mut self, n: i16, flags: u16) -> u16 {
        self.ambient_load_one(n, flags)
    }
    fn ambient_speakers(&mut self) -> u16 {
        Adventure::ambient_speakers(self)
    }
    fn pal_set(&mut self, rgb: Far, start: u16, count: u16) {
        let bytes: Vec<u8> = (0..count * 3)
            .map(|i| {
                self.far_byte(Far {
                    off: rgb.off.wrapping_add(i),
                    seg: rgb.seg,
                })
            })
            .collect();
        self.dac.load(num::low8(start), &bytes);
    }
    fn pal_load(&mut self, rec: Far) {
        let at = |i: u16| Far {
            off: rec.off.wrapping_add(i),
            seg: rec.seg,
        };
        let start = self.far_byte(at(0));
        let count = match self.far_byte(at(1)) {
            0 => 0x100,
            n => u16::from(n),
        };
        let rgb: Vec<u8> = (0..count * 3).map(|i| self.far_byte(at(2 + i))).collect();
        self.load_palette(start, &rgb);
    }
    fn pal_load_local(&mut self, rec: &[u8]) {
        if let [start, n, rgb @ ..] = rec {
            let count = if *n == 0 { 0x100 } else { usize::from(*n) };
            let rgb = rgb.get(..count * 3).unwrap_or(rgb).to_vec();
            self.load_palette(*start, &rgb);
        }
    }
    fn pal_ramps(&mut self, src: Far, dst: Far, ramps: [u16; 3], n: u16) {
        let Some(table) = self.flatten.ramps().copied() else {
            self.events
                .push(Event::Unbuilt("0724h:03D7h", u32::from(n)));
            return;
        };
        for i in 0..n.wrapping_mul(3) {
            let at = |p: Far| Far {
                off: p.off.wrapping_add(i),
                seg: p.seg,
            };
            let v = self.far_byte(at(src));
            let k = usize::from(ramps[usize::from(i % 3)]) * 0x40 + usize::from(v);
            let Some(&out) = table.get(k) else {
                self.events
                    .push(Event::Unbuilt("0724h:03D7h past the ramps", u32::from(v)));
                return;
            };
            self.set_far_byte(at(dst), out);
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
    fn effect_volume(&mut self, v: u16) -> u16 {
        self.music.effect_volume(v)
    }
    fn digital_stop(&mut self, _wait: u16) {
        if self.music.dac() {
            self.music_call(super::music::Call::StopEffects);
        }
    }
    fn set_far_byte(&mut self, at: Far, v: u8) {
        if at.seg == self.dgroup {
            self.set_byte(at.off, v);
        } else if self.set_object_byte(at, v).is_some() {
            // A field the engine keeps, written through.
        } else if at
            .mover_field()
            .and_then(|(i, field)| self.set_mover_byte(i, field, v))
            .is_some()
        {
            // A walking record's field the engine keeps.
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
        if let Some(b) = self.room_byte(at) {
            return b;
        }
        if let Some(b) = at
            .mover_field()
            .and_then(|(i, field)| self.mover_byte(i, field))
        {
            b
        } else if at.seg == self.dgroup {
            self.byte(at.off)
        } else if let Some(b) = self.object_byte(at) {
            b
        } else if at.seg == OBJECT_HANDLE || at.seg == MOVER_HANDLE {
            self.events.push(Event::Unbuilt(
                "far read",
                (u32::from(at.seg) << 16) | u32::from(at.off),
            ));
            0
        } else if at.seg == 0 {
            // A null pointer reads the interrupt table at `0000h:0000h`,
            // below the program, whose vectors the machine and the
            // program's start-up set; the image's segments count from the
            // program's start, so none of its bytes is that. The paths played to the games' ends read
            // through none; one that does is said.
            self.events.push(Event::Unbuilt(
                "far read through a null pointer",
                u32::from(at.off),
            ));
            0
        } else {
            let at = usize::from(at.seg) * 16 + usize::from(at.off);
            self.image.bytes().get(at).copied().unwrap_or(0)
        }
    }
}

impl Adventure {
    /// A byte behind one of the room's handles: the header's far pointer
    /// to the cel table (`+1Ch`, *Dunkle Schatten*'s `0x0487B`, *Bi-Fi
    /// 2*'s `0x12D8F`), the table's entry 0, and the palette record it
    /// names. Anything else behind them is told as not built and read as
    /// 0. `None` for an address that is no room handle.
    fn room_byte(&mut self, at: Far) -> Option<u8> {
        let far = |p: Far, i: u16| -> u8 {
            let w = if i < 2 { p.off } else { p.seg };
            w.to_le_bytes()[usize::from(i & 1)]
        };
        let served = match at.seg {
            ROOM_HANDLE => (0x1C..0x20).contains(&at.off).then(|| {
                let table = Far {
                    off: 0,
                    seg: ROOM_TABLE_HANDLE,
                };
                far(table, at.off - 0x1C)
            }),
            ROOM_TABLE_HANDLE => (at.off < 4).then(|| {
                let palette = Far {
                    off: 0,
                    seg: ROOM_PALETTE_HANDLE,
                };
                far(palette, at.off)
            }),
            ROOM_PALETTE_HANDLE => self.room().and_then(|r| {
                let room = &r.room;
                match at.off {
                    0 => Some(room.palette_start),
                    // The count as its byte: 256 colors are a 0.
                    1 => Some(num::low8(num::word(room.palette_count))),
                    k => room.palette_rgb.get(usize::from(k - 2)).copied(),
                }
            }),
            _ => return None,
        };
        Some(served.unwrap_or_else(|| {
            self.events.push(Event::Unbuilt(
                "the room behind its header",
                (u32::from(at.seg) << 16) | u32::from(at.off),
            ));
            0
        }))
    }

    /// A byte of an object's record, for the fields the engine keeps:
    /// the kind `+0`, the flags `+2`, the point `+4`/`+6`, the frame `+9`,
    /// the cel drawn
    /// `+0Ah`, the cel the frames count from `+0Ch`, the state `+12h`, the
    /// id `+13h` and the cels in its block `+15h`.
    fn object_byte(&self, at: Far) -> Option<u8> {
        let (id, field) = at.object_field()?;
        let o = self.objects.get(self.object_index(num::low8(id))?)?;
        match field {
            0x00 => Some(num::low8(o.kind)),
            0x01 => Some(o.kind.to_le_bytes()[1]),
            0x02 => Some(o.flags),
            0x04 => Some(num::low16(o.x).to_le_bytes()[0]),
            0x05 => Some(num::low16(o.x).to_le_bytes()[1]),
            0x06 => Some(num::low16(o.y).to_le_bytes()[0]),
            0x07 => Some(num::low16(o.y).to_le_bytes()[1]),
            0x09 => Some(o.frame),
            0x0A => Some(o.cel.to_le_bytes()[0]),
            0x0B => Some(o.cel.to_le_bytes()[1]),
            0x0C => Some(o.cel_base.to_le_bytes()[0]),
            0x0D => Some(o.cel_base.to_le_bytes()[1]),
            0x12 => Some(o.state),
            0x13 => Some(o.id),
            0x15 => Some(o.phases),
            _ => None,
        }
    }

    /// A write to one of those fields: the flags, the point, the frame,
    /// the two cel words and the block's count, as the logic writes them. A change of
    /// the live bit is told as the object being shown or hidden, as the
    /// routines that change it tell it.
    fn set_object_byte(&mut self, at: Far, v: u8) -> Option<()> {
        let (id, field) = at.object_field()?;
        let id = num::low8(id);
        let i = self.object_index(id)?;
        let o = &mut self.objects[i];
        let put = |w: &mut u16, high: bool| {
            let [lo, hi] = w.to_le_bytes();
            *w = if high {
                u16::from_le_bytes([lo, v])
            } else {
                u16::from_le_bytes([v, hi])
            };
        };
        let put_point = |c: &mut i32, high: bool| {
            let [lo, hi] = num::low16(*c).to_le_bytes();
            let w = if high {
                u16::from_le_bytes([lo, v])
            } else {
                u16::from_le_bytes([v, hi])
            };
            *c = i32::from(num::signed(w));
        };
        match field {
            // `mov byte es:[bx], k`: the kind, as *Bi-Fi 2*'s Gefängnis makes
            // its door one for the visit (`0x17832`) and back (`0x178FD`).
            0x00 | 0x01 => {
                put(&mut o.kind, field == 0x01);
                return Some(());
            }
            0x02 => {}
            0x04 | 0x05 => {
                put_point(&mut o.x, field == 0x05);
                return Some(());
            }
            0x06 | 0x07 => {
                put_point(&mut o.y, field == 0x07);
                return Some(());
            }
            0x09 => {
                o.frame = v;
                return Some(());
            }
            0x0A | 0x0B => {
                put(&mut o.cel, field == 0x0B);
                return Some(());
            }
            0x0C | 0x0D => {
                put(&mut o.cel_base, field == 0x0D);
                return Some(());
            }
            // `mov byte es:[bx+12h], v`: the animator's state, as Bi-Fi 2's
            // Enterhaken - Maschinenraum writes it (`0x13875`, `0x13BCF`).
            0x12 => {
                o.state = v;
                return Some(());
            }
            0x15 => {
                o.phases = v;
                return Some(());
            }
            _ => return None,
        }
        let was = self.objects[i].flags & objects::LIVE;
        self.objects[i].flags = v;
        match (was != 0, v & objects::LIVE != 0) {
            (true, false) => self.events.push(Event::ObjectHidden(id)),
            (false, true) => self.events.push(Event::ObjectShown(id)),
            _ => {}
        }
        Some(())
    }

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
            m.walk_target,
            m.two_object,
            m.anim_wait,
            m.status_age,
        ]
        .contains(&addr)
            || self.script_word(addr).is_some()
            || self.callback_word(addr).is_some()
    }

    /// The code a slot setter was handed, in the engine's own form: as it
    /// stands for a build that copies it, recoded for one that keeps its
    /// own fields ([`super::Profile::slot_style`]).
    fn slot_code(&self, code: u16) -> u16 {
        self.profile.slot_style.map_or(code, |s| s.style(code))
    }

    /// Whether an address is one of the callback's two words, and which:
    /// `true` its segment.
    fn callback_word(&self, addr: u16) -> Option<bool> {
        let w = self.profile.callback?.word;
        (addr == w || addr == w.wrapping_add(2)).then_some(addr != w)
    }

    /// Which of the script's words an address names: 0 to 5 the six from
    /// [`DsMap::script`] on, 6 [`DsMap::script_start`].
    fn script_word(&self, addr: u16) -> Option<usize> {
        let m = self.map();
        if Some(addr) == m.script_start {
            return Some(Script::START);
        }
        let d = addr.checked_sub(m.script?)?;
        (d % 2 == 0 && d < 12).then_some(usize::from(d / 2))
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
        } else if addr == count.wrapping_add(2) {
            num::byte(self.inv_lo)
        } else if addr == count.wrapping_add(3) {
            num::byte(self.inv_hi)
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
            let (lo, hi) = (count.wrapping_add(2), count.wrapping_add(3));
            self.ds[usize::from(count)] = num::byte(self.items.len());
            self.ds[usize::from(scroll)] = num::byte(self.scroll);
            self.ds[usize::from(lo)] = num::byte(self.inv_lo);
            self.ds[usize::from(hi)] = num::byte(self.inv_hi);
            for (i, &id) in self.items.iter().enumerate() {
                self.ds[usize::from(list) + i] = id;
            }
            self.ds[usize::from(addr)] = v;
            let n = usize::from(self.ds[usize::from(count)]);
            let at = usize::from(list);
            self.items = self.ds[at..at + n].to_vec();
            self.scroll = usize::from(self.ds[usize::from(scroll)]);
            self.inv_lo = usize::from(self.ds[usize::from(lo)]);
            self.inv_hi = usize::from(self.ds[usize::from(hi)]);
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
