//! What a click means: the `DOORDER` machine.
//!
//! Everything the player does with the picture arrives here. `ICTRL` copies the
//! input snapshot into the `_ORDER` block and calls `DOORDER` (`0x7cd61`),
//! which is a state machine on the mode cell at `+0x0c`: idle routes go to the
//! verb menu or a walk, the twelves and up belong to [`crate::dialogue`], and
//! the nineties are the forced orders a script hands in.
//!
//! Every route joins again at [`Engine::order_tail`] (`0x7eae2`), which is
//! where the mode is cleared and the interaction ends — so a branch that
//! returns without reaching it leaves the machine busy for good.
//!
//! The record helpers live here because this is where records are read: fields
//! are byte offsets into a packed `(module << 16) | offset` address, and the
//! block's own layout is named in [`crate::menu`].

use crate::menu;
use crate::stack::pop1;
use crate::{Address, Engine, Memory, Vm};
use motionvm_motion_forth::{AddressSpace, Host, Machine};
use motionvm_motion_forth::{Error, Result};

/// The `_ORDER` block's fields, by name.
///
/// `_ORDER` is one record in module 2 that the interaction machine, the verb
/// menu and the conversation code all read and write — three files here, one
/// block there. The offsets are named once, here, so that the same field
/// cannot be `RECORD` in one file and a bare `0x17c` in another.
///
/// Glob-imported rather than qualified at each use, because these names are
/// the vocabulary of all three files and reading `o(MODE)` beats
/// `o(block::MODE)` a hundred times over.
pub(crate) mod block {
    // The `_ORDER` block.
    pub(crate) const VERB: u32 = 0x00;
    pub(crate) const TARGET: u32 = 0x04;
    pub(crate) const OBJECT: u32 = 0x08;
    pub(crate) const MODE: u32 = 0x0c;
    /// First of the five menu descriptors on the scene screen, and on the bar.
    pub(crate) const SCENE_MENU: u32 = 0x10;
    pub(crate) const BAR_MENU: u32 = 0x14;
    /// The verb table: twelve bytes an entry — rest sprite, end sprite, handler.
    pub(crate) const VERBS: u32 = 0x18;
    pub(crate) const TASK: u32 = 0x78;
    pub(crate) const SCREEN: u32 = 0x7c;
    pub(crate) const BAR_SCREEN: u32 = 0x80;
    pub(crate) const AREAS: u32 = 0x84;
    pub(crate) const ITEMS: u32 = 0x88;
    pub(crate) const MMX: u32 = 0x8c;
    pub(crate) const MMY: u32 = 0x90;
    pub(crate) const IMX: u32 = 0x94;
    pub(crate) const IMY: u32 = 0x98;
    pub(crate) const MLK: u32 = 0x9c;
    pub(crate) const MRK: u32 = 0xa0;
    pub(crate) const PRESSED: u32 = 0xa4;
    pub(crate) const INVENTORY: u32 = 0xac;
    pub(crate) const CALCINV: u32 = 0xb0;
    pub(crate) const SET_CURSOR: u32 = 0xb8;
    pub(crate) const ARROW: u32 = 0xbc;
    pub(crate) const POINTER: u32 = 0xc0;
    /// Five cells, one per menu slot: 0 idle, 1 growing, 2 shrinking.
    pub(crate) const PULSE: u32 = 0xc8;
    pub(crate) const SLOT: u32 = 0xf0;
    pub(crate) const AREA_HIT: u32 = 0xfc;
    pub(crate) const CAPTION: u32 = 0x100;
    pub(crate) const APPROACH: u32 = 0x110;
    pub(crate) const OPENED: u32 = 0x114;
    pub(crate) const AFTER: u32 = 0x11c;
    pub(crate) const PICKED: u32 = 0x120;
    pub(crate) const FINISHED: u32 = 0x124;
    pub(crate) const HELD: u32 = 0x128;
    pub(crate) const GATE1: u32 = 0x134;
    pub(crate) const TAKEN: u32 = 0x138;
    pub(crate) const CANCELED: u32 = 0x13c;
    pub(crate) const GATE2: u32 = 0x1cc;
    pub(crate) const STARTED: u32 = 0x1dc;

    // The conversation's own fields, named here for the first time: they were
    // only ever written as bare offsets in `dialogue.rs`.
    /// The location's talk word, run to ask what should be said.
    pub(crate) const TALK: u32 = 0x50;
    /// How many hot areas the location declares.
    pub(crate) const AREA_COUNT: u32 = 0xf4;
    /// The speaker table: one 0x28-byte entry per speaker.
    pub(crate) const SPEAKERS: u32 = 0x16c;
    /// The `SDTB` table and `SDTXT` entry of the "say nothing" line.
    pub(crate) const QUIET_TABLE: u32 = 0x198;
    pub(crate) const QUIET_TEXT: u32 = 0x19c;
    /// Three cells: which answer each of the three menu slots is showing.
    pub(crate) const CHOSEN: u32 = 0x1a4;
    /// The deferred-change queue: where it is, how many it holds, how many are
    /// in it. Entries are 0x22 bytes.
    pub(crate) const CHANGE_QUEUE: u32 = 0x1c0;
    pub(crate) const CHANGE_ROOM: u32 = 0x1c4;
    pub(crate) const CHANGE_COUNT: u32 = 0x1c8;

    // What the verbs need on top of that.
    /// The handler word of verb *n* is `VERBS + 12·(n−1) + 8`.
    pub(crate) const HANDLER: u32 = 8;
    /// `TEXTTOPERSON`'s optional hook, run once the text has been placed.
    pub(crate) const PLACED: u32 = 0x168;
    /// The description descriptor (`_IINFO`), its text block, and the text shown
    /// when a thing has none of its own.
    pub(crate) const INFO_DESC: u32 = 0x140;
    pub(crate) const INFO_BLOCK: u32 = 0x144;
    pub(crate) const INFO_SPARE: u32 = 0x14c;
    /// What verbs 3 and 4 say when the script turns them down.
    pub(crate) const NO_HANDLE: u32 = 0x150;
    pub(crate) const NO_USE: u32 = 0x154;
    /// Where verbs 7 and 6 start looking for their keyword. In the shipped game
    /// these hold small numbers (169 and 170), so resolving them lands nowhere —
    /// the keyword that counts is the one the verb's own word answers with.
    pub(crate) const GIVE_KEY: u32 = 0x15c;
    pub(crate) const INFO_KEY: u32 = 0x160;
    /// The conversation: its record, its fields, the node, and the first of the
    /// four descriptors the answers are drawn on.
    pub(crate) const RECORD: u32 = 0x17c;
    pub(crate) const FIELDS: u32 = 0x180;
    pub(crate) const ANSWER_DESCS: u32 = 0x174;
    pub(crate) const NODE: u32 = 0x194;
    /// An answer is 0x12 bytes and carries its name at +8.
    pub(crate) const ANSWER: u32 = 0x12;
    pub(crate) const A_NAME: u32 = 8;
    pub(crate) const INFO_COLOR: u32 = 0x1e0;
    pub(crate) const INFO_FONT: u32 = 0x1e4;
}

use block::*;

/// What differs between the two engines' interaction machines.
///
/// The rest of this file and [`crate::menu`] is one reading both binaries
/// confirm: the 16-bit `DOORDER` (`ENVIRO.EXE` file `0x1255f`), `EXECORDER`
/// (`0d34:164e`) and the menu routines in front of them (`0d34:0006`–
/// `0d34:0b9b`) are the same machine on a 260-byte `_ORDER` block with every
/// field at half the offset — the same modes, the same click path, the same
/// verb dispatch. What is not the same is the size of the inventory bar and
/// the pixels around the menu, how a verb waits for the figure, and whether
/// the change queue is checked for room.
#[derive(Clone, Copy)]
pub(crate) struct Rules {
    /// Where the bar's first slot starts, and how wide one of its eight is.
    pub bar_x0: i32,
    pub slot_w: i32,
    /// How far down the verb strip stands over the bar.
    pub bar_menu_y: i32,
    /// How far apart the strip's icons are; the strip is centered by half a
    /// step per icon.
    pub icon_step: i32,
    /// The hot spot of the menu pointer, and of an item carried in the hand.
    pub pointer_hot: (i32, i32),
    pub held_hot: (i32, i32),
    /// Whether every verb waits for the figure by its command cell — 0 or
    /// 999 — as the 16-bit engine does, or whether mode 5 and the armed walk
    /// test the walking flag instead, as the 32-bit one does.
    pub idle_by_command: bool,
    /// Whether `ADDMESSPIPE` checks the change queue's room.
    pub change_room_checked: bool,
    /// The inventory's own rules, for the take verb and the flash entry.
    pub inventory: crate::words::InventoryRules,
}

/// MOTION 32-bit, from `ENGINE.EXE`: slots of 64 from x 0x40, the strip
/// 0x30 down and 0x30 apart, the pointer at (0x10, 0x10) and a held item at
/// (0x20, 0x18); mode 5 and the armed walk test `person[0x1a8]`.
pub(crate) const M32_RULES: Rules = Rules {
    bar_x0: 0x40,
    slot_w: 64,
    bar_menu_y: 0x30,
    icon_step: 0x30,
    pointer_hot: (0x10, 0x10),
    held_hot: (0x20, 0x18),
    idle_by_command: false,
    change_room_checked: true,
    inventory: crate::words::M32_RULES,
};

/// MOTION 16-bit, from `ENVIRO.EXE`: slots of 32 from x 0x20 (`0d34:2127`),
/// the strip 0x14 down (`0d34:23b3`) and 0x14 apart (`0d34:0157`), the
/// pointer at (8, 8) and a held item at (0x10, 0xa) (`0d34:2172`); every
/// wait is on `person[0xe6]` being 0 or 999 (`0d34:2621`, `0d34:2ade`);
/// `ADDMESSPIPE` (`0d34:3901`) never reads the room cell.
pub(crate) const M16_RULES: Rules = Rules {
    bar_x0: 0x20,
    slot_w: 32,
    bar_menu_y: 0x14,
    icon_step: 0x14,
    pointer_hot: (8, 8),
    held_hot: (0x10, 0xa),
    idle_by_command: true,
    change_room_checked: false,
    inventory: crate::words::M16_RULES,
};

/// A field's address on this machine: `off` is the 32-bit layout's byte
/// offset, and the field sits `off / 4` cells into the record on either.
pub(crate) fn at(mem: &dyn AddressSpace, base: u32, off: u32) -> i32 {
    mem.offset(base as i32, (off / 4) as i32 * mem.cell_size())
}

/// A cell of a record, by the 32-bit layout's byte offset.
pub(crate) fn get<M: Machine>(vm: &M, base: u32, off: u32) -> Result<i32> {
    let mem = vm.space();
    mem.fetch_cell(at(mem, base, off))
}

pub(crate) fn put<M: Machine>(vm: &mut M, base: u32, off: u32, v: i32) -> Result<()> {
    let mem = vm.space_mut();
    let a = at(mem, base, off);
    mem.store_cell(a, v)
}

/// The conversation half of the machine: modes 12 to 18, the talk verb, the
/// answer menus and their colors.
///
/// Read from `ENGINE.EXE` for the 32-bit machine and built in
/// [`crate::dialogue`]; read from `ENVIRO.EXE` for the 16-bit machine and
/// built in [`crate::dialogue16`] — an older layout on a smaller screen,
/// which is why the two are not one reading with rules.
pub(crate) trait Conversation<M: Machine> {
    /// One of the conversation modes, 12 to 18.
    fn conversation_mode(&mut self, vm: &mut M, order: u32, mode: i32) -> Result<bool>;
    /// Verb 5, `TALK`.
    fn conversation_talk(&mut self, vm: &mut M, order: u32, target: i32) -> Result<()>;
    /// The answer layout, `calc_dialog`, as verbs 6 and 7 enter it.
    fn conversation_enter(&mut self, vm: &mut M, order: u32, flag: i32) -> Result<()>;
    /// The tail's work in mode 14: the answer under the pointer lights up.
    fn conversation_tail(&mut self, vm: &mut M, order: u32) -> Result<()>;
}

impl Conversation<Vm> for Engine {
    fn conversation_mode(&mut self, vm: &mut Vm, order: u32, mode: i32) -> Result<bool> {
        match mode {
            // A conversation is running: 12 and 13 are a line standing on
            // screen, 16 is the end of it. `?DIALON` reports exactly this
            // range, 12 to 18, which is how a location waits for a
            // conversation to finish.
            12 | 13 => self.dialog_waiting(vm, order, mode == 13),
            14 => self.dialog_picking(vm, order),
            16 => self.dialog_over(vm, order),
            _ => Err(Error::Unread {
                what: format!("DOORDER: the mode {mode} branch"),
                at: match mode {
                    15 => "0x7e6d0",
                    17 => "0x7e918",
                    _ => "0x7e9b1",
                },
            }),
        }
    }

    fn conversation_talk(&mut self, vm: &mut Vm, order: u32, target: i32) -> Result<()> {
        self.exec_talk(vm, order, target)
    }

    fn conversation_enter(&mut self, vm: &mut Vm, order: u32, flag: i32) -> Result<()> {
        self.calc_dialog(vm, order, flag)
    }

    /// The answer under the pointer lights up. Every frame, for each of the
    /// four descriptors: inside its box it wants the speaker table's +4,
    /// outside it wants +0x18, and `SDCOL` only runs when it is not already
    /// that (0x7ec71-0x7ecd6) — the compare is there so a text is not
    /// re-measured for nothing.
    fn conversation_tail(&mut self, vm: &mut Vm, order: u32) -> Result<()> {
        let at = |off: u32| Self::field(order, off);
        let table = vm.mem.fetch(at(SPEAKERS))?;
        let slots = vm.mem.fetch(at(ANSWER_DESCS))? as i32;
        let (px, py) = (vm.mem.fetch(at(MMX))? as i32, vm.mem.fetch(at(MMY))? as i32);
        self.select_screen(vm.mem.fetch(at(SCREEN))?);
        for i in 0..4i32 {
            self.select_descriptor((slots + i) as u32);
            if self.descriptor_active() == 0 {
                continue;
            }
            let x = self.descriptor_x();
            let y = self.descriptor_y();
            let x2 = x + self.descriptor_width();
            let y2 = y + self.descriptor_height();
            let under = px >= x && py >= y && px <= x2 && py <= y2;
            let want = Self::cell(&vm.mem, table, if under { 4 } else { 0x18 })? as i32;
            if self.descriptor_color() != want {
                self.set_color(want)?;
            }
        }
        Ok(())
    }
}

impl Engine {
    /// `DOORDER ( _ORDER -- )`, the click dispatcher — 8072 bytes from 0x7cd61
    /// to 0x7ece8, the entry into the game's whole interaction machine.
    ///
    /// What is built here is its branch structure and the route an idle frame
    /// takes: with no button down and mode 0 it goes 0x7ce33 → 0x7d088 →
    /// 0x7d459 → 0x7eae2 and out, doing nothing. That is enough for the
    /// controller to reach the line that starts the location, which is what
    /// makes the game runnable and the comparison against the original
    /// available again.
    ///
    /// Every branch that has not been read through yet stops with its own
    /// address rather than falling through. That is deliberately not a stub: a
    /// stub is silent and indistinguishable from a finished word, this names
    /// itself the moment the game reaches it.
    pub(crate) fn do_order<M>(&mut self, vm: &mut M, rules: Rules) -> Result<bool>
    where
        M: Machine,
        Engine: Host<M> + Conversation<M>,
    {
        let order = pop1(vm.data(), "DOORDER")? as u32;

        // An argument that names no module does nothing at all.
        //
        // The handler puts its argument through 0x67404 (0x7cd79) and reads
        // +0xac, +0x88, +0x84 and +0xa8 off it, so it wants the `_ORDER` block,
        // and `ICTRL` passes exactly that. `FORCE_ORDER` (module 13) does not:
        // it fills the block, sets the mode to 97, and then hands over
        // `_ORDER @` — the block's *first cell*, which it has just set to the
        // verb. Every one of the forty `FORCE_ORDER` call sites in the game
        // passes `TALK`, so what arrives here is the number 5, and module 0
        // does not exist. `FORCE_ORDER` (module 603) of
        // Die Enviro-Kids greifen ein does the same
        // with a flat address, which names no loaded module either.
        //
        // That call is not where the work is. The work is the four stores, and
        // `ICTRL` picks them up on the next frame: mode 97 has its own branch
        // at 0x7ea22, reached with the real block. So the original getting a 5
        // here reads whatever module 0 would be and finds no mode it knows —
        // nothing happens, no check needed, which is why there is none.
        if !vm.space().is_live(order as i32) {
            return Ok(true);
        }
        let f = |vm: &M, off: u32| get(vm, order, off);

        // The prologue resolves four pointers out of the block. On the idle
        // route none of them is used, but they are address arithmetic with no
        // side effect, so they stay where the handler has them.
        let _inventory = f(vm, 0xac)?;
        let _items = f(vm, 0x88)?;
        let _areas = f(vm, 0x84)?;

        if f(vm, 0xa8)? != 0 {
            return self.order_tail(vm, order);
        }

        let mode = f(vm, 0x0c)?;
        let (left, right) = (f(vm, 0x9c)?, f(vm, 0xa0)?);
        let pressed = f(vm, 0xa4)?;

        // `_MPRESSED` is what distinguishes a fresh press from a held button:
        // the branches below all want it to be zero. The jumps read the other
        // way round — as if a set flag meant "act" — send every idle frame
        // into the right-click branch; the flag means "still held".
        let fresh = pressed == 0;

        // 0x7cdcd: modes 0 and 9 take the click path; mode 8 only on a fresh
        // press with neither pending field set; every other mode is a menu and
        // belongs to 0x7d4a8.
        let click_path = match mode {
            0 | 9 => true,
            8 => (left != 0 || right != 0) && fresh && f(vm, 0x134)? == 0 && f(vm, 0x1cc)? == 0,
            _ => false,
        };
        if !click_path {
            // 0x7d4a8 chains the modes the verb menu walks through — 2 at
            // 0x7d4ab, then 3, 4, 5, 6, 7 and 8 at 0x7d568, 0x7d972, 0x7dab3,
            // 0x7dafe, 0x7dbe3 and 0x7dd15 — and runs on into the three
            // forced-order modes at 0x7ea22. A mode none of them claims drops
            // out of the bottom of the chain (0x7eaa1) into the epilogue.
            return match mode {
                12..=18 => self.conversation_mode(vm, order, mode),
                97..=99 => self.order_forced(vm, order, mode, rules),
                2..=8 => {
                    menu::mode_chain(self, vm, order, mode, rules)?;
                    self.order_tail(vm, order)
                }
                _ => self.order_tail(vm, order),
            };
        }

        // 0x7ce33: a fresh left press is a click on the inventory bar or into
        // the scene.
        //
        // `armed` is the handler's local at -0x18. The left click sets it at
        // 0x7d081 together with mode 9, and the tail below tests it: a walk
        // ordered by *this* frame's click is deliberately left to the next
        // frame. Read as "run it at once" it puts a halt where the original
        // has none.
        let mut armed = false;
        if left != 0 && fresh && f(vm, 0x134)? == 0 && f(vm, 0x1cc)? == 0 {
            self.order_left_click(vm, order, rules)?;
            armed = get(vm, order, MODE)? == 9;
        }
        // 0x7d088: the same for the right button, which opens the verb menu.
        if right != 0 && fresh && f(vm, 0x1cc)? == 0 {
            menu::right_click(self, vm, order, rules)?;
        }
        // 0x7d459: a walk that was already pending runs now, as verb 8 — but
        // only while the location's own task is between jobs.
        if f(vm, 0x0c)? == 9 && !armed && menu::figure_free(vm, order, rules)? {
            put(vm, order, MODE, 0)?;
            let target = f(vm, 0x04)?;
            self.exec_order(vm, order, 8, target, 0, rules)?;
        }
        self.order_tail(vm, order)
    }

    /// The three forced-order modes, 0x7ea22 through 0x7eae2.
    ///
    /// `FORCE_ORDER` parks a verb, a target and a flag in the block and sets
    /// the mode to 97 — the whole of what a location does to make somebody talk
    /// without anybody clicking. The next `DOORDER`, the one `ICTRL` makes
    /// every frame with the real block, finds it here:
    ///
    /// ```text
    /// 97 (0x7ea22):  mode = 99;  EXECORDER(block, o[0], o[4], o[8])
    /// 98 (0x7ea53):  o[0x11c] & 1 ? EXECORDER(…) : run o[0x124], mode = 0xe
    /// 99 (0x7ea9a):  o[0x11c] & 1 ? EXECORDER(…) : run o[0x124], mode = 0
    /// ```
    ///
    /// All three then join the epilogue. The callback at +0x124 takes nothing —
    /// 0x7ea68 fetches it and calls straight into the interpreter.
    pub(crate) fn order_forced<M>(
        &mut self,
        vm: &mut M,
        order: u32,
        mode: i32,
        rules: Rules,
    ) -> Result<bool>
    where
        M: Machine,
        Engine: Host<M> + Conversation<M>,
    {
        let execute = if mode == 97 {
            put(vm, order, MODE, 99)?;
            true
        } else if get(vm, order, AFTER)? & 1 == 0 {
            self.order_callback(vm, order, 0x124, &[])?;
            put(vm, order, MODE, if mode == 98 { 0x0e } else { 0 })?;
            false
        } else {
            true
        };
        if execute {
            let verb = get(vm, order, VERB)?;
            let target = get(vm, order, TARGET)?;
            let flag = get(vm, order, OBJECT)?;
            self.exec_order(vm, order, verb, target, flag, rules)?;
        }
        self.order_tail(vm, order)
    }

    /// `EXECORDER`: one verb, on one target. Head at 0x7bf80, body at 0x7bfbb.
    ///
    /// The head does nothing but save its four register arguments and jump into
    /// the body, which resolves the same three pointers `DOORDER`'s prologue
    /// does and then dispatches on the verb through a jump table at 0x7bf9b:
    ///
    /// | verb | 1 | 2 | 3 | 4 | 5 `TALK` | 6 | 7 | 8 |
    /// |---|---|---|---|---|---|---|---|---|
    /// | at | 0x7c00c | 0x7c111 | 0x7c3aa | 0x7c543 | **0x7c69e** | 0x7c7e8 | 0x7ca7f | 0x7cd43 |
    ///
    /// A verb outside 1..=8 leaves at once — `cmpl $7` on `verb - 1` unsigned
    /// and `ja 0x7cd5b` at 0x7bff9, which is the exit.
    ///
    /// **The table has to be read as data, not disassembled.** A disassembler
    /// following the `jmpl *%cs:0x7bf9b` decodes the table itself as
    /// instructions and loses its footing a few bytes in — which is why
    /// `EXECORDER` measures seventeen instructions when it is 783.
    pub(crate) fn exec_order<M>(
        &mut self,
        vm: &mut M,
        order: u32,
        verb: i32,
        target: i32,
        _flag: i32,
        rules: Rules,
    ) -> Result<()>
    where
        M: Machine,
        Engine: Host<M> + Conversation<M>,
    {
        // The same three the prologue of `DOORDER` resolves, at 0x7bfbe,
        // 0x7bfcf and 0x7bfe0. Address arithmetic with no side effect; they
        // stay because the handler has them.
        let _inventory = get(vm, order, INVENTORY)?;
        let _items = get(vm, order, ITEMS)?;
        let _areas = get(vm, order, AREAS)?;

        match verb {
            5 => self.conversation_talk(vm, order, target),
            1..=8 => {
                menu::exec_verb(self, vm, order, verb, rules)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Verb 5, `TALK`, at 0x7c69e.
    ///
    /// It sets four things up out of the block and then asks the location what
    /// should be said:
    ///
    /// ```text
    /// o[0x7c] ACTSCR    o[0x140] ACTDESC    o[0x1e0] SDCOL    o[0x1e4] SDTDT
    /// if o[0x50] ≠ 0:  push o[4], run o[0x50]
    ///     said = pop;  −1 means there was nothing to say
    ///     o[0x17c] = said;  o[0x180] = pop
    /// ```
    ///
    /// So the location's talk word takes the target and answers with **two**
    /// values, both addresses, which 0x7b258 and 0x7b49c then work through.
    /// Those two are not read yet — 3368 bytes between them — so this stops
    /// where the reading stops.
    ///
    /// `GSCRX` and `GSCRY` (0x7c702, 0x7c714) are left out on purpose: their
    /// results go into locals that only the unread part uses, and neither reads
    /// anything this does not already know.
    pub(crate) fn exec_talk(&mut self, vm: &mut Vm, order: u32, target: i32) -> Result<()> {
        let at = |off: u32| Self::field(order, off);
        // The four the handler sets before it says anything: which screen and
        // which descriptor, then the color and the template to say it in.
        // Their offsets into the `_ORDER` block are the evidence and stay in
        // sight.
        self.select_screen(vm.mem.fetch(at(SCREEN))?);
        self.select_descriptor(vm.mem.fetch(at(INFO_DESC))?);
        self.set_color(vm.mem.fetch(at(INFO_COLOR))? as i32)?;
        self.set_template(vm.mem.fetch(at(INFO_FONT))? as i32)?;

        // 0x7c726: no talk word, nothing to say.
        if vm.mem.fetch(at(TALK))? == 0 {
            return Ok(());
        }
        self.order_callback(vm, order, 0x50, &[target])?;
        let said = pop1(&mut vm.data, "EXECORDER")?;
        // 0x7c75b: -1 is the location saying it has no line for this target.
        if said == -1 {
            return Ok(());
        }
        vm.mem.store(at(RECORD), said as u32)?;
        let with = pop1(&mut vm.data, "EXECORDER")?;
        vm.mem.store(at(FIELDS), with as u32)?;
        // 0x7c7b1 and 0x7c7de, with the block's own +0x194 set from the record
        // in between (0x7c7bf, and again at 0x7c7d3 — the handler stores it
        // twice, either side of `HIDEMOUSE`).
        self.dialogue_changes(vm, order, said as u32, with as u32)?;
        let entry = Self::cell(&vm.mem, said as u32, 4)?;
        vm.mem.store(at(NODE), entry)?;
        self.calc_dialog(vm, order, 0)
    }

    /// A field of a record, by packed address and byte offset.
    pub(crate) fn field(base: u32, off: u32) -> Address {
        Address::new(base >> 16, (base & 0xffff).wrapping_add(off))
    }

    /// A 32-bit field of a record, wherever it happens to start.
    ///
    /// Conversation records are packed at strides the interpreter never uses —
    /// 0x12 for an answer, 0x22 for a branch — so every other one begins
    /// mid-cell. The native handlers read them straight off a pointer and do
    /// not care; `Memory::fetch` addresses cells and would quietly hand back
    /// the neighbors, which is how the answer menu came to show a spoken line.
    pub(crate) fn cell(mem: &Memory, base: u32, off: u32) -> Result<u32> {
        mem.fetch_unaligned(Self::field(base, off))
    }

    /// Two NUL-terminated strings in module memory, compared as `CompareString`
    /// at 0x11950 does — which is `strcmp` with both null pointers counting as
    /// equal.
    pub(crate) fn same_name(vm: &Vm, a: u32, b: u32) -> Result<bool> {
        for i in 0..64u32 {
            let (x, y) = (
                vm.mem.fetch_byte(Self::field(a, i))?,
                vm.mem.fetch_byte(Self::field(b, i))?,
            );
            if x != y {
                return Ok(false);
            }
            if x == 0 {
                return Ok(true);
            }
        }
        Ok(true)
    }

    /// The left half of `DOORDER` at 0x7ce73: a fresh left press.
    ///
    /// On the inventory bar it takes the item into the hand; in the scene it
    /// asks 0x7a66c what is under the pointer and either picks the item up —
    /// only if its flag bit 0 says it can be — or, for an exit, arms mode 9 so
    /// the caller runs the walk verb.
    ///
    /// Either way the pointer becomes the item's own sprite, one higher than
    /// the one drawn in the bar, hung at the engine's hot spot for a held
    /// item.
    pub(crate) fn order_left_click<M>(&mut self, vm: &mut M, order: u32, rules: Rules) -> Result<()>
    where
        M: Machine,
        Engine: Host<M> + Conversation<M>,
    {
        put(vm, order, VERB, 0)?;
        put(vm, order, MODE, 0)?;

        let items = get(vm, order, ITEMS)? as u32;
        let imx = get(vm, order, IMX)?;

        // Taking an item into the hand, from wherever it was found.
        let take = |me: &mut Self, vm: &mut M, item: i32, from_scene: bool| -> Result<()> {
            put(vm, order, TARGET, item)?;
            let sprite = get(vm, items, item as u32 * 20 + 8)?;
            let (hx, hy) = rules.held_hot;
            me.order_callback(vm, order, 0xb8, &[sprite + 1, hx, hy])?;
            put(vm, order, MODE, 3)?;
            put(vm, order, VERB, 4)?;
            if from_scene {
                me.set_flash_entry(vm, order, rules)?;
                put(vm, order, TAKEN, 1)?;
            } else {
                me.order_callback(vm, order, 0x114, &[3])?;
                put(vm, order, TAKEN, -1)?;
            }
            Ok(())
        };

        // The bar: eight slots, exactly where `CCALCINV` puts them.
        if (rules.bar_x0..rules.bar_x0 + 8 * rules.slot_w).contains(&imx) {
            let list = get(vm, order, INVENTORY)?;
            let slot = (imx - rules.bar_x0) / rules.slot_w;
            let index = get(vm, list as u32, 0)? + slot;
            let item =
                vm.space()
                    .fetch_cell(crate::words::slot_address(vm.space(), list, index))?;
            if item != 0 {
                take(self, vm, item, false)?;
            }
            return Ok(());
        }

        self.area_under_pointer(vm, order)?;
        let hit = get(vm, order, AREA_HIT)?;
        if hit == 0 {
            return Ok(());
        }
        let areas = get(vm, order, AREAS)? as u32;
        let field = |vm: &M, k: u32| get(vm, areas, (hit as u32 - 1000) * 64 + k);
        let item = field(vm, 0x28)?;
        if item != 0 {
            // Bit 0 of the item's flags is what makes it takeable at all.
            if get(vm, items, item as u32 * 20 + 4)? & 1 != 0 {
                take(self, vm, item, true)?;
            }
            return Ok(());
        }
        let exit = field(vm, 0x24)?;
        if exit != 0 {
            put(vm, order, TARGET, exit)?;
            put(vm, order, MODE, 9)?;
        }
        Ok(())
    }

    /// `0x7a66c` — which of the location's hot areas the pointer is in.
    ///
    /// The same rectangle test as `?XINSIDE`, but over the table the `_ORDER`
    /// block carries at +0x84 with its count at +0xF4, and the answer goes into
    /// +0xFC. It is stored **biased by 1000**, which is why the handler then
    /// addresses area fields with large negative displacements: `area*64 -
    /// 0xf9d8` folds to `i*64 + 0x28`. The 16-bit `hit_area` (`0d34:03d6`)
    /// is the same walk over 32-byte records.
    pub(crate) fn area_under_pointer<M: Machine>(&mut self, vm: &mut M, order: u32) -> Result<()> {
        let areas = get(vm, order, AREAS)? as u32;
        put(vm, order, AREA_HIT, 0)?;
        let (mx, my) = (get(vm, order, MMX)?, get(vm, order, MMY)?);
        for i in 0..get(vm, order, AREA_COUNT)? {
            let c: Vec<i32> = (0..4)
                .map(|k| get(vm, areas, (i as u32) * 64 + k * 4).unwrap_or(0))
                .collect();
            if mx >= c[0] && mx <= c[2] && my >= c[1] && my <= c[3] {
                put(vm, order, AREA_HIT, i + 1000)?;
                break;
            }
        }
        Ok(())
    }

    /// `SETFLASHENTRY` at 0x7ac36 — the picked-up item goes into the inventory.
    ///
    /// Adds it, finds it again, and scrolls the bar so it is on screen if it
    /// landed past the eighth slot; then redraws through `CALCINV`, whose
    /// address the block carries at +0xB0. The 16-bit `flash_entry`
    /// (`0d34:086c`) is the same.
    pub(crate) fn set_flash_entry<M>(&mut self, vm: &mut M, order: u32, rules: Rules) -> Result<()>
    where
        M: Machine,
        Engine: Host<M>,
    {
        let list = get(vm, order, INVENTORY)?;
        let item = get(vm, order, HELD)?;

        let screen = get(vm, order, BAR_SCREEN)?;
        self.select_screen(screen as u32);
        crate::words::add_to_inventory(vm.space_mut(), item, list, rules.inventory)?;

        let slot = |vm: &M, i: i32| {
            vm.space()
                .fetch_cell(crate::words::slot_address(vm.space(), list, i))
        };
        let mut i = 0;
        while i < 99 {
            let v = slot(vm, i)?;
            if v == 0 || v == item {
                break;
            }
            i += 1;
        }
        if slot(vm, i)? == item && i >= 8 {
            let offset = get(vm, list as u32, 0)?;
            if i - 7 > offset {
                put(vm, list as u32, 0, i - 7)?;
            }
        }
        self.order_callback(vm, order, 0xb0, &[])
    }

    /// Runs one of the bytecode words the `_ORDER` block carries, with its
    /// arguments — how the native handlers reach back into the game. The
    /// block holds what the game stores for a word: a packed address on the
    /// 32-bit machine, a word id on the 16-bit one (`1400:028f` runs it
    /// there); the machine resolves either.
    pub(crate) fn order_callback<M>(
        &mut self,
        vm: &mut M,
        order: u32,
        slot: u32,
        args: &[i32],
    ) -> Result<()>
    where
        M: Machine,
        Engine: Host<M>,
    {
        let raw = get(vm, order, slot)?;
        let Some(target) = vm.callback_target(raw) else {
            return Err(Error::Unsupported(format!(
                "DOORDER: the callback at +{slot:#x} of the order block, {raw:#x}, names no word"
            )));
        };
        vm.data().extend_from_slice(args);
        vm.call_nested(target, self)
    }

    /// The tail at 0x7eae2, which every route through `DOORDER` joins.
    ///
    /// Not a menu builder, as it was once filed — the common epilogue. With a
    /// menu open it lets `ANIMATEORDERS` breathe on the right screen; mode 0xE
    /// walks four descriptors; everything else falls straight through, which
    /// is what an idle frame does. The 16-bit tail (`0d34:33db`) is the same.
    pub(crate) fn order_tail<M>(&mut self, vm: &mut M, order: u32) -> Result<bool>
    where
        M: Machine,
        Engine: Host<M> + Conversation<M>,
    {
        match get(vm, order, MODE)? {
            mode @ (2 | 4 | 0x11) => {
                let bar = mode != 4;
                let screen = get(vm, order, if bar { 0x80 } else { 0x7c })?;
                let base = get(vm, order, if bar { 0x14 } else { 0x10 })?;
                menu::animate(self, vm, order, screen, base)?;
                Ok(true)
            }
            0xe => {
                self.conversation_tail(vm, order)?;
                Ok(true)
            }
            _ => Ok(true),
        }
    }
}
