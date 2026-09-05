//! The 16-bit machine's primitives: what each kernel word the interpreter
//! owns does, on 16-bit cells.
//!
//! The arithmetic wraps at 16 bits and every result is pushed sign-extended.
//!
//! **Its own engine, not a copy of the other one.** Most of these rules are
//! read out of the 16-bit binaries, and where they were read they differ
//! from the 32-bit machine's. The loops are the clearest case: the 32-bit
//! engine keeps a loop's limit in a frame
//! beside the return stack, and this one keeps limit *and* index on the return
//! stack itself (`DO` at `LL.EXE` `0af7:05d5` and `ENVIRO.EXE` `12c8:01b8`).
//! That is not a detail — Victor Loomes' `STOPLOOP` (module 605) leaves a loop
//! early by rewriting those very cells through `R>` and `>R`, and a machine
//! that kept the limit somewhere of its own turned that into a loop with no
//! end. `LOOP`, `+LOOP`, `_ULoopEnd`, `LEAVE`, `=IF` and `WHILE` are read at
//! the addresses their arms cite, `EXECUTE` takes a word id where the other
//! takes an address, and `I'` and `_ChElseDup` exist only here.
//!
//! The rest is read too, in `ENVIRO.EXE`: the comparisons answer 1 and 0
//! and compare signed (`=` at `12c8:03f2`, `<` at `12c8:0464`, `>=` at
//! `1977:0098`), `NOT`, `AND` and `OR` are logical (`12c8:04e1`,
//! `1977:0068`, `12c8:04f5`) beside the bitwise `&` (`12c8:0530`), `+`,
//! `-` and `*` are the plain 16-bit operations (`1977:000c`, `1977:001d`,
//! `12c8:000e`), and `/` and `MOD` divide **unsigned** (`div`, at
//! `12c8:0030` and `12c8:0072`) — which is where this machine parts from the
//! other one's signed division, and where `/` by zero answers 0 rather than
//! faulting. What both machines genuinely share is their own bookkeeping,
//! which lives in [`crate::core`].

use super::{CELL, Vm};
use crate::cell;
use crate::prims::{Prim, flag};
use crate::{Address, Error, Host, Result};

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
    "I'",
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
            .get(cell::index(ordinal))
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
                self.push(v);
                Ok(false)
            }
            // A variable: the byte address of the cell after the opcode,
            // then return — measured by `ARR n + @` idioms throughout the
            // modules and by every variable body being `_PutAdr value`.
            Prim::PutAdr => {
                let at = self.ip;
                self.push(i32::from(at));
                Ok(true)
            }
            Prim::PutConst => {
                let (_, v) = self.operand();
                self.push(v);
                Ok(true)
            }
            // A string built into the word: push its address and skip it.
            // The skip is the compiler's rule — the terminator included and
            // the length rounded up to a cell — whether the 16-bit handler
            // counts the same way is open.
            Prim::PutStringAdr => {
                let at = self.ip;
                self.push(i32::from(at));
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
                self.core
                    .push_trace(format!("{here:>12}  _PutString (skipped)"));
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
        // `(len + 2) / 2` cells: the terminator and the padding to a cell,
        // which is what the handler adds to the instruction pointer
        // (`12c8:134c`: `strlen`, plus two, shifted right).
        self.ip = at.wrapping_add(((len + 2) / 2) * CELL);
    }

    fn step_branch(&mut self, ordinal: u32, here: Address) -> Result<()> {
        let inline = self.binding.inline;
        let (operand_at, distance) = self.operand();
        let forward = inline.is_forward(ordinal);
        let target = if forward {
            operand_at.wrapping_add(cell::low16(distance * i32::from(CELL)))
        } else {
            operand_at.wrapping_sub(cell::low16(distance * i32::from(CELL)))
        };

        let taken = if ordinal == inline.check_if {
            // IF: jump when the flag is false.
            self.pop("IF")? == 0
        } else if ordinal == inline.check_eif {
            // =IF: compare-and-branch; taken when the two differ, both cells
            // popped — the handler (`LL.EXE` 0af7:06d5, `ENVIRO.EXE`
            // 12c8:02e0) pops twice, compares, and skips the operand only on
            // equality. The same rule as the 32-bit engine's.
            let (b, a) = (self.pop("=IF")?, self.pop("=IF")?);
            a != b
        } else if ordinal == inline.check_else || ordinal == inline.repeat {
            true
        } else if ordinal == inline.until {
            self.pop("UNTIL")? == 0
        } else if ordinal == inline.loop_break {
            // WHILE leaves on **true**: the handler (`LL.EXE` 0af7:0782,
            // `ENVIRO.EXE` 12c8:03c6) takes the forward jump on a non-zero
            // flag and stays in the loop body on zero — the same rule as the
            // 32-bit engine's.
            self.pop("WHILE")? != 0
        } else if ordinal == inline.loop_end || ordinal == inline.add_loop {
            // `LOOP` and `+LOOP` step the index — the top return-stack cell —
            // in place and read the limit from the cell under it, where
            // `_LoopStart` put them. The branch back is not taken once
            // limit <= index, signed and by the same `jle` whatever the
            // step's sign (`_LoopEnd` at `LL.EXE` 0af7:0604, `_AddLoop` at
            // 0af7:0665; the same code in `ENVIRO.EXE` at 12c8:01f3 and
            // 12c8:025c), and falling out pops both cells.
            let step = if ordinal == inline.loop_end {
                1
            } else {
                self.pop("+LOOP")?
            };
            let n = self.ret.len();
            if n < 2 {
                return Err(Error::StackUnderflow {
                    word: "LOOP",
                    at: Some(here),
                });
            }
            let index = cell::signed16(self.ret[n - 1]).wrapping_add(cell::short(step));
            self.ret[n - 1] = cell::unsigned16(index);
            let again = cell::signed16(self.ret[n - 2]) > index;
            if !again {
                self.ret.truncate(n - 2);
            }
            again
        } else if ordinal == inline.u_loop_end {
            // `_ULoopEnd` is `_LoopEnd` with the compare **unsigned** in the
            // three later builds (`ENVIRO.EXE` 12c8:0225, `HPPLAY.EXE`
            // 12a0:01cd, `BMZ.EXE` 12bb:021b: `incw`, then `jbe`); `LL.EXE`'s
            // older 0af7:0634 pops a return-stack cell where the others step
            // the index. No game reaches the word; the machine carries the
            // later builds' shape.
            let n = self.ret.len();
            if n < 2 {
                return Err(Error::StackUnderflow {
                    word: "LOOP",
                    at: Some(here),
                });
            }
            let index = self.ret[n - 1].wrapping_add(1);
            self.ret[n - 1] = index;
            let again = self.ret[n - 2] > index;
            if !again {
                self.ret.truncate(n - 2);
            }
            again
        } else if Some(ordinal) == inline.ch_else_dup {
            // `ELSEDUP`: its runtime is unread and no site in
            // Die Enviro-Kids greifen ein reaches it.
            return Err(Error::Unread {
                what: "_ChElseDup, the ELSEDUP runtime".into(),
                binary: "ENVIRO.EXE",
                at: "the handler the core table names at ordinal 43",
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

            // 16-bit wrap on every result: `push` truncates. `+` and `-` add
            // and subtract in place (`1977:000c`, `1977:001d`); `*` is a
            // `mul` whose low word is kept (`12c8:000e`).
            Prim::Add => self.binary("+", |a, b| a.wrapping_add(b))?,
            Prim::Sub => self.binary("-", |a, b| a.wrapping_sub(b))?,
            Prim::Mul => self.binary("*", |a, b| a.wrapping_mul(b))?,
            // Both divide the low words **unsigned** — `xor dx, dx` then
            // `div` — so a negative dividend is a large one. `/`
            // (`12c8:0030`) tests the divisor first and answers 0 for a zero;
            // `MOD` (`12c8:0072`) tests nothing, and a zero divisor is the
            // processor's own fault, which ends the program.
            Prim::Div => {
                let (b, a) = (self.pop("/")?, self.pop("/")?);
                let (a, b) = (cell::low16(a), cell::low16(b));
                self.push(i32::from(a.checked_div(b).unwrap_or(0)));
            }
            Prim::Mod => {
                let (b, a) = (self.pop("MOD")?, self.pop("MOD")?);
                let (a, b) = (cell::low16(a), cell::low16(b));
                if b == 0 {
                    return Err(Error::DivideByZero { at: here });
                }
                self.push(i32::from(a % b));
            }

            // True is 1 and false 0, and the compare is signed: `=` at
            // `12c8:03f2`, `!=` at `12c8:0418`, `>` at `12c8:043e` (`jle`),
            // `<` at `12c8:0464` (`jge`), `>=` and `<=` at `1977:0098` and
            // `1977:00ba`, `0=` at `12c8:048a`.
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
            // Logical: `AND` (`1977:0068`) counts the non-zero operands and
            // answers 1 for two, `OR` (`12c8:04f5`) answers 1 for either, and
            // `NOT` (`12c8:04e1`) is `neg; sbb; inc` — 1 for zero. `&` and
            // `|` are the bitwise pair (`12c8:0530`).
            Prim::And => self.binary("AND", |a, b| flag(a != 0 && b != 0))?,
            Prim::Or => self.binary("OR", |a, b| flag(a != 0 || b != 0))?,
            Prim::BitAnd => self.binary("&", |a, b| a & b)?,
            Prim::BitOr => self.binary("|", |a, b| a | b)?,
            Prim::Shl => self.binary("<<", |a, b| a.wrapping_shl(cell::unsigned(b)))?,
            Prim::Shr => self.binary(">>", |a, b| {
                i32::from(cell::low16(a) >> (cell::unsigned(b) & 15))
            })?,
            Prim::BitNot => {
                let a = self.pop("~")?;
                self.push(!a);
            }

            // Byte addresses, aligned by the memory.
            Prim::Fetch => {
                let a = self.pop("@")?;
                let v = cell::sign16(self.mem.fetch(cell::low16(a)));
                self.push(v);
            }
            Prim::Store => {
                let (addr, v) = (self.pop("!")?, self.pop("!")?);
                self.mem.store(cell::low16(addr), cell::low16(v));
            }
            Prim::FetchByte => {
                let a = self.pop("C@")?;
                let v = self.mem.fetch_byte(cell::low16(a));
                self.push(i32::from(v));
            }
            Prim::StoreByte => {
                let (addr, v) = (self.pop("C!")?, self.pop("C!")?);
                self.mem.store_byte(cell::low16(addr), cell::low8(v));
            }

            // The return stack holds 16-bit cells; a value comes back
            // sign-extended, the way it went in.
            Prim::ToR => {
                let a = self.pop(">R")?;
                self.ret.push(cell::low16(a));
            }
            Prim::FromR => {
                let a = self.ret.pop().ok_or(Error::StackUnderflow {
                    word: "R>",
                    at: Some(here),
                })?;
                self.push(cell::sign16(a));
            }
            // Whatever is on top of the return stack, loop counter or not —
            // `XYLSITEM.` uses `>R I … R>` as a copy of the top.
            Prim::LoopIndex => {
                let v = *self.ret.last().ok_or(Error::StackUnderflow {
                    word: "I",
                    at: Some(here),
                })?;
                self.push(cell::sign16(v));
            }
            // `I'` reads the cell behind the one `I` reads — the handler is
            // `I` with the fetch at `+2` instead of `+0` (`0af7:095e` against
            // `0af7:094a` in `LL.EXE`), and that stack grows downward, so the
            // cell at `+2` is the one pushed before it. Only Victor Loomes
            // calls it.
            Prim::NextLoopIndex => {
                let n = self.ret.len();
                let v = *n.checked_sub(2).and_then(|i| self.ret.get(i)).ok_or(
                    Error::StackUnderflow {
                        word: "I'",
                        at: Some(here),
                    },
                )?;
                self.push(cell::sign16(v));
            }
            // `J` binds in none of the five 16-bit builds — no kernel table
            // names it — so this is the layout's answer rather than a
            // handler's: the outer index sits under the inner loop's pair.
            Prim::OuterLoopIndex => {
                let n = self.ret.len();
                let v = *n.checked_sub(3).and_then(|i| self.ret.get(i)).ok_or(
                    Error::StackUnderflow {
                        word: "J",
                        at: Some(here),
                    },
                )?;
                self.push(cell::sign16(v));
            }
            // `LEAVE` copies the index over the limit — `LL.EXE` 0af7:069f,
            // `ENVIRO.EXE` 12c8:029a — so the next LOOP steps out. Nothing
            // is popped and the flow does not move.
            Prim::Leave => {
                let n = self.ret.len();
                if n >= 2 {
                    self.ret[n - 2] = self.ret[n - 1];
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
            // `EXECUTE ( id -- )`: a global word id, through the table —
            // `LOCINIT EXECUTE` with `LOCINIT` = `CONST 549` is the measured
            // use.
            Prim::Execute => {
                let id = cell::low16(self.pop("EXECUTE")?);
                let Some(body) = self.mem.resolve(id) else {
                    return Err(Error::UnboundWord { id, at: here });
                };
                self.ret.push(self.ip);
                self.ip = body;
            }
            // DO: `limit index DO`, as in `706 701 DO I =>EXIST LOOP`. The
            // handler (`LL.EXE` 0af7:05d5, the same code in `ENVIRO.EXE` at
            // 12c8:01b8) makes room for two return-stack cells and pops the
            // data stack into them: the index into the top one, the limit
            // under it. Both live on the return stack and nowhere else, and
            // bytecode edits them there: Victor Loomes' `STOPLOOP` (module
            // 605) rewrites the index through `R>` and `>R`, and a machine
            // that kept the limit somewhere of its own turned that early
            // exit into a loop that never ends.
            Prim::LoopStart => {
                let (index, limit) = (self.pop("DO")?, self.pop("DO")?);
                self.ret.push(cell::low16(limit));
                self.ret.push(cell::low16(index));
            }

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
