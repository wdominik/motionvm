//! The mode machine: the block at `DS:288Dh` that the entry function
//! clears (`0x0EDA8`, `38h` bytes), and the routines that drive it once a
//! frame.
//!
//! Every frame of a visit, the handler's loop calls the dispatcher
//! `0EB9h:0FCEh` (`0x9C8`), and the dispatcher's head is the machine's
//! step: it defers the start of event 111 while `[1ED0h]` is set
//! (`0x0FB69`), asks what task 0 is doing (`0CFDh:0FD9h`, `0x0FB85`), runs
//! the step on that answer (`0x0FB8D`, the near call to `0965h`), and ends
//! the frame when an exit is asked for, `[1F48h]` not zero (`0x0FB91`).
//!
//! The words of the block, by address:
//!
//! | word | what |
//! |---|---|
//! | `[288Dh]` | the verb, [`Adventure::verb`] |
//! | `[288Fh]` | the mode, [`Adventure::mode`] |
//! | `[2891h]`, `[2893h]` | the object chosen and its class |
//! | `[2895h]`, `[2897h]` | the second object and its class |
//! | `[2899h]` | the two-object flag |
//! | `[289Bh]` | the bit field: [`WALK`], [`LOCKED`], and what a rule asked to wait for |
//! | `[289Dh]` | the verb as chosen, before a rule changed it |
//! | `[289Fh]` | the node the figure is walking to |
//! | `[28A3h]`, `[28A5h]` | the hotspot under the pointer last frame |
//! | `[28BBh]`, `[28BDh]` | the gate: a far routine the machine asks before it goes on |
//! | `[28C3h]` | the status refresh age |
//!
//! Three gates are read. `1586h:059Fh` is installed by a script exit and
//! starts the walk to the door; `1586h:0000h` is installed by the walk
//! verb's rule and, once the figure stands, ends the visit; `0EB9h:0489h`
//! is installed by the sixth verb's rule and by Escape, and opens the game
//! options.
//!
//! The road out of a room, as the two gates and the rule between them make
//! it: a `03 78` control in a text (`1586h:05E1h`) chooses the door and
//! puts the machine into mode 7 with the first gate installed; when the
//! tasks fall idle the step asks the gate, which walks the figure to the
//! door's node and drops to mode 4; when the figure stands there the step
//! calls the arrival routine, which runs the rule search with flag 4 and
//! finds the walk verb's rule, which installs the second gate and walks
//! the figure on to the door's twin node; when it stands there the step
//! asks that gate, which finds the door in the location's exit table and
//! sets `[1F48h]` and `[1F4Ah]`, and the visit is over.

use crate::num;
use crate::titles::panel::Panel;
use crate::titles::subject::Subject;

use super::{Adventure, Event, KEY_ESCAPE, Opened, objects};

/// `[28BBh]`/`[28BDh]`: which far routine the machine asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Gate {
    /// Nothing installed.
    #[default]
    None,
    /// `1586h:059Fh`, installed by a script exit (`0x15E4F`).
    ScriptExit,
    /// `1586h:0000h`, installed by the walk verb's rule (`0x15B52`).
    Door,
    /// `0EB9h:0489h`, installed by the sixth verb's rule (`0x0F056`) and
    /// by Escape (`0x0FD3E`): the game options.
    Options,
}

/// `[28A3h]`/`[28A5h]`: what the pointer was over when the dispatcher last
/// looked — a hotspot of the room, a widget of the strip, or nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Hover {
    #[default]
    None,
    Spot(usize),
    Widget(usize),
}

/// Bit 0 of `[289Bh]`: a walk is under way for the machine.
pub(crate) const WALK: u16 = 1;
/// Bit 1: input belongs to a script (`0x1041A`), and a click does nothing
/// (`0x0FB9E`).
pub(crate) const LOCKED: u16 = 2;
/// Bit 4: wait for the tasks to fall idle (`0x0F3EA`, answer 10).
pub(crate) const WAIT_TASKS: u16 = 0x10;
/// Bit 7: wait for the figure to stand (`0x0F3E2`, answer 9).
pub(crate) const WAIT_FIGURE: u16 = 0x80;
/// Bits 4 to 8: everything a rule can ask the machine to wait for.
const WAIT_MASK: u16 = 0x1F0;
/// `and word [289Bh], 0FE0Fh`: the wait bits gone, the rest kept.
const KEEP: u16 = 0xFE0F;

/// The twin of an exit's node: the node the figure leaves through
/// (`add ax, 7D0h` at `0x15B7D`, `0x1588B`).
const TWIN: i16 = 0x7D0;

impl Adventure {
    /// The dispatcher's head (`0x0FB69` to `0x0FB91`), once a frame.
    pub(crate) fn dispatch(&mut self) {
        // Event 111 armed task 0 and asked for it to start once the
        // figure is linked (`0x0FB69`).
        if self.script.g1ed0 != 0 && self.player_linked() {
            self.script.g1ed0 = 0;
            self.task_start(0);
        }
        // The query's menu half (`0x0E041`) follows the pointer every
        // frame.
        if self.tasks.t[0].kind == 5 {
            self.menu_hover();
        }
        let busy = self.task_query();
        self.mode_step(busy);
        // `0x0FB91`: an exit asked for ends the frame; `0x0FB9E`: a script
        // that holds the input lets nothing else happen. The options a
        // gate opened end it too.
        if self.leave.is_some()
            || self.entry_word != 0
            || self.bits & LOCKED != 0
            || self.options_up()
        {
            return;
        }
        self.hover_frame(busy);
    }

    /// `0CFDh:0FD9h` without its input half: what task 0 is doing, for
    /// the step.
    ///
    /// The kind `[1DA0h]` of task 0 picks one of six arms (`0x0DFAE`,
    /// `0x0DFBC`).
    ///
    /// The table at `0x0E1B0`: kind 0 answers 0 (`0x0DFC1`), kind 2
    /// answers −1 (`0x0DFC6`), kinds 3 and 4 answer 1 after looking at a
    /// click (`0x0DFCC`, [`Adventure::click_on_text`]), kinds 1 and 5 —
    /// and anything past 5 — answer 2 after the menu's hover (`0x0E041`).
    /// So a hotspot's own text does not count as busy: a walk that waits
    /// for the tasks goes on under it.
    pub(crate) fn task_query(&self) -> i16 {
        match self.tasks.t[0].kind {
            0 => 0,
            2 => -1,
            3 | 4 => 1,
            _ => 2,
        }
    }

    /// `0EB9h:0965h(busy)`: the step.
    ///
    /// Modes 4 to 8 go through a table at `cs:0A03h` (`0x0F4FB`, `0x0F509`).
    /// In modes 4, 5 and 6 the figure standing, `[16BBh]` not `FFFFh`
    /// (`0x0F50E`), with nothing busy (`0x0F515`) is an arrival (`0x0F519`,
    /// the near call to `087Fh`). Mode 7 becomes 8 (`0x0F51E`). In mode 8 what
    /// is waited for in `[289Bh]` is looked at (`0x0F526`), then the gate is
    /// asked with 0 (`0x0F567`), the status age set to `0Ah` (`0x0F56D`), and
    /// the door that was come through, `[1EDEh]`, looked at (`0x0F573`).
    fn mode_step(&mut self, busy: i16) {
        match self.mode {
            4..=6 => {
                if self.player_linked() && busy <= 0 {
                    self.arrival();
                }
            }
            7 => self.mode = 8,
            8 => {
                let b = self.bits;
                let ask = (b & WAIT_TASKS != 0 && busy <= 0)
                    || (b & 0x20 != 0 && self.word_28bf == 0)
                    || (b & 0x100 != 0 && self.word_28c1 == 0)
                    || (b & WAIT_FIGURE != 0 && self.player_linked())
                    || b & WAIT_MASK == 0;
                if ask {
                    self.ask_gate(0);
                    if self.options_up() {
                        // The frame ends at the gate; the rest of it runs
                        // when the menu closes.
                        return;
                    }
                    self.status_refresh_age = 0x0A;
                }
            }
            _ => {}
        }
        // `0x0F573`: the door the player came through closes once the
        // figure is neither on its node nor walking there.
        if self.entry_door != 0
            && self.player_node_id() != self.entry_door
            && self.walk_target != self.entry_door
        {
            self.entry_door_closes();
        }
    }

    /// `0EB9h:149Dh`: the entry door plays its animation 0 and is
    /// forgotten.
    ///
    /// Only with a door remembered in `[1EDEh]` (`0x1002D`): its record
    /// (`0BB6h:052Bh`, `0x1003A`) plays animation 0 (`0883h:1F82h`,
    /// `0x10042`), and the word is cleared (`0x1004A`).
    pub(super) fn entry_door_closes(&mut self) {
        let door = num::low8(num::unsigned(self.entry_door));
        self.entry_door = 0;
        self.object_anim(door, 0);
    }

    /// `0EB9h:087Fh`: the figure stands, in mode 4, 5 or 6.
    ///
    /// While walking somewhere, `[289Fh]` set (`0x0F413`), and not there yet
    /// (`0x0F41D`), the figure is either still moving or stopped short
    /// (`0x0F4BA`), and stopped short runs the rules with `1080h`, then `80h`
    /// (`0x0F4D3`). Arrived, modes 4 and 5 (`0x0F42C`) run the rules with flag
    /// 4 (`0x0F446`), where 6 is a second object wanted (`0x0F451`) and 7
    /// nothing more (`0x0F456`); mode 6 runs them with flag `20h` (`0x0F4B6`).
    fn arrival(&mut self) {
        if self.walk_target != 0 && self.player_node_id() != self.walk_target {
            // `0x0F4BA`.
            if !self.player_linked() {
                return;
            }
            self.walk_target = 0;
            self.mode = 7;
            self.bits &= KEEP;
            if self.rule_search(0x1080) != 0 {
                self.mode_reset();
            } else {
                let r = self.rule_search(0x80);
                self.mode_answer(r);
            }
            return;
        }
        self.walk_target = 0;
        match self.mode {
            4 | 5 => {
                self.mode = 7;
                self.bits &= KEEP;
                match self.rule_search(4) {
                    // `0x0F460`: the rule wants a second object.
                    6 => {
                        self.mode = 3;
                        self.two_object = 0;
                        self.verb_chosen = u16::from(self.verb);
                        self.second_object = (0, 0);
                        self.status_refresh_age = 1;
                    }
                    7 => {}
                    r => self.mode_answer(r),
                }
            }
            6 => {
                self.mode = 7;
                self.bits &= KEEP;
                let r = self.rule_search(0x20);
                self.mode_answer(r);
            }
            _ => {}
        }
    }

    /// `0EB9h:0836h(code)`: what a rule's answer does to the machine.
    ///
    /// Answers 8 to 12 set a bit of `[289Bh]` (`0x0F3C9`): 8 `40h`
    /// (`0x0F3DB`), 9 `80h`, wait for the figure (`0x0F3E2`), 10 `10h`, wait
    /// for the tasks (`0x0F3EA`), 11 `20h` (`0x0F3F1`), 12 `100h` (`0x0F3F8`).
    /// Anything else resets the machine (`0x0F400`, the near call to `003Fh`).
    fn mode_answer(&mut self, code: i16) {
        match code {
            8 => self.bits |= 0x40,
            9 => self.bits |= WAIT_FIGURE,
            10 => self.bits |= WAIT_TASKS,
            11 => self.bits |= 0x20,
            12 => self.bits |= 0x100,
            _ => self.mode_reset(),
        }
    }

    /// `0EB9h:003Fh`: the machine back to nothing chosen.
    ///
    /// Mode 0 (`0x0EBCF`), `[289Bh]` masked with `FE0Eh` (`0x0EBDB`), no verb
    /// (`0x0EBF9`), nothing hovered, `[28A5h] = FFFFh` (`0x0EC35`), and the
    /// status age 2 (`0x0EC41`). The objects, the second object, the verb as
    /// chosen and the six hover points between them are cleared too.
    pub(crate) fn mode_reset(&mut self) {
        self.mode = 0;
        self.two_object = 0;
        self.bits &= 0xFE0E;
        self.object1 = 0;
        self.object2 = 0;
        self.second_object = (0, 0);
        self.verb = 0;
        self.verb_chosen = 0;
        self.hover_spot = None;
        self.second_spot = None;
        self.last_hover = Hover::None;
        self.status_refresh_age = 2;
    }

    /// `0EB9h:0654h(arg)`: ask the gate, and clear up when it answers.
    ///
    /// The answer is 1 when there is no gate or the mode is below 4
    /// (`0x0F1E8`, `0x0F1EB`). Otherwise the gate `[28BBh]` is called
    /// (`0x0F1FE`); 0 is nothing more (`0x0F207`), and anything else clears the
    /// wait bits, `[289Bh]` masked with `FE0Fh` (`0x0F209`), uninstalls the
    /// gate (`0x0F215`) and resets the machine (`0x0F220`).
    ///
    /// Between the two, `0x0F21B` calls `179Eh:1017h`: the pointer's shape
    /// taken again.
    pub(super) fn ask_gate(&mut self, arg: i16) -> i16 {
        let mut answer = 1;
        if self.mode >= 4 && self.gate != Gate::None {
            answer = match self.gate {
                Gate::None => 1,
                Gate::ScriptExit => self.gate_script_exit(arg),
                Gate::Door => self.gate_door(arg),
                Gate::Options => {
                    if self.gate_options(arg) {
                        // The menu is up and the rest of this call runs
                        // when it closes (`Adventure::options_gate_close`).
                        return 1;
                    }
                    0
                }
            };
        }
        if answer != 0 {
            self.bits &= KEEP;
            self.gate = Gate::None;
            self.pointer_resync();
            self.mode_reset();
        }
        answer
    }

    /// `1586h:05E1h(k)`: a script leaves through exit `100 + k`.
    ///
    /// The hover is forgotten (`0EB9h:18F4h`, `0x15E44`) and the gate
    /// `1586h:059Fh` installed (`0x15E49`, `0x15E4F`); the wait bits go,
    /// `[289Bh]` masked with `FE0Fh` (`0x15E55`), and bits `12h` — locked, wait
    /// for the tasks — set (`0x15E5B`). The object is the exit's, class 1
    /// (`0x15E63`, `0x15E66`). Below mode 7 (`0x15E6C`) it is a walk, bit 0
    /// (`0x15E73`), and the mode becomes 7 (`0x15E78`).
    ///
    /// Nothing walks yet: the gate does, once the text is over.
    pub(crate) fn script_exit(&mut self, object: u8) {
        self.hover_forgotten();
        self.gate = Gate::ScriptExit;
        self.bits = (self.bits & KEEP) | LOCKED | WAIT_TASKS;
        self.object1 = u16::from(object);
        self.object2 = 1;
        if self.mode < 7 {
            self.bits |= WALK;
            self.mode = 7;
        }
        self.events.push(Event::ScriptExit(object));
    }

    /// `1586h:059Fh(arg)`: the script exit's gate.
    ///
    /// Only when asked with 0 (`0x15E02`): the walk verb `0Ah` becomes the
    /// chosen and the current verb (`0x15E08`, `0x15E0B`), the mode 4
    /// (`0x15E11`), bit 0 of `[289Bh]` set (`0x15E17`), and the figure walks
    /// to the object's node, `[289Fh] = [2891h]` (`0x15E1E`, `0A9Fh:0617h` at
    /// `0x15E29`). The gate uninstalls itself (`0x15E31`) and answers 0,
    /// nothing more (`0x15E3D`).
    fn gate_script_exit(&mut self, arg: i16) -> i16 {
        if arg != 0 {
            return 0;
        }
        self.verb_chosen = 0x0A;
        self.verb = 0x0A;
        self.mode = 4;
        self.bits |= WALK;
        let object = num::signed(self.object1);
        self.walk_target = object;
        self.walk_to_object(object, -1);
        self.gate = Gate::None;
        0
    }

    /// `1586h:0000h(arg)`: the door's gate.
    ///
    /// Asked with anything but 0 it answers 1 (`0x15865`). Otherwise input
    /// is locked (`0EB9h:188Ah`, `0x1586E`) and the verb is `0Ah` (`0x15878`).
    /// Not yet walking to a twin, `[289Fh]` below `7D0h` (`0x1587E`), the
    /// figure walks to the object's twin, its id plus `7D0h` (`0x1588B`,
    /// `0A9Fh:0617h` at `0x15893`); when that walk started (`0x1589B`) it
    /// waits for the figure, bit `80h` (`0x158AE`), and answers 0. Else the
    /// exit table of the row decides (`0x158B8`): the eight ids at `+9`
    /// (`0x158CE`), the eight destinations at `+11h` (`0x158E7`), a
    /// destination below 61 (`0x158FC`) and of a kind other than 0
    /// (`0x1590B`); the exit goes into `[1F48h]` and where it leads into
    /// `[1F4Ah]` (`0x15925`, `0x15929`).
    fn gate_door(&mut self, arg: i16) -> i16 {
        if arg != 0 {
            return 1;
        }
        self.input_locked();
        self.hover_forgotten();
        self.verb = 0x0A;
        let object = num::signed(self.object1);
        if self.walk_target < TWIN && self.walk_to_object(object + TWIN, -1) > 0 {
            self.walk_target = object + TWIN;
            self.bits = (self.bits & KEEP) | WAIT_FIGURE;
            return 0;
        }
        let here = usize::try_from(self.loc).unwrap_or(usize::MAX);
        let dest = self
            .locations
            .exits(&self.image, here)
            .into_iter()
            .find(|&(exit, _)| i16::from(exit) == object)
            .map(|(_, dest)| i32::from(dest));
        if let Some(dest) = dest
            && dest > 0
            && dest < 61
        {
            let kind = usize::try_from(dest)
                .ok()
                .and_then(|d| self.locations.kind(&self.image, d))
                .unwrap_or(0);
            if kind == 0 {
                // `0x15919`: a row that is not a place. The lock comes
                // off and the pointer is reset; the visit goes on.
                self.input_unlocked();
                self.hover_reset();
            } else {
                // `0x15929`: the exit id and the destination into the
                // main loop's two words; the handler's frame loop sees
                // the first and ends the visit.
                self.entry_word = u16::from(num::low8(num::unsigned(object)));
                self.location_word = num::low16(dest);
                self.leave = Some((num::low8(num::unsigned(object)), dest));
            }
        }
        1
    }

    /// `13D0h:009Ch(flags)`: the rule search, and the handler it finds.
    ///
    /// The location's own list comes first (`cs:[4]`, `0x13DA8`): rules for
    /// the object `[2891h]` (`0x13DB3`), for its class `[2893h]` complemented
    /// (`0x13DBC`), and for object 0 (`0x13DD5`); then the global table the
    /// same way (`cs:[0]`, `0x13DDC`); then both lists from their first rule
    /// for any object, −1 (`0x13E04`, `0x13E13`). A rule takes part when its
    /// flags meet this phase's (`0x13E54`) and its verb is this one or any
    /// (`0x13E5E`); its handler is called with the flags (`0x13EB3`). An
    /// answer of 0 keeps looking (`0x13EBD`), −1 starts over (`0x13E32`), and
    /// nothing found answers −1 (`0x13E3A`).
    ///
    /// A handler is code. The one read is the walk verb's, `1586h:02E9h`
    /// ([`Adventure::walk_rule`]); any other that the search reaches is
    /// noted as [`Event::UnreadRule`] and taken as answered with 1, which
    /// resets the machine — the same as a handler that did nothing.
    fn rule_search(&mut self, flags: u16) -> i16 {
        let here = usize::try_from(self.loc).unwrap_or(usize::MAX);
        let own = self.locations.rules(&self.image, here);
        let global = self.global_rules.clone();
        for _restart in 0..4 {
            let verb = u16::from(self.verb);
            let object = num::signed(self.object1);
            let class = !num::signed(self.object2);
            let keys: Vec<i16> = if self.object2 != 0 {
                vec![object, class, 0]
            } else {
                vec![object, 0]
            };
            let mut answer = None;
            'lists: for (list, keys) in [
                (&own, &keys),
                (&global, &keys),
                (&own, &vec![-1]),
                (&global, &vec![-1]),
            ] {
                for &key in keys {
                    for r in list.iter() {
                        if r.object != key || r.phase & flags == 0 {
                            continue;
                        }
                        if flags & 1 == 0 && r.verb != verb && r.verb != 0 {
                            continue;
                        }
                        if flags & 0x3000 != 0 {
                            answer = Some(0);
                            break 'lists;
                        }
                        let a = self.rule_handler(r.handler, flags);
                        if a != 0 {
                            answer = Some(a);
                            break 'lists;
                        }
                    }
                }
            }
            match answer {
                Some(-1) => continue,
                Some(a) => return a,
                None => return -1,
            }
        }
        -1
    }

    /// The handler a rule names, by its far address: the location logic's
    /// first, then the two of the library's that are read here.
    fn rule_handler(&mut self, far: (u16, u16), flags: u16) -> i16 {
        if let Some(answer) = (self.rule_hook)(self, far, flags) {
            return answer;
        }
        match far {
            (0x1586, 0x02E9) => self.walk_rule(),
            // `0EB9h:04BDh` (`0x0F04D`): the sixth verb's rule installs the
            // options' gate and waits for the figure.
            (0x0EB9, 0x04BD) => {
                self.gate = Gate::Options;
                9
            }
            (0x1586, 0x0364) => self.door_rule(),
            (0x1586, 0x0125) => self.rule_look(),
            (0x1586, 0x017C) => self.rule_talk(),
            (0x1586, 0x01CC) => self.rule_take(),
            (0x1586, 0x0273) => self.rule_use(),
            (0x1586, 0x02C3) => self.rule_use_with(),
            (0x1586, 0x00D3) => self.rule_give(),
            (0x1586, 0x04A3) => self.rule_take_or_give(),
            // The hover rules, phase 1: the verb a class of object means
            // when none is picked (`0x15BB6` and on).
            (0x1586, 0x0356) | (0x1586, 0x040E) => {
                self.verb = if far.1 == 0x0356 { 1 } else { 10 };
                1
            }
            (0x1586, 0x03CE) => {
                self.verb = 10;
                1
            }
            (0x1586, 0x041C) => {
                self.verb = 4;
                1
            }
            (0x1586, 0x042A) => {
                self.verb = 2;
                1
            }
            // The phase-2 rules of `Benutze` on the classes 5, 2 and 4
            // (`0x15C3C`, `0x15C4A`, `0x15E80`): the verb becomes the walk
            // verb — or 7 for an object not live, 8 for a live class 5.
            (0x1586, 0x03DC) => {
                self.verb = 10;
                5
            }
            (0x1586, 0x03EA) => {
                let object = num::signed(self.object1);
                self.verb = if self.object_is_live(object) { 10 } else { 7 };
                5
            }
            (0x1586, 0x0620) => {
                let object = num::signed(self.object1);
                self.verb = if self.object_is_live(object) { 8 } else { 7 };
                5
            }
            // The item rules (`0x15D27` to `0x15DFE`): items 9 and 2 given
            // up for item 3, and a scene by the pair.
            (0x1586, 0x04C7) => {
                self.take_item(9);
                self.take_item(2);
                self.give_item(3);
                1
            }
            (0x1586, 0x04E7) => self.rule_scene(!22),
            (0x1586, 0x04F7) => {
                let n = if self.sav_byte(0x270E) != 0 { !33 } else { !21 };
                self.rule_scene(n)
            }
            (0x1586, 0x0512) => {
                let n = if self.carried(num::low8(self.second_object.0)) {
                    !25
                } else if self.second_object.1 == 2 {
                    !24
                } else {
                    !23
                };
                self.rule_scene(n)
            }
            (0x1586, 0x053F) => self.rule_scene(!27),
            (0x1586, 0x054F) => self.rule_scene(!28),
            (0x1586, 0x055F) => self.rule_scene(!29),
            (0x1586, 0x056F) => self.rule_scene(!30),
            (0x1586, 0x057F) => self.rule_scene(!31),
            (0x1586, 0x058F) => self.rule_scene(!32),
            _ => {
                self.events.push(Event::UnreadRule { far, flags });
                1
            }
        }
    }

    /// A rule that is one scene and the answer `0Ah`.
    fn rule_scene(&mut self, n: i16) -> i16 {
        self.scene_load(n);
        10
    }

    /// The text slot `k` of the hotspot the object was chosen through
    /// (`[28B3h]`, its `+16h + 4k`), when it has one.
    fn own_text(&self, k: usize) -> Option<Vec<u8>> {
        let spot = self.room()?.hotspots.spots.get(self.hover_spot?)?;
        let text = spot.texts.get(k)?;
        (!text.is_empty()).then(|| motionvm_parsec_formats::to_cp437(text))
    }

    /// `1586h:0125h`, `Betrachte` on anything: the hotspot's `texts[3]`
    /// (`0x15996`, `+22h`), else bank 2's place for an item
    /// (`~(id + 4000h)`, `0x159C7`), else bank 999's place 16.
    fn rule_look(&mut self) -> i16 {
        if let Some(text) = self.own_text(3) {
            self.hotspot_text(&text);
        } else if (1..0x17).contains(&self.object1) {
            let n = !(num::signed(self.object1) + 0x4000);
            self.scene_load(n);
        } else {
            self.scene_load(!16);
        }
        10
    }

    /// `1586h:017Ch`, `Sprich mit`: `texts[4]` (`+26h`), else place 4 for
    /// something carried and 3 for anything else.
    fn rule_talk(&mut self) -> i16 {
        if let Some(text) = self.own_text(4) {
            self.hotspot_text(&text);
        } else if self.carried(num::low8(self.object1)) {
            self.scene_load(!4);
        } else {
            self.scene_load(!3);
        }
        10
    }

    /// `1586h:01CCh`, `Nimm`: `texts[5]` (`+2Ah`); else an item that is
    /// there and live goes out of the room and into the player's hands
    /// (`0x15A8F`, `0x15AB2`); else one of places 9, 10 and 11, rolled.
    fn rule_take(&mut self) -> i16 {
        if let Some(text) = self.own_text(5) {
            self.hotspot_text(&text);
            return 10;
        }
        if self.object1 < 0x17
            && let Some(i) = self.live_object(num::low8(self.object1))
        {
            self.objects[i].flags &= !objects::LIVE;
            let id = num::low8(self.object1);
            self.events.push(Event::ObjectHidden(id));
            self.give_item(id);
            return 10;
        }
        let n = !num::signed(self.roll(9, 11));
        self.scene_load(n);
        10
    }

    /// `1586h:0273h`, `Benutze`: place 7 for something carried; else
    /// `texts[2]` (`+1Eh`), else place 6.
    fn rule_use(&mut self) -> i16 {
        if self.carried(num::low8(self.object1)) {
            self.scene_load(!7);
        } else if let Some(text) = self.own_text(2) {
            self.hotspot_text(&text);
        } else {
            self.scene_load(!6);
        }
        10
    }

    /// `1586h:02C3h`, `Benutze … mit`: place 15 when both are class 2, 12
    /// when the second is, 8 otherwise.
    fn rule_use_with(&mut self) -> i16 {
        let n = if self.second_object.1 == 2 && self.object2 == 2 {
            !15
        } else if self.second_object.1 == 2 {
            !12
        } else {
            !8
        };
        self.rule_scene(n)
    }

    /// `1586h:00D3h`, `Gib … an`: one of places 0 to 2 rolled when the
    /// second object is carried; else for a class-2 second object slot 9
    /// names it and place 14 plays; else place 5.
    fn rule_give(&mut self) -> i16 {
        let second = self.second_object;
        let n = if self.carried(num::low8(second.0)) {
            !num::signed(self.roll(0, 2))
        } else if second.1 == 2 {
            if let Ok(id) = u8::try_from(second.0) {
                self.subjects.set(9, 0, Subject::Object(id));
            }
            !14
        } else {
            !5
        };
        self.rule_scene(n)
    }

    /// `1586h:04A3h`, the third button's rule: `Gib` for something carried,
    /// `Nimm` otherwise (`0x15D14`, `0x15D1C`); answer 5.
    fn rule_take_or_give(&mut self) -> i16 {
        self.verb = if self.carried(num::low8(self.object1)) {
            9
        } else {
            12
        };
        5
    }

    /// `0EB9h:0A0Dh(id, hotspot)`: an object chosen, by hover or click.
    ///
    /// An id below `17h` is an item, with its name row and class 0
    /// (`0x0F5AC`); a hotspot takes its class from bits 8 to 11 of its mask
    /// (`0x0F5EE`). In mode 0 the same object again does nothing (`0x0F60B`);
    /// otherwise the hover rules run with phase 1 (`13D0h:009Ch`, `0x0F636`),
    /// and their answer sets `[2899h]` to 1 for −1 or 2, nothing to do here
    /// (`0x0F648`), else to 2, something to do (`0x0F663`). Mode 2 runs phase
    /// 2 (`0x0F6BA`), mode 3 phase `10h`, for the second object (`0x0F796`).
    pub(crate) fn select_object(&mut self, id: i16, spot: Option<usize>) {
        let id = if id <= 0 { 0x7FFF } else { id };
        let class = if id < 0x17 {
            // `DS:0527h + id`: the item classes, all zero in the image.
            0
        } else {
            let Some(mask) =
                spot.and_then(|i| self.room()?.hotspots.spots.get(i).map(|h| h.verb_mask))
            else {
                return;
            };
            (mask >> 8) & 0xF
        };
        let id = num::unsigned(id);
        match self.mode {
            0 => {
                if self.object1 == id {
                    return;
                }
                self.object1 = id;
                self.object2 = class;
                self.hover_spot = spot;
                match self.rule_search(1) {
                    -1 | 2 => {
                        self.two_object = 1;
                        self.object2 = 0;
                    }
                    _ => self.two_object = 2,
                }
            }
            2 => {
                if self.two_object == 4 && self.object1 != id {
                    self.two_object = 0;
                    self.verb = num::low8(self.verb_chosen);
                }
                if self.object1 == id {
                    return;
                }
                self.object1 = id;
                self.object2 = class;
                self.hover_spot = spot;
                self.verb_chosen = u16::from(self.verb);
                self.two_object = match self.rule_search(2) {
                    2 => {
                        self.object2 = 0;
                        1
                    }
                    5 => 4,
                    7 => 5,
                    _ => 3,
                };
            }
            3 => {
                if self.object1 == id && id != 0x7FFF && self.second_object.0 != id {
                    self.second_object = (id, 0);
                    self.two_object = 1;
                    self.second_spot = None;
                    self.status_refresh_age = 1;
                }
                if matches!(self.two_object, 4 | 5) && self.second_object.0 != id {
                    self.two_object = 0;
                    self.verb = num::low8(self.verb_chosen);
                }
                if self.second_object.0 == id {
                    return;
                }
                self.second_object = (id, class);
                self.second_spot = spot;
                self.verb_chosen = u16::from(self.verb);
                self.two_object = match self.rule_search(0x10) {
                    2 => {
                        self.second_object.1 = 0;
                        1
                    }
                    5 => 4,
                    7 => 5,
                    _ => 3,
                };
            }
            _ => return,
        }
        self.status_refresh_age = 1;
    }

    /// `0EB9h:0C58h(id, hotspot)`: a click on an object of the room or an
    /// item of the strip.
    ///
    /// Modes 4 to 8 go through a table at `cs:0D7Bh` (`0x0F7FA`): the same
    /// object again takes modes 4, 5, 6 back to 0, 2, 3 (`0x0F818`), another
    /// asks the gate with 1, mode 8 with 2 (`0x0F82F`). In modes 0 to 3 the
    /// object is chosen (`0A0Dh`, `0x0F80C`); then in modes 0 and 2
    /// (`0x0F89F`) with something to do (`0x0F8B2`) a two-object verb wants its
    /// second (`0776h`, `0x0F8BE`), else the rule is looked for (`07C7h`,
    /// `0x0F8C5`) and the figure walks (`0698h`, `0x0F8D4`); without a walk the
    /// arrival comes at once (`087Fh`, `0x0F905`).
    pub(crate) fn click_object(&mut self, id: i16, spot: Option<usize>) {
        let id = if id <= 0 { 0x7FFF } else { id };
        match self.mode {
            4..=6 => {
                let same = if self.mode == 6 {
                    self.second_object.0 == num::unsigned(id)
                } else {
                    self.object1 == num::unsigned(id)
                };
                if same {
                    self.mode = match self.mode {
                        4 => 0,
                        5 => 2,
                        _ => 3,
                    };
                    self.status_refresh_age = 1;
                    return;
                }
                if self.ask_gate(1) == 0 {
                    return;
                }
                self.select_object(id, spot);
                return;
            }
            7 => return,
            8 => {
                if self.ask_gate(2) == 0 {
                    return;
                }
                self.select_object(id, spot);
                return;
            }
            _ => {}
        }
        self.select_object(id, spot);
        // `0x0F892`: a room object needs its hotspot; an item needs none.
        if id >= 0x17 && spot.is_none() {
            return;
        }
        match self.mode {
            0 | 2 => {
                if matches!(self.two_object, 0 | 1) {
                    return;
                }
                if !self.two_object_verb() {
                    self.check_rule();
                }
                let object = num::signed(self.object1);
                let spot = self.hover_spot;
                if !self.walk_to_hotspot(object, spot) {
                    self.arrival();
                }
            }
            3 => {
                if matches!(self.two_object, 0 | 1) {
                    return;
                }
                self.check_rule();
                let object = num::signed(self.second_object.0);
                let spot = self.second_spot;
                if !self.walk_to_hotspot(object, spot) {
                    self.arrival();
                }
            }
            _ => {}
        }
    }

    /// `0EB9h:0776h`: a verb that wants a second object — 5 and 9 — puts
    /// the machine into mode 3 and clears the second (`0x0F313`).
    fn two_object_verb(&mut self) -> bool {
        if !matches!(self.verb, 5 | 9) {
            return false;
        }
        self.mode = 3;
        self.two_object = 0;
        self.verb_chosen = u16::from(self.verb);
        self.second_object = (0, 0);
        self.second_spot = None;
        self.status_refresh_age = 1;
        true
    }

    /// `0EB9h:07C7h`: whether a rule answers for the pair, asked without
    /// running it (`1004h`, `1020h`); when one does the machine goes to
    /// the acting mode — 4 from 0, 5 from 2, 6 from 3.
    fn check_rule(&mut self) {
        if self.two_object < 2 {
            return;
        }
        let (next, flags) = match self.mode {
            0 => (4, 0x1004),
            2 => (5, 0x1004),
            3 => (6, 0x1020),
            _ => {
                self.status_refresh_age = 1;
                return;
            }
        };
        if self.rule_search(flags) == 0 {
            self.mode = next;
            self.verb_chosen = u16::from(self.verb);
            if self.two_object == 4 {
                self.two_object = 3;
            }
        }
        self.status_refresh_age = 1;
    }

    /// `0EB9h:0698h(object, hotspot)`: the figure walks where the hotspot
    /// says. Answers whether a walk is under way.
    ///
    /// The mask's top two bits decide (`0x0F24C`): `01` puts the node in the
    /// low 14 bits (`0x0F253`); `1x` means the node with the object's id, the
    /// low byte against it (`0x0F26D`), with the facing in bits 12 and 13
    /// (`0x0F276`), and a figure already there only faces it (`0A9Fh:0C0Ch`,
    /// `0x0F290`). With nowhere to go from a twin, `[289Fh]` at `7D0h` or
    /// above, it backs off it (`0x0F29C`); otherwise the walk (`0A9Fh:0617h`,
    /// `0x0F2C5`). With no hotspot a room object, id `17h` or above, walks to
    /// its own node (`0x0F2DC`).
    fn walk_to_hotspot(&mut self, object: i16, spot: Option<usize>) -> bool {
        let Some(mask) = spot.and_then(|i| self.room()?.hotspots.spots.get(i).map(|h| h.verb_mask))
        else {
            if object >= 0x17 {
                if self.walk_to_object(object, -1) != 0 {
                    self.walk_target = object;
                    return true;
                }
                self.walk_target = 0;
            }
            return false;
        };
        let mut target = 0i16;
        let mut facing = -1i16;
        match mask & 0xC000 {
            0 => {}
            0x4000 => {
                let node = num::signed(mask & 0x3FFF);
                if self.player_node_id() != node {
                    target = node;
                }
            }
            _ => {
                if i16::from(num::low8(mask)) == object {
                    facing = num::signed((mask >> 12) & 3);
                    if self.player_node_id() != object {
                        target = object;
                    } else {
                        self.face_player(facing);
                    }
                }
            }
        }
        if target == 0 {
            if self.walk_target >= TWIN {
                let back = self.walk_target - TWIN;
                self.walk_to_object(back, -1);
            }
            return false;
        }
        if self.walk_to_object(target, facing) != 0 {
            self.walk_target = target;
            return true;
        }
        false
    }

    /// The dispatcher's tail (`0x0FDE1` to `0x0FFEF`): what the pointer is
    /// over, once a frame while nothing is busy.
    ///
    /// The same record under the pointer as last frame, `[28A5h]` against
    /// `[1862h]`, does nothing (`0x0FDE1`); with the pointer not followed,
    /// `[1E64h]` zero, it is only remembered (`0x0FDF7`). Over the strip,
    /// `0C3Fh:08EBh` answers −1 frozen, the complement of a verb, an item, or
    /// 0 (`0x0FE01`). Over the room mode 1 resets (`0x0FE17`); a hotspot, bit
    /// 15 of its record's `+10h` (`0x0FE3C`), gets its `texts[0]` as the hover
    /// text (`0CFDh:1810h`, `0x0FE8F`) and is chosen through `0A0Dh`
    /// (`0x0FEC6`). Over nothing the hover text stops (`0x0FEC9`), modes 0 and
    /// 1 reset (`0x0FED5`) and the object is let go (`0x0FF37`). A verb button
    /// is taken in mode 1 (`0x0FF9F`); an item is chosen (`0A0Dh(si, 0, 0)`,
    /// `0x0FFC8`).
    pub(crate) fn hover_frame(&mut self, busy: i16) {
        if busy > 0 {
            return;
        }
        let (x, y) = self.mouse;
        let widget = if y >= self.panel_y {
            self.panel_widgets.hit(x, y).map(|w| usize::from(w.id))
        } else {
            None
        };
        let spot = if widget.is_none() {
            self.hotspot_index_under_pointer()
        } else {
            None
        };
        let now = match (widget, spot) {
            (Some(w), _) => Hover::Widget(w),
            (None, Some(i)) => Hover::Spot(i),
            _ => Hover::None,
        };
        if now == self.last_hover && now != Hover::None {
            return;
        }
        if !self.hover_enabled {
            // `0x0FFD5`.
            if busy != 0 {
                self.task_stop(0);
            }
            self.last_hover = now;
            return;
        }
        // `0C3Fh:08EBh` answers for a verb button and for a slot that holds
        // an item; any other widget — an empty slot, an arrow — answers 0
        // (`0x0CD2E`, `0x0CD3C`), and the tail goes on as over the room,
        // where its record is no hotspot (`0x0FE3C`): the nothing road.
        let taken = widget
            .and_then(|_| self.panel_widgets.hit(x, y))
            .is_some_and(|w| {
                Panel::verb_of(w).is_some()
                    || Panel::slot_of(w)
                        .and_then(|n| self.item_in_slot(n))
                        .is_some()
            });
        if widget.is_some() && taken {
            // `0x0FF81`.
            if busy != 0 {
                self.task_stop(0);
            }
            self.last_hover = now;
            self.hover_taken(x, y);
            return;
        }
        if self.mode == 1 {
            self.mode_reset();
        }
        match spot {
            Some(i) => {
                self.last_hover = now;
                let (id, text) = {
                    let Some(h) = self.room().and_then(|r| r.hotspots.spots.get(i)) else {
                        return;
                    };
                    (
                        i16::from(num::low8(h.verb_mask)),
                        h.texts
                            .first()
                            .filter(|t| !t.is_empty())
                            .map(|t| motionvm_parsec_formats::to_cp437(t)),
                    )
                };
                match text {
                    Some(text) => {
                        if !self.hover_text(&text) {
                            self.last_hover = Hover::None;
                        }
                    }
                    None => {
                        if busy < 0 {
                            self.task_stop(0);
                        }
                    }
                }
                self.select_object(id, Some(i));
            }
            None => self.hover_nothing(now, busy),
        }
    }

    /// `0x0FF81`: a verb button or a slot with an item under the pointer —
    /// a verb taken in mode 1, an item chosen.
    fn hover_taken(&mut self, x: i32, y: i32) {
        let Some(widget) = self.panel_widgets.hit(x, y) else {
            return;
        };
        if let Some(v) = Panel::verb_of(widget) {
            if v >= 0xF || v == self.verb || self.mode >= 2 {
                return;
            }
            self.mode_reset();
            self.verb = v;
            self.mode = 1;
            self.status_refresh_age = 1;
        } else if let Some(item) = Panel::slot_of(widget).and_then(|n| self.item_in_slot(n)) {
            self.select_object(i16::from(item), None);
        }
    }

    /// `0x0FEC9`: nothing under the pointer — the hover text stopped,
    /// modes 0 and 1 reset, and the object let go.
    fn hover_nothing(&mut self, now: Hover, busy: i16) {
        if busy != 0 {
            self.task_stop(0);
        }
        if self.mode <= 1 {
            self.mode_reset();
        }
        // `0x0FEDF`: the same record as last time, nothing more.
        if self.last_hover == now {
            return;
        }
        self.last_hover = now;
        if self.two_object == 4 {
            self.two_object = 0;
            self.verb = num::low8(self.verb_chosen);
            self.status_refresh_age = 1;
        }
        match self.mode {
            0 | 2 => {
                if self.object1 == 0 {
                    return;
                }
                self.object1 = 0;
                self.object2 = 0;
                self.hover_spot = None;
            }
            3 => {
                if self.second_object.0 == 0 {
                    return;
                }
                self.second_object = (0, 0);
                self.second_spot = None;
            }
            _ => return,
        }
        self.status_refresh_age = 1;
    }

    /// `1586h:0364h`: the seventh verb's rule — verb 7, any object, flag 4
    /// (`0x2155A`) — which the walk verb's rule hands a door to.
    ///
    /// The object's record (`0BB6h:052Bh`, `0x15BCC`) of kind 4 is a door
    /// (`0x15BE0`): it plays its animation 1 and opens, thirty ticks to code 2
    /// (`0883h:1F82h`, `0x15BED`); with no door remembered (`0x15BF5`) it
    /// becomes the door to close later, `[1EDEh]` (`0x15BFF`); `[28C1h] = 2`
    /// waits for the animation's code 2 (`0x15C02`), and the answer is 12
    /// (`0x15C08`). Anything else asks for sample 12 (`0EB9h:1949h`,
    /// `0x15C0D`) and is made live and marked, bit 0 of `+2` (`0x15C18`), and
    /// the answer is 1.
    fn door_rule(&mut self) -> i16 {
        let Ok(id) = u8::try_from(self.object1) else {
            return 1;
        };
        let Some(i) = self.object_index(id) else {
            return 1;
        };
        if self.objects[i].kind == 4 {
            self.object_anim(id, 1);
            if self.entry_door == 0 {
                self.entry_door = i16::from(id);
            }
            self.anim_code_wait(2);
            12
        } else {
            self.events.push(Event::Sample(12));
            self.objects[i].flags |= objects::LIVE;
            1
        }
    }

    /// `[28C1h] = code`: the machine waits until an animation reaches an
    /// opcode carrying `code`, which the animator reports through
    /// `0EB9h:1D97h` (`0x0109A8`, `cmp si, [28C1h]`; `0x0109AE` clears it).
    fn anim_code_wait(&mut self, code: u16) {
        self.word_28c1 = code;
    }

    /// `1586h:02E9h`: the walk verb's rule — verb `0Ah`, any object, flag 4
    /// (`0x215CA`).
    ///
    /// The door's gate `1586h:0000h` is installed (`0x15B4C`). The door come
    /// through, `[1EDEh]` (`0x15B58`), or one stood on, `[16BBh]`
    /// (`0x15B61`), equal to the object answers verb 7, −1; so does an object
    /// that is not live (`14DDh:07E7h`, `0x15B6E`). Else the figure walks to
    /// the object's twin, its id plus `7D0h` (`0x15B7D`, `0A9Fh:0617h` at
    /// `0x15B85`), and when that walk started, input is locked
    /// (`0EB9h:188Ah`, `0x15B91`) and the walk recorded in `[289Fh]`
    /// (`0x15BA1`); the answer is 9, wait for the figure (`0x15BB1`).
    fn walk_rule(&mut self) -> i16 {
        self.gate = Gate::Door;
        let object = num::signed(self.object1);
        if self.entry_door == object && self.player_node_id() == object {
            self.verb = 7;
            return -1;
        }
        if !self.object_is_live(object) {
            self.verb = 7;
            return -1;
        }
        if self.walk_to_object(object + TWIN, -1) != 0 {
            self.input_locked();
            self.hover_forgotten();
            self.walk_target = object + TWIN;
        }
        9
    }

    /// `14DDh:07E7h(id)`: whether the object with this id is live.
    ///
    /// The record comes from `0BB6h:052Bh` (`0x155BE`). Kinds 1 and 2
    /// (`0x155DC`) answer bit 0 of `+2` (`0x155F6`); kind 4 (`0x155E6`)
    /// answers whether bits 0 and 1 of `+2` are 1 (`0x15604`); any other kind
    /// answers 3 (`0x15615`).
    ///
    /// An id with no record at all answers −1 (`0x0155CF`), which every
    /// caller takes as live: a door with no object of its own opens.
    ///
    /// The record is one of the location's objects as they stand now.
    pub(crate) fn object_is_live(&self, id: i16) -> bool {
        let Ok(id) = u8::try_from(id) else {
            return true;
        };
        let Some(o) = self.object_index(id).and_then(|i| self.objects.get(i)) else {
            return true;
        };
        match o.kind {
            1 | 2 => o.flags & 1 == 1,
            4 => o.flags & 3 == 1,
            _ => true,
        }
    }

    /// `0EB9h:188Ah`: input locked to the script.
    ///
    /// Bit 1 of `[289Bh]` is set (`0x1041A`), nothing is hovered,
    /// `[28A5h] = FFFFh` (`0x1041F`), and the band held dark
    /// (`0C3Fh:0581h(4)`, `0x1042B`).
    pub(super) fn input_locked(&mut self) {
        self.bits |= LOCKED;
        self.panel_tint(4);
    }

    /// `0EB9h:18A4h`: the lock off again, the band let go.
    pub(super) fn input_unlocked(&mut self) {
        self.bits &= !LOCKED;
        self.panel_release(4);
    }

    /// `0EB9h:18F4h`: the hover forgotten and the pointer reset.
    ///
    /// `[1E64h]` is cleared (`0x10484`), `[28A5h] = FFFFh` (`0x1048A`), the
    /// status age 1 (`0x10496`). Then the pointer goes off the screen
    /// (`179Eh:0882h`, `0x1049C`) and is unfrozen again (`179Eh:091Ah`,
    /// `0x104A1`), so it keeps animating unseen.
    pub(crate) fn hover_forgotten(&mut self) {
        self.hover_enabled = false;
        self.last_hover = Hover::None;
        self.status_refresh_age = 1;
        self.pointer_off();
        self.pointer_thaw();
    }

    /// `0EB9h:1917h`: the same with `[1E64h]` set, and the input flushed;
    /// the pointer is unfrozen, the keys flushed, the input polled twice
    /// and the pointer shown.
    ///
    /// The calls in order: `179Eh:091Ah` (`0x104BF`), `179Eh:071Dh`
    /// (`0x104C4`), `179Eh:01B0h` twice (`0x104C9`, `0x104CE`) and
    /// `179Eh:0830h` (`0x104D3`).
    pub(super) fn hover_reset(&mut self) {
        self.hover_enabled = true;
        self.last_hover = Hover::None;
        self.status_refresh_age = 1;
        self.pointer_thaw();
        self.pointer_poll();
        self.pointer_poll();
        self.pointer_on();
    }

    /// `0A9Fh:0617h(1693h, id, facing)`: walk the figure to the node with
    /// `id`, with the routine's own answer: 2 when it already stands there
    /// — and then turns it round, `02CFh(walker, facing + 3)` with a facing
    /// of −1 being the about-face code (`0x0B04D`) — or when a walk
    /// started (`0x0B139`), 1 when a running leg is kept (`0x0B25C`), 0
    /// when there is no such node or no way, −1 when the first edge is
    /// blocked.
    ///
    /// A running animation is stopped first (`14DDh:01D4h`, `0x0B030`). On
    /// the node already, `+28h` (`0x0B03B`), it faces and answers 2
    /// (`0x0B04D`); else the node is looked up by id (`0x0B08E`), none
    /// answering 0 (`0x0B09B`), and a leg found answers 1 (`0x0B25C`).
    pub(crate) fn walk_to_object(&mut self, id: i16, facing: i16) -> i16 {
        self.stop_player_anim();
        let Some(mut w) = self.walker.take() else {
            return 0;
        };
        let answer = match self.room().and_then(|r| r.walk()) {
            Some(graph) => w.walk_to_code(graph, id, facing),
            None => 0,
        };
        self.walker = Some(w);
        self.sync_links();
        answer
    }
}

impl Adventure {
    /// `0EB9h:0489h(arg)` (`0x0F019`), the options' gate, up to the menu.
    ///
    /// Asked with anything but 0 it answers 0 (`0x0F01C`). Otherwise task 0
    /// stops (`0CFDh:1A0Eh`, `0x0F028`), the options come up (`11A0h:0451h`,
    /// `0x0F02E`), the machine resets (`0x0F033`) and no ticks are owed,
    /// `[1C76h] = 0` (`0x0F036`). Staying, `[1F48h]` zero (`0x0F03C`), the
    /// input is let go (`0EB9h:18A4h`, `0x0F045`), and the answer is 1
    /// (`0x0F048`).
    ///
    /// Answers whether the menu is up; what follows the call runs at its
    /// close, [`Adventure::options_gate_close`].
    fn gate_options(&mut self, arg: i16) -> bool {
        if arg != 0 {
            return false;
        }
        self.task_stop(0);
        if self.options_map.is_none() {
            self.events.push(Event::Unbuilt("11A0h:0451h", 0));
            return false;
        }
        self.options_open(Opened::Gate);
        true
    }

    /// The rest of the gate once the menu has closed, and the rest of the
    /// `0EB9h:0654h` that asked it, in the original's order: the machine
    /// reset, `[1C76h]` zero, the input let go when no exit was asked for;
    /// then the answer 1's clearing up (`0x0F209`); then the step's
    /// `[28C3h] = 0Ah` (`0x0F56D`).
    pub(crate) fn options_gate_close(&mut self) {
        self.mode_reset();
        // `0x0F036`: the ticks the menu was up for are dropped.
        self.clock.ticks = 0;
        if self.entry_word == 0 {
            self.input_unlocked();
        }
        self.bits &= KEEP;
        self.gate = Gate::None;
        self.pointer_resync();
        self.mode_reset();
        self.status_refresh_age = 0x0A;
    }

    /// A key, as the dispatcher's head and its event table take it.
    ///
    /// A text that is up takes any key as it takes a click (`0x0DFF7`) and
    /// a choice menu takes every key (`0x0E0F7`); both clear the event, so
    /// nothing after them sees it. A script that holds the input lets no
    /// key through (`0x0FB9E`), and an exit asked for ends the frame first
    /// (`0x0FB91`). Past those, `cs:1465h` (`0x0FCE6`) names five keys, of
    /// which Escape, `100h`, is read: [`Adventure::escape_arm`].
    pub(crate) fn key_event(&mut self, code: u16) {
        if matches!(self.tasks.t[0].kind, 3 | 4) {
            self.click_on_text_key();
            return;
        }
        if matches!(self.tasks.t[0].kind, 1 | 5) {
            return;
        }
        if self.leave.is_some() || self.entry_word != 0 || self.bits & LOCKED != 0 {
            return;
        }
        if code == KEY_ESCAPE {
            self.escape_arm();
        }
    }

    /// `0EB9h:1190h` (`0x0FD20`): Escape puts the options' gate up.
    ///
    /// The gate in place is asked with 1 (`0x0FD20`) and the input locked
    /// (`0EB9h:188Ah`, `0x0FD2F`). Then the verb is 6 (`0x0FD32`), the gate
    /// `0EB9h:0489h` (`0x0FD38`, `0x0FD3E`), the status age 3 (`0x0FD44`), the
    /// wait bits clear, `[289Bh]` masked with `FE0Fh` (`0x0FD4A`), and the mode
    /// 7 (`0x0FD50`); with the figure walking, `[16BBh]` not `FFFFh`
    /// (`0x0FD56`), it waits for the figure, bit `80h` (`0x0FD5D`).
    pub(crate) fn escape_arm(&mut self) {
        if self.ask_gate(1) == 0 {
            return;
        }
        self.input_locked();
        self.verb = 6;
        self.gate = Gate::Options;
        self.status_refresh_age = 3;
        self.bits &= KEEP;
        self.mode = 7;
        if self.player_node_id() == -1 {
            self.bits |= WAIT_FIGURE;
        }
    }
}
