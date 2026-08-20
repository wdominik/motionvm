//! Module residency and the four savegame words.
//!
//! One of the ten groups `plain_word` hands a word to, in the order the
//! original's own match had them. A group that does not know the word
//! answers `None` and the next one is asked.

use crate::Engine;
use crate::Error;
use crate::Result;
use crate::save;
use crate::stack::pop1;
use motionvm_forth::Memory;

use crate::stack::pop_n;

use crate::STATUS_HINTS;

impl Engine {
    pub(crate) fn words_resources(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut Memory,
    ) -> Result<Option<()>> {
        match name {
            // --- resources and modules --------------------------------------
            // One handler per word, and in particular the `…STAT-` range
            // markers are handled by the `STATUS_HINTS` arm further down and
            // nowhere else. A second arm here would come first and win, and it
            // would assert them onto the `NO_EFFECT` list — which is not where
            // they live, so each would fault a debug build the moment it ran.
            // `4:START` runs `XGFXSTAT-` six times and each of its four
            // siblings at least once, all before any test that enters through
            // `INCLLOC` gets far enough to notice.
            //
            // Loads `%03d.SCR` into the first free descriptor slot, and frees a
            // slot again. Every module is already resident here, so neither
            // moves any memory — what they do keep is the *bookkeeping*, and
            // that is not decoration: `=>PUTAS` writes the modules these two
            // leave marked, in the order they leave them in, and a savegame
            // that carried the other seventy-odd modules would restore state
            // the original discards at every change of location.
            //
            // `=>GET` leaves nothing on the stack: the handler has no call to
            // the push helper anywhere in its body. The arity table's single
            // push came from the scan running past the function's end, which is
            // a reminder that its boundaries are inferred from the next
            // handler's address, not from the code.
            //
            // Neither reloads a module's bytes. In the original a location's
            // modules come back from the resource file pristine, so their
            // variables reset on every re-entry; here they persist. That is a
            // difference of its own, older than saving and not touched by it —
            // for the location a savegame is *in* the end state is the same,
            // because `=>GETAS` overwrites all of it anyway.
            "=>GET" => {
                let n = pop1(stack, "=>GET")?;
                self.mark_resident(n.max(0) as u32);
            }
            "=>ERASE" => {
                let n = pop1(stack, "=>ERASE")?;
                self.mark_gone(n.max(0) as u32);
            }
            // The bulk of a savegame: every resident module's memory, written
            // in one go and put back in one go. Between them they carry the
            // whole of the player's progress, because script variables *are*
            // module memory — the inventory in module 2, the story flags in 11,
            // the person records in the location's own modules.
            //
            // `=>PUTAS` (0x6559e) walks descriptor slots 1 to 31 and writes
            // each occupied one; `=>GETAS` (0x657a4) walks the same slots and
            // copies back. Which modules those are, and in which order, is
            // [`Engine::slots`]. The layout is ours and the reasons are in
            // `save.rs`, but the set and the moment are the original's.
            "=>PUTAS" => {
                let id = pop1(stack, "=>PUTAS")?;
                let Some(path) = self.save_path(id, "FRZ") else {
                    return Err(Error::NoSaveDir {
                        word: "=>PUTAS",
                        id,
                    });
                };
                let resident = self.resident();
                // The original complains "Kein einziges Modul geladen!" and
                // gives up (error 0x19 at 0x655e8). Reaching this with an empty
                // table here means something else: the run never went through
                // `4:START`, so the bookkeeping was never filled in and the
                // savegame would be missing everything.
                if resident.is_empty() {
                    return Err(Error::Unsupported(format!(
                        "=>PUTAS {id}: no module is marked resident — this run did not start \
                         through 4:START, so there is nothing to write"
                    )));
                }
                let mut images = Vec::with_capacity(resident.len());
                for module in resident {
                    let Some(m) = mem.module(module) else {
                        return Err(Error::Unsupported(format!(
                            "=>PUTAS {id}: module {module} is marked resident but not loaded"
                        )));
                    };
                    images.push(save::ModuleImage {
                        module,
                        cells: m.cells().to_vec(),
                    });
                }
                std::fs::write(&path, save::write_frz(&images)).map_err(|e| Error::Io {
                    word: "=>PUTAS",
                    path: path.clone(),
                    source: e,
                })?;
            }
            "=>GETAS" => {
                let id = pop1(stack, "=>GETAS")?;
                let Some(path) = self.save_path(id, "FRZ") else {
                    return Err(Error::NoSaveDir {
                        word: "=>GETAS",
                        id,
                    });
                };
                let bytes = std::fs::read(&path).map_err(|e| Error::Io {
                    word: "=>GETAS",
                    path: path.clone(),
                    source: e,
                })?;
                let what = format!("=>GETAS {id}");
                let images = save::read_frz(&bytes, &what).map_err(Error::Unsupported)?;
                // Everything is checked before anything is applied. Half a
                // savegame in memory is the one outcome with no way back and no
                // trail leading to it.
                for image in &images {
                    match mem.module(image.module) {
                        Some(m) if m.cells().len() == image.cells.len() => {}
                        Some(m) => {
                            return Err(Error::Unsupported(format!(
                                "{what}: module {} is {} cells here and {} in the savegame",
                                image.module,
                                m.cells().len(),
                                image.cells.len()
                            )));
                        }
                        None => {
                            return Err(Error::Unsupported(format!(
                                "{what}: module {} is in the savegame but not loaded",
                                image.module
                            )));
                        }
                    }
                }
                for image in &images {
                    let m = mem.module_mut(image.module).expect("checked just above");
                    m.restore(&image.cells)?;
                }
            }
            // The other half of a savegame, and the reason the module image on
            // its own would not be enough: the handles the scripts keep in that
            // memory point at descriptors and screens, which live out here.
            //
            // The original dumps its whole descriptor tree (0x6dd87) and reads
            // it back node by node (0x6e355), rebuilding every pointer as it
            // goes. What that keeps, and what it leaves for the scripts to
            // re-establish, is in `save.rs`.
            "PUTANIM" => {
                let id = pop1(stack, "PUTANIM")?;
                let Some(path) = self.save_path(id, "anm") else {
                    return Err(Error::NoSaveDir {
                        word: "PUTANIM",
                        id,
                    });
                };
                let anim = self.snapshot();
                std::fs::write(&path, save::write_anm(&anim)).map_err(|e| Error::Io {
                    word: "PUTANIM",
                    path: path.clone(),
                    source: e,
                })?;
            }
            "GETANIM" => {
                let id = pop1(stack, "GETANIM")?;
                let Some(path) = self.save_path(id, "anm") else {
                    return Err(Error::NoSaveDir {
                        word: "GETANIM",
                        id,
                    });
                };
                // The original checks this one open, and only this one
                // (0x6e37e): a missing `.anm` makes it return without touching
                // anything. That is because `START` may reach it with no
                // savegame present. Here the file is named in the error
                // instead — the load path only gets here for a slot `EXIST`
                // has already said yes to, so a missing file is a fault.
                let bytes = std::fs::read(&path).map_err(|e| Error::Io {
                    word: "GETANIM",
                    path: path.clone(),
                    source: e,
                })?;
                let anim =
                    save::read_anm(&bytes, &format!("GETANIM {id}")).map_err(Error::Unsupported)?;
                self.restore(anim).map_err(Error::Unsupported)?;
            }

            // Makes mirrored copies of a run of sprites: `1880 1889 9` fills
            // 1889..1897 from 1880..1888. That it really generates them is
            // settled by the destinations — `821 881 3` writes 881..883, and
            // those are not in the resources at all.
            //
            // The axis is the one thing not measured. Mirroring left to right
            // is what a walk cycle needs, and what "vertikal spiegeln" means in
            // German — about the vertical axis. If a character ever faces the
            // wrong way, this is the line to turn.
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
