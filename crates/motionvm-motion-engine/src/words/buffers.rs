//! The 16-bit kernel's buffer words: `BUFON`, `SETBUF`, `SDBUF`, `KILLNBUF`,
//! `RESETBUF`.
//!
//! Asked before every shared group on the 16-bit machine, because the 32-bit
//! path answers `SETBUF` and `RESETBUF` as inert — which they are for that
//! game — and `SDBUF` as a by-name descriptor field. Here they keep the
//! buffers of [`crate::buffer::Buffers`]; what the kernel does with a buffer
//! is unread, see there.

use crate::Engine;
use crate::stack::pop_n;
use crate::stack::pop1;
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;

impl Engine {
    pub(crate) fn words_buffers(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
            // `RUN`: `NEWANIM BUFON`, once, before the first picture.
            Word::BUFON => self.buffers.on = true,
            // `( w h id -- )`: `320 200 1 SETBUF` in the intro, `100 140 ?LPB
            // SETBUF` in `NEWPERS`, `0 0 1 SETBUF` on the way out.
            Word::SETBUF_16 => {
                let a = pop_n(stack, 3, "SETBUF")?;
                self.buffers.set(a[2], a[0], a[1]);
            }
            // `( id -- )`: the current descriptor draws through buffer `id`.
            Word::SDBUF_16 => {
                let id = pop1(stack, "SDBUF")?;
                if let Some(d) = self.descriptor_mut() {
                    d.buffer = Some(id);
                }
            }
            Word::KILLNBUF => {
                let a = pop_n(stack, 2, "KILLNBUF")?;
                self.buffers.kill(a[0], a[1]);
            }
            Word::RESETBUF_16 => self.buffers.reset(),
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
