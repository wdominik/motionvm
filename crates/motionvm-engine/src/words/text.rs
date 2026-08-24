//! Registering a font and defining a text template.
//!
//! One of the ten groups `plain_word` hands a word to, in the order the
//! original's own match had them. A group that does not know the word
//! answers `None` and the next one is asked.

use crate::Engine;
use crate::Result;
use crate::TextTemplate;
use crate::stack::pop_n;
use crate::stack::pop1;
use motionvm_forth::AddressSpace;

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
            "DEFTDT" => {
                let args = pop_n(stack, 9, "DEFTDT")?;
                let id = *args.last().unwrap_or(&0);
                self.templates.push(TextTemplate { id, args });
            }

            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
