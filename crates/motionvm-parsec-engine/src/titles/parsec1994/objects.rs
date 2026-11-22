//! The objects of the current location as they stand at run time, and
//! the library routines in segment `14DDh` that show, hide and bind them.
//!
//! The original works on the 32-byte records inside the loaded room blob
//! and rewrites them in place: the animator counts a frame into `+9`
//! (`0x08971`), the entry pass writes a state into `+12h` (`0x14E3A`), a
//! hide clears bit 0 of `+2` (`0x157C1`). This engine keeps the rooms
//! parsed and reloads nothing, so the fields the program writes need a
//! home of their own — one record per object, rebuilt from the file on
//! every arrival the way the loader's fresh copy is, and read by the
//! drawer, the animator and the rules instead of the file's bytes. The one
//! arrival that loads no fresh copy is from one forest row to the next:
//! the loader keeps the picture, and the records go on as the last row
//! left them, its object map taken off them (see
//! [`Adventure::bind_object_map`]).
//!
//! What is *not* taken from the file is what the original does not take
//! from the file either: on the way out of a location `14DDh:068Ah`
//! writes one byte per object of kinds 0, 1 and 5 into the location's
//! own 32-byte record at `DS:1F56h + 20h × [1F4Ch]` (`0x9EF`), and the
//! entry pass `14DDh:0293h` reads them back before the handler runs
//! (`0x151E5`). The sixty-one records are zero in the image
//! (`0x23E46`), which is what leaves every object as the file has it on
//! a first visit.

use crate::num;
use motionvm_parsec_formats::{MapDefaults, MapEntry, Overlay, RoomFile};

use super::{Adventure, Body, Event, Far, Picture};

/// Bit 0 of `+2`: the object is live — drawn, clickable, animated.
pub(crate) const LIVE: u8 = 1;
/// Bit 1 of `+2`: hidden by its animation (a frame of `FFh`). A live
/// object with this bit is not drawn: the draw pass wants `(+2 & 3) == 1`
/// (`0x0BFC2`).
pub(crate) const HIDDEN: u8 = 2;
/// Bit 6 of `+2`: drawn mirrored (`0x0BB79`).
pub(crate) const MIRROR: u8 = 0x40;

/// State `+12h` of an object bound to an `AN` entry of its own.
pub(crate) const STATE_SINGLE: u8 = 7;
/// State `+12h` of the head of a chain.
pub(crate) const STATE_HEAD: u8 = 8;
/// State `+12h` of a part of a chain, placed by its head.
pub(crate) const STATE_PART: u8 = 9;
/// State `+12h` of the figure while an animation of its own runs
/// (`0x14F9A`).
pub(crate) const STATE_FIGURE_ANIM: u8 = 0x0A;
/// State `+12h` a kind-8 record takes instead (`0x14F3D`); no such record
/// stands in Dunkle Schatten's rooms.
pub(crate) const STATE_NPC_ANIM: u8 = 0x0B;

/// One 32-byte object record as the program writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObjectRt {
    /// `+0`, the kind: 4 a door, 8 a walking figure, 0/1/5 the kinds the
    /// location record remembers.
    pub kind: u16,
    /// `+2`, the flags: [`LIVE`], [`HIDDEN`], [`MIRROR`].
    pub flags: u8,
    /// `+3`, the anchor: 1 centers the cel on the point, 2 centers it and
    /// stands it on the point, anything else draws from the point.
    pub align: u8,
    /// `+4`, `+6`.
    pub x: i32,
    /// See [`Self::x`].
    pub y: i32,
    /// `+8`: which loaded file `+0Ah` counts in — 0 the room's own table,
    /// 1 the player's cels.
    pub bundle: u8,
    /// `+9`: the frame, a cel counted from `+0Ch`.
    pub frame: u8,
    /// `+0Ah`: the cel drawn, `+0Ch` plus the frame (`0x0956B`).
    pub cel: u16,
    /// `+0Ch`: the cel the frames count from.
    pub cel_base: u16,
    /// `+0Eh`, `+10h`: the scale on each axis, `100h` and 0 both one.
    pub scale: (u16, u16),
    /// `+12h`: the state, 0 for a still object.
    pub state: u8,
    /// `+13h`: the id.
    pub id: u8,
    /// `+15h`: how many cels the object's block holds.
    pub phases: u8,
    /// `+16h`: the ticks until the animator looks at it again.
    pub countdown: i32,
    /// `+18h`: the `AN` entry it is bound to, as an index into the
    /// runtime's list of them.
    pub ani: Option<usize>,
    /// `+18h` to `+1Fh` before the entry pass binds anything there: the
    /// defaults an object map's entry takes when its new id is `FFh`.
    /// The binding clears them (`0x7F5B`), and the slot unload between
    /// two forest rows writes the entry the object was bound to back into
    /// them (`0x8712`–`0x8751`).
    pub defaults: MapDefaults,
    /// The layer whose list holds it, for the draw order.
    pub layer: u8,
}

impl ObjectRt {
    /// The record as the loader leaves it: the file's bytes, and `+1` set
    /// from `+15h` the way `0563h:0220h`'s caller does (`0x7994` region,
    /// "`+1 := (+15h > 1)`").
    pub(crate) fn from_file(o: &Overlay) -> Self {
        ObjectRt {
            kind: o.kind,
            flags: o.flags,
            align: o.align,
            x: i32::from(o.x),
            y: i32::from(o.y),
            bundle: o.bundle,
            frame: o.slot,
            cel: o.cel_index,
            cel_base: o.cel_base,
            scale: o.scale,
            state: 0,
            id: o.id,
            phases: o.init,
            countdown: 0,
            ani: None,
            defaults: o.map,
            layer: o.layer,
        }
    }

    /// `(+2 & 3) == 1`: what the draw pass draws (`0x0BFC2`, `0x0C020`).
    pub(crate) fn visible(&self) -> bool {
        self.flags & (LIVE | HIDDEN) == LIVE
    }
}

/// One hotspot as it stands at run time: its active bit follows the object
/// it names (`14DDh:074Bh`), and an animation may move its rectangle
/// (`0883h:13F2h`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HotspotRt {
    /// Bit 0 of the record's `+4`.
    pub active: bool,
    /// The rectangle, `+8` to `+0Eh`.
    pub x: i32,
    /// See [`Self::x`].
    pub y: i32,
    /// See [`Self::x`].
    pub w: i32,
    /// See [`Self::x`].
    pub h: i32,
}

/// A forest row's object map as the room loader bound it: the picture's
/// slot with the map's renaming applied, and what each entry bound.
#[derive(Debug, Clone)]
pub(crate) struct BoundMap {
    /// Slot 20's file with its hotspots' and its walk graphs' nodes' ids
    /// renamed — what [`Adventure::room`] answers while the row is loaded.
    pub room: RoomFile,
    /// The id the map gave each entry of the room's `AN` block, in
    /// directory order; `None` for one it left alone.
    pub ani_ids: Vec<Option<i16>>,
    /// The map's `+6`: what it renames, and what of the row's own it
    /// loads (bit 3, the song).
    pub flags: u16,
    /// The map's `+14h`: the room's song as the binding found it
    /// (`0x7C50`), which the slot unload puts back when the row loaded
    /// one of its own (`0x860C`).
    pub song: Far,
    /// Each entry as bound, beside the object and the hotspot it holds.
    bound: Vec<Binding>,
}

/// One entry of a [`BoundMap`]: the entry as the binding left it, and
/// the records whose `+0Ch` and `+10h` pointers it holds.
#[derive(Debug, Clone, Copy)]
struct Binding {
    entry: MapEntry,
    object: Option<usize>,
    hotspot: Option<usize>,
}

/// `0x8129`–`0x81E7`: every graph's nodes renamed, the graphs in the
/// header's order and each graph's nodes in its own.
fn bind_map_nodes(room: &mut RoomFile, entries: &[MapEntry]) {
    let mut taken = vec![false; entries.len()];
    for node in room.walks.iter_mut().flat_map(|g| g.nodes.iter_mut()) {
        if num::signed(node.id) <= 0 {
            continue;
        }
        if let Some(k) = (0..entries.len()).find(|&k| !taken[k] && entries[k].id == node.id) {
            taken[k] = true;
            node.id = entries[k].new_id;
        }
    }
}

impl Adventure {
    /// The records of the location just entered, as the loader's fresh
    /// copy has them, and the hotspots beside them.
    pub(super) fn rebuild_objects(&mut self) {
        let (objects, hotspots) = match self.room() {
            Some(room) => (
                room.room.overlays.iter().map(ObjectRt::from_file).collect(),
                room.hotspots
                    .spots
                    .iter()
                    .map(|h| HotspotRt {
                        active: h.active & 1 == 1,
                        x: i32::from(h.x),
                        y: i32::from(h.y),
                        w: i32::from(h.w),
                        h: i32::from(h.h),
                    })
                    .collect(),
            ),
            None => (Vec::new(), Vec::new()),
        };
        self.objects = objects;
        self.hotspots_rt = hotspots;
    }

    /// The row's object map bound to the records just loaded, as the room
    /// loader binds it (`0x7BFB`–`0x830C`). Only the Wald's and the maze's
    /// slots carry a map, so only those rows change what slot 20's picture
    /// holds; for any other row there is nothing bound. From one forest row
    /// to the next the loader keeps the picture (`0x7806`) and binds the
    /// next row's map onto the records as the slot unload left them
    /// ([`Self::unbind_object_map`]).
    ///
    /// For a forest row the loader first writes the row's exits into the
    /// map (`0x7CA7`–`0x7D7D`): entry `i` of the first four takes the id of
    /// the row's exit `i + 1` (the byte at `+0Ah + i` of its record); with
    /// no such exit it hides, and entry `i + 8` takes the id 2000; with one,
    /// entry `i + 4` hides under the id 0 and entry `i + 8` takes the exit's
    /// id plus 2000. So the picture's exits 101 to 104, their nodes and the
    /// twins at 2101 to 2104 take the ids the row's own exits have.
    ///
    /// Then every record of every layer, the layers in order and each
    /// layer's records from its last to its first, binds the first entry
    /// not yet bound to an object whose id is the record's (`0x7DB6`–
    /// `0x7DF5`). An entry whose new id is `FFh` takes the record's
    /// defaults first (`0x7DFC`–`0x7E6D`). The record takes the new id
    /// (`0x7EA4`); a cel below zero hides it, `FFh` sets its state to 3,
    /// any other is the frame it shows from its base (`0x7EC0`–`0x7F12`);
    /// the offset moves it (`0x7F28`, `0x7F3D`). Whether or not it bound,
    /// every record leaves with its defaults cleared and a state of 7, 8
    /// or 9 back at 0 (`0x7F5B`–`0x7F92`), for the entry pass to bind
    /// afresh.
    ///
    /// The map's `+6` flags then rename the rest of the picture, each kind
    /// of record binding entries of its own: bit 0 the hotspots
    /// (`0x7FB5`), whose id is the low byte of the mask at `+10h` — the
    /// mask's low byte is cleared and the new id ored in (`0x8039`,
    /// `0x8049`), a cel of zero or more makes the hotspot active and one
    /// below zero inactive (`0x8050`–`0x8064`), and the offset moves its
    /// rectangle (`0x8073`, `0x8081`); bit 5 every node of every walk
    /// graph whose id is above zero (`0x811E`, `0x8174`), which takes the
    /// new id (`0x81BA`); bit 4 every entry of the `AN` block without a
    /// chain (`0x826A`, `0x8296`), which takes the new id as the object it
    /// animates (`0x82E5`). A flag left clear takes that resource from the
    /// row's own slot instead (`0x80A2`, `0x81E9`, `0x8310`), which holds
    /// none: the picture's stays as it is.
    ///
    /// The byte at `+6` goes into the object's `+1` (`0x7EB9`), which only
    /// `0883h:000Fh` reads, to leave a still single-cel record's rectangle
    /// off the list of dirty ones (`0x884D`); this engine redraws the
    /// whole frame and keeps no such list.
    pub(super) fn bind_object_map(&mut self) {
        self.bound_map = None;
        let Ok(loc) = u16::try_from(self.loc) else {
            return;
        };
        let Some(map) = self.object_maps.get(usize::from(loc)).cloned().flatten() else {
            return;
        };
        let Some(mut room) = self
            .rooms
            .get(usize::from(self.room_slot(loc)))
            .cloned()
            .flatten()
        else {
            return;
        };
        let mut entries = map.entries;
        if self.forest_row(loc) {
            self.write_exits(loc, &mut entries);
        }
        let mut bound: Vec<Binding> = entries
            .iter()
            .map(|&entry| Binding {
                entry,
                object: None,
                hotspot: None,
            })
            .collect();
        self.bind_map_objects(&mut entries, &mut bound);
        if map.flags & 1 != 0 {
            self.bind_map_hotspots(&mut room, &entries, &mut bound);
        }
        if map.flags & 0x20 != 0 {
            bind_map_nodes(&mut room, &entries);
        }
        let mut ani_ids = Vec::new();
        if map.flags & 0x10 != 0
            && let Some(block) = &room.animations
        {
            let mut taken = vec![false; entries.len()];
            ani_ids = block
                .entries()
                .map(|a| {
                    if a.chain != 0 {
                        return None;
                    }
                    let id = num::unsigned(a.id);
                    let k = (0..entries.len()).find(|&k| !taken[k] && entries[k].id == id)?;
                    taken[k] = true;
                    Some(num::signed(entries[k].new_id))
                })
                .collect();
        }
        for (b, e) in bound.iter_mut().zip(&entries) {
            b.entry = *e;
        }
        self.bound_map = Some(BoundMap {
            room,
            ani_ids,
            flags: map.flags,
            song: Far::NULL,
            bound,
        });
    }

    /// `0x7CA7`–`0x7D7D`: a forest row's exits written into its map; see
    /// [`Self::bind_object_map`].
    fn write_exits(&self, loc: u16, entries: &mut [MapEntry]) {
        let row = usize::from(self.locations.table) * 16
            + usize::from(loc) * self.locations.stride
            + self.locations.exits;
        for i in 0..4 {
            let exit = self.image.bytes().get(row + 1 + i).copied().unwrap_or(0);
            let mut set = |k: usize, id: u16, cel: Option<i16>| {
                if let Some(e) = entries.get_mut(k) {
                    e.new_id = id;
                    if let Some(c) = cel {
                        e.cel = c;
                    }
                }
            };
            set(i, u16::from(exit), (exit == 0).then_some(-1));
            if exit == 0 {
                set(i + 8, 2000, Some(-1));
            } else {
                set(i + 4, 0, Some(-1));
                set(i + 8, u16::from(exit) + 2000, None);
            }
        }
    }

    /// `0x7D80`–`0x7FAF`: the objects' half of the binding.
    fn bind_map_objects(&mut self, entries: &mut [MapEntry], bound: &mut [Binding]) {
        let mut order: Vec<usize> = (0..self.objects.len()).collect();
        order.sort_by_key(|&i| (self.objects[i].layer, std::cmp::Reverse(i)));
        for i in order {
            let id = u16::from(self.objects[i].id);
            if let Some(k) =
                (0..entries.len()).find(|&k| bound[k].object.is_none() && entries[k].id == id)
            {
                bound[k].object = Some(i);
                let e = &mut entries[k];
                if e.new_id == 0xFF {
                    let d = self.objects[i].defaults;
                    e.new_id = u16::from(d.id);
                    e.cel = d.cel;
                    e.live = d.live;
                    e.dx = d.dx;
                    e.dy = d.dy;
                }
                let o = &mut self.objects[i];
                o.id = num::low8(e.new_id);
                if e.cel < 0 {
                    o.flags = 0;
                } else if e.cel == 0xFF {
                    o.flags = 3;
                } else {
                    o.frame = num::low8(num::unsigned(e.cel));
                    o.cel = o.cel_base.wrapping_add(u16::from(o.frame));
                    o.flags = LIVE;
                }
                o.x = i32::from(num::low16(o.x).wrapping_add(num::unsigned(e.dx)));
                o.y = i32::from(num::low16(o.y).wrapping_add(num::unsigned(e.dy)));
            }
            let o = &mut self.objects[i];
            o.defaults = MapDefaults::default();
            o.ani = None;
            if matches!(o.state, STATE_SINGLE | STATE_HEAD | STATE_PART) {
                o.state = 0;
            }
        }
    }

    /// `0x7FC0`–`0x809B`: the hotspots' half, in the table's order.
    fn bind_map_hotspots(
        &mut self,
        room: &mut RoomFile,
        entries: &[MapEntry],
        bound: &mut [Binding],
    ) {
        for (s, spot) in room.hotspots.spots.iter_mut().enumerate() {
            let id = spot.verb_mask & 0xFF;
            let Some(k) =
                (0..entries.len()).find(|&k| bound[k].hotspot.is_none() && entries[k].id == id)
            else {
                continue;
            };
            bound[k].hotspot = Some(s);
            let e = entries[k];
            spot.verb_mask = (spot.verb_mask & 0xFF00) | e.new_id;
            if let Some(h) = self.hotspots_rt.get_mut(s) {
                h.active = e.cel >= 0;
                h.x = i32::from(num::low16(h.x).wrapping_add(num::unsigned(e.dx)));
                h.y = i32::from(num::low16(h.y).wrapping_add(num::unsigned(e.dy)));
            }
        }
    }

    /// The slot unload between two forest rows, `0x86CE`–`0x87F6`: every
    /// record the map bound is given back what the binding took. An
    /// object takes the entry as its defaults (`0x8712`–`0x8751`), its own
    /// id back (`0x875D`) and its offset off (`0x876B`, `0x8779`); a
    /// hotspot its id back (`0x879B`, `0x87A1`) and its offset off
    /// (`0x87AF`, `0x87BD`); a node and an `AN` entry their ids back
    /// (`0x87D2`, `0x87E7`). The active bits, the objects' flags and
    /// frames stay as the visit left them: the next row's map binds onto
    /// the records as they are.
    ///
    /// The ids come back by the next binding starting again from the
    /// slot's file, which holds them; the records this engine keeps of
    /// its own are put back here. Answers whether there was a map.
    pub(crate) fn unbind_object_map(&mut self) -> bool {
        let Some(map) = self.bound_map.take() else {
            return false;
        };
        for b in &map.bound {
            let e = b.entry;
            if let Some(o) = b.object.and_then(|i| self.objects.get_mut(i)) {
                o.defaults = MapDefaults {
                    id: num::low8(e.new_id),
                    live: e.live,
                    cel: e.cel,
                    dx: e.dx,
                    dy: e.dy,
                };
                o.id = num::low8(e.id);
                o.x = i32::from(num::low16(o.x).wrapping_sub(num::unsigned(e.dx)));
                o.y = i32::from(num::low16(o.y).wrapping_sub(num::unsigned(e.dy)));
            }
            if let Some(h) = b.hotspot.and_then(|s| self.hotspots_rt.get_mut(s)) {
                h.x = i32::from(num::low16(h.x).wrapping_sub(num::unsigned(e.dx)));
                h.y = i32::from(num::low16(h.y).wrapping_sub(num::unsigned(e.dy)));
            }
        }
        self.bound_map = Some(map);
        true
    }

    /// `0BB6h:052Bh(id)`: the first object with this id, in the order the
    /// layers list them.
    pub(crate) fn object_index(&self, id: u8) -> Option<usize> {
        self.objects.iter().position(|o| o.id == id)
    }

    /// `14DDh:0007h(out, ani, id, state)`: the object with this id bound
    /// to an entry (`0x14E24`–`0x14E3A`): `+1` set, `+18h` the entry,
    /// `+12h` the state. Answers the object, or `None` when the location
    /// has none by that id — and then nothing is written, which is what
    /// leaves `out` null in the original.
    fn bind_object(&mut self, id: u8, ani: usize, state: u8) -> Option<usize> {
        let i = self.object_index(id)?;
        let o = &mut self.objects[i];
        o.state = state;
        o.ani = Some(ani);
        Some(i)
    }

    /// The animation half of `14DDh:0293h` (`0x150C6`–`0x151A5`): every
    /// entry of the room's block that names a room object is bound to it.
    /// In a chain, the first node whose `+2` has a zero low byte while no
    /// head is bound yet (`0x150EC`, `0x150F7`) becomes the head, in state
    /// 8 (`0x15108`), and the entry takes its id (`0x1510E`); every other
    /// node is a part, in state 9 (`0x15142`). An entry without a chain
    /// binds the object itself, in state 7 (`0x15170`).
    ///
    /// [`Self::settle_objects`] is the rest of the pass.
    pub(super) fn bind_animations(&mut self) {
        for ai in 0..self.anis.len() {
            let run = self.anis[ai].clone();
            if run.global || run.id <= 0 {
                continue;
            }
            let Some(block) = self.anim_block(false) else {
                return;
            };
            if run.chain == 0 {
                let id = num::low8(num::unsigned(run.id));
                if let Some(i) = self.bind_object(id, ai, STATE_SINGLE) {
                    self.anis[ai].head = Some(Body::Object(i));
                }
                continue;
            }
            let mut at = usize::from(run.chain);
            let mut head = false;
            while let Some(node) = block.node(at) {
                let id = num::low8(node.id);
                if node.kind & 0xFF == 0 && !head {
                    head = true;
                    self.anis[ai].id = i16::from(id);
                    if let Some(i) = self.bind_object(id, ai, STATE_HEAD) {
                        self.anis[ai].head = Some(Body::Object(i));
                    }
                } else {
                    self.bind_object(id, ai, STATE_PART);
                }
                at = usize::from(node.next);
            }
        }
    }

    /// The object half of `14DDh:0293h` (`0x151E5`–`0x1530A`), object by
    /// object in the layers' order: what the location's record remembers
    /// of it is put back, its state is settled, and its hotspot follows
    /// its live bit.
    ///
    /// For an object of kind 0 a remembered byte of 1 or 2 is its flags
    /// byte plus one (`0x15218`, `0x15221`). For kinds 1 and 5 a 1 is not
    /// live, a 2 live and hidden (`0x15235`, `0x1524E`), and anything above
    /// live (`0x15258`) on the frame the byte names less three
    /// (`0x15262`). An object in state 7 that is live and hidden shows no
    /// frame, `+9` set to `FFh` (`0x152C9`), and a state the binding left
    /// behind is cleared (`0x15302`).
    ///
    /// A zero byte leaves the object as the file has it, which is what
    /// every record holds until a visit writes it.
    pub(super) fn settle_objects(&mut self) {
        let here = usize::try_from(self.loc).unwrap_or(usize::MAX);
        let record = self.location_records.get(here).copied().unwrap_or([0; 32]);
        let mut at = 0usize;
        for i in 0..self.objects.len() {
            let kind = self.objects[i].kind;
            if matches!(kind, 0 | 1 | 5) {
                // The record is thirty-two bytes and the original does
                // not stop at its end; no shipped room has that many.
                let b = record.get(at).copied().unwrap_or(0);
                at += 1;
                let o = &mut self.objects[i];
                match (kind, b) {
                    (0, 1 | 2) => o.flags = b - 1,
                    (0, _) | (_, 0) => {}
                    (_, 1) => o.flags = 0,
                    (_, 2) => o.flags = LIVE | HIDDEN,
                    (_, b) => {
                        o.flags = LIVE;
                        o.frame = b.wrapping_sub(3);
                        o.cel = o.cel_base.wrapping_add(u16::from(o.frame));
                    }
                }
            }
            let o = self.objects[i].clone();
            match (o.state, o.ani) {
                (STATE_SINGLE, Some(ai)) => {
                    self.anis[ai].frame = if o.flags & (LIVE | HIDDEN) == LIVE | HIDDEN {
                        0xFF
                    } else {
                        o.frame
                    };
                }
                (STATE_SINGLE | STATE_HEAD | STATE_PART, None) => self.objects[i].state = 0,
                _ => {}
            }
            self.hotspot_follows(i);
        }
    }

    /// `14DDh:068Ah(record)`: the objects' state written into the
    /// location's record on the way out (`0x9EF`), one byte per object of
    /// kinds 0, 1 and 5 in the layers' order: for kind 0 its live bit plus
    /// one (`0x154B8`); for kinds 1 and 5 a 1 when it is not live
    /// (`0x154CE`), a 2 when it is hidden (`0x154D5`), and its frame plus
    /// three otherwise (`0x154EA`) — what [`Self::settle_objects`] reads
    /// back.
    pub(super) fn store_location_record(&mut self, loc: i32) {
        let Some(slot) = usize::try_from(loc)
            .ok()
            .and_then(|l| self.location_records.get_mut(l))
        else {
            return;
        };
        let mut at = 0usize;
        for o in &self.objects {
            let b = match o.kind {
                0 => (o.flags & LIVE) + 1,
                1 | 5 => {
                    if o.flags & LIVE == 0 {
                        1
                    } else if o.flags & HIDDEN != 0 {
                        2
                    } else {
                        o.frame.wrapping_add(3)
                    }
                }
                _ => continue,
            };
            if let Some(cell) = slot.get_mut(at) {
                *cell = b;
            }
            at += 1;
        }
    }

    /// `14DDh:074Bh(obj)`: the hotspot whose verb mask ends in the object's
    /// id takes the object's live bit as its own (`0x155B6`, `0x155C0`).
    /// The key is the id with bit 15 set (`0x15529`), matched against a
    /// mask's id and bit 15 (`0x15572`); objects of kinds 1 to 4 and 9 have
    /// no hotspot to move (`0x1553C`). A live object makes its hotspot
    /// active, bit 0 of `+4` (`0x15586`), any other inactive (`0x15590`).
    pub(crate) fn hotspot_follows(&mut self, i: usize) {
        let Some(o) = self.objects.get(i) else {
            return;
        };
        if o.id == 0 || matches!(o.kind, 1..=4 | 9) {
            return;
        }
        let key = u16::from(o.id) | 0x8000;
        let live = o.flags & LIVE != 0;
        let Some(room) = self.room() else {
            return;
        };
        let spot = room
            .hotspots
            .spots
            .iter()
            .position(|h| h.verb_mask & 0x80FF == key);
        if let Some(s) = spot
            && let Some(h) = self.hotspots_rt.get_mut(s)
        {
            h.active = live;
        }
    }

    /// `14DDh:084Ah(id)`: the object shown — its live bit set, and for the
    /// head of a chain every part's, hidden until the animator places
    /// them (`0x15687`, `or byte es:[bx+2], 3`). Answers whether there
    /// was an object.
    pub(crate) fn object_show_all(&mut self, id: u8) -> bool {
        let Some(i) = self.object_index(id) else {
            return false;
        };
        if self.objects[i].state == STATE_HEAD {
            if let Some(ai) = self.objects[i].ani
                && let Some(block) = self.anim_block(self.anis[ai].global)
            {
                let mut at = usize::from(self.anis[ai].chain);
                while let Some(node) = block.node(at) {
                    if let Some(p) = self.object_index(num::low8(node.id))
                        && self.objects[p].flags & LIVE == 0
                    {
                        self.objects[p].flags |= LIVE | HIDDEN;
                    }
                    at = usize::from(node.next);
                }
            }
        } else {
            self.objects[i].flags |= LIVE;
        }
        self.events.push(Event::ObjectShown(id));
        self.hotspot_follows(i);
        true
    }

    /// `14DDh:096Bh(id)`: the object hidden — its live bit cleared, and a
    /// chain's parts with it (`0x15791`, `0x157C1`).
    pub(crate) fn object_hide(&mut self, id: u8) {
        let Some(i) = self.object_index(id) else {
            return;
        };
        if self.objects[i].state == STATE_HEAD
            && let Some(ai) = self.objects[i].ani
            && let Some(block) = self.anim_block(self.anis[ai].global)
        {
            let mut at = usize::from(self.anis[ai].chain);
            while let Some(node) = block.node(at) {
                if let Some(p) = self.object_index(num::low8(node.id)) {
                    self.objects[p].flags &= !LIVE;
                }
                at = usize::from(node.next);
            }
        }
        self.objects[i].flags &= !LIVE;
        self.events.push(Event::ObjectHidden(id));
        self.hotspot_follows(i);
    }

    /// `14DDh:0A06h(id, k)`: the object shown on frame `k` (`0x157F1`).
    pub(crate) fn object_show_frame(&mut self, id: u8, k: i16) {
        let Some(i) = self.object_index(id) else {
            return;
        };
        self.objects[i].flags |= LIVE;
        self.object_frame(i, k);
        self.events.push(Event::ObjectShown(id));
        self.hotspot_follows(i);
    }

    /// `0883h:03EBh(obj, k)`: frame `k` on an object, or hidden for a `k`
    /// below zero (`0x08C28`); a state-7 object's entry takes the frame
    /// too (`0x08C4D`, `0x08CAF`).
    pub(crate) fn object_frame(&mut self, i: usize, k: i16) {
        let Some(o) = self.objects.get_mut(i) else {
            return;
        };
        if k < 0 {
            o.flags |= HIDDEN;
            if o.state == STATE_SINGLE
                && let Some(ai) = o.ani
            {
                self.anis[ai].frame = 0xFF;
            }
            return;
        }
        o.frame = num::low8(num::unsigned(k));
        o.cel = o
            .cel_base
            .wrapping_add(u16::from(num::low8(num::unsigned(k))));
        o.flags &= !HIDDEN;
        if o.state == STATE_SINGLE
            && let Some(ai) = o.ani
        {
            self.anis[ai].frame = num::low8(num::unsigned(k));
        }
    }

    /// The width and height of the cel an object draws, unscaled, or
    /// `None` when its bundle has no picture at that index. Bundle 0 is
    /// the room's own table, bundle 1 the player's cels (`0x10AA0`).
    pub(crate) fn object_cel_size(&self, bundle: u8, cel: u16) -> Option<(i32, i32)> {
        let pic = self.object_cel(bundle, cel)?;
        Some((i32::from(pic.width), i32::from(pic.height)))
    }

    /// The cel itself, for the drawer.
    pub(crate) fn object_cel(&self, bundle: u8, cel: u16) -> Option<&Picture> {
        match bundle {
            0 => self
                .room()?
                .room
                .object_cels
                .get(usize::from(cel))?
                .as_ref(),
            1 => self.figure_cels.get(usize::from(cel)),
            _ => None,
        }
    }
}
