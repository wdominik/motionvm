//! The records that walk besides the player, and the layer chains every
//! walking record hangs in.
//!
//! A room's people are 30h-byte records in the room file itself
//! ([`motionvm_parsec_formats::rdv::MoverRecord`]), not in the data segment. The
//! loader leaves them in no chain (every chain head is 0 in every file),
//! so they are neither moved nor drawn until the location's logic binds
//! one to a walk graph with `0A9Fh:0EA6h`. From then on the same routines
//! run them as run the player: the mover pass `0A9Fh:0892h`, the animator
//! `0883h:0305h` and the drawer `0BB6h:027Ah`, each over the layers'
//! chains in chain order.
//!
//! **The chains.** Each layer of the room has one, headed at `+10h` of
//! its directory record and linked through `+20h` of the records.
//! `0A9Fh:015Ah` puts a record at the **head** of the chain its `+2Ah`
//! names (`0x0AB8E`–`0x0AB9F`) and `0A9Fh:002Fh` takes it out
//! (`0x0AA1F`). The pass, the animator and the drawer all start at the
//! head, so the record linked last is drawn first, under the others.
//! [`Adventure::chains`] keeps that order, head first.
//!
//! The location logic reaches a person through the handle
//! [`Far::mover`], which names the record by its place in the room's
//! table — `30h` apart, so a field offset added to it stays the record's.

use crate::num;
use motionvm_render::Framebuffer;

use super::objects::{LIVE, MIRROR};
use super::{Adventure, Body, Far, Subject, Walker, align_point, anim, objects, scaled};
use crate::titles::walker::Link;

/// Bit 1 of `+2`: the drawer leaves a record out unless `(+2 & 3) == 1`
/// (`0x0BFC2`), and the snap clears it (`and byte es:[bx+2], 0FDh`,
/// `0x0B79B`).
const HIDDEN_BIT: u8 = 0x02;
/// Bit 7 of `+2`, which the snap sets with bit 0 (`or byte es:[bx+2],
/// 81h`, `0x0B796`).
const MARKED: u8 = 0x80;

/// One of the room's 30h-byte records as the program has it now.
#[derive(Debug, Clone)]
pub(crate) struct MoverRt {
    /// The walk: position, cel, gait, and the graph header's runtime.
    pub walker: Walker,
    /// `+2` but for bit 40h, which the walker keeps ([`Walker::mirror`]).
    pub flags: u8,
    /// `+3`.
    pub align: u8,
    /// `+8`: 0 in every shipped record, the room's own cels.
    pub bundle: u8,
    /// `+13h`.
    pub id: u8,
    /// `+24h`: the walk graph the record is bound to, by its index in the
    /// room's walk-graph resource.
    pub graph: Option<usize>,
}

impl MoverRt {
    /// `+2` as the program has it.
    pub(crate) fn flags(&self) -> u8 {
        (self.flags & !MIRROR) | if self.walker.mirror { MIRROR } else { 0 }
    }
}

impl Adventure {
    /// The room's records as the loader leaves them, and its chains empty.
    pub(super) fn rebuild_movers(&mut self) {
        let reach = self.gait.reach;
        let (movers, layers) = match self.room() {
            Some(room) => (
                room.room
                    .movers
                    .iter()
                    .map(|r| MoverRt {
                        walker: Walker::from_record(r, reach),
                        flags: r.flags,
                        align: r.align,
                        bundle: r.bundle,
                        id: r.id,
                        graph: None,
                    })
                    .collect(),
                room.room.layers.len(),
            ),
            None => (Vec::new(), 0),
        };
        self.movers = movers;
        self.chains = vec![Vec::new(); layers];
        if let Some(w) = &mut self.walker {
            w.link = None;
        }
    }

    /// The record at the head of a layer's chain.
    pub(crate) fn chain_head(&self, layer: usize) -> Option<Body> {
        self.chains.get(layer).and_then(|c| c.first().copied())
    }

    /// The record after `body` in the chain it hangs in now — which, for
    /// a record a pass has just moved to another layer, is that layer's
    /// old head: the loops read `+20h` after the step (`0x0B501`,
    /// `0x08BF3`).
    pub(crate) fn chain_next(&self, body: Body) -> Option<Body> {
        self.chains.iter().find_map(|c| {
            let at = c.iter().position(|&b| b == body)?;
            Some(c.get(at + 1).copied())
        })?
    }

    /// The chains of every record a walker call left an operation on,
    /// brought up to date: `002Fh` takes the record out wherever it hangs,
    /// `015Ah` puts it at the head of its layer's — or nowhere, for a
    /// layer the room does not have (`0x0AB75`, `jbe`).
    pub(crate) fn sync_links(&mut self) {
        let mut ops = Vec::new();
        if let Some(op) = self.walker.as_mut().and_then(|w| w.link.take()) {
            ops.push((Body::Figure, op));
        }
        for (i, m) in self.movers.iter_mut().enumerate() {
            if let Some(op) = m.walker.link.take() {
                ops.push((Body::Mover(i), op));
            }
        }
        for (body, op) in ops {
            for c in &mut self.chains {
                c.retain(|&b| b != body);
            }
            if let Link::Relink(layer) = op
                && let Some(c) = self.chains.get_mut(usize::from(layer))
            {
                c.insert(0, body);
            }
        }
    }

    /// `0A9Fh:0892h(delta)`: every chain's records stepped, layer by layer
    /// and head first (`0x0B2B9`–`0x0B525`). Nothing moves in a room
    /// without a walk-graph resource (`[2865h]`, `0x0B288`).
    pub(crate) fn mover_pass(&mut self, delta: i32) {
        if self.room().is_none_or(|r| r.walks.is_empty()) {
            return;
        }
        for layer in 0..self.chains.len() {
            let mut at = self.chain_head(layer);
            while let Some(body) = at {
                self.pass_member(body, delta);
                self.sync_links();
                at = self.chain_next(body);
            }
        }
    }

    /// One record of the pass. It is skipped without bit 0 of `+2`
    /// (`0x0B2D9`), without a graph at `+24h` (`0x0B2E3`) and while an
    /// animation of its own runs, `+12h == 0Ah` (`0x0B2EF`).
    fn pass_member(&mut self, body: Body, delta: i32) {
        let Some(room) = super::loaded_room(
            self.bound_map.as_ref(),
            self.edited_room.as_ref(),
            &self.rooms,
            self.slot(),
        ) else {
            return;
        };
        match body {
            Body::Figure => {
                if let Some(w) = self.walker.as_mut()
                    && w.state != objects::STATE_FIGURE_ANIM
                    && let Some(graph) = room.walk()
                {
                    w.pass(graph, delta);
                }
            }
            Body::Mover(i) => {
                if let Some(m) = self.movers.get_mut(i)
                    && m.flags & LIVE != 0
                    && let Some(graph) = m.graph.and_then(|g| room.walks.get(g))
                {
                    m.walker.pass(graph, delta);
                }
            }
            Body::Object(_) => {}
        }
    }

    /// `0BB6h:05B0h(id)` (`0x0C110`): the room's table first, by `+13h`
    /// (`0x0C150`–`0x0C165`), then every layer's chain
    /// (`0x0C187`–`0x0C1BA`) — which is where the player is found, by its
    /// id `FFh`, when it hangs in one — else null (`0x0C1C5`).
    pub(crate) fn find_mover(&self, id: u8) -> Far {
        if let Some(i) = self.movers.iter().position(|m| m.id == id) {
            return Far::mover(i);
        }
        let player_linked = self.chains.iter().flatten().any(|&b| b == Body::Figure);
        if id == anim::PLAYER_OBJECT && player_linked {
            return Far {
                off: self.ds_map.walker,
                seg: self.dgroup,
            };
        }
        Far::NULL
    }

    /// What `0BB6h:05B0h(id)` finds, as a subject: a person, or the player
    /// when it hangs in a chain.
    pub(crate) fn mover_subject(&self, id: u8) -> Option<Subject> {
        let found = self.find_mover(id);
        if let Some(i) = found.mover_index() {
            Some(Subject::Mover(i))
        } else if found != Far::NULL {
            Some(Subject::Player)
        } else {
            None
        }
    }

    /// `0A9Fh:0EA6h(rec, graph, node, facing)` (`0x0B896`) on a person.
    ///
    /// Without walk graphs, `[2865h]` null, it leaves at once (`0x0B89F`).
    /// A record already bound, `+24h` set, is taken out first (`0x0B8C4`,
    /// the near call to `0F42h` at `0x0B8D4`). `graph` must be below the
    /// header's count, else it leaves (`0x0B8EA`); then `+24h` takes the
    /// graph's header (`0x0B903`), the header's `+0Ch` takes the record
    /// (`0x0B917`), and the snap `0C59h` gives the answer (`0x0B928`).
    ///
    /// The two early exits leave `AX` at what the test left there; no
    /// call of Dunkle Schatten's reaches them (every one binds graph 1 of
    /// a room that has two), and this answers 0 for them. A node the graph
    /// does not have makes the snap answer 0 with the record out of every
    /// chain, but bound: `+24h` and the header's `+0Ch` are already
    /// written, and the header keeps the file's mode `FFFFh`.
    pub(crate) fn bind_mover(&mut self, i: usize, graph: u16, node: u16, facing: i16) -> i16 {
        let count = self.room().map_or(0, |r| r.walks.len());
        if count == 0 || i >= self.movers.len() {
            return 0;
        }
        if self.movers[i].graph.is_some() {
            self.detach_mover(i);
        }
        let g = usize::from(graph);
        if g >= count {
            return 0;
        }
        self.movers[i].graph = Some(g);
        self.snap_mover(i, node, facing)
    }

    /// `0A9Fh:0C59h(rec, node, facing)` (`0x0B649`) on a person bound to a
    /// graph: the walker's snap, with a facing below zero taken from the
    /// hotspot of the node's id turned round (`0x0B67D`–`0x0B6E5`), and
    /// `+2` live and shown (`0x0B796`, `0x0B79B`).
    pub(crate) fn snap_mover(&mut self, i: usize, node: u16, facing: i16) -> i16 {
        let Some(room) = super::loaded_room(
            self.bound_map.as_ref(),
            self.edited_room.as_ref(),
            &self.rooms,
            self.slot(),
        ) else {
            return 0;
        };
        let Some(m) = self.movers.get(i) else {
            return 0;
        };
        let Some(graph) = m.graph.and_then(|g| room.walks.get(g)) else {
            return 0;
        };
        let facing = if facing < 0 {
            i16::from(self.facing_for(room, i32::from(node)))
        } else {
            facing
        };
        let m = &mut self.movers[i];
        let ok = m.walker.snap_or_unlink(graph, num::signed(node), facing);
        if ok {
            m.flags = (m.flags | LIVE | MARKED) & !HIDDEN_BIT;
        }
        self.sync_links();
        i16::from(ok)
    }

    /// `0A9Fh:0617h(rec, node, facing)` (`0x0B007`) on a person, with the
    /// routine's own answer ([`Walker::walk_to_code`]). A record bound to
    /// no graph would have the routine read a header through a null
    /// pointer; no call of Dunkle Schatten's does that, and this answers 0.
    pub(crate) fn walk_mover(&mut self, i: usize, node: u16, facing: i16) -> i16 {
        let Some(room) = super::loaded_room(
            self.bound_map.as_ref(),
            self.edited_room.as_ref(),
            &self.rooms,
            self.slot(),
        ) else {
            return 0;
        };
        if room.walks.is_empty() {
            return 0;
        }
        let Some(m) = self.movers.get_mut(i) else {
            return 0;
        };
        let Some(graph) = m.graph.and_then(|g| room.walks.get(g)) else {
            return 0;
        };
        let answer = m.walker.walk_to_code(graph, num::signed(node), facing);
        self.sync_links();
        answer
    }

    /// `0A9Fh:1002h(rec, node)` (`0x0B9F2`) and `0A9Fh:105Ch(rec, node)`
    /// (`0x0BA4C`): every edge of `node` in the graph `rec` is bound to
    /// gets `+8 := value`, `3FFh` and 0. Nothing without walk graphs
    /// (`[2865h]`, `0x0B9F8`); the node is found as the near search at
    /// `0x0A9F4` finds it, from the last down, and one the graph does not
    /// have is left alone (`0x0BA3E`). The edges are its circular chain
    /// from the node's first (`+0Ah`), on through `+4` where the node is
    /// the edge's first end and `+6` where it is its second, until the
    /// first comes round again (`0x0B9A4`–`0x0B9F1`).
    ///
    /// The edges are the room's memory in the original, so the edit lasts
    /// until the slot is loaded again: here it goes into a copy of the
    /// loaded room made for this visit. A record that is neither the
    /// player nor a person, or one bound to no graph, is reported.
    pub(crate) fn set_node_edges(&mut self, rec: Far, node: u16, value: u16) {
        if self.room().is_none_or(|r| r.walks.is_empty()) {
            return;
        }
        let graph = if let Some(i) = rec.mover_index() {
            self.movers.get(i).and_then(|m| m.graph)
        } else if rec
            == (Far {
                off: self.ds_map.walker,
                seg: self.dgroup,
            })
        {
            Some(0)
        } else {
            None
        };
        let Some(g) = graph else {
            self.events.push(super::Event::Unbuilt(
                "0A9Fh:1002h",
                (u32::from(rec.off) << 16) | u32::from(node),
            ));
            return;
        };
        if self.bound_map.is_none() && self.edited_room.is_none() {
            self.edited_room = self.room().cloned();
        }
        let room = match (&mut self.bound_map, &mut self.edited_room) {
            (Some(m), _) => &mut m.room,
            (None, Some(r)) => r,
            (None, None) => return,
        };
        let Some(walk) = room.walks.get_mut(g) else {
            return;
        };
        let Some(n) = walk.nodes.iter().rposition(|x| x.id == node) else {
            return;
        };
        let first = walk.nodes[n].first_edge;
        let mut e = first;
        loop {
            let Some(edge) = walk.edges.get_mut(usize::from(e)) else {
                return;
            };
            edge.blocked = value;
            e = if usize::from(edge.a) == n {
                edge.next_a
            } else {
                edge.next_b
            };
            if e == first {
                return;
            }
        }
    }

    /// `0A9Fh:0F42h(rec)` (`0x0B932`) and `0A9Fh:0E7Fh(rec)` (`0x0B86F`)
    /// on a person: out of its chain. Neither touches `+24h`, the header,
    /// or the walk — a record taken out mid-leg stays where it is, because
    /// the pass does not reach it.
    pub(crate) fn detach_mover(&mut self, i: usize) {
        if let Some(m) = self.movers.get_mut(i) {
            m.walker.detach_draw_chain();
        }
        self.sync_links();
    }

    /// A byte of a person's record, for the fields the location logic
    /// reads: the kind, `+2`, `+3`, the position, the phase and cel, the
    /// scale, the state (6 while it walks), the id, the gait, the node id
    /// at `+28h` (`FFFFh` while it walks), the layer and the kept facing.
    pub(crate) fn mover_byte(&self, i: usize, field: u16) -> Option<u8> {
        let m = self.movers.get(i)?;
        let w = &m.walker;
        let word = |v: i32, hi: bool| -> u8 {
            let [lo, h] = num::low16(v).to_le_bytes();
            if hi { h } else { lo }
        };
        let (cel, _) = w.cel();
        Some(match field {
            0x00 => w.kind,
            0x02 => m.flags(),
            0x03 => m.align,
            0x04 | 0x05 => word(w.x, field & 1 != 0),
            0x06 | 0x07 => word(w.y, field & 1 != 0),
            0x08 => m.bundle,
            0x09 => w.phase,
            0x0A | 0x0B => word(num::count(cel), field & 1 != 0),
            0x0C | 0x0D => word(i32::from(w.base), field & 1 != 0),
            0x0E..=0x11 => word(w.scale, field & 1 != 0),
            0x12 => {
                if w.walking {
                    6
                } else {
                    0
                }
            }
            0x13 => m.id,
            0x14 => num::low8(num::low16(w.gait.speed)),
            0x15 => w.gait.phases,
            0x28 | 0x29 => word(i32::from(w.dest_id), field & 1 != 0),
            0x2A | 0x2B => word(i32::from(w.rec_layer), field & 1 != 0),
            0x2E | 0x2F => word(i32::from(w.final_facing), field & 1 != 0),
            _ => return None,
        })
    }

    /// Where a person's record resolves to (`0x0D2DA`): the same fields as
    /// an object's — the point at `+4`/`+6`, the cel `+0Ah` of bundle 0
    /// scaled by `+0Eh`/`+10h`, the alignment `+3` — whether or not the
    /// record hangs in a chain.
    ///
    /// Answers the point and the cel's scaled width and height.
    pub(crate) fn mover_box(&self, i: usize) -> Option<(i32, i32, i32, i32)> {
        let m = self.movers.get(i)?;
        let w = &m.walker;
        let (cel, _) = w.cel();
        let Some((cw, ch)) = self.object_cel_size(m.bundle, num::word(cel)) else {
            return Some((w.x, w.y, 0, 0));
        };
        let s = num::small(w.scale);
        let (width, height) = (scaled(cw, s), scaled(ch, s));
        let (x, y) = align_point(m.align, w.x, w.y, width, height);
        Some((x, y, width, height))
    }

    /// `0BB6h:000Ah` on a person (`0x0BB6A`): drawn like an object when
    /// `(+2 & 3) == 1` (`0x0BFC2`), its cel out of the bundle `+8` names.
    pub(super) fn draw_mover(&self, fb: &mut Framebuffer, i: usize) {
        let Some(m) = self.movers.get(i) else {
            return;
        };
        if m.flags & (LIVE | HIDDEN_BIT) != LIVE {
            return;
        }
        let w = &m.walker;
        let (cel, mirror) = w.cel();
        let Some(pic) = self.object_cel(m.bundle, num::word(cel)) else {
            return;
        };
        let s = num::small(w.scale);
        Self::blit_record(fb, pic, (w.x, w.y), (s, s), m.align, mirror);
    }
}
