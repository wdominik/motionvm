//! The two tasks the game plays its scenes with.
//!
//! The record array at `DS:1D9Eh` has **two** 74-byte records — every
//! loop over it counts from 2 (`0x0E213`, `0x0E6DB`, `0x0DF6F`). Task 0 is
//! the location's scene: a dialogue, a hotspot's own line, a choice menu,
//! the queue of scenes behind it. Task 1 is the second voice, the one a
//! location hands a range of dialogues to draw from at random.
//!
//! A record's fields, by the offset each is read at:
//!
//! | offset | field | read at |
//! |---|---|---|
//! | `+00` | countdown, in ticks | `0x0E23D` |
//! | `+02` | kind: 0 idle, 1 finish, 2 pages of a hotspot text, 3 a text with its cue, 4 a dialogue, 5 a menu | `0x0E24B` |
//! | `+04` | the subject slot the speaker is | `0x0D990` |
//! | `+06`, `+08` | the point the box hangs on | `0x0E45F` |
//! | `+0Ah` | the kind asked for next | `0x0E420` |
//! | `+0Ch` | a fixed duration, when a script set one | `0x0D83E` |
//! | `+0Eh`, `+10h` | the point the resolver found | `0x0D45F` |
//! | `+12h` | the style word | `0x0D7C6` |
//! | `+14h` | a page was just turned | `0x0D778` |
//! | `+16h` | bit 0 running, bits 1 and 2 the menu's | `0x0D8E8` |
//! | `+18h` | the speakers the dialogue uses, one bit each | `0x0D0C7` |
//! | `+1Ah` | bit 15 armed, bit 8 hold for a click, bit 1 protected, bit 0 repeat | `0x0E21E` |
//! | `+1Ch`, `+1Eh` | the range task 1 draws from | `0x0E39A` |
//! | `+20h` | the scratch buffer the page is copied into | `0x0D92B` |
//! | `+24h` | the speaker's style | `0x0D96D` |
//! | `+28h` | the text cursor | `0x0D741` |
//! | `+2Ch` | the dialogue | `0x0D5EE` |
//! | `+30h` | the composed box | `0x0D512` |
//! | `+3Eh`, `+40h` | the box's `+0Eh`/`+10h`: where the page it was composed from starts | `0x0D50A` |
//! | `+42h` | the record being played | `0x0D9D9` |
//! | `+46h` | the record to play next | `0x0D9CC` |
//!
//! `+26h`, the segment half of the style's far pointer, is `2139h` at the
//! writes `0x0D967`, `0x0E882` and `0x0E8CC` — the segment the style rows
//! are in — so the style is kept as the row entry it points at.

use crate::titles::caption::Caption;
use crate::titles::xp1994::Style;

/// Which of the three banks a dialogue came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum BankId {
    /// Bank 999, `DIALOG.DAT` resource 0: what a verb that leads nowhere says.
    G999,
    /// Bank 2, resource 1: what the inventory says.
    G2,
    /// The location's own, resource 3 of its slot.
    Own,
}

/// A dialogue, by bank and place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DialogueRef {
    pub bank: BankId,
    pub place: usize,
}

/// One 74-byte record.
#[derive(Debug, Clone, Default)]
pub(crate) struct Task {
    /// `+00`.
    pub countdown: i32,
    /// `+02`.
    pub kind: u16,
    /// `+04`.
    pub slot: usize,
    /// `+06`, `+08`.
    pub point: (i32, i32),
    /// `+0Ah`.
    pub request: u16,
    /// `+0Ch`.
    pub fixed: i32,
    /// `+0Eh`, `+10h`.
    pub resolved: (i32, i32),
    /// `+12h`.
    pub style_word: u16,
    /// `+14h`.
    pub page_turned: bool,
    /// `+16h`.
    pub run: u16,
    /// `+18h`.
    pub speakers: u16,
    /// `+1Ah`.
    pub flags: u16,
    /// `+1Ch`, `+1Eh`.
    pub range: (i16, i16),
    /// `+20h`: the page after substitution.
    pub scratch: Vec<u8>,
    /// `+24h`.
    pub style: Option<Style>,
    /// `+28h`: where in the scratch the next page starts.
    pub cursor: usize,
    /// `+2Ch`.
    pub dialogue: Option<DialogueRef>,
    /// `+30h`.
    pub box_: Option<Caption>,
    /// `+3Eh`, `+40h` — the box's `+0Eh`/`+10h`: where in the scratch the
    /// page the box was last composed from starts. The compositor writes
    /// it as it begins (`0x12743`, `0x12747`), and the recompose when the
    /// speaker has moved reads it back (`0x0D50A`).
    pub page: usize,
    /// `+42h`.
    pub record: Option<usize>,
    /// `+46h`.
    pub next: Option<usize>,
}

/// Bit 15 of `+1Ah`: armed, waiting to be started (`0x0EA1B`).
pub(crate) const ARMED: u16 = 0x8000;
/// Bit 10 of `+1Ah`, from a `03 05` control in the text (`0x0D243`).
pub(crate) const FLAG_400: u16 = 0x400;
/// Bit 9 of `+1Ah`, from a `03 02` control (`0x0D22E`).
pub(crate) const FLAG_200: u16 = 0x200;
/// Bit 8 of `+1Ah`: hold the page until a click, from a `03 01` control
/// (`0x0D222`); the tick then parks the countdown at `7FFFh` (`0x0E2CD`).
pub(crate) const HOLD: u16 = 0x100;
/// Bit 1 of `+1Ah`: no dialogue may interrupt this one. `0x0D0F9` tests
/// it and jumps **to the reset** when it is clear, so a task with the bit
/// set refuses the scene instead.
pub(crate) const PROTECTED: u16 = 2;
/// Bit 0 of `+1Ah`: task 1 draws another dialogue when this one ends
/// (`0x0E381`).
pub(crate) const REPEAT: u16 = 1;

/// Bit 0 of `+16h`: the box is up and needs painting (`0x0EA33`).
pub(crate) const RUNNING: u16 = 1;

/// The value a parked countdown holds (`0x0E2CD`, `0x0E415`).
pub(crate) const PARKED: i32 = 0x7FFF;

/// The task array and the globals around it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tasks {
    /// The two records.
    pub t: [Task; 2],
    /// `[1E44h]`: whether the tick does anything at all. `0CFDh:1B89h`
    /// sets it at the end of entering a location.
    pub enabled: bool,
    /// The scene queue: sixteen numbers at `DS:1D16h`, a head at
    /// `[1CF2h]` and a count at `[1CF4h]`, filled by `0CFDh:1781h` while a
    /// dialogue is up and drained by task 0 when one ends (`0x0E323`).
    pub queue: [i16; 16],
    pub queue_head: usize,
    pub queue_len: usize,
    /// `[1E40h]`: the mode cue requested at `0x0D691`.
    pub cue_wanted: u16,
    /// Last mode passed to the cursor setter at `0x0E1D1`; this is not
    /// `[1E42h]`, which the setter does not write.
    pub cue_current: u16,
    /// `[1E42h]`: the independent exclusion word compared at `0x0D685`.
    pub cue_guard: u16,
    /// `[1E3Eh]`: draw passes until applying the request (`0x0E1C7`).
    pub cue_in: i32,
    /// `[1E46h]`: the default cue is 1 when set, else 2 (`0x0D69D`).
    pub cue_figure: bool,
    /// `[1CE8h]` and `[1E4Ch]`: the choices that survived their flags, in
    /// screen order.
    pub choices: Vec<usize>,
    /// `[1E36h]`: the next-list the last record left behind, as the
    /// record it belongs to.
    pub choice_list: Option<(DialogueRef, usize)>,
}

impl Tasks {
    /// Both records empty, nothing enabled, the queue clear.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// `0CFDh:1781h`: put a scene behind the one that is playing. Answers
    /// `false` when the queue is full.
    pub(crate) fn enqueue(&mut self, scene: i16) -> bool {
        if self.queue_len >= 16 {
            return false;
        }
        let at = (self.queue_len + self.queue_head) & 0xF;
        self.queue[at] = scene;
        self.queue_len += 1;
        true
    }

    /// Whether task 0 is busy enough that a scene has to queue
    /// (`0x0E757`: kind or request at 4 or above).
    pub(crate) fn busy(&self) -> bool {
        self.t[0].kind >= 4 || self.t[0].request >= 4
    }
}
