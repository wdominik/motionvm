//! The 16-bit machine's primitives: what each kernel word the interpreter
//! owns does, on 16-bit cells.
//!
//! The arithmetic wraps at 16 bits and every result is pushed sign-extended;
//! the comparison, logic, loop and branch rules are the 32-bit engine's
//! measured ones taken as the hypothesis for this kernel — the handlers in
//! `ENVIRO.EXE` are unread — and each arm below says which it is.

use super::{CELL, Vm};
use crate::prims::{Prim, flag};
use crate::{Address, Error, Host, Loop, Result};

/// Names of the primitives implemented here, for reporting coverage.
pub const IMPLEMENTED: &[&str] = &[
    "##",
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
    "=",
    "<",
    ">",
    ">=",
    "<=",
    "0=",
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
    "RANDOM",
    "EXECUTE",
    "LEAVE",
    "_PutLit",
    "_PutAdr",
    "_PutConst",
    "_PutString",
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
    /// Executes one primitive. Answers `true` when the primitive is a
    /// complete word behavior and the caller should return — `##`,
    /// `_PutAdr`, `_PutConst`.
    pub(crate) fn step_primitive(
        &mut self,
        ordinal: u32,
        here: Address,
        host: &mut dyn Host<Vm>,
    ) -> Result<bool> {
        let prim = self
            .prims
            .get(ordinal as usize)
            .copied()
            .unwrap_or(Prim::Absent);
        if prim == Prim::Absent {
            return Err(Error::UnknownOrdinal { ordinal, at: here });
        }
        self.note(ordinal);
        match prim {
            Prim::Absent => Err(Error::UnknownOrdinal { ordinal, at: here }),
            // `##`: the return. Measured — every colon definition in the 65
            // modules ends with cell 0x8001.
            Prim::Return => Ok(true),
            Prim::Branch => self.step_branch(ordinal, here).map(|_| false),
            Prim::PutLit => {
                let (_, v) = self.operand();
                self.push(v as i32);
                Ok(false)
            }
            // A variable: the byte address of the cell after the opcode,
            // then return — measured by `ARR n + @` idioms throughout the
            // modules and by every variable body being `_PutAdr value`.
            Prim::PutAdr => {
                let at = self.ip;
                self.push(at as i32);
                Ok(true)
            }
            Prim::PutConst => {
                let (_, v) = self.operand();
                self.push(v as i32);
                Ok(true)
            }
            // A string built into the word: push its address and skip it.
            // The skip is the compiler's rule — the terminator included and
            // the length rounded up to a cell — whether the 16-bit handler
            // counts the same way is open.
            Prim::PutStringAdr => {
                let at = self.ip;
                self.push(at as i32);
                self.skip_string(at);
                Ok(false)
            }
            // `."`: the runtime that prints its string on a text console.
            // There is no console behind a 320×200 game; the two sites in
            // Die Enviro-Kids greifen ein sit on a debug path. Skipped, and
            // noted in the trace.
            Prim::PutString => {
                let at = self.ip;
                self.skip_string(at);
                if let Some(t) = self.trace.as_mut() {
                    t.push(format!("{here:>12}  _PutString (skipped)"));
                }
                Ok(false)
            }
            other => self.step_simple(other, ordinal, here, host).map(|_| false),
        }
    }

    /// Moves `ip` past a NUL-terminated string that starts at `at`.
    fn skip_string(&mut self, at: u16) {
        let mut len = 0u16;
        while self.mem.fetch_byte(at.wrapping_add(len)) != 0 {
            len = len.wrapping_add(1);
        }
        // `(len + 2) / 2` cells: the terminator and the padding to a cell.
        self.ip = at.wrapping_add(((len + 2) / 2) * CELL);
    }

    fn step_branch(&mut self, ordinal: u32, here: Address) -> Result<()> {
        let inline = self.binding.inline;
        let (operand_at, distance) = self.operand();
        let forward = inline.is_forward(ordinal);
        let target = if forward {
            operand_at.wrapping_add((distance as i32 * CELL as i32) as u16)
        } else {
            operand_at.wrapping_sub((distance as i32 * CELL as i32) as u16)
        };

        let taken = if ordinal == inline.check_if {
            // IF: jump when the flag is false.
            self.pop("IF")? == 0
        } else if ordinal == inline.check_eif {
            // =IF: compare-and-branch; taken when the two differ. The 32-bit
            // engine's measured rule, taken as the hypothesis here.
            let (b, a) = (self.pop("=IF")?, self.pop("=IF")?);
            a != b
        } else if ordinal == inline.check_else || ordinal == inline.repeat {
            true
        } else if ordinal == inline.until {
            self.pop("UNTIL")? == 0
        } else if ordinal == inline.loop_break {
            // WHILE leaves on **true** — measured on the 32-bit engine,
            // hypothesis here.
            self.pop("WHILE")? != 0
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
                    let index = (self.ret[l.slot] as i16) as i32 + step;
                    self.ret[l.slot] = index as u16;
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
                        at: here,
                    });
                }
            }
        } else if Some(ordinal) == inline.ch_else_dup {
            // `ELSEDUP`: its runtime is unread and no site in
            // Die Enviro-Kids greifen ein reaches it.
            return Err(Error::Unread {
                what: "_ChElseDup, the ELSEDUP runtime".into(),
                at: "ENVIRO.EXE, the handler the core table names at ordinal 43",
            });
        } else {
            return Err(Error::UnknownOrdinal { ordinal, at: here });
        };

        if taken {
            self.ip = target;
        }
        Ok(())
    }

    /// Everything that is neither a branch nor an inline runtime.
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

            // 16-bit wrap on every result: `push` truncates.
            Prim::Add => self.binary("+", |a, b| a.wrapping_add(b))?,
            Prim::Sub => self.binary("-", |a, b| a.wrapping_sub(b))?,
            Prim::Mul => self.binary("*", |a, b| a.wrapping_mul(b))?,
            Prim::Div | Prim::Mod => {
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

            // True is 1 — measured on the 32-bit engine, hypothesis here.
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
            // Logical, as in the 32-bit engine; `&` and `|` are the bitwise
            // pair.
            Prim::And => self.binary("AND", |a, b| flag(a != 0 && b != 0))?,
            Prim::Or => self.binary("OR", |a, b| flag(a != 0 || b != 0))?,
            Prim::BitAnd => self.binary("&", |a, b| a & b)?,
            Prim::BitOr => self.binary("|", |a, b| a | b)?,
            Prim::Shl => self.binary("<<", |a, b| a.wrapping_shl(b as u32))?,
            Prim::Shr => self.binary(">>", |a, b| ((a as u16) >> (b as u32 & 15)) as i32)?,
            Prim::BitNot => {
                let a = self.pop("~")?;
                self.push(!a);
            }

            // Byte addresses, aligned by the memory.
            Prim::Fetch => {
                let a = self.pop("@")?;
                let v = self.mem.fetch(a as u16) as i16;
                self.push(v as i32);
            }
            Prim::Store => {
                let (addr, v) = (self.pop("!")?, self.pop("!")?);
                self.mem.store(addr as u16, v as u16);
            }
            Prim::FetchByte => {
                let a = self.pop("C@")?;
                let v = self.mem.fetch_byte(a as u16);
                self.push(v as i32);
            }
            Prim::StoreByte => {
                let (addr, v) = (self.pop("C!")?, self.pop("C!")?);
                self.mem.store_byte(addr as u16, v as u8);
            }

            // The return stack holds 16-bit cells; a value comes back
            // sign-extended, the way it went in.
            Prim::ToR => {
                let a = self.pop(">R")?;
                self.ret.push(a as u16);
            }
            Prim::FromR => {
                let a = self.ret.pop().ok_or(Error::StackUnderflow {
                    word: "R>",
                    at: here,
                })?;
                self.push((a as i16) as i32);
            }
            // Whatever is on top of the return stack, loop counter or not —
            // `XYLSITEM.` uses `>R I … R>` as a copy of the top.
            Prim::LoopIndex => {
                let v = *self.ret.last().ok_or(Error::StackUnderflow {
                    word: "I",
                    at: here,
                })?;
                self.push((v as i16) as i32);
            }
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
                self.push((v as i16) as i32);
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
            // `EXECUTE ( id -- )`: a global word id, through the table —
            // `LOCINIT EXECUTE` with `LOCINIT` = `CONST 549` is the measured
            // use.
            Prim::Execute => {
                let id = self.pop("EXECUTE")? as u16;
                let Some(body) = self.mem.resolve(id) else {
                    return Err(Error::UnboundWord { id, at: here });
                };
                self.ret.push(self.ip);
                self.ip = body;
            }
            // DO: `limit index DO`, as in `706 701 DO I =>EXIST LOOP`.
            Prim::LoopStart => {
                let (index, limit) = (self.pop("DO")?, self.pop("DO")?);
                self.ret.push(index as u16);
                self.loops.push(Loop {
                    limit,
                    slot: self.ret.len() - 1,
                });
            }

            Prim::Host => {
                let Some(name) = self.ordinal_name(ordinal).map(str::to_owned) else {
                    return Err(Error::UnknownOrdinal { ordinal, at: here });
                };
                if !host.word(&name, self)? {
                    return Err(Error::Unimplemented {
                        ordinal,
                        name,
                        at: here,
                    });
                }
            }

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
