//! Making descriptors, setting their fields, reading them back.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Descriptor;
use crate::Engine;
use crate::Field;
use crate::Insert;
use crate::Placement;
use crate::Shows;
use crate::descriptor::SLOTS;
use crate::profile::TextInserts;
use crate::stack::pop_n;
use crate::stack::pop1;
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;
use motionvm_motion_forth::cell;

impl Engine {
    pub(crate) fn words_descriptors(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        mem: &dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
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
            Word::NEWSETDESC => {
                // A 16-bit screen has room for a hundred descriptors, and the
                // three later builds' handler (`05f1:0ad4`) jumps past every
                // pop and the push when the count stands at a hundred: the
                // six arguments stay on the stack and no handle comes back.
                // `LL.EXE`'s does not look, which the profile says. Nothing in
                // the shipped games gets near it.
                if self.profile.screen_holds_a_hundred {
                    let screen = self.display.current.unwrap_or(0);
                    let on_screen = self
                        .scene
                        .descriptors
                        .iter()
                        .filter(|d| d.screen == screen)
                        .count();
                    if on_screen >= 100 {
                        return Ok(Some(()));
                    }
                }
                let a = pop_n(stack, 6, "NEWSETDESC")?;
                let handle = self.next_handle();
                let stamp = self.next_stamp();
                self.scene.descriptors.push(Descriptor {
                    handle,
                    stamp,
                    screen: self.display.current.unwrap_or(0),
                    x: a[0],
                    y: a[1],
                    level: a[2],
                    // The graphics argument goes into the one field, marker
                    // off: `05f1:0b51` writes it raw, so `0 15 -1 NEWSETDESC`
                    // — which is how the scripts make a text descriptor — is a
                    // block on graphic 0 until `SDTXT` says otherwise.
                    shows: Shows::Picture(a[3]),
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
                self.scene.selected = Some(self.scene.descriptors.len() - 1);
                stack.push(cell::signed(handle));
            }
            // `( -- handle )`: a bare descriptor, as the 16-bit `NEWDESC` (file
            // `0x9bb8`) makes one — the screen's next number, selected, with
            // nothing set. No script of Dunkle Schatten 2 calls it.
            Word::NEWDESC => {
                let handle = self.next_handle();
                let stamp = self.next_stamp();
                self.scene.descriptors.push(Descriptor {
                    handle,
                    stamp,
                    screen: self.display.current.unwrap_or(0),
                    wait: -1,
                    ..Default::default()
                });
                self.scene.selected = Some(self.scene.descriptors.len() - 1);
                self.scene.selected_handle = Some(handle);
                stack.push(cell::signed(handle));
            }
            Word::ACTDESC => {
                let h = cell::unsigned(pop1(stack, "ACTDESC")?);
                self.select_descriptor(h);
            }
            Word::SDINACTIVE => self.set_active(false),
            Word::SDACTIVE => self.set_active(true),
            // Not a toggle: `0x72104` hands the descriptor a buffer number out
            // of a counter at 0xDB4B4, writes it to +0x1C and sets flag 0x08.
            // What hangs off that number is the save-under the drawer fills —
            // and having one is the only way anything in this engine is ever
            // erased. See [`Descriptor::buffer`](crate::Descriptor::buffer).
            Word::SDAUTOBUF => {
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
            Word::SD_PCT_SHR => {
                let v = pop1(stack, "SD%SHR")?;
                self.set_field(Field::SD_PCT_SHR, v);
                self.set_field(Field::SDH_PCT_SHR, v);
                self.set_field(Field::SDV_PCT_SHR, v);
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
            //
            // Counted on the text as laid out: both handlers run the layout
            // first (`0x731cd` → `0x6c9f8`, R78 `0x5ea51` → `0x5a100`) and
            // answer the length of its buffer — R109 from the layout's own
            // `+0x18`, R78 through `strlen` (`0x11b80`). A text with inserts
            // is measured with them in.
            Word::GDTEXTLEN | Word::GDTXTLEN => {
                let d = self.descriptor_mut().cloned().unwrap_or_default();
                let n = self
                    .laid_out_text(&d, mem)
                    .map_or(0, |s| cell::count(s.chars().count()));
                stack.push(n);
            }
            // `SDBLK` (16-bit file `0xa625`) sets bit 0x2000 of the text
            // word and `SDNORM` (file `0xa726`) takes the layout bits away
            // again — `and $0x80FF` clears the center modes with it. The
            // drawer reads the bit as justification: every line starts at
            // the block's left edge and its inner spaces stretch to the
            // widest line (`14ee:11cf`, `14ee:111c`) — the newspaper's
            // module 615 is the caller.
            Word::SDBLK => {
                if let Some(d) = self.descriptor_mut() {
                    d.fields.set(Field::SDBLK, 1);
                }
            }
            Word::SDNORM => {
                if let Some(d) = self.descriptor_mut() {
                    d.fields.clear(Field::SDBLK);
                    d.x_mode = Placement::Edge;
                    d.y_mode = Placement::Edge;
                }
            }
            // A toggle with no argument.
            Word::SDPOS => self.note_no_effect(word),
            // `( value slot -- )` on R78 (`0x611c0`), `( value kind slot -- )`
            // on R109 (`0x75cfe`): the slot is popped first, the value last,
            // and which build this is was read off the handler
            // ([`crate::Profile`]'s text-insert capability). Both store only
            // for a slot below five (`cmpl $5; jge` past everything, `0x75d4a`
            // and `0x61204`) and then mark the descriptor as any change is
            // marked (`0x6ab6e`, R78 `0x58ae0`) — R109 measuring it afresh
            // (`0x6c8c1`), which lays it out; so does this. A negative slot
            // passes the test and writes below the array in the original,
            // into the record's line window and buffer pointer; nothing
            // shipped does it, and it is noted rather than modelled.
            Word::SDINSERT => {
                let (value, kind, slot) = match self.profile.inserts {
                    TextInserts::Kinded => {
                        let a = pop_n(stack, 3, "SDINSERT")?;
                        (a[0], a[1], a[2])
                    }
                    TextInserts::Addresses => {
                        let a = pop_n(stack, 2, "SDINSERT")?;
                        (a[0], 0, a[1])
                    }
                    TextInserts::Absent => {
                        return Err(motionvm_motion_forth::Error::Unsupported(
                            "SDINSERT on a kernel whose text records have no insert slots".into(),
                        ));
                    }
                };
                if slot >= cell::count(SLOTS) {
                    return Ok(Some(()));
                }
                let Some(index) = cell::at(slot) else {
                    self.note_unhandled(word, Some(format!("slot {slot} below the array")));
                    return Ok(Some(()));
                };
                let Some(d) = self.descriptor_mut() else {
                    return Ok(Some(()));
                };
                d.inserts[index] = Insert { value, kind };
                self.lay_out_current(mem);
                self.touch_current();
            }
            // The placement words differ only in what the value means, and the
            // original encodes exactly that as a mode next to the coordinate.
            // Which axis and which mode is all these arms carry; the store is
            // [`Engine::place_x`] and [`Engine::place_y`].
            Word::SDX => {
                let v = pop1(stack, "placement")?;
                self.place_x(v, Placement::Edge)?;
            }
            Word::SDCX | Word::SDCEN => {
                let v = pop1(stack, "placement")?;
                self.place_x(v, Placement::Center)?;
            }
            Word::SDOX => {
                let v = pop1(stack, "placement")?;
                self.place_x(v, Placement::FarEdge)?;
            }
            Word::SDY => {
                let v = pop1(stack, "placement")?;
                self.place_y(v, Placement::Edge)?;
            }
            Word::SDCY | Word::SDVCEN => {
                let v = pop1(stack, "placement")?;
                self.place_y(v, Placement::Center)?;
            }
            Word::SDOY => {
                let v = pop1(stack, "placement")?;
                self.place_y(v, Placement::FarEdge)?;
            }
            // Every one of these takes exactly one value. The arm is the stack
            // ABI and nothing else — what each one means is on the method.
            Word::SDLEV
            | Word::SDLV
            | Word::SDZ
            | Word::SDSPR
            | Word::SDBL
            | Word::SDTXT
            | Word::SDTB
            | Word::SDCOL
            | Word::SDFNT
            | Word::SDTDT
            | Word::SDWAIT
            | Word::SDWORD => {
                let v = pop1(stack, "descriptor setter")?;
                match word {
                    Word::SDLEV | Word::SDLV | Word::SDZ => self.set_level(v)?,
                    Word::SDSPR => self.set_sprite(v)?,
                    Word::SDBL => self.set_block(v)?,
                    Word::SDTXT => self.set_text(v)?,
                    Word::SDTB => self.set_text_table(v)?,
                    Word::SDCOL => self.set_color(v)?,
                    Word::SDFNT => self.set_font(v)?,
                    Word::SDTDT => self.set_template(v)?,
                    Word::SDWAIT => self.set_wait(v)?,
                    // The screening is the machine's, so it happens here.
                    _ => self.set_callback(mem.callable(v))?,
                }
                // A new text or table is a new layout, which the original's
                // setters measure on the spot (`0x6c8c1` in both, R78
                // `0x58ae0`); the mark above was against the old one.
                if matches!(word, Word::SDTXT | Word::SDTB) {
                    self.lay_out_current(mem);
                    self.touch_current();
                }
            }
            // Getters, for completeness and for the oracle round-trips. Each
            // answers with one value; what each one means is on its method.
            Word::GDX
            | Word::GDY
            | Word::GDLEV
            | Word::GDLV
            | Word::GDZ
            | Word::GDSPR
            | Word::GDACTIVE
            | Word::GDBL
            | Word::GDTXT
            | Word::GDTB
            | Word::GDCX
            | Word::GDCY
            | Word::GDWIDTH
            | Word::GDHEIGHT
            | Word::GDXLEN
            | Word::GDYLEN
            | Word::GDOX
            | Word::GDOY
            | Word::GDCOL => {
                let v = match word {
                    Word::GDX => self.descriptor_x(),
                    Word::GDY => self.descriptor_y(),
                    Word::GDLEV | Word::GDLV | Word::GDZ => self.descriptor_level(),
                    Word::GDACTIVE => self.descriptor_active(),
                    Word::GDBL => self.descriptor_block(),
                    Word::GDTXT => self.descriptor_text_entry(),
                    Word::GDTB => self.descriptor_table(),
                    Word::GDCOL => self.descriptor_color(),
                    Word::GDCX => self.descriptor_center_x(),
                    Word::GDCY => self.descriptor_center_y(),
                    Word::GDWIDTH | Word::GDXLEN => self.descriptor_width(),
                    Word::GDHEIGHT | Word::GDYLEN => self.descriptor_height(),
                    Word::GDOX => self.descriptor_far_x(),
                    Word::GDOY => self.descriptor_far_y(),
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
            // membership. It is the first thing to revisit when groups arrive.
            Word::KILLDESC => {
                let handle = cell::unsigned(pop1(stack, "KILLDESC")?);
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
            Word::KILLNDESC => {
                let handle = cell::unsigned(pop1(stack, "KILLNDESC")?);
                // The 16-bit handler (file `0x9d3a`) frees the active screen's
                // descriptors from the number up and sets the count back, so
                // the next `NEWSETDESC` takes that number again.
                if self.profile.per_screen_descriptors {
                    let screen = self.display.current.unwrap_or(0);
                    self.forget_descriptors(|d| d.screen == screen && d.handle >= handle);
                    return Ok(Some(()));
                }
                let Some(from) = self
                    .scene
                    .descriptors
                    .iter()
                    .position(|d| d.handle == handle)
                else {
                    return Ok(Some(()));
                };
                let screen = self.scene.descriptors[from].screen;
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
            Word::Q_ACTDESC | Word::GDNR => {
                let h = self
                    .scene
                    .selected
                    .and_then(|i| self.scene.descriptors.get(i))
                    .map(|d| d.handle);
                stack.push(cell::signed(h.unwrap_or(0)));
            }
            // The rest of the one-argument setters, which are also the field
            // keys they write. Their names are kept as given; what each
            // controls is measurable with the `GD*` getters when it matters,
            // and a meaning guessed ahead of a measurement is a
            // `KILLNDESC`-shaped trap: plausible, silent, and wrong.
            //
            // The key comes off the word, so one lookup answers both whether
            // this is a setter and which field it writes — the arm and the
            // field map cannot name different things.
            _ => {
                let Some(key) = word.descriptor_field() else {
                    return Ok(None);
                };
                let v = pop1(stack, "descriptor setter")?;
                self.set_field(key, v);
                // The line window is part of the layout (`0x610e0`, `0x61150`
                // mark and measure like any change), so it is laid out again
                // here as `SDTXT` is above.
                if matches!(key, Field::SDSTARTLINE | Field::SDALINES) {
                    self.lay_out_current(mem);
                    self.touch_current();
                }
            }
        }
        Ok(Some(()))
    }
}
