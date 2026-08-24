//! The inventory bar: what it shows, and what goes in and out of it.
//!
//! One of the groups `plain_word` hands a word to. A group that does not
//! know the word answers `None` and the next one is asked.
//!
//! Both engines keep the inventory in the game's own memory, and their
//! handlers are the same algorithm cell for cell: a list — scroll offset in
//! its first cell, up to 99 item numbers from byte 4 on, zero-terminated —
//! and a table of five-cell item records with flags in the second cell and
//! the sprite in the third. The 32-bit handlers are read from `ENGINE.EXE`
//! (`?INVINCL` at `0x7f055`), the 16-bit ones from `ENVIRO.EXE` (`CCALCINV`
//! at file `0x13a5d`, `ADDTOINV` `0x13cb4`, `?INVINCL` `0x13d20`,
//! `SUBFROMINV` `0x13d98`); the two places they differ are [`Rules`].

use crate::stack::pop_n;
use crate::{Engine, Result};
use motionvm_forth::AddressSpace;

/// What differs between the two engines' inventory handlers; everything else
/// in this file is one reading that both binaries confirm.
#[derive(Clone, Copy)]
pub(crate) struct Rules {
    /// Where the list scrolls to after `ADDTOINV` filled slot `i`, if it
    /// scrolls at all.
    pub scroll_after_add: fn(i32) -> Option<i32>,
    /// How `CCALCINV` empties the eight slot descriptors before laying the
    /// bar out again: by hiding them, or by giving them sprite 0.
    pub clear_by_hiding: bool,
    /// What the trailing `0 … SCRX` of `CCALCINV` does on this machine:
    /// the 32-bit handler moves the screen origin; the 16-bit one
    /// (`05f1:06f9`, called at the handler's end, file `0x13cad`) moves
    /// the window over the surface **and requests the full rebuild** the
    /// frame loop answers with a cleared surface — without it the bar
    /// never repaints what the slots no longer cover.
    pub scrx_moves_window: bool,
}

/// MOTION 32-bit, as read from `ENGINE.EXE` with DS2: past the eighth slot
/// the list scrolls so the new item is on screen, and a cleared slot keeps
/// its descriptor active with no sprite.
pub(crate) const M32_RULES: Rules = Rules {
    scroll_after_add: |i| (i >= 8).then_some(i - 7),
    clear_by_hiding: false,
    scrx_moves_window: false,
};

/// MOTION 16-bit, as read from `ENVIRO.EXE`: `ADDTOINV` (file `0x13cb4`)
/// scrolls only from the tenth slot on and by one less — the new item lands
/// one past the window, where the forward arrow reaches it — and `CCALCINV`
/// (file `0x13a5d`) clears a slot with `SDINACTIVE`, not `SDBL 0`.
pub(crate) const M16_RULES: Rules = Rules {
    scroll_after_add: |i| (i > 8).then_some(i - 8),
    clear_by_hiding: true,
    scrx_moves_window: true,
};

impl Engine {
    pub(crate) fn words_inventory(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut dyn AddressSpace,
        rules: Rules,
    ) -> Result<Option<()>> {
        match name {
            // Lays the inventory bar out. `CALCINV` calls it the same way in
            // both games — DS2's module 5 and ENVIRO's module 602:
            //
            //   _ACTINV @  _FITEM  _INVDOWN @  _INVUP @  _ITEM @  _IS @
            //   _ORDER <75 cells> + @  _ORDER <74 cells> + @  CCALCINV
            //
            // so: the list `ADDTOINV` fills, the table of items, the two scroll
            // arrows, the first of eight slot descriptors, the inventory
            // screen, and — for the selected item — the callback word that
            // `SDWORD` hangs on its descriptor, not a caption as the argument
            // was read at first.
            //
            // The bar shows eight at a time, and the list's own head cell is
            // how far it has scrolled.
            "CCALCINV" => {
                let a = pop_n(stack, 8, "CCALCINV")?;
                let (list, items) = (a[0], a[1]);
                let (arrow_down, arrow_up) = (a[2], a[3]);
                let (first_slot, screen) = (a[4], a[5]);
                let (caption, selected) = (a[6], a[7]);
                let cell = mem.cell_size();

                // Both of these index with values that come out of module
                // memory, so both wrap rather than multiply into an overflow.
                // A negative scroll offset is reachable — in DS2 the back
                // arrow's branch is `_ACTINV @ @ 8 - _ACTINV @ !` (module 4,
                // `0x02b28`) with no floor under it, and the hit box does not
                // care whether the arrow is being drawn — and a garbage item
                // number follows from reading outside the list. The original
                // indexes both unchecked; what it reads there is whatever the
                // heap holds, which is not reproducible from here, but a panic
                // is not what it does either.
                let entry = |mem: &dyn AddressSpace, i: i32| slot_address(mem, list, i);
                let record = |mem: &dyn AddressSpace, item: i32, field: i32| {
                    mem.offset(
                        items,
                        item.wrapping_mul(5).wrapping_add(field).wrapping_mul(cell),
                    )
                };
                self.select_screen(screen as u32);
                for arrow in [arrow_up, arrow_down] {
                    self.select_descriptor(arrow as u32);
                    self.set_active(false);
                }

                // Empty all eight slots first, so a shorter list cannot leave
                // the tail of a longer one standing.
                for d in first_slot..first_slot + 8 {
                    self.select_descriptor(d as u32);
                    if rules.clear_by_hiding {
                        self.set_active(false);
                    } else {
                        self.set_sprite(0)?;
                    }
                    self.set_wait(-1)?;
                    self.set_callback(mem.callable(-1))?;
                }

                // A scrolled list whose window is no longer full scrolls back
                // until it is — otherwise removing an item would leave gaps.
                let mut offset = mem.fetch_cell(list)?;
                // A negative offset means the back arrow was clicked at the
                // start of the list. Taken as zero here, and counted, because
                // the alternative is to walk backwards out of the list and
                // report whatever the machine makes of that — which is noise,
                // not the original's answer. If the bar is ever seen to scroll
                // oddly, this is the entry to look for.
                if offset < 0 {
                    self.note_unhandled("CCALCINV (negative scroll offset)".into());
                    offset = 0;
                    mem.store_cell(list, 0)?;
                }
                if offset != 0 {
                    let mut j = offset;
                    while offset + 8 > j && mem.fetch_cell(entry(mem, j))? != 0 {
                        j += 1;
                    }
                    if offset + 8 > j {
                        offset = (j - 8).max(0);
                        mem.store_cell(list, offset)?;
                    }
                }

                // An arrow each way, shown only when there is something that
                // way: above the window, or one item past its end.
                if offset > 0 {
                    self.select_descriptor(arrow_up as u32);
                    self.set_active(true);
                }
                if mem.fetch_cell(entry(mem, offset + 8))? != 0 {
                    self.select_descriptor(arrow_down as u32);
                    self.set_active(true);
                }

                for i in offset..offset + 8 {
                    let item = mem.fetch_cell(entry(mem, i))?;
                    if item == 0 {
                        break;
                    }
                    self.select_descriptor((first_slot + i - offset) as u32);
                    self.set_active(true);
                    self.set_sprite(mem.fetch_cell(record(mem, item, 2))?)?;
                    // Only the selected item carries the caption.
                    if item == selected {
                        self.set_callback(mem.callable(caption))?;
                        self.set_wait(0)?;
                    }
                    // `and $0xfe` then `or $8`, verbatim in both binaries: bit
                    // 0 goes, bit 3 arrives, and everything above the low byte
                    // is dropped.
                    let flags = mem.fetch_cell(record(mem, item, 1))?;
                    mem.store_cell(record(mem, item, 1), (flags & 0xfe) | 8)?;
                }
                if rules.scrx_moves_window {
                    if let Some(s) = self.display.current_mut() {
                        s.pos.0 = 0;
                    }
                    self.rebuild_current_screen();
                } else {
                    self.set_screen_origin_x(0);
                }
            }
            // `?INVINCL ( item list -- f )`: is it in there?
            //
            // Walks the zero-terminated slots and stops on a match or on the
            // end; the answer is only whether the cell it stopped on holds
            // something. Nothing else — no index, no side effect.
            "?INVINCL" => {
                let a = pop_n(stack, 2, "?INVINCL")?;
                let (item, list) = (a[0], a[1]);
                let mut i = 0;
                while i < 99 {
                    let v = mem.fetch_cell(slot_address(mem, list, i))?;
                    if v == 0 || v == item {
                        break;
                    }
                    i += 1;
                }
                let found = i < 99 && mem.fetch_cell(slot_address(mem, list, i))? != 0;
                stack.push(found as i32);
            }
            "ADDTOINV" => {
                let a = pop_n(stack, 2, "inventory")?;
                add_to_inventory(mem, a[0], a[1], rules)?;
            }
            "SUBFROMINV" => {
                let a = pop_n(stack, 2, "inventory")?;
                remove_from_inventory(mem, a[0], a[1])?;
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}

/// Where slot `i` of a list lives: four bytes past the head cell in both
/// engines — which is the second cell of a 32-bit list and the third of a
/// 16-bit one, whose second cell no handler touches.
pub(crate) fn slot_address(mem: &dyn AddressSpace, list: i32, i: i32) -> i32 {
    mem.offset(list, 4i32.wrapping_add(i.wrapping_mul(mem.cell_size())))
}

/// `ADDTOINV`: puts an item in the first free slot, then scrolls the list the
/// way the engine's [`Rules`] say.
///
/// Takes module memory rather than the engine because that is all it touches:
/// the inventory is a list in the game's own memory, not engine state.
pub(crate) fn add_to_inventory(
    mem: &mut dyn AddressSpace,
    item: i32,
    list: i32,
    rules: Rules,
) -> Result<()> {
    let mut i = 0;
    while i < 99 && mem.fetch_cell(slot_address(mem, list, i))? != 0 {
        i += 1;
    }
    if i < 99 {
        mem.store_cell(slot_address(mem, list, i), item)?;
    }
    if let Some(head) = (rules.scroll_after_add)(i) {
        mem.store_cell(list, head)?;
    }
    Ok(())
}

/// `SUBFROMINV`: finds an item, then closes the gap by shifting the rest down.
///
/// An item that is not in the list leaves it alone. See
/// [`add_to_inventory`] for why this takes memory rather than the engine.
pub(crate) fn remove_from_inventory(
    mem: &mut dyn AddressSpace,
    item: i32,
    list: i32,
) -> Result<()> {
    let mut i = 0;
    while i < 99 {
        let v = mem.fetch_cell(slot_address(mem, list, i))?;
        if v == 0 || v == item {
            break;
        }
        i += 1;
    }
    if i < 99 && mem.fetch_cell(slot_address(mem, list, i))? == item {
        while i < 98 && mem.fetch_cell(slot_address(mem, list, i))? != 0 {
            let next = mem.fetch_cell(slot_address(mem, list, i + 1))?;
            mem.store_cell(slot_address(mem, list, i), next)?;
            i += 1;
        }
    }
    Ok(())
}
