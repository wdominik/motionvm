//! What a script asks of the figure.
//!
//! The location hook (`0EB9h:1968h`) reaches the walker through a handful
//! of library routines in segment `0A9Fh`; these are those calls as this
//! engine has them. The walk itself and the exit are the mode machine's
//! ([`super::mode`]).

use super::{Adventure, Api, Event, Walker};
use crate::num;

impl Adventure {
    /// `0A9Fh:0C0Ch(1693h, dir)`: the figure faces direction `dir`.
    ///
    /// Not for a record of kind 8 (`0x0B613`). Sample 2 is asked for
    /// (`0EB9h:1949h`, `0x0B619`), an animation on the record stopped
    /// (`14DDh:01D4h`, `0x0B627`), and the standing cel of `dir + 3` set
    /// (`0x0B641`, a near call to `02CFh`).
    ///
    /// The hook's arms call this with what they call a node; it is a
    /// direction, and `02CFh` on the figure's record is `01B9h`, which
    /// picks the block of cels by it.
    pub(crate) fn face_player(&mut self, dir: i16) {
        if self.room().and_then(|r| r.walk()).is_none() {
            return;
        }
        self.effect_1949(2);
        self.stop_player_anim();
        if let Some(w) = &mut self.walker {
            w.face(dir);
        }
    }

    /// `0A9Fh:10F7h(k)`: a turn animation on the figure.
    ///
    /// Nothing happens unless the figure is linked, `[16BBh]` not `FFFFh`
    /// (`0x0BAF8`); a running animation is stopped (`14DDh:01D4h`,
    /// `0x0BB03`). Then `k` picks the animation: 1 is animation 0 and only
    /// at facing 0 (`0x0BB21`), 2 is animation 1 (`0x0BB2A`), 3 is
    /// animation 1 and only at facing 0 (`0x0BB2F`). Sample 2 is asked for
    /// (`0EB9h:1949h`, `0x0BB39`) and the animation started (`0883h:20D9h`,
    /// `0x0BB46`).
    ///
    /// `[16BFh]` is the record's `+2Ch`, the block base
    /// `facing × (phases + 1)`, so zero is facing 0. Answers 1 when an
    /// animation started.
    pub(crate) fn turn_player(&mut self, k: i16) -> i16 {
        if !self.player_linked() {
            return 0;
        }
        self.stop_player_anim();
        let facing = self.player_facing();
        let anim = match k {
            1 if facing != 0 => return 0,
            3 if facing != 0 => return 0,
            2 | 3 => 1,
            _ => 0,
        };
        self.effect_1949(2);
        self.anim_player(anim);
        1
    }

    /// `0A9Fh:0C59h(1693h, id, facing)`: the figure put on the node with
    /// `id`, facing `facing` — or, below zero, the way the hotspot with
    /// that id says, turned round. Answers whether the node exists.
    pub(crate) fn snap_player(&mut self, id: i16, facing: i16) -> bool {
        self.stop_player_anim();
        let mut w = self.take_player();
        let ok = match self.room().and_then(|r| r.walk().map(|g| (r, g))) {
            Some((room, graph)) => {
                let facing = if facing < 0 {
                    i16::from(self.facing_for(room, i32::from(id)))
                } else {
                    facing
                };
                w.snap_or_unlink(graph, id, facing)
            }
            None => false,
        };
        self.walker = Some(w);
        if ok {
            self.player_snapped();
        }
        self.sync_links();
        ok
    }

    /// `0C3Fh:095Ch(id)`: item `id` given, unless the player has it
    /// (`0x0CD5A`, `0B06h`) or it is not an item (`0x0CD4F`, `17h` and up).
    pub(crate) fn give_item(&mut self, id: u8) {
        if u16::from(id) >= self.items_end() || self.carried(id) {
            return;
        }
        self.give(id);
        self.events.push(Event::ItemGiven(id));
    }

    /// `[16BBh] != -1`: the figure stands on a node of the graph. The
    /// record's `+28h` is the node's id, and the walk puts `0FFFFh` there
    /// while it moves (`0x0B375`), so this is false mid-walk too.
    pub(crate) fn player_linked(&self) -> bool {
        self.player_node_id() != -1
    }

    /// `[16BBh]`, the record's `+28h`: the id of the node the figure
    /// stands on, −1 while it walks (`0x0AF65`). Only the snap and the
    /// arrival write another (`0x0B7B8`, `0x0B36E`), so leaving a room
    /// keeps it.
    pub(crate) fn player_node_id(&self) -> i16 {
        num::signed(self.word(self.ds_map.walker.wrapping_add(0x28)))
    }

    /// `[16BFh] / ([16A8h] + 1)`: the figure's facing, read back out of
    /// its cel block.
    pub(crate) fn player_facing(&self) -> i16 {
        self.walker.as_ref().map_or(2, Walker::facing)
    }
}
