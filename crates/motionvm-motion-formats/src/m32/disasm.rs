//! Turns a module's threaded code back into readable Forth.
//!
//! A body cell is one of three things, and telling them apart is what this does:
//!
//! * a kernel word, tagged `0x4000` in the high half, named through the
//!   [`Binding`] its game's binary yields (see [`crate::m32::le`]);
//! * a reference to a word in a module, as `(module << 16) | offset`;
//! * inline data, when the previous cell was one of the words that consume the
//!   cell after them — `_PutLit`, `_PutAdr`, `_PutConst`.
//!
//! Without that last rule a literal would be mistaken for a word reference,
//! since a small integer looks exactly like an offset into module 0.

use crate::cursor::Cursor;
use crate::kernel::Binding;
use crate::m32::le::TAG_KERNEL;
use crate::m32::scr::{Entry, ScrModule};
use crate::{cp437_to_string, nul_terminated};

/// What a single cell turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cell {
    /// A kernel primitive, resolved through the ordinal table.
    Kernel {
        /// The ordinal the cell carried.
        ordinal: u32,
        /// Its name from the kernel table.
        name: String,
    },
    /// A kernel ordinal the binding has no word at — a cell that is not a
    /// word at all, which is what a decoder that has lost its footing sees.
    UnknownKernel {
        /// The ordinal that is in no table.
        ordinal: u32,
    },
    /// A call into a module's dictionary. `name` is filled in when the target
    /// module has been registered with [`Disassembler::learn`].
    Call {
        /// The module the target lives in.
        module: u32,
        /// Byte offset of the target's body within it.
        offset: u32,
        /// The word's name, once the module has been learned.
        name: Option<String>,
    },
    /// Inline data consumed by the preceding word.
    Data(u32),
    /// An inline string, consumed by `_PutString` or `_PutStringAdr`.
    Text(String),
}

impl Cell {
    /// One cell as it reads in a listing.
    ///
    /// Names where a name is known, `EXIT` for the return that ends every
    /// definition, and small numbers in decimal but large ones in hex — which
    /// is what tells a count from an address at a glance.
    pub fn render(&self) -> String {
        match self {
            Cell::Kernel { name, .. } => name.clone(),
            Cell::UnknownKernel { ordinal } => format!("<kernel:{ordinal}>"),
            Cell::Call {
                module,
                offset,
                name,
            } => match name {
                Some(n) => n.to_string(),
                // Module 0 is the kernel's base module; offset 0 there is the
                // return that ends every definition.
                None if *module == 0 && *offset == 0 => "EXIT".into(),
                None => format!("<{module}:{offset:#x}>"),
            },
            Cell::Text(t) => format!("{t:?}"),
            Cell::Data(v) => {
                // Small values read better as decimal, big ones as hex.
                if *v < 0x10000 {
                    format!("{v}")
                } else {
                    format!("{v:#x}")
                }
            }
        }
    }
}

/// Turns a word body's cells back into something readable.
///
/// Holds the kernel's binding for the primitives and a symbol table for
/// calls; [`Disassembler::learn`] fills the second from a parsed module.
#[derive(Debug)]
pub struct Disassembler<'a> {
    binding: &'a Binding,
    /// Word names by `(module, body offset)`, so calls can be shown by name.
    /// A call targets a word's body, not its header, which is why the key is
    /// the body offset.
    symbols: std::collections::HashMap<(u32, u32), String>,
}

impl<'a> Disassembler<'a> {
    /// A disassembler that knows the kernel's words but no module's.
    pub fn new(binding: &'a Binding) -> Self {
        Self {
            binding,
            symbols: Default::default(),
        }
    }

    /// Registers a module's words so that calls into it resolve to names.
    pub fn learn(&mut self, m: &ScrModule) {
        for e in &m.entries {
            self.symbols
                .insert((m.module, e.call_offset()), e.name.clone());
        }
    }

    fn call_name(&self, module: u32, offset: u32) -> Option<&str> {
        self.symbols.get(&(module, offset)).map(String::as_str)
    }

    fn kernel_name(&self, ordinal: u32) -> Option<&str> {
        self.binding.name(ordinal)
    }

    /// Decodes one word's body.
    ///
    /// Operands have to be consumed as they come: the cell after `_PutLit` is a
    /// number, and `_PutString` is followed by raw text rather than cells.
    pub fn decode(&self, entry: &Entry) -> Vec<Cell> {
        let raw: Vec<u8> = entry.body.iter().flat_map(|c| c.to_le_bytes()).collect();
        let mut out = Vec::new();
        let mut c = Cursor::new(&raw, 0);
        while let Ok(cell) = c.u32() {
            if cell >> 16 != TAG_KERNEL >> 16 {
                let (module, offset) = (cell >> 16, cell & 0xffff);
                out.push(Cell::Call {
                    module,
                    offset,
                    name: self.call_name(module, offset).map(str::to_string),
                });
                continue;
            }
            let ordinal = cell & 0xffff;
            let Some(name) = self.kernel_name(ordinal) else {
                out.push(Cell::UnknownKernel { ordinal });
                continue;
            };
            out.push(Cell::Kernel {
                ordinal,
                name: name.to_string(),
            });
            if self.binding.inline.takes_cell(ordinal) {
                if let Ok(operand) = c.u32() {
                    out.push(Cell::Data(operand));
                }
            } else if self.binding.inline.takes_string(ordinal) {
                let text = nul_terminated(c.remaining());
                out.push(Cell::Text(cp437_to_string(text)));
                // The terminator, then padding out to the next cell.
                let after = c.position().saturating_add(text.len()).saturating_add(1);
                c.seek(after.next_multiple_of(4));
            }
        }
        out
    }

    /// Renders a whole module.
    pub fn module(&self, m: &ScrModule) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        let (emod, eoff) = m.entry_point();
        let _ = writeln!(s, "\\ module {} - {} words", m.module, m.entries.len());
        if m.entry != 0 {
            let _ = writeln!(s, "\\ entry point: module {emod}, offset {eoff:#x}");
        }
        let _ = writeln!(
            s,
            "\\ module memory: {} bytes, then {} more",
            m.mem_len(),
            m.second_area_len
        );
        if m.tail_len != 0 {
            let _ = writeln!(s, "\\ {} bytes past the second region", m.tail_len);
        }
        for e in &m.entries {
            let _ = writeln!(s);
            let truncated = if e.declared_len > e.name.chars().count() {
                "..."
            } else {
                ""
            };
            let _ = writeln!(
                s,
                "{:#07x}  : {}{}   \\ {} cells",
                e.offset,
                e.name,
                truncated,
                e.body.len()
            );
            let cells = self.decode(e);
            // One line per eight cells keeps long threads readable while still
            // letting an offset be located quickly.
            for (at, chunk) in (e.body_offset()..).step_by(8 * 4).zip(cells.chunks(8)) {
                let rendered: Vec<String> = chunk.iter().map(Cell::render).collect();
                let _ = writeln!(s, "{at:#07x}    {}", rendered.join(" "));
            }
            let _ = writeln!(s, "         ;");
        }
        s
    }

    /// Counts how often each kernel word is used across the given modules.
    ///
    /// This is the number that decides how much of the kernel actually has to be
    /// reimplemented: the table has 356 entries, but a game only reaches for a
    /// fraction of them.
    pub fn usage(&self, modules: &[ScrModule]) -> Vec<(String, usize)> {
        let mut counts: std::collections::HashMap<String, usize> = Default::default();
        for m in modules {
            for e in &m.entries {
                for c in self.decode(e) {
                    if let Cell::Kernel { name, .. } = c {
                        let count: &mut usize = counts.entry(name).or_default();
                        *count = count.saturating_add(1);
                    }
                }
            }
        }
        let mut v: Vec<_> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v
    }
}
