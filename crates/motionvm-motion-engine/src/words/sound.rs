//! Starting and stopping a tune, and the 32-bit engine's five sample words.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::stack::{pop_n, pop1};
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;

impl Engine {
    pub(crate) fn words_sound(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        mem: &dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
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
            Word::STARTTUNE => {
                let a = pop_n(stack, 2, "STARTTUNE")?;
                let (tune, looping) = (a[0], a[1]);
                let handle = self.start_tune(tune, looping);
                stack.push(handle);
            }
            Word::ENDTUNE => {
                let handle = pop1(stack, "ENDTUNE")?;
                self.sound.playing = false;
                if let Some(music) = self.sound.sink.as_mut() {
                    music.stop(handle);
                }
            }
            // The samples — see [`crate::sample`] for the reading of the
            // five handlers. `( block loops -- handle )`: the count on top
            // (`0x6ad23` pops it first) goes to the layer's record, where the
            // mixer reads it as the data runs out.
            Word::STARTSAMPLE => {
                let a = pop_n(stack, 2, "STARTSAMPLE")?;
                let [block, loops] = [a[0], a[1]];
                stack.push(self.start_block_sample(block, loops));
            }
            // `( name$ loops -- handle )`: the string's address is resolved
            // into module memory (`0x568f0`) and the name read out of it.
            Word::TO_STARTSAMPLE => {
                let a = pop_n(stack, 2, "->STARTSAMPLE")?;
                let name = mem.read_bytes(a[0], 80).unwrap_or_default();
                let name = motionvm_motion_formats::cp437_to_string(
                    motionvm_motion_formats::nul_terminated(&name),
                );
                stack.push(self.start_file_sample(&name, a[1]));
            }
            // `( handle -- )`.
            Word::STOPSAMPLE => {
                let handle = pop1(stack, "STOPSAMPLE")?;
                self.stop_sample(handle);
            }
            // `( handle -- t | -1 )`.
            Word::Q_STIME => {
                let handle = pop1(stack, "?STIME")?;
                let t = self.sample_time(handle);
                stack.push(t);
            }
            // `( f -- )`: zero ducks, anything else restores (`0x6b39b`).
            Word::MUSVOLUME => {
                let f = pop1(stack, "MUSVOLUME")?;
                self.music_volume(f != 0);
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
