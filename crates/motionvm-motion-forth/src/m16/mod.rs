//! The 16-bit machine: one flat address space of 64 KiB, modules placed into
//! it by `=>GET`, a table from global word id to body, and an interpreter
//! whose return is the kernel word `##`. This is the machine of
//! `ENVIRO.EXE`, as Die Enviro-Kids greifen ein runs on it.
//!
//! # What the modules establish, and what they do not
//!
//! The *shape* of this machine is read off the 65 compiled modules of that
//! game, which are the only authority on hand: a cell is 16 bits; a kernel
//! word is `0x8000 | ordinal`; any other cell calls the word with that
//! global id — sixteen different modules define id 549 and the loader runs
//! whichever is resident, so ids resolve through a table `=>GET` fills and
//! `=>ERASE` empties; `##` (ordinal 1) ends every colon definition, so it is
//! the return; `_PutAdr` pushes the byte address of the cell after it and
//! returns; branches land on the operand cell's index plus or minus the
//! distance, and `DO` is `limit index DO`. All of that is measured.
//!
//! What the modules cannot establish is **handler-level behavior**, and the
//! handlers in `ENVIRO.EXE` are unread. Where this machine needs such a rule
//! it takes the 32-bit engine's measured one as the hypothesis — true is 1,
//! `WHILE` leaves on true, `LOOP` compares the stepped index against the
//! limit the same way, `@` and `!` align to a cell — and says so at the
//! spot. Two things are this machine's own choice, with no original to
//! match: modules are placed first-fit from `0x100` upward (where the
//! original puts them is open), and the data stack holds its 16-bit cells
//! sign-extended in `i32` so that the engine's words see one stack type for
//! both machines; every push wraps to 16 bits first.
//!
//! # Addresses
//!
//! A data address is a flat 16-bit byte address — what `_PutAdr` pushes and
//! `@`/`!` take. [`Address`] (module and offset) is the neutral location the
//! crate reports in and accepts from a host; this machine maps it to the flat
//! address by adding the module's base, and back by looking the flat address
//! up in the loaded modules. The offset counts bytes from the start of the
//! module's image in memory, which is the start of the item as the container
//! holds it — so a word's body offset in the module file is its offset here.

use std::collections::BTreeMap;

use motionvm_motion_formats::Binding;
use motionvm_motion_formats::m16::mz::{KERNEL_BIT, ORDINAL_MASK};
use motionvm_motion_formats::m16::scr::ScrModule;

use crate::{Address, Error, Host, Result, Run, prims};

mod prims16;

pub use prims16::IMPLEMENTED;

/// Bytes per cell.
pub const CELL: u16 = 2;
/// The whole address space: 16-bit byte addresses.
pub const SPACE: usize = 0x1_0000;
/// Where the first module is placed. Addresses below it are never handed
/// out, so a zero address stays invalid — what the original does here is
/// open; this is the machine's own choice.
const FIRST_BASE: u16 = 0x100;

/// One loaded module: where it sits and what it defines.
pub struct Module {
    /// The module number, as the container and `=>GET` name it.
    pub number: u16,
    /// Flat address of the module image's first byte.
    pub base: u16,
    /// Length of the image.
    pub len: u16,
    /// Word names by body address, for traces and for calling by name.
    names: BTreeMap<u16, String>,
    /// The global ids this module defines.
    ids: Vec<u16>,
}

impl Module {
    /// The flat address of a word's body, by name.
    pub fn word(&self, name: &str) -> Option<u16> {
        self.names
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(&a, _)| a)
    }

    /// The name of the word whose body starts at this flat address.
    pub fn name_at(&self, flat: u16) -> Option<&str> {
        self.names.get(&flat).map(String::as_str)
    }

    /// Whether `flat` lies inside this module's image.
    fn contains(&self, flat: u16) -> bool {
        flat >= self.base && (flat as usize) < self.base as usize + self.len as usize
    }
}

/// The flat memory, the modules placed in it, and the word table.
pub struct Memory {
    bytes: Vec<u8>,
    modules: BTreeMap<u16, Module>,
    /// Body address per global word id; 0 is unbound. Every id fits a
    /// `u16`, so the table is the whole id space.
    table: Vec<u16>,
    /// Free ranges as `(start, len)`, ascending, never adjacent.
    free: Vec<(u16, u16)>,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    /// An empty space with nothing loaded.
    pub fn new() -> Self {
        Self {
            bytes: vec![0; SPACE],
            modules: BTreeMap::new(),
            table: vec![0; SPACE],
            free: vec![(FIRST_BASE, (SPACE - FIRST_BASE as usize) as u16)],
        }
    }

    /// Places a module — what `=>GET` does — and binds its ids.
    ///
    /// The whole item goes into memory as it lies in the container, so a
    /// word's body address is the module's base plus its body offset. The
    /// place is the first free range that fits, from low addresses up; a
    /// module already loaded is replaced in place when the new image has the
    /// same length, otherwise unloaded and placed afresh. Ids the module
    /// defines are bound to its bodies, taking over from whatever held them.
    pub fn load(&mut self, item: &[u8], parsed: &ScrModule) -> Result<u16> {
        let number = parsed.module;
        if item.len() > SPACE - FIRST_BASE as usize {
            return Err(Error::OutOfMemory {
                module: number as u32,
                need: item.len(),
            });
        }
        let len = item.len() as u16;
        let base = match self.modules.get(&number) {
            Some(m) if m.len == len => m.base,
            Some(_) => {
                self.unload(number);
                self.take(len, number)?
            }
            None => self.take(len, number)?,
        };
        self.bytes[base as usize..base as usize + item.len()].copy_from_slice(item);
        let mut names = BTreeMap::new();
        let mut ids = Vec::with_capacity(parsed.entries.len());
        for e in &parsed.entries {
            let body = base + e.body_offset() as u16;
            names.insert(body, e.name.clone());
            ids.push(e.id);
            self.table[e.id as usize] = body;
        }
        self.modules.insert(
            number,
            Module {
                number,
                base,
                len,
                names,
                ids,
            },
        );
        Ok(base)
    }

    /// First-fit allocation out of the free list.
    fn take(&mut self, len: u16, number: u16) -> Result<u16> {
        let Some(i) = self.free.iter().position(|&(_, l)| l >= len) else {
            return Err(Error::OutOfMemory {
                module: number as u32,
                need: len as usize,
            });
        };
        let (start, have) = self.free[i];
        // Keep every module on an even address: bodies are cells.
        if have == len {
            self.free.remove(i);
        } else {
            self.free[i] = (start + len, have - len);
        }
        Ok(start)
    }

    /// Removes a module — what `=>ERASE` does — and frees its ids and space.
    ///
    /// An id is unbound only if it still points into this module: a later
    /// `=>GET` of a module defining the same id has taken it over, and that
    /// binding stays. Answers whether the module was loaded.
    pub fn unload(&mut self, number: u16) -> bool {
        let Some(m) = self.modules.remove(&number) else {
            return false;
        };
        for &id in &m.ids {
            if m.contains(self.table[id as usize]) {
                self.table[id as usize] = 0;
            }
        }
        self.bytes[m.base as usize..m.base as usize + m.len as usize].fill(0);
        self.give(m.base, m.len);
        true
    }

    /// Returns a range to the free list, merging with its neighbors.
    fn give(&mut self, start: u16, len: u16) {
        let end = start as usize + len as usize;
        let i = self
            .free
            .partition_point(|&(s, _)| (s as usize) < start as usize);
        let mut start = start as usize;
        let mut end = end;
        // Merge with the range after.
        if i < self.free.len() && self.free[i].0 as usize == end {
            end += self.free[i].1 as usize;
            self.free.remove(i);
        }
        // Merge with the range before.
        if i > 0 && self.free[i - 1].0 as usize + self.free[i - 1].1 as usize == start {
            start = self.free[i - 1].0 as usize;
            self.free.remove(i - 1);
        }
        let i = self.free.partition_point(|&(s, _)| (s as usize) < start);
        self.free.insert(i, (start as u16, (end - start) as u16));
    }

    /// Whether module `n` is loaded.
    pub fn is_loaded(&self, n: u16) -> bool {
        self.modules.contains_key(&n)
    }

    /// Module `n`, if loaded.
    pub fn module(&self, n: u16) -> Option<&Module> {
        self.modules.get(&n)
    }

    /// The loaded modules, ascending by number.
    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.modules.values()
    }

    /// The body a global id is bound to, if any.
    pub fn resolve(&self, id: u16) -> Option<u16> {
        let body = self.table[id as usize];
        (body != 0).then_some(body)
    }

    /// The module and offset a flat address lies in, if it lies in a module.
    pub fn locate(&self, flat: u16) -> Option<Address> {
        self.modules
            .values()
            .find(|m| m.contains(flat))
            .map(|m| Address::new(m.number as u32, (flat - m.base) as u32))
    }

    /// The flat address a module-relative location names, if the module is
    /// loaded.
    pub fn flat(&self, addr: Address) -> Option<u16> {
        let m = self.modules.get(&(addr.module() as u16))?;
        let flat = m.base as usize + addr.offset() as usize;
        (flat < SPACE).then_some(flat as u16)
    }

    /// The address of a word's body by name alone, across the resident
    /// modules — what the kernel's own lookup (`ENVIRO.EXE` `12c8:0677`)
    /// answers for a conversation's branch that names a word. The original
    /// walks its word headers from the top of the arena down, so it finds
    /// the most recently loaded definition first; here the highest-placed
    /// one is taken, which is the same module unless one was erased from
    /// under another — see the placement departure.
    pub fn lookup(&self, name: &str) -> Option<Address> {
        self.modules
            .values()
            .filter_map(|m| m.word(name).map(|flat| (m.base, m.number, flat)))
            .max_by_key(|&(base, _, _)| base)
            .map(|(base, number, flat)| Address::new(number as u32, (flat - base) as u32))
    }

    /// The address of a word's body, by module and name.
    pub fn word(&self, module: u16, name: &str) -> Option<Address> {
        let m = self.modules.get(&module)?;
        m.word(name)
            .map(|flat| Address::new(module as u32, (flat - m.base) as u32))
    }

    /// Reads a cell at the address as given, odd or even: the `@` handler
    /// (`ENVIRO.EXE` file `0x1c9af`) adds the address to the arena base and
    /// reads there, with no mask. `!` is the one that aligns.
    pub fn fetch(&self, flat: u16) -> u16 {
        let a = flat as usize;
        u16::from_le_bytes([self.bytes[a], self.bytes[(a + 1) % SPACE]])
    }

    /// Writes a cell, the address aligned down to a cell first: the `!`
    /// handler (`ENVIRO.EXE` file `0x15f63`) shifts bit 0 out of the
    /// address before adding the arena base. The kernel's own words take
    /// their pointers from the same rule (the helper at file `0x17690` masks
    /// bit 0), which is why [`crate::AddressSpace`] reads aligned too.
    pub fn store(&mut self, flat: u16, value: u16) {
        let a = (flat & !1) as usize;
        let [lo, hi] = value.to_le_bytes();
        self.bytes[a] = lo;
        self.bytes[(a + 1) % SPACE] = hi;
    }

    /// Reads one byte.
    pub fn fetch_byte(&self, flat: u16) -> u8 {
        self.bytes[flat as usize]
    }

    /// Writes one byte.
    pub fn store_byte(&mut self, flat: u16, value: u8) {
        self.bytes[flat as usize] = value;
    }

    /// `n` bytes from `flat`, or an error if they run past the end of the
    /// space.
    pub fn read_bytes(&self, flat: u16, n: usize) -> Result<&[u8]> {
        self.bytes
            .get(flat as usize..flat as usize + n)
            .ok_or_else(|| Error::OutOfRange {
                addr: self.locate(flat).unwrap_or(Address::new(0, flat as u32)),
                at: None,
            })
    }

    /// Writes raw bytes at `flat` — what `GET` does with a block.
    pub fn write_bytes(&mut self, flat: u16, bytes: &[u8]) -> Result<()> {
        let end = flat as usize + bytes.len();
        if end > SPACE {
            return Err(Error::OutOfRange {
                addr: self.locate(flat).unwrap_or(Address::new(0, flat as u32)),
                at: None,
            });
        }
        self.bytes[flat as usize..end].copy_from_slice(bytes);
        Ok(())
    }

    /// A module's whole image as it stands in memory.
    pub fn image(&self, number: u16) -> Option<&[u8]> {
        let m = self.modules.get(&number)?;
        Some(&self.bytes[m.base as usize..m.base as usize + m.len as usize])
    }

    /// Puts a module image back — the length must match to the byte.
    pub fn restore(&mut self, number: u16, image: &[u8]) -> Result<()> {
        let Some(m) = self.modules.get(&number) else {
            return Err(Error::Savegame {
                what: format!("module {number} is not loaded"),
            });
        };
        if image.len() != m.len as usize {
            return Err(Error::Unsupported(format!(
                "module {number}: the saved image is {} bytes where the module is {}",
                image.len(),
                m.len
            )));
        }
        let base = m.base as usize;
        self.bytes[base..base + image.len()].copy_from_slice(image);
        Ok(())
    }
}

/// An execution set aside by [`Vm::park`], to be resumed later.
pub struct Context {
    ip: u16,
    ret: Vec<u16>,
    steps: u64,
}

/// The interpreter: two stacks, the flat memory, and where it is.
pub struct Vm {
    /// The memory, the modules and the word table.
    pub mem: Memory,
    binding: Binding,
    prims: Vec<prims::Prim>,
    /// The data stack: 16-bit cells, sign-extended. Every push wraps.
    pub data: Vec<i32>,
    ret: Vec<u16>,
    ip: u16,
    steps: u64,
    /// Guards against a runaway program; generous but finite.
    pub step_limit: u64,
    trace: Option<Vec<String>>,
    /// Deterministic source for `RANDOM`: a seeded LCG, never the clock or
    /// the operating system's entropy — see the 32-bit machine's, which says
    /// what that buys.
    rng: u32,
    nested: u32,
}

impl Vm {
    /// A machine bound to a kernel, with nothing loaded.
    pub fn new(binding: &Binding) -> Self {
        Self {
            mem: Memory::new(),
            prims: prims::dispatch_table(binding),
            binding: binding.clone(),
            data: Vec::new(),
            ret: Vec::new(),
            ip: 0,
            steps: 0,
            step_limit: 5_000_000,
            trace: None,
            rng: 0x1234_5678,
            nested: 0,
        }
    }

    /// Loads a module — `=>GET` — and answers where it was placed.
    pub fn load(&mut self, item: &[u8], parsed: &ScrModule) -> Result<u16> {
        self.mem.load(item, parsed)
    }

    /// Unloads a module — `=>ERASE`.
    pub fn unload(&mut self, number: u16) -> bool {
        self.mem.unload(number)
    }

    /// The kernel this machine is bound to.
    pub fn binding(&self) -> &Binding {
        &self.binding
    }

    /// The kernel word's name for an ordinal.
    pub fn ordinal_name(&self, ordinal: u32) -> Option<&str> {
        self.binding.name(ordinal)
    }

    /// Starts recording every executed word, with where it was.
    pub fn start_trace(&mut self) {
        self.trace.get_or_insert_with(Vec::new);
    }

    /// What has been recorded, oldest first, or `None` if nothing is.
    pub fn trace(&self) -> Option<&[String]> {
        self.trace.as_deref()
    }

    /// Where the machine stands, as a module-relative location — module 0
    /// and the flat address when it stands outside every module.
    pub fn here(&self) -> Address {
        self.mem
            .locate(self.ip)
            .unwrap_or(Address::new(0, self.ip as u32))
    }

    /// The flat address of a location, or the error for a module that is not
    /// loaded.
    fn flat_of(&self, addr: Address) -> Result<u16> {
        self.mem.flat(addr).ok_or(Error::NoSuchModule {
            module: addr.module(),
            at: self.here(),
            cell: addr.0,
        })
    }

    /// Points the machine at a word without running it.
    pub fn start(&mut self, start: Address) -> Result<()> {
        self.ip = self.flat_of(start)?;
        self.ret.clear();
        self.steps = 0;
        Ok(())
    }

    /// Runs the word at `start` to completion.
    ///
    /// Only for hosts that never ask to pause; a host that does gets
    /// [`Error::Suspended`].
    pub fn call(&mut self, start: Address, host: &mut dyn Host<Vm>) -> Result<()> {
        self.start(start)?;
        match self.resume(host)? {
            Run::Done => Ok(()),
            Run::Yielded => Err(Error::Suspended { at: self.here() }),
        }
    }

    /// Runs the kernel word that is running again on the next resume.
    ///
    /// The interpreter steps past a cell before it hands the word to the
    /// host, so this steps back. It is what a word needs that cannot finish
    /// in one go: the original blocks inside its own handler until a click
    /// answers it, and there is nowhere to block here — the frame has to end
    /// and the word has to be asked again.
    pub fn repeat_word(&mut self) {
        self.ip = self.ip.wrapping_sub(CELL);
    }

    /// Lifts the current execution out of the machine, leaving it free. The
    /// data stack stays, as it does in the re-entrant interpreter call the
    /// original's `ANIMPLAY` makes.
    pub fn park(&mut self) -> Context {
        Context {
            ip: self.ip,
            ret: std::mem::take(&mut self.ret),
            steps: self.steps,
        }
    }

    /// Puts a parked execution back.
    pub fn unpark(&mut self, saved: Context) {
        self.ip = saved.ip;
        self.ret = saved.ret;
        self.steps = saved.steps;
    }

    /// Runs a word to completion on the machine's own memory and data stack,
    /// with a return stack of its own, then puts the interrupted execution
    /// back. Pausing is suppressed for the duration, as in the 32-bit machine.
    pub fn call_nested(&mut self, addr: Address, host: &mut dyn Host<Vm>) -> Result<()> {
        let flat = self.flat_of(addr)?;
        let saved = self.park();
        self.ip = flat;
        self.ret.clear();
        self.steps = 0;
        self.nested += 1;
        let outcome = match self.resume(host) {
            Ok(Run::Done) => Ok(()),
            Ok(Run::Yielded) => Err(Error::Suspended { at: self.here() }),
            Err(e) => Err(e),
        };
        self.nested -= 1;
        self.unpark(saved);
        outcome
    }

    /// Runs until the word returns or the host asks to pause.
    pub fn resume(&mut self, host: &mut dyn Host<Vm>) -> Result<Run> {
        loop {
            self.steps += 1;
            if self.steps > self.step_limit {
                return Err(Error::StepLimit(self.step_limit));
            }
            let here = self.here();
            let cell = self.mem.fetch(self.ip);
            self.ip = self.ip.wrapping_add(CELL);

            if cell & KERNEL_BIT != 0 {
                let ordinal = (cell & ORDINAL_MASK) as u32;
                let done = self.step_primitive(ordinal, here, host)?;
                if done {
                    match self.ret.pop() {
                        Some(a) => self.ip = a,
                        None => return Ok(Run::Done),
                    }
                }
                if let Some((raw, args)) = host.pending_call() {
                    let Some(target) = crate::Machine::callback_target(self, raw) else {
                        return Err(Error::UnboundWord {
                            id: raw as u16,
                            at: self.here(),
                        });
                    };
                    let flat = self.flat_of(target)?;
                    for v in args {
                        self.push(v);
                    }
                    self.ret.push(self.ip);
                    self.ip = flat;
                }
                if self.nested == 0 && host.wants_pause() {
                    return Ok(Run::Yielded);
                }
                continue;
            }

            // A call to a global word id, through the table `=>GET` fills.
            let Some(body) = self.mem.resolve(cell) else {
                return Err(Error::UnboundWord { id: cell, at: here });
            };
            self.ret.push(self.ip);
            self.ip = body;
        }
    }

    /// Records an executed word, with where it was.
    fn note(&mut self, ordinal: u32) {
        if self.trace.is_none() {
            return;
        }
        let here = self.here();
        let what = self.ordinal_name(ordinal).unwrap_or("?").to_string();
        if let Some(t) = self.trace.as_mut() {
            t.push(format!("{here:>12}  {what}"));
        }
    }

    fn pop(&mut self, word: &'static str) -> Result<i32> {
        self.data.pop().ok_or(Error::StackUnderflow {
            word,
            at: self.here(),
        })
    }

    /// Pushes a value, wrapped to the 16-bit cell and sign-extended.
    fn push(&mut self, v: i32) {
        self.data.push((v as i16) as i32);
    }

    /// Reads the inline operand that follows the current instruction:
    /// its flat address and its value.
    fn operand(&mut self) -> (u16, i16) {
        let at = self.ip;
        let v = self.mem.fetch(at) as i16;
        self.ip = at.wrapping_add(CELL);
        (at, v)
    }

    fn next_rng(&mut self) -> u32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.rng
    }
}

/// Everything a caller needs to run a module's word by name.
pub fn word_address(vm: &Vm, module: u16, word: &str) -> Option<Address> {
    vm.mem.word(module, word)
}

impl crate::AddressSpace for Memory {
    fn fetch_cell(&self, raw: i32) -> Result<i32> {
        Ok((self.fetch(raw as u16 & !1) as i16) as i32)
    }

    fn store_cell(&mut self, raw: i32, value: i32) -> Result<()> {
        self.store(raw as u16, value as u16);
        Ok(())
    }

    fn fetch_byte(&self, raw: i32) -> Result<u8> {
        Ok(Memory::fetch_byte(self, raw as u16))
    }

    fn read_bytes(&self, raw: i32, n: usize) -> Result<Vec<u8>> {
        Memory::read_bytes(self, raw as u16, n).map(<[u8]>::to_vec)
    }

    fn write_bytes(&mut self, raw: i32, bytes: &[u8]) -> Result<()> {
        Memory::write_bytes(self, raw as u16, bytes)
    }

    /// Flat addresses add and wrap at 16 bits, like the stack cells they are.
    fn offset(&self, raw: i32, bytes: i32) -> i32 {
        ((raw as u16).wrapping_add(bytes as u16) as i16) as i32
    }

    fn cell_size(&self) -> i32 {
        CELL as i32
    }

    /// A callback is a global word id: it can be run if a loaded module
    /// binds it. What the 16-bit `NEWSETDESC` and `SDWORD` handlers check
    /// beyond that is unread; 0 and -1 clear the field as their 32-bit
    /// counterparts do, and the sixth argument of every `NEWSETDESC` in the
    /// game is -1.
    fn callable(&self, raw: i32) -> i32 {
        if raw == 0 || raw == -1 {
            return 0;
        }
        self.resolve(raw as u16).map_or(0, |_| raw)
    }

    fn is_live(&self, raw: i32) -> bool {
        self.locate(raw as u16).is_some()
    }

    fn module_image(&self, module: u32) -> Option<Vec<u8>> {
        self.image(module as u16).map(|b| b.to_vec())
    }

    fn restore_module(&mut self, module: u32, image: &[u8]) -> Result<()> {
        self.restore(module as u16, image)
    }
}

impl crate::Machine for Vm {
    type Context = Context;

    fn start(&mut self, at: Address) -> Result<()> {
        Vm::start(self, at)
    }

    fn resume(&mut self, host: &mut dyn Host<Self>) -> Result<Run> {
        Vm::resume(self, host)
    }

    fn call_nested(&mut self, at: Address, host: &mut dyn Host<Self>) -> Result<()> {
        Vm::call_nested(self, at, host)
    }

    fn park(&mut self) -> Context {
        Vm::park(self)
    }

    fn unpark(&mut self, saved: Context) {
        Vm::unpark(self, saved);
    }

    fn word_address(&self, module: u32, name: &str) -> Option<Address> {
        word_address(self, module as u16, name)
    }

    /// A variable's body is `_PutAdr` and its cell: the value is one cell
    /// past the body's start.
    fn variable(&self, word: Address) -> Option<i32> {
        let flat = self.mem.flat(word)?;
        Some((self.mem.fetch(flat.wrapping_add(CELL)) as i16) as i32)
    }

    fn set_variable(&mut self, word: Address, value: i32) -> Result<()> {
        let flat = self.mem.flat(word).ok_or(Error::NoSuchModule {
            module: word.module(),
            at: self.here(),
            cell: word.0,
        })?;
        self.mem.store(flat.wrapping_add(CELL), value as u16);
        Ok(())
    }

    /// A stored callback is a global word id, resolved through the table.
    fn callback_target(&self, raw: i32) -> Option<Address> {
        let flat = self.mem.resolve(raw as u16)?;
        self.mem.locate(flat)
    }

    fn data(&mut self) -> &mut Vec<i32> {
        &mut self.data
    }

    fn space(&self) -> &dyn crate::AddressSpace {
        &self.mem
    }

    fn space_mut(&mut self) -> &mut dyn crate::AddressSpace {
        &mut self.mem
    }
}
