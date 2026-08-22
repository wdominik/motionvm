//! Screen activation, redraw, and the two music words.
//!
//! One of the ten groups `plain_word` hands a word to, in the order the
//! original's own match had them. A group that does not know the word
//! answers `None` and the next one is asked.

use crate::Engine;
use crate::Result;
use motionvm_forth::Memory;

impl Engine {
    pub(crate) fn words_redraw(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        _mem: &mut Memory,
    ) -> Result<Option<()>> {
        match name {
            // --- screen redraw ----------------------------------------------
            // The engine double-buffers; a still frame is composed on demand,
            // so these mark the screen rather than doing work.
            "DRAWSCR" | "FRESHSCREEN" => self.dirty = true,
            "SCRACT" => {
                if let Some(s) = self.display.current_mut() {
                    s.active = true;
                }
            }
            "SCRINACT" => {
                if let Some(s) = self.display.current_mut() {
                    s.active = false;
                }
            }
            // `0x74801` calls the same fill `FADEOUT` uses. The surface
            // persists, so what was saved from under a descriptor is a copy of
            // a picture that has just been thrown away.
            "ERASESCR" => {
                let Some(s) = self.display.current_mut() else {
                    return Ok(Some(()));
                };
                s.buffer.fill(0);
                let handle = s.handle;
                self.forget_rebuilds(handle);
            }
            "REMSCR" => {
                if let Some(h) = self.display.current {
                    self.forget_rebuilds(h);
                    self.descriptors.retain(|d| d.screen != h);
                    self.display.screens.retain(|s| s.handle != h);
                    self.display.current = self.display.screens.last().map(|s| s.handle);
                }
            }
            "SETBUF" => {
                self.inert(stack, 3, "SETBUF")?;
            }
            "RESETBUF" => self.note_no_effect(name),
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
