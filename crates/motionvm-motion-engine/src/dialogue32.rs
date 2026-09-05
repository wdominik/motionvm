//! Conversations: the modes of the interaction machine from 12 to 18.
//!
//! A conversation is a record in a location's module — a text table, a set of
//! answers, and a graph of branch nodes — driven by the mode cell at
//! `_ORDER+0x0c`. `?DIALON` is exactly "that cell is between 12 and 18", and
//! while it is, the location's own task machine waits.
//!
//! The work is split the way the original splits it: one function per mode, and
//! `calc_dialog` (`0x7b49c`) as the switchboard the modes call back into. The
//! records are read byte-exactly rather than cell-wise — they sit 0x12 and 0x22
//! apart, so every other one starts mid-cell, and reading them as cells hands
//! back a neighbor instead. That is how a spoken line once turned up among the
//! answers.

use crate::menu;
use crate::order::M32_RULES;
use crate::order::block::{
    ANSWER_DESCS, ARROW, BAR_MENU, BAR_SCREEN, CAPTION, CHANGE_COUNT, CHANGE_QUEUE, CHANGE_ROOM,
    CHOSEN, FIELDS, GATE1, IMX, MLK, MMX, MMY, MODE, MRK, NODE, OBJECT, PICKED, PRESSED,
    QUIET_TABLE, QUIET_TEXT, RECORD, SCREEN, SLOT, SPEAKERS, TARGET, VERB,
};
use crate::{Address, Engine, Placement};
use motionvm_motion_forth::cell;
use motionvm_motion_forth::m32;
use motionvm_motion_forth::{Error, Result};

/// The four addresses a conversation is: its record, and the three arrays
/// behind it.
///
/// `_ORDER` carries only two of them — the record at `RECORD`, the field area
/// at `FIELDS` — and 0x7b4d1-0x7b50c derives the other two by walking the
/// counts in the record: the answers begin where the fields do, the lines
/// `record[+8]` answers on, the branches `record[+0xc]` lines after those.
/// Every mode that stands on a node needs all four, so they travel as one
/// rather than being derived again in each and handed on in threes and fours
/// of bare `u32`s that nothing tells apart.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Conversation {
    /// Entry node at +4, answer count at +8, line count at +0xc, the name at
    /// +0x24, and the per-answer permission bits from +0x30, four apart.
    record: u32,
    /// The answers, 0x12 bytes apart: +0 the node it says, +4 the next
    /// answer, +8 the name.
    answers: u32,
    /// The lines, 0x10 apart: +0 text, +4 table, +8 next, +0xc speaker.
    lines: u32,
    /// The branches, 0x22 apart: +0x16 the target, +0x1e the next node.
    branches: u32,
}

impl Conversation {
    /// Reads the four out of `_ORDER`, the way 0x7b4d1-0x7b50c does.
    fn of(vm: &m32::Vm, order: u32) -> Result<Self> {
        let record = vm.mem.fetch(Engine::field(order, RECORD))?;
        let answers = vm.mem.fetch(Engine::field(order, FIELDS))?;
        let lines = Engine::field(answers, Engine::cell(&vm.mem, record, 8)? * 0x12).0;
        let branches = Engine::field(lines, Engine::cell(&vm.mem, record, 0x0c)? * 0x10).0;
        Ok(Self {
            record,
            answers,
            lines,
            branches,
        })
    }
}

impl Engine {
    /// The deferred changes a location has queued for this conversation, 0x7b258.
    ///
    /// The queue hangs at `o[0x1c0]` with `o[0x1c8]` entries of 0x22 bytes: the
    /// conversation's name at +0, an answer's name at +9, and what to do at
    /// +0x12. An entry that matches both names is applied and then shifted out
    /// of the queue (0x7b399: move the rest down, `i--`, `o[0x1c8]--`), so each
    /// one fires once.
    ///
    /// ```text
    /// 1 → record[0x30 + j*4] |= 1     allow the answer
    /// 2 → record[0x30 + j*4] &= 0xfe  forbid it
    /// 3 → record[+4]    = answer[j]   set the entry node
    /// 4 → record[+0x20] = answer[j]
    /// ```
    pub(crate) fn dialogue_changes(
        &self,
        vm: &mut m32::Vm,
        order: u32,
        record: u32,
        fields: u32,
    ) -> Result<()> {
        let o = |off: u32| Self::field(order, off);
        if cell::signed(vm.mem.fetch(o(CHANGE_COUNT))?) <= 0 {
            return Ok(());
        }
        let queue = vm.mem.fetch(o(CHANGE_QUEUE))?;
        let answers = vm.mem.fetch(o(CHANGE_COUNT))?;
        let mut i = 0u32;
        let mut count = cell::signed(answers);
        while i < cell::unsigned(count) {
            let entry = Self::field(queue, i * 0x22).0;
            if !Self::same_name(vm, Self::field(record, 0x24).0, entry)? {
                i += 1;
                continue;
            }
            let options = Self::cell(&vm.mem, record, 8)?;
            for j in 0..options {
                let answer = Self::field(fields, j * 0x12).0;
                if !Self::same_name(vm, Self::field(answer, 8).0, Self::field(entry, 9).0)? {
                    continue;
                }
                let flag = Self::field(record, 0x30 + j * 4);
                let id = Self::cell(&vm.mem, answer, 0)?;
                match cell::signed(Self::cell(&vm.mem, entry, 0x12)?) {
                    1 => {
                        let v = vm.mem.fetch(flag)?;
                        vm.mem.store(flag, v | 1)?;
                    }
                    2 => {
                        let v = vm.mem.fetch(flag)?;
                        vm.mem.store(flag, v & 0xfe)?;
                    }
                    3 => vm.mem.store(Self::field(record, 4), id)?,
                    4 => vm.mem.store(Self::field(record, 0x20), id)?,
                    _ => {}
                }
            }
            // 0x7b399: the entry is used up; the rest of the queue moves down
            // over it and the same index is looked at again.
            for k in i..cell::unsigned(count) - 1 {
                for b in 0..0x22u32 {
                    let v = vm.mem.fetch_byte(Self::field(queue, (k + 1) * 0x22 + b))?;
                    vm.mem.store_byte(Self::field(queue, k * 0x22 + b), v)?;
                }
            }
            count -= 1;
            vm.mem.store(o(CHANGE_COUNT), cell::unsigned(count))?;
        }
        Ok(())
    }

    /// Modes 12 and 13: a line is on screen and the conversation is waiting for
    /// it. 0x7dd99 and 0x7df49 — the same routine twice, differing only in
    /// which speaker gets the calls: entry 0 for a line with nobody named, the
    /// line's own +0xc otherwise.
    ///
    /// ```text
    /// o[0x7c] ACTSCR   o[0x174] ACTDESC   GDACTIVE
    /// run out:            speaker(1);  CALCDIALOG mode 1
    /// else fresh press:   SDWAIT 0      \ the click cuts it short
    /// else:               speaker(2)
    /// then every other speaker with 4
    /// ```
    ///
    /// The click does not skip the line by itself — it sets the wait to zero,
    /// and the descriptor's own callback switches it off on the next frame.
    /// Same mechanism as `SETT1`, which is why it needed the `SDWAIT` callback
    /// before any of this could work.
    pub(crate) fn dialog_waiting(
        &mut self,
        vm: &mut m32::Vm,
        order: u32,
        named: bool,
    ) -> Result<bool> {
        let o = |off: u32| Self::field(order, off);
        let table = vm.mem.fetch(o(SPEAKERS))?;
        let count = Self::speakers(vm, table)?;
        let speaker = if named {
            let talk = Conversation::of(vm, order)?;
            let node = vm.mem.fetch(o(NODE))?;
            Self::cell(&vm.mem, talk.lines, node * 0x10 + 0x0c)?
        } else {
            0
        };

        self.select_screen(vm.mem.fetch(o(SCREEN))?);
        self.select_descriptor(vm.mem.fetch(o(ANSWER_DESCS))?);
        let showing = self.descriptor_active() != 0;

        let voice = Self::cell(&vm.mem, table, speaker * 0x28)?;
        if !showing {
            if voice != 0 {
                vm.data.push(1);
                vm.call_nested(Address::new(voice >> 16, voice & 0xffff), self)?;
            }
            self.calc_dialog(vm, order, 1)?;
        } else {
            let left = cell::signed(vm.mem.fetch(o(MLK))?);
            let right = cell::signed(vm.mem.fetch(o(MRK))?);
            let pressed = cell::signed(vm.mem.fetch(o(PRESSED))?);
            if (left != 0 || right != 0) && pressed == 0 {
                self.set_wait(0)?;
            } else if voice != 0 {
                vm.data.push(2);
                vm.call_nested(Address::new(voice >> 16, voice & 0xffff), self)?;
            }
        }

        // Everybody who is not speaking gets a 4 every frame. Mode 12 counts
        // from 1 (0x7dee2) because entry 0 is the one it just handled; mode 13
        // counts from 0 and skips the named speaker instead (0x7e163, 0x7e191)
        // — the same thing, since entry 0 is the unnamed speaker.
        self.speakers_idle(vm, table, count, Some(speaker))?;
        self.order_tail(vm, order)
    }

    /// Every speaker word but `except` hears 4: the idle call the machine
    /// makes each frame a conversation stands — 0x7dee2 and 0x7e163 under a
    /// line, 0x7e669 under the answers, where nobody is speaking and nobody
    /// is skipped.
    fn speakers_idle(
        &mut self,
        vm: &mut m32::Vm,
        table: u32,
        count: u32,
        except: Option<u32>,
    ) -> Result<()> {
        for i in (0..count).filter(|&i| except != Some(i)) {
            let word = Self::cell(&vm.mem, table, i * 0x28)?;
            if word != 0 {
                vm.data.push(4);
                vm.call_nested(Address::new(word >> 16, word & 0xffff), self)?;
            }
        }
        Ok(())
    }

    /// Mode 14, 0x7e1da: the answers are up and the pointer decides.
    ///
    /// A fresh left press with a known pointer (`o[0x8c] != -1`) is tested
    /// against each of the four descriptors in turn — `GDX`, `GDY` and the
    /// stored size give the box (0x7e309-0x7e367) — and the first one it lands
    /// in wins. All four then go away together (0x7e3ba).
    ///
    /// ```text
    /// the fourth → o[0x194] = record[+0x20]       \ say nothing
    /// otherwise  → o[0x194] = answer[o[0x1a4+i*4]][+0]
    ///              with bit 1 of the permission set, bit 0 is cleared
    ///              (0x7e4e1) — an answer that can be given once
    /// ```
    ///
    /// Then `o[0x120]`, and `CALCDIALOG` mode 0 shows the new node.
    ///
    /// Without a fresh left press in the scene, a fresh right press over the
    /// bar (0x7e52e) puts the item menu up instead, mode 17 — see
    /// [`Self::dialog_item_menu`]. Either way every speaker then hears 4
    /// (0x7e669).
    pub(crate) fn dialog_picking(&mut self, vm: &mut m32::Vm, order: u32) -> Result<bool> {
        let o = |off: u32| Self::field(order, off);
        let table = vm.mem.fetch(o(SPEAKERS))?;
        let count = Self::speakers(vm, table)?;
        let (left, right) = (
            cell::signed(vm.mem.fetch(o(MLK))?),
            cell::signed(vm.mem.fetch(o(MRK))?),
        );
        let fresh = cell::signed(vm.mem.fetch(o(PRESSED))?) == 0;
        let (px, py) = (
            cell::signed(vm.mem.fetch(o(MMX))?),
            cell::signed(vm.mem.fetch(o(MMY))?),
        );

        if left != 0 && fresh && px != -1 {
            self.pick_answer(vm, order, px, py)?;
        } else if right != 0 && fresh && cell::signed(vm.mem.fetch(o(IMX))?) != -1 {
            // 0x7e559: a slot with something in it. The mode goes up before
            // the strip does — `GMSHOWMENU` reads it (0x7a303) and adds the
            // look verb to every menu but this one's and the answers'.
            if let Some(slot) = menu::bar_slot(vm, order, M32_RULES)?
                && slot.item != 0
            {
                vm.mem.store(o(TARGET), cell::unsigned(slot.item))?;
                vm.mem.store(o(SLOT), cell::unsigned(slot.index))?;
                vm.mem.store(o(MODE), 0x11)?;
                let strip = menu::bar_strip(vm, order, slot, M32_RULES)?;
                menu::show_menu(self, vm, order, strip, 0x60, M32_RULES)?;
            }
        }
        self.speakers_idle(vm, table, count, None)?;
        self.order_tail(vm, order)
    }

    /// The left press of mode 14, 0x7e299 to 0x7e524: the first of the four
    /// boxes under the pointer wins, and all four go away together.
    fn pick_answer(&mut self, vm: &mut m32::Vm, order: u32, px: i32, py: i32) -> Result<()> {
        let o = |off: u32| Self::field(order, off);
        let slots = cell::signed(vm.mem.fetch(o(ANSWER_DESCS))?);
        self.select_screen(vm.mem.fetch(o(SCREEN))?);
        for i in 0..4i32 {
            self.select_descriptor(cell::unsigned(slots + i));
            if self.descriptor_active() == 0 {
                continue;
            }
            let x = self.descriptor_x();
            let y = self.descriptor_y();
            let x2 = x + self.descriptor_width();
            let y2 = y + self.descriptor_height();
            if px < x || py < y || px > x2 || py > y2 {
                continue;
            }

            for j in 0..4i32 {
                self.select_descriptor(cell::unsigned(slots + j));
                self.set_active(false);
            }

            let record = vm.mem.fetch(o(RECORD))?;
            if i == 3 {
                let quiet = Self::cell(&vm.mem, record, 0x20)?;
                vm.mem.store(o(NODE), quiet)?;
            } else {
                let answers = vm.mem.fetch(o(FIELDS))?;
                let chosen = vm.mem.fetch(o(CHOSEN + cell::unsigned(i) * 4))?;
                let node = Self::cell(&vm.mem, answers, chosen * 0x12)?;
                vm.mem.store(o(NODE), node)?;
                let flag = Self::field(record, 0x30 + chosen * 4);
                let v = vm.mem.fetch(flag)?;
                if v & 2 != 0 {
                    vm.mem.store(flag, v & 0xfe)?;
                }
            }
            self.cursor_state.visible = false;
            self.order_callback(vm, order, PICKED, &[])?;
            self.calc_dialog(vm, order, 0)?;
            break;
        }
        Ok(())
    }

    /// Mode 17, 0x7e918: the item menu stands over the bar, mid-conversation
    /// — the two verbs 6 and 7, `INFO` and `GIVE`, on the item the right
    /// click landed on.
    ///
    /// `HIGHLIGHTORDERS` every frame. A fresh left press is `CHOOSEORDERS`
    /// over those two, which sets the verb, takes the strip down and leaves
    /// mode 18; a fresh right press with the first gate clear takes the strip
    /// down and goes back to the answers, mode 14 (0x7e997).
    pub(crate) fn dialog_item_menu(&mut self, vm: &mut m32::Vm, order: u32) -> Result<bool> {
        let o = |off: u32| Self::field(order, off);
        let (screen, base) = (
            cell::signed(vm.mem.fetch(o(BAR_SCREEN))?),
            cell::signed(vm.mem.fetch(o(BAR_MENU))?),
        );
        menu::highlight(self, vm, order, screen, base)?;
        let fresh = cell::signed(vm.mem.fetch(o(PRESSED))?) == 0;
        if cell::signed(vm.mem.fetch(o(MLK))?) != 0 && fresh {
            menu::choose(self, vm, order, screen, base, 0x60, M32_RULES)?;
        }
        if cell::signed(vm.mem.fetch(o(MRK))?) != 0
            && fresh
            && cell::signed(vm.mem.fetch(o(GATE1))?) == 0
        {
            menu::remove_menu(self, vm, order)?;
            vm.mem.store(o(MODE), 14)?;
        }
        self.order_tail(vm, order)
    }

    /// Mode 18, 0x7e9b1: the pick waits for the figure — its command cell
    /// empty or at the arrived marker — and then runs as a forced order:
    /// mode 98, `EXECORDER` on the verb, the item and the flag, and the picked
    /// word after it. Mode 98 hands back to the answers when the order is
    /// done (0x7ea53).
    pub(crate) fn dialog_item_order(&mut self, vm: &mut m32::Vm, order: u32) -> Result<bool> {
        let o = |off: u32| Self::field(order, off);
        if menu::walk_done(vm, order)? {
            vm.mem.store(o(MODE), 98)?;
            let (verb, target, flag) = (
                cell::signed(vm.mem.fetch(o(VERB))?),
                cell::signed(vm.mem.fetch(o(TARGET))?),
                cell::signed(vm.mem.fetch(o(OBJECT))?),
            );
            self.exec_order(vm, order, verb, target, flag, M32_RULES)?;
            self.order_callback(vm, order, PICKED, &[])?;
        }
        self.order_tail(vm, order)
    }

    /// Mode 16, 0x7e8b9: the conversation is done with.
    ///
    /// The location's talk word is called once more with −1 — the same word
    /// `EXECORDER` asked for a line, now told there will not be another — the
    /// pointer comes back, and the block drops to mode 0 so `?DIALON` reports
    /// the conversation as over.
    pub(crate) fn dialog_over(&mut self, vm: &mut m32::Vm, order: u32) -> Result<bool> {
        self.order_callback(vm, order, 0x50, &[-1])?;
        self.cursor_state.visible = true;
        vm.mem.store(Self::field(order, 0x0c), 0)?;
        self.order_callback(vm, order, 0x124, &[])?;
        if Self::cell(&vm.mem, order, 0x1d8)? != 0 {
            self.order_callback(vm, order, 0x1d8, &[])?;
        }
        self.order_tail(vm, order)
    }

    /// `CALCDIALOG`, the native routine at 0x7b49c. Mode 1 steps to the next
    /// node first; both modes then show whatever node the block now stands on.
    ///
    /// The conversation is three arrays behind one record, laid out at
    /// 0x7b4d1-0x7b50c:
    ///
    /// ```text
    /// record[+4]    entry node          record[+8]    answer count
    /// record[+0xc]  line count          record[+0x24] name
    /// record[+0x30] permissions, 4 bytes per answer
    /// fields                     the answers, 0x12 each (+0 id, +8 name)
    /// + record[+8]*0x12          the lines,   0x10 each (+0 text, +4 table,
    ///                                                    +8 next, +0xc speaker)
    /// + record[+0xc]*0x10        the branches, 0x22 each (+0x1e next)
    /// ```
    ///
    /// The node lives in `o[0x194]`, and the number says what kind it is: −1 is
    /// over, under 1000 a spoken line, 1000 to 2000 an answer the player picks,
    /// 2000 to 3000 a branch. Those thresholds are 0x3e8/0x7d0/0xbb8 at
    /// 0x7b52a, 0x7b559 and 0x7b58c — they separate *kinds*, not values, which
    /// is the one thing to get wrong here.
    pub(crate) fn calc_dialog(&mut self, vm: &mut m32::Vm, order: u32, mode: i32) -> Result<()> {
        let o = |off: u32| Self::field(order, off);
        let talk = Conversation::of(vm, order)?;

        if mode == 1 {
            self.dialog_advance(vm, order, talk)?;
        }

        let node = cell::signed(vm.mem.fetch(o(NODE))?);
        if node == -1 {
            return self.dialog_finish(vm, order);
        }
        if node < 1000 {
            return self.dialog_speak(vm, order, talk, cell::unsigned(node));
        }
        if node < 2000 {
            return self.dialog_choose(vm, order, talk, cell::unsigned(node) - 1000);
        }
        if node < 3000 {
            return self.dialog_branch(vm, order, talk, cell::unsigned(node) - 2000);
        }
        Ok(())
    }

    /// The answer menu, 0x7b9fd to 0x7bd4c.
    ///
    /// Four descriptors are laid out from the bottom of the view upwards:
    /// `o[0x174]`, `+1` and `+2` carry up to three answers, `+3` the standing
    /// "say nothing" line out of `o[0x198]`/`o[0x19C]`. Which answers those are
    /// is decided by walking the chain from the node — each answer's `+4` names
    /// the next — and skipping any whose flag bit 0 is clear (0x7bbfd). Their
    /// indices are kept in `o[0x1a4]`, `+4` and `+8` so the click can find them
    /// again.
    ///
    /// The stacking is `SDOY` and then `GDCY SDVCEN`: place by the bottom edge,
    /// then re-center on it. Each line moves the next one up by its own height
    /// plus 0x11 (0x7bd34), starting 0x23 above the "say nothing" line, which
    /// itself sits at `GSCRY + 0x168`.
    ///
    /// Ends in mode 14, where `DOORDER` waits for the pointer.
    pub(crate) fn dialog_choose(
        &mut self,
        vm: &mut m32::Vm,
        order: u32,
        talk: Conversation,
        first: u32,
    ) -> Result<()> {
        let Conversation {
            record,
            answers,
            lines,
            ..
        } = talk;
        let o = |off: u32| Self::field(order, off);
        let voice = vm.mem.fetch(o(SPEAKERS))?;
        self.cursor_state.visible = true;
        self.order_callback(vm, order, 0x124, &[])?;
        let arrow = cell::signed(vm.mem.fetch(o(ARROW))?);
        self.order_callback(vm, order, 0xb8, &[arrow, 0, 0])?;

        self.select_screen(vm.mem.fetch(o(SCREEN))?);
        self.select_descriptor(vm.mem.fetch(o(CAPTION))?);
        self.set_text(1)?;
        let sx = self.screen_origin_x();
        let sy = self.screen_origin_y();
        let mut y = sy + 0x168;
        for i in 0..3 {
            vm.mem.store(o(CHOSEN + i * 4), 0)?;
        }

        let slots = cell::signed(vm.mem.fetch(o(ANSWER_DESCS))?);
        // The "say nothing" line first, at the bottom.
        self.select_descriptor(cell::unsigned(slots + 3));
        self.set_active(true);
        self.set_wait(-1)?;
        // Both fields are read before either is stored, as the original's own
        // pair of pushes does.
        let (text, table_id) = (
            cell::signed(vm.mem.fetch(o(QUIET_TEXT))?),
            cell::signed(vm.mem.fetch(o(QUIET_TABLE))?),
        );
        self.set_text(text)?;
        self.set_text_table(table_id)?;
        self.note_no_effect(crate::words::Word::SDNORM);
        self.place_x(sx + 0x140, Placement::Center)?;
        self.place_y(y, Placement::Center)?;
        y -= 0x23;
        // +4 is the color, +8 the template, in the speaker's own record.
        self.set_color(cell::signed(Self::cell(&vm.mem, voice, 4)?))?;
        self.set_template(cell::signed(Self::cell(&vm.mem, voice, 8)?))?;

        let mut n = cell::signed(first);
        for i in 0..3u32 {
            // 0x7bbfd: step over answers whose bit 0 is clear.
            while n != -1 && Self::cell(&vm.mem, record, 0x30 + cell::unsigned(n) * 4)? & 1 == 0 {
                n = cell::signed(Self::cell(&vm.mem, answers, cell::unsigned(n) * 0x12 + 4)?);
            }
            if n == -1 {
                break;
            }
            vm.mem.store(o(CHOSEN + i * 4), cell::unsigned(n))?;
            self.select_descriptor(cell::unsigned(slots + cell::signed(i)));
            self.set_active(true);

            // An answer names a line, and the line carries the words.
            let says = Self::cell(&vm.mem, answers, cell::unsigned(n) * 0x12)?;
            let line = Self::field(lines, says * 0x10).0;
            self.set_text_table(cell::signed(Self::cell(&vm.mem, line, 4)?))?;
            self.set_wait(-1)?;
            self.set_text(cell::signed(Self::cell(&vm.mem, line, 0)?))?;
            self.place_x(sx + 0x140, Placement::Center)?;
            // Placed by its bottom edge, then re-centered on where that put it —
            // the original reads the center straight back out and stores it, so
            // the descriptor ends up centered rather than bottom-anchored.
            self.place_y(y, Placement::FarEdge)?;
            let cy = self.descriptor_center_y();
            self.place_y(cy, Placement::Center)?;
            self.set_color(cell::signed(Self::cell(&vm.mem, voice, 4)?))?;
            self.set_template(cell::signed(Self::cell(&vm.mem, voice, 8)?))?;
            self.set_level(cell::signed(Self::cell(&vm.mem, voice, 0x14)?))?;
            y -= self.descriptor_height() + 0x11;
            n = cell::signed(Self::cell(&vm.mem, answers, cell::unsigned(n) * 0x12 + 4)?);
        }

        vm.mem.store(o(MODE), 14)
    }

    /// A branch node, 0x7bd5b to 0x7bf6b: a change rather than something said.
    ///
    /// With no name at +0 the change lands on this conversation at once — the
    /// same four operations the queue knows, plus two that run a word looked up
    /// by name (0x61596). With a name it is meant for a *different*
    /// conversation and is copied onto the queue at `o[0x1c0]` instead
    /// (0x7bf41), which is what `dialogue_changes` drains when that one starts.
    ///
    /// Either way the node is not a stop: it ends by stepping on (0x7bf6b).
    pub(crate) fn dialog_branch(
        &mut self,
        vm: &mut m32::Vm,
        order: u32,
        talk: Conversation,
        index: u32,
    ) -> Result<()> {
        let Conversation {
            record, branches, ..
        } = talk;
        let o = |off: u32| Self::field(order, off);
        let entry = Self::field(branches, index * 0x22).0;
        if vm.mem.fetch_byte(Self::field(entry, 0))? == 0 {
            let target = Self::cell(&vm.mem, entry, 0x16)?;
            let flag = Self::field(record, 0x30 + target * 4);
            match cell::signed(Self::cell(&vm.mem, entry, 0x12)?) {
                1 => {
                    let v = vm.mem.fetch(flag)?;
                    vm.mem.store(flag, v | 1)?;
                }
                2 => {
                    let v = vm.mem.fetch(flag)?;
                    vm.mem.store(flag, v & 0xfe)?;
                }
                3 => vm.mem.store(Self::field(record, 4), target)?,
                4 => vm.mem.store(Self::field(record, 0x20), target)?,
                // 5 and 6 run a word named at +9 rather than changing a field:
                // 0x61596 looks the name up across the loaded modules and the
                // handler builds its address out of the two globals the search
                // leaves behind (0x7be3f-0x7be55). Neither pushes anything
                // first, so the word takes no arguments.
                //
                // Only 5 checks the search (0x7be58); a name that is nowhere
                // reaches the diagnostic channel (0x1F40) and nothing runs.
                // 6 runs the address unchecked (0x7bece), so a miss calls
                // whatever word the previous hit left in the globals. It then
                // pops the offset the next step adds to the node and spends
                // (0x7bee3, read back at 0x7b644) — unconditionally, and the
                // pop helper 0x66c95 answers 0 when the stack has nothing.
                op @ (5 | 6) => {
                    let mut name = String::new();
                    for i in 0..9u32 {
                        match vm.mem.fetch_byte(Self::field(entry, 9 + i))? {
                            0 => break,
                            b => name.push(char::from(b)),
                        }
                    }
                    // In slot order, which is the order the original's own
                    // module table carries: `=>GET` takes the first free slot
                    // and `=>ERASE` frees one in place, so two modules that
                    // define one name are told apart by which was loaded
                    // into the earlier slot and not by which has the lower
                    // number.
                    let hit = vm.mem.lookup(&name, &self.resident());
                    if op == 5 {
                        if let Some(addr) = hit {
                            vm.call_nested(addr, self)?;
                        }
                    } else {
                        let Some(addr) = hit.or(vm.mem.last_hit) else {
                            return Err(Error::Unread {
                                what: "CALCDIALOG: action 6 with no word ever found — the \
                                       original would run on the null globals here \
                                       (module 0, cell 4)"
                                    .into(),
                                binary: "ENGINE.EXE",
                                at: "0x7bece",
                            });
                        };
                        vm.call_nested(addr, self)?;
                        self.dialogue.offset = vm.data.pop().unwrap_or(0);
                    }
                    // 0x7be80/0x7bee8: only these two actions drain the queue
                    // of deferred changes — 1 to 4 leave straight away
                    // (0x7bdaf and friends jump past 0x7b258).
                    let (first, second) = (vm.mem.fetch(o(RECORD))?, vm.mem.fetch(o(FIELDS))?);
                    self.dialogue_changes(vm, order, first, second)?;
                }
                _ => {}
            }
        } else {
            // 0x7bef8: named, so it belongs to another conversation — onto the
            // queue, if there is room (0x7bf1e guards it and complains).
            let queue = vm.mem.fetch(o(CHANGE_QUEUE))?;
            let (room, used) = (
                cell::signed(vm.mem.fetch(o(CHANGE_ROOM))?),
                cell::signed(vm.mem.fetch(o(CHANGE_COUNT))?),
            );
            if room > used {
                for b in 0..0x22u32 {
                    let v = vm.mem.fetch_byte(Self::field(entry, b))?;
                    vm.mem
                        .store_byte(Self::field(queue, cell::unsigned(used) * 0x22 + b), v)?;
                }
                vm.mem.store(o(CHANGE_COUNT), cell::unsigned(used) + 1)?;
            }
        }
        self.calc_dialog(vm, order, 1)
    }

    /// Mode 1, 0x7b527 to 0x7b6b4: which node comes after this one.
    ///
    /// Each kind keeps its successor in its own place — a line at +8, an answer
    /// at +0, a branch at +0x1e — and a branch's successor is numbered in a
    /// different scheme, so it is mapped back (0x7b5bc-0x7b644): 0…999 becomes
    /// a branch, 1000…1999 a line, 2000…2999 an answer.
    ///
    /// Two globals ride along. `0xdbd64` is an offset something else has queued
    /// and is consumed here; `0xdbd60` remembers the last answer node so that
    /// the node 4000 can mean "back to where the player was".
    pub(crate) fn dialog_advance(
        &mut self,
        vm: &mut m32::Vm,
        order: u32,
        talk: Conversation,
    ) -> Result<()> {
        let Conversation {
            answers,
            lines,
            branches,
            ..
        } = talk;
        let o = |off: u32| Self::field(order, off);
        let node = cell::signed(vm.mem.fetch(o(NODE))?);
        // Byte-exact throughout: an answer sits 0x12 apart and a branch 0x22,
        // so +0x1e never lands on a cell. Reading the branch successor with
        // `fetch` handed back `[.. .. ff ff]` = -65536 for a stored -1 — the
        // negative node that then walked the spoken-line path.
        let mut next = if node < 1000 {
            cell::signed(Self::cell(&vm.mem, lines, cell::unsigned(node) * 0x10 + 8)?)
        } else if node < 2000 {
            cell::signed(Self::cell(
                &vm.mem,
                answers,
                (cell::unsigned(node) - 1000) * 0x12,
            )?)
        } else if node < 3000 {
            let raw = cell::signed(Self::cell(
                &vm.mem,
                branches,
                (cell::unsigned(node) - 2000) * 0x22 + 0x1e,
            )?);
            match raw {
                0..=999 => raw + 2000,
                1000..=1999 => raw - 1000,
                2000..=2999 => raw - 1000,
                _ => raw,
            }
        } else {
            node
        };

        // 0x7b644: a queued offset, spent on the way past.
        if self.dialogue.offset != 0 {
            next += self.dialogue.offset;
            self.dialogue.offset = 0;
        }
        // 0x7b669: 4000 means "the answer the player was last on".
        if next == 4000 {
            next = self.dialogue.return_node;
        } else if (1000..2000).contains(&next) {
            self.dialogue.return_node = next;
        }
        vm.mem.store(o(NODE), cell::unsigned(next))?;
        Ok(())
    }

    /// A spoken line, 0x7b780 to 0x7b9fd.
    ///
    /// The line says which text to show and, at +0xc, who says it. That index
    /// picks an entry out of the speaker table at `o[0x16c]` — color, template,
    /// place and level — and decides the mode the conversation waits in: 13
    /// when somebody is named, 12 when nobody is.
    ///
    /// The clamp afterwards is `TSC`'s, done natively: twenty pixels of margin
    /// on a 640 by 400 view, re-centered through `GDCX`/`GDCY` each time it has
    /// to move. It only works because those getters answer with the drawn
    /// corner and the stored size.
    pub(crate) fn dialog_speak(
        &mut self,
        vm: &mut m32::Vm,
        order: u32,
        talk: Conversation,
        node: u32,
    ) -> Result<()> {
        let o = |off: u32| Self::field(order, off);
        let line = Self::field(talk.lines, node * 0x10).0;
        let speaker = Self::cell(&vm.mem, line, 0x0c)?;
        let table = vm.mem.fetch(o(SPEAKERS))?;
        let voice = Self::field(table, speaker * 0x28).0;
        vm.mem.store(o(MODE), if speaker != 0 { 13 } else { 12 })?;

        self.select_descriptor(vm.mem.fetch(o(ANSWER_DESCS))?);
        self.set_active(true);
        // Seven fields in the order the handler stores them: the line's table
        // and text, the speaker's color, template, center and level.
        self.set_text_table(cell::signed(Self::cell(&vm.mem, line, 4)?))?;
        self.set_color(cell::signed(Self::cell(&vm.mem, voice, 4)?))?;
        self.set_template(cell::signed(Self::cell(&vm.mem, voice, 8)?))?;
        self.set_text(cell::signed(Self::cell(&vm.mem, line, 0)?))?;
        self.place_x(
            cell::signed(Self::cell(&vm.mem, voice, 0x0c)?),
            Placement::Center,
        )?;
        self.place_y(
            cell::signed(Self::cell(&vm.mem, voice, 0x10)?),
            Placement::Center,
        )?;
        self.set_level(cell::signed(Self::cell(&vm.mem, voice, 0x14)?))?;

        let sx = self.screen_origin_x();
        let sy = self.screen_origin_y();
        // 0x7b8d0 onwards. 0x14 is the margin, 0x280 and 0x190 the view. Each
        // move re-centers through the matching `GD…` so the descriptor keeps
        // the centered placement it came in with.
        // One axis at a time, and the two are the same routine. Six word names
        // collapse to three getters and one setter, because `SDX`, `SDOX` and
        // `SDCEN` are one store with three [`Placement`]s — which is how the
        // original encodes them too.
        type Get = fn(&mut Engine) -> i32;
        type Place = fn(&mut Engine, i32, Placement) -> Result<()>;
        let axes: [(Get, Get, Get, Place, i32, i32); 2] = [
            (
                Engine::descriptor_x,
                Engine::descriptor_far_x,
                Engine::descriptor_center_x,
                Engine::place_x,
                sx,
                0x280,
            ),
            (
                Engine::descriptor_y,
                Engine::descriptor_far_y,
                Engine::descriptor_center_y,
                Engine::place_y,
                sy,
                0x190,
            ),
        ];
        for (near, far, center, place, origin, span) in axes {
            if near(self) - 0x14 < origin {
                place(self, origin + 0x14, Placement::Edge)?;
                let c = center(self);
                place(self, c, Placement::Center)?;
            }
            if far(self) + 0x14 > origin + span {
                place(self, origin + span - 0x14, Placement::FarEdge)?;
                let c = center(self);
                place(self, c, Placement::Center)?;
            }
        }

        // 0x7b9ea: the location's own word for "a line just went up".
        self.order_callback(vm, order, 0x1a0, &[])
    }

    /// The end of a conversation, 0x7b6c4: every speaker gets a last call with
    /// a 3, and the block goes into mode 16 for `DOORDER` to finish off.
    pub(crate) fn dialog_finish(&mut self, vm: &mut m32::Vm, order: u32) -> Result<()> {
        let table = Self::cell(&vm.mem, order, 0x16c)?;
        for i in 0..Self::speakers(vm, table)? {
            let word = Self::cell(&vm.mem, table, i * 0x28)?;
            if word != 0 {
                vm.data.push(3);
                vm.call_nested(Address::new(word >> 16, word & 0xffff), self)?;
            }
        }
        vm.mem.store(Self::field(order, 0x0c), 0x10)
    }

    /// How many speakers the table holds. Ten is only where the search stops
    /// (0x7b6e8); what ends it is an entry whose +4 is −1.
    pub(crate) fn speakers(vm: &m32::Vm, table: u32) -> Result<u32> {
        for i in 0..10u32 {
            if cell::signed(Self::cell(&vm.mem, table, i * 0x28 + 4)?) == -1 {
                return Ok(i);
            }
        }
        Ok(10)
    }
}
