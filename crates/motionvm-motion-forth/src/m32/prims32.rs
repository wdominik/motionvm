//! The 32-bit machine's primitives: what each kernel word the interpreter
//! owns does, on 32-bit cells.

use super::{CELL, Vm, branch};
use crate::cell;
use crate::prims::{Prim, flag};
use crate::{Address, Error, Host, Result};

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

impl Vm {
    /// Executes one primitive.
    ///
    /// Returns `true` when the primitive is a complete word behavior and the
    /// caller should return, which is the case for `_PutAdr` and `_PutConst`.
    pub(crate) fn step_primitive(
        &mut self,
        ordinal: u32,
        here: Address,
        host: &mut dyn Host<Vm>,
    ) -> Result<bool> {
        let prim = self.prim(ordinal);
        if prim == Prim::Absent {
            return Err(Error::UnknownOrdinal { ordinal, at: here });
        }
        self.note(ordinal);

        match prim {
            // Answered above, and the compiler does not know it.
            Prim::Absent => Err(Error::UnknownOrdinal { ordinal, at: here }),
            // `##`: a return, the same as the zero cell that ends every
            // definition this compiler wrote.
            Prim::Return => Ok(true),
            // `."`: skip the string the way `_PutStringAdr` does, without
            // pushing — there is no console here for it to print on.
            Prim::PutString => {
                let at = self.ip;
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
            // Branches read an operand and share their arithmetic, which is
            // keyed by the ordinal rather than by which branch this is.
            Prim::Branch => self.step_branch(ordinal).map(|_| false),
            // Push the following cell and carry on.
            Prim::PutLit => {
                let (_, v) = self.operand()?;
                self.push(cell::signed(v));
                Ok(false)
            }
            // A variable: push the address of its data cell, then return. The
            // cell belongs to the word body, which is why module memory is
            // writable.
            Prim::PutAdr => {
                let at = self.ip;
                self.push(cell::signed(at.0));
                Ok(true)
            }
            // A constant: push the value in the following cell, then return.
            Prim::PutConst => {
                let (_, v) = self.operand()?;
                self.push(cell::signed(v));
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
                self.push(cell::signed(at.0));
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
        // Which ordinal is which word is this kernel's, not this build's, so
        // the arms compare against the bound set rather than matching literals.
        let inline = self.inline;
        // Branch distances count cells, while an address counts bytes, so the
        // arithmetic happens in cells and the result is converted back.
        let (operand_at, distance) = self.operand()?;
        let target = branch::target(&inline, ordinal, operand_at / CELL, distance) * CELL;
        let module = self.ip.module();

        // IF: jump when the flag is false.
        let taken = if ordinal == inline.check_if {
            self.pop("IF")? == 0
        // =IF: compare-and-branch in one word. Pops two values and runs the
        // arm when they are equal, so the branch is taken when they differ.
        // Measured: `3 3 =IF 111 R ! ENDIF` stored 111, `3 4 =IF` left R
        // untouched.
        } else if ordinal == inline.check_eif {
            let (b, a) = (self.pop("=IF")?, self.pop("=IF")?);
            a != b
        // ELSE: unconditional jump past the else-arm.
        } else if ordinal == inline.check_else {
            true
        // UNTIL: jump back while the flag is false.
        } else if ordinal == inline.until {
            self.pop("UNTIL")? == 0
        // WHILE: leaves the loop when the flag is **true** — the opposite of
        // standard Forth, and the reason the kernel calls it `_LoopBreak`
        // rather than `_While`. Measured: with the original engine,
        // `0 BEGIN DUP 3 >= WHILE 1 + REPEAT` counts up to 3, while the
        // standard-Forth reading `DUP 3 <` stops immediately at 0.
        } else if ordinal == inline.loop_break {
            self.pop("WHILE")? != 0
        // REPEAT: unconditional jump back to BEGIN.
        } else if ordinal == inline.repeat {
            true
        } else if ordinal == inline.loop_end
            || ordinal == inline.add_loop
            || ordinal == inline.u_loop_end
        {
            let step = if ordinal == inline.loop_end {
                1
            } else {
                self.pop("+LOOP")?
            };
            match self.loops.last().copied() {
                Some(l) => {
                    let index = cell::signed(self.ret[l.slot].0) + step;
                    self.ret[l.slot] = Address(cell::unsigned(index));
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
                        at: Some(self.ip),
                    });
                }
            }
        } else {
            unreachable!("Inline::is_branch admitted {ordinal}")
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
        host: &mut dyn Host<Vm>,
    ) -> Result<()> {
        match prim {
            Prim::Dup => {
                let v = *self.data.last().ok_or(Error::StackUnderflow {
                    word: "DUP",
                    at: Some(here),
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
                        at: Some(here),
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
            Prim::Shl => self.binary("<<", |a, b| a.wrapping_shl(cell::unsigned(b)))?,
            Prim::Shr => self.binary(">>", |a, b| a.wrapping_shr(cell::unsigned(b)))?,
            Prim::BitNot => {
                let a = self.pop("~")?;
                self.push(!a);
            }

            Prim::Fetch => {
                let a = self.pop("@")?;
                let v = self.fetch(Address(cell::unsigned(a)))?;
                self.push(cell::signed(v));
            }
            Prim::Store => {
                let (addr, v) = (self.pop("!")?, self.pop("!")?);
                self.store(Address(cell::unsigned(addr)), cell::unsigned(v))?;
            }
            // Addresses are byte-granular, so these are real byte accesses
            // rather than the low byte of a cell.
            Prim::FetchByte => {
                let a = self.pop("C@")?;
                let v = self.mem.fetch_byte(Address(cell::unsigned(a)))?;
                self.push(i32::from(v));
            }
            Prim::StoreByte => {
                let (addr, v) = (self.pop("C!")?, self.pop("C!")?);
                self.mem
                    .store_byte(Address(cell::unsigned(addr)), cell::low8(v))?;
            }

            Prim::ToR => {
                let a = self.pop(">R")?;
                self.ret.push(Address(cell::unsigned(a)));
            }
            Prim::FromR => {
                let a = self.ret.pop().ok_or(Error::StackUnderflow {
                    word: "R>",
                    at: Some(here),
                })?;
                self.push(cell::signed(a.0));
            }
            // Whatever is on top of the return stack, loop counter or not.
            Prim::LoopIndex => {
                let v = *self.ret.last().ok_or(Error::StackUnderflow {
                    word: "I",
                    at: Some(here),
                })?;
                self.push(cell::signed(v.0));
            }
            // The enclosing loop's counter, which is its own slot.
            // `I'` is the 16-bit kernel's, and no 32-bit module names it.
            Prim::NextLoopIndex => {
                return Err(Error::Unread {
                    what: "I', which this kernel does not have".into(),
                    binary: "ENGINE.EXE",
                    at: "no such word in its kernel table",
                });
            }
            Prim::OuterLoopIndex => {
                let n = self.loops.len();
                let l = self
                    .loops
                    .get(n.wrapping_sub(2))
                    .ok_or(Error::StackUnderflow {
                        word: "J",
                        at: Some(here),
                    })?;
                let v = self.ret[l.slot];
                self.push(cell::signed(v.0));
            }
            Prim::Leave => {
                if let Some(l) = self.loops.pop() {
                    self.ret.truncate(l.slot);
                }
            }

            // `RANDOM ( n -- r )`: `r` in `0..n`. The original divides
            // unsigned — a zero would fault it and a negative count leaves
            // its hash unreduced — and no shipped call pushes either, so
            // zero is what those get here.
            Prim::Random => {
                let n = self.pop("RANDOM")?;
                let r = self.next_rng();
                self.push(if n > 0 {
                    cell::signed(r % cell::unsigned(n))
                } else {
                    0
                });
            }
            Prim::Execute => {
                let a = self.pop("EXECUTE")?;
                self.ret.push(self.ip);
                self.ip = Address(cell::unsigned(a));
            }

            // DO: opens a loop frame. Argument order is Forth's usual
            // `limit start DO`, confirmed by running a counted loop in the
            // original and reading back what it accumulated.
            Prim::LoopStart => {
                let (index, limit) = (self.pop("DO")?, self.pop("DO")?);
                self.ret.push(Address(cell::unsigned(index)));
                self.loops.push(crate::Loop {
                    limit,
                    slot: self.ret.len() - 1,
                });
            }

            // Anything the interpreter does not own is the engine's, named
            // by the ordinal the cell held: [`Host::word`] dispatches on it
            // through a table of its own.
            //
            // The ordinal cannot be missing from the kernel table here —
            // `Prim::Host` is only ever written for one that came out of it —
            // but saying so with a `panic!` would put one on the interpreter's
            // path for no gain, so the impossible case answers the way an
            // ordinal that is genuinely not a word does.
            Prim::Host => {
                self.core.host_word();
                // The name is materialized on the error paths and nowhere
                // else. Building it for every host word would be a map lookup
                // and an allocation apiece, for a string the engine has no use
                // for: it resolves the ordinal once when the game opens.
                if !host.word(ordinal, self)? {
                    let Some(name) = self.ordinal_name(ordinal).map(str::to_owned) else {
                        return Err(Error::UnknownOrdinal { ordinal, at: here });
                    };
                    // Both the ordinal and the name: a report of `ordinal 0`
                    // for every unimplemented word would name no place.
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
            | Prim::Return
            | Prim::Branch
            | Prim::PutLit
            | Prim::PutAdr
            | Prim::PutConst
            | Prim::PutString
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
