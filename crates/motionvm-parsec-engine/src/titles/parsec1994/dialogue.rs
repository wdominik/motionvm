//! Playing a scene: the tick, the player, the page walker, the page.
//!
//! Everything here is segment `0CFDh` of Dunkle Schatten's `XP01`, read
//! instruction by instruction; the module doc of [`super::tasks`] has the
//! record. The routines and their entry points:
//!
//! | routine | entry | here |
//! |---|---|---|
//! | load a scene | `0CFDh:172Fh` | [`Adventure::scene_load`] |
//! | resolve a scene number to a bank place | `0CFDh:059Eh` | [`Adventure::scene_bind`] |
//! | play a hotspot's own text | `0CFDh:18C2h` | [`Adventure::hotspot_text`] |
//! | arm, start, stop | `0CFDh:1A2Eh`, `1A3Fh`, `1A0Eh` | [`Adventure::task_arm`] … |
//! | the tick | `0CFDh:11ECh` | [`Adventure::tick_tasks`] |
//! | the page walker | `0CFDh:0754h` | [`Adventure::pages`] |
//! | the control scanner | `0CFDh:01BEh` | [`Adventure::scan_controls`] |
//! | the player | `0CFDh:09E9h` | [`Adventure::play`] |
//! | a page's preparation | `0CFDh:0932h` | [`Adventure::prepare_page`] |
//! | the substitution pass | `13A7h:0023h` | [`Adventure::substitute`] |
//! | the frame pass | `0CFDh:16D5h`, `1565h` | [`Adventure::frame_tasks`], [`Adventure::paint_tasks`] |

use crate::num;
use motionvm_parsec_formats::Dialogue;
use motionvm_render::Framebuffer;

use super::tasks::{ARMED, BankId, DialogueRef, PARKED, PROTECTED, REPEAT, RUNNING};
use super::{Adventure, Bank, Caption};

/// What happened inside the scene machine, for a test to read back and for
/// a title's hook to act on. The original raises most of these through the
/// far hook at `[1E32h]`; the rest are this engine's own notes of a call it
/// made or could not make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    /// The far hook was called with `(code, arg, task)`.
    Hook { code: u16, arg: i16, task: usize },
    /// A `03 n` control past the five the scanner keeps for itself.
    Script { n: u8, task: usize },
    /// `0CFDh:06AFh` asked for mode cue `n`.
    Cue(u16),
    /// A page went up: which task, the page's text.
    PageUp { task: usize, text: String },
    /// A task was reset, taking its box down.
    Reset(usize),
    /// A scene number resolved to a place, or did not.
    SceneBound {
        scene: i16,
        place: Option<DialogueRef>,
    },
    /// A scene was refused because the other task speaks with the same
    /// speaker (`0x0D0EA`).
    SceneRefused(i16),
    /// A record set a game flag (`0x0D915`).
    FlagSet(u8),
    /// A record's choice was hidden because its flag holds (`0x0DB43`).
    ChoiceSpent { record: usize },
    /// More than one choice survived and the menu went up: its lines,
    /// each with the choice it belongs to.
    MenuUp { lines: Vec<(u16, String)> },
    /// `0CFDh:0EFEh`: the record chosen out of the menu.
    ChoiceTaken(usize),
    /// The hook handed the call on to the location's own (`0x1085E`).
    LocationHook { code: u16, arg: i16, task: usize },
    /// `0883h:20D9h(1693h, k)`: animation `k` asked for on the figure.
    FigureAnim(i16),
    /// `0883h:1F82h(record, k)`: animation `k` asked for on an object.
    ObjectAnim { object: u8, anim: u8 },
    /// `14DDh:0A06h(id, 1)`: an object shown, its live bit set.
    ObjectShown(u8),
    /// `14DDh:096Bh(id)`: an object hidden, its live bit cleared.
    ObjectHidden(u8),
    /// `1586h:05E1h`: a script leaves through exit `100 + k`.
    ScriptExit(u8),
    /// `10B8h:0210h(k)`: transition effect `k` of six, rolled on the way
    /// out of a location and played.
    Transition(u8),
    /// `10B8h:03BAh` in *Dunkle Schatten*: the reveal over the playfield a
    /// handler calls, which plays.
    Reveal,
    /// What the program shows after its main loop could not be read, and
    /// it exits at once instead.
    EndingUnread(String),
    /// The rule search reached a handler this engine has not read.
    UnreadRule { far: (u16, u16), flags: u16 },
    /// The location logic wrote a word through a far pointer outside
    /// `DGROUP` that the engine does not carry out.
    FarWrite { seg: u16, off: u16, value: u16 },
    /// The location logic called an engine routine whose engine side is
    /// not built, with what it asked for packed into a word.
    Unbuilt(&'static str, u32),
    /// `0C3Fh:095Ch(id)`: an item given.
    ItemGiven(u8),
    /// `0EB9h:1949h(k)`: the effect in slot `k` asked for; with the switch
    /// on, a digital driver and the slot filled, it plays
    /// ([`Adventure::effect_1949`](super::Adventure::effect_1949)).
    Sample(u8),
    /// An animation's `17` opcode reported `code` for the object, through
    /// `0EB9h:1D97h` — and on to the location's own hook at `[1E60h]`.
    AnimCode {
        /// The object's id, `FFh` for the figure.
        object: u8,
        /// The code.
        code: u16,
    },
}

impl Adventure {
    /// `0CFDh:172Fh`: load scene `n` into task 0 and ask for kind 4.
    ///
    /// It binds the scene with `0CFDh:059Eh` (`0x0E703`), which says
    /// `NULLDLG` for none; empties the queue `[1CF4h]` (`0x0E711`); takes the
    /// records from past the bank's four-byte header (`0x0E71E`); asks for kind 4
    /// in `[1DA8h]` (`0x0E728`) with the flags `[1DB8h]`, the countdown
    /// `[1D9Eh]` and the fixed time `[1DAAh]` cleared (`0x0E72E`–`0x0E73A`);
    /// and runs the tick `0CFDh:11ECh` with zero (`0x0E740`).
    ///
    /// Answers with the dialogue's record count, the word at the blob's
    /// `+2` (`0x0E74C`), or 0 when nothing was bound.
    pub(crate) fn scene_load(&mut self, n: i16) -> u16 {
        if !self.scene_bind(n, 0) {
            return 0;
        }
        self.tasks.queue_len = 0;
        let t = &mut self.tasks.t[0];
        t.next = Some(0);
        t.request = 4;
        t.flags = 0;
        t.countdown = 0;
        t.fixed = 0;
        self.tick_tasks(0);
        self.tasks.t[0]
            .dialogue
            .and_then(|r| self.dialogue_of(r))
            .map_or(0, |d| num::word(d.len()))
    }

    /// `0CFDh:19A4h(lo, hi, flags)`: one of scenes `lo..=hi` rolled and
    /// bound to task 1, the background task, with `flags` (bit 0: roll the
    /// next when this one is over, `0x0E381`).
    ///
    /// Task 1's range goes into `+1Ch`/`+1Eh` (`[1E04h]`, `[1E06h]`,
    /// `0x0E97E`), the scene is rolled by `1EB5h:0028h(lo, hi)` (`0x0E988`) and
    /// bound to task 1 by `0CFDh:059Eh` (`0x0E996`); `+46h` takes its first
    /// record (`0x0E9AD`), `+1Ah` the flags (`0x0E9B4`), `+0Ah` asks for kind 4,
    /// play (`0x0E9B7`), with no wait and no fixed time (`0x0E9BD`, `0x0E9C3`),
    /// and one tick with nothing follows (`0x0E9CC`).
    ///
    /// Answers the dialogue's record count, or 0 when nothing was bound.
    pub(crate) fn ambient_load(&mut self, lo: i16, hi: i16, flags: u16) -> u16 {
        self.tasks.t[1].range = (lo, hi);
        let n = num::signed(self.roll(num::unsigned(lo), num::unsigned(hi)));
        if !self.scene_bind(n, 1) {
            return 0;
        }
        let t = &mut self.tasks.t[1];
        t.next = Some(0);
        t.flags = flags;
        t.request = 4;
        t.countdown = 0;
        t.fixed = 0;
        self.tick_tasks(0);
        self.tasks.t[1]
            .dialogue
            .and_then(|r| self.dialogue_of(r))
            .map_or(0, |d| num::word(d.len()))
    }

    /// `0CFDh:059Eh`: which place scene `n` names, bound to task `i`.
    ///
    /// A negative number is a place (`0x0D5D6`, its complement): below 4000h in
    /// bank 999 (`0x0D5DC`), from 4000h up in bank 2, less 4001h (`0x0D5F0`). A
    /// number that is not negative indexes the location's own bank while it is
    /// below the count `[1EEAh]` (`0x0D5EA`). When nothing is found, place 13h of
    /// bank 999 stands in (`0x0D5FD`).
    ///
    /// The place-13h fallback is skipped when `[1EDCh]` bit 8 is set — the
    /// `/TEST` switch — which a plain start never sets. Then `0CFDh:0094h`
    /// takes the dialogue on: it collects the speakers' bits, clears the
    /// spent bit of any record whose flags read `9FF0h` under that mask,
    /// and refuses the whole scene when the *other* task is speaking with
    /// an overlapping speaker and is not interruptible (`0x0D0EA`).
    pub(super) fn scene_bind(&mut self, n: i16, i: usize) -> bool {
        // `0x0D57B`: the task's last dialogue over first.
        self.dialogue_over_pointer(i);
        let place = if n < 0 {
            let p = num::unsigned(!n);
            if p < 0x4000 {
                (usize::from(p) < self.dialogues.0.len()).then_some(DialogueRef {
                    bank: BankId::G999,
                    place: usize::from(p),
                })
            } else {
                let p = usize::from(p - 0x4001);
                (p < self.dialogues.1.len()).then_some(DialogueRef {
                    bank: BankId::G2,
                    place: p,
                })
            }
        } else {
            let count = self.own_bank().map_or(0, Bank::len);
            (usize::from(num::unsigned(n)) < count).then_some(DialogueRef {
                bank: BankId::Own,
                place: usize::from(num::unsigned(n)),
            })
        };
        // A place the directory keeps as `NULLPTR` binds nothing: the far
        // pointer the resolver reads is null there (`0x0D615`).
        let mut place = place.filter(|r| self.dialogue_of(*r).is_some());
        if place.is_none() && self.dialogues.0.len() > 0x13 {
            place = Some(DialogueRef {
                bank: BankId::G999,
                place: 0x13,
            })
            .filter(|r| self.dialogue_of(*r).is_some());
        }
        self.events.push(Event::SceneBound { scene: n, place });
        let Some(r) = place else {
            self.tasks.t[i].dialogue = None;
            return false;
        };
        // `0CFDh:0094h`.
        let Some(d) = self.dialogue_of(r) else {
            return false;
        };
        let count = d.len();
        let mut speakers = 0u16;
        for k in 0..count {
            let flags = self.record_flags(r, k);
            if flags & 0xF != 0 {
                speakers |= 1 << ((flags & 0xF) - 1);
            } else if flags & 0x9FF0 == 0x9FF0 {
                self.set_record_flags(r, k, flags & 0x7FFF);
            }
        }
        let other = 1 - i;
        if self.tasks.t[other].kind > 0 && self.tasks.t[other].speakers & speakers != 0 {
            // `0x0D120`: task 0 is never interrupted, and neither is a
            // task with bit 1 set (`0x0D129`, `test … 2 ; je` to the reset).
            if other == 0 || self.tasks.t[other].flags & PROTECTED != 0 {
                self.events.push(Event::SceneRefused(n));
                return false;
            }
            // `0x0D137`: the other task is reset for this one, and the
            // hook hears of it with code 6 (`0x0D150`).
            self.task_reset(other);
            self.hook_call(6, num::signed(self.tasks.t[other].speakers), other);
        }
        let t = &mut self.tasks.t[i];
        t.dialogue = Some(r);
        t.speakers = speakers;
        // `0x0D16C`: the hook hears of the claim with code 0, and task 0
        // holds the band (`0C3Fh:0581h(1)`, `0x0D181`).
        self.hook_call(0, num::signed(speakers), i);
        if i == 0 {
            self.panel_tint(1);
        }
        true
    }

    /// `0CFDh:18C2h`: a hotspot's own line, played by task 0 as kind 3.
    ///
    /// The text is substituted into the scratch by `13A7h:0023h` (`0x0E8AC`);
    /// the slot is 0, the player (`0x0E8B6`), with one speaker (`0x0E8BC`) and
    /// style 0 at `2139h` (`0x0E8CC`). The point is the player's x (`[1697h]`,
    /// `0x0E8D8`) and, above its feet `[1699h]`, forty rows of its scale
    /// `[16A3h]` (`0x0E8DE`–`0x0E8E7`); kind 3 is asked for (`0x0E8F1`) with
    /// style word 1 (`0x0E8F7`).
    pub(crate) fn hotspot_text(&mut self, text: &[u8]) {
        let (x, y, scale) = self.player_record();
        let scratch = self.substitute(text);
        let t = &mut self.tasks.t[0];
        t.scratch = scratch;
        t.slot = 0;
        t.speakers = 1;
        t.style = self.styles.first().copied();
        t.resolved = (x, y - ((scale * 0x28) >> 8));
        t.request = 3;
        t.style_word = 1;
        t.countdown = 0;
        t.fixed = 0;
        t.flags = 0;
        t.cursor = 0;
        self.tick_tasks(0);
    }

    /// `0CFDh:1810h(text, 0, …)`: a hotspot's `texts[0]` shown at the
    /// pointer while it is over the hotspot — task 0 as kind 2, on slot
    /// 11, which is put on the pointer's own point.
    ///
    /// With a text or dialogue up — kind above 2 — it does nothing and answers
    /// 0 (`0x0E7E3`). Otherwise slot 11 (`0x0E7EF`) takes the pointer's point
    /// (`[1854h]` into `[1D9Ah]`, `0x0E80E`; marked a point, `0x0E83A`), kind 2
    /// is asked for (`0x0E840`) for as long as it takes, a fixed time of `7FFFh`
    /// (`0x0E87C`), and the style before the row is `2139h` (`0x0E882`).
    pub(crate) fn hover_text(&mut self, text: &[u8]) -> bool {
        if self.tasks.t[0].kind > 2 {
            return false;
        }
        let (x, y) = self.mouse;
        self.subjects.set_point(11, 1, num::short(x), num::short(y));
        let scratch = self.substitute(text);
        let t = &mut self.tasks.t[0];
        t.scratch = scratch;
        t.slot = 11;
        t.speakers = 1;
        t.style = Some(self.hover_style);
        t.request = 2;
        t.countdown = 0;
        t.flags = 0;
        t.fixed = PARKED;
        t.style_word = 0;
        t.cursor = 0;
        self.tick_tasks(0);
        true
    }

    /// `0CFDh:1A2Eh`: arm task `i` — it waits for a start (`or word
    /// [bx+1DB8h], 8000h`).
    pub(crate) fn task_arm(&mut self, i: usize) {
        self.tasks.t[i].flags |= ARMED;
    }

    /// `0CFDh:1A3Fh`: start an armed task.
    ///
    /// Only an armed task starts (`[bx+1DB8h]` bit 15, `0x0EA0F`): the bit is
    /// cleared (`0x0EA28`), `+1Ch` bit 0 set, running (`0x0EA33`), the page
    /// composed again if the speaker moved (`0x0EA38`), and the hook `[1E32h]`
    /// told code 10 with the slot and the task (`0x0EA4A`).
    pub(crate) fn task_start(&mut self, i: usize) {
        if self.tasks.t[i].flags & ARMED == 0 {
            return;
        }
        self.tasks.t[i].flags &= !ARMED;
        self.tasks.t[i].run |= RUNNING;
        // `0CFDh:04B4h`: for a kind of 3 or more on an object or the
        // player, resolve the point, and recompose when it moved.
        let t = &self.tasks.t[i];
        let slot_kind = self.subjects.get(t.slot).map(|s| s.code & 0xF000);
        if t.kind > 2 && matches!(slot_kind, Some(0x1000 | 0x2000)) && self.resolve_point(i) {
            // `0x0D50A`: from the page the box carries, not from the
            // cursor, which `0x0D8C7` has already moved past it; what the
            // compositor answers is not looked at (`0x0D517`).
            let page = self.tasks.t[i].page;
            self.compose(i, page);
        }
        if self.profile.task_start_tells_hook {
            let slot = num::signed(num::word(self.tasks.t[i].slot));
            self.hook_call(10, slot, i);
        }
    }

    /// `0CFDh:1A0Eh`: stop task `i` — countdown 0, kind 1 asked for.
    pub(crate) fn task_stop(&mut self, i: usize) {
        self.tasks.t[i].countdown = 0;
        self.tasks.t[i].request = 1;
    }

    /// `0CFDh:06DFh`: reset task `i`.
    ///
    /// No record (`0x0D6BA`), kind 0 (`0x0D6CB`), the countdown cleared
    /// (`0x0D6D6`); for task 0 the band is shown again (`0C3Fh:0659h(1)`,
    /// `0x0D6E0`) and the cue reset (`0x0D6E8`).
    pub(super) fn task_reset(&mut self, i: usize) {
        let t = &mut self.tasks.t[i];
        t.record = None;
        t.kind = 0;
        t.countdown = 0;
        t.box_ = None;
        self.events.push(Event::Reset(i));
        if i == 0 {
            self.panel_release(1);
            self.cue_default();
        }
    }

    /// `0CFDh:06CDh` (`0x0D69D`): cue 1 when `[1E46h]` is set, else 2.
    pub(super) fn cue_default(&mut self) {
        let cue = if self.tasks.cue_figure { 1 } else { 2 };
        self.cue(cue);
    }

    /// `0CFDh:06AFh`: ask for mode cue `n`, three draw passes out.
    fn cue(&mut self, n: u16) {
        if n == self.tasks.cue_guard || n == self.tasks.cue_wanted {
            return;
        }
        self.tasks.cue_wanted = n;
        self.tasks.cue_in = 3;
        self.events.push(Event::Cue(n));
    }

    /// `0CFDh:11ECh`: the tick, for both tasks.
    ///
    /// The shape of it, per task from 1 down to 0:
    ///
    /// An armed or held-off task — `[bx+1DB8h]` bits 15 or 10 — with time left
    /// is skipped (`0x0E21E`, `0x0E22B`); otherwise the countdown `[bx+1D9Eh]`
    /// goes down (`0x0E23D`) and on zero the kind decides what happens, through
    /// a jump table (`0x0E259`). Then a request in `[bx+1DA8h]` is taken
    /// (`0x0E420`) and the new kind started through a second table (`0x0E4A9`).
    ///
    /// Answers as the original does for whoever asks about task 0
    /// (`0x0E4F8`): 0 idle, -1 for kind 2, 2 for kind 5, 1 otherwise.
    pub(crate) fn tick_tasks(&mut self, delta: i32) -> i16 {
        // `[1E3Eh]`: the cue countdown (`0x0E1C0`).
        if self.tasks.cue_in > 0 {
            self.tasks.cue_in -= 1;
            if self.tasks.cue_in == 0 {
                // `0x0E1CD`: `push word [1E40h] ; call far 0590h:064Dh`.
                self.tasks.cue_current = self.tasks.cue_wanted;
                self.pointer_mode(self.tasks.cue_wanted);
            }
        }
        if !self.tasks.enabled {
            // `0x0E1DE`: the larger of kind and request stands in.
            let k = self.tasks.t[0].kind.max(self.tasks.t[0].request);
            return Self::answer_for(k);
        }
        let waits = ARMED | self.profile.text_flags.quick.unwrap_or(0);
        for i in (0..2).rev() {
            let t = &self.tasks.t[i];
            if t.flags & waits != 0 && t.countdown > 0 {
                continue;
            }
            self.tasks.t[i].countdown -= delta;
            if self.tasks.t[i].countdown <= 0 {
                self.expire(i);
            }
            self.take_request(i);
        }
        Self::answer_for(self.tasks.t[0].kind)
    }

    fn answer_for(kind: u16) -> i16 {
        match kind {
            0 => 0,
            2 => -1,
            5 => 2,
            _ => 1,
        }
    }

    /// The arm the expired task's kind selects (`cs:[1559h]`).
    fn expire(&mut self, i: usize) {
        match self.tasks.t[i].kind {
            // `0x0E25E`.
            0 => self.tasks.t[i].countdown = 0,
            // `0x0E41B`: nothing but the request check.
            1 => {}
            // `0x0E26C`: a hotspot text's pages are done.
            2 => {
                if self.tasks.t[i].request != 2 {
                    self.tasks.t[i].fixed = 0;
                }
                self.tasks.t[i].kind = 0;
                self.tasks.t[i].countdown = 0;
                self.tasks.t[i].box_ = None;
            }
            // `0x0E295`: the next page, or the end.
            3 => {
                if self.tasks.t[i].request == 0 && self.pages(i) == 0 {
                    self.task_reset(i);
                }
            }
            // `0x0E2BA`: a dialogue's next page, next record, or end.
            4 => {
                let t = &self.tasks.t[i];
                if i == 0 && t.flags & self.profile.text_flags.hold != 0 && t.request == 0 {
                    self.tasks.t[0].countdown = PARKED;
                    return;
                }
                if self.tasks.t[i].request != 0 {
                    // `0x0E3F5`: a request came in over a dialogue.
                    self.task_reset(i);
                    self.hook_call(6, num::signed(num::word(i)), i);
                    return;
                }
                if self.pages(i) != 0 {
                    return;
                }
                if self.play(i, false) != 0 {
                    return;
                }
                self.task_reset(i);
                self.hook_call(5, num::signed(num::word(i)), i);
                self.after_dialogue(i);
            }
            // `0x0E410`: a menu waits for a click.
            5 => self.tasks.t[i].countdown = PARKED,
            _ => {}
        }
    }

    /// What follows a finished dialogue (`0x0E31F`): task 0 drains its
    /// queue, task 1 draws the next of its range.
    fn after_dialogue(&mut self, i: usize) {
        if i == 0 {
            if self.tasks.queue_len == 0 {
                return;
            }
            self.tasks.queue_len -= 1;
            let head = self.tasks.queue_head;
            let scene = self.tasks.queue[head];
            if self.scene_bind(scene, 0) {
                let t = &mut self.tasks.t[0];
                t.next = Some(0);
                t.request = 4;
            }
            self.tasks.queue_head = (head + 1) & 0xF;
            return;
        }
        let t = &self.tasks.t[1];
        if t.flags & REPEAT == 0 {
            return;
        }
        let (lo, hi) = t.range;
        let scene = if lo < hi {
            num::signed(self.rng.inclusive(num::unsigned(lo), num::unsigned(hi)))
        } else {
            lo
        };
        if self.scene_bind(scene, 1) {
            let t = &mut self.tasks.t[1];
            t.next = Some(0);
            t.request = 4;
        }
    }

    /// `0x0E41B`: a kind asked for becomes the kind, and starts.
    ///
    /// The request becomes the kind `[bx+1DA0h]` (`0x0E44B`) and is cleared
    /// (`0x0E454`); the point comes from the resolver (`0x0E468`) and the cursor
    /// starts at the scratch (`0x0E490`). The kind picks the start from a jump
    /// table (`0x0E4A9`): 1 resets, 2 shows pages, 3 cues and shows pages, 4
    /// plays.
    fn take_request(&mut self, i: usize) {
        let t = &mut self.tasks.t[i];
        if t.request == 0 || t.request == t.kind {
            return;
        }
        t.kind = t.request;
        t.request = 0;
        t.point = t.resolved;
        t.cursor = 0;
        match t.kind {
            1 => self.task_reset(i),
            2 => {
                if self.pages(i) == 0 {
                    self.task_reset(i);
                }
            }
            3 => {
                if i == 0 {
                    self.panel_tint(1);
                    self.cue(4);
                }
                if self.pages(i) == 0 {
                    self.task_reset(i);
                }
            }
            4 => {
                self.tasks.t[i].record = None;
                self.play(i, false);
            }
            _ => {}
        }
    }

    /// `0CFDh:0754h`: the next page of the task's text.
    ///
    /// A zero byte at the cursor is the end (`0x0D741`), `0Dh` a page break
    /// (`0x0D755`) that turns the page — `+12h` set to 1, `0x0D778` — and holds
    /// a base's worth `[167Dh]`, answering 1 (`0x0D783`). Otherwise the
    /// controls are scanned for the adjustment (`0CFDh:01BEh`, `0x0D79E`), the
    /// speaker's point resolved (`0CFDh:02C0h`, `0x0D7B1`), the page composed
    /// (`1269h:0082h`, `0x0D7F6`) and clamped to 320 × 128 (`1269h:0008h`,
    /// `0x0D81B`), and its characters counted (`0CFDh:071Dh`, `0x0D830`), at
    /// most a thousand (`0x0D865`). The time to hold it is `[167Fh]` per line
    /// plus `[1681h]` per character shifted right by 5 (`0x0D871`, `0x0D87C`),
    /// scaled by the adjustment plus 8 over 8 when the adjustment is above −8
    /// (`0x0D887`, `0x0D890`), and kept between the floor `[167Dh]` and the
    /// ceiling `[1683h]` (`0x0D8AC`, `0x0D8A2`). The cursor moves past the page
    /// (`0x0D8C7`) and past one break (`0x0D8D4`), and the task runs (`+1Ch`
    /// bit 0, `0x0D8E8`).
    ///
    /// Answers with the characters of the page shown, 1 for a page turned,
    /// 0 for the end of the text.
    pub(super) fn pages(&mut self, i: usize) -> i32 {
        self.tasks.t[i].page_turned = false;
        let t = &self.tasks.t[i];
        let at = t.cursor;
        match t.scratch.get(at) {
            None | Some(0) => return 0,
            Some(0x0D) => {
                let base = self.timing().base;
                let t = &mut self.tasks.t[i];
                t.cursor += 1;
                if t.scratch.get(t.cursor).is_none_or(|&b| b == 0) {
                    return 0;
                }
                t.page_turned = true;
                t.countdown = i32::from(base);
                return 1;
            }
            _ => {}
        }
        let adjust = self.scan_controls(i, at);
        self.resolve_point(i);
        let Some(lines) = self.compose(i, at) else {
            return 0;
        };
        let page_len = Self::page_len(&self.tasks.t[i].scratch, at);
        let t = &mut self.tasks.t[i];
        if t.fixed != 0 {
            t.countdown = t.fixed;
        } else {
            if i == 0 {
                self.panel_tint(1);
            }
            let chars = page_len.min(0x3E8);
            let timing = self.timing();
            let mut d = i32::from(timing.base)
                + i32::from(timing.per_line) * lines
                + ((i32::from(timing.per_char) * chars) >> 5);
            if adjust > -8 {
                d = (d * (adjust + 8)) >> 3;
            } else {
                d = i32::from(timing.base);
            }
            let d = d.min(i32::from(timing.max)).max(i32::from(timing.base));
            self.tasks.t[i].countdown = d;
        }
        let t = &mut self.tasks.t[i];
        t.cursor += num::index(page_len);
        if t.scratch.get(t.cursor) == Some(&0x0D) {
            t.cursor += 1;
        }
        t.run |= RUNNING;
        let text =
            motionvm_parsec_formats::cp437_to_string(&t.scratch[at..at + num::index(page_len)]);
        self.events.push(Event::PageUp { task: i, text });
        page_len
    }

    /// `0CFDh:071Dh`: the characters of a page — every line measured
    /// (`13A7h:0196h`, controls and their arguments counted) and the line
    /// breaks between them, while the byte that ends a line is `0Ah`
    /// (`0x0D719`). That is the distance from the cursor to the page's end.
    fn page_len(scratch: &[u8], from: usize) -> i32 {
        num::count(crate::titles::caption::page_end(scratch, from) - from)
    }

    /// `0CFDh:01BEh(task, text)`: the controls in the page, and what they
    /// do. Before the first, task 0's flags are cleared and the hook hears
    /// code 8 with the task's slot; the scan itself is `0CFDh:01EEh`.
    ///
    /// For task 0 the flags `[1DB8h]` are cleared first (`0x0D19C`), and with
    /// a hook installed at `[1E32h]` (`0x0D1A2`) it hears code 8 with the
    /// task's slot `+04h` (`0x0D1B1`–`0x0D1B7`). A byte above `0Dh` is a
    /// character (`0x0D1CB`); the others go through a jump table (`0x0D1D2`): 0
    /// and `0Dh` end the page, 1 and 2 skip their argument, and `03 n` with `n`
    /// from 1 to 5 is the scanner's own (`0x0D20C`) — 1 holds (task 0, flag
    /// `100h`, `0x0D222`, after `[1E48h]` cleared at `0x0D21C`), 2 sets flag
    /// `200h` (`0x0D22E`), 3 makes the page longer and 4 shorter (`0x0D236`,
    /// `0x0D239`), 5 sets flag `400h` (task 0, `0x0D243`). An `n` above 5
    /// goes into `[1687h]` as an event for the hook, code 7 (`0x0D24C`). The
    /// end raises code 9 (`0x0D1EA`). Which bits, and whether `03 05` is the
    /// scanner's own, is the build's: [`super::profile::TextFlags`].
    ///
    /// Answers with the adjustment: `03 03`s less `03 04`s.
    pub(super) fn scan_controls(&mut self, i: usize, from: usize) -> i32 {
        if i == 0 {
            self.tasks.t[0].flags = 0;
        }
        let slot = num::signed(num::word(self.tasks.t[i].slot));
        self.hook_call(8, slot, i);
        let mut adjust = 0;
        let mut at = from;
        loop {
            let b = self.tasks.t[i].scratch.get(at).copied().unwrap_or(0);
            at += 1;
            match b {
                0 | 0x0D => {
                    let slot = num::signed(num::word(self.tasks.t[i].slot));
                    self.hook_call(9, slot, i);
                    return adjust;
                }
                1 | 2 => at += 1,
                3 => {
                    let n = self.tasks.t[i].scratch.get(at).copied().unwrap_or(0);
                    at += 1;
                    let bits = self.profile.text_flags;
                    match (n, bits.quick) {
                        (1, _) if i == 0 => {
                            self.page_release_code = 0;
                            self.tasks.t[0].flags |= bits.hold;
                        }
                        (2, _) if i == 0 => self.tasks.t[0].flags |= bits.firm,
                        (3, _) => adjust += 1,
                        (4, _) => adjust -= 1,
                        (5, Some(quick)) if i == 0 => self.tasks.t[0].flags |= quick,
                        (1 | 2, _) | (5, Some(_)) => {}
                        (n, _) => {
                            self.events.push(Event::Script { n, task: i });
                            self.hook_call(7, i16::from(n), i);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// `0CFDh:09E9h`: play the task's next record.
    ///
    /// The record to play next `[bx+1DE6h]` becomes the one being played
    /// `[bx+1DE2h]` (`0x0D9CC`, `0x0D9D9`). Its flags (`0x0D9FE`) can be forced
    /// to speaker 0 (`0x0DA07`); a non-zero id is kept in `[1685h]` (`0x0DA30`)
    /// and told to the hook with code 4 (`0x0DA41`). The low nibble picks the
    /// arm (`0x0DAC4`): 1 prepares the page with speaker 0, shows it, cues 4
    /// and tells the hook 1 (`0x0DDE1`); `n` above 1 prepares it with speaker
    /// `n − 1`, shows it, cues 3 for task 0 and tells the hook 2 (`0x0DE1A`); 0
    /// is the choice list (`0x0DAD6`). The reply list is remembered in
    /// `[1E38h]` (`0x0DE60`) and its first entry is the next record
    /// (`0x0DE89`); with no reply the dialogue is over (`0CFDh:056Eh`,
    /// `0x0DEB2`).
    ///
    /// Answers with the task's kind afterwards, 0 when there was nothing to
    /// play.
    pub(super) fn play(&mut self, i: usize, force_mode1: bool) -> u16 {
        let mut shown = 0;
        let mut force_mode1 = force_mode1;
        loop {
            let Some(r) = self.tasks.t[i].dialogue else {
                return 0;
            };
            let Some(k) = self.tasks.t[i].next else {
                return 0;
            };
            self.tasks.t[i].record = Some(k);
            let Some(d) = self.dialogue_of(r) else {
                return 0;
            };
            let Some(rec) = d.record(k) else {
                return 0;
            };
            let text = rec.text.to_owned();
            let next: Vec<u16> = rec.next.to_vec();
            let has_reply = !rec.reply.is_empty() || !rec.next.is_empty();
            let mut flags = self.record_flags(r, k);
            if force_mode1 {
                flags = (flags & 0xFFF0) | 1;
            }
            let nibble = flags & 0xF;
            if nibble != 0 && rec.id != 0 {
                self.hook_call(4, num::signed(rec.id), i);
            }
            match nibble {
                0 => {
                    if self.choose(i, r, k) {
                        return self.tasks.t[i].kind;
                    }
                    // A single survivor: `0x0DCD9` makes it the record,
                    // tells the hook its id (code 4, `0x0DD47`) and falls
                    // into the arm at `0x0DDE1` — the one a nibble of 1
                    // takes — so it is spoken as the player's own line
                    // whatever its own nibble says. Without that, a
                    // choice record that is its own last survivor would
                    // be chosen again forever.
                    force_mode1 = true;
                    continue;
                }
                1 => {
                    self.prepare_page(i, 0, flags, &text);
                    if self.pages(i) != 0 {
                        self.cue(4);
                        self.hook_call(1, num::signed(num::word(i)), i);
                        shown += 1;
                    }
                }
                n => {
                    let speaker = usize::from(n) - 1;
                    self.prepare_page(i, speaker, flags, &text);
                    if self.pages(i) != 0 {
                        if i == 0 {
                            self.cue(3);
                        }
                        self.hook_call(2, num::signed(num::word(speaker)), i);
                        shown += 1;
                    }
                }
            }
            // The tail shared by the two text arms (`0x0DE60`).
            if has_reply && !next.is_empty() {
                self.tasks.choice_list = Some((r, k));
                self.tasks.t[i].next = Some(usize::from(next[0]));
            } else {
                // `0x0DEB2`: `call 056Eh`, the dialogue over.
                self.dialogue_over_pointer(i);
                self.tasks.choice_list = None;
                self.tasks.t[i].next = None;
                self.tasks.t[i].dialogue = None;
            }
            if shown == 0 {
                continue;
            }
            return self.tasks.t[i].kind;
        }
    }

    /// The choice arm (`0x0DAD6`): which of the next-list's records
    /// survive their flags, and what happens when one or none does.
    ///
    /// A spent record (bit 15) is skipped (`0x0DB24`); a conditional one (bit
    /// 12, `0x0DB2A`) whose flag `[273Fh + n]` holds (`0x0DB39`) is spent and
    /// skipped (`0x0DB43`); any other survives into `[1CE8h]` (`0x0DB53`). One
    /// survivor needs no menu (`[1E4Ch] == 1`, `0x0DB6A`); more tell the hook 8
    /// and put a menu up (`0x0DB80`).
    ///
    /// The list is `[1E36h]`, the next-list of the record that ended
    /// before this one (`0x0DB06`) — the choices are that record's
    /// answers, of which this record is the first. Without one the task is
    /// reset (`0x0DB10`).
    ///
    /// Answers `true` when the menu path was taken and the task now parks.
    fn choose(&mut self, i: usize, r: DialogueRef, k: usize) -> bool {
        let Some(list) = self.tasks.choice_list.and_then(|(lr, lk)| {
            self.dialogue_of(lr)
                .and_then(|d| d.record(lk))
                .map(|rec| rec.next.to_vec())
        }) else {
            // `0x0DAE0`: `call 056Eh`, the dialogue over.
            self.dialogue_over_pointer(i);
            self.task_reset(i);
            self.tasks.t[i].next = None;
            self.tasks.t[i].dialogue = None;
            return false;
        };
        let mut survivors = Vec::new();
        for &idx in &list {
            let idx = usize::from(idx);
            let flags = self.record_flags(r, idx);
            if flags & 0x8000 != 0 {
                continue;
            }
            if flags & 0x1000 != 0 {
                let flag = usize::from((flags & 0x0FF0) >> 4);
                if self.flag(flag) != 0 {
                    self.set_record_flags(r, idx, flags | 0x8000);
                    self.events.push(Event::ChoiceSpent { record: idx });
                    continue;
                }
            }
            survivors.push(idx);
        }
        self.tasks.choices = survivors.clone();
        let _ = k;
        if survivors.len() <= 1 {
            // `0x0DC9F`: none leaves nothing to play; one is played at once.
            self.tasks.t[i].next = survivors.first().copied();
            if survivors.is_empty() {
                self.tasks.t[i].dialogue = None;
            }
            return false;
        }
        self.hook_call(8, -1, i);
        self.menu_build(i, &survivors);
        true
    }

    /// `0CFDh:0932h`: a record's text becomes the task's page.
    ///
    /// A setter record (`flags & 3000h == 2000h`, `0x0D904`) sets its flag in
    /// `[273Fh + n]` (`0x0D915`). The text is substituted into the scratch by
    /// `13A7h:0023h` (`0x0D93D`), the speaker's style row is `3n + 9`
    /// (`0x0D95C`), the speaker becomes the slot (`0x0D990`), kind 4 is set
    /// (`0x0D999`) with style word 0 (`0x0D9A4`), and the point resolved
    /// (`0CFDh:02C0h`, `0x0D9AC`).
    fn prepare_page(&mut self, i: usize, speaker: usize, flags: u16, text: &str) {
        if flags & 0x3000 == 0x2000 {
            let flag = usize::from((flags & 0x0FF0) >> 4);
            self.set_flag(flag);
            self.events.push(Event::FlagSet(num::byte(flag)));
        }
        let bytes = motionvm_parsec_formats::to_cp437(text);
        let scratch = self.substitute(&bytes);
        let style = self.styles.get(speaker).copied();
        let t = &mut self.tasks.t[i];
        t.scratch = scratch;
        t.style = style;
        t.cursor = 0;
        t.slot = speaker;
        t.kind = 4;
        t.style_word = 0;
        self.resolve_point(i);
    }

    /// `13A7h:0023h`: copy a text, resolving `02 n` out of the substitution
    /// row and carrying `01 n` and `03 n` through.
    ///
    /// A byte above 1Fh is a character (`0x13AB3`), 0 the end (`0x13AB7`), and
    /// one above 3 a character too (`0x13ABB`); 2 takes entry `n` of the row
    /// (`0x13ABF`), while 1 and 3 copy both bytes (`0x13AC8`).
    pub(super) fn substitute(&self, text: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(text.len());
        let mut it = text.iter().copied();
        while let Some(b) = it.next() {
            match b {
                0 => break,
                2 => {
                    let n = it.next().unwrap_or(0);
                    if let Some(s) = self.substitutions.get(usize::from(n)) {
                        out.extend_from_slice(s);
                    }
                }
                1 | 3 => {
                    out.push(b);
                    if let Some(n) = it.next() {
                        out.push(n);
                    }
                }
                b => out.push(b),
            }
        }
        out
    }

    /// `0CFDh:02C0h` for a task: the slot's point into `+0Eh`/`+10h`, the
    /// slot's payload into the style word, and whether the point differs
    /// from the one the box hangs on (`0x0D446`).
    ///
    /// Every arm of the resolver ends at `0x0D3CC`, which copies the low
    /// twelve bits of the slot's code into the task's `+12h`. The slot's code `[bx+1D56h]` (`0x0D3D2`), masked to its low twelve bits
    /// (`0x0D3D6`), is the style word `[bx+1DB0h]` (`0x0D3DF`).
    ///
    /// The page routine resolves before it composes (`0x0D7B1`), so the
    /// style word a page is laid out by is its slot's payload, whatever
    /// the setup wrote before: slot 10's `53h` centres the box on the
    /// middle of the playfield with its lines flush left, and a raw point
    /// set with code 3 centres the box on the point. Bits 0–3 also move a
    /// record's point by its cel's extent (`0x0D3E3`–`0x0D42E`); no
    /// record slot of Dunkle Schatten carries a payload, so that move is
    /// not made here.
    fn resolve_point(&mut self, i: usize) -> bool {
        let slot = self.tasks.t[i].slot;
        if let Some(s) = self.subjects.get(slot) {
            self.tasks.t[i].style_word = s.payload();
        }
        let Some(p) = self.subjects.point(self, slot) else {
            return false;
        };
        let t = &mut self.tasks.t[i];
        let moved = p != t.point;
        t.resolved = p;
        t.point = p;
        moved
    }

    /// `1269h:0082h` and `1269h:0008h` for the page of the task's text
    /// that starts at `at`: compose the box against the point and clamp it
    /// to the playfield, and keep where the page starts in the box
    /// (`0x12743`). Answers with what the compositor returns, the
    /// characters plus the lines less one (`0x12861`) — `None` for 0, the
    /// empty page `0x0D800` refuses.
    fn compose(&mut self, i: usize, at: usize) -> Option<i32> {
        let t = &self.tasks.t[i];
        let at = at.min(t.scratch.len());
        let end = crate::titles::caption::page_end(&t.scratch, at);
        let page = &t.scratch[at..end];
        let (box_, count) = Caption::compose(&self.font, page, t.point, t.style_word);
        let t = &mut self.tasks.t[i];
        t.page = at;
        t.box_ = Some(box_);
        if count == 0 {
            return None;
        }
        Some(count)
    }

    /// `0CFDh:16D5h`: the frame pass over both tasks — which have a box up
    /// to paint. Answers as the original does: -1 for kind 2 on task 0, 2
    /// for a menu, 1 for anything else that is up, 0 for nothing.
    pub(crate) fn frame_tasks(&mut self) -> i16 {
        let mut r = 0;
        for i in (0..2).rev() {
            let t = &self.tasks.t[i];
            if t.request != 0 {
                continue;
            }
            r = match t.kind {
                0 => 0,
                2 => -1,
                5 => 2,
                _ => 1,
            };
            // `0CFDh:1565h`: a page that was turned is not drawn, and the
            // running bit comes down once the box has been painted.
            self.tasks.t[i].run &= !RUNNING;
        }
        r
    }

    /// The boxes that are up, painted (`1269h:01E0h` from `0x0D5B6`).
    /// `0CFDh:16D5h`: task 1, then task 0 — a kind of 5 paints the menu
    /// instead of a box (`0x0E6E4`), and that is [`Adventure::paint_menu`]'s.
    pub(crate) fn paint_tasks(&self, fb: &mut Framebuffer) {
        for t in self.tasks.t.iter().rev() {
            if t.request != 0 || t.flags & ARMED != 0 || t.page_turned || matches!(t.kind, 0 | 5) {
                continue;
            }
            let (Some(b), Some(style)) = (&t.box_, t.style) else {
                continue;
            };
            b.paint(fb, &self.font, t.style_word, style, self.flatten);
        }
    }

    /// Whether task 0 has a box up, for a test to look at.
    #[cfg(test)]
    pub(crate) fn caption_is_up(&self) -> bool {
        let t = &self.tasks.t[0];
        !matches!(t.kind, 0 | 5) && t.request == 0 && t.flags & ARMED == 0 && t.box_.is_some()
    }

    /// The text of the page task 0 shows, joined for a test to look at.
    #[cfg(test)]
    pub(crate) fn caption_text(&self) -> String {
        self.tasks.t[0]
            .box_
            .as_ref()
            .map(|b| {
                b.lines
                    .iter()
                    .map(|l| motionvm_parsec_formats::cp437_to_string(l))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    }

    /// The dialogue a reference names, out of the bank it lives in.
    pub(crate) fn dialogue_of(&self, r: DialogueRef) -> Option<&Dialogue> {
        match r.bank {
            BankId::G999 => self.dialogues.0.get(r.place),
            BankId::G2 => self.dialogues.1.get(r.place),
            BankId::Own => self.own_bank()?.get(r.place),
        }
    }

    /// Record `k`'s flags as they stand — the original writes into the
    /// loaded blob, and this keeps the writes beside the blob instead.
    pub(crate) fn record_flags(&self, r: DialogueRef, k: usize) -> u16 {
        if let Some(&f) = self.record_flag_writes.get(&(r, k)) {
            return f;
        }
        self.dialogue_of(r)
            .and_then(|d| d.record(k))
            .map_or(0, |rec| rec.flags)
    }

    pub(super) fn set_record_flags(&mut self, r: DialogueRef, k: usize, flags: u16) {
        self.record_flag_writes.insert((r, k), flags);
    }

    /// The far hook at `[1E32h]`: recorded, then the logic's.
    pub(crate) fn hook_call(&mut self, code: u16, arg: i16, task: usize) {
        self.events.push(Event::Hook { code, arg, task });
        (self.hook)(self, code, num::unsigned(arg), num::word(task));
    }

    /// The walker record's position and scale (`[1697h]`, `[1699h]`,
    /// `[16A3h]`), as the hotspot-text entry reads them: the figure where
    /// it stands, or the record's own initial values where none stands.
    fn player_record(&self) -> (i32, i32, i32) {
        match &self.walker {
            Some(w) => (w.x, w.y, w.scale),
            None => (100, 100, 0x100),
        }
    }
}

#[cfg(test)]
mod tests {

    /// `XP01.EXE`'s `0x0D685` compares the exclusion word, not the applied mode.
    /// Returning to mode 4 must replace a pending mode 1 even while mode 4
    /// is still displayed. `0x0E1C7` counts calls, not elapsed task ticks.
    #[test]
    fn returning_to_applied_cursor_replaces_a_pending_request() {
        let Some(mut g) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        g.tasks.enabled = false;
        g.tasks.cue_current = 4;
        g.tasks.cue_wanted = 4;
        g.tasks.cue_in = 0;
        g.cue(1);
        g.tick_tasks(120);
        assert_eq!(g.tasks.cue_in, 2);
        g.cue(4);
        assert_eq!((g.tasks.cue_wanted, g.tasks.cue_in), (4, 3));
        g.tick_tasks(120);
        g.cue(4);
        assert_eq!(
            g.tasks.cue_in, 2,
            "same pending request does not restart delay"
        );
        g.tick_tasks(120);
        assert_eq!(g.tasks.cue_in, 1);
        g.tick_tasks(120);
        assert_eq!((g.tasks.cue_current, g.tasks.cue_in), (4, 0));
        g.cue(g.tasks.cue_guard);
        assert_eq!((g.tasks.cue_wanted, g.tasks.cue_in), (4, 0));
    }

    /// The exclusion word, read out of Dunkle Schatten's `XP01.EXE`.
    #[test]
    fn the_shipped_cursor_exclusion_word_is_zero() {
        let Some(dir) = crate::titles::ds1::gamedata_or_skip() else {
            return;
        };
        let img = crate::exe::Image::open_any(&dir, "XP01.EXE").unwrap();
        assert_eq!(crate::titles::xp1994::cursor_cue_guard(&img).unwrap(), 0);
    }
}
