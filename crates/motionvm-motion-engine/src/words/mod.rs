//! The kernel words, one file per group.
//!
//! A word arrives as an **ordinal** and is turned into a [`Word`] by a table
//! the opener built — see [`crate::Engine::bind_words`] and [`word`], which is
//! the whole of the name → meaning map. What is left here is the routing: each
//! group is a match over `Word`, and a group that does not know a value
//! answers `None` so the next is asked.
//!
//! **The order of those calls decides nothing.** It would if a word arrived
//! as a `&str` and each group matched it against string literals, the first
//! to recognize one winning: two groups claiming a name would be resolved by
//! nothing but the order of the calls, and nothing would detect it. Thirteen
//! names really are two words apiece, one per machine, and a call order is
//! how a name-keyed dispatch would tell them apart. Here they are two values
//! apiece, decided by which resolver ran, and the two arms that would
//! otherwise match on *table membership* — the resource status hints and the
//! descriptor setters — are ordinary values too. What is left of the order is
//! taste.
//!
//! A group is one subject — descriptors, screens, the inventory bar, the
//! dialogue queue — because the original's own section order is not one: it
//! visits screens twice and palette twice.

pub(crate) mod word;
pub(crate) use word::Word;

mod buffers;
mod descriptors;
mod dialogue;
mod dowalk;
mod input;
mod inventory;
mod m16;
// The two inventory operations are the game's own list surgery and the verb
// menu does it directly, without going round through the words.
pub(crate) use inventory::{
    M16_RULES, M32_RULES, Rules as InventoryRules, add_to_inventory, remove_from_inventory,
    slot_address,
};
mod palette;
pub(crate) mod pointer;
mod redraw;
mod resources;
mod saves;
mod screens;
mod sound;
mod state;
mod text;
mod transitions;

// The dispatchers: the two `Host` impls, which answer the words that need the
// machine itself, and the two chains behind them.

use crate::Engine;
use crate::order;
use motionvm_motion_forth as forth;
use motionvm_motion_forth::cell;
use motionvm_motion_forth::m32;
use motionvm_motion_forth::{Host, Machine};

impl Host<m32::Vm> for Engine {
    /// Stops the interpreter while a transition plays, which is what the
    /// original does by never returning from `FADEOUT` until it is finished.
    fn pending_call(&mut self) -> Option<(i32, Vec<i32>)> {
        self.pending_call.take()
    }

    fn wants_pause(&mut self) -> bool {
        self.in_transition() || self.entering_loop || self.poll_yield()
    }

    /// The ordinal is resolved through the table the opener built; an ordinal
    /// this engine has no word for is not ours, and the machine reports it by
    /// name.
    fn word(&mut self, ordinal: u32, vm: &mut m32::Vm) -> motionvm_motion_forth::Result<bool> {
        let Some(word) = self.word_of(ordinal) else {
            return Ok(false);
        };
        match word {
            // The interaction machine runs bytecode of its own and therefore
            // needs the machine, not just its stack and memory.
            Word::DOORDER => return self.do_order(vm, order::M32_RULES),
            // `( module -- )`: `=>GET` (0x64999) loads `%03d.SCR` out of the
            // resource file into the first free descriptor slot — a fresh copy
            // every time, so a location's modules come back pristine on every
            // re-entry and their variables start over — and `=>ERASE` gives
            // the memory back and frees the slot. Both need the machine, which
            // is why they are answered here rather than in a group. The slot
            // table is not decoration: `=>PUTAS` writes the modules these two
            // leave marked, in the order they leave them in, and a savegame
            // that carried the other seventy-odd modules would restore state
            // the original discards at every change of location.
            //
            // `=>GET` leaves nothing on the stack: the handler has no call to
            // the push helper anywhere in its body. The arity table's single
            // push came from the scan running past the function's end, which
            // is a reminder that its boundaries are inferred from the next
            // handler's address, not from the code.
            Word::RES_GET => {
                let n = crate::stack::pop1(&mut vm.data, "=>GET")?;
                self.get_module(&mut vm.mem, cell::unsigned(n.max(0)))?;
                return Ok(true);
            }
            // A module that is not loaded is nothing to give back: the first
            // `INCLLOC` after boot erases 100, 200 and 300 while `_ACTLOC` is
            // still zero.
            Word::RES_ERASE => {
                let n = crate::stack::pop1(&mut vm.data, "=>ERASE")?;
                let n = cell::unsigned(n.max(0));
                vm.unload(n);
                self.mark_gone(n);
                return Ok(true);
            }
            _ => {}
        }
        let m32::Vm { data, mem, .. } = vm;
        self.word32(word, data, mem)
    }
}

impl Engine {
    /// Offers a resolved word to the groups until one takes it.
    ///
    /// Every word that does not need the machine itself — which is all of them
    /// but two. A word that has to run bytecode of its own gets
    /// `&mut m32::Vm` in [`Host::word`] above and is handled there instead.
    ///
    /// The order of these calls is not load-bearing: a [`Word`] is one value
    /// and exactly one group matches it, whichever order they are asked in.
    /// It is kept as the original's own section order because that is a
    /// reader's map of the kernel, and because changing it would gain nothing.
    ///
    /// The fifteen near-identical blocks are written out rather than folded:
    /// a loop over function pointers would put a layer between the reader and
    /// the list of groups, and the list is the only thing here worth reading.
    pub(crate) fn word32(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        mem: &mut m32::Memory,
    ) -> motionvm_motion_forth::Result<bool> {
        if self.words_state(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_screens(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_descriptors(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_text(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_saves(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_resources(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_transitions(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_dowalk(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_input(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_inventory(word, stack, mem, M32_RULES)?.is_some() {
            return Ok(true);
        }
        if self
            .words_dialogue(word, stack, mem, order::M32_RULES)?
            .is_some()
        {
            return Ok(true);
        }
        if self.words_sound(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self
            .words_pointer(word, stack, mem, pointer::M32_RULES)?
            .is_some()
        {
            return Ok(true);
        }
        if self.words_palette(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_redraw(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        Ok(false)
    }
}

impl Host<forth::m16::Vm> for Engine {
    fn pending_call(&mut self) -> Option<(i32, Vec<i32>)> {
        self.pending_call.take()
    }

    fn wants_pause(&mut self) -> bool {
        // A request that has not been answered holds the machine where the
        // original's own handler holds it: inside the word, until a click.
        let asking = self.request.as_ref().is_some_and(|r| r.answer.is_none());
        asking || self.in_transition() || self.entering_loop || self.poll_yield()
    }

    /// The 16-bit machine's words. Three need the machine itself — the two
    /// that load and drop modules, and the one that installs the frame
    /// handler by word id — and are answered here; the rest go to
    /// [`Engine::plain_word16`] with the stack and the memory.
    fn word(
        &mut self,
        ordinal: u32,
        vm: &mut forth::m16::Vm,
    ) -> motionvm_motion_forth::Result<bool> {
        let Some(word) = self.word_of(ordinal) else {
            return Ok(false);
        };
        match word {
            // `( module -- )`: loads a module out of the container and binds
            // its ids — which the machine does; what the engine keeps is the
            // residency list, the same bookkeeping `=>GET` keeps on the 32-bit
            // machine.
            Word::RES_GET => {
                let n = crate::stack::pop1(&mut vm.data, "=>GET")?;
                let item = self
                    .resources
                    .as_ref()
                    .and_then(|r| r.script(cell::unsigned(n.max(0))))
                    .ok_or(motionvm_motion_forth::Error::MissingResource {
                        kind: "module",
                        id: n,
                        word: "=>GET",
                        at: None,
                    })?;
                let parsed =
                    motionvm_motion_formats::m16::scr::ScrModule::parse(&item).map_err(|e| {
                        motionvm_motion_forth::Error::Unsupported(format!("=>GET {n}: {e}"))
                    })?;
                vm.load(&item, &parsed)?;
                self.mark_resident(cell::unsigned(n.max(0)));
                Ok(true)
            }
            Word::RES_ERASE_16 => {
                let n = crate::stack::pop1(&mut vm.data, "=>ERASE")?;
                vm.unload(cell::low16(n.max(0)));
                self.mark_gone(cell::unsigned(n.max(0)));
                Ok(true)
            }
            // `( word-id -- )`: the per-frame handler `ANIMPLAY` runs —
            // `400 SCRCTRL` for `CTRL`, `1086 SCRCTRL` for the intro's `ICTRL`.
            // The 32-bit kernel's `CTRL` takes a packed address instead.
            // The interaction machine runs bytecode of its own and needs the
            // machine, as on the 32-bit side.
            Word::DOORDER => self.do_order(vm, order::M16_RULES),
            // `( x y w h block string n default cap[n] -- answer )`: the
            // system's message box. The handler (`0104:35a0`) pops eight and
            // then `n` captions — `n` is the seventh, the eighth is the
            // button Enter answers — fetches the strings out of the text
            // block, and blocks inside the drawer until something answers.
            //
            // Nothing can block here, so the word is asked again every frame
            // until it has an answer: the first ask puts the box up, and
            // [`Engine::wants_pause`] holds the machine on the word while the
            // frames that draw it and read the pointer go by. See
            // [`crate::request`] for the geometry, all of it read from the
            // drawer.
            Word::REQUEST => {
                if let Some(answer) = self.request.as_ref().and_then(|r| r.answer) {
                    self.request = None;
                    vm.data.push(answer);
                    return Ok(true);
                }
                if self.request.is_some() {
                    // Still asking: come back to this cell next frame.
                    vm.repeat_word();
                    return Ok(true);
                }
                let mut pop = || crate::stack::pop1(&mut vm.data, "REQUEST");
                let (x, y, w, h) = (pop()?, pop()?, pop()?, pop()?);
                let (block, string, buttons, default) = (pop()?, pop()?, pop()?, pop()?);
                let mut captions = Vec::new();
                for _ in 0..buttons.clamp(0, 5) {
                    captions.push(pop()?);
                }
                // The string number is **one-based**: the fetcher at
                // `0104:80ac` admits an index the table's own count is not
                // less than (`0104:80e2`, `jge`) and reaches the first string
                // at index 1 (`0104:8103`, the offset is the index doubled).
                let text = |eng: &mut Engine, n: i32| {
                    eng.text_table(block)
                        .and_then(|t| t.get(cell::at(n.max(1) - 1)?))
                        .unwrap_or_default()
                        .to_string()
                };
                let message = text(self, string);
                let captions = captions.into_iter().map(|n| text(self, n)).collect();
                self.request = Some(crate::request::Request {
                    x,
                    y,
                    w,
                    h,
                    message,
                    captions,
                    default,
                    fg: self.system_fg,
                    bg: self.system_bg,
                    answer: None,
                    was_down: true,
                });
                vm.repeat_word();
                Ok(true)
            }
            Word::SCRCTRL => {
                let id = crate::stack::pop1(&mut vm.data, "SCRCTRL")?;
                // The handler is a store and nothing else — `0104:165d` in
                // `LL.EXE` pops the id and writes it to the screen's `+0x14`
                // without looking at it, and the id is only resolved when a
                // frame comes to run it. So a negative id is not a word that
                // failed to bind, it is the absence of a controller: Victor
                // Loomes' `RUN` clears the field with `-1 SCRCTRL` before its
                // intro and installs `CTRL` with `432 SCRCTRL` after.
                if let Some(s) = self.display.current_mut() {
                    s.controller = id;
                }
                if id < 0 {
                    return Ok(true);
                }
                let Some(target) = vm.callback_target(id) else {
                    return Err(motionvm_motion_forth::Error::UnboundWord {
                        id: cell::low16(id),
                        at: vm.here(),
                    });
                };
                self.controller = Some(target);
                Ok(true)
            }
            _ => {
                let forth::m16::Vm { data, mem, .. } = vm;
                self.word16(word, data, mem)
            }
        }
    }
}

impl Engine {
    /// The same for the 16-bit machine's stack and memory.
    ///
    /// The 16-bit-only groups go first — `m16` and the real buffer words —
    /// and then the groups both generations share: the walk with its offsets
    /// scaled to 2-byte cells, the inventory with the rules read from
    /// `ENVIRO.EXE`, the savegame words over the 16-bit arena. That the two
    /// chains are in slightly different orders needs no argument: the
    /// resolver has already decided which of the thirteen two-meaning names
    /// this is, so no group can take a word another group wanted.
    pub(crate) fn word16(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        mem: &mut motionvm_motion_forth::m16::Memory,
    ) -> motionvm_motion_forth::Result<bool> {
        if self.words_m16(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_buffers(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_state(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_screens(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_descriptors(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_text(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_resources(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_transitions(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_input(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_saves(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_dowalk(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_inventory(word, stack, mem, M16_RULES)?.is_some() {
            return Ok(true);
        }
        if self
            .words_dialogue(word, stack, mem, order::M16_RULES)?
            .is_some()
        {
            return Ok(true);
        }
        if self.words_sound(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self
            .words_pointer(word, stack, mem, pointer::M16_RULES)?
            .is_some()
        {
            return Ok(true);
        }
        if self.words_palette(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_redraw(word, stack, mem)?.is_some() {
            return Ok(true);
        }
        Ok(false)
    }
}

impl Engine {
    /// Runs one kernel word **by name**, for a caller that has a name and no
    /// kernel — which is every test that drives the words directly.
    ///
    /// The engine itself never comes this way: a word reaches it as an
    /// ordinal, resolved through a table filled when the game opened. This is
    /// the same resolution done one word at a time, so a test spells the word
    /// the way the disassembly does.
    ///
    /// Loading a savegame, for instance, is `GET`, `INCLLOC`, `GETANIM` and
    /// `=>GETAS` in that order, and reproducing a reported state is far
    /// quicker from a savegame than from an hour of play.
    pub fn plain_word32(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut m32::Memory,
    ) -> motionvm_motion_forth::Result<bool> {
        match Word::of_m32(name) {
            Some(word) => self.word32(word, stack, mem),
            None => Ok(false),
        }
    }

    /// [`Engine::plain_word32`] for the 16-bit machine, whose kernel gives ten
    /// of these names a different meaning.
    pub fn plain_word16(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut motionvm_motion_forth::m16::Memory,
    ) -> motionvm_motion_forth::Result<bool> {
        match Word::of_m16(name) {
            Some(word) => self.word16(word, stack, mem),
            None => Ok(false),
        }
    }
}
