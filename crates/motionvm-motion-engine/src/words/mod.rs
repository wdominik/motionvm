//! The kernel words, one file per group.
//!
//! `Engine::plain_word32` is a single match of ninety-odd arms; the groups here
//! are that match cut into files, nothing more. How many there are is the
//! `mod` list below and nowhere else — fifteen of them are asked on the
//! 32-bit machine and seventeen on the 16-bit one, and a count written into
//! each file would be a count to maintain in each file.
//!
//! **The order of the groups is the order of the original's match, and it is
//! load-bearing.** Two arms match on table membership rather than on a literal,
//! so an arm that moves across one of them changes which words it catches — and
//! a duplicate name in two groups is resolved by nothing but this order; see
//! the note on [`Engine::plain_word32`](crate::Engine::plain_word32). The match
//! itself — both `Host` impls and the two dispatchers — stands at the end of
//! this file, beside the rule it has to keep.
//!
//! Within that constraint a group is one subject — descriptors, screens, the
//! inventory bar, the dialogue queue — because the original's own section
//! order is not one: it visits screens twice and palette twice.

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

// The dispatchers themselves, beside the rule they must keep: the order of
// the groups above is the order of the original's match, and these are that
// match.

use crate::Engine;
use crate::order;
use motionvm_motion_formats::m32::ScrModule;
use motionvm_motion_forth as forth;
use motionvm_motion_forth::m32;
use motionvm_motion_forth::{Address, Host, Machine};

impl Host<m32::Vm> for Engine {
    /// Stops the interpreter while a transition plays, which is what the
    /// original does by never returning from `FADEOUT` until it is finished.
    fn pending_call(&mut self) -> Option<(i32, Vec<i32>)> {
        self.pending_call.take()
    }

    fn wants_pause(&mut self) -> bool {
        self.in_transition() || self.entering_loop || self.poll_yield()
    }

    fn word(&mut self, name: &str, vm: &mut m32::Vm) -> motionvm_motion_forth::Result<bool> {
        // The interaction machine runs bytecode of its own and therefore needs
        // the machine, not just its stack and memory.
        if name == "DOORDER" {
            return self.do_order(vm, order::M32_RULES);
        }
        // `( module -- )`: `=>GET` (0x64999) loads `%03d.SCR` out of the
        // resource file into the first free descriptor slot — a fresh copy
        // every time, so a location's modules come back pristine on every
        // re-entry and their variables start over. The machine here holds
        // every module from the start, so what the word does is put the
        // container's image back over the one in memory, which is the same
        // reset; the slot bookkeeping is [`Engine::mark_resident`]. A module
        // the container does not hold is only marked, as before: the first
        // `INCLLOC` asks for 100, 200 and 300 while `_ACTLOC` is still zero.
        if name == "=>GET" {
            let n = crate::stack::pop1(&mut vm.data, "=>GET")?;
            let n = n.max(0) as u32;
            self.mark_resident(n);
            if let Some(item) = self.resources.as_ref().and_then(|r| r.script(n))
                && let Ok(parsed) = ScrModule::parse(&item)
            {
                vm.load(&item, &parsed);
            }
            return Ok(true);
        }
        let m32::Vm { data, mem, .. } = vm;
        self.plain_word32(name, data, mem)
    }
}

impl Engine {
    /// Runs one kernel word by name, with its arguments on `stack`.
    ///
    /// Every word that does not need the machine itself — which is all of them
    /// so far. A word that has to run bytecode of its own gets `&mut m32::Vm` in
    /// [`Host::word`] above and is handled there instead.
    ///
    /// Public so a test can drive the same words the bytecode does — loading a
    /// savegame, for instance, is `GET`, `INCLLOC`, `GETANIM` and `=>GETAS` in
    /// that order, and reproducing a reported state is far quicker from a
    /// savegame than from an hour of play.
    ///
    /// **The order of these calls is the order of the original's own match, and
    /// it has to stay that way.** Two of the arms match on table membership
    /// rather than on a literal — the descriptor setters and the resource
    /// status hints — so where they sit decides what reaches them. A
    /// duplicate sitting earlier in the match wins silently — nothing
    /// detects one; the sequence below is the whole defense. Splitting the
    /// match into files is exactly the change that invites one in.
    ///
    /// Which is why the fifteen near-identical `if` blocks below — seventeen
    /// in [`Engine::plain_word16`] — are written out rather than folded. The
    /// repetition has been examined and kept. A loop over function pointers or
    /// a map keyed by name would destroy the property outright: neither has an
    /// order a reader can see. A two-line macro would keep the order visible
    /// and still cost something real — the one thing a reader of this function
    /// must check is the sequence of group names against the original's match,
    /// and a macro puts a layer between them for no gain but height. Thirty-two
    /// lines of the same shape are what a hand-checkable order looks like.
    pub fn plain_word32(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut m32::Memory,
    ) -> motionvm_motion_forth::Result<bool> {
        if self.words_state(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_screens(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_descriptors(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_text(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_saves(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_resources(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_transitions(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_dowalk(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_input(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_inventory(name, stack, mem, M32_RULES)?.is_some() {
            return Ok(true);
        }
        if self
            .words_dialogue(name, stack, mem, order::M32_RULES)?
            .is_some()
        {
            return Ok(true);
        }
        if self.words_sound(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self
            .words_pointer(name, stack, mem, pointer::M32_RULES)?
            .is_some()
        {
            return Ok(true);
        }
        if self.words_palette(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_redraw(name, stack, mem)?.is_some() {
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
    fn word(&mut self, name: &str, vm: &mut forth::m16::Vm) -> motionvm_motion_forth::Result<bool> {
        match name {
            // `( module -- )`: loads a module out of the container and binds
            // its ids — which the machine does; what the engine keeps is the
            // residency list, the same bookkeeping `=>GET` keeps on the 32-bit
            // machine.
            "=>GET" => {
                let n = crate::stack::pop1(&mut vm.data, "=>GET")?;
                let item = self
                    .resources
                    .as_ref()
                    .and_then(|r| r.script(n.max(0) as u32))
                    .ok_or_else(|| motionvm_motion_forth::Error::Unimplemented {
                        ordinal: 0,
                        name: format!("=>GET: module {n} is not in the container"),
                        at: Address(0),
                    })?;
                let parsed =
                    motionvm_motion_formats::m16::scr::ScrModule::parse(&item).map_err(|e| {
                        motionvm_motion_forth::Error::Unsupported(format!("=>GET {n}: {e}"))
                    })?;
                vm.load(&item, &parsed)?;
                self.mark_resident(n.max(0) as u32);
                Ok(true)
            }
            "=>ERASE" => {
                let n = crate::stack::pop1(&mut vm.data, "=>ERASE")?;
                vm.unload(n.max(0) as u16);
                self.mark_gone(n.max(0) as u32);
                Ok(true)
            }
            // `( word-id -- )`: the per-frame handler `ANIMPLAY` runs —
            // `400 SCRCTRL` for `CTRL`, `1086 SCRCTRL` for the intro's `ICTRL`.
            // The 32-bit kernel's `CTRL` takes a packed address instead.
            // The interaction machine runs bytecode of its own and needs the
            // machine, as on the 32-bit side.
            "DOORDER" => self.do_order(vm, order::M16_RULES),
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
            "REQUEST" => {
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
                        .and_then(|t| t.get(n.max(1) as usize - 1))
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
            "SCRCTRL" => {
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
                        id: id as u16,
                        at: vm.here(),
                    });
                };
                self.controller = Some(target);
                Ok(true)
            }
            _ => {
                let forth::m16::Vm { data, mem, .. } = vm;
                self.plain_word16(name, data, mem)
            }
        }
    }
}

impl Engine {
    /// Runs one kernel word by name on the 16-bit machine's stack and memory.
    ///
    /// The 16-bit-only words go first — they include the buffer words the
    /// 32-bit path treats as inert — and then the groups both generations
    /// share: the walk with its offsets scaled to 2-byte cells, the
    /// inventory with the rules read from `ENVIRO.EXE`, the savegame words
    /// over the 16-bit arena. Their order is *almost*
    /// [`Engine::plain_word32`]'s — the saves, input and walk groups sit
    /// later here — and the difference is safe to hold: none of the moved
    /// groups shares a word with anything it moved across, which is the one
    /// thing the ordering rule protects.
    ///
    /// Written out one call per line for the reason [`Engine::plain_word32`]
    /// gives: the sequence is the thing to be checked, and it has to be
    /// readable without expanding anything.
    pub fn plain_word16(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut motionvm_motion_forth::m16::Memory,
    ) -> motionvm_motion_forth::Result<bool> {
        if self.words_m16(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_buffers(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_state(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_screens(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_descriptors(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_text(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_resources(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_transitions(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_input(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_saves(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_dowalk(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_inventory(name, stack, mem, M16_RULES)?.is_some() {
            return Ok(true);
        }
        if self
            .words_dialogue(name, stack, mem, order::M16_RULES)?
            .is_some()
        {
            return Ok(true);
        }
        if self.words_sound(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self
            .words_pointer(name, stack, mem, pointer::M16_RULES)?
            .is_some()
        {
            return Ok(true);
        }
        if self.words_palette(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        if self.words_redraw(name, stack, mem)?.is_some() {
            return Ok(true);
        }
        Ok(false)
    }
}
