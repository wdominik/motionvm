//! Module residency and the four savegame words, on either machine.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked. The 16-bit game saves the same
//! three files through the same words in the same order — its `DOINVSAVE`
//! (module 650) runs `PUT`, `PUTANIM`, `=>PUTAS`, its load path in `CTRL`
//! `GET`, `INCLLOC`, `GETANIM`, `=>GETAS` — so the group serves both, with
//! the module images taken through [`AddressSpace`] and laid out per game
//! ([`motionvm_motion_formats::Generation`]).

use crate::Engine;
use crate::save;
use crate::stack::pop1;
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Error;
use motionvm_motion_forth::Result;

impl Engine {
    pub(crate) fn words_saves(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
            // --- resources and modules --------------------------------------
            // One handler per word, and in particular the `…STAT-` range
            // markers are handled by the `STATUS_HINTS` arm further down and
            // nowhere else. A second arm here would come first and win, and it
            // would assert them onto the `NO_EFFECT` list — which is not where
            // they live, so each would fault a debug build the moment it ran.
            // `4:START` runs `XGFXSTAT-` six times and each of its four
            // siblings at least once, all before any test that enters through
            // `INCLLOC` gets far enough to notice.
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
            // `save/`, but the set and the moment are the original's.
            Word::RES_PUTAS => {
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
                    return Err(Error::Savegame {
                        what: format!(
                            "=>PUTAS {id}: no module is marked resident — this run did not \
                             start through 4:START, so there is nothing to write"
                        ),
                    });
                }
                let mut images = Vec::with_capacity(resident.len());
                for module in resident {
                    let Some(bytes) = mem.module_image(module) else {
                        return Err(Error::Savegame {
                            what: format!(
                                "=>PUTAS {id}: module {module} is marked resident but not loaded"
                            ),
                        });
                    };
                    images.push(save::ModuleImage { module, bytes });
                }
                let layout = self.profile.save_layout;
                save::write_atomically(&path, &save::write_frz(&images, layout, self.save_slug()))
                    .map_err(|e| Error::Io {
                        word: "=>PUTAS",
                        path: path.clone(),
                        source: e,
                    })?;
            }
            Word::RES_GETAS => {
                let id = pop1(stack, "=>GETAS")?;
                let bytes = self.read_slot("=>GETAS", id, "FRZ")?;
                let what = format!("=>GETAS {id}");
                let images =
                    save::read_frz(&bytes, &what, self.profile.save_layout, self.save_slug())
                        .map_err(|e| Error::Savegame {
                            what: e.to_string(),
                        })?;
                // Everything is checked before anything is applied. Half a
                // savegame in memory is the one outcome with no way back and no
                // trail leading to it.
                for image in &images {
                    match mem.module_image(image.module) {
                        Some(here) if here.len() == image.bytes.len() => {}
                        Some(here) => {
                            return Err(Error::Savegame {
                                what: format!(
                                    "{what}: module {} is {} bytes here and {} in the savegame",
                                    image.module,
                                    here.len(),
                                    image.bytes.len()
                                ),
                            });
                        }
                        None => {
                            return Err(Error::Savegame {
                                what: format!(
                                    "{what}: module {} is in the savegame but not loaded",
                                    image.module
                                ),
                            });
                        }
                    }
                }
                for image in &images {
                    mem.restore_module(image.module, &image.bytes)?;
                }
            }
            // The other half of a savegame, and the reason the module image on
            // its own would not be enough: the handles the scripts keep in that
            // memory point at descriptors and screens, which live out here.
            //
            // The original dumps its whole descriptor tree (0x6dd87) and reads
            // it back node by node (0x6e355), rebuilding every pointer as it
            // goes. What that keeps, and what it leaves for the scripts to
            // re-establish, is in `save/`.
            Word::PUTANIM => {
                let id = pop1(stack, "PUTANIM")?;
                let Some(path) = self.save_path(id, "anm") else {
                    return Err(Error::NoSaveDir {
                        word: "PUTANIM",
                        id,
                    });
                };
                let anim = self.snapshot();
                let layout = self.profile.save_layout;
                save::write_atomically(&path, &save::write_anm(&anim, layout, self.save_slug()))
                    .map_err(|e| Error::Io {
                        word: "PUTANIM",
                        path: path.clone(),
                        source: e,
                    })?;
            }
            Word::GETANIM => {
                let id = pop1(stack, "GETANIM")?;
                // The original checks this one open, and only this one
                // (0x6e37e): a missing `.anm` makes it return without touching
                // anything. That is because `START` may reach it with no
                // savegame present. Here it is a fault instead — the load path
                // only gets here for a slot `EXIST` has already said yes to —
                // and one the reader below names as an incomplete slot.
                let bytes = self.read_slot("GETANIM", id, "anm")?;
                let anim = save::read_anm(
                    &bytes,
                    &format!("GETANIM {id}"),
                    self.profile.save_layout,
                    self.save_slug(),
                )
                .map_err(|e| Error::Savegame {
                    what: e.to_string(),
                })?;
                self.restore(anim)
                    .map_err(|what| Error::Savegame { what })?;
            }

            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
