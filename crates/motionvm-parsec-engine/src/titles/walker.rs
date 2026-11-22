//! The player walker, read at instruction level.
//!
//! The flood (`0x0AD23`), the leg setup (`0A9Fh:0404h`,
//! `0x0AF0B`–`0x0AF2D`), the snap (`0A9Fh:0C59h`), the retarget
//! (`0x0B13D`) and the mover (`0724h:0035h`, `0x07275`) are read, and the
//! items below cite each instruction where they reproduce it.
//!
//! The original keeps its runtime state inside the loaded walk-graph blob
//! (header `+10h`..`+26h`) and a walker record in the data segment; this
//! module keeps the same quantities in named fields. The record's own
//! bytes are the init: feet (100, 100), align 2, scale `100h`, and the
//! gait, which is read out of the record (see [`crate::titles::mover`]).
//!
//! The mover writes the position it interpolated into the graph header's
//! `+28h`/`+2Ah`/`+2Ch` (`0x072B3` onwards), and the record takes it over
//! later (`0x0B593`).
//!
//! That "later" is the animator's, and it is measured: the original's
//! mover pass `0A9Fh:0892h` and animator `0883h:0305h`, run under an
//! 8086 emulator on Dunkle Schatten's own bytes, move the record only
//! when the animator takes a phase step, and leave `+28h` at `FFFFh` for
//! one pass after the last leg ends. [`Walker::pass`] and
//! [`Walker::animate_walk`] are those two halves.
//!
//! The same machinery walks the people of a room. Their records are
//! the room's own 30h-byte ones, and only the record's kind (`+0`) tells
//! them apart from the player: it decides what a direction does to the cel
//! (`0A9Fh:02CFh`) and which cel stands at the end of a walk.

use crate::num;
use motionvm_parsec_formats::rdv::{MoverRecord, WalkGraph};

/// The player's kind (`+0` of `DS:1693h`): `02CFh` gives it direction
/// blocks (`0A9Fh:01B9h`, the table at `cs:0315h`, `0x0ACD9`).
pub(crate) const KIND_PLAYER: u8 = 3;

/// What a walk does to the chain the record hangs in, for the caller that
/// keeps the chains in order.
///
/// `0A9Fh:002Fh` takes a record out of its layer's chain, `0A9Fh:015Ah`
/// puts it at the **head** of the chain `+2Ah` names (`0x0AB8E`–`0x0AB9F`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Link {
    /// `002Fh`, then `015Ah` into this layer.
    Relink(u8),
    /// `002Fh` alone.
    Unlink,
}

/// What a walker's two animation bytes mean, once they are read.
///
/// Both come out of the record the program ships — see
/// [`crate::titles::mover`] — rather than being written down here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Gait {
    /// The walker's `+14h`, the interval, which does two jobs: the leg
    /// setup reads it (`0x0AEF9`) and multiplies the edge's length by it
    /// (`0x0AF0B`–`0x0AF10`), and the animator reads it into `SI`
    /// (`0x0896D`) and adds it back onto the countdown `+16h` every time it
    /// advances a phase (`0x08AF7`). So the gait is tied to the speed — one
    /// phase per `+14h` ticks.
    pub speed: i32,
    /// Walk phases per direction block (`+15h`): the animator wraps the
    /// phase `+9` at it (`0x089A7`), and the snap divides the block `+2Ch`
    /// by it plus one to get the facing back (`0x0BAC2`–`0x0BAD0`). A block
    /// holds one more cel than that; the extra one is the standing frame.
    pub phases: u8,
    /// The dividend the t-step is built from — the constant the leg setup
    /// pushes as a long before it calls the runtime's divide
    /// (`0x0AF15`–`0x0AF19`).
    ///
    /// Dunkle Schatten pushes `2FFFAh`. A leg then costs `7FFFh / tstep`
    /// ticks, which is `7FFFh * len * interval / dividend` — proportional
    /// to the edge's length, which is what an evenly paced walk looks
    /// like.
    pub reach: i32,
}

/// One leg of a walk: the interpolation state `0A9Fh:0404h` writes into the
/// graph header (`+18h`..`+26h`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Leg {
    /// Via-edge index (`header+12h`).
    pub via: usize,
    /// From-node A's x, y, scale (`+1Ch`/`+20h`/`+24h` — always endpoint
    /// A; walking B→A runs the same interpolation backwards).
    pub from: (i32, i32, i32),
    /// `2*(B−A)` for x, y, scale (`+1Eh`/`+22h`/`+26h`).
    pub delta2: (i32, i32, i32),
    /// Signed per-frame t step (`+18h`), negative when walking B→A.
    pub tstep: i32,
    /// t (`+1Ah`): 0..7FFFh forward, 7FFFh..0 backward.
    pub t: i32,
}

/// The walker record (`DS:1693h`) plus the graph-header runtime fields.
#[derive(Debug, Clone)]
pub(crate) struct Walker {
    /// Feet position (`+4`/`+6`). Draw with align 2: center x, feet y.
    pub x: i32,
    /// Feet y.
    pub y: i32,
    /// Cel scale, base `100h` (`+0Eh`/`+10h` — the snap and the copy at
    /// `0x0B593` write both).
    pub scale: i32,
    /// `+2Ch` of a kind-3 record: the block of its facing, `facing ×
    /// (+15h + 1)`. The facing is read back out of it ([`Self::facing`],
    /// `0x0ABC3`, `0x0BACA`), and a turn moves `+0Ah` and `+0Ch` by the
    /// difference to the new block ([`Self::turn`]); it is not the block
    /// the cel counts from, which is `+0Ch`.
    pub block: i16,
    /// Walk-cycle phase 0..`+15h`−1 (`+9`), advanced at each of the
    /// animator's phase steps while walking, wrapping
    /// (`0x0899A`–`0x089C4`).
    pub phase: u8,
    /// The record walks: the animator steps it
    /// ([`Self::animate_walk`]) and the walk cel shows.
    pub walking: bool,
    /// `+16h`, the animation countdown. `0883h:0305h` subtracts the
    /// elapsed ticks from it every frame, and while that leaves it at or
    /// below 0 the animator puts `+14h` back on and advances one phase.
    /// Nothing resets it between legs — the gait runs free — but the
    /// arrival (`0x0B32F`) zeroes it.
    pub anim_acc: i32,
    /// Current node index (`header+14h`), if the walker stands on the
    /// graph of the current room.
    pub node: Option<usize>,
    /// Which layer chain the walker hangs in (`+2Ah`), or `None` while it
    /// hangs in none.
    ///
    /// `0A9Fh:002Fh` takes the record out of a chain and `0A9Fh:015Ah`
    /// puts it in ([`Link`]); a snap always relinks (`0x0B7E2`), a leg
    /// only when the new edge names another layer
    /// (`0x0AFA2`–`0x0AFBD`). The drawer walks the chains, so a walker
    /// that hangs in none is not drawn at all.
    pub layer: Option<u8>,
    /// Node id under the walker (`+28h`; `FFFFh` while moving).
    pub dest_id: i16,
    /// The node index the walk is currently going to (`header+16h`). A
    /// second walk naming the same one is a no-op; naming another one
    /// mid-walk is the retarget at `0x0B13D`.
    pub target: Option<usize>,
    /// Facing to take on arrival (`+2Eh`; negative keeps the walk cel's
    /// direction).
    pub final_facing: i16,
    /// Flood result: per node `(via edge, distance)` (`node+0Ch`/`+0Eh`).
    pub dists: Vec<(i16, u16)>,
    /// The active leg, when mode 2.
    pub leg: Option<Leg>,
    /// The two animation bytes, from the walker record.
    pub gait: Gait,
    /// `+12h` as the animator writes it: `0Ah` while an animation of
    /// the figure's own runs (`0x14F9A`), 0 once `14DDh:01D4h` stops it.
    /// The walk's own states are kept in [`Self::walking`].
    pub state: u8,
    /// `+9` while the animation runs — a cel of the figure's file counted
    /// from its first, which is what `14DDh:0098h` makes of the record's
    /// cel base (`0x14F66`).
    pub anim_frame: u8,
    /// `+18h`: the `AN` entry the figure is bound to, as an index into
    /// the runtime's runs.
    pub ani: Option<usize>,
    /// `+0`, the record's kind: [`KIND_PLAYER`], or 6, 7, 10h, 11h for a
    /// person (see [`Walker::turn`]).
    pub kind: u8,
    /// `+0Ch`: the cel the walk counts from (`+0Ah = +0Ch + +9`,
    /// `0x089C4`, `0x0AF87`, `0x0B819`). A kind-3 record moves it with
    /// every turn ([`Self::turn`]) and the figure's animation rebases it
    /// (`0x14F66`, `0x1501B`); a wrapping word, as the original's.
    pub base: u16,
    /// Bit 40h of `+2`: the record drawn mirrored. A kind-3 record sets it
    /// when a turn passes the second block and clears it when it comes
    /// back (`0x0AC0C`, `0x0AC1E`); an animation's `18` opcode sets it as
    /// the stream says.
    pub mirror: bool,
    /// `+0Ah` as a record's bytes last gave it, until the walk next writes
    /// it — the snap, a leg, a phase step, the arrival, an animation's end.
    /// Every one of those writes `+0Ch` plus `+9` (or the standing cel), so
    /// while this is `None` the cel is that sum ([`Self::cel`]); a turn
    /// moves a held one by the same difference as `+0Ch` (`0x0AC28`).
    pub cel_held: Option<u16>,
    /// The graph header's `+28h`/`+2Ah`/`+2Ch`: where the mover put
    /// the walk last (`0x072B3`–`0x072CD`). The record follows it only at
    /// the animator's phase steps ([`Walker::animate_walk`]).
    pub head: (i32, i32, i32),
    /// The header's mode 1 (`0x0B4E9`): the last leg is over, and the next
    /// pass is the arrival.
    pub arriving: bool,
    /// The arrival stood the record on `+0Ch` rather than on the block's
    /// last cel (kinds 6 and 10h, `0x0B3AC`–`0x0B3C8`).
    pub stand_at_base: bool,
    /// The header's mode is other than the file's `FFFFh`: a snap has set
    /// it to 0 (`0x0B7D0`). Until then `0A9Fh:0617h` refuses to walk
    /// (`0x0B0A6`–`0x0B0BB`).
    pub ready: bool,
    /// `+2Ah`, the layer the record names — also while it hangs in no
    /// chain, which [`Self::layer`] does not say.
    pub rec_layer: u8,
    /// The chain operation the last call made, for the caller to apply.
    pub link: Option<Link>,
}

impl Walker {
    /// The image init: feet (100, 100), scale `100h`, facing 2 (front),
    /// standing — the record's own bytes, with the gait it carries.
    pub(crate) fn new(gait: Gait) -> Self {
        let front = 2 * (i16::from(gait.phases) + 1);
        Walker {
            gait,
            x: 100,
            y: 100,
            scale: 0x100,
            block: front,
            phase: 0,
            walking: false,
            anim_acc: 0,
            node: None,
            layer: None,
            dest_id: -1,
            target: None,
            final_facing: -1,
            dists: Vec::new(),
            leg: None,
            state: 0,
            anim_frame: 0,
            ani: None,
            kind: KIND_PLAYER,
            base: num::unsigned(front),
            mirror: false,
            cel_held: None,
            head: (100, 100, 0x100),
            arriving: false,
            stand_at_base: false,
            ready: false,
            rec_layer: 0,
            link: None,
        }
    }

    /// A room's 30h-byte record as the loader leaves it: its own position,
    /// cel, kind, gait, and the `+28h`/`+2Ah` the file carries, in no
    /// chain and on no graph (the chain heads are 0 in every file).
    pub(crate) fn from_record(rec: &MoverRecord, reach: i32) -> Self {
        let gait = Gait {
            speed: i32::from(rec.interval),
            phases: rec.phases.max(1),
            reach,
        };
        let mut w = Walker::new(gait);
        w.kind = rec.kind;
        w.base = rec.base;
        w.mirror = rec.flags & 0x40 != 0;
        w.x = i32::from(num::signed(rec.x));
        w.y = i32::from(num::signed(rec.y));
        w.scale = i32::from(rec.scale.0);
        w.head = (w.x, w.y, w.scale);
        w.phase = rec.phase;
        w.dest_id = num::signed(rec.node);
        w.rec_layer = num::low8(rec.layer);
        // The file's `+0Ah` is the cel until a snap or a walk sets one;
        // a standing record of these kinds shows `+0Ch` plus an offset.
        w.stand_at_base = rec.cel == rec.base;
        w
    }

    /// The cel to draw and whether it is mirrored: `+0Ah`, which is `+0Ch`
    /// plus the phase while walking and plus `+15h` standing, and bit 40h.
    /// For a kind-3 record `+0Ch` is where the turns have put it — the
    /// facing's block, with facing 3 drawn as block 1 mirrored (`0x0AC0C`) —
    /// or wherever a restored record left it.
    pub(crate) fn cel(&self) -> (usize, bool) {
        if let Some(cel) = self.cel_held {
            return (usize::from(cel), self.mirror);
        }
        if self.kind != KIND_PLAYER {
            // `+0Ah` of a person: `+0Ch` plus the phase while it walks
            // (`0x0AF83`, `0x089C4`), plus the phase count where a snap
            // or a kind-7 arrival stands it (`0x0B819`, `0x0B3DE`), and
            // `+0Ch` itself after a kind-6 arrival (`0x0B3C4`).
            let off = if self.walking {
                self.phase
            } else if self.stand_at_base {
                0
            } else {
                self.gait.phases
            };
            return (usize::from(self.base) + usize::from(off), self.mirror);
        }
        let phase = if self.walking {
            self.phase
        } else {
            self.gait.phases
        };
        (
            usize::from(self.base.wrapping_add(u16::from(phase))),
            self.mirror,
        )
    }

    /// The facing a kind-3 record's block says: `+2Ch / (+15h + 1)`,
    /// signed (`0x0ABC9`, `0x0BAD0`).
    pub(crate) fn facing(&self) -> i16 {
        self.block.wrapping_div(i16::from(self.gait.phases) + 1)
    }

    /// Leaves the room's chains behind.
    ///
    /// The graph header and the layer chains belong to the room, so
    /// leaving it leaves them behind; the entry function's tail
    /// (`0x0EF4B`) puts the walker down again by snapping it.
    pub(crate) fn unlink(&mut self) {
        self.link = Some(Link::Unlink);
        self.arriving = false;
        self.node = None;
        self.layer = None;
        self.leg = None;
        self.target = None;
        self.walking = false;
        self.dists.clear();
    }

    /// Teleports onto the node with `id`: the node half of `0A9Fh:0C59h`
    /// (`0x0B738`–`0x0B819`). Returns `false` when the graph has no such
    /// node.
    pub(crate) fn snap(&mut self, graph: &WalkGraph, id: i16, facing: i16) -> bool {
        let Some(idx) = lookup(graph, id) else {
            return false;
        };
        let n = &graph.nodes[idx];
        self.x = i32::from(n.x);
        self.y = i32::from(n.y);
        self.scale = i32::from(n.scale);
        self.node = Some(idx);
        self.layer = Some(n.layer);
        self.rec_layer = n.layer;
        self.dest_id = id;
        self.leg = None;
        self.target = None;
        self.walking = false;
        // The snap writes the node into the header's position too
        // (`0x0B74F`, `0x0B76D`, `0x0B78F`), sets the mode to 0
        // (`0x0B7D0`), and always links the record at its chain's head
        // (`0x0B7E2`).
        self.head = (self.x, self.y, self.scale);
        self.arriving = false;
        self.ready = true;
        self.link = Some(Link::Relink(n.layer));
        if facing >= 0 {
            self.turn(facing + 3);
        }
        // `+9 := +15h` and the cel `+0Ch + +15h` (`0x0B809`–`0x0B819`).
        self.phase = self.gait.phases;
        self.cel_held = None;
        self.stand_at_base = false;
        true
    }

    /// `0A9Fh:0C59h` whole: [`Self::snap`], but the
    /// record leaves its chain first (`0x0B722`), so a node the graph does
    /// not have leaves it in none (`0x0B733`).
    pub(crate) fn snap_or_unlink(&mut self, graph: &WalkGraph, id: i16, facing: i16) -> bool {
        if self.snap(graph, id, facing) {
            return true;
        }
        self.layer = None;
        self.link = Some(Link::Unlink);
        false
    }

    /// `0A9Fh:02CFh(rec, code)`: what a direction code does to the record,
    /// by its kind (the table at `cs:0315h`, `0x0ACD9`).
    ///
    /// | kind | routine | what |
    /// |---|---|---|
    /// | 3 | `01B9h` | the block of the facing `code − 3`; below 3, turned round |
    /// | 6, 7 | `0254h` | no block: codes 3 and 4 mirror off, 5 and 6 on (`cs:02B4h`), any other toggles bit 40h |
    /// | 10h, 11h | `02BCh` | the tiles marked, nothing else |
    /// | others | — | nothing |
    ///
    /// For kind 3 the new block is the facing times `+15h + 1`. When it is
    /// the block `+2Ch` already holds nothing happens (`0x0ABF1`);
    /// otherwise `+0Ah` and `+0Ch` move by the difference, less two blocks
    /// with the mirror set when the new block is past the second, plus two
    /// blocks with the mirror cleared when the old one was
    /// (`0x0AC02`–`0x0AC30`). So the cel follows `+0Ch`, not the block: a
    /// record whose two disagree keeps disagreeing.
    pub(crate) fn turn(&mut self, code: i16) {
        match self.kind {
            KIND_PLAYER => {
                let stride = i16::from(self.gait.phases) + 1;
                let facing = if code < 3 {
                    self.facing() ^ 2
                } else {
                    (code - 3) & 3
                };
                let block = facing.wrapping_mul(stride);
                if block == self.block {
                    return;
                }
                let two = 2 * stride;
                let mut d = block.wrapping_sub(self.block);
                if block > two {
                    self.mirror = true;
                    d = d.wrapping_sub(two);
                } else if self.block > two {
                    self.mirror = false;
                    d = d.wrapping_add(two);
                }
                self.base = self.base.wrapping_add(num::unsigned(d));
                if let Some(cel) = &mut self.cel_held {
                    *cel = cel.wrapping_add(num::unsigned(d));
                }
                self.block = block;
            }
            6 | 7 => {
                self.mirror = match code {
                    3 | 4 => false,
                    5 | 6 => true,
                    _ => !self.mirror,
                };
            }
            _ => {}
        }
    }

    /// `0A9Fh:02CFh(dir + 3)` on the figure's record — `0A9Fh:01B9h` for
    /// the player — the standing cel of a direction. A `dir` below zero is the
    /// code 2 the callers pass as `−1 + 3`, and that turns the figure
    /// round: `0x0ABCB` reads the facing back out of `+2Ch` and xors 2.
    pub(crate) fn face(&mut self, dir: i16) {
        self.turn(dir.saturating_add(3));
    }

    /// Removes only the draw-chain link, preserving the walker's node
    /// and position. `XP01.EXE`'s `0x0AA1F` bypasses the record through
    /// the layer head (`0x0AA78`) or predecessor (`0x0AB1A`); unlike
    /// leaving a room, it does not discard the graph or movement state.
    pub(crate) fn detach_draw_chain(&mut self) {
        self.link = Some(Link::Unlink);
        self.layer = None;
    }

    /// Walk from the current node to the node with `id`. `facing` is
    /// taken on arrival (negative: keep). Returns `false` when there is no
    /// such node or no first leg.
    pub(crate) fn walk_to(&mut self, graph: &WalkGraph, id: i16, facing: i16) -> bool {
        if self.dest_id == id {
            // Already standing on it: face and be done.
            if facing >= 0 {
                self.turn(facing + 3);
            }
            return true;
        }
        let Some(dest) = lookup(graph, id) else {
            return false;
        };
        self.final_facing = facing;
        // Header `+10h`: mode 2 is a walk already under way, and a second
        // walk then retargets it rather than starting over (`0x0B0A6`).
        if self.leg.is_some() {
            return self.retarget(graph, dest);
        }
        self.dists = flood(graph, dest);
        self.target = Some(dest);
        self.next_leg(graph)
    }

    /// `0A9Fh:0617h(rec, id, facing)` with its own answer, the way the
    /// location logic sees it.
    ///
    /// Standing on the node already, it turns to facing + 3 and answers 2
    /// (`0x0B03B`, `0x0B04D`); a node the id does not name answers 0
    /// (`0x0B08E`). Otherwise the facing is kept at `+2Eh` (`0x0B09E`), and
    /// the mode at `+10h` decides (`0x0B0A6`): 0 or 1 is a fresh walk, whose
    /// first leg answers 0, −1, or the turn `02CFh(code)` and 2 (`0x0B10D`,
    /// `0x0B139`); 2 is the retarget (`0x0B0B8`), which answers 1 when it
    /// keeps the leg (`0x0B25C`) and 2 after `02CFh(2)` when it turns round
    /// (`0x0B26F`); any other mode answers 0.
    pub(crate) fn walk_to_code(&mut self, graph: &WalkGraph, id: i16, facing: i16) -> i16 {
        if self.dest_id == id {
            self.turn(facing.saturating_add(3));
            return 2;
        }
        let Some(dest) = lookup(graph, id) else {
            return 0;
        };
        self.final_facing = facing;
        if !self.ready {
            return 0;
        }
        if self.leg.is_some() {
            return self.retarget_code(graph, dest);
        }
        self.dists = flood(graph, dest);
        self.target = Some(dest);
        match self.next_leg_code(graph) {
            3..=6 => 2,
            code => code,
        }
    }

    /// `0x0B13D`: `0617h` on a walker that is already mode 2.
    ///
    /// The leg it is on is not abandoned — it is either kept or walked
    /// backwards. When the new destination is one of the current edge's
    /// own endpoints, both their via-edges are cleared so that arriving
    /// there ends the walk; otherwise the flood runs again from the new
    /// destination and the endpoint it now favours decides the direction.
    /// Turning round negates the t-step and calls `02CFh(2)` (`0x0B26F`);
    /// a code below 3 reads the facing back out of `+2Ch` and XORs 2
    /// (`0x0ABCB`) — 0 becomes 2, 1 becomes 3.
    fn retarget(&mut self, graph: &WalkGraph, dest: usize) -> bool {
        self.retarget_code(graph, dest) > 0
    }

    /// The same with `0617h`'s answer: 1 when the leg is kept (`0x0B25C`),
    /// 2 when the walker turns round on it (`0x0B26F`).
    fn retarget_code(&mut self, graph: &WalkGraph, dest: usize) -> i16 {
        if self.target == Some(dest) {
            return 1;
        }
        self.target = Some(dest);
        let Some(leg) = self.leg.as_ref() else {
            return 0;
        };
        let Some(e) = graph.edges.get(leg.via) else {
            return 0;
        };
        let (a, b) = (usize::from(e.a), usize::from(e.b));
        let tstep = leg.tstep;
        let turn = if dest == a || dest == b {
            for end in [a, b] {
                if let Some(d) = self.dists.get_mut(end) {
                    d.0 = -1;
                }
            }
            if dest == a { tstep >= 0 } else { tstep <= 0 }
        } else {
            self.dists = flood(graph, dest);
            let (da, db) = (self.dists[a].1, self.dists[b].1);
            if da < db { tstep >= 0 } else { tstep <= 0 }
        };
        if turn {
            if let Some(leg) = self.leg.as_mut() {
                leg.tstep = -leg.tstep;
            }
            self.turn(2);
            2
        } else {
            1
        }
    }

    /// `0A9Fh:0404h` with a yes/no answer: pick the via edge out of the
    /// current node, load the interpolation, face the direction, start
    /// the walk animation. `false` when there is no leg (arrived or the
    /// edge is blocked).
    fn next_leg(&mut self, graph: &WalkGraph) -> bool {
        self.next_leg_code(graph) > 0
    }

    /// The same with `0A9Fh:0404h`'s answer: 0 when the node has no way on
    /// (`0x0AE1D`), −1 when its edge is blocked (`0x0AE34`), otherwise the
    /// direction code 3..6 the walker was turned by (`0x0AFE3`–`0x0B000`).
    fn next_leg_code(&mut self, graph: &WalkGraph) -> i16 {
        let Some(cur) = self.node else { return 0 };
        let via = match self.dists.get(cur) {
            Some(&(via, _)) if via >= 0 => usize::from(num::unsigned(via)),
            _ => return 0,
        };
        let Some(e) = graph.edges.get(via) else {
            return 0;
        };
        if e.blocked != 0 {
            return -1;
        }
        let a = &graph.nodes[usize::from(e.a)];
        let b = &graph.nodes[usize::from(e.b)];
        let from = (i32::from(a.x), i32::from(a.y), i32::from(a.scale));
        let delta2 = (
            2 * (i32::from(b.x) - i32::from(a.x)),
            2 * (i32::from(b.y) - i32::from(a.y)),
            2 * (i32::from(b.scale) - i32::from(a.scale)),
        );
        // `0x0AF10`: `imul dx` makes a 32-bit product, and the `cwd` right
        // after it (`0x0AF12`) throws the high word away — so the multiply
        // is 16-bit. What follows is `push dx / push ax / push 2 /
        // push 0FFFAh` and a call to the runtime's long divide at
        // `0000h:01E2h` (`0x0AF19`), and the order matters: that helper
        // reads its **first** long from the last two words pushed and
        // divides it by the second. So the product is the *divisor* and
        // `2FFFAh` the dividend, not the other way round — a long edge
        // costs more ticks than a short one, and the walk is evenly
        // paced.
        let product = i32::from(num::signed(e.len).wrapping_mul(num::short(self.gait.speed)));
        // The runtime's divide is signed (`0x0021B`–`0x0023A`), and
        // `0x0AF22` keeps the low word of its quotient as the step;
        // `0x0AF26`–`0x0AF2D` make a step of zero 1. A zero product is the
        // one divisor the original does not survive: with both its words
        // zero the divide takes its short road (`0x00219`) to `div bx` at
        // `0x00280`, which traps. Here it reaches the guard as a zero
        // quotient. In play neither the low word nor the guard matters:
        // the shortest of the 559 edges in Dunkle Schatten's rooms is 4,
        // the fastest walk speed the options offer the player is 2
        // (`DS:14E6h`), the second record walks at 10 (`DS:16D7h`) and the
        // rooms' people at 12, 15 or 17 — so the product is at least 8,
        // and the quotient fits a word and is never zero.
        let quotient = if product == 0 {
            0
        } else {
            self.gait.reach / product
        };
        let mut tstep = i32::from(num::short(quotient));
        if tstep == 0 {
            tstep = 1;
        }
        let t;
        if usize::from(e.b) == cur {
            // We sit at B: negate the step, start at the far end.
            tstep = -tstep;
            t = 0x7FFF;
        } else {
            t = 0;
        }
        self.leg = Some(Leg {
            via,
            from,
            delta2,
            tstep,
            t,
        });
        // The edge's layer (`+0Eh`); a different one means `002Fh` out of
        // the old chain and `015Ah` into the new one (`0x0AFA2`–`0x0AFBD`).
        // The same layer leaves the record where it hangs — in no chain,
        // if it was taken out of one.
        let layer = num::low8(e.layer);
        if layer != self.rec_layer {
            self.rec_layer = layer;
            self.layer = Some(layer);
            self.link = Some(Link::Relink(layer));
        }
        // Mode 2 (`0x0AF5C`).
        self.arriving = false;
        self.dest_id = -1;
        if !self.walking {
            self.walking = true;
            self.phase = 0;
            self.cel_held = None;
        }
        // Direction code 3..6 by axis and sign: `cmp word es:[bx+0Ch], 0`
        // at `0x0AFCE` asks the edge's axis word, and a nonzero one turns
        // the walker by the sign of the y delta (`+22h`, `0x0AED3`) against
        // the t-step's (`0x0AFD9`–`0x0AFE8`), a zero one by the x delta's
        // (`+1Eh`, `0x0AEB5`; `0x0AFF1`–`0x0B000`). The word is the room's
        // choice, not the edge's slope: of the 559 edges in Dunkle
        // Schatten's rooms, 57 of the 193 with a nonzero axis are at least
        // as wide as they are tall, and 5 of the 366 with a zero one are
        // taller than they are wide.
        let dir = if e.axis != 0 {
            if (delta2.1 ^ tstep) < 0 { 3 } else { 5 }
        } else if (delta2.0 ^ tstep) < 0 {
            6
        } else {
            4
        };
        // The callers hand the code to `02CFh` (`0x0B139`, `0x0B4F8`).
        self.turn(dir);
        dir
    }

    /// The mover's arithmetic: t advanced, and the point it names — or
    /// `Some(None)` when t ran out, `None` when there is no leg.
    /// `0724h:0035h` multiplies in sixteen bits and ends the leg on an overflow or a sum below
    /// zero (`0x07297`–`0x0729F`), which is the same test for the steps a
    /// leg can have.
    fn interpolate(&mut self, frames: i32) -> Option<Option<(i32, i32, i32)>> {
        let leg = self.leg.as_mut()?;
        let t = leg.t + leg.tstep * frames;
        if !(0..=0x7FFF).contains(&t) {
            return Some(None);
        }
        leg.t = t;
        let lerp = |from: i32, d2: i32| -> i32 {
            let p = t * d2;
            from + (p >> 16) + ((p >> 15) & 1)
        };
        Some(Some((
            lerp(leg.from.0, leg.delta2.0),
            lerp(leg.from.1, leg.delta2.1),
            lerp(leg.from.2, leg.delta2.2),
        )))
    }

    /// `0A9Fh:0892h(delta)` for this record: the mover pass.
    ///
    /// Measured, not only read: the original pass and animator, run under
    /// an 8086 emulator on Dunkle Schatten's own records and graphs, and
    /// this and [`Self::animate_walk`] give the same record, tick by tick
    /// (see the tests).
    ///
    /// - **Mode 2** (`0x0B416`): `0724h:0035h` moves t and writes the point
    ///   into the header ([`Self::head`]), not into the record. At the end
    ///   of a leg the record stands on the node reached — the sign of the
    ///   t-step says which end (`0x0B444`–`0x0B472`, `0x0B4A0`–`0x0B4BF`) —
    ///   and `0404h` starts the next leg. When there is none the mode
    ///   becomes 1 (`0x0B4E9`); a blocked edge also makes the arrival face
    ///   front (`+2Eh := 2`, `0x0B4DF`). `+28h` stays `FFFFh`.
    /// - **Mode 1** (`0x0B32F`), the next pass: `+28h` the node's id
    ///   (`0x0B36E`), the chain of the node's layer when it differs, mode
    ///   0, the record standing — on `+0Ch` for kinds 6 and 10h, on the
    ///   block's last cel for the others (`0x0B3AC`–`0x0B3E6`) — the
    ///   countdown 0, and the kept facing taken (`0x0B3F3`).
    ///
    /// The pass skips a record whose `+12h` is `0Ah` (`0x0B2EF`); the
    /// caller asks that, and the other two skips (`0x0B2D9`, `0x0B2E3`).
    pub(crate) fn pass(&mut self, graph: &WalkGraph, delta: i32) {
        if self.arriving {
            self.arriving = false;
            let Some(n) = self.node.and_then(|i| graph.nodes.get(i)) else {
                return;
            };
            self.dest_id = num::signed(n.id);
            if n.layer != self.rec_layer {
                self.rec_layer = n.layer;
                self.layer = Some(n.layer);
                self.link = Some(Link::Relink(n.layer));
            }
            self.walking = false;
            self.cel_held = None;
            self.stand_at_base = matches!(self.kind, 6 | 0x10);
            self.phase = if self.stand_at_base {
                0
            } else {
                self.gait.phases
            };
            self.anim_acc = 0;
            if self.final_facing >= 0 {
                self.turn(self.final_facing + 3);
            }
            return;
        }
        match self.interpolate(delta) {
            None => {}
            Some(Some(p)) => self.head = p,
            Some(None) => {
                let Some(leg) = self.leg.take() else { return };
                let Some(e) = graph.edges.get(leg.via) else {
                    return;
                };
                let reached = usize::from(if leg.tstep < 0 { e.a } else { e.b });
                let Some(n) = graph.nodes.get(reached) else {
                    return;
                };
                self.x = i32::from(n.x);
                self.y = i32::from(n.y);
                self.scale = i32::from(n.scale);
                self.node = Some(reached);
                match self.next_leg_code(graph) {
                    -1 => {
                        self.final_facing = 2;
                        self.arriving = true;
                    }
                    0 => self.arriving = true,
                    _ => {}
                }
            }
        }
    }

    /// `0883h:0305h(delta)` on a walking record (`+12h == 6`): the
    /// countdown `+16h` taken down, and for every interval it covers the
    /// step `0883h:015Ch` — `0A9Fh:0B4Ch` copies the header's point into
    /// the record (`0x0B593`–`0x0B5C3`), then the phase goes round
    /// (`0x0899A`–`0x089C4`) and the interval goes back on (`0x08AF7`).
    ///
    /// So the drawn record moves once a phase, every `+14h` ticks, while
    /// t moves every tick — measured on the original routines under an
    /// 8086 emulator, see the tests.
    ///
    /// Answers the footstep samples the copy plays: `0A9Fh:0B4Ch` calls
    /// `0EB9h:1949h(0)` at phase 1 and `(1)` at phase 4, read before the
    /// phase moves on (`0x0B5DC`, then `0x0899D`), for every kind but 8
    /// (`0x0B5D6`).
    pub(crate) fn animate_walk(&mut self, delta: i32) -> Vec<u8> {
        let mut steps = Vec::new();
        if !self.walking {
            return steps;
        }
        self.anim_acc -= delta;
        // Bounded like the animator's own tail: a zero interval would
        // otherwise never put the countdown above zero.
        for _ in 0..64 {
            if self.anim_acc > 0 {
                break;
            }
            (self.x, self.y, self.scale) = self.head;
            if self.kind != 8 {
                match self.phase {
                    1 => steps.push(0),
                    4 => steps.push(1),
                    _ => {}
                }
            }
            self.phase = (self.phase + 1) % self.gait.phases.max(1);
            self.cel_held = None;
            self.anim_acc += self.gait.speed;
        }
        steps
    }
}

/// The node whose `+8` is `id`, which `0617h` asks for at `0x0B08E`.
/// This scans from the last node down.
fn lookup(graph: &WalkGraph, id: i16) -> Option<usize> {
    (0..graph.nodes.len())
        .rev()
        .find(|&i| num::signed(graph.nodes[i].id) == id)
}

/// The flood (`0x0AD23`), as a walk starts it: reset every node to
/// FFFFh/FFFFh, zero the destination, flood. Cost of an edge from
/// distance `d` is `(blocked | len) + d + 10`, saturating
/// (`0x0AD45`–`0x0AD58`); a node improves when its distance is strictly
/// larger, remembers the via edge, and floods on. The recursion order is
/// the original's chain order, so equal-cost ties resolve the same way.
pub(crate) fn flood(graph: &WalkGraph, dest: usize) -> Vec<(i16, u16)> {
    let mut dists = vec![(-1i16, u16::MAX); graph.nodes.len()];
    if dest >= graph.nodes.len() {
        return dists;
    }
    dists[dest].1 = 0;
    relax(graph, &mut dists, 0, dest);
    dists
}

fn relax(graph: &WalkGraph, dists: &mut Vec<(i16, u16)>, dist: u16, node: usize) {
    let head = graph.nodes[node].first_edge;
    let mut si = head;
    loop {
        let Some(e) = graph.edges.get(usize::from(si)) else {
            return;
        };
        let cost = u32::from(e.blocked | e.len) + u32::from(dist) + 10;
        let cost = u16::try_from(cost).unwrap_or(u16::MAX);
        let (other, next) = if usize::from(e.a) == node {
            (usize::from(e.b), e.next_a)
        } else {
            (usize::from(e.a), e.next_b)
        };
        if dists[other].1 > cost {
            dists[other] = (num::signed(si), cost);
            relax(graph, dists, cost, other);
        }
        si = next;
        // Chain is circular; also stop if our own distance improved
        // underneath us during the recursion.
        if si == head || dists[node].1 != dist {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    /// The player's gait out of Dunkle Schatten, so these stay tests of
    /// the walk and not of the reading: interval 13 and six phases from the
    /// record at `DS:1693h` (`mover`'s tests read them), and the leg
    /// dividend `2FFFAh` the leg setup pushes (`0x0AF15`).
    const GAIT: Gait = Gait {
        speed: 13,
        phases: 6,
        reach: 0x2FFFA,
    };

    /// One tick of the frame pass over one record:
    /// [`Walker::pass`], then [`Walker::animate_walk`].
    fn tick(w: &mut Walker, g: &WalkGraph) {
        w.pass(g, 1);
        w.animate_walk(1);
    }

    use super::*;
    use motionvm_parsec_formats::rdv::{MoverRecord, WalkEdge, WalkGraph, WalkNode};

    fn node(x: u16, y: u16, id: i16, first_edge: u16) -> WalkNode {
        layered(x, y, id, first_edge, 0)
    }

    fn layered(x: u16, y: u16, id: i16, first_edge: u16, layer: u8) -> WalkNode {
        WalkNode {
            x,
            y,
            scale: 0x100,
            layer,
            id: num::unsigned(id),
            first_edge,
        }
    }

    /// A --0-- B --1-- C in a line; both chains are circular.
    fn line() -> WalkGraph {
        WalkGraph {
            nodes: vec![node(10, 50, 1, 0), node(50, 50, -2, 0), node(90, 50, 2, 1)],
            edges: vec![
                WalkEdge {
                    a: 0,
                    b: 1,
                    next_a: 0,
                    next_b: 1,
                    blocked: 0,
                    len: 40,
                    axis: 0,
                    layer: 0,
                },
                WalkEdge {
                    a: 1,
                    b: 2,
                    next_a: 0,
                    next_b: 1,
                    blocked: 0,
                    len: 40,
                    axis: 0,
                    layer: 0,
                },
            ],
        }
    }

    /// The step is the low word of the divide's quotient, as `0x0AF22`
    /// stores it, and a zero one is 1 (`0x0AF2D`): `2FFFAh / 3` is
    /// `0FFFEh`, a step of −2. A zero product, on which the original's
    /// divide traps, is the engine's own choice: it reaches the guard and
    /// steps by 1.
    #[test]
    fn the_step_is_the_quotients_low_word_and_never_zero() {
        let mut g = line();
        g.edges[0].len = 1;
        let step = |g: &WalkGraph, speed: i32| {
            let mut w = Walker::new(Gait { speed, ..GAIT });
            assert!(w.snap(g, 1, -1), "on node A");
            w.walk_to(g, 2, -1);
            w.leg.as_ref().expect("a leg").tstep
        };
        assert_eq!(step(&g, 3), -2, "0FFFEh read signed");
        assert_eq!(step(&g, 2), 0x7FFD, "17FFDh's low word");
        assert_eq!(step(&g, 0), 1, "the guard");
        g.edges[0].len = 40;
        assert_eq!(step(&g, 13), 0x2FFFA / 520, "a shipped edge and gait");
    }

    /// The snap links the walker into the chain of the node's own layer
    /// (`+6`); the leg moves it to the chain the edge names (`+0Eh`). A
    /// walker that has snapped nowhere hangs in no chain and is not drawn.
    #[test]
    fn the_chain_comes_from_the_node_then_from_the_edge() {
        let mut g = line();
        g.nodes[0] = layered(10, 50, 1, 0, 2);
        g.edges[0].layer = 1;
        let mut w = Walker::new(GAIT);
        assert_eq!(w.layer, None, "the image record is in no chain");
        assert!(w.snap(&g, 1, -1));
        assert_eq!(w.layer, Some(2), "node +6");
        assert!(w.walk_to(&g, 2, -1));
        assert_eq!(w.layer, Some(1), "edge +0Eh, the 002Fh/015Ah swap");
        w.unlink();
        assert_eq!(w.layer, None);
        assert_eq!(w.node, None);
    }

    #[test]
    fn the_flood_prefers_the_short_way_and_names_the_via_edge() {
        let g = line();
        let d = flood(&g, 2);
        assert_eq!(d[2], (-1, 0));
        assert_eq!(d[1], (1, 50), "len 40 + 10");
        assert_eq!(d[0], (0, 100));
    }

    #[test]
    fn a_walk_crosses_both_legs_and_arrives_facing_the_request() {
        let g = line();
        let mut w = Walker::new(GAIT);
        assert!(w.snap(&g, 1, -1));
        assert_eq!((w.x, w.y), (10, 50));
        assert!(w.walk_to(&g, 2, 0));
        assert!(w.walking);
        assert_eq!(w.facing(), 1, "rightward is code 4, facing 1");
        let mut frames = 0;
        while w.walking {
            tick(&mut w, &g);
            frames += 1;
            assert!(frames < 1000, "walk must terminate");
        }
        assert_eq!((w.x, w.y), (90, 50));
        assert_eq!(w.dest_id, 2);
        assert_eq!(w.facing(), 0, "arrival facing from the walk request");
        assert_eq!(w.anim_acc, 0, "the arrival zeroes the countdown");
    }

    /// `0x0B13D`: a second `0617h` in mode 2 does not restart the walk.
    /// Naming the end the walker is walking away from turns it round on
    /// the same leg — t-step negated, facing XOR 2 — and clears both
    /// endpoints' via-edges so arriving there ends the walk.
    #[test]
    fn a_retarget_turns_the_walker_round_on_its_own_leg() {
        let g = line();
        let mut w = Walker::new(GAIT);
        assert!(w.snap(&g, 1, -1));
        assert!(w.walk_to(&g, 2, -1), "start walking to the far end");
        assert_eq!(w.facing(), 1, "rightward");
        for _ in 0..40 {
            tick(&mut w, &g);
        }
        let (mid_x, step) = (w.x, w.leg.as_ref().expect("mid-leg").tstep);
        assert!(step > 0 && mid_x > 10 && mid_x < 50, "on the first leg");
        // Same target again: a no-op (`header+16h` already names it).
        assert!(w.walk_to(&g, 2, -1));
        assert_eq!(w.leg.as_ref().unwrap().tstep, step, "unchanged");
        // Back to where it came from.
        assert!(w.walk_to(&g, 1, -1));
        assert_eq!(w.leg.as_ref().unwrap().tstep, -step, "turned round");
        assert_eq!(w.facing(), 3, "1 XOR 2 — leftward, mirrored");
        let mut frames = 0;
        while w.walking {
            tick(&mut w, &g);
            frames += 1;
            assert!(frames < 1000, "the walk back must terminate");
        }
        assert_eq!((w.x, w.y), (10, 50), "arrived back at the near end");
        assert_eq!(w.dest_id, 1);
    }

    /// Retargeting to a node that is on neither end of the current leg
    /// floods again and keeps walking when the leg already points the
    /// right way.
    #[test]
    fn a_retarget_past_the_leg_keeps_the_direction_it_already_has() {
        let g = line();
        let mut w = Walker::new(GAIT);
        assert!(w.snap(&g, 1, -1));
        // Node index 1 is the middle node, id −2; walking there from the
        // near end is the same first leg as walking to the far end.
        assert!(w.walk_to(&g, -2, -1));
        let step = w.leg.as_ref().unwrap().tstep;
        assert!(w.walk_to(&g, 2, -1), "retarget to the far end");
        assert_eq!(w.leg.as_ref().unwrap().tstep, step, "no turn needed");
        let mut frames = 0;
        while w.walking {
            tick(&mut w, &g);
            frames += 1;
            assert!(frames < 1000);
        }
        assert_eq!((w.x, w.y), (90, 50), "walked through to the far end");
    }

    #[test]
    fn a_backward_leg_runs_the_same_interpolation_down() {
        let g = line();
        let mut w = Walker::new(GAIT);
        assert!(w.snap(&g, 2, -1));
        assert!(w.walk_to(&g, 1, -1));
        assert_eq!(w.facing(), 3, "leftward is code 6, facing 3 (mirrored)");
        assert!(w.cel().1, "facing 3 draws mirrored");
        while w.walking {
            tick(&mut w, &g);
        }
        assert_eq!((w.x, w.y), (10, 50));
        assert_eq!(w.facing(), 3, "no arrival facing requested");
    }

    /// A state the original's record and header were in after one half
    /// of a tick: `(tick, half, x, y, scale, phase, +28h)`, half 0 after
    /// the mover pass and 1 after the animator.
    type Seen = (u32, u8, i32, i32, i32, u8, u16);

    /// Runs `w` the way the frame pass does — `0A9Fh:0892h(1)`, then
    /// `0883h:0305h(1)` — and checks it against the states the original
    /// went through, each held until the next one.
    fn follow(w: &mut Walker, g: &WalkGraph, seen: &[Seen], ticks: u32) {
        let at = |t: u32, h: u8| {
            seen.iter()
                .rev()
                .find(|s| (s.0, s.1) <= (t, h))
                .copied()
                .expect("a state")
        };
        for t in 1..=ticks {
            for h in 0..2u8 {
                if h == 0 {
                    w.pass(g, 1);
                } else {
                    w.animate_walk(1);
                }
                let want = at(t, h);
                assert_eq!(
                    (w.x, w.y, w.scale, w.phase, num::unsigned(w.dest_id)),
                    (want.2, want.3, want.4, want.5, want.6),
                    "tick {t} half {h}"
                );
            }
        }
    }

    /// The Laden's person (Dunkle Schatten slot 9, id 83h) walks from node
    /// 12Ch to 12Dh, as the Laden's logic has it walk (`0x2558`,
    /// `0x256D`): kind 6, `+0Ch` 1, interval 0Fh, six phases; the two nodes
    /// at (166, 91) and (121, 88) and the edge between them, length 2Dh,
    /// out of graph 1 of the slot's walk-graph resource.
    ///
    /// **The sequence is measured**: the original `0A9Fh:0EA6h`, `0617h`,
    /// and then per tick `0A9Fh:0892h(1)` and `0883h:0305h(1)`, run under
    /// an 8086 emulator on the slot's own relocated bytes. The record's
    /// `+4`/`+6` follow the header only at the phase steps — ticks 1, 15,
    /// 30 and every fifteen after — and `+28h` stays `FFFFh` for the pass
    /// after the leg ends (113) and names 12Dh at 114.
    #[test]
    fn a_person_walks_as_the_original_pass_and_animator_measured() {
        let g = WalkGraph {
            nodes: vec![
                WalkNode {
                    x: 166,
                    y: 91,
                    scale: 0x100,
                    layer: 0,
                    id: 0x12C,
                    first_edge: 0,
                },
                WalkNode {
                    x: 121,
                    y: 88,
                    scale: 0x100,
                    layer: 0,
                    id: 0x12D,
                    first_edge: 0,
                },
            ],
            edges: vec![WalkEdge {
                a: 1,
                b: 0,
                next_a: 0,
                next_b: 0,
                blocked: 0,
                len: 0x2D,
                axis: 0,
                layer: 0,
            }],
        };
        let rec = MoverRecord {
            kind: 6,
            flags: 1,
            align: 2,
            x: 0x6C,
            y: 0x2C,
            bundle: 0,
            phase: 0,
            cel: 1,
            base: 1,
            scale: (0, 0),
            state: 3,
            id: 0x83,
            interval: 0x0F,
            phases: 6,
            node: 0,
            layer: 0,
        };
        let mut w = Walker::from_record(&rec, 0x2FFFA);
        assert_eq!(w.dest_id, 0, "+28h as the file has it");
        assert!(!w.ready, "header mode FFFFh until a snap");
        assert_eq!(w.walk_to_code(&g, 0x12D, 3), 0, "no walk before the snap");
        assert!(w.snap_or_unlink(&g, 0x12C, 3));
        assert_eq!(w.cel(), (7, true), "+0Ch + +15h, code 6 mirrors");
        assert_eq!(w.walk_to_code(&g, 0x12D, 3), 2, "0617h answers 2");
        assert_eq!(w.cel(), (1, true), "+0Ah := +0Ch at the leg");
        let seen: &[Seen] = &[
            (0, 1, 166, 91, 0x100, 0, 0xFFFF),
            (1, 1, 166, 91, 0x100, 1, 0xFFFF),
            (15, 1, 160, 91, 0x100, 2, 0xFFFF),
            (30, 1, 154, 90, 0x100, 3, 0xFFFF),
            (45, 1, 148, 90, 0x100, 4, 0xFFFF),
            (60, 1, 142, 89, 0x100, 5, 0xFFFF),
            (75, 1, 136, 89, 0x100, 0, 0xFFFF),
            (90, 1, 130, 89, 0x100, 1, 0xFFFF),
            (105, 1, 124, 88, 0x100, 2, 0xFFFF),
            (113, 0, 121, 88, 0x100, 2, 0xFFFF),
            (114, 0, 121, 88, 0x100, 0, 0x12D),
        ];
        follow(&mut w, &g, seen, 114);
        assert!(!w.walking);
        assert_eq!(w.cel(), (1, true), "kind 6 stands on +0Ch (0x0B3C4)");
    }

    /// The player on two legs: Dunkle Schatten slot 9, graph 0, from node
    /// 6Dh at (66, 90) through the waypoint `FFFDh` at (84, 90) to 41h at
    /// (79, 109) with scale 18Dh; edges of length 12h (horizontal) and 29h
    /// (vertical); the player's record, kind 3, interval 0Dh, six phases.
    ///
    /// Measured the same way, with `0A9Fh:0C59h(1693h, 6Dh, 2)` and
    /// `0617h(1693h, 41h, 2)`: the leg join at tick 40 puts the record on
    /// the waypoint at once, the next leg's points reach it only at the
    /// phase steps, and the arrival comes a pass after the end (130, 131).
    #[test]
    fn the_player_walks_two_legs_as_the_original_measured() {
        let node = |x, y, scale, id: u16, first_edge| WalkNode {
            x,
            y,
            scale,
            layer: 1,
            id,
            first_edge,
        };
        let g = WalkGraph {
            nodes: vec![
                node(66, 90, 0x100, 0x6D, 0),
                node(84, 90, 0x100, 0xFFFD, 0),
                node(79, 109, 0x18D, 0x41, 1),
            ],
            edges: vec![
                WalkEdge {
                    a: 0,
                    b: 1,
                    next_a: 0,
                    next_b: 1,
                    blocked: 0,
                    len: 0x12,
                    axis: 0,
                    layer: 1,
                },
                WalkEdge {
                    a: 2,
                    b: 1,
                    next_a: 1,
                    next_b: 0,
                    blocked: 0,
                    len: 0x29,
                    axis: 1,
                    layer: 1,
                },
            ],
        };
        let mut w = Walker::new(Gait {
            speed: 0x0D,
            phases: 6,
            reach: 0x2FFFA,
        });
        assert!(w.snap_or_unlink(&g, 0x6D, 2));
        assert_eq!(w.cel(), (20, false), "block 2, standing");
        assert_eq!(w.walk_to_code(&g, 0x41, 2), 2);
        assert_eq!(w.cel(), (7, false), "block 1, phase 0");
        let seen: &[Seen] = &[
            (0, 1, 66, 90, 0x100, 0, 0xFFFF),
            (1, 1, 66, 90, 0x100, 1, 0xFFFF),
            (13, 1, 72, 90, 0x100, 2, 0xFFFF),
            (26, 1, 78, 90, 0x100, 3, 0xFFFF),
            (39, 1, 84, 90, 0x100, 4, 0xFFFF),
            (52, 1, 83, 93, 0x113, 5, 0xFFFF),
            (65, 1, 83, 95, 0x128, 0, 0xFFFF),
            (78, 1, 82, 98, 0x13C, 1, 0xFFFF),
            (91, 1, 81, 101, 0x151, 2, 0xFFFF),
            (104, 1, 80, 104, 0x165, 3, 0xFFFF),
            (117, 1, 80, 106, 0x17A, 4, 0xFFFF),
            (130, 0, 79, 109, 0x18D, 4, 0xFFFF),
            (130, 1, 79, 109, 0x18D, 5, 0xFFFF),
            (131, 0, 79, 109, 0x18D, 6, 0x41),
        ];
        follow(&mut w, &g, seen, 131);
        assert_eq!(w.cel(), (20, false), "front, standing");
    }
}
