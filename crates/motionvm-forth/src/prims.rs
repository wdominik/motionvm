//! The kernel primitives the VM can execute so far.
//!
//! Only the ones whose meaning is either plain Forth or was measured are here.
//! Anything else returns [`Error::Unimplemented`] naming the word, so a gap
//! shows up as a clear stop rather than as wrong output — which matters,
//! because a silently mis-implemented primitive would corrupt game state in a
//! way that is very hard to trace back.

use motionvm_formats::le::inline;

use crate::{Address, CELL, Error, Host, Result, Vm, branch};

/// Names of the primitives implemented here, for reporting coverage.
pub const IMPLEMENTED: &[&str] = &[
    "+",
    "-",
    "*",
    "/",
    "MOD",
    "SWAP",
    "DUP",
    "OVER",
    "ROT",
    "DROP",
    "@",
    "!",
    "C@",
    "C!",
    "=",
    "<",
    ">",
    ">=",
    "<=",
    "0=",
    "0>",
    "0<",
    "!=",
    "NOT",
    "AND",
    "OR",
    "<<",
    ">>",
    "~",
    "&",
    "|",
    ">R",
    "R>",
    "I",
    "J",
    "RANDOM",
    "EXECUTE",
    "LEAVE",
    "_PutLit",
    "_PutAdr",
    "_PutConst",
    "_PutStringAdr",
    "_CheckIf",
    "_CheckEIf",
    "_CheckElse",
    "_Until",
    "_Repeat",
    "_LoopBreak",
    "_LoopStart",
    "_LoopEnd",
    "_AddLoop",
    "_ULoopEnd",
];

/// Which primitive an ordinal stands for.
///
/// Answered once per kernel word when the machine is built, and indexed by
/// ordinal thereafter. The obvious alternative — turn the ordinal back into
/// its name on every executed cell and match that name against fifty-odd
/// literals — costs a map lookup and a heap allocation per cell, and the name
/// is only ever wanted for a trace, an error message or a handover to the
/// engine. None of those is the hot path.
///
/// `Copy` and one byte wide, which is what makes the table cheap: 356 kernel
/// words at ordinals up to about two thousand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Prim {
    /// No kernel word has this ordinal, so the cell is not a word at all.
    /// [`Error::UnknownOrdinal`].
    Absent,
    /// A branch word. Which one is still decided by the ordinal, because the
    /// arithmetic is shared and keyed by it — see [`crate::branch`].
    Branch,
    /// Not the interpreter's. The engine is asked for it, by name.
    Host,

    // The four whose operand follows them inline. Decided by ordinal rather
    // than by name; see [`prim_of`].
    PutLit,
    PutAdr,
    PutConst,
    PutStringAdr,

    // Stack.
    Dup,
    Drop,
    Swap,
    Over,
    Rot,

    // Arithmetic.
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison. Every one of these pushes a flag, and this engine's flag is
    // 1 rather than the -1 many Forths use — see [`flag`].
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    ZeroEq,
    ZeroGt,
    ZeroLt,
    Not,

    // Logical, and the bitwise pair they are constantly confused with.
    And,
    Or,
    BitAnd,
    BitOr,
    Shl,
    Shr,
    BitNot,

    // Memory. Byte-granular, because addresses are.
    Fetch,
    Store,
    FetchByte,
    StoreByte,

    // Return stack and loop counters, which share it.
    ToR,
    FromR,
    LoopIndex,
    OuterLoopIndex,
    Leave,
    LoopStart,

    Random,
    Execute,
}

/// The primitive a kernel word's *name* stands for, or [`Prim::Host`].
///
/// Only the names the interpreter recognizes by name. The branch words and the
/// four inline runtimes are deliberately not here: those are decided by
/// ordinal, and keeping that split is what makes the table faithful. The
/// interpreter has always checked the ordinal for them *before* it looked at
/// any name, so a kernel table whose name and ordinal disagreed would run the
/// branch — and it must go on doing exactly that. [`Vm::new`] reproduces the
/// order by filling from names first and overwriting numerically after.
pub(crate) fn prim_of(name: &str) -> Prim {
    match name {
        "DUP" => Prim::Dup,
        "DROP" => Prim::Drop,
        "SWAP" => Prim::Swap,
        "OVER" => Prim::Over,
        "ROT" => Prim::Rot,

        "+" => Prim::Add,
        "-" => Prim::Sub,
        "*" => Prim::Mul,
        "/" => Prim::Div,
        "MOD" => Prim::Mod,

        "=" => Prim::Eq,
        "!=" => Prim::Ne,
        "<" => Prim::Lt,
        ">" => Prim::Gt,
        "<=" => Prim::Le,
        ">=" => Prim::Ge,
        "0=" => Prim::ZeroEq,
        "0>" => Prim::ZeroGt,
        "0<" => Prim::ZeroLt,
        "NOT" => Prim::Not,

        "AND" => Prim::And,
        "OR" => Prim::Or,
        "&" => Prim::BitAnd,
        "|" => Prim::BitOr,
        "<<" => Prim::Shl,
        ">>" => Prim::Shr,
        "~" => Prim::BitNot,

        "@" => Prim::Fetch,
        "!" => Prim::Store,
        "C@" => Prim::FetchByte,
        "C!" => Prim::StoreByte,

        ">R" => Prim::ToR,
        "R>" => Prim::FromR,
        "I" => Prim::LoopIndex,
        "J" => Prim::OuterLoopIndex,
        "LEAVE" => Prim::Leave,
        "_LoopStart" => Prim::LoopStart,

        "RANDOM" => Prim::Random,
        "EXECUTE" => Prim::Execute,

        _ => Prim::Host,
    }
}

/// The dispatch table: one [`Prim`] per ordinal, dense from zero.
///
/// Dense rather than a map because the whole point is to get a `Copy` answer
/// out of an index. Ordinals run to about two thousand for the second kernel
/// table, so this is a couple of kilobytes with the gaps marked [`Prim::Absent`]
/// — and an ordinal in a gap is exactly the "no kernel word has this" case the
/// interpreter has always reported.
///
/// **The match order is the interpreter's own.** The branch words and the four
/// inline runtimes are settled by ordinal first, and only what is left over is
/// looked up by name. That is how `step_primitive` has always read, and a
/// kernel table whose name and ordinal disagreed has always run the ordinal.
pub(crate) fn dispatch_table(ordinals: &std::collections::BTreeMap<u32, String>) -> Vec<Prim> {
    let top = ordinals.keys().next_back().copied().unwrap_or(0) as usize;
    let mut table = vec![Prim::Absent; top + 1];
    for (&ordinal, name) in ordinals {
        table[ordinal as usize] = match ordinal {
            o if branch::is_branch(o) => Prim::Branch,
            inline::PUT_LIT => Prim::PutLit,
            inline::PUT_ADR => Prim::PutAdr,
            inline::PUT_CONST => Prim::PutConst,
            inline::PUT_STRING_ADR => Prim::PutStringAdr,
            _ => prim_of(name),
        };
    }
    table
}

impl Vm {
    /// Executes one primitive.
    ///
    /// Returns `true` when the primitive is a complete word behavior and the
    /// caller should return, which is the case for `_PutAdr` and `_PutConst`.
    pub(crate) fn step_primitive(
        &mut self,
        ordinal: u32,
        here: Address,
        host: &mut dyn Host,
    ) -> Result<bool> {
        let prim = self.prim(ordinal);
        if prim == Prim::Absent {
            return Err(Error::UnknownOrdinal { ordinal, at: here });
        }
        self.note(ordinal);

        match prim {
            // Answered above, and the compiler does not know it.
            Prim::Absent => Err(Error::UnknownOrdinal { ordinal, at: here }),
            // Branches read an operand and share their arithmetic, which is
            // keyed by the ordinal rather than by which branch this is.
            Prim::Branch => self.step_branch(ordinal).map(|_| false),
            // Push the following cell and carry on.
            Prim::PutLit => {
                let (_, v) = self.operand()?;
                self.push(v as i32);
                Ok(false)
            }
            // A variable: push the address of its data cell, then return. The
            // cell belongs to the word body, which is why module memory is
            // writable.
            Prim::PutAdr => {
                let at = self.ip;
                self.push(at.0 as i32);
                Ok(true)
            }
            // A constant: push the value in the following cell, then return.
            Prim::PutConst => {
                let (_, v) = self.operand()?;
                self.push(v as i32);
                Ok(true)
            }
            // A string built into the word: push its address and carry on.
            //
            // Like `_PutLit` and unlike the two above, it is an instruction
            // rather than a whole word behavior — the handler (0x665da) never
            // returns, and across the game's 147 uses not one opens a body.
            // The address it pushes is an ordinary packed one, which is why
            // `ADDMESSPIPE` and `CompareString` can take it straight.
            //
            // The skip is `(len + 3) >> 2` cells, counted from the string's
            // first byte and *not* counting the terminator — which is one cell
            // short of what the compiler laid down whenever the length divides
            // by four. In the whole game that is one string, `"GANRUFBA"`, at
            // four sites, and there the handler lands on the padding cell,
            // which is zero and therefore a return. Worked through, the stack
            // ends up exactly as it would have on the long way round, so the
            // original's arithmetic is safe as well as authentic. Our own
            // disassembler still needs the compiler's rule to walk a body —
            // both are right, for different questions.
            Prim::PutStringAdr => {
                let at = self.ip;
                self.push(at.0 as i32);
                let mut len = 0;
                while self
                    .mem
                    .fetch_byte(Address::new(at.module(), at.offset() + len))?
                    != 0
                {
                    len += 1;
                }
                self.ip = at.plus_cells((len + 3) / CELL);
                Ok(false)
            }
            other => self.step_simple(other, ordinal, here, host).map(|_| false),
        }
    }

    fn step_branch(&mut self, ordinal: u32) -> Result<()> {
        // Branch distances count cells, while an address counts bytes, so the
        // arithmetic happens in cells and the result is converted back.
        let (operand_at, distance) = self.operand()?;
        let target = branch::target(ordinal, operand_at / crate::CELL, distance) * crate::CELL;
        let module = self.ip.module();

        let taken = match ordinal {
            // IF: jump when the flag is false.
            inline::CHECK_IF => self.pop("IF")? == 0,
            // =IF: compare-and-branch in one word. Pops two values and runs
            // the arm when they are equal, so the branch is taken when they
            // differ. Measured: `3 3 =IF 111 R ! ENDIF` stored 111, `3 4 =IF`
            // left R untouched.
            inline::CHECK_EIF => {
                let (b, a) = (self.pop("=IF")?, self.pop("=IF")?);
                a != b
            }
            // ELSE: unconditional jump past the else-arm.
            inline::CHECK_ELSE => true,
            // UNTIL: jump back while the flag is false.
            inline::UNTIL => self.pop("UNTIL")? == 0,
            // WHILE: leaves the loop when the flag is **true** — the opposite
            // of standard Forth, and the reason the kernel calls it
            // `_LoopBreak` rather than `_While`. Measured: with the original
            // engine, `0 BEGIN DUP 3 >= WHILE 1 + REPEAT` counts up to 3, while
            // the standard-Forth reading `DUP 3 <` stops immediately at 0.
            inline::LOOP_BREAK => self.pop("WHILE")? != 0,
            // REPEAT: unconditional jump back to BEGIN.
            inline::REPEAT => true,
            inline::LOOP_END | inline::ADD_LOOP | inline::U_LOOP_END => {
                let step = match ordinal {
                    inline::LOOP_END => 1,
                    _ => self.pop("+LOOP")?,
                };
                match self.loops.last().copied() {
                    Some(l) => {
                        let index = self.ret[l.slot].0 as i32 + step;
                        self.ret[l.slot] = Address(index as u32);
                        let done = if step >= 0 {
                            index >= l.limit
                        } else {
                            index <= l.limit
                        };
                        if done {
                            self.loops.pop();
                            self.ret.truncate(l.slot);
                        }
                        !done
                    }
                    None => {
                        return Err(Error::StackUnderflow {
                            word: "LOOP",
                            at: self.ip,
                        });
                    }
                }
            }
            _ => unreachable!("branch::is_branch admitted {ordinal}"),
        };

        if taken {
            self.ip = Address::new(module, target);
        }
        Ok(())
    }

    /// Everything that is neither a branch nor an inline runtime.
    ///
    /// `ordinal` is carried alongside `prim` for the two cold paths that still
    /// want a name: handing the word to the engine, and saying which word was
    /// not implemented. Nothing on the way to an arm below looks it up.
    fn step_simple(
        &mut self,
        prim: Prim,
        ordinal: u32,
        here: Address,
        host: &mut dyn Host,
    ) -> Result<()> {
        match prim {
            Prim::Dup => {
                let v = *self.data.last().ok_or(Error::StackUnderflow {
                    word: "DUP",
                    at: here,
                })?;
                self.push(v);
            }
            Prim::Drop => {
                self.pop("DROP")?;
            }
            Prim::Swap => {
                let (b, a) = (self.pop("SWAP")?, self.pop("SWAP")?);
                self.push(b);
                self.push(a);
            }
            Prim::Over => {
                let n = self.data.len();
                let v = *self
                    .data
                    .get(n.wrapping_sub(2))
                    .ok_or(Error::StackUnderflow {
                        word: "OVER",
                        at: here,
                    })?;
                self.push(v);
            }
            Prim::Rot => {
                let (c, b, a) = (self.pop("ROT")?, self.pop("ROT")?, self.pop("ROT")?);
                self.push(b);
                self.push(c);
                self.push(a);
            }

            Prim::Add => self.binary("+", |a, b| a.wrapping_add(b))?,
            Prim::Sub => self.binary("-", |a, b| a.wrapping_sub(b))?,
            Prim::Mul => self.binary("*", |a, b| a.wrapping_mul(b))?,
            Prim::Div | Prim::Mod => {
                // Both pops report the word that is running. A fixed `/` in
                // the second one would report an underflow in `MOD` against a
                // word that was never on the stack.
                let word = if prim == Prim::Div { "/" } else { "MOD" };
                let (b, a) = (self.pop(word)?, self.pop(word)?);
                if b == 0 {
                    return Err(Error::DivideByZero { at: here });
                }
                self.push(if prim == Prim::Div {
                    a.wrapping_div(b)
                } else {
                    a.wrapping_rem(b)
                });
            }

            Prim::Eq => self.compare("=", |a, b| a == b)?,
            Prim::Ne => self.compare("!=", |a, b| a != b)?,
            Prim::Lt => self.compare("<", |a, b| a < b)?,
            Prim::Gt => self.compare(">", |a, b| a > b)?,
            Prim::Le => self.compare("<=", |a, b| a <= b)?,
            Prim::Ge => self.compare(">=", |a, b| a >= b)?,
            Prim::ZeroEq => {
                let a = self.pop("0=")?;
                self.push(flag(a == 0));
            }
            Prim::ZeroGt => {
                let a = self.pop("0>")?;
                self.push(flag(a > 0));
            }
            Prim::ZeroLt => {
                let a = self.pop("0<")?;
                self.push(flag(a < 0));
            }
            Prim::Not => {
                let a = self.pop("NOT")?;
                self.push(flag(a == 0));
            }

            // `AND` and `OR` are logical, not bitwise: their handlers compare
            // both operands against zero and push 1 or 0. The bitwise pair is
            // `&` and `|`, which really do `and`/`or` the words together.
            //
            // Treating `AND` as bitwise looks harmless — most conditions are
            // flag-against-flag — until one side is a value. `_NEXTLOC @
            // _INVMODE @ 1 <= AND` in `ICTRL` is exactly that: with location 2
            // pending, `2 & 1` came out 0 and the game never left the intro,
            // running on for as long as it was asked to without ever changing
            // scene.
            Prim::And => self.binary("AND", |a, b| flag(a != 0 && b != 0))?,
            Prim::Or => self.binary("OR", |a, b| flag(a != 0 || b != 0))?,
            Prim::BitAnd => self.binary("&", |a, b| a & b)?,
            Prim::BitOr => self.binary("|", |a, b| a | b)?,
            Prim::Shl => self.binary("<<", |a, b| a.wrapping_shl(b as u32))?,
            Prim::Shr => self.binary(">>", |a, b| a.wrapping_shr(b as u32))?,
            Prim::BitNot => {
                let a = self.pop("~")?;
                self.push(!a);
            }

            Prim::Fetch => {
                let a = self.pop("@")?;
                let v = self.fetch(Address(a as u32))?;
                self.push(v as i32);
            }
            Prim::Store => {
                let (addr, v) = (self.pop("!")?, self.pop("!")?);
                self.store(Address(addr as u32), v as u32)?;
            }
            // Addresses are byte-granular, so these are real byte accesses
            // rather than the low byte of a cell.
            Prim::FetchByte => {
                let a = self.pop("C@")?;
                let v = self.mem.fetch_byte(Address(a as u32))?;
                self.push(v as i32);
            }
            Prim::StoreByte => {
                let (addr, v) = (self.pop("C!")?, self.pop("C!")?);
                self.mem.store_byte(Address(addr as u32), v as u8)?;
            }

            Prim::ToR => {
                let a = self.pop(">R")?;
                self.ret.push(Address(a as u32));
            }
            Prim::FromR => {
                let a = self.ret.pop().ok_or(Error::StackUnderflow {
                    word: "R>",
                    at: here,
                })?;
                self.push(a.0 as i32);
            }
            // Whatever is on top of the return stack, loop counter or not.
            Prim::LoopIndex => {
                let v = *self.ret.last().ok_or(Error::StackUnderflow {
                    word: "I",
                    at: here,
                })?;
                self.push(v.0 as i32);
            }
            // The enclosing loop's counter, which is its own slot.
            Prim::OuterLoopIndex => {
                let n = self.loops.len();
                let l = self
                    .loops
                    .get(n.wrapping_sub(2))
                    .ok_or(Error::StackUnderflow {
                        word: "J",
                        at: here,
                    })?;
                let v = self.ret[l.slot];
                self.push(v.0 as i32);
            }
            Prim::Leave => {
                if let Some(l) = self.loops.pop() {
                    self.ret.truncate(l.slot);
                }
            }

            Prim::Random => {
                let n = self.pop("RANDOM")?;
                let r = self.next_rng();
                self.push(if n > 0 { (r % n as u32) as i32 } else { 0 });
            }
            Prim::Execute => {
                let a = self.pop("EXECUTE")?;
                self.ret.push(self.ip);
                self.ip = Address(a as u32);
            }

            // DO: opens a loop frame. Argument order is Forth's usual
            // `limit start DO`, confirmed by running a counted loop in the
            // original and reading back what it accumulated.
            Prim::LoopStart => {
                let (index, limit) = (self.pop("DO")?, self.pop("DO")?);
                self.ret.push(Address(index as u32));
                self.loops.push(crate::Loop {
                    limit,
                    slot: self.ret.len() - 1,
                });
            }

            // Anything the interpreter does not own is the engine's. This is
            // the one place a name is still materialized, and the engine wants
            // one: [`Host::word`] dispatches on it.
            //
            // The ordinal cannot be missing here — `Prim::Host` is only ever
            // written for an ordinal that came out of the kernel table — but
            // saying so with a `panic!` would put one on the interpreter's
            // path for no gain, so the impossible case answers the way an
            // ordinal that is genuinely not a word does.
            Prim::Host => {
                let Some(name) = self.ordinal_name(ordinal).map(str::to_owned) else {
                    return Err(Error::UnknownOrdinal { ordinal, at: here });
                };
                if !host.word(&name, self)? {
                    // Used to report `ordinal 0` for every unimplemented word,
                    // because this layer had only the name. It has both now.
                    return Err(Error::Unimplemented {
                        ordinal,
                        name,
                        at: here,
                    });
                }
            }

            // The branch and inline words never arrive here: `step_primitive`
            // answers them itself, and `Absent` before that.
            Prim::Absent
            | Prim::Branch
            | Prim::PutLit
            | Prim::PutAdr
            | Prim::PutConst
            | Prim::PutStringAdr => return Err(Error::UnknownOrdinal { ordinal, at: here }),
        }
        Ok(())
    }

    fn binary(&mut self, word: &'static str, f: impl Fn(i32, i32) -> i32) -> Result<()> {
        let (b, a) = (self.pop(word)?, self.pop(word)?);
        self.push(f(a, b));
        Ok(())
    }

    fn compare(&mut self, word: &'static str, f: impl Fn(i32, i32) -> bool) -> Result<()> {
        let (b, a) = (self.pop(word)?, self.pop(word)?);
        self.push(flag(f(a, b)));
        Ok(())
    }
}

/// True is 1, in every word that produces a flag — not the -1 many Forths use.
/// Measured: `1 1 = R !` stored 1.
///
/// Worth stating because the handlers look otherwise at a glance: each opens
/// with `movl $0xffffffff,-4(%ebp)` before its stack-depth check. That -1 is
/// the "arguments are there" flag, not the result — the result goes into
/// -8(%ebp) further down, and it is 1. Misreading the first one for the second
/// is what the oracle test caught.
fn flag(b: bool) -> i32 {
    i32::from(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The words the interpreter recognizes by *ordinal* rather than by name,
    /// with the ordinal that decides them.
    ///
    /// Not a second source of truth: the test below only uses it to ask
    /// [`branch::is_branch`] and the inline constants, which are the real ones.
    const BY_ORDINAL: &[(&str, u32)] = &[
        ("_PutLit", inline::PUT_LIT),
        ("_PutAdr", inline::PUT_ADR),
        ("_PutConst", inline::PUT_CONST),
        ("_PutStringAdr", inline::PUT_STRING_ADR),
        ("_CheckIf", inline::CHECK_IF),
        ("_CheckEIf", inline::CHECK_EIF),
        ("_CheckElse", inline::CHECK_ELSE),
        ("_Until", inline::UNTIL),
        ("_Repeat", inline::REPEAT),
        ("_LoopBreak", inline::LOOP_BREAK),
        ("_LoopEnd", inline::LOOP_END),
        ("_AddLoop", inline::ADD_LOOP),
        ("_ULoopEnd", inline::U_LOOP_END),
    ];

    /// Whether `step_primitive` decides this ordinal before it looks at a name.
    fn numeric(ordinal: u32) -> bool {
        branch::is_branch(ordinal)
            || matches!(
                ordinal,
                inline::PUT_LIT | inline::PUT_ADR | inline::PUT_CONST | inline::PUT_STRING_ADR
            )
    }

    /// Every name this crate claims to implement is resolved by exactly one of
    /// the two routes — and no name by both.
    ///
    /// This is the guard against the table and the list drifting apart. Without
    /// it, adding a primitive and forgetting to teach [`prim_of`] about it
    /// fails silently: the word goes to the engine, which does not have it, and
    /// the game stops somewhere unrelated with `Unimplemented`.
    ///
    /// Exclusive rather than inclusive on purpose. A name that resolved both
    /// ways would be ambiguous — the ordinal would win and the entry in
    /// `prim_of` would be dead code that reads as if it did something.
    #[test]
    fn every_implemented_name_resolves_exactly_once() {
        for name in IMPLEMENTED {
            let by_name = prim_of(name) != Prim::Host;
            let by_ordinal = BY_ORDINAL
                .iter()
                .find(|(n, _)| n == name)
                .is_some_and(|&(_, o)| numeric(o));
            assert!(
                by_name ^ by_ordinal,
                "{name}: by name {by_name}, by ordinal {by_ordinal} — it must be exactly one"
            );
        }
    }

    /// `_LoopStart` is the odd one out and stays that way.
    ///
    /// It reads as a loop word and sits among the branch ordinals, but it takes
    /// no operand and does not jump — it opens the loop frame. Guessing by
    /// family would have put it on the numeric route, where it would have run
    /// the branch arithmetic over whatever cell followed it.
    #[test]
    fn loop_start_is_resolved_by_name() {
        assert_eq!(prim_of("_LoopStart"), Prim::LoopStart);
        assert!(!BY_ORDINAL.iter().any(|(n, _)| *n == "_LoopStart"));
    }

    /// A word the engine owns is not the interpreter's.
    #[test]
    fn engine_words_go_to_the_host() {
        for name in ["ACTSCR", "ACTDESC", "SDCEN", "FADEIN", "DOORDER"] {
            assert_eq!(prim_of(name), Prim::Host, "{name}");
        }
    }

    fn table(entries: &[(u32, &str)]) -> Vec<Prim> {
        dispatch_table(
            &entries
                .iter()
                .map(|&(o, n)| (o, n.to_string()))
                .collect::<std::collections::BTreeMap<_, _>>(),
        )
    }

    /// The table answers by name for the ordinary words and marks the gaps.
    #[test]
    fn the_table_resolves_names_and_marks_gaps() {
        let t = table(&[(104, "DUP"), (109, "+"), (114, "ACTSCR")]);
        assert_eq!(t[104], Prim::Dup);
        assert_eq!(t[109], Prim::Add);
        assert_eq!(t[114], Prim::Host);
        // A gap between two entries is not a word.
        assert_eq!(t[105], Prim::Absent);
    }

    /// The ordinal decides the branch and inline words, not the name.
    ///
    /// This is the property that keeps the table faithful to the interpreter it
    /// replaces: `step_primitive` tested `branch::is_branch(ordinal)` and the
    /// four inline constants *before* it matched any name, so a kernel table
    /// that disagreed with itself ran the ordinal. Building the table from
    /// names alone would have quietly changed that.
    #[test]
    fn the_ordinal_wins_over_the_name() {
        let t = table(&[
            (inline::CHECK_IF, "DUP"),
            (inline::PUT_LIT, "DROP"),
            (inline::PUT_ADR, "_PutAdr"),
            (inline::PUT_CONST, "_PutConst"),
            (inline::PUT_STRING_ADR, "_PutStringAdr"),
            (inline::LOOP_END, "_LoopEnd"),
        ]);
        assert_eq!(t[inline::CHECK_IF as usize], Prim::Branch);
        assert_eq!(t[inline::PUT_LIT as usize], Prim::PutLit);
        assert_eq!(t[inline::PUT_ADR as usize], Prim::PutAdr);
        assert_eq!(t[inline::PUT_CONST as usize], Prim::PutConst);
        assert_eq!(t[inline::PUT_STRING_ADR as usize], Prim::PutStringAdr);
        assert_eq!(t[inline::LOOP_END as usize], Prim::Branch);
    }

    /// An empty kernel table is one `Absent` entry, not a panic.
    #[test]
    fn an_empty_kernel_still_builds() {
        assert_eq!(table(&[]), vec![Prim::Absent]);
    }
}
