//! Setting the palette.
//!
//! One of the ten groups `plain_word` hands a word to, in the order the
//! original's own match had them. A group that does not know the word
//! answers `None` and the next one is asked.

use crate::Engine;
use crate::Result;
use crate::stack::pop1;
use motionvm_forth::Memory;

use crate::stack::pop_n;

impl Engine {
    pub(crate) fn words_palette(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        _mem: &mut Memory,
    ) -> Result<Option<()>> {
        match name {
            // --- palette ----------------------------------------------------
            // Selects the palette a later draw uses. Loading it here keeps the
            // colors right even before anything is drawn.
            "SETPAL" => {
                let id = pop1(stack, "SETPAL")?;
                if let Some(p) = self.load_palette(id) {
                    self.display.palette = p;
                }
            }
            "CUTPAL" => {
                pop1(stack, "CUTPAL")?;
                self.note_no_effect(name);
            }

            // The handler pops blue, green, red and hands them to a lookup that
            // returns a single 16-bit value: a palette index, not a packed
            // color. Six-bit components, the same scale the palettes are
            // stored in.
            "RGB->COL" => {
                let c = pop_n(stack, 3, "RGB->COL")?;
                let want = [
                    c[0].clamp(0, 63) as u8,
                    c[1].clamp(0, 63) as u8,
                    c[2].clamp(0, 63) as u8,
                ];
                let pal = &self.display.palette;
                // Nearest entry rather than an exact match: the game asks for
                // pure white, and whether the palette in force holds exactly
                // 63,63,63 is not something a caller can know.
                let best = (0..256)
                    .min_by_key(|&i| {
                        let o = i * 3;
                        want.iter()
                            .zip(&pal.raw[o..o + 3])
                            .map(|(&a, &b)| {
                                let d = a as i32 - (b & 63) as i32;
                                d * d
                            })
                            .sum::<i32>()
                    })
                    .unwrap_or(0);
                stack.push(best as i32);
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
