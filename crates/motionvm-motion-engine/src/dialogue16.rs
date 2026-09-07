//! Conversations on the 16-bit machine: the modes of the interaction machine
//! from 12 to 18, as `ENVIRO.EXE` has them.
//!
//! The same design as the 32-bit engine's ([`crate::dialogue32`]) — a record
//! in a location's module with a table of answers, a table of lines and a
//! graph of branch nodes, driven by the mode cell of the `_ORDER` block and
//! a node number whose range says what kind of node it is — but an older
//! layout and a different screen: two speaker words instead of a speaker
//! table, two talking-head descriptors, the answers stacked downward on the
//! left, colors and templates out of the block. So this is the 16-bit
//! reading on its own, not the 32-bit code with rules. Read at
//! `calc_dialog` (`0d34:0ed9`), the change drain (`0d34:0d42`), the finish
//! (`0d34:0d04`), the `DOORDER` modes (`0d34:2d16`–`0d34:33c0`), verb 5
//! (`0d34:1b2a`); the file offset of `seg:off` is `0x3200 + seg × 16 + off`.
//!
//! ```text
//! record   +2 entry node   +4 answers   +6 lines   +0xa/+0xc head sprites
//!          +0x10 quiet node   +0x12 name   +0x1c permissions, a cell each
//! answer   14 bytes: +0 node, +2 next answer, +4 name
//! line     8 bytes:  +0 text, +2 table, +4 next, +6 flags (bit 0: right)
//! branch   26 bytes: +0 name, +9 answer name, +0x12 kind, +0x14 target,
//!          +0x18 next
//! ```
//!
//! Offsets into the `_ORDER` block are the 16-bit block's own — the 32-bit
//! offsets halved, named in [`crate::order::block`] for that machine.

use crate::menu;
use crate::order::{Conversation, M16_RULES};
use crate::{Engine, Placement};
use motionvm_motion_forth::Machine;
use motionvm_motion_forth::cell;
use motionvm_motion_forth::m16;
use motionvm_motion_forth::{Error, Result};

/// The block's conversation fields, 16-bit offsets.
mod g {
    pub(super) const VERB: i32 = 0x00;
    pub(super) const TARGET: i32 = 0x02;
    pub(super) const OBJECT: i32 = 0x04;
    pub(super) const MODE: i32 = 0x06;
    pub(super) const BAR_MENU: i32 = 0x0a;
    /// Verb 5's handler word, `VERBS + 4 × 6 + 4`.
    pub(super) const TALK: i32 = 0x28;
    pub(super) const SCREEN: i32 = 0x3e;
    pub(super) const BAR_SCREEN: i32 = 0x40;
    pub(super) const MMX: i32 = 0x46;
    pub(super) const MMY: i32 = 0x48;
    pub(super) const IMX: i32 = 0x4a;
    pub(super) const MLK: i32 = 0x4e;
    pub(super) const MRK: i32 = 0x50;
    pub(super) const PRESSED: i32 = 0x52;
    pub(super) const SET_CURSOR: i32 = 0x5c;
    pub(super) const ARROW: i32 = 0x5e;
    pub(super) const SLOT: i32 = 0x78;
    pub(super) const CAPTION: i32 = 0x80;
    pub(super) const PICKED: i32 = 0x90;
    pub(super) const FINISHED: i32 = 0x92;
    pub(super) const GATE1: i32 = 0x9a;
    pub(super) const INFO_DESC: i32 = 0xa0;
    /// The two talking heads, left and right.
    pub(super) const HEAD_L: i32 = 0xb6;
    pub(super) const HEAD_R: i32 = 0xb8;
    /// The first of four answer descriptors; the fourth is the quiet line.
    pub(super) const ANSWER_DESCS: i32 = 0xba;
    pub(super) const RECORD: i32 = 0xbe;
    pub(super) const FIELDS: i32 = 0xc0;
    /// The left speaker's color, the answers' color, the right speaker's.
    pub(super) const LEFT_COL: i32 = 0xc2;
    pub(super) const NORMAL_COL: i32 = 0xc4;
    pub(super) const RIGHT_COL: i32 = 0xc6;
    pub(super) const NODE: i32 = 0xca;
    pub(super) const QUIET_TABLE: i32 = 0xcc;
    pub(super) const QUIET_TEXT: i32 = 0xce;
    /// Run after a line went up.
    pub(super) const SAID: i32 = 0xd0;
    /// Three cells: which answer each of the three slots shows.
    pub(super) const CHOSEN: i32 = 0xd2;
    /// The two speaker words, run with a phase: 0 start, 1 line over, 2
    /// still talking, 3 end, 4 listening.
    pub(super) const SPEAK_L: i32 = 0xdc;
    pub(super) const SPEAK_R: i32 = 0xde;
    pub(super) const CHANGE_QUEUE: i32 = 0xe0;
    pub(super) const CHANGE_COUNT: i32 = 0xe4;
    /// The left speaker's template, the right speaker's.
    pub(super) const LEFT_FONT: i32 = 0xe8;
    pub(super) const RIGHT_FONT: i32 = 0xea;
    /// Run when a conversation is over, if set.
    pub(super) const EXIT: i32 = 0xec;
    pub(super) const INFO_COLOR: i32 = 0xf0;
    pub(super) const INFO_FONT: i32 = 0xf2;
}

const ANSWER: i32 = 14;
const LINE: i32 = 8;
const BRANCH: i32 = 26;
/// The same, as the byte count `read_bytes` takes.
const BRANCH_BYTES: usize = 26;

/// The conversation's three tables, resolved once.
struct Tables {
    record: i32,
    answers: i32,
    lines: i32,
    branches: i32,
}

fn fetch(vm: &m16::Vm, at: i32) -> Result<i32> {
    vm.space().fetch_cell(at)
}

fn store(vm: &mut m16::Vm, at: i32, v: i32) -> Result<()> {
    vm.space_mut().store_cell(at, v)
}

fn field(vm: &m16::Vm, base: i32, off: i32) -> i32 {
    vm.space().offset(base, off)
}

fn block(vm: &m16::Vm, order: u32, off: i32) -> Result<i32> {
    fetch(vm, field(vm, cell::signed(order), off))
}

fn set_block(vm: &mut m16::Vm, order: u32, off: i32, v: i32) -> Result<()> {
    let at = field(vm, cell::signed(order), off);
    store(vm, at, v)
}

/// The NUL-terminated name at an address, as `1251:000f` compares one.
fn name_at(vm: &m16::Vm, at: i32) -> Result<Vec<u8>> {
    let mem = vm.space();
    let mut name = Vec::new();
    for i in 0..64 {
        match mem.fetch_byte(mem.offset(at, i))? {
            0 => break,
            b => name.push(b),
        }
    }
    Ok(name)
}

fn tables(vm: &m16::Vm, order: u32) -> Result<Tables> {
    let record = block(vm, order, g::RECORD)?;
    let answers = block(vm, order, g::FIELDS)?;
    let lines = field(vm, answers, fetch(vm, field(vm, record, 4))? * ANSWER);
    let branches = field(vm, lines, fetch(vm, field(vm, record, 6))? * LINE);
    Ok(Tables {
        record,
        answers,
        lines,
        branches,
    })
}

impl Engine {
    /// Runs the word whose id the block holds at `off`, with `args` — if the
    /// cell is not zero, which is how every caller guards it.
    fn run16(&mut self, vm: &mut m16::Vm, order: u32, off: i32, args: &[i32]) -> Result<()> {
        let id = block(vm, order, off)?;
        if id == 0 {
            return Ok(());
        }
        let Some(target) = vm.callback_target(id) else {
            return Err(Error::UnboundWord {
                id: cell::low16(id),
                at: vm.here(),
            });
        };
        vm.data.extend_from_slice(args);
        vm.call_nested(target, self)
    }

    /// The change drain, `0d34:0d42`: every queued change whose names match
    /// this conversation and one of its answers is applied and taken off
    /// the queue.
    fn changes16(&self, vm: &mut m16::Vm, order: u32, record: i32, answers: i32) -> Result<()> {
        let queue = block(vm, order, g::CHANGE_QUEUE)?;
        let perms = field(vm, record, 0x1c);
        let mine = name_at(vm, field(vm, record, 0x12))?;
        let mut i = 0;
        while i < block(vm, order, g::CHANGE_COUNT)? {
            let entry = field(vm, queue, i * BRANCH);
            if name_at(vm, entry)? != mine {
                i += 1;
                continue;
            }
            let count = fetch(vm, field(vm, record, 4))?;
            for j in 0..count {
                let answer = field(vm, answers, j * ANSWER);
                if name_at(vm, field(vm, answer, 4))? != name_at(vm, field(vm, entry, 9))? {
                    continue;
                }
                let flag = field(vm, perms, j * 2);
                let node = fetch(vm, answer)?;
                match fetch(vm, field(vm, entry, 0x12))? {
                    1 => {
                        let v = fetch(vm, flag)?;
                        store(vm, flag, v | 1)?;
                    }
                    2 => {
                        let v = fetch(vm, flag)?;
                        store(vm, flag, v & 0xfe)?;
                    }
                    3 => store(vm, field(vm, record, 2), node)?,
                    4 => store(vm, field(vm, record, 0x10), node)?,
                    _ => {}
                }
            }
            // Used up: the rest of the queue moves down over it and the same
            // index is looked at again (`0d34:0e8a`–`0d34:0ec0`).
            let count = block(vm, order, g::CHANGE_COUNT)?;
            let rest = vm.space().read_bytes(
                field(vm, queue, (i + 1) * BRANCH),
                // Nothing to move once the count has gone below the index.
                cell::at((count - i) * BRANCH).unwrap_or(0),
            )?;
            let at = field(vm, queue, i * BRANCH);
            vm.space_mut().write_bytes(at, &rest)?;
            set_block(vm, order, g::CHANGE_COUNT, count - 1)?;
        }
        Ok(())
    }

    /// `calc_dialog`, `0d34:0ed9`. Mode 1 steps to the next node first; both
    /// modes then show whatever node the block now stands on.
    fn calc16(&mut self, vm: &mut m16::Vm, order: u32, mode: i32) -> Result<()> {
        let t = tables(vm, order)?;
        if mode == 1 {
            self.advance16(vm, order, &t)?;
        }
        let node = block(vm, order, g::NODE)?;
        if node == -1 {
            // `0d34:1068`: both speakers hear 3, the heads go, mode 16.
            self.run16(vm, order, g::SPEAK_L, &[3])?;
            self.run16(vm, order, g::SPEAK_R, &[3])?;
            return self.finish16(vm, order);
        }
        if node < 1000 {
            return self.speak16(vm, order, &t, node);
        }
        if node < 2000 {
            return self.choose16(vm, order, &t, node - 1000);
        }
        if node < 3000 {
            return self.branch16(vm, order, &t, node - 2000);
        }
        Ok(())
    }

    /// `0d34:0f6a`–`0d34:1062`: which node comes after this one. A branch's
    /// successor is numbered in another scheme and mapped back; 4000 means
    /// the answer the player was last on, kept in one global (`DS:0x107a`).
    fn advance16(&mut self, vm: &mut m16::Vm, order: u32, t: &Tables) -> Result<()> {
        let node = block(vm, order, g::NODE)?;
        let mut next = if node < 1000 {
            fetch(vm, field(vm, t.lines, node * LINE + 4))?
        } else if node < 2000 {
            fetch(vm, field(vm, t.answers, (node - 1000) * ANSWER))?
        } else if node < 3000 {
            let raw = fetch(vm, field(vm, t.branches, (node - 2000) * BRANCH + 0x18))?;
            match raw {
                0..=999 => raw + 2000,
                1000..=2999 => raw - 1000,
                _ => raw,
            }
        } else {
            node
        };
        if next == 4000 {
            next = self.dialogue.return_node;
        } else if (1000..2000).contains(&next) {
            self.dialogue.return_node = next;
        }
        set_block(vm, order, g::NODE, next)
    }

    /// `0d34:0d04`: the talking heads go, and mode 16 ends the conversation
    /// in `DOORDER`.
    fn finish16(&mut self, vm: &mut m16::Vm, order: u32) -> Result<()> {
        self.select_screen(cell::unsigned(block(vm, order, g::SCREEN)?));
        for head in [g::HEAD_L, g::HEAD_R] {
            self.select_descriptor(cell::unsigned(block(vm, order, head)?));
            self.set_active(false);
        }
        set_block(vm, order, g::MODE, 0x10)
    }

    /// A spoken line, `0d34:10cd`–`0d34:1218`: the line's text in the
    /// speaker's color and template, centered at (160, 80) of the view, on
    /// level 0x73; mode 12 for the left speaker, 13 for the right; then the
    /// block's "a line went up" word.
    fn speak16(&mut self, vm: &mut m16::Vm, order: u32, t: &Tables, node: i32) -> Result<()> {
        self.select_screen(cell::unsigned(block(vm, order, g::SCREEN)?));
        let sx = self.screen_origin_x();
        let sy = self.screen_origin_y();
        let line = field(vm, t.lines, node * LINE);
        let right = fetch(vm, field(vm, line, 6))? & 1 != 0;
        self.select_descriptor(cell::unsigned(block(vm, order, g::ANSWER_DESCS)?));
        self.set_active(true);
        let (color, font, mode) = if right {
            (g::RIGHT_COL, g::RIGHT_FONT, 0xd)
        } else {
            (g::LEFT_COL, g::LEFT_FONT, 0xc)
        };
        self.set_color(block(vm, order, color)?)?;
        self.set_template(block(vm, order, font)?)?;
        set_block(vm, order, g::MODE, mode)?;
        self.set_text(fetch(vm, line)?)?;
        self.set_text_table(fetch(vm, field(vm, line, 2))?)?;
        self.place_x(sx + 0xa0, Placement::Center)?;
        self.place_y(sy + 0x50, Placement::Center)?;
        self.set_level(0x73)?;
        self.run16(vm, order, g::SAID, &[])
    }

    /// The answer menu, `0d34:122a`–`0d34:14b6`: up to three answers from the
    /// chain that starts at `first`, skipping those whose permission bit 0
    /// is clear, stacked downward from 20 below the view's top at x 70,
    /// each the next one's height plus 7 lower; the quiet line at 130. All
    /// in the answers' color and the left template. Ends in mode 14.
    fn choose16(&mut self, vm: &mut m16::Vm, order: u32, t: &Tables, first: i32) -> Result<()> {
        self.show_pointer();
        self.run16(vm, order, g::FINISHED, &[])?;
        let arrow = block(vm, order, g::ARROW)?;
        self.run16(vm, order, g::SET_CURSOR, &[arrow, 0, 0])?;
        self.select_screen(cell::unsigned(block(vm, order, g::SCREEN)?));
        self.select_descriptor(cell::unsigned(block(vm, order, g::CAPTION)?));
        self.set_text(1)?;
        let sx = self.screen_origin_x();
        let sy = self.screen_origin_y();
        let mut y = sy + 0x14;
        for i in 0..3 {
            set_block(vm, order, g::CHOSEN + 2 * i, 0)?;
        }
        let perms = field(vm, t.record, 0x1c);
        let slots = block(vm, order, g::ANSWER_DESCS)?;
        let color = block(vm, order, g::NORMAL_COL)?;
        let font = block(vm, order, g::LEFT_FONT)?;
        let mut n = first;
        for i in 0..3 {
            while n != -1 && fetch(vm, field(vm, perms, n * 2))? & 1 == 0 {
                n = fetch(vm, field(vm, t.answers, n * ANSWER + 2))?;
            }
            if n == -1 {
                break;
            }
            set_block(vm, order, g::CHOSEN + 2 * i, n)?;
            self.select_descriptor(cell::unsigned(slots + i));
            self.set_active(true);
            self.set_wait(-1)?;
            let line = field(
                vm,
                t.lines,
                fetch(vm, field(vm, t.answers, n * ANSWER))? * LINE,
            );
            self.set_text(fetch(vm, line)?)?;
            self.set_text_table(fetch(vm, field(vm, line, 2))?)?;
            self.note_no_effect(crate::words::Word::SDNORM);
            self.place_x(sx + 0x46, Placement::Edge)?;
            self.place_y(y, Placement::Edge)?;
            self.set_color(color)?;
            self.set_template(font)?;
            y += self.descriptor_height() + 7;
            n = fetch(vm, field(vm, t.answers, n * ANSWER + 2))?;
        }
        self.select_descriptor(cell::unsigned(slots + 3));
        self.set_active(true);
        self.set_wait(-1)?;
        self.set_text(block(vm, order, g::QUIET_TEXT)?)?;
        self.set_text_table(block(vm, order, g::QUIET_TABLE)?)?;
        self.note_no_effect(crate::words::Word::SDNORM);
        self.place_x(sx + 0x46, Placement::Edge)?;
        self.place_y(sy + 0x82, Placement::Edge)?;
        self.set_color(color)?;
        self.set_template(font)?;
        set_block(vm, order, g::MODE, 0xe)
    }

    /// A branch node, `0d34:14c8`–`0d34:163b`: a change rather than a line.
    /// Unnamed, it lands on this conversation at once — the four changes
    /// the queue knows, plus kind 5, which runs a word by name and then
    /// drains the queue; named, it is copied onto the queue for another
    /// conversation, with no room check. Either way the node steps on.
    fn branch16(&mut self, vm: &mut m16::Vm, order: u32, t: &Tables, index: i32) -> Result<()> {
        let entry = field(vm, t.branches, index * BRANCH);
        if vm.space().fetch_byte(entry)? == 0 {
            let target = fetch(vm, field(vm, entry, 0x14))?;
            let flag = field(vm, t.record, 0x1c + target * 2);
            match fetch(vm, field(vm, entry, 0x12))? {
                1 => {
                    let v = fetch(vm, flag)?;
                    store(vm, flag, v | 1)?;
                }
                2 => {
                    let v = fetch(vm, flag)?;
                    store(vm, flag, v & 0xfe)?;
                }
                3 => store(vm, field(vm, t.record, 2), target)?,
                4 => store(vm, field(vm, t.record, 0x10), target)?,
                5 => {
                    let name = name_at(vm, field(vm, entry, 9))?;
                    let name = String::from_utf8_lossy(&name).into_owned();
                    // `12c8:0677` finds the word's header by name and the
                    // handler runs the id it carries; a name that is nowhere
                    // would run the null header's id there.
                    let Some(target) = vm.mem.lookup(&name) else {
                        return Err(Error::Unread {
                            what: format!(
                                "CALCDIALOG: branch kind 5 names the word {name:?}, which no \
                                 resident module defines"
                            ),
                            binary: "ENVIRO.EXE",
                            at: "0d34:15a6",
                        });
                    };
                    vm.call_nested(target, self)?;
                    self.changes16(vm, order, t.record, t.answers)?;
                }
                _ => {}
            }
        } else {
            let queue = block(vm, order, g::CHANGE_QUEUE)?;
            let used = block(vm, order, g::CHANGE_COUNT)?;
            let bytes = vm.space().read_bytes(entry, BRANCH_BYTES)?;
            let at = field(vm, queue, used * BRANCH);
            vm.space_mut().write_bytes(at, &bytes)?;
            set_block(vm, order, g::CHANGE_COUNT, used + 1)?;
        }
        self.calc16(vm, order, 1)
    }

    /// A fresh press of either button, as the modes test it.
    fn pressed16(&self, vm: &m16::Vm, order: u32) -> Result<bool> {
        Ok(
            (block(vm, order, g::MLK)? != 0 || block(vm, order, g::MRK)? != 0)
                && block(vm, order, g::PRESSED)? == 0,
        )
    }

    /// Modes 12 and 13 (`0d34:2d16`, `0d34:2db7`): a line stands. While the
    /// descriptor shows, the speaking side hears 2 each frame — or the
    /// press cuts the line short with `SDWAIT 0`; once it is gone, 1, and
    /// the next node. The other side hears 4 every frame.
    fn standing16(&mut self, vm: &mut m16::Vm, order: u32, right: bool) -> Result<()> {
        let (speaking, listening) = if right {
            (g::SPEAK_R, g::SPEAK_L)
        } else {
            (g::SPEAK_L, g::SPEAK_R)
        };
        self.select_screen(cell::unsigned(block(vm, order, g::SCREEN)?));
        self.select_descriptor(cell::unsigned(block(vm, order, g::ANSWER_DESCS)?));
        if self.descriptor_active() == 0 {
            self.run16(vm, order, speaking, &[1])?;
            self.calc16(vm, order, 1)?;
        } else if self.pressed16(vm, order)? {
            self.set_wait(0)?;
        } else {
            self.run16(vm, order, speaking, &[2])?;
        }
        self.run16(vm, order, listening, &[4])
    }

    /// Mode 14, `0d34:2e98`: the answers are up and the pointer decides. A
    /// fresh left press in the scene is tested against the four descriptors;
    /// the hit's answer becomes the node (the fourth is the quiet node), an
    /// answer with permission bit 1 loses bit 0, and `calc_dialog` shows the
    /// next node. A fresh right press over the bar opens the give/info menu
    /// on the item there, mode 17.
    fn picking16(&mut self, vm: &mut m16::Vm, order: u32) -> Result<()> {
        let fresh = block(vm, order, g::PRESSED)? == 0;
        let left = block(vm, order, g::MLK)? != 0;
        let right = block(vm, order, g::MRK)? != 0;
        let (px, py) = (block(vm, order, g::MMX)?, block(vm, order, g::MMY)?);
        if left && fresh && px != -1 {
            self.select_screen(cell::unsigned(block(vm, order, g::SCREEN)?));
            let slots = block(vm, order, g::ANSWER_DESCS)?;
            for i in 0..4 {
                self.select_descriptor(cell::unsigned(slots + i));
                if self.descriptor_active() == 0 {
                    continue;
                }
                let (x, y) = (self.descriptor_x(), self.descriptor_y());
                let (x2, y2) = (x + self.descriptor_width(), y + self.descriptor_height());
                if px < x || py < y || px > x2 || py > y2 {
                    continue;
                }
                for j in 0..4 {
                    self.select_descriptor(cell::unsigned(slots + j));
                    self.set_active(false);
                }
                let record = block(vm, order, g::RECORD)?;
                if i == 3 {
                    let quiet = fetch(vm, field(vm, record, 0x10))?;
                    set_block(vm, order, g::NODE, quiet)?;
                } else {
                    let answers = block(vm, order, g::FIELDS)?;
                    let chosen = block(vm, order, g::CHOSEN + 2 * i)?;
                    let node = fetch(vm, field(vm, answers, chosen * ANSWER))?;
                    set_block(vm, order, g::NODE, node)?;
                    let flag = field(vm, record, 0x1c + chosen * 2);
                    let v = fetch(vm, flag)?;
                    if v & 2 != 0 {
                        store(vm, flag, v & 0xfe)?;
                    }
                }
                self.hide_pointer();
                self.run16(vm, order, g::PICKED, &[])?;
                self.calc16(vm, order, 0)?;
                break;
            }
        } else if right
            && fresh
            && block(vm, order, g::IMX)? != -1
            && let Some(slot) = menu::bar_slot(vm, order, M16_RULES)?
            && slot.item != 0
        {
            set_block(vm, order, g::TARGET, slot.item)?;
            set_block(vm, order, g::SLOT, slot.index)?;
            set_block(vm, order, g::MODE, 0x11)?;
            let strip = menu::bar_strip(vm, order, slot, M16_RULES)?;
            menu::show_menu(self, vm, order, strip, 0x60, M16_RULES)?;
        }
        self.run16(vm, order, g::SPEAK_L, &[4])?;
        self.run16(vm, order, g::SPEAK_R, &[4])
    }

    /// Mode 15, `0d34:315d`: a last line stands; when it is gone both sides
    /// hear 3 and the heads go.
    fn last_line16(&mut self, vm: &mut m16::Vm, order: u32) -> Result<()> {
        self.select_screen(cell::unsigned(block(vm, order, g::SCREEN)?));
        self.select_descriptor(cell::unsigned(block(vm, order, g::ANSWER_DESCS)?));
        if self.descriptor_active() == 0 {
            self.run16(vm, order, g::SPEAK_L, &[3])?;
            self.run16(vm, order, g::SPEAK_R, &[3])?;
            self.finish16(vm, order)?;
        } else {
            self.run16(vm, order, g::SPEAK_L, &[2])?;
        }
        self.run16(vm, order, g::SPEAK_R, &[4])
    }

    /// Mode 16, `0d34:3232`: over. The talk word hears -1, the pointer comes
    /// back, the block drops to mode 0, the finished word runs, and the
    /// exit word if there is one.
    fn over16(&mut self, vm: &mut m16::Vm, order: u32) -> Result<()> {
        self.run16(vm, order, g::TALK, &[-1])?;
        self.show_pointer();
        set_block(vm, order, g::MODE, 0)?;
        self.run16(vm, order, g::FINISHED, &[])?;
        self.run16(vm, order, g::EXIT, &[])
    }

    /// Mode 17, `0d34:3286`: the give/info strip over the bar, mid-talk.
    fn bar_menu16(&mut self, vm: &mut m16::Vm, order: u32) -> Result<()> {
        let (screen, base) = (
            block(vm, order, g::BAR_SCREEN)?,
            block(vm, order, g::BAR_MENU)?,
        );
        menu::highlight(self, vm, order, screen, base)?;
        let fresh = block(vm, order, g::PRESSED)? == 0;
        if block(vm, order, g::MLK)? != 0 && fresh {
            menu::choose(self, vm, order, screen, base, 0x60, M16_RULES)?;
        }
        if block(vm, order, g::MRK)? != 0 && fresh && block(vm, order, g::GATE1)? == 0 {
            menu::remove_menu(self, vm, order)?;
            set_block(vm, order, g::MODE, 0xe)?;
        }
        Ok(())
    }

    /// Mode 18, `0d34:32f8`: a give or an info picked from that strip runs
    /// once the figure is free, in mode 98.
    fn give16(&mut self, vm: &mut m16::Vm, order: u32) -> Result<()> {
        if !menu::figure_free(vm, order, M16_RULES)? {
            return Ok(());
        }
        set_block(vm, order, g::MODE, 0x62)?;
        let (verb, target, object) = (
            block(vm, order, g::VERB)?,
            block(vm, order, g::TARGET)?,
            block(vm, order, g::OBJECT)?,
        );
        self.exec_order(vm, order, verb, target, object, M16_RULES)?;
        self.run16(vm, order, g::PICKED, &[])
    }
}

impl Conversation<m16::Vm> for Engine {
    fn conversation_mode(&mut self, vm: &mut m16::Vm, order: u32, mode: i32) -> Result<bool> {
        match mode {
            0xc => self.standing16(vm, order, false)?,
            0xd => self.standing16(vm, order, true)?,
            0xe => self.picking16(vm, order)?,
            0xf => self.last_line16(vm, order)?,
            0x10 => self.over16(vm, order)?,
            0x11 => self.bar_menu16(vm, order)?,
            0x12 => self.give16(vm, order)?,
            _ => {}
        }
        self.order_tail(vm, order)
    }

    /// Verb 5, `TALK`, `0d34:1b2a`: the location's talk word is asked for
    /// the conversation — it answers with the record and its field table,
    /// or -1 for nothing to say — the queued changes are applied, the two
    /// heads go up with the record's sprites, both speakers hear 0, and
    /// `calc_dialog` shows the entry node.
    fn conversation_talk(&mut self, vm: &mut m16::Vm, order: u32, target: i32) -> Result<()> {
        self.select_screen(cell::unsigned(block(vm, order, g::SCREEN)?));
        self.select_descriptor(cell::unsigned(block(vm, order, g::INFO_DESC)?));
        self.set_color(block(vm, order, g::INFO_COLOR)?)?;
        self.set_template(block(vm, order, g::INFO_FONT)?)?;
        let sx = self.screen_origin_x();
        let sy = self.screen_origin_y();
        if block(vm, order, g::TALK)? == 0 {
            return Ok(());
        }
        self.run16(vm, order, g::TALK, &[target])?;
        let said = crate::stack::pop1(&mut vm.data, "EXECORDER")?;
        if said == -1 {
            return Ok(());
        }
        set_block(vm, order, g::RECORD, said)?;
        let with = crate::stack::pop1(&mut vm.data, "EXECORDER")?;
        set_block(vm, order, g::FIELDS, with)?;
        self.changes16(vm, order, said, with)?;
        let record = said;
        self.select_descriptor(cell::unsigned(block(vm, order, g::HEAD_L)?));
        self.set_active(true);
        self.set_sprite(fetch(vm, field(vm, record, 0xa))?)?;
        self.place_x(sx, Placement::Edge)?;
        self.place_y(sy + 0xa5, Placement::FarEdge)?;
        self.select_descriptor(cell::unsigned(block(vm, order, g::HEAD_R)?));
        self.set_active(true);
        self.set_sprite(fetch(vm, field(vm, record, 0xc))?)?;
        self.place_x(sx + 0x140, Placement::FarEdge)?;
        self.place_y(sy + 0xa5, Placement::FarEdge)?;
        let entry = fetch(vm, field(vm, record, 2))?;
        set_block(vm, order, g::NODE, entry)?;
        self.run16(vm, order, g::SPEAK_L, &[0])?;
        self.run16(vm, order, g::SPEAK_R, &[0])?;
        self.hide_pointer();
        self.calc16(vm, order, 0)
    }

    fn conversation_enter(&mut self, vm: &mut m16::Vm, order: u32, flag: i32) -> Result<()> {
        self.calc16(vm, order, flag)
    }

    /// The tail's work in mode 14 (`0d34:3432`): the answer under the
    /// pointer takes the left speaker's color, the others the answers'.
    fn conversation_tail(&mut self, vm: &mut m16::Vm, order: u32) -> Result<()> {
        let slots = block(vm, order, g::ANSWER_DESCS)?;
        let (px, py) = (block(vm, order, g::MMX)?, block(vm, order, g::MMY)?);
        let (hover, normal) = (
            block(vm, order, g::LEFT_COL)?,
            block(vm, order, g::NORMAL_COL)?,
        );
        self.select_screen(cell::unsigned(block(vm, order, g::SCREEN)?));
        for i in 0..4 {
            self.select_descriptor(cell::unsigned(slots + i));
            if self.descriptor_active() == 0 {
                continue;
            }
            let (x, y) = (self.descriptor_x(), self.descriptor_y());
            let inside = px >= x
                && py >= y
                && px <= x + self.descriptor_width()
                && py <= y + self.descriptor_height();
            let want = if inside { hover } else { normal };
            if self.descriptor_color() != want {
                self.set_color(want)?;
            }
        }
        Ok(())
    }
}
