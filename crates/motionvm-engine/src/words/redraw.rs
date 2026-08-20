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
            "ERASESCR" => {
                if let Some(s) = self.display.current_mut() {
                    s.buffer.fill(0);
                }
            }
            "REMSCR" => {
                if let Some(h) = self.display.current {
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
