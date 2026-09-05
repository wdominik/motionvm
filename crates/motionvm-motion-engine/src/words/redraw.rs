//! Screen activation, redraw, and the two music words.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;

impl Engine {
    pub(crate) fn words_redraw(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
            // --- screen redraw ----------------------------------------------
            // The engine double-buffers; a still frame is composed on demand,
            // so these mark the screen rather than doing work.
            Word::DRAWSCR | Word::FRESHSCREEN => self.dirty = true,
            Word::SCRACT => {
                if let Some(s) = self.display.current_mut() {
                    s.active = true;
                }
            }
            Word::SCRINACT => {
                if let Some(s) = self.display.current_mut() {
                    s.active = false;
                }
            }
            // `0x74801` calls the same fill `FADEOUT` uses. The surface
            // persists, so what was saved from under a descriptor is a copy of
            // a picture that has just been thrown away.
            Word::ERASESCR => {
                let Some(s) = self.display.current_mut() else {
                    return Ok(Some(()));
                };
                s.buffer.fill(0);
                let handle = s.handle;
                self.forget_rebuilds(handle);
            }
            Word::REMSCR => {
                if let Some(h) = self.display.current {
                    self.forget_rebuilds(h);
                    self.scene.descriptors.retain(|d| d.screen != h);
                    self.display.screens.retain(|s| s.handle != h);
                    self.display.current = self.display.screens.last().map(|s| s.handle);
                }
            }
            Word::SETBUF => {
                self.inert(stack, 3, Word::SETBUF)?;
            }
            Word::RESETBUF => self.note_no_effect(word),
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
