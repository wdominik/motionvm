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

use crate::order::block::*;
use crate::{Address, Engine, Error, Placement, Result, Vm};

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
        &mut self,
        vm: &mut Vm,
        order: u32,
        record: u32,
        fields: u32,
    ) -> Result<()> {
        let o = |off: u32| Self::field(order, off);
        if vm.mem.fetch(o(CHANGE_COUNT))? as i32 <= 0 {
            return Ok(());
        }
        let queue = vm.mem.fetch(o(CHANGE_QUEUE))?;
        let answers = vm.mem.fetch(o(CHANGE_COUNT))?;
        let mut i = 0u32;
        let mut count = answers as i32;
        while i < count as u32 {
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
                match Self::cell(&vm.mem, entry, 0x12)? as i32 {
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
            for k in i..count as u32 - 1 {
                for b in 0..0x22u32 {
                    let v = vm.mem.fetch_byte(Self::field(queue, (k + 1) * 0x22 + b))?;
                    vm.mem.store_byte(Self::field(queue, k * 0x22 + b), v)?;
                }
            }
            count -= 1;
            vm.mem.store(o(CHANGE_COUNT), count as u32)?;
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
    pub(crate) fn dialog_waiting(&mut self, vm: &mut Vm, order: u32, named: bool) -> Result<bool> {
        let o = |off: u32| Self::field(order, off);
        let table = vm.mem.fetch(o(SPEAKERS))?;
        let count = Self::speakers(vm, table)?;
        let speaker = if named {
            let record = vm.mem.fetch(o(RECORD))?;
            let fields = vm.mem.fetch(o(FIELDS))?;
            let lines = Self::field(fields, Self::cell(&vm.mem, record, 8)? * 0x12).0;
            let node = vm.mem.fetch(o(NODE))?;
            Self::cell(&vm.mem, lines, node * 0x10 + 0x0c)?
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
            let left = vm.mem.fetch(o(MLK))? as i32;
            let right = vm.mem.fetch(o(MRK))? as i32;
            let pressed = vm.mem.fetch(o(PRESSED))? as i32;
            if (left != 0 || right != 0) && pressed == 0 {
                self.set_wait(0)?;
            } else if voice != 0 {
                vm.data.push(2);
                vm.call_nested(Address::new(voice >> 16, voice & 0xffff), self)?;
            }
        }

        // Everybody who is not speaking gets a 4 every frame. Mode 12 counts
        // from 1 (0x7dee2) because entry 0 is the one it just handled; mode 13
        // counts from 0 and skips the named speaker instead (0x7e163, 0x7e191).
        let first = if named { 0 } else { 1 };
        for i in first..count {
            if named && i == speaker {
                continue;
            }
            let word = Self::cell(&vm.mem, table, i * 0x28)?;
            if word != 0 {
                vm.data.push(4);
                vm.call_nested(Address::new(word >> 16, word & 0xffff), self)?;
            }
        }
        self.order_tail(vm, order)
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
    /// The right click on the inventory bar (0x7e52e) opens the verb menu
    /// mid-conversation through `GMSHOWMENU`. That path is unread, and says so
    /// when it is taken.
    pub(crate) fn dialog_picking(&mut self, vm: &mut Vm, order: u32) -> Result<bool> {
        let o = |off: u32| Self::field(order, off);
        let (left, right) = (vm.mem.fetch(o(MLK))? as i32, vm.mem.fetch(o(MRK))? as i32);
        let pressed = vm.mem.fetch(o(PRESSED))? as i32;
        let (px, py) = (vm.mem.fetch(o(MMX))? as i32, vm.mem.fetch(o(MMY))? as i32);

        if right != 0 && pressed == 0 && vm.mem.fetch(o(IMX))? as i32 != -1 {
            let ix = vm.mem.fetch(o(IMX))? as i32;
            if (0x40..0x240).contains(&ix) {
                return Err(Error::Unread {
                    what: "DOORDER: the verb menu over the inventory during a conversation".into(),
                    at: "0x7e559, GMSHOWMENU",
                });
            }
        }
        if left == 0 || pressed != 0 || px == -1 {
            return self.order_tail(vm, order);
        }

        let slots = vm.mem.fetch(o(ANSWER_DESCS))? as i32;
        self.select_screen(vm.mem.fetch(o(SCREEN))?);
        for i in 0..4i32 {
            self.select_descriptor((slots + i) as u32);
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
                self.select_descriptor((slots + j) as u32);
                self.set_active(false);
            }

            let record = vm.mem.fetch(o(RECORD))?;
            if i == 3 {
                let quiet = Self::cell(&vm.mem, record, 0x20)?;
                vm.mem.store(o(NODE), quiet)?;
            } else {
                let answers = vm.mem.fetch(o(FIELDS))?;
                let chosen = vm.mem.fetch(o(CHOSEN + i as u32 * 4))?;
                let node = Self::cell(&vm.mem, answers, chosen * 0x12)?;
                vm.mem.store(o(NODE), node)?;
                let flag = Self::field(record, 0x30 + chosen * 4);
                let v = vm.mem.fetch(flag)?;
                if v & 2 != 0 {
                    vm.mem.store(flag, v & 0xfe)?;
                }
            }
            self.pointer_visible = false;
            self.order_callback(vm, order, 0x120, &[])?;
            self.calc_dialog(vm, order, 0)?;
            break;
        }
        self.order_tail(vm, order)
    }

    /// Mode 16, 0x7e8b9: the conversation is done with.
    ///
    /// The location's talk word is called once more with −1 — the same word
    /// `EXECORDER` asked for a line, now told there will not be another — the
    /// pointer comes back, and the block drops to mode 0 so `?DIALON` reports
    /// the conversation as over.
    pub(crate) fn dialog_over(&mut self, vm: &mut Vm, order: u32) -> Result<bool> {
        self.order_callback(vm, order, 0x50, &[-1])?;
        self.pointer_visible = true;
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
    pub(crate) fn calc_dialog(&mut self, vm: &mut Vm, order: u32, mode: i32) -> Result<()> {
        let o = |off: u32| Self::field(order, off);
        let record = vm.mem.fetch(o(RECORD))?;
        let fields = vm.mem.fetch(o(FIELDS))?;
        let answers = fields;
        let lines = Self::field(fields, Self::cell(&vm.mem, record, 8)? * 0x12).0;
        let branches = Self::field(lines, Self::cell(&vm.mem, record, 0x0c)? * 0x10).0;

        if mode == 1 {
            self.dialog_advance(vm, order, answers, lines, branches)?;
        }

        let node = vm.mem.fetch(o(NODE))? as i32;
        if node == -1 {
            return self.dialog_finish(vm, order);
        }
        if node < 1000 {
            return self.dialog_speak(vm, order, lines, node as u32);
        }
        if node < 2000 {
            return self.dialog_choose(vm, order, record, answers, lines, node as u32 - 1000);
        }
        if node < 3000 {
            return self.dialog_branch(vm, order, record, answers, branches, node as u32 - 2000);
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
    // The arguments are the handler's own: `order`, `record`, `answers`,
    // `lines` and `first` are the five cells it is called with. Grouping them
    // would hide which five.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dialog_choose(
        &mut self,
        vm: &mut Vm,
        order: u32,
        record: u32,
        answers: u32,
        lines: u32,
        first: u32,
    ) -> Result<()> {
        let o = |off: u32| Self::field(order, off);
        let voice = vm.mem.fetch(o(SPEAKERS))?;
        self.pointer_visible = true;
        self.order_callback(vm, order, 0x124, &[])?;
        let arrow = vm.mem.fetch(o(ARROW))? as i32;
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

        let slots = vm.mem.fetch(o(ANSWER_DESCS))? as i32;
        // The "say nothing" line first, at the bottom.
        self.select_descriptor((slots + 3) as u32);
        self.set_active(true);
        self.set_wait(-1)?;
        // Both fields are read before either is stored, as the original's own
        // pair of pushes does.
        let (text, table_id) = (
            vm.mem.fetch(o(QUIET_TEXT))? as i32,
            vm.mem.fetch(o(QUIET_TABLE))? as i32,
        );
        self.set_text(text)?;
        self.set_text_table(table_id)?;
        self.note_no_effect("SDNORM");
        self.place_x(sx + 0x140, Placement::Center)?;
        self.place_y(y, Placement::Center)?;
        y -= 0x23;
        // +4 is the color, +8 the template, in the speaker's own record.
        self.set_color(Self::cell(&vm.mem, voice, 4)? as i32)?;
        self.set_template(Self::cell(&vm.mem, voice, 8)? as i32)?;

        let mut n = first as i32;
        for i in 0..3u32 {
            // 0x7bbfd: step over answers whose bit 0 is clear.
            while n != -1 && Self::cell(&vm.mem, record, 0x30 + n as u32 * 4)? & 1 == 0 {
                n = Self::cell(&vm.mem, answers, n as u32 * 0x12 + 4)? as i32;
            }
            if n == -1 {
                break;
            }
            vm.mem.store(o(CHOSEN + i * 4), n as u32)?;
            self.select_descriptor((slots + i as i32) as u32);
            self.set_active(true);

            // An answer names a line, and the line carries the words.
            let says = Self::cell(&vm.mem, answers, n as u32 * 0x12)?;
            let line = Self::field(lines, says * 0x10).0;
            self.set_text_table(Self::cell(&vm.mem, line, 4)? as i32)?;
            self.set_wait(-1)?;
            self.set_text(Self::cell(&vm.mem, line, 0)? as i32)?;
            self.place_x(sx + 0x140, Placement::Center)?;
            // Placed by its bottom edge, then re-centered on where that put it —
            // the original reads the center straight back out and stores it, so
            // the descriptor ends up centered rather than bottom-anchored.
            self.place_y(y, Placement::FarEdge)?;
            let cy = self.descriptor_center_y();
            self.place_y(cy, Placement::Center)?;
            self.set_color(Self::cell(&vm.mem, voice, 4)? as i32)?;
            self.set_template(Self::cell(&vm.mem, voice, 8)? as i32)?;
            self.set_level(Self::cell(&vm.mem, voice, 0x14)? as i32)?;
            y -= self.descriptor_height() + 0x11;
            n = Self::cell(&vm.mem, answers, n as u32 * 0x12 + 4)? as i32;
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
    // Same argument list as [`Self::dialog_choose`], deliberately — the two
    // are called from the same dispatch with the same five cells, and one of
    // them ignoring `answers` is a fact about the handler, not a reason to
    // give the pair different shapes.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dialog_branch(
        &mut self,
        vm: &mut Vm,
        order: u32,
        record: u32,
        _answers: u32,
        branches: u32,
        index: u32,
    ) -> Result<()> {
        let o = |off: u32| Self::field(order, off);
        let entry = Self::field(branches, index * 0x22).0;
        if vm.mem.fetch_byte(Self::field(entry, 0))? == 0 {
            let target = Self::cell(&vm.mem, entry, 0x16)?;
            let flag = Self::field(record, 0x30 + target * 4);
            match Self::cell(&vm.mem, entry, 0x12)? as i32 {
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
                            b => name.push(b as char),
                        }
                    }
                    let hit = vm.mem.lookup(&name);
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
                                at: "0x7bece",
                            });
                        };
                        vm.call_nested(addr, self)?;
                        self.dialog_offset = vm.data.pop().unwrap_or(0);
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
                vm.mem.fetch(o(CHANGE_ROOM))? as i32,
                vm.mem.fetch(o(CHANGE_COUNT))? as i32,
            );
            if room > used {
                for b in 0..0x22u32 {
                    let v = vm.mem.fetch_byte(Self::field(entry, b))?;
                    vm.mem
                        .store_byte(Self::field(queue, used as u32 * 0x22 + b), v)?;
                }
                vm.mem.store(o(CHANGE_COUNT), used as u32 + 1)?;
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
        vm: &mut Vm,
        order: u32,
        answers: u32,
        lines: u32,
        branches: u32,
    ) -> Result<()> {
        let o = |off: u32| Self::field(order, off);
        let node = vm.mem.fetch(o(NODE))? as i32;
        // Byte-exact throughout: an answer sits 0x12 apart and a branch 0x22,
        // so +0x1e never lands on a cell. Reading the branch successor with
        // `fetch` handed back `[.. .. ff ff]` = -65536 for a stored -1 — the
        // negative node that then walked the spoken-line path.
        let mut next = if node < 1000 {
            Self::cell(&vm.mem, lines, node as u32 * 0x10 + 8)? as i32
        } else if node < 2000 {
            Self::cell(&vm.mem, answers, (node as u32 - 1000) * 0x12)? as i32
        } else if node < 3000 {
            let raw = Self::cell(&vm.mem, branches, (node as u32 - 2000) * 0x22 + 0x1e)? as i32;
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
        if self.dialog_offset != 0 {
            next += self.dialog_offset;
            self.dialog_offset = 0;
        }
        // 0x7b669: 4000 means "the answer the player was last on".
        if next == 4000 {
            next = self.dialog_return;
        } else if (1000..2000).contains(&next) {
            self.dialog_return = next;
        }
        vm.mem.store(o(NODE), next as u32)?;
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
        vm: &mut Vm,
        order: u32,
        lines: u32,
        node: u32,
    ) -> Result<()> {
        let o = |off: u32| Self::field(order, off);
        let line = Self::field(lines, node * 0x10).0;
        let speaker = Self::cell(&vm.mem, line, 0x0c)?;
        let table = vm.mem.fetch(o(SPEAKERS))?;
        let voice = Self::field(table, speaker * 0x28).0;
        vm.mem.store(o(MODE), if speaker != 0 { 13 } else { 12 })?;

        self.select_descriptor(vm.mem.fetch(o(ANSWER_DESCS))?);
        self.set_active(true);
        // Seven fields in the order the handler stores them: the line's table
        // and text, the speaker's color, template, center and level.
        self.set_text_table(Self::cell(&vm.mem, line, 4)? as i32)?;
        self.set_color(Self::cell(&vm.mem, voice, 4)? as i32)?;
        self.set_template(Self::cell(&vm.mem, voice, 8)? as i32)?;
        self.set_text(Self::cell(&vm.mem, line, 0)? as i32)?;
        self.place_x(Self::cell(&vm.mem, voice, 0x0c)? as i32, Placement::Center)?;
        self.place_y(Self::cell(&vm.mem, voice, 0x10)? as i32, Placement::Center)?;
        self.set_level(Self::cell(&vm.mem, voice, 0x14)? as i32)?;

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
        for (near, far, center, place, origin, span) in [
            (
                Engine::descriptor_x as Get,
                Engine::descriptor_far_x as Get,
                Engine::descriptor_center_x as Get,
                Engine::place_x as Place,
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
        ] {
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
    pub(crate) fn dialog_finish(&mut self, vm: &mut Vm, order: u32) -> Result<()> {
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
    pub(crate) fn speakers(vm: &Vm, table: u32) -> Result<u32> {
        for i in 0..10u32 {
            if Self::cell(&vm.mem, table, i * 0x28 + 4)? as i32 == -1 {
                return Ok(i);
            }
        }
        Ok(10)
    }
}
