//! Making descriptors, setting their fields, reading them back.
//!
//! One of the ten groups `plain_word` hands a word to, in the order the
//! original's own match had them. A group that does not know the word
//! answers `None` and the next one is asked.

use crate::Descriptor;
use crate::DescriptorKind;
use crate::Engine;
use crate::Placement;
use crate::descriptor::DESCRIPTOR_SETTERS;
use crate::stack::pop_n;
use crate::stack::pop1;
use motionvm_forth::AddressSpace;
use motionvm_forth::Result;

impl Engine {
    pub(crate) fn words_descriptors(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match name {
            // --- descriptors ------------------------------------------------
            // Six arguments, not the three the call sites suggest. The handler
            // pops six in a row (0x70c08 onwards): x, y, level, a graphic id,
            // one value it drops on the floor, and — topmost — a **callback**.
            // The graphic goes into the same four-byte payload `SDBL`
            // allocates, so a descriptor made this way already has its picture,
            // which is why a room background reads `0 0 2 1010 0 0 NEWSETDESC
            // _BG ! SDACTIVE` with no `SDSPR` after it. Dropping that argument
            // cost a whole room's backdrop.
            //
            // The callback is the same field `SDWORD` writes: 0 and -1 mean
            // none and clear it (0x70c83-0x70c8f), anything else is checked as
            // an address and stored at +0x14 (0x70cc9) — the screening is the
            // machine's, [`AddressSpace::callable`]. Dropping *that* one cost
            // the start of the game. `_TI1`, the descriptor every line of narration
            // goes through, is made with `160 100 100 1 0 0x51780 NEWSETDESC`,
            // and 0x51780 is module 5 at 0x1780 — listing 0x17b0, which is the
            // body of `FOLLOWMAN`, a word whose entire definition is
            // `SDINACTIVE EXIT`. That is how a text switches itself off when
            // its `SDWAIT` runs out, and how `?READYT1` ever comes back true.
            //
            // Hence the -1 wait (0x70ccf): with a callback and a wait of zero
            // the frame walk would fire it on the very first frame and turn
            // every descriptor off before anything was drawn. The two belong
            // together.
            "NEWSETDESC" => {
                let a = pop_n(stack, 6, "NEWSETDESC")?;
                let handle = self.next_handle();
                let stamp = self.next_stamp();
                self.descriptors.push(Descriptor {
                    handle,
                    stamp,
                    screen: self.display.current.unwrap_or(0),
                    x: a[0],
                    y: a[1],
                    level: a[2],
                    block: (a[3] != 0).then_some(a[3] as u32),
                    kind: if a[3] != 0 {
                        DescriptorKind::Block
                    } else {
                        DescriptorKind::Empty
                    },
                    active: true,
                    // `NEWSETDESC` writes the flag word 0xD000 at 0x70d07 —
                    // active, dirty and changed — so a fresh descriptor is
                    // drawn by the very next pass without anything marking it.
                    dirty: true,
                    changed: true,
                    callback: mem.callable(a[5]),
                    wait: -1,
                    ..Default::default()
                });
                self.selected = Some(self.descriptors.len() - 1);
                stack.push(handle as i32);
            }
            // `( -- handle )`: a bare descriptor, as the 16-bit `NEWDESC` (file
            // `0x9bb8`) makes one — the screen's next number, selected, with
            // nothing set. No script of Dunkle Schatten 2 calls it.
            "NEWDESC" => {
                let handle = self.next_handle();
                let stamp = self.next_stamp();
                self.descriptors.push(Descriptor {
                    handle,
                    stamp,
                    screen: self.display.current.unwrap_or(0),
                    wait: -1,
                    ..Default::default()
                });
                self.selected = Some(self.descriptors.len() - 1);
                self.selected_handle = Some(handle);
                stack.push(handle as i32);
            }
            "ACTDESC" => {
                let h = pop1(stack, "ACTDESC")? as u32;
                self.select_descriptor(h);
            }
            "SDINACTIVE" => self.set_active(false),
            "SDACTIVE" => self.set_active(true),
            // Not a toggle: `0x72104` hands the descriptor a buffer number out
            // of a counter at 0xDB4B4, writes it to +0x1C and sets flag 0x08.
            // What hangs off that number is the save-under the drawer fills —
            // and having one is the only way anything in this engine is ever
            // erased. See [`Descriptor::buffer`](crate::Descriptor::buffer).
            "SDAUTOBUF" => {
                if let Some(d) = self.descriptor_mut() {
                    d.auto_buffer = true;
                }
            }
            // Both shrink factors at once. Read from both binaries: the 32-bit
            // handler (`ENGINE.EXE` `0x721b8`) writes the descriptor's
            // horizontal and vertical factor from the one argument, the
            // 16-bit one (`ENVIRO.EXE` file `0xb38b`) calls `SDH%SHR` and
            // `SDV%SHR` with it. Setting the two named fields as well keeps a
            // later `SDH%SHR` from being undone by an earlier `SD%SHR`'s
            // fallback, and an earlier one from surviving it.
            "SD%SHR" => {
                let v = pop1(stack, "SD%SHR")?;
                self.set_field("SD%SHR", v);
                self.set_field("SDH%SHR", v);
                self.set_field("SDV%SHR", v);
            }
            // The rest of the one-argument setters. Their names are kept as
            // given; what each controls is measurable with the `GD*` getters
            // when it matters, and a meaning guessed ahead of a measurement
            // is a `KILLNDESC`-shaped trap: plausible, silent, and wrong.
            _ if DESCRIPTOR_SETTERS.contains(&name) => {
                let v = pop1(stack, "descriptor setter")?;
                // The `&'static str` from the table rather than the borrowed
                // `name`, because that is what the field map is keyed on. The
                // search cannot miss — the guard just walked the same slice —
                // but it says so by not answering rather than by asserting.
                if let Some(&key) = DESCRIPTOR_SETTERS.iter().find(|k| **k == name) {
                    self.set_field(key, v);
                }
            }
            // How long the current text is, in characters, newlines counted.
            //
            // Measured, not stored: there is no field for it, so answering
            // from one gives zero forever. Two values measured against the
            // original settle both the meaning and the counting —
            // with table 6 selected, entry 108 answers 79 and entry 109 answers
            // 380, and those are exactly the string lengths *including* their
            // newlines (76 and 373 without).
            //
            // It matters more than a getter usually does. `TSX` in module 5
            // turns this length into how long a speech stays up — under 20 it
            // uses 20, over 100 a third of it, over 50 a half, then scales by
            // `_TSPEED` percent. With zero the first branch always won, and
            // every line in the game stood for the same two seconds instead of
            // a time that follows what it says.
            "GDTEXTLEN" | "GDTXTLEN" => {
                let d = self.descriptor_mut().cloned().unwrap_or_default();
                let n = self
                    .descriptor_text(&d)
                    .map_or(0, |s| s.chars().count() as i32);
                stack.push(n);
            }
            // `SDBLK` (16-bit file `0xa625`) sets bit 0x2000 of the text
            // word and `SDNORM` (file `0xa726`) takes the layout bits away
            // again — `and $0x80FF` clears the center modes with it. The
            // drawer reads the bit as justification: every line starts at
            // the block's left edge and its inner spaces stretch to the
            // widest line (`14ee:11cf`, `14ee:111c`) — the newspaper's
            // module 615 is the caller.
            "SDBLK" => {
                if let Some(d) = self.descriptor_mut() {
                    d.fields.insert("SDBLK", 1);
                }
            }
            "SDNORM" => {
                if let Some(d) = self.descriptor_mut() {
                    d.fields.remove("SDBLK");
                    d.x_mode = Placement::Edge;
                    d.y_mode = Placement::Edge;
                }
            }
            // A toggle with no argument.
            "SDPOS" => self.note_no_effect(name),
            "SDINSERT" => {
                let a = pop_n(stack, 3, "SDINSERT")?;
                if let Some(d) = self.descriptor_mut() {
                    d.fields.insert("INSERT", a[0]);
                }
                // What it inserts is part of what the descriptor shows, so the
                // change is marked like any other (0x6ab6e).
                self.touch_current();
            }
            // The placement words differ only in what the value means, and the
            // original encodes exactly that as a mode next to the coordinate.
            // Which axis and which mode is all these arms carry; the store is
            // [`Engine::place_x`] and [`Engine::place_y`].
            "SDX" => {
                let v = pop1(stack, "placement")?;
                self.place_x(v, Placement::Edge)?;
            }
            "SDCX" | "SDCEN" => {
                let v = pop1(stack, "placement")?;
                self.place_x(v, Placement::Center)?;
            }
            "SDOX" => {
                let v = pop1(stack, "placement")?;
                self.place_x(v, Placement::FarEdge)?;
            }
            "SDY" => {
                let v = pop1(stack, "placement")?;
                self.place_y(v, Placement::Edge)?;
            }
            "SDCY" | "SDVCEN" => {
                let v = pop1(stack, "placement")?;
                self.place_y(v, Placement::Center)?;
            }
            "SDOY" => {
                let v = pop1(stack, "placement")?;
                self.place_y(v, Placement::FarEdge)?;
            }
            // Every one of these takes exactly one value. The arm is the stack
            // ABI and nothing else — what each one means is on the method.
            "SDLEV" | "SDLV" | "SDZ" | "SDSPR" | "SDBL" | "SDTXT" | "SDTB" | "SDCOL" | "SDFNT"
            | "SDTDT" | "SDWAIT" | "SDWORD" => {
                let v = pop1(stack, "descriptor setter")?;
                match name {
                    "SDLEV" | "SDLV" | "SDZ" => self.set_level(v)?,
                    "SDSPR" => self.set_sprite(v)?,
                    "SDBL" => self.set_block(v)?,
                    "SDTXT" => self.set_text(v)?,
                    "SDTB" => self.set_text_table(v)?,
                    "SDCOL" => self.set_color(v)?,
                    "SDFNT" => self.set_font(v)?,
                    "SDTDT" => self.set_template(v)?,
                    "SDWAIT" => self.set_wait(v)?,
                    // The screening is the machine's, so it happens here.
                    _ => self.set_callback(mem.callable(v))?,
                }
            }
            // Getters, for completeness and for the oracle round-trips. Each
            // answers with one value; what each one means is on its method.
            "GDX" | "GDY" | "GDLEV" | "GDLV" | "GDZ" | "GDSPR" | "GDACTIVE" | "GDBL" | "GDTXT"
            | "GDTB" | "GDCX" | "GDCY" | "GDWIDTH" | "GDHEIGHT" | "GDXLEN" | "GDYLEN" | "GDOX"
            | "GDOY" | "GDCOL" => {
                let v = match name {
                    "GDX" => self.descriptor_x(),
                    "GDY" => self.descriptor_y(),
                    "GDLEV" | "GDLV" | "GDZ" => self.descriptor_level(),
                    "GDACTIVE" => self.descriptor_active(),
                    "GDBL" => self.descriptor_block(),
                    "GDTXT" => self.descriptor_text_entry(),
                    "GDTB" => self.descriptor_table(),
                    "GDCOL" => self.descriptor_color(),
                    "GDCX" => self.descriptor_center_x(),
                    "GDCY" => self.descriptor_center_y(),
                    "GDWIDTH" | "GDXLEN" => self.descriptor_width(),
                    "GDHEIGHT" | "GDYLEN" => self.descriptor_height(),
                    "GDOX" => self.descriptor_far_x(),
                    "GDOY" => self.descriptor_far_y(),
                    _ => self.descriptor_sprite(),
                };
                stack.push(v);
            }

            // --- descriptors, continued -------------------------------------
            // One descriptor, out of its screen's list. The handler detaches it
            // from a group first — parent at +0x1c, unhooked through 0x6b5d2
            // and 0x6b7f7 — and then closes the gap in the list (0x70f95
            // onwards: find the index, `+0x418--`, shift the rest down).
            //
            // The group half has nothing to do here yet: nothing models group
            // membership, `SDINSERT` only files a value away under its name. It
            // is the first thing to revisit when groups arrive.
            "KILLDESC" => {
                let handle = pop1(stack, "KILLDESC")? as u32;
                self.forget_descriptors(|d| d.handle == handle);
            }
            // Not one descriptor but a *tail*: this one and every later one on
            // the same screen.
            //
            // The handler (0x71047) looks its argument up in the screen's list,
            // and then kills whatever sits at that index until the list is that
            // short (0x710bb-0x710e5) — always the same index, because each
            // `KILLDESC` shifts the next one into it.
            //
            // This is how a location is torn down. `INCLLOC` runs `_BG @
            // KILLNDESC` before it loads anything new, and `_BG` still holds the
            // *previous* location's background — the first descriptor a
            // location macro makes. So everything from that point on goes and
            // everything made before it stays: the system descriptors, `_TI1`,
            // `_IINFO`, the inventory bar.
            //
            // Read as a screen handle instead — which is what stood here — the
            // argument matched no screen and nothing was ever removed, so every
            // location piled up on the last one. The classroom was still
            // drawing the title screen's picture underneath it.
            //
            // A handle that names no descriptor does nothing, as in the
            // original: the search ends with `i == count` and the kill loop
            // never starts (0x710ac).
            "KILLNDESC" => {
                let handle = pop1(stack, "KILLNDESC")? as u32;
                // The 16-bit handler (file `0x9d3a`) frees the active screen's
                // descriptors from the number up and sets the count back, so
                // the next `NEWSETDESC` takes that number again.
                if self.per_screen_descriptors {
                    let screen = self.display.current.unwrap_or(0);
                    self.forget_descriptors(|d| d.screen == screen && d.handle >= handle);
                    return Ok(Some(()));
                }
                let Some(from) = self.descriptors.iter().position(|d| d.handle == handle) else {
                    return Ok(Some(()));
                };
                let screen = self.descriptors[from].screen;
                let mut i = 0;
                self.forget_descriptors(|d| {
                    let doomed = i >= from && d.screen == screen;
                    i += 1;
                    doomed
                });
            }

            // The handle of whatever `ACTDESC` last selected. `current` holds an
            // index into the descriptor list, so the handle has to be read back
            // out of it.
            //
            // `GDNR` is the same word under another name — not merely
            // equivalent but the same two instructions, `mov 0xDB4C0,%eax` then
            // push, at 0x71652 and 0x720c2. That cell holds the handle; the
            // resolved record lives separately in 0xF2AF0, which is what every
            // other `GD…` reads.
            "?ACTDESC" | "GDNR" => {
                let h = self
                    .selected
                    .and_then(|i| self.descriptors.get(i))
                    .map(|d| d.handle);
                stack.push(h.unwrap_or(0) as i32);
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
