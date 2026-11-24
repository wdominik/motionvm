//! The player's record, `30h` bytes of the data segment, and the walker
//! the runtime keeps in its place.
//!
//! The original walks, turns, animates and draws the figure through these
//! bytes, the save copies them out whole (`0x14C73`) and the restore back
//! whole (`0x14D0C`). Here [`Walker`] holds what the routines read and
//! write, and the record is that walker written out: a read of any byte
//! of it is the walker's, and a write puts the bytes back into the walker.
//! Nothing of the record exists twice.
//!
//! ```text
//! +0    u8   kind         the walker's (3)                 0x0ACC8
//! +1    u8                carried                          0x0884D (read)
//! +2    u8   flags        bit 40h the walker's mirror;     0x0AC0C, 0x0AC1E
//!                         bit 0 live, bit 1, bit 80h carried, the snap
//!                         sets 1 and clears 2              0x0B796, 0x0B79B
//! +3    u8   align        carried                          0x0BC4B (read)
//! +4    u16  x, +6 y      the walker's                     0x0B747, 0x0B59A
//! +8    u8   bundle       carried (1, the figure file)     0x0BB87 (read)
//! +9    u8   phase        the walker's, or the animation's frame
//! +0Ah  u16  cel          +0Ch plus +9, or as the bytes gave it
//! +0Ch  u16  base         the walker's                     0x0AC2C, 0x14F66
//! +0Eh  u16  scale x, +10h scale y: the walker's one scale  0x0B783
//! +12h  u8   state        0 standing, 6 walking, 0Ah animating
//! +13h  u8   id           carried (0FFh)                   0x14FDD (read)
//! +14h  u8   factor       the walker's gait                0x0896D, 0x0AEF9
//! +15h  u8   phases       the walker's gait                0x089A7, 0x0BAC2
//! +16h  i16  countdown    the walker's                     0x08B5E
//! +18h  far  AN entry     carried: an address              0x14F92
//! +1Ch  u16               carried: no access to it found
//! +1Eh  u16  figure file  carried: the n of PLAY0n.DAT     0x10A58, 0x10A68
//! +20h  far  next         carried: an address              0x0AB8E
//! +24h  far  graph        carried: an address              0x07AA5
//! +28h  i16  node id      the walker's                     0x0AF65, 0x0B36E, 0x0B7B8
//! +2Ah  u16  layer        the walker's, low byte           0x0B7B4
//! +2Ch  i16  block        the walker's                     0x0AC30
//! +2Eh  i16  facing kept  the walker's                     0x0B09E, 0x14F4B
//! ```
//!
//! What is carried lives in the runtime's copy of the data segment, which
//! the image fills and the restore overwrites; the walker's fields are
//! written into it on every write so that a byte written alone lands among
//! the record as it stands. Three of the carried fields are far pointers
//! into the original's heap — the `AN` entry the figure was last bound
//! to, the next record of its chain, the room's walk graph — which this
//! runtime has no addresses for: they keep what the image or the last
//! restore put there.
//!
//! Before the first location the figure has no walker yet, and the record
//! is the bytes alone; the walker is made from them when the first entry
//! places it.

use super::{Adventure, Walker, objects, render};
use crate::num;

/// The record's length: the save's `push 30h` at `0x14C6E`.
pub(crate) const RECORD: u16 = 0x30;

/// `+1Eh`: the figure set the loader last loaded, the `n` of `PLAY0n.DAT`.
pub(crate) const FIGURE_SET: usize = 0x1E;

/// Bit 40h of `+2`.
const MIRROR_BIT: u8 = 0x40;

/// `+12h` while the walk runs (`0x0AF7E`).
const STATE_WALKING: u8 = 6;

fn word(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn put(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

/// The walker written over the record's carried bytes.
fn write_out(w: &Walker, b: &mut [u8]) {
    b[0x00] = w.kind;
    b[0x02] = (b[0x02] & !MIRROR_BIT) | if w.mirror { MIRROR_BIT } else { 0 };
    put(b, 0x04, num::low16(w.x));
    put(b, 0x06, num::low16(w.y));
    let animating = w.state == objects::STATE_FIGURE_ANIM;
    b[0x09] = if animating { w.anim_frame } else { w.phase };
    put(b, 0x0A, num::word(render::figure_cel(w).0));
    put(b, 0x0C, w.base);
    put(b, 0x0E, num::low16(w.scale));
    put(b, 0x10, num::low16(w.scale));
    b[0x12] = if w.walking && w.state == 0 {
        STATE_WALKING
    } else {
        w.state
    };
    b[0x14] = num::low8(num::low16(w.gait.speed));
    b[0x15] = w.gait.phases;
    put(b, 0x16, num::low16(w.anim_acc));
    put(b, 0x28, num::unsigned(w.dest_id));
    b[0x2A] = w.rec_layer;
    put(b, 0x2C, num::unsigned(w.block));
    put(b, 0x2E, num::unsigned(w.final_facing));
}

/// The record's bytes read into the walker. What the graph header holds
/// — the node index, the leg, the flood, the chain — is not in the record
/// and stays as it is.
fn read_in(w: &mut Walker, b: &[u8]) {
    w.kind = b[0x00];
    w.mirror = b[0x02] & MIRROR_BIT != 0;
    w.x = i32::from(num::signed(word(b, 0x04)));
    w.y = i32::from(num::signed(word(b, 0x06)));
    let state = b[0x12];
    w.walking = state == STATE_WALKING;
    w.state = if w.walking { 0 } else { state };
    if w.state == objects::STATE_FIGURE_ANIM {
        w.anim_frame = b[0x09];
    } else {
        w.phase = b[0x09];
    }
    w.base = word(b, 0x0C);
    // One scale: the two words are written together everywhere
    // (`0x0B783`, `0x0B5B7`), the second last.
    w.scale = i32::from(word(b, 0x10));
    w.gait.speed = i32::from(b[0x14]);
    w.gait.phases = b[0x15];
    w.anim_acc = i32::from(num::signed(word(b, 0x16)));
    w.dest_id = num::signed(word(b, 0x28));
    w.rec_layer = b[0x2A];
    w.block = num::signed(word(b, 0x2C));
    w.final_facing = num::signed(word(b, 0x2E));
    // `+0Ah` is held as given until the walk next writes it.
    let cel = word(b, 0x0A);
    w.cel_held = None;
    w.cel_held = (render::figure_cel(w).0 != usize::from(cel)).then_some(cel);
}

impl Adventure {
    /// Which byte of the player's record an address names.
    pub(super) fn player_record_at(&self, addr: u16) -> Option<usize> {
        let at = addr.checked_sub(self.ds_map.walker)?;
        (at < RECORD).then_some(usize::from(at))
    }

    /// The data segment's copy of the record with the walker written into
    /// it, so that it is the record as it stands.
    fn player_record_sync(&mut self) {
        if let Some(w) = &self.walker {
            let at = usize::from(self.ds_map.walker);
            write_out(w, &mut self.ds[at..at + usize::from(RECORD)]);
        }
    }

    /// One byte of the record.
    pub(super) fn player_record_byte(&self, i: usize) -> u8 {
        let at = usize::from(self.map().walker);
        let mut b = self.ds[at..at + usize::from(RECORD)].to_vec();
        if let Some(w) = &self.walker {
            write_out(w, &mut b);
        }
        b[i]
    }

    /// One byte of the record written: the record as it stands, the byte
    /// put in, the walker read back out of it.
    pub(crate) fn set_player_record_byte(&mut self, i: usize, v: u8) {
        self.player_record_sync();
        let at = usize::from(self.map().walker);
        self.ds[at + i] = v;
        if let Some(w) = &mut self.walker {
            read_in(w, &self.ds[at..at + usize::from(RECORD)]);
        }
    }

    /// The record's 30h bytes, as the save copies them.
    #[cfg(test)]
    pub(crate) fn player_record_bytes(&self) -> Vec<u8> {
        (0..usize::from(RECORD))
            .map(|i| self.player_record_byte(i))
            .collect()
    }

    /// The figure's walker: the one it has, or — before the first entry
    /// placed it — one made from the record's bytes.
    pub(crate) fn take_player(&mut self) -> Walker {
        if let Some(w) = self.walker.take() {
            return w;
        }
        let mut w = Walker::new(self.gait);
        let at = usize::from(self.ds_map.walker);
        read_in(&mut w, &self.ds[at..at + usize::from(RECORD)]);
        w
    }

    /// The snap's `or byte es:[bx+2], 81h ; and byte es:[bx+2], 0FDh`
    /// (`0x0B796`) on the figure's carried flag bits. Bit 80h is the
    /// marker's (`0883h:000Fh`, `0x08848`), which the drawer clears when it
    /// next draws the record (`0x0BC86`); this runtime draws every frame
    /// whole and keeps no marks, so it never sets it.
    pub(super) fn player_snapped(&mut self) {
        let at = usize::from(self.ds_map.walker) + 2;
        self.ds[at] = (self.ds[at] | 0x01) & !0x02;
    }
}
