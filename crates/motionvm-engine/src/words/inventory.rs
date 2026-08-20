//! The inventory bar: what it shows, and what goes in and out of it.
//!
//! One of the groups `plain_word` hands a word to. A group that does not
//! know the word answers `None` and the next one is asked.

use crate::stack::{callable, pop_n};
use crate::{Engine, Result};
use motionvm_forth::{Address, Memory};

impl Engine {
    pub(crate) fn words_inventory(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut Memory,
    ) -> Result<Option<()>> {
        match name {
            // The inventory is a plain array in module memory: a scroll offset
            // at +0, then up to 99 slots of item numbers with a zero marking
            // the end. Both words take the list as their second argument.
            // Lays the inventory bar out. `CALCINV` in module 5 calls it as
            //
            //   _ACTINV @  _FITEM  _INVDOWN @  _INVUP @  _ITEM @  _IS @
            //   _ORDER 300 + @  _ORDER 296 + @  CCALCINV
            //
            // so: the list `ADDTOINV` fills, the table of items, the two scroll
            // arrows, the first of eight slot descriptors, the inventory
            // screen, and — for the selected item — the callback word that
            // `SDWORD` hangs on its descriptor, not a caption as the argument
            // was read at first.
            //
            // The item table has twenty bytes an entry, sprite at +8 and flags
            // at +4. The bar shows eight at a time, and the list's own head
            // cell is how far it has scrolled.
            "CCALCINV" => {
                let a = pop_n(stack, 8, "CCALCINV")?;
                let (list, items) = (a[0] as u32, a[1] as u32);
                let (arrow_down, arrow_up) = (a[2], a[3]);
                let (first_slot, screen) = (a[4], a[5]);
                let (caption, selected) = (a[6], a[7]);

                let head = Address::new(list >> 16, list & 0xffff);
                // Both of these index with values that come out of module
                // memory, so both wrap rather than multiply into an overflow.
                // A negative scroll offset is reachable — the back arrow's
                // branch is `_ACTINV @ @ 8 - _ACTINV @ !` (module 4,
                // `0x02b28`) with no floor under it, and the hit box does not
                // care whether the arrow is being drawn — and a garbage item
                // number follows from reading outside the list. The original
                // indexes both unchecked; what it reads there is whatever the
                // heap holds, which is not reproducible from here, but a panic
                // is not what it does either.
                let entry = |i: i32| {
                    let off = 4u32.wrapping_add((i as u32).wrapping_mul(4));
                    Address::new(list >> 16, (list & 0xffff).wrapping_add(off))
                };
                let record = |item: u32, off: u32| {
                    let at = item.wrapping_mul(20).wrapping_add(off);
                    Address::new(items >> 16, (items & 0xffff).wrapping_add(at))
                };
                self.select_screen((screen) as u32);
                for arrow in [arrow_up, arrow_down] {
                    self.select_descriptor((arrow) as u32);
                    self.set_active(false);
                }

                // Empty all eight slots first, so a shorter list cannot leave
                // the tail of a longer one standing.
                for d in first_slot..first_slot + 8 {
                    self.select_descriptor((d) as u32);
                    self.set_sprite(0)?;
                    self.set_wait(-1)?;
                    self.set_callback(callable(-1, mem))?;
                }

                // A scrolled list whose window is no longer full scrolls back
                // until it is — otherwise removing an item would leave gaps.
                let mut offset = mem.fetch(head)? as i32;
                // A negative offset means the back arrow was clicked at the
                // start of the list. Taken as zero here, and counted, because
                // the alternative is to walk backwards out of the list and
                // report whatever `Memory::fetch` makes of that — which is
                // noise, not the original's answer. If the bar is ever seen to
                // scroll oddly, this is the entry to look for.
                if offset < 0 {
                    self.note_unhandled("CCALCINV (negative scroll offset)".into());
                    offset = 0;
                    mem.store(head, 0)?;
                }
                if offset != 0 {
                    let mut j = offset;
                    while offset + 8 > j && mem.fetch(entry(j))? != 0 {
                        j += 1;
                    }
                    if offset + 8 > j {
                        offset = (j - 8).max(0);
                        mem.store(head, offset as u32)?;
                    }
                }

                // An arrow each way, shown only when there is something that
                // way: above the window, or one item past its end.
                if offset > 0 {
                    self.select_descriptor((arrow_up) as u32);
                    self.set_active(true);
                }
                if mem.fetch(entry(offset + 8))? != 0 {
                    self.select_descriptor((arrow_down) as u32);
                    self.set_active(true);
                }

                for i in offset..offset + 8 {
                    let item = mem.fetch(entry(i))?;
                    if item == 0 {
                        break;
                    }
                    self.select_descriptor((first_slot + i - offset) as u32);
                    self.set_active(true);
                    self.set_sprite(mem.fetch(record(item, 8))? as i32)?;
                    // Only the selected item carries the caption.
                    if item as i32 == selected {
                        self.set_callback(callable(caption, mem))?;
                        self.set_wait(0)?;
                    }
                    // `andl $0xfe` then `orb $8`, verbatim: bit 0 goes, bit 3
                    // arrives, and everything above the low byte is dropped.
                    let flags = mem.fetch(record(item, 4))?;
                    mem.store(record(item, 4), (flags & 0xfe) | 8)?;
                }
                self.set_screen_origin_x(0);
            }
            // `?INVINCL ( gegenstand liste -- f )`, 0x7f055: is it in there?
            //
            // Walks the zero-terminated slots and stops on a match or on the
            // end; the answer is only whether the cell it stopped on holds
            // something. Nothing else — no index, no side effect.
            "?INVINCL" => {
                let a = pop_n(stack, 2, "?INVINCL")?;
                let (item, list) = (a[0], a[1] as u32);
                let slot = |i: u32| Address::new(list >> 16, (list & 0xffff) + 4 + i * 4);
                let mut i = 0;
                while i < 99 {
                    let v = mem.fetch(slot(i))?;
                    if v == 0 || v == item as u32 {
                        break;
                    }
                    i += 1;
                }
                let found = i < 99 && mem.fetch(slot(i))? != 0;
                stack.push(found as i32);
            }
            "ADDTOINV" => {
                let a = pop_n(stack, 2, "inventory")?;
                add_to_inventory(mem, a[0], a[1] as u32)?;
            }
            "SUBFROMINV" => {
                let a = pop_n(stack, 2, "inventory")?;
                remove_from_inventory(mem, a[0], a[1] as u32)?;
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}

/// Where a list keeps its scroll offset, and where slot `i` lives.
///
/// The list is a head cell followed by 99 slots. Both words address it the same
/// way, which is the only thing they share.
fn slots(list: u32) -> (Address, impl Fn(u32) -> Address) {
    let head = Address::new(list >> 16, list & 0xffff);
    (head, move |i| {
        Address::new(list >> 16, (list & 0xffff) + 4 + i * 4)
    })
}

/// `ADDTOINV`: puts an item in the first free slot.
///
/// Once past the eighth the list scrolls so the new one is on screen — the bar
/// shows eight.
///
/// Takes module memory rather than the engine because that is all it touches:
/// the inventory is a list in the game's own memory, not engine state.
pub(crate) fn add_to_inventory(mem: &mut Memory, item: i32, list: u32) -> Result<()> {
    let (head, slot) = slots(list);
    let mut i = 0;
    while i < 99 && mem.fetch(slot(i))? != 0 {
        i += 1;
    }
    if i < 99 {
        mem.store(slot(i), item as u32)?;
    }
    if i >= 8 {
        mem.store(head, i - 7)?;
    }
    Ok(())
}

/// `SUBFROMINV`: finds an item, then closes the gap by shifting the rest down.
///
/// An item that is not in the list leaves it alone. See
/// [`add_to_inventory`] for why this takes memory rather than the engine.
pub(crate) fn remove_from_inventory(mem: &mut Memory, item: i32, list: u32) -> Result<()> {
    let (_, slot) = slots(list);
    let mut i = 0;
    while i < 99 {
        let v = mem.fetch(slot(i))?;
        if v == 0 || v == item as u32 {
            break;
        }
        i += 1;
    }
    if i < 99 && mem.fetch(slot(i))? == item as u32 {
        while i < 98 && mem.fetch(slot(i))? != 0 {
            let next = mem.fetch(slot(i + 1))?;
            mem.store(slot(i), next)?;
            i += 1;
        }
    }
    Ok(())
}
