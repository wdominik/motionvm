//! Turns a 16-bit module's threaded code back into readable Forth.
//!
//! A body cell is one of three things:
//!
//! * a kernel word, `0x8000 | ordinal`, named through the [`Binding`];
//! * a call to the word with that global id, named once the module that
//!   defines it has been [learned](Disassembler::learn);
//! * an operand of the word before it — a literal, a data cell, a branch
//!   distance, or the bytes of a string.
//!
//! A variable or constant body — one that opens with `_PutAdr` or
//! `_PutConst` — is that word and its operand, and everything after it is
//! data: `ALLOT` cells, never fetched as code. The decoder stops there rather
//! than reading a table of coordinates as a run of calls.

use crate::cp437_to_string;
use crate::kernel::Binding;
use crate::m16::mz::{KERNEL_BIT, ORDINAL_MASK};
use crate::m16::scr::{Entry, ScrModule};

/// What a single cell turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cell {
    /// A kernel word, resolved through the binding.
    Kernel {
        /// The ordinal the cell carried.
        ordinal: u32,
        /// Its name.
        name: String,
    },
    /// A kernel ordinal the binding does not have.
    UnknownKernel {
        /// The ordinal that is in no table.
        ordinal: u32,
    },
    /// A call to a global word id. `name` is filled in when a module that
    /// defines the id has been learned.
    Call {
        /// The id.
        id: u16,
        /// Its name, if known.
        name: Option<String>,
    },
    /// A literal or a data cell consumed by the word before it.
    Data(i16),
    /// A branch distance, with the cell index it lands on.
    Branch {
        /// The distance as stored.
        distance: i16,
        /// The target, counted in cells from the start of the body.
        target: usize,
    },
    /// An inline string, consumed by `_PutString` or `_PutStringAdr`.
    Text(String),
}

impl Cell {
    /// One cell as it reads in a listing.
    pub fn render(&self) -> String {
        match self {
            Cell::Kernel { name, .. } => name.clone(),
            Cell::UnknownKernel { ordinal } => format!("<kernel:{ordinal}>"),
            Cell::Call { id, name } => match name {
                Some(n) => n.clone(),
                None => format!("<id:{id}>"),
            },
            Cell::Data(v) => v.to_string(),
            Cell::Branch { distance, target } => format!("{distance} -> {target}"),
            Cell::Text(t) => format!("{t:?}"),
        }
    }
}

/// Turns a word body's cells back into something readable.
///
/// Holds the binding for the kernel words and a symbol table for the calls;
/// [`Disassembler::learn`] fills the second from a parsed module. Ids are
/// global and reused across modules that are never resident together, so a
/// later `learn` of the same id replaces the earlier name: learn the modules
/// that would be resident together, and clone the disassembler before
/// learning a location's own.
#[derive(Clone)]
pub struct Disassembler<'a> {
    binding: &'a Binding,
    symbols: std::collections::HashMap<u16, String>,
}

impl<'a> Disassembler<'a> {
    /// A disassembler that knows the kernel's words but no module's.
    pub fn new(binding: &'a Binding) -> Self {
        Self {
            binding,
            symbols: Default::default(),
        }
    }

    /// Registers a module's words so that calls to its ids resolve to names.
    pub fn learn(&mut self, m: &ScrModule) {
        for e in &m.entries {
            self.symbols.insert(e.id, e.name.clone());
        }
    }

    /// Decodes one word's body.
    pub fn decode(&self, entry: &Entry) -> Vec<Cell> {
        let inline = &self.binding.inline;
        let cells = &entry.body;
        let mut out = Vec::with_capacity(cells.len());
        let mut p = 0usize;
        // A variable or a constant: the opening word, its one cell, and data.
        let data_from = if entry.is_variable() || entry.is_constant() {
            2
        } else {
            usize::MAX
        };
        while p < cells.len() {
            if p >= data_from {
                out.push(Cell::Data(cells[p] as i16));
                p += 1;
                continue;
            }
            let cell = cells[p];
            p += 1;
            if cell & KERNEL_BIT == 0 {
                out.push(Cell::Call {
                    id: cell,
                    name: self.symbols.get(&cell).cloned(),
                });
                continue;
            }
            let ordinal = (cell & ORDINAL_MASK) as u32;
            let Some(name) = self.binding.name(ordinal) else {
                out.push(Cell::UnknownKernel { ordinal });
                continue;
            };
            out.push(Cell::Kernel {
                ordinal,
                name: name.to_string(),
            });
            if inline.is_branch(ordinal) {
                if let Some(&d) = cells.get(p) {
                    let distance = d as i16;
                    let target = if inline.is_forward(ordinal) {
                        p.wrapping_add(distance as usize)
                    } else {
                        p.wrapping_sub(distance as usize)
                    };
                    out.push(Cell::Branch { distance, target });
                    p += 1;
                }
            } else if inline.takes_cell(ordinal) {
                if let Some(&d) = cells.get(p) {
                    out.push(Cell::Data(d as i16));
                    p += 1;
                }
            } else if inline.takes_string(ordinal) {
                let bytes: Vec<u8> = cells[p..].iter().flat_map(|c| c.to_le_bytes()).collect();
                let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                out.push(Cell::Text(cp437_to_string(&bytes[..end])));
                // The terminator is included and the length rounded up to a
                // cell — the compiler's rule.
                p += (end + 2) / 2;
            }
        }
        out
    }

    /// Renders a whole module.
    pub fn module(&self, m: &ScrModule) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        let _ = writeln!(
            s,
            "\\ module {}  ids {}-{}  {} words",
            m.module,
            m.first_id,
            m.last_id,
            m.entries.len()
        );
        for e in &m.entries {
            let _ = writeln!(s);
            let kind = if e.is_variable() {
                "VAR"
            } else if e.is_constant() {
                "CONST"
            } else {
                ":"
            };
            let truncated = if e.declared_len > e.name.chars().count() {
                "..."
            } else {
                ""
            };
            let _ = writeln!(
                s,
                "{kind} {}{truncated}   \\ id {}, {} cells",
                e.name,
                e.id,
                e.body.len()
            );
            let cells = self.decode(e);
            // One line per eight cells, prefixed with the index of the first
            // — the same count the branch targets use.
            for (i, chunk) in cells.chunks(8).enumerate() {
                let rendered: Vec<String> = chunk.iter().map(Cell::render).collect();
                let _ = writeln!(s, "{:>5}    {}", i * 8, rendered.join(" "));
            }
            let _ = writeln!(s, "      ;");
        }
        s
    }

    /// Counts how often each kernel word is used across the given modules.
    ///
    /// Data cells are not counted: a variable's `ALLOT` storage may hold any
    /// value, and walking it as code would inflate `@`, `!` and `+`.
    pub fn usage<'m>(
        &self,
        modules: impl IntoIterator<Item = &'m ScrModule>,
    ) -> Vec<(String, usize)> {
        let mut counts: std::collections::HashMap<String, usize> = Default::default();
        for m in modules {
            for e in &m.entries {
                for c in self.decode(e) {
                    if let Cell::Kernel { name, .. } = c {
                        *counts.entry(name).or_default() += 1;
                    }
                }
            }
        }
        let mut v: Vec<_> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v
    }
}
