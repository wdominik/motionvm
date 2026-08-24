//! The kernel's word table, as both engine generations carry it, and the
//! binding that turns a bytecode ordinal into one of its words.
//!
//! Both generations register their kernel words in NUL-terminated arrays of
//! `{name, handler}` pairs inside the engine binary — 8-byte pairs of 32-bit
//! pointers in the relocated LE image of the 32-bit engine
//! ([`crate::m32::le`]), 8-byte pairs of far pointers in the 16-bit MZ image
//! ([`crate::m16::mz`]). A [`KernelWord`] is one such entry. How an ordinal
//! in threaded code maps onto an entry differs per generation — five per
//! table index from a measured base in the 32-bit engine, one per index from
//! 1 and from 105 in the 16-bit one — and a [`Binding`] is that answer made
//! concrete: the ordinal-to-name map, plus the ordinals of the words that
//! carry an operand in the cell after them, which is what a machine or a
//! disassembler has to know before it can walk a body.

/// One entry of the Forth kernel's word table.
#[derive(Debug, Clone)]
pub struct KernelWord {
    /// The word's name, as the bytecode and the compiler know it.
    pub name: String,
    /// Address of the C function implementing the word: a virtual address in
    /// the relocated image for the 32-bit engine, a file offset for the 16-bit
    /// one.
    pub handler: u32,
    /// Address of the table entry itself, in the same space as `handler`.
    pub entry: u32,
    /// Which of the kernel's tables this came from, in address order.
    pub table: usize,
    /// Index within that table.
    pub index: usize,
}

/// The ordinals of the kernel words whose operand follows them inline, for
/// one generation's kernel.
///
/// Every field is an ordinal in that kernel's numbering. `ch_else_dup` — the
/// `ELSEDUP` runtime — is `None` where its ordinal is not known: the 32-bit
/// kernel's was never measured, and no 32-bit module uses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inline {
    /// `_PutLit`: one cell, the literal; pushes it and continues.
    pub put_lit: u32,
    /// `_PutAdr`: one cell of storage; pushes its address and returns.
    pub put_adr: u32,
    /// `_PutConst`: one cell; pushes its value and returns.
    pub put_const: u32,
    /// `_PutString`: a NUL-terminated string padded to a cell boundary.
    pub put_string: u32,
    /// `_PutStringAdr`: the same payload; pushes its address and continues.
    pub put_string_adr: u32,
    /// `_CheckIf` — `IF`, a forward distance.
    pub check_if: u32,
    /// `_CheckEIf` — `=IF`, forward.
    pub check_eif: u32,
    /// `_ChElseDup` — `ELSEDUP`, forward; unknown where `None`.
    pub ch_else_dup: Option<u32>,
    /// `_CheckElse` — `ELSE`, forward.
    pub check_else: u32,
    /// `_Until`, backward.
    pub until: u32,
    /// `_Repeat`, backward.
    pub repeat: u32,
    /// `_LoopBreak` — `WHILE`, forward.
    pub loop_break: u32,
    /// `_LoopEnd` — `LOOP`, backward.
    pub loop_end: u32,
    /// `_AddLoop` — `+LOOP`, backward.
    pub add_loop: u32,
    /// `_ULoopEnd` — `/LOOP`, backward.
    pub u_loop_end: u32,
}

impl Inline {
    /// The names the fields stand for, in field order, with the field's value.
    fn named(&self) -> [(&'static str, Option<u32>); 15] {
        [
            ("_PutLit", Some(self.put_lit)),
            ("_PutAdr", Some(self.put_adr)),
            ("_PutConst", Some(self.put_const)),
            ("_PutString", Some(self.put_string)),
            ("_PutStringAdr", Some(self.put_string_adr)),
            ("_CheckIf", Some(self.check_if)),
            ("_CheckEIf", Some(self.check_eif)),
            ("_ChElseDup", self.ch_else_dup),
            ("_CheckElse", Some(self.check_else)),
            ("_Until", Some(self.until)),
            ("_Repeat", Some(self.repeat)),
            ("_LoopBreak", Some(self.loop_break)),
            ("_LoopEnd", Some(self.loop_end)),
            ("_AddLoop", Some(self.add_loop)),
            ("_ULoopEnd", Some(self.u_loop_end)),
        ]
    }

    /// Whether the word jumps forward from its operand cell.
    pub fn is_forward(&self, ordinal: u32) -> bool {
        [
            Some(self.check_if),
            Some(self.check_eif),
            self.ch_else_dup,
            Some(self.check_else),
            Some(self.loop_break),
        ]
        .contains(&Some(ordinal))
    }

    /// Whether the word jumps backward from its operand cell.
    pub fn is_backward(&self, ordinal: u32) -> bool {
        [
            self.until,
            self.repeat,
            self.loop_end,
            self.add_loop,
            self.u_loop_end,
        ]
        .contains(&ordinal)
    }

    /// Whether the word is any branch at all.
    pub fn is_branch(&self, ordinal: u32) -> bool {
        self.is_forward(ordinal) || self.is_backward(ordinal)
    }

    /// Whether the word is followed by one cell of operand — a literal, a
    /// data cell, or a branch distance.
    pub fn takes_cell(&self, ordinal: u32) -> bool {
        [self.put_lit, self.put_adr, self.put_const].contains(&ordinal) || self.is_branch(ordinal)
    }

    /// Whether the word is followed by a NUL-terminated string.
    pub fn takes_string(&self, ordinal: u32) -> bool {
        ordinal == self.put_string || ordinal == self.put_string_adr
    }

    /// The inline set read off a kernel's names: every field from the word
    /// of that name, or `None` if any name but `_ChElseDup` is missing.
    ///
    /// This is how the 16-bit kernel's set is bound. It is a derivation, not
    /// a measurement — the measurement is that walking all 65 of ENVIRO's
    /// modules with this set meets no unknown ordinal and ends every body at
    /// the next word's header. The 32-bit kernel's set is measured directly
    /// and kept as constants ([`crate::m32::le::INLINE`]).
    pub fn by_name(words: &[(u32, String)]) -> Option<Inline> {
        let find = |name: &str| words.iter().find(|(_, n)| n == name).map(|&(o, _)| o);
        Some(Inline {
            put_lit: find("_PutLit")?,
            put_adr: find("_PutAdr")?,
            put_const: find("_PutConst")?,
            put_string: find("_PutString")?,
            put_string_adr: find("_PutStringAdr")?,
            check_if: find("_CheckIf")?,
            check_eif: find("_CheckEIf")?,
            ch_else_dup: find("_ChElseDup"),
            check_else: find("_CheckElse")?,
            until: find("_Until")?,
            repeat: find("_Repeat")?,
            loop_break: find("_LoopBreak")?,
            loop_end: find("_LoopEnd")?,
            add_loop: find("_AddLoop")?,
            u_loop_end: find("_ULoopEnd")?,
        })
    }

    /// The field names and ordinals, for a listing or a test.
    pub fn entries(&self) -> Vec<(&'static str, u32)> {
        self.named()
            .into_iter()
            .filter_map(|(n, o)| o.map(|o| (n, o)))
            .collect()
    }
}

/// One generation's kernel, bound: what every ordinal is called, and which
/// ordinals carry an operand.
///
/// A machine is built from this and nothing else about the kernel; a
/// disassembler likewise. The words are sorted by ordinal.
#[derive(Debug, Clone)]
pub struct Binding {
    /// `(ordinal, name)`, ascending by ordinal, one entry per kernel word
    /// whose ordinal is known.
    pub words: Vec<(u32, String)>,
    /// The inline operand set.
    pub inline: Inline,
}

impl Binding {
    /// The name of the word at `ordinal`, if the kernel has one there.
    pub fn name(&self, ordinal: u32) -> Option<&str> {
        self.words
            .binary_search_by_key(&ordinal, |&(o, _)| o)
            .ok()
            .map(|i| self.words[i].1.as_str())
    }

    /// The ordinal of the word called `name`, if the kernel has one.
    pub fn ordinal(&self, name: &str) -> Option<u32> {
        self.words.iter().find(|(_, n)| n == name).map(|&(o, _)| o)
    }

    /// How many words are bound.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// Whether nothing is bound.
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}
