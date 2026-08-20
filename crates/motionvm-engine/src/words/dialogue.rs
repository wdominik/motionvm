//! Queueing a change for a later conversation.
//!
//! One of the groups `plain_word` hands a word to. A group that does not
//! know the word answers `None` and the next one is asked.

use crate::stack::pop_n;
use crate::{Engine, Error, Result};
use motionvm_forth::Memory;

impl Engine {
    pub(crate) fn words_dialogue(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut Memory,
    ) -> Result<Option<()>> {
        match name {
            // `ADDMESSPIPE ( name1 name2 art a b _ORDER -- )`, 0x7f1dd.
            //
            // Puts a change on the queue a *later* conversation works off —
            // the same 0x22-byte records the branch nodes write and
            // `dialogue_changes` drains, with the conversation's name at +0,
            // the answer's at +9 and the action at +0x12. The two names arrive
            // as the addresses `_PutStringAdr` pushed, straight out of the
            // word that ran; they are copied, not referenced, which is why
            // their padding in the module image does not matter.
            //
            // A full queue makes the original complain and wait for a key. A
            // silent overwrite of the record past the end would be worse than
            // stopping, so it stops and says where.
            "ADDMESSPIPE" => {
                let a = pop_n(stack, 6, "ADDMESSPIPE")?;
                let (name1, name2) = (a[0] as u32, a[1] as u32);
                let (kind, first, second) = (a[2], a[3], a[4]);
                let order = a[5] as u32;
                let o = |off: u32| Self::field(order, off);
                let (queue, room, used) = (
                    mem.fetch(o(0x1c0))?,
                    mem.fetch(o(0x1c4))? as i32,
                    mem.fetch(o(0x1c8))? as i32,
                );
                if room <= used {
                    return Err(Error::Unread {
                        what: format!(
                            "ADDMESSPIPE: the queue holds {room} and already has {used}; \
                             the original complains here and waits for a key"
                        ),
                        at: "0x7f26d",
                    });
                }
                let entry = Self::field(queue, used as u32 * 0x22).0;
                for (from, at) in [(name1, 0u32), (name2, 9)] {
                    for i in 0..9u32 {
                        let b = mem.fetch_byte(Self::field(from, i))?;
                        mem.store_byte(Self::field(entry, at + i), b)?;
                        if b == 0 {
                            break;
                        }
                    }
                }
                mem.store(Self::field(entry, 0x12), kind as u32)?;
                mem.store(Self::field(entry, 0x16), first as u32)?;
                mem.store(Self::field(entry, 0x1a), second as u32)?;
                mem.store(o(0x1c8), used as u32 + 1)?;
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
