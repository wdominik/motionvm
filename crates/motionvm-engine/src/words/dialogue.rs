//! Queueing a change for a later conversation.
//!
//! One of the groups `plain_word` hands a word to. A group that does not
//! know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::order::{Rules, at};
use crate::stack::pop_n;
use motionvm_forth::AddressSpace;
use motionvm_forth::{Error, Result};

impl Engine {
    pub(crate) fn words_dialogue(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut dyn AddressSpace,
        rules: Rules,
    ) -> Result<Option<()>> {
        match name {
            // `ADDMESSPIPE ( name1 name2 art a b _ORDER -- )`, `ENGINE.EXE`
            // 0x7f1dd and `ENVIRO.EXE` file `0x13e41`.
            //
            // Puts a change on the queue a *later* conversation works off —
            // the same records the branch nodes write and the conversation
            // drains: two nine-byte names at +0 and +9, then the kind and
            // two values, a cell each, so a record is 0x22 bytes on the
            // 32-bit machine and 0x1a on the 16-bit. The two names arrive as
            // the addresses `_PutStringAdr` pushed, straight out of the word
            // that ran; they are copied, not referenced, which is why their
            // padding in the module image does not matter.
            //
            // A full queue makes the 32-bit engine complain and wait for a
            // key; the 16-bit one never reads the room cell. A silent
            // overwrite of the record past the end would be worse than
            // stopping, so the 32-bit rule stops and says where.
            "ADDMESSPIPE" => {
                let a = pop_n(stack, 6, "ADDMESSPIPE")?;
                let (name1, name2) = (a[0], a[1]);
                let (kind, first, second) = (a[2], a[3], a[4]);
                let order = a[5] as u32;
                let cell = mem.cell_size();
                let (queue, room, used) = (
                    mem.fetch_cell(at(mem, order, 0x1c0))?,
                    mem.fetch_cell(at(mem, order, 0x1c4))?,
                    mem.fetch_cell(at(mem, order, 0x1c8))?,
                );
                if rules.change_room_checked && room <= used {
                    return Err(Error::Unread {
                        what: format!(
                            "ADDMESSPIPE: the queue holds {room} and already has {used}; \
                             the original complains here and waits for a key"
                        ),
                        at: "0x7f26d",
                    });
                }
                let record = 0x12 + 4 * cell;
                let entry = mem.offset(queue, used * record);
                for (from, to) in [(name1, 0), (name2, 9)] {
                    for i in 0..9 {
                        let b = mem.fetch_byte(mem.offset(from, i))?;
                        mem.write_bytes(mem.offset(entry, to + i), &[b])?;
                        if b == 0 {
                            break;
                        }
                    }
                }
                mem.store_cell(mem.offset(entry, 0x12), kind)?;
                mem.store_cell(mem.offset(entry, 0x12 + cell), first)?;
                mem.store_cell(mem.offset(entry, 0x12 + 2 * cell), second)?;
                mem.store_cell(at(mem, order, 0x1c8), used + 1)?;
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
