//! The 32-bit machine: the address model, the module memory and the
//! interpreter of `ENGINE.EXE` V0.06.06/R109, as Dunkle Schatten 2 runs on it.

use std::collections::BTreeMap;

use motionvm_motion_formats::m32::le::{KernelWord, TAG_KERNEL};
use motionvm_motion_formats::m32::scr::ScrModule;

use crate::{Address, Error, Host, Loop, Result, Run, prims};

mod prims32;

/// Bytes per cell.
pub const CELL: u32 = 4;

impl Address {
    /// Builds an address from a call cell, whose offset counts cells.
    pub fn from_call(cell: u32) -> Self {
        Self::new(cell >> 16, (cell & 0xffff) * CELL)
    }

    /// The next cell along.
    pub fn next(self) -> Self {
        Self::new(self.module(), self.offset() + CELL)
    }

    /// This address moved by `cells` cells.
    pub fn plus_cells(self, cells: u32) -> Self {
        Self::new(self.module(), self.offset() + cells * CELL)
    }
}

/// Where a module's addressable memory starts inside its file.
const ADDRESS_BASE: usize = 0x30;

/// One module's cells, addressable and writable.
pub struct Module {
    /// The module number, as the resource container and every call cell name it.
    pub number: u32,
    pub(crate) cells: Vec<u32>,
    /// Word names by cell offset, for tracing and for `run` by name.
    names: BTreeMap<u32, String>,
}

impl Module {
    /// Loads a module from its raw bytes plus the parse used for word names.
    ///
    /// The addressable memory is the *whole* module image from `ADDRESS_BASE`
    /// on, not just the dictionary: modules carry data past it — dialogue
    /// tables and the like — and the code reaches into it with ordinary
    /// addresses.
    pub fn load(item: &[u8], parsed: &ScrModule) -> Self {
        let cells = item[ADDRESS_BASE..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&c| u32::from_le_bytes(c))
            .collect();
        let names = parsed
            .entries
            .iter()
            .map(|e| (e.call_offset(), e.name.clone()))
            .collect();
        Self {
            number: parsed.module,
            cells,
            names,
        }
    }

    /// The address of a word by name.
    ///
    /// `Entry::call_offset` counts cells, because that is what a call cell
    /// holds; an [`Address`] counts bytes.
    pub fn word(&self, name: &str) -> Option<Address> {
        self.names
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(&cells, _)| Address::new(self.number, cells * CELL))
    }

    /// The name of the word whose body starts at this byte offset, if any.
    pub fn name_at(&self, byte_offset: u32) -> Option<&str> {
        self.names.get(&(byte_offset / CELL)).map(String::as_str)
    }

    /// The module's memory, for whoever wants to write it all down at once.
    pub fn cells(&self) -> &[u32] {
        &self.cells
    }

    /// Puts a whole module image back, as `=>GETAS` does when a savegame loads.
    ///
    /// The length must match to the cell, and this is the only place that says
    /// so. Everything the interpreter carries across the call — `ip`, the
    /// return stack, a parked context — is a packed `(module << 16) | offset`
    /// and not a pointer into this vector, so replacing the contents underneath
    /// a running word is safe *as long as the addresses still mean the same
    /// thing*. Shorten the image and every one of them silently moves.
    ///
    /// It is a real case, not a hypothetical: the savegame is loaded from
    /// inside `ICTRL`, which lives in module 4, and module 4 is one of the
    /// modules being restored. The original does exactly the same, and gets
    /// away with it because a module's code cells are identical between saving
    /// and loading — only its variables differ.
    pub fn restore(&mut self, cells: &[u32]) -> Result<()> {
        if cells.len() != self.cells.len() {
            return Err(Error::Unsupported(format!(
                "module {}: the saved image is {} cells where the module is {}",
                self.number,
                cells.len(),
                self.cells.len()
            )));
        }
        self.cells.copy_from_slice(cells);
        Ok(())
    }
}

/// An execution set aside by [`Vm::park`], to be resumed later.
///
/// Holds no data stack on purpose: see [`Vm::park`].
pub struct Context {
    ip: Address,
    ret: Vec<Address>,
    steps: u64,
}

/// Branch target arithmetic.
///
/// Every one of these was read off a module the original compiler produced:
///
/// ```text
/// : T 1 IF 2 ELSE 3 ENDIF ;      _CheckIf   operand@3 dist 5 -> 8   (the ELSE arm)
///                                _CheckElse operand@7 dist 3 -> 10  (past the ENDIF)
/// : T 1 =IF 2 ENDIF ;            _CheckEIf  operand@3 dist 3 -> 6
/// : T BEGIN 1 UNTIL ;            _Until     operand@3 dist 3 -> 0   (back to BEGIN)
/// : T BEGIN 1 WHILE 2 REPEAT ;   _LoopBreak operand@3 dist 5 -> 8
///                                _Repeat    operand@7 dist 7 -> 0
/// : T 0 10 DO 1 LOOP ;           _LoopEnd   operand@8 dist 3 -> 5   (loop body)
/// : T 0 10 DO 1 2 +LOOP ;        _AddLoop   operand@10 dist 5 -> 5
/// ```
///
/// Which way a given word counts is [`motionvm_motion_formats::Inline::is_forward`],
/// read off the kernel the machine was bound to rather than assumed, so a
/// build that hands the branch words other ordinals still decodes.
pub mod branch {
    use motionvm_motion_formats::Inline;

    /// The cell a branch jumps to, given where its operand sits.
    pub fn target(inline: &Inline, ordinal: u32, operand_at: u32, distance: u32) -> u32 {
        if inline.is_forward(ordinal) {
            operand_at.wrapping_add(distance)
        } else {
            operand_at.wrapping_sub(distance)
        }
    }
}

/// The loaded modules, addressable as one space.
#[derive(Default)]
pub struct Memory {
    modules: BTreeMap<u32, Module>,
    /// Addresses in modules that are not loaded, with how often each was
    /// touched. See [`Memory::fetch`] — the game does this on purpose-ish, and
    /// it must be visible rather than silent.
    ///
    /// The one `RefCell` on the machine: it keeps `Memory` `Send` but not
    /// `Sync`, which is why the machine promises `Send` alone — a `Sync`
    /// wish starts here.
    loose: std::cell::RefCell<BTreeMap<Address, u32>>,
    /// Where the last successful lookup landed — the globals `0xee668` and
    /// `0xee66c`, kept because action 6 of a dialogue branch (0x7bece) builds
    /// its call address out of them without checking the search's answer.
    pub last_hit: Option<Address>,
}

impl Memory {
    /// Adds a module, replacing any module of the same number.
    pub fn insert(&mut self, m: Module) {
        self.modules.insert(m.number, m);
    }

    /// Module `n`, if it is loaded.
    pub fn module(&self, n: u32) -> Option<&Module> {
        self.modules.get(&n)
    }

    /// Module `n`, to be written to.
    pub fn module_mut(&mut self, n: u32) -> Option<&mut Module> {
        self.modules.get_mut(&n)
    }

    /// Whether module `n` is loaded.
    pub fn contains(&self, n: u32) -> bool {
        self.modules.contains_key(&n)
    }

    /// The first word of this name anywhere, as the lookup at 0x61596 finds it.
    ///
    /// That routine hashes the name's length and its characters 0, 1, 2 and 5
    /// (0x615a4-0x61635), then walks the loaded modules — `0..[0xdb027]` — and
    /// searches each one's buckets. The hashing is only speed; the answer is
    /// "the first word of that name, module by module". It leaves the module in
    /// `0xee668` and the cell in `0xee66c`, and the caller builds an address
    /// out of them.
    ///
    /// One difference worth knowing: the original walks its module table at
    /// `0xee6d0` (stride 0x30, count `0xdb027`, module number at +0x10) in the
    /// order entries were made, this map is keyed by module number. As long as
    /// a name occurs once that is the same answer; where it occurs twice the
    /// choice could differ. Every duplicate seen so far lives in location
    /// modules that are never loaded together.
    /// Takes `&mut self` because a hit is *stored*: the original keeps the
    /// last successful lookup in two globals and a dialogue branch reads them
    /// back without checking the search's own answer. See [`Memory::last_hit`].
    /// The borrow is the game's behavior, not this interpreter's bookkeeping.
    pub fn lookup(&mut self, name: &str) -> Option<Address> {
        let hit = self.modules.values().find_map(|m| m.word(name));
        if hit.is_some() {
            self.last_hit = hit;
        }
        hit
    }

    /// Reads a cell of module memory.
    ///
    /// A module that is not loaded answers **zero** instead of failing. That
    /// is not leniency for its own sake: the original's `@` (0x626fc) checks
    /// nothing at all — it works out
    /// `[[[0xEE6D0 + [0xEE6C0 + (v>>16)·4]·0x30] + 0x14] + (v & 0xffff)]` and
    /// reads whatever is there. The game relies on that. Location 5's macro
    /// tests `KRSCHRZIEHE @ 0 =`, but `KRSCHRZIEHER` is the *item number* 24,
    /// not the flag variable `_?KRSCHRZIEHER` the author meant, so the read
    /// lands on address 24 — module 0, the kernel's own module, which we do
    /// not have. The original shrugs and reads a kernel cell; refusing to
    /// answer stopped the game on five of its locations.
    ///
    /// What such a read *would* have answered is not knowable — module 0's
    /// data area is allocated at run time (0x5f39f) and filled by the kernel —
    /// so zero is the only value that can be justified. Every one of them is
    /// counted and reported rather than passing in silence.
    pub fn fetch(&self, addr: Address) -> Result<u32> {
        let Some(m) = self.modules.get(&addr.module()) else {
            self.note_loose(addr);
            return Ok(0);
        };
        m.cells
            .get((addr.offset() / CELL) as usize)
            .copied()
            .ok_or(Error::OutOfRange { addr, at: None })
    }

    /// The instruction fetch, which is *not* forgiving.
    ///
    /// Reading data through a stray pointer is something the game does; being
    /// about to execute one is something only a mistake of ours can produce,
    /// and it should say so.
    pub fn fetch_code(&self, addr: Address) -> Result<u32> {
        self.modules
            .get(&addr.module())
            .and_then(|m| m.cells.get((addr.offset() / CELL) as usize).copied())
            .ok_or(Error::OutOfRange { addr, at: None })
    }

    /// Writes a cell. A module that is not loaded swallows it, as above.
    pub fn store(&mut self, addr: Address, value: u32) -> Result<()> {
        if !self.modules.contains_key(&addr.module()) {
            self.note_loose(addr);
            return Ok(());
        }
        let cell = self
            .modules
            .get_mut(&addr.module())
            .and_then(|m| m.cells.get_mut((addr.offset() / CELL) as usize))
            .ok_or(Error::OutOfRange { addr, at: None })?;
        *cell = value;
        Ok(())
    }

    /// Remembers a touch of a module that is not loaded, so the run can say so.
    fn note_loose(&self, addr: Address) {
        *self.loose.borrow_mut().entry(addr).or_insert(0) += 1;
    }

    /// Every address in an unloaded module that was read or written, with how
    /// often. Empty in a healthy run.
    pub fn loose(&self) -> std::collections::BTreeMap<Address, u32> {
        self.loose.borrow().clone()
    }

    /// Reads one byte. Addresses are byte-granular, so this needs no alignment.
    pub fn fetch_byte(&self, addr: Address) -> Result<u8> {
        let word = self.fetch(Address::new(addr.module(), addr.offset() & !(CELL - 1)))?;
        Ok(word.to_le_bytes()[(addr.offset() % CELL) as usize])
    }

    /// A 32-bit read that does not care about alignment.
    ///
    /// [`Self::fetch`] addresses *cells* and rounds down, which is exactly
    /// right for the interpreter: every bytecode cell is aligned by
    /// construction. Native handlers are not so tidy. They translate the base
    /// of a record to a pointer — 0x67404 masks that to a cell — and then index
    /// with byte strides of their own: 0x12 for a conversation's answers, 0x22
    /// for its branches, neither of which divides by four. Every other entry
    /// therefore starts mid-cell, and reading it through `fetch` silently
    /// returns the neighboring bytes instead.
    pub fn fetch_unaligned(&self, addr: Address) -> Result<u32> {
        let byte = |i: u32| self.fetch_byte(Address::new(addr.module(), addr.offset() + i));
        Ok(u32::from_le_bytes([byte(0)?, byte(1)?, byte(2)?, byte(3)?]))
    }

    /// Writes one byte, by reading the cell around it and putting it back.
    ///
    /// Memory is addressed in cells; a byte write is therefore never atomic
    /// here, which matches what the original does with `C!`.
    pub fn store_byte(&mut self, addr: Address, value: u8) -> Result<()> {
        let aligned = Address::new(addr.module(), addr.offset() & !(CELL - 1));
        let mut word = self.fetch(aligned)?.to_le_bytes();
        word[(addr.offset() % CELL) as usize] = value;
        self.store(aligned, u32::from_le_bytes(word))
    }

    /// Writes raw bytes starting at a cell address.
    ///
    /// Resources are byte blobs — `GET` copies one into module memory — while
    /// addresses are cell-granular, so a write starts on a cell boundary and
    /// runs on byte by byte from there.
    pub fn write_bytes(&mut self, addr: Address, bytes: &[u8]) -> Result<()> {
        let m = self
            .modules
            .get_mut(&addr.module())
            .ok_or(Error::OutOfRange { addr, at: None })?;
        let start = (addr.offset() / CELL) as usize;
        let cells = bytes.len().div_ceil(CELL as usize);
        if start + cells > m.cells.len() {
            return Err(Error::OutOfRange { addr, at: None });
        }
        for (i, chunk) in bytes.chunks(CELL as usize).enumerate() {
            let mut word = m.cells[start + i].to_le_bytes();
            word[..chunk.len()].copy_from_slice(chunk);
            m.cells[start + i] = u32::from_le_bytes(word);
        }
        Ok(())
    }
}

/// The interpreter: two stacks, the loaded modules, and where it is.
///
/// `mem` and `data` are public because [`Host::word`] is handed the whole
/// machine and destructures them — the engine's words push and pop like any
/// other word does.
pub struct Vm {
    /// The loaded modules.
    pub mem: Memory,
    ordinals: BTreeMap<u32, String>,
    /// The ordinals of the words the compiler writes inline, as this kernel
    /// hands them out.
    ///
    /// The branch decoder reads its word set from here rather than from the
    /// ordinals `ENGINE.EXE` V0.06.06/R109 happens to use, because a kernel
    /// that numbered `_CheckIf` differently would still *bind* correctly — the
    /// dispatch table below is built from the same scan — and would then be
    /// decoded as the wrong construct, which is the kind of difference that
    /// runs for a while before it goes wrong. Only the inline set is kept: the
    /// names are already in `ordinals`.
    inline: motionvm_motion_formats::Inline,
    /// What each ordinal is, decided once when the machine is built.
    ///
    /// See [`prims::dispatch_table`]. The names in `ordinals` stay for the
    /// three things that genuinely want one — a trace, an error message, and
    /// handing a word over to the engine.
    prims: Vec<prims::Prim>,
    /// The data stack. Cells are `i32`: the game does signed arithmetic on
    /// them constantly, and reads them back as addresses just as often.
    pub data: Vec<i32>,
    ret: Vec<Address>,
    loops: Vec<Loop>,
    ip: Address,
    steps: u64,
    /// Guards against a runaway program; generous but finite.
    pub step_limit: u64,
    /// When set, every executed word is appended here — see
    /// [`Vm::start_trace`].
    trace: Option<Vec<String>>,
    /// When set, report every change to this cell and what caused it — see
    /// [`Vm::watch`].
    watch: Option<Address>,
    /// Deterministic source for `RANDOM`: a seeded LCG, never the clock or
    /// the operating system's entropy.
    ///
    /// The engine reads no wall clock anywhere, and this is the other half of
    /// that: a given input state has to render a given frame, every time, or
    /// comparing two renderings of a scene across a change proves nothing —
    /// which is how the drawing code in this project is verified.
    rng: u32,
    /// How deep inside [`Vm::call_nested`] the machine is. Non-zero means
    /// pausing is suppressed; see there.
    nested: u32,
}

impl Vm {
    /// A machine that knows the kernel's words but has no modules yet.
    ///
    /// The kernel table comes out of `ENGINE.EXE`; without it an ordinal is
    /// only a number and nothing can be named, traced or reported.
    pub fn new(kernel: &[KernelWord]) -> Self {
        Self::with_binding(&motionvm_motion_formats::m32::le::binding_of(kernel))
    }

    /// A machine over an already bound kernel.
    pub fn with_binding(binding: &motionvm_motion_formats::Binding) -> Self {
        let ordinals: BTreeMap<u32, String> = binding.words.iter().cloned().collect();
        let prims = prims::dispatch_table(binding);
        Self {
            mem: Memory::default(),
            ordinals,
            inline: binding.inline,
            prims,
            data: Vec::new(),
            ret: Vec::new(),
            loops: Vec::new(),
            ip: Address(0),
            steps: 0,
            step_limit: 5_000_000,
            trace: None,
            watch: None,
            nested: 0,
            rng: 0x1234_5678,
        }
    }

    /// Loads one script module out of its resource item.
    pub fn load(&mut self, item: &[u8], parsed: &ScrModule) {
        self.mem.insert(Module::load(item, parsed));
    }

    /// Module `n`, if it is loaded.
    pub fn module(&self, n: u32) -> Option<&Module> {
        self.mem.module(n)
    }

    /// Starts recording every executed word, with where it was.
    ///
    /// Diagnostics rather than state: nothing in the interpreter reads the
    /// trace back, and a run without one behaves identically. It is a method
    /// rather than a field so that switching it on is a thing a caller does,
    /// not a thing a caller may forget it did.
    pub fn start_trace(&mut self) {
        self.trace.get_or_insert_with(Vec::new);
    }

    /// What has been recorded, oldest first, or `None` if nothing is.
    pub fn trace(&self) -> Option<&[String]> {
        self.trace.as_deref()
    }

    /// Reports every change to one cell, and what caused it, into the trace.
    ///
    /// Module memory is code and data at once, so a stray store does not fail —
    /// it quietly rewrites an instruction, and the program goes wrong somewhere
    /// else entirely. Watching a cell is the only practical way to find the
    /// culprit, and the report is only useful among the words around it, so
    /// this turns the trace on as well.
    pub fn watch(&mut self, cell: Address) {
        self.watch = Some(cell);
        self.start_trace();
    }

    /// The kernel word's name for an ordinal.
    pub fn ordinal_name(&self, ordinal: u32) -> Option<&str> {
        self.ordinals.get(&ordinal).map(String::as_str)
    }

    /// What an ordinal is. Past the end of the table is the same answer as a
    /// gap in it: no kernel word has it.
    pub(crate) fn prim(&self, ordinal: u32) -> prims::Prim {
        self.prims
            .get(ordinal as usize)
            .copied()
            .unwrap_or(prims::Prim::Absent)
    }

    /// Reads one cell. See [`Memory::fetch`] for what an unloaded module answers.
    pub fn fetch(&self, addr: Address) -> Result<u32> {
        self.mem.fetch(addr)
    }

    /// Writes one cell.
    pub fn store(&mut self, addr: Address, value: u32) -> Result<()> {
        self.mem.store(addr, value)
    }

    /// Runs the word at `start` to completion.
    ///
    /// Only for hosts that never ask to pause — the oracle tests and anything
    /// with no engine behind it. A host that does pause gets [`Error::Suspended`]
    /// rather than a silent difference in behavior, because only the caller
    /// knows how to make time pass.
    pub fn call(&mut self, start: Address, host: &mut dyn Host<Vm>) -> Result<()> {
        self.start(start);
        match self.resume(host)? {
            Run::Done => Ok(()),
            Run::Yielded => Err(Error::Suspended { at: self.ip }),
        }
    }

    /// Lifts the current execution out of the machine, leaving it free.
    ///
    /// The original's `ANIMPLAY` is native code that runs the bytecode
    /// interpreter again from inside itself: a re-entrant call with a return
    /// stack of its own, while the data stack stays shared. That is how the
    /// game's main loop calls the controller every frame without the word that
    /// started it ever returning. Reproducing it needs a position that can be
    /// set aside and put back, which is all this is — and the data stack is
    /// deliberately not part of it, for the same reason it was shared there.
    pub fn park(&mut self) -> Context {
        Context {
            ip: self.ip,
            ret: std::mem::take(&mut self.ret),
            steps: self.steps,
        }
    }

    /// Puts a parked execution back, so the next `resume` continues it.
    pub fn unpark(&mut self, saved: Context) {
        self.ip = saved.ip;
        self.ret = saved.ret;
        self.steps = saved.steps;
    }

    /// Runs a word to completion on the machine's own memory and data stack,
    /// with a return stack of its own, then puts the interrupted execution
    /// back. This is the re-entrant interpreter call the original's native
    /// handlers make, and the reason `Host::word` is given the whole machine.
    ///
    /// A word called this way must not block: it runs inside a primitive that
    /// is itself mid-execution, and there is nowhere to park that. Nothing
    /// reached this way does — they set the pointer, lay the inventory out and
    /// suchlike.
    pub fn call_nested(&mut self, addr: Address, host: &mut dyn Host<Vm>) -> Result<()> {
        let saved = self.park();
        self.start(addr);
        // Pausing is suppressed for the duration. It models a *blocking word*
        // in the outer flow — the interpreter stopping where it stands while a
        // fade plays — and a re-entrant call is not that: the original's native
        // handlers run the interpreter again and it returns before they carry
        // on. Leaving the check in meant a callback launched during a
        // transition yielded on its first primitive and could never finish,
        // which is what `FINISHTEXT` ran into.
        self.nested += 1;
        let outcome = match self.resume(host) {
            Ok(Run::Done) => Ok(()),
            Ok(Run::Yielded) => Err(Error::Suspended { at: self.ip }),
            Err(e) => Err(e),
        };
        self.nested -= 1;
        self.unpark(saved);
        outcome
    }

    /// Points the machine at a word without running it.
    pub fn start(&mut self, start: Address) {
        self.ip = start;
        self.ret.clear();
        self.steps = 0;
    }

    /// Runs until the word returns or the host asks to pause.
    ///
    /// Pausing is how the original's blocking words are reproduced. Four of them
    /// — `FADEOUT`, `FADEIN`, `ANIMPLAY`, `ANIMSIM` — run their own frame loop
    /// inside the handler, so the bytecode does not continue until they are
    /// finished. Reproducing that needs the interpreter to stop mid-word and
    /// pick up exactly where it left off, which works because everything that
    /// makes up its position — `ip`, the return stack, the loop stack — lives in
    /// the struct rather than on Rust's stack.
    ///
    /// The pause takes effect *after* the word that asked for it and before the
    /// next cell. That matters: the cells right after `FADEOUT` are the ones
    /// that swap the picture, so stopping there is what keeps the old picture on
    /// screen while it fades out — no copy of it required.
    pub fn resume(&mut self, host: &mut dyn Host<Vm>) -> Result<Run> {
        let mut watched = self.watch.and_then(|a| self.fetch(a).ok());
        loop {
            if let (Some(addr), Some(before)) = (self.watch, watched) {
                let now = self.fetch(addr).ok();
                if now != Some(before) {
                    let word = self
                        .trace
                        .as_ref()
                        .and_then(|t| t.last().cloned())
                        .unwrap_or_else(|| "?".into());
                    // Into the trace rather than onto stderr. A library that
                    // prints has decided for its caller where the report goes
                    // and when — and here it is the wrong answer twice over:
                    // the report belongs *between* the words that surround it,
                    // which is exactly where the trace puts it, and a caller
                    // that wants it on stderr can still send it there. This is
                    // why [`Vm::watch`] turns the trace on: without one there
                    // would be nowhere to say it.
                    let ip = self.ip;
                    let steps = self.steps;
                    if let Some(t) = self.trace.as_mut() {
                        t.push(format!(
                            "{ip:>12}  !! watch {addr}: {before:#010x} -> {:#010x} at step {steps} (after {word})",
                            now.unwrap_or(0),
                        ));
                    }
                    watched = now;
                }
            }
            self.steps += 1;
            if self.steps > self.step_limit {
                return Err(Error::StepLimit(self.step_limit));
            }
            let here = self.ip;
            // Jumping into a module that is not there is a different matter
            // from reading through a stray pointer: the game does the second
            // on purpose-ish, the first only by mistake. So this stays strict,
            // and it names the word that jumped rather than repeating the
            // address it landed on.
            let cell = self.mem.fetch_code(here).map_err(|e| match e {
                Error::OutOfRange { addr, at: None } => Error::OutOfRange {
                    addr,
                    at: self.ret.last().copied(),
                },
                other => other,
            })?;
            self.ip = here.next();

            if cell == 0 {
                match self.ret.pop() {
                    Some(a) => self.ip = a,
                    None => return Ok(Run::Done),
                }
                continue;
            }

            if cell >> 16 == TAG_KERNEL >> 16 {
                let ordinal = cell & 0xffff;
                // A read that ran off the end of a module knows the address it
                // wanted but not the word that wanted it; this is the one
                // place that knows both.
                let done = self
                    .step_primitive(ordinal, here, host)
                    .map_err(|e| match e {
                        Error::OutOfRange { addr, at: None } => Error::OutOfRange {
                            addr,
                            at: Some(here),
                        },
                        other => other,
                    })?;
                if done {
                    // The primitive was a complete word behavior and returned.
                    match self.ret.pop() {
                        Some(a) => self.ip = a,
                        None => return Ok(Run::Done),
                    }
                }
                if let Some((raw, args)) = host.pending_call() {
                    let Some(target) = crate::Machine::callback_target(self, raw) else {
                        return Err(Error::Unsupported(format!(
                            "a pending call to {raw:#x}, which names no word"
                        )));
                    };
                    self.data.extend_from_slice(&args);
                    self.ret.push(self.ip);
                    self.ip = target;
                }
                if self.nested == 0 && host.wants_pause() {
                    return Ok(Run::Yielded);
                }
                continue;
            }

            // A call cell counts cells; everything else counts bytes.
            let target = Address::from_call(cell);
            if !self.mem.contains(target.module()) {
                return Err(Error::NoSuchModule {
                    module: target.module(),
                    at: here,
                    cell,
                });
            }
            self.ret.push(self.ip);
            self.ip = target;
        }
    }

    /// Records an executed word, with where it was.
    ///
    /// The address is the point of it: a run of `@`, `AND` and `_CheckIf` looks
    /// the same everywhere, and without a position there is no way to say which
    /// of a long word's conditions the trace is standing in.
    ///
    /// Takes the ordinal and looks the name up here, rather than being handed
    /// one. Turning an ordinal back into a name costs a map lookup and a heap
    /// allocation, and the caller cannot know whether anyone is listening — a
    /// trace is off in every normal run, so the cost has to be paid inside the
    /// `is_none` check and not before it.
    fn note(&mut self, ordinal: u32) {
        if self.trace.is_none() {
            return;
        }
        let ip = self.ip;
        let what = self.ordinal_name(ordinal).unwrap_or("?").to_string();
        if let Some(t) = self.trace.as_mut() {
            t.push(format!("{ip:>12}  {what}"));
        }
    }

    fn pop(&mut self, word: &'static str) -> Result<i32> {
        self.data
            .pop()
            .ok_or(Error::StackUnderflow { word, at: self.ip })
    }

    fn push(&mut self, v: i32) {
        self.data.push(v);
    }

    /// Reads the inline operand that follows the current instruction.
    fn operand(&mut self) -> Result<(u32, u32)> {
        let at = self.ip;
        let v = self.mem.fetch_code(at)?;
        self.ip = at.next();
        Ok((at.offset(), v))
    }

    fn next_rng(&mut self) -> u32 {
        // Any decent generator will do; what matters is that it is repeatable.
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.rng
    }
}

/// Everything a caller needs to run a module's word by name.
pub fn word_address(vm: &Vm, module: u32, word: &str) -> Option<Address> {
    vm.module(module)?.word(word)
}

pub use prims32::IMPLEMENTED;

impl crate::AddressSpace for Memory {
    fn fetch_cell(&self, raw: i32) -> Result<i32> {
        self.fetch(Address(raw as u32)).map(|v| v as i32)
    }

    fn store_cell(&mut self, raw: i32, value: i32) -> Result<()> {
        self.store(Address(raw as u32), value as u32)
    }

    fn fetch_byte(&self, raw: i32) -> Result<u8> {
        Memory::fetch_byte(self, Address(raw as u32))
    }

    fn read_bytes(&self, raw: i32, n: usize) -> Result<Vec<u8>> {
        let at = Address(raw as u32);
        (0..n as u32)
            .map(|i| Memory::fetch_byte(self, Address::new(at.module(), at.offset() + i)))
            .collect()
    }

    fn write_bytes(&mut self, raw: i32, bytes: &[u8]) -> Result<()> {
        Memory::write_bytes(self, Address(raw as u32), bytes)
    }

    /// The module half stays, the offset moves: a record's fields and a
    /// table's rows never cross a module.
    fn offset(&self, raw: i32, bytes: i32) -> i32 {
        let at = Address(raw as u32);
        Address::new(at.module(), at.offset().wrapping_add(bytes as u32)).0 as i32
    }

    fn cell_size(&self) -> i32 {
        CELL as i32
    }

    /// `NEWSETDESC` does not take what it is given. Zero and -1 clear the
    /// field outright (0x70c83-0x70c8f), and anything else is put through
    /// three checks before the store at 0x70cc9 — the first two of which are
    /// mirrored here: 0x68241 takes the module from bits 16..30 and rejects
    /// the address if that module is not loaded, and 0x682a0 additionally
    /// rejects an offset at or past the module's own cell count (0x68314). A
    /// cell that answers to [`Memory::fetch`] has passed both.
    ///
    /// It matters because the field is used without checking: the frame walk
    /// hands +0x14 straight to the interpreter once the wait runs out. Not
    /// every call site passes an address — location 1 hands over a plain 177
    /// — and stored unfiltered that is a jump to nowhere.
    ///
    /// The handler's third check (0x68367, on the word header behind the
    /// address) is not mirrored: these two already reject everything seen,
    /// and the header's layout would have to be guessed.
    ///
    /// The module half is masked here and nowhere else. `Address::new` takes
    /// a module number from code that knows it is one; this takes an `i32`
    /// out of module memory that may be anything at all, and a value with
    /// rubbish in its top bits would otherwise become an address in a module
    /// number that cannot exist rather than being rejected. Fourteen bits is
    /// far more than the game's numbering needs — the highest module it
    /// names is 330.
    fn callable(&self, raw: i32) -> i32 {
        if raw == 0 || raw == -1 {
            return 0;
        }
        let v = raw as u32;
        let addr = Address::new((v >> 16) & 0x3fff, v & 0xffff);
        if self.fetch(addr).is_ok() { raw } else { 0 }
    }

    fn is_live(&self, raw: i32) -> bool {
        self.contains(raw as u32 >> 16)
    }

    fn module_image(&self, module: u32) -> Option<Vec<u8>> {
        self.module(module)
            .map(|m| m.cells().iter().flat_map(|c| c.to_le_bytes()).collect())
    }

    fn restore_module(&mut self, module: u32, image: &[u8]) -> Result<()> {
        let m = self.module_mut(module).ok_or_else(|| Error::Savegame {
            what: format!("module {module} is not loaded"),
        })?;
        let cells: Vec<u32> = image
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        m.restore(&cells)
    }
}

impl crate::Machine for Vm {
    type Context = Context;

    fn start(&mut self, at: Address) -> Result<()> {
        Vm::start(self, at);
        Ok(())
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
        word_address(self, module, name)
    }

    /// A variable's body opens with the `_PutAdr` that pushes its own
    /// address, so the value sits in the cell right after it.
    fn variable(&self, word: Address) -> Option<i32> {
        self.fetch(word.next()).ok().map(|v| v as i32)
    }

    fn set_variable(&mut self, word: Address, value: i32) -> Result<()> {
        self.store(word.next(), value as u32)
    }

    /// A stored callback is the packed address itself.
    fn callback_target(&self, raw: i32) -> Option<Address> {
        Some(Address::new(raw as u32 >> 16, raw as u32 & 0xffff))
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
