//! Sprite mirroring, the crunch words, and whether a slot exists.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::stack::pop1;
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;

use crate::stack::pop_n;
use motionvm_motion_forth::cell;

impl Engine {
    pub(crate) fn words_resources(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
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
            Word::XGFXVFLIP | Word::GFXVFLIP => {
                let (src, dst, count) = if word == Word::GFXVFLIP {
                    let a = pop_n(stack, 2, "GFXVFLIP")?;
                    (a[0], a[1], 1)
                } else {
                    let a = pop_n(stack, 3, "XGFXVFLIP")?;
                    (a[0], a[1], a[2].max(0))
                };
                for i in 0..count {
                    let from = cell::unsigned((src + i).max(0));
                    let Some(sprite) = self.sprite(from) else {
                        continue;
                    };
                    let mut flipped = sprite.clone();
                    let w = usize::from(sprite.width);
                    for (row, out) in sprite
                        .pixels
                        .chunks_exact(w)
                        .zip(flipped.pixels.chunks_exact_mut(w))
                    {
                        for (x, p) in row.iter().enumerate() {
                            out[w - 1 - x] = *p;
                        }
                    }
                    let to = cell::unsigned((dst + i).max(0));
                    self.scene.sprites.insert(to, flipped);
                    // Kept so a savegame can put it back. The mirrored sprite
                    // goes into the pool under an id no resource file holds, so
                    // nothing can load it again — only doing the flip once more
                    // can. Recorded as a recipe rather than as pixels: the
                    // source is a real resource and always available.
                    self.persistence
                        .flips
                        .retain(|(_, existing)| *existing != to);
                    self.persistence.flips.push((from, to));
                }
            }
            Word::GFXCRUNCH => {
                self.inert(stack, 2, Word::GFXCRUNCH)?;
            }
            Word::XGFXCRUNCH => {
                self.inert(stack, 3, Word::XGFXCRUNCH)?;
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
            Word::EXIST => {
                let n = pop1(stack, "EXIST")?;
                let found = self.save_path(n, "blk").is_some_and(|p| p.exists());
                stack.push(if found { -1 } else { 0 });
            }
            // The status hints, which are inert here but not free: each pops
            // its own number of arguments, and the counts are not uniform.
            // The arity comes off the word rather than out of a table searched
            // by name, so the arm and the count cannot disagree — and one
            // lookup answers both whether this is a hint and how deep it
            // reaches.
            _ => {
                let Some(arity) = word.status_hint() else {
                    return Ok(None);
                };
                pop_n(stack, arity, "resource status hint")?;
                self.note_unhandled(word, None);
            }
        }
        Ok(Some(()))
    }
}
