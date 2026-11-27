//! Records that live in the program's own memory, and the roll a location
//! runs through one.
//!
//! A location's logic can keep a record of the animator's and the
//! drawer's shape in its own data segment and link it into a layer's
//! chain: *Bi-Fi 2*'s Vorführraum links `2446h:0960h` with `05DCh:0D6Ah`
//! (`0x16882`). Such a record is [`Body::Data`]; its fields are the
//! image's bytes, read and written where the original has them, so what
//! the logic and the library write there stays from one visit to the
//! next as it does in the original. Its `+20h` is the one field the
//! runtime does not keep in the image: the chain's order is
//! [`Adventure::chains`]'s.
//!
//! What such a record does in the passes:
//!
//! - The mover pass skips it, its walk graph `+24h` being null
//!   (`0x06640`–`0x06662`, [`Adventure::mover_pass`]).
//! - The animator counts the ticks off its countdown `+16h` while it is
//!   live and has a state (`0x042C4`–`0x042DF`), and steps it when none
//!   are left. A state above the step's arms goes to the row's record
//!   hook, with code 0 ([`super::profile::Profile::anim_arms`]).
//! - The drawer calls the record hook with code 1 instead of drawing a
//!   cel when `+2` has bit 4 (`0x071ED`, `0x0724B`) — the drawing the
//!   hook does is the location's. [`Adventure::custom_draw_pass`] makes
//!   those calls once a frame, after the animator, and what they draw is
//!   painted at the record's place in the picture ([`Adventure::render_scene`]).
//!
//! The library routines of segment `0284h` keep one roll of lines
//! ([`RoomRoll`]): laid out (`0284h:015Eh`), its cursor kept
//! (`0284h:0001h`), drawn (`0284h:0079h`) and freed (`0284h:029Ah`). The
//! drawing is a picture the drawer's pass leaves on the back buffer; here
//! it is latched by the call and painted by the next composition.

use super::anim::Body;
use super::objects::{HIDDEN, LIVE};
use super::title::roll::{Cursor, Roll};
use super::{Adventure, Event, Far};
use crate::num;

/// Bit 4 of `+2`: the drawer calls the record hook with code 1 instead
/// of drawing the record's cel (`test byte es:[bx+2], 10h`, `0x071ED`,
/// `0x0724B`; *Dunkle Schatten*'s `0x0BFCA`, `0x0C028`).
pub(crate) const CUSTOM_DRAW: u8 = 0x10;

/// The roll the `0284h` routines keep.
#[derive(Debug, Clone)]
pub(crate) struct RoomRoll {
    /// The lines as `0284h:015Eh` laid them out.
    pub roll: Roll,
    /// `[1D2Ah]`, the line the roll is drawn from.
    pub cursor: Cursor,
    /// `[1D28h]`, the position the cursor was last taken to.
    pub pos: i32,
    /// What the drawer's pass drew this frame: for each call of
    /// `0284h:0079h`, the record the drawer was at, and the cursor and
    /// the position it drew from.
    pub shown: Vec<(Body, Cursor, i32)>,
}

impl Adventure {
    /// Whether `at` is a record in the program's own memory: not the data
    /// group, which the runtime keeps apart, and none of the handles the
    /// runtime gives out for the records it keeps in its own shape.
    pub(crate) fn is_image_record(&self, at: Far) -> bool {
        at != Far::NULL && at.seg != self.dgroup && !at.is_handle()
    }

    /// The image's byte at `rec + field`.
    pub(crate) fn image_byte(&self, rec: Far, field: u16) -> u8 {
        let at = usize::from(rec.seg) * 16 + usize::from(rec.off.wrapping_add(field));
        self.image.bytes().get(at).copied().unwrap_or(0)
    }

    /// The image's byte at `rec + field` set.
    pub(crate) fn set_image_byte(&mut self, rec: Far, field: u16, v: u8) {
        let at = usize::from(rec.seg) * 16 + usize::from(rec.off.wrapping_add(field));
        self.image.poke(at, v);
    }

    /// The image's word at `rec + field`.
    pub(crate) fn image_word(&self, rec: Far, field: u16) -> u16 {
        u16::from_le_bytes([
            self.image_byte(rec, field),
            self.image_byte(rec, field.wrapping_add(1)),
        ])
    }

    /// The image's word at `rec + field` set.
    pub(crate) fn set_image_word(&mut self, rec: Far, field: u16, v: u16) {
        let [lo, hi] = v.to_le_bytes();
        self.set_image_byte(rec, field, lo);
        self.set_image_byte(rec, field.wrapping_add(1), hi);
    }

    /// `05DCh:0D6Ah(rec)` (`0x06B2A`) on a record in the image: its
    /// animation stopped (`0F48h:01A1h`), taken out of any chain
    /// (`0x05DF9`), and put at the head of the chain its `+2Ah` names
    /// when the room has that layer (`0x05F24`, `cmp [es:bx+0Ch], cx ;
    /// jna`, `0x05F3F`).
    ///
    /// The stop acts only on a record in state `0Ah` (`0x0F628`), an
    /// animation bound to an `AN` entry, which no record in the image
    /// is; one would be told as not built.
    pub(crate) fn link_image_record(&mut self, rec: Far) {
        self.stop_image_record(rec, "05DCh:0D6Ah");
        let body = Body::Data(rec);
        for c in &mut self.chains {
            c.retain(|&b| b != body);
        }
        let layer = usize::from(self.image_word(rec, 0x2A));
        if let Some(c) = self.chains.get_mut(layer) {
            c.insert(0, body);
        }
    }

    /// `05DCh:0D95h(rec)` (`0x06B55`) on a record in the image: its
    /// animation stopped and the record out of its chain (`0x05DF9`).
    /// `false` for a record no chain holds, which is not one the logic
    /// linked.
    pub(crate) fn unlink_image_record(&mut self, rec: Far) -> bool {
        let body = Body::Data(rec);
        if !self.chains.iter().flatten().any(|&b| b == body) {
            return false;
        }
        self.stop_image_record(rec, "05DCh:0D95h");
        for c in &mut self.chains {
            c.retain(|&b| b != body);
        }
        true
    }

    /// `0F48h:01A1h(rec)` on a record in the image: nothing unless its
    /// state is `0Ah` (`cmp byte es:[bx+12h], 0Ah ; jz`, `0x0F628`).
    fn stop_image_record(&mut self, rec: Far, caller: &'static str) {
        if self.image_byte(rec, 0x12) == super::objects::STATE_FIGURE_ANIM {
            self.events.push(Event::Unbuilt(
                caller,
                (u32::from(rec.seg) << 16) | u32::from(rec.off),
            ));
        }
    }

    /// The far address the animator and the drawer hand the record hook
    /// for a body: its record as the logic would name it.
    pub(crate) fn body_far(&self, body: Body) -> Far {
        match body {
            Body::Object(i) => self
                .objects
                .get(i)
                .map_or(Far::NULL, |o| Far::object(u16::from(o.id))),
            Body::Figure => Far {
                off: self.ds_map.walker,
                seg: self.dgroup,
            },
            Body::Mover(i) => Far::mover(i),
            Body::Data(rec) => rec,
        }
    }

    /// The drawer's calls of the record hook, once a frame after the
    /// animator: the frame's drawing forgotten, then, layer by layer as
    /// the drawer goes (`0BB6h:027Ah`'s order), every object of the layer
    /// and then every record of its chain that is live, not hidden and
    /// has [`CUSTOM_DRAW`] set, the hook called with it and code 1
    /// (`push 1 ; push seg ; push bx ; call far [2D1Ch]`, `0x071FD`,
    /// `0x0725B`). Without a hook such a record draws nothing
    /// (`0x071FB`, `0x07259`).
    pub(crate) fn custom_draw_pass(&mut self) {
        if let Some(r) = &mut self.room_roll {
            r.shown.clear();
        }
        let Some(table) = self.logic_table else {
            return;
        };
        let custom = |flags: u8| flags & (LIVE | HIDDEN) == LIVE && flags & CUSTOM_DRAW != 0;
        let layers = self.room().map_or(0, |r| r.room.layers.len());
        let mut calls = Vec::new();
        for layer in 0..layers {
            for (i, o) in self.objects.iter().enumerate() {
                if usize::from(o.layer) == layer && custom(o.flags) {
                    calls.push(Body::Object(i));
                }
            }
            let mut at = self.chain_head(layer);
            while let Some(body) = at {
                if custom(self.chain_record_flags(body)) {
                    calls.push(body);
                }
                at = self.chain_next(body);
            }
        }
        for body in calls {
            self.drawing = Some(body);
            let far = self.body_far(body);
            table.record_hook(self, far, 1);
        }
        self.drawing = None;
    }

    /// `+2` of a chain's record as the drawer reads it.
    pub(crate) fn chain_record_flags(&self, body: Body) -> u8 {
        match body {
            Body::Object(i) => self.objects.get(i).map_or(0, |o| o.flags),
            Body::Figure => self.ds[usize::from(self.ds_map.walker) + 2],
            Body::Mover(i) => self.movers.get(i).map_or(0, |m| m.flags()),
            Body::Data(rec) => self.image_byte(rec, 2),
        }
    }

    /// `0284h:015Eh`: the roll laid out from the far pointers at `row`, a
    /// row that ends at a null one, with the font every text of the
    /// program is set in — the record `[19E0h]` names, whose font words
    /// are written once, at the start (`0x01434`, `0x01442`). The cursor
    /// stands on the first line (`0x029C9`–`0x029D3`). Its answer is where
    /// the roll ends.
    pub(crate) fn roll_layout_impl(
        &mut self,
        row: Far,
        spec: &super::title::roll::RollSpec,
    ) -> u16 {
        let at = |i: u16| Far {
            off: row.off.wrapping_add(i),
            seg: row.seg,
        };
        let mut texts = Vec::new();
        for n in 0..0x1000u16 {
            let p = n.wrapping_mul(4);
            let w = |api: &mut Self, k: u16| {
                u16::from_le_bytes([
                    super::Api::far_byte(api, at(p + k)),
                    super::Api::far_byte(api, at(p + k + 1)),
                ])
            };
            let line = Far {
                off: w(self, 0),
                seg: w(self, 2),
            };
            if line == Far::NULL {
                break;
            }
            let mut text = Vec::new();
            for k in 0..0x100u16 {
                let b = super::Api::far_byte(
                    self,
                    Far {
                        off: line.off.wrapping_add(k),
                        seg: line.seg,
                    },
                );
                if b == 0 {
                    break;
                }
                text.push(b);
            }
            texts.push(text);
        }
        let roll = Roll::layout(&texts, &self.font, *spec);
        let end = roll.end();
        self.room_roll = Some(RoomRoll {
            roll,
            cursor: Cursor::default(),
            pos: 0,
            shown: Vec::new(),
        });
        num::low16(end)
    }

    /// `0284h:0001h(pos)`: the cursor taken to `pos`, which is kept
    /// (`mov [1D28h], si`, `0x028B0`); the weights it passed
    /// ([`Cursor::advance`]).
    pub(crate) fn roll_seek_impl(&mut self, pos: u16) -> u16 {
        let Some(r) = &mut self.room_roll else {
            self.events
                .push(Event::Unbuilt("0284h:0001h without a roll", u32::from(pos)));
            return 0;
        };
        let pos = i32::from(num::signed(pos));
        let weight = r.cursor.advance(&r.roll, pos);
        r.pos = pos;
        num::low16(weight)
    }

    /// `0284h:0079h`: the roll drawn at the kept position, from the
    /// cursor's line, at the record the drawer is at.
    pub(crate) fn roll_draw_impl(&mut self) {
        let Some(body) = self.drawing else {
            self.events
                .push(Event::Unbuilt("0284h:0079h outside the drawer", 0));
            return;
        };
        match &mut self.room_roll {
            Some(r) => {
                let (cursor, pos) = (r.cursor, r.pos);
                r.shown.push((body, cursor, pos));
            }
            None => self
                .events
                .push(Event::Unbuilt("0284h:0079h without a roll", 0)),
        }
    }

    /// What the drawer's pass drew at `body` this frame, painted.
    pub(crate) fn paint_custom(&self, fb: &mut motionvm_render::Framebuffer, body: Body) {
        let Some(r) = &self.room_roll else {
            return;
        };
        for &(_, cursor, pos) in r.shown.iter().filter(|(b, ..)| *b == body) {
            r.roll.draw_from(fb, &self.font, cursor, pos);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{Adventure, CUSTOM_DRAW};

    /// Every record the room files hold, objects and people, and the
    /// player's record in the data group, that the animator would hand
    /// the record hook — a state above `arms` — or the drawer would:
    /// [`CUSTOM_DRAW`] set. Each as its slot (the player's none),
    /// `"object"`, `"mover"` or `"player"`, its index, `+2` and `+12h`.
    pub(crate) fn hooked_records(
        adv: &Adventure,
        arms: u8,
    ) -> Vec<(usize, &'static str, usize, u8, u8)> {
        let mut out = Vec::new();
        let w = usize::from(adv.ds_map.walker);
        let (flags, state) = (adv.ds[w + 2], adv.ds[w + 0x12]);
        if state > arms || flags & CUSTOM_DRAW != 0 {
            out.push((usize::MAX, "player", 0, flags, state));
        }
        for (slot, room) in adv.rooms.iter().enumerate() {
            let Some(room) = room else { continue };
            let records = room
                .room
                .overlays
                .iter()
                .map(|o| ("object", o.flags, o.state))
                .chain(room.room.movers.iter().map(|m| ("mover", m.flags, m.state)));
            for (i, (kind, flags, state)) in records.enumerate() {
                if state > arms || flags & CUSTOM_DRAW != 0 {
                    out.push((slot, kind, i, flags, state));
                }
            }
        }
        out
    }

    /// The far pointer at `field` of every row of the location table that
    /// is not null, by row.
    pub(crate) fn row_far_pointers(adv: &Adventure, field: usize) -> Vec<(usize, (u16, u16))> {
        let l = &adv.locations;
        let b = adv.image.bytes();
        let word = |at: usize| u16::from_le_bytes([b[at], b[at + 1]]);
        (0..l.count)
            .filter_map(|n| {
                let at = usize::from(l.table) * 16 + n * l.stride + field;
                let far = (word(at + 2), word(at));
                (far != (0, 0)).then_some((n, far))
            })
            .collect()
    }
}
