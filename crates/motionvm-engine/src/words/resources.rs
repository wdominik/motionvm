//! Module residency and the four savegame words.
//!
//! One of the ten groups `plain_word` hands a word to, in the order the
//! original's own match had them. A group that does not know the word
//! answers `None` and the next one is asked.

use crate::Engine;
use crate::stack::pop1;
use motionvm_forth::AddressSpace;
use motionvm_forth::Result;

use crate::stack::pop_n;

use crate::STATUS_HINTS;

impl Engine {
    pub(crate) fn words_resources(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match name {
            // Installs mirror aliases for a run of sprites: `1880 1889 9`
            // makes 1889..1897 the mirrors of 1880..1888. The original
            // (`05f1:221b`; `GFXVFLIP` `05f1:21f2` is the count-1 form)
            // copies no pixels — it sets the flag byte `ds:[0x2694+dst]`
            // to 0x80 and the source id in the table behind `ds:[0x764E]`,
            // and the loader mirrors on first use. The copy here is eager
            // instead, which comes out the same because `Engine::sprite`
            // loads the source from the container on the spot and nothing
            // evicts the pool; the recipe below keeps savegames honest.
            //
            // The axis is the one thing not measured. Mirroring left to
            // right is what a walk cycle and the intro's card flip need,
            // and what "vertikal spiegeln" means in German — about the
            // vertical axis. If a character ever faces the wrong way,
            // this is the line to turn.
            "XGFXVFLIP" | "GFXVFLIP" => {
                let (src, dst, count) = if name == "GFXVFLIP" {
                    let a = pop_n(stack, 2, "GFXVFLIP")?;
                    (a[0], a[1], 1)
                } else {
                    let a = pop_n(stack, 3, "XGFXVFLIP")?;
                    (a[0], a[1], a[2].max(0))
                };
                for i in 0..count {
                    let from = (src + i).max(0) as u32;
                    let Some(sprite) = self.sprite(from) else {
                        continue;
                    };
                    let mut flipped = sprite.clone();
                    let w = sprite.width as usize;
                    for (row, out) in sprite
                        .pixels
                        .chunks_exact(w)
                        .zip(flipped.pixels.chunks_exact_mut(w))
                    {
                        for (x, p) in row.iter().enumerate() {
                            out[w - 1 - x] = *p;
                        }
                    }
                    let to = (dst + i).max(0) as u32;
                    self.sprites.insert(to, flipped);
                    // Kept so a savegame can put it back. The mirrored sprite
                    // goes into the pool under an id no resource file holds, so
                    // nothing can load it again — only doing the flip once more
                    // can. Recorded as a recipe rather than as pixels: the
                    // source is a real resource and always available.
                    self.flips.retain(|(_, existing)| *existing != to);
                    self.flips.push((from, to));
                }
            }
            "GFXCRUNCH" => {
                self.inert(stack, 2, "GFXCRUNCH")?;
            }
            "XGFXCRUNCH" => {
                self.inert(stack, 3, "XGFXCRUNCH")?;
            }
            // Whether a savegame slot is taken. The name comes from the
            // template at 0xd4872, "#F0R3i.blk" — three digits, zero-padded —
            // so the `701 … 705` that `SHOW_FILES` walks are the files 701.blk
            // to 705.blk.
            //
            // Unlike `=>EXIST`, which asks the resource catalog, this is a
            // plain file test: 0x66b0e calls the runtime's `exists` and nothing
            // else. It looks in the save directory rather than in the game
            // data, which is where the original would have put the files and
            // where they must not go.
            //
            // The answer is -1, not 1: 0x66b18 stores 0xFFFFFFFF on the found
            // side and 0x66b77 stores 0 on the other. Nothing in the game can
            // tell the difference, because `SHOW_FILES` only asks `IF` — but a
            // word that was read has no business guessing.
            "EXIST" => {
                let n = pop1(stack, "EXIST")?;
                let found = self.save_path(n, "blk").is_some_and(|p| p.exists());
                stack.push(if found { -1 } else { 0 });
            }
            // One lookup, not two: the arity comes back from the same search
            // that decided the arm applies, so there is no `expect` here for the
            // two to disagree about.
            _ if STATUS_HINTS.iter().any(|(n, _)| *n == name) => {
                let (_, arity) = STATUS_HINTS
                    .iter()
                    .find(|(n, _)| *n == name)
                    .copied()
                    .unwrap_or((name, 0));
                pop_n(stack, arity, "resource status hint")?;
                self.note_unhandled(name.to_string());
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
