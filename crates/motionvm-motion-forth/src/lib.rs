//! The Forth virtual machines that execute the games' compiled modules —
//! one per engine generation.
//!
//! The two generations of MOTION compile the same language to the same kind
//! of threaded code and run it on two different machines: 32-bit cells,
//! packed `(module << 16) | offset` addresses, a zero cell as the return and
//! `0x4000xxxx` kernel cells in `ENGINE.EXE` ([`m32`]); 16-bit cells, one
//! flat address space with a word-id table, the kernel word `##` as the
//! return and `0x8000 | ordinal` kernel cells in `ENVIRO.EXE` ([`m16`]). Each
//! machine is its own interpreter, checkable against its own binary.
//!
//! The crate root holds what the two share: [`Address`], the neutral
//! module-and-offset location every report and every host call uses;
//! [`Error`] and [`Run`]; the [`Host`] boundary through which the engine's
//! words arrive; [`AddressSpace`], the engine's view of a machine's memory,
//! and [`Machine`], the driver's view of a machine; the dispatch of kernel
//! ordinals onto the primitives the interpreters own, built from a
//! [`motionvm_motion_formats::Binding`] — and the primitives' names, which are the
//! same in both kernels where they exist. An unqualified name here holds for
//! both machines.

pub mod m16;
pub mod m32;
mod prims;

/// A location in a loaded module: the module number in the high half, a
/// byte offset in the low half.
///
/// This is the one address type both machines report in and accept from a
/// host. For the 32-bit machine it is also the runtime value — the packed
/// `(module << 16) | byte offset` that `@` decodes, with the offset counting
/// bytes from file offset 0x30 of the module; see [`m32`] for the measurement
/// and for the call-cell encoding that counts cells instead. For the 16-bit
/// machine it is a name for a flat address — the module's base plus the
/// offset, counted from the start of the module image — and [`m16::Memory`]
/// translates both ways.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address(pub u32);

impl Address {
    /// An address from a module number and a **byte** offset within it.
    ///
    /// The offset is masked to sixteen bits, which is where it lives in the
    /// packed representation. `module` is not masked — see `Engine::callable`,
    /// which does mask it, for the one place the two notions differ.
    pub fn new(module: u32, byte_offset: u32) -> Self {
        Self((module << 16) | (byte_offset & 0xffff))
    }

    /// The module half.
    pub fn module(self) -> u32 {
        self.0 >> 16
    }

    /// Byte offset from the module's address base.
    pub fn offset(self) -> u32 {
        self.0 & 0xffff
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{:#x}", self.module(), self.offset())
    }
}

/// Everything that can stop a run.
///
/// Each variant carries the address it happened at, because a Forth backtrace
/// is not recoverable after the fact: the return stack is unwound by the time
/// anyone asks. `at` is what makes an error name a place in the game's own
/// bytecode rather than a place in this interpreter.
#[derive(Debug)]
pub enum Error {
    /// A primitive that has not been implemented yet. Named, never silent.
    Unimplemented {
        /// The kernel ordinal that was reached.
        ordinal: u32,
        /// Its name in the kernel word table, so the report is readable.
        name: String,
        /// Where it was reached from.
        at: Address,
    },
    /// An ordinal that is in no kernel word table — so not a word at all, and
    /// usually a sign that execution has wandered into data.
    UnknownOrdinal {
        /// The ordinal as it was decoded.
        ordinal: u32,
        /// Where it was decoded.
        at: Address,
    },
    /// A word wanted more arguments than the stack held.
    StackUnderflow {
        /// The word that asked, by name.
        word: &'static str,
        /// Where it was running.
        at: Address,
    },
    /// A call or a read into a module that is not loaded.
    ///
    /// Not always fatal in the original either: see [`m32::Memory::fetch`], which
    /// answers zero for data reads rather than raising this.
    NoSuchModule {
        /// The module number asked for.
        module: u32,
        /// Where it was asked for.
        at: Address,
        /// The call cell as it stood, kept because its offset half is often the
        /// only clue to what was meant.
        cell: u32,
    },
    /// A call to a global word id that no loaded module binds — the 16-bit
    /// machine's counterpart of [`Error::NoSuchModule`]: `=>GET` has not
    /// loaded the module that defines it, or `=>ERASE` has dropped it.
    UnboundWord {
        /// The id the cell named.
        id: u16,
        /// Where it was named.
        at: Address,
    },
    /// `=>GET` found no room in the 16-bit machine's 64 KiB space.
    OutOfMemory {
        /// The module that did not fit.
        module: u32,
        /// Its size in bytes.
        need: usize,
    },
    /// A read or write that ran past the end of a module that *is* loaded.
    ///
    /// `at` is filled in by [`m32::Vm::resume`] on the way out, because the memory
    /// itself has no idea which word was running.
    OutOfRange {
        /// The address that was out of range.
        addr: Address,
        /// The word that was running, once [`m32::Vm::resume`] has filled it in.
        at: Option<Address>,
    },
    /// `/` or `MOD` with a zero divisor.
    ///
    /// What the original does here is not established — a Watcom-built DOS
    /// binary would trap rather than answer — so this refuses instead of
    /// inventing a quotient. No shipped module divides by a value that can
    /// reach zero, which is why the question has never had to be settled.
    DivideByZero {
        /// Where the division was.
        at: Address,
    },
    /// Tripped the step limit, which means the program is looping forever.
    StepLimit(u64),
    /// The host asked to pause and the caller cannot let it. See [`m32::Vm::call`].
    Suspended {
        /// Where the run had got to when the host asked to pause.
        at: Address,
    },
    /// A save word ran with no save directory set.
    ///
    /// Its own variant because it is neither a fault in the data nor an
    /// unfinished port: the original writes its slots beside `ENGINE.EXE`, and
    /// this refuses to write into the game directory at all, so a run that was
    /// never given somewhere else simply cannot save. The game carries on.
    NoSaveDir {
        /// The word that tried.
        word: &'static str,
        /// The slot it was working on.
        id: i32,
    },
    /// A save word could not read or write its file.
    ///
    /// Carries the `io::Error` as a source rather than folding it into a
    /// string, so that "the disk is full" and "the directory was deleted" stay
    /// distinguishable to anything that walks the chain.
    Io {
        /// The word that tried.
        word: &'static str,
        /// The file it was working on.
        path: std::path::PathBuf,
        /// What the operating system said.
        source: std::io::Error,
    },
    /// A savegame does not fit the game that is running.
    Savegame {
        /// What did not fit.
        what: String,
    },
    /// Behavior of the original that has not been read out of it yet.
    ///
    /// The project's alternative to a stub: rather than guess at what a branch
    /// does and draw something plausible, the word stops and says which address
    /// in `ENGINE.EXE` would have to be read to finish it. Distinct from
    /// [`Error::Unimplemented`], which is about a *primitive* that was never
    /// built, and from [`Error::Unsupported`], where the behavior is known and
    /// deliberately not reproduced.
    ///
    /// Also the one greppable list of what is left to do: every construction
    /// site in the engine names itself here, with the address to read.
    Unread {
        /// What was reached.
        what: String,
        /// Where to read it, as an address in `ENGINE.EXE`.
        at: &'static str,
    },
    /// A word was reached with an argument the host cannot honor. Distinct
    /// from `Unimplemented`: the word exists and works, this particular case
    /// does not, and saying so beats drawing something wrong.
    Unsupported(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unimplemented { ordinal, name, at } => {
                write!(
                    f,
                    "{at}: primitive {name} (ordinal {ordinal}) is not implemented"
                )
            }
            Self::UnknownOrdinal { ordinal, at } => {
                write!(f, "{at}: no kernel word has ordinal {ordinal}")
            }
            Self::StackUnderflow { word, at } => write!(f, "{at}: stack underflow in {word}"),
            Self::NoSuchModule { module, at, cell } => write!(
                f,
                "{at}: cell {cell:#010x} calls module {module}, which is not loaded"
            ),
            Self::UnboundWord { id, at } => {
                write!(f, "{at}: word id {id} is bound to no loaded module")
            }
            Self::OutOfMemory { module, need } => {
                write!(
                    f,
                    "module {module}: {need} bytes do not fit the 64 KiB space"
                )
            }
            Self::OutOfRange { addr, at } => match at {
                Some(at) => write!(f, "{at}: address {addr} is outside its module"),
                None => write!(f, "address {addr} is outside its module"),
            },
            Self::DivideByZero { at } => write!(f, "{at}: division by zero"),
            Self::StepLimit(n) => write!(f, "step limit of {n} reached; probably looping"),
            Self::NoSaveDir { word, id } => write!(
                f,
                "{word} {id}: no save directory is set, and writing beside the \
                 game data the way the original does is not allowed here"
            ),
            Self::Io { word, path, source } => {
                write!(f, "{word}: {}: {source}", path.display())
            }
            Self::Savegame { what } => write!(f, "{what}"),
            Self::Unread { what, at } => write!(f, "{what} — not yet read, at {at}"),
            Self::Unsupported(what) => write!(f, "{what}"),
            Self::Suspended { at } => {
                write!(
                    f,
                    "{at}: the host asked to pause, but this caller cannot resume it"
                )
            }
        }
    }
}

impl std::error::Error for Error {
    /// The wrapped cause, for the one variant that has one.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// The crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Run {
    /// The word returned.
    Done,
    /// The host asked to pause. Call [`m32::Vm::resume`] again to carry on.
    Yielded,
}

/// Where the words that are not part of the interpreter live.
///
/// The VM knows the stack, the threaded code and the control flow, and nothing
/// about graphics, sound or resources. Those words are the *engine*, and they
/// arrive through this trait — which keeps `motionvm-motion-forth` testable on its own
/// and stops the interpreter from growing a dependency on a framebuffer.
///
/// Returning `Ok(false)` means "not mine", and the VM turns that into an
/// [`Error::Unimplemented`] naming the word.
///
/// Generic over the machine, because the engine's words see the same stack
/// type on both and differ only in what the machine behind it is. There is
/// no default machine: an implementation names the generation it hosts.
pub trait Host<M> {
    /// The machine is handed over whole, not just its stack and memory.
    ///
    /// Most words only want `vm.data` and `vm.mem`. A few are native handlers
    /// that run the bytecode interpreter again from inside themselves —
    /// `DOORDER` does it twenty-two times, through word addresses kept in the
    /// game's own state block — and those need [`m32::Vm::call_nested`]. Handing
    /// over the machine is what makes that possible at all.
    ///
    /// Some engine words write into module memory — `GET` loads a resource to
    /// an address — so the stack alone is not enough. Memory lives in its own
    /// struct precisely so it can be handed over while the interpreter keeps
    /// the rest of itself.
    fn word(&mut self, name: &str, vm: &mut M) -> Result<bool>;

    /// A word the primitive just run wants called before anything else.
    ///
    /// Some native handlers end by running the bytecode interpreter again on a
    /// word of their own choosing — `MOUSEINFO` sets the pointer that way, with
    /// `FXATMOUSE`. Every such call in it is the last thing its branch does, so
    /// reproducing it as a tail call is exact: the arguments go on the shared
    /// data stack and control continues in the named word, returning where the
    /// primitive would have. The word is named the way the game's own state
    /// names it — a packed address on the 32-bit machine, a word id on the
    /// 16-bit one — and the machine resolves it with
    /// [`Machine::callback_target`].
    fn pending_call(&mut self) -> Option<(i32, Vec<i32>)> {
        None
    }

    /// Whether the host wants the interpreter to stop after the current word.
    ///
    /// Asked once per word. A host that says yes gets [`Run::Yielded`] and the
    /// run can be picked up again with [`m32::Vm::resume`]; this is how a transition
    /// holds the game still without the engine having to own the loop.
    fn wants_pause(&mut self) -> bool {
        false
    }
}

/// A host that implements nothing, for running pure computation, on either
/// machine.
pub struct NullHost;

impl Host<m32::Vm> for NullHost {
    fn word(&mut self, _n: &str, _vm: &mut m32::Vm) -> Result<bool> {
        Ok(false)
    }
}

impl Host<m16::Vm> for NullHost {
    fn word(&mut self, _n: &str, _vm: &mut m16::Vm) -> Result<bool> {
        Ok(false)
    }
}

/// The engine's view of a machine's memory.
///
/// A kernel word pops a value that names a place — the destination `GET`
/// copies a block to, a table `?XINSIDE` walks, a callback `SDWORD` stores —
/// and acts on it without knowing which machine it is running on. This is
/// the set of things it can do with such a value: a packed
/// `(module << 16) | offset` on the 32-bit machine, a flat 16-bit byte
/// address on the 16-bit one, either way the `i32` it came off the stack as.
/// A cell is the machine's cell — four bytes or two — and [`cell_size`]
/// says which, for the words that stride through records.
///
/// [`cell_size`]: AddressSpace::cell_size
pub trait AddressSpace {
    /// Reads the cell at `raw`, sign-extended to the stack's width.
    fn fetch_cell(&self, raw: i32) -> Result<i32>;
    /// Writes a cell at `raw`.
    fn store_cell(&mut self, raw: i32, value: i32) -> Result<()>;
    /// Reads the byte at `raw`.
    fn fetch_byte(&self, raw: i32) -> Result<u8>;
    /// Reads `n` bytes from `raw`.
    fn read_bytes(&self, raw: i32, n: usize) -> Result<Vec<u8>>;
    /// Writes raw bytes at `raw` — what `GET` does with a block.
    fn write_bytes(&mut self, raw: i32, bytes: &[u8]) -> Result<()>;
    /// `raw` moved by `bytes`, in the machine's own encoding — the next
    /// field of a record, the next record of a table.
    fn offset(&self, raw: i32, bytes: i32) -> i32;
    /// Bytes per cell: 4 or 2.
    fn cell_size(&self) -> i32;
    /// A callback argument screened the way the machine's kernel screens it:
    /// the value itself if it names a word that could be run, 0 otherwise.
    /// 0 and -1 are "no callback" and answer 0.
    fn callable(&self, raw: i32) -> i32;
    /// Whether a record could stand at this address: inside a loaded module
    /// on either machine. The interaction machine asks before it reads the
    /// `_ORDER` block, because one caller hands it a small number instead.
    fn is_live(&self, raw: i32) -> bool;
    /// A loaded module's memory, byte for byte — what `=>PUTAS` writes into a
    /// savegame. `None` for a module that is not loaded.
    fn module_image(&self, module: u32) -> Option<Vec<u8>>;
    /// Puts a module's memory back from such an image, which has to be the
    /// module's size to the byte — what `=>GETAS` does.
    fn restore_module(&mut self, module: u32, image: &[u8]) -> Result<()>;
}

/// What a driver needs of a machine to run a game on it — starting and
/// resuming words, parking an execution while another runs, and reading
/// the game's own variables — without being one machine's driver.
///
/// This is the boundary between the engine's `Game` and the two
/// interpreters. It is deliberately not an abstraction over the interpreter
/// itself: how a cell is decoded, what an address is, how a loop frame is
/// kept, are each machine's own. What the driver does is the same for both.
pub trait Machine: Sized + Send {
    /// An execution set aside by [`Machine::park`].
    type Context: Send;

    /// Points the machine at a word without running it.
    fn start(&mut self, at: Address) -> Result<()>;
    /// Runs until the word returns or the host asks to pause.
    fn resume(&mut self, host: &mut dyn Host<Self>) -> Result<Run>;
    /// Runs a word to completion with a return stack of its own, then puts
    /// the interrupted execution back.
    fn call_nested(&mut self, at: Address, host: &mut dyn Host<Self>) -> Result<()>;
    /// Lifts the current execution out of the machine, leaving it free.
    fn park(&mut self) -> Self::Context;
    /// Puts a parked execution back.
    fn unpark(&mut self, saved: Self::Context);
    /// The address of a word by module and name.
    fn word_address(&self, module: u32, name: &str) -> Option<Address>;
    /// The value of the variable whose word is at `word` — the cell after
    /// the `_PutAdr` that opens it.
    fn variable(&self, word: Address) -> Option<i32>;
    /// Writes the variable whose word is at `word`.
    fn set_variable(&mut self, word: Address, value: i32) -> Result<()>;
    /// The word a stored callback value names: a packed address on the
    /// 32-bit machine, a global word id on the 16-bit one.
    fn callback_target(&self, raw: i32) -> Option<Address>;
    /// The data stack.
    fn data(&mut self) -> &mut Vec<i32>;
    /// The module memory, as the engine's words see it.
    fn space(&self) -> &dyn AddressSpace;
    /// The same, to write.
    fn space_mut(&mut self) -> &mut dyn AddressSpace;
}

/// A loop frame, as `DO` … `LOOP` needs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Loop {
    pub(crate) limit: i32,
    /// Where this loop's counter sits on the return stack.
    ///
    /// `DO` puts its index there rather than on a stack of its own, which is
    /// what makes `I` simply "the top of the return stack". Measured against the
    /// original: `4711 >R I` gives 4711, and inside a loop a `>R` shadows the
    /// index — `2 0 DO 9 >R I …` gives 9, not the index. A separate loop stack
    /// gets the plain cases right and both of those wrong.
    pub(crate) slot: usize,
}
