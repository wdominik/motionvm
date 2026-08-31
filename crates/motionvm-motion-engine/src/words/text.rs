//! Registering a font and defining a text template.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::TextTemplate;
use crate::stack::pop_n;
use crate::stack::pop1;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;

impl Engine {
    pub(crate) fn words_text(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match name {
            // --- fonts and text ---------------------------------------------
            "+FONT" => {
                let n = pop1(stack, "+FONT")?;
                stack.push(self.load_font(n).unwrap_or(-1));
            }
            // Nine arguments, with the template number on top. The static
            // count over the handler says seventeen, but the id is popped
            // first, checked against 1..9, and then one of two eight-pop
            // branches runs — so nine is what any single call takes.
            //
            // The store is a fixed array indexed by the id — `ds:0x1A0C`,
            // ten bytes an entry, on the 16-bit machine — so defining a
            // template again *replaces* it. Die Enviro-Kids greifen ein leans
            // on that: the
            // intro loads its own shadow font and defines templates 6 and
            // 2 over it (module 610), frees that font on its way out
            // (`_SHFONT @ -FONT`), and `RUN` then defines all nine
            // templates afresh over a new handle (module 100, after
            // `=>ERASE`). Appending instead left the intro's entry first
            // in line with a font that no longer existed, and every text
            // on templates 2 and 6 lost its outline for the whole game.
            "DEFTDT" => {
                let args = pop_n(stack, 9, "DEFTDT")?;
                let id = *args.last().unwrap_or(&0);
                match self.templates.iter_mut().find(|t| t.id == id) {
                    Some(t) => t.args = args,
                    None => self.templates.push(TextTemplate { id, args }),
                }
            }

            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
