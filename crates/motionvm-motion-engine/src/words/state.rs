//! Video mode, the subsystem flags, and the two block-file words.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::stack::pop_n;
use crate::stack::pop1;
use crate::words::Word;
use motionvm_motion_forth::Address;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Error;
use motionvm_motion_forth::Result;
use motionvm_motion_forth::cell;

use crate::video::{MODE_320X200X256, MODE_640X480X32K, MODE_640X480X256};

impl Engine {
    pub(crate) fn words_state(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
            // --- video mode and subsystem state -----------------------------
            Word::MODE_640X480X256 => stack.push(MODE_640X480X256),
            Word::MODE_320X200X256 => stack.push(MODE_320X200X256),
            Word::MODE_640X480X32K => stack.push(MODE_640X480X32K),
            // `SETRES` selects, `TOGFX` enters — and sizes the display to the
            // mode, which is where a mode this renderer does not draw is
            // refused. See `video.rs` for the reading.
            Word::SETRES => self.select_mode(pop1(stack, "SETRES")?),
            Word::TOGFX => self.enter_graphics()?,
            Word::GFXTO => self.leave_graphics(),
            // No sound, matching how the reference runs; `HICOLOR` answers for
            // the selected mode, as the original does (`0xd6600`).
            Word::Q_SOUND => stack.push(0),
            Word::HICOLOR => stack.push(i32::from(self.hicolor())),
            Word::RESETANIM | Word::RESETFONT => self.note_no_effect(word),
            // The resource loader, and the reason nothing was ever drawn while
            // it was a stub: `STARTUP` fills the location jump table with
            // `200 _LOCTABLE 99 GET`, and `INCLLOC` loads a location's routes,
            // click areas and items the same way. It copies a BLOCK resource
            // into module memory — block 99 is exactly the 200 bytes STARTUP
            // asks for.
            Word::GET => {
                let a = pop_n(stack, 3, "GET")?;
                let (size, addr, id) = (cell::at(a[0]).unwrap_or(0), a[1], a[2]);
                // A loose file in the save directory answers before the banks
                // do. That is how the original finds what `PUT` just wrote:
                // 0x668aa registers the id in the block catalog as present on
                // disk (0x556c1) and drops the cached copy, so the next `GET`
                // resolves to the file. Only the five save ids are ever written,
                // and none of them exists in any container, so which source wins
                // is unobservable in the shipped game — but taking the file
                // first is what makes `PUT` and `GET` a round trip for *any*
                // id, which is a property a test can hold on to.
                let loose = self
                    .save_path(id, "blk")
                    .and_then(|p| std::fs::read(p).ok());
                let block = match loose {
                    Some(data) => Some(data),
                    None => self.resources.as_ref().and_then(|r| r.block(id)),
                };
                match block.as_deref() {
                    Some(data) => {
                        // A size of zero means the whole resource, not nothing:
                        // 0x66a8a tests the argument and, when it is zero, asks
                        // the resource for its own length through 0x207fe
                        // before the copy at 0x66ac4. `->DIAL` in module 5
                        // relies on it — `0 _DIALFIELD ROT GET` is how every
                        // conversation's fields are loaded — so reading it as
                        // "copy zero bytes" left every dialogue empty.
                        let n = if size == 0 {
                            data.len()
                        } else {
                            size.min(data.len())
                        };
                        mem.write_bytes(addr, &data[..n])?;
                    }
                    // A missing resource is not a no-op: something downstream
                    // will read the memory that should have been filled.
                    None => {
                        return Err(Error::MissingResource {
                            kind: "block",
                            id,
                            word: "GET",
                            at: None,
                        });
                    }
                }
            }
            // The other half of the pair, and the first of a savegame's three
            // artifacts. `ICTRL` writes exactly one thing by hand when the
            // player picks a slot — `4 _AKTLT (701+slot) PUT`, four bytes
            // holding the location number — and it exists so that loading knows
            // which location to enter before the module image goes back on top.
            //
            // The handler at 0x668aa takes the same three arguments in the same
            // order as `GET`, resolves the packed address into module memory,
            // and writes from there. It also rewrites `rsc.inf`, a memory image
            // of the resource manager full of live heap pointers; that is a DOS
            // cache and has no counterpart here.
            Word::PUT => {
                let a = pop_n(stack, 3, "PUT")?;
                let (size, addr, id) = (cell::at(a[0]).unwrap_or(0), a[1], a[2]);
                let Some(path) = self.save_path(id, "blk") else {
                    return Err(Error::NoSaveDir { word: "PUT", id });
                };
                let bytes = mem.read_bytes(addr, size)?;
                crate::save::write_atomically(&path, &bytes).map_err(|e| Error::Io {
                    word: "PUT",
                    path: path.clone(),
                    source: e,
                })?;
            }
            // Hands the frame loop the word to run. It does not loop here: the
            // caller owns the clock, and the original's loop is this word being
            // called again and again. The 32-bit kernel's word, taking a
            // packed address; the 16-bit kernel installs its controller with
            // `SCRCTRL` and a word id instead.
            Word::CTRL => {
                let raw = cell::unsigned(pop1(stack, "CTRL")?);
                self.controller = Some(Address::new(raw >> 16, raw & 0xffff));
            }
            Word::ERRORLEVEL => {
                pop1(stack, "ERRORLEVEL")?;
                self.note_no_effect(word);
            }
            Word::SPEEDMODE => {
                pop1(stack, "SPEEDMODE")?;
                self.note_no_effect(word);
            }
            Word::DREQUEST => {
                self.inert(stack, 3, Word::DREQUEST)?;
            }
            Word::RESETTI => self.note_no_effect(word),
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
