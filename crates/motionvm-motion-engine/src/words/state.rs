//! Video mode, the subsystem flags, and the two block-file words.
//!
//! One of the groups `plain_word` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::MODE_320X200X256;
use crate::MODE_640X480X32K;
use crate::MODE_640X480X256;
use crate::stack::pop_n;
use crate::stack::pop1;
use motionvm_motion_forth::Address;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Error;
use motionvm_motion_forth::Result;

impl Engine {
    pub(crate) fn words_state(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match name {
            // --- video mode and subsystem state -----------------------------
            "640x480x256" => stack.push(MODE_640X480X256),
            "320x200x256" => stack.push(MODE_320X200X256),
            "640x480x32K" => stack.push(MODE_640X480X32K),
            // The mode is checked, not just kept: this renderer draws 640x480
            // indexed and nothing else, so any other mode would quietly give a
            // wrong picture instead of an honest stop.
            "SETRES" => {
                let mode = pop1(stack, "SETRES")?;
                if mode != MODE_640X480X256 {
                    return Err(Error::Unsupported(format!(
                        "SETRES asks for video mode {mode}; only 640x480x256 is drawn"
                    )));
                }
                self.mode = mode;
            }
            // A pair: `TOGFX` enters graphics with the mode `SETRES` stored,
            // `GFXTO` leaves it again. There is no video hardware behind them
            // here, and the framebuffer exists either way, but the flag is
            // kept because the game asks about it.
            "TOGFX" => self.graphics = true,
            "GFXTO" => self.graphics = false,
            // No sound and no hicolor, matching how the reference runs.
            "?SOUND" => stack.push(0),
            "HICOLOR" => stack.push(0),
            "RESETANIM" | "RESETFONT" => self.note_no_effect(name),
            // The resource loader, and the reason nothing was ever drawn while
            // it was a stub: `STARTUP` fills the location jump table with
            // `200 _LOCTABLE 99 GET`, and `INCLLOC` loads a location's routes,
            // click areas and items the same way. It copies a BLOCK resource
            // into module memory — block 99 is exactly the 200 bytes STARTUP
            // asks for.
            "GET" => {
                let a = pop_n(stack, 3, "GET")?;
                let (size, addr, id) = (a[0].max(0) as usize, a[1], a[2]);
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
                        return Err(Error::Unimplemented {
                            ordinal: 0,
                            name: format!("GET: block {id} is not in the resource banks"),
                            at: Address(addr as u32),
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
            "PUT" => {
                let a = pop_n(stack, 3, "PUT")?;
                let (size, addr, id) = (a[0].max(0) as usize, a[1], a[2]);
                let Some(path) = self.save_path(id, "blk") else {
                    return Err(Error::NoSaveDir { word: "PUT", id });
                };
                let bytes = mem.read_bytes(addr, size)?;
                std::fs::write(&path, &bytes).map_err(|e| Error::Io {
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
            "CTRL" => {
                let raw = pop1(stack, "CTRL")? as u32;
                self.controller = Some(Address::new(raw >> 16, raw & 0xffff));
            }
            "ERRORLEVEL" => {
                pop1(stack, "ERRORLEVEL")?;
                self.note_no_effect(name);
            }
            "SPEEDMODE" => {
                pop1(stack, "SPEEDMODE")?;
                self.note_no_effect(name);
            }
            "DREQUEST" => {
                self.inert(stack, 3, "DREQUEST")?;
            }
            "RESETTI" => self.note_no_effect(name),
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
