//! Starting and stopping a tune.
//!
//! One of the groups `plain_word` hands a word to. A group that does not
//! know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::stack::{pop_n, pop1};
use motionvm_forth::AddressSpace;
use motionvm_forth::Result;

impl Engine {
    pub(crate) fn words_sound(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match name {
            // The music. `0x7F98D` takes the loop flag, `0x7F99A` the tune
            // number, and `0x7FB24` is the **only** exit — one `push()` of a
            // local that is still zero unless a sequence really started. So
            // exactly one value comes back, and when there is no sound it is 0.
            //
            // That last part means our silent case is faithful by
            // construction: the original checks its "sound is up" flag at
            // `0x7F9A9` and jumps straight to the same exit.
            //
            // (Counting pushes statically gives two, which is the upper bound
            // a branchy handler always yields: both arms carry a `push()` and
            // only one arm runs. Answering with two leaves a stray zero on the
            // stack at every location change.)
            "STARTTUNE" => {
                let a = pop_n(stack, 2, "STARTTUNE")?;
                let (tune, looping) = (a[0], a[1]);
                let handle = self.start_tune(tune, looping);
                stack.push(handle);
            }
            "ENDTUNE" => {
                let handle = pop1(stack, "ENDTUNE")?;
                if let Some(music) = self.music.as_mut() {
                    music.stop(handle);
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
