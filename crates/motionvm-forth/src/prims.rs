//! The kernel primitives the VM can execute so far.
//!
//! Only the ones whose meaning is either plain Forth or was measured are here.
//! Anything else returns [`crate::Error::Unimplemented`] naming the word, so a gap
//! shows up as a clear stop rather than as wrong output — which matters,
//! because a silently mis-implemented primitive would corrupt game state in a
//! way that is very hard to trace back.

use motionvm_formats::Binding;

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
    /// [`crate::Error::UnknownOrdinal`].
    Absent,
    /// A branch word. Which one is still decided by the ordinal, because the
    /// arithmetic is shared and keyed by it — see [`crate::m32::branch`].
    Branch,
    /// Not the interpreter's. The engine is asked for it, by name.
    Host,
    /// `##` — the return. The 16-bit machine ends every definition with it;
    /// the 32-bit machine's definitions end with a zero cell, and `##`
    /// returns there too should a module name it.
    Return,

    // The five whose operand follows them inline. Decided by ordinal rather
    // than by name; see [`prim_of`].
    PutLit,
    PutAdr,
    PutConst,
    PutString,
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
/// branch — and it must go on doing exactly that. [`crate::m32::Vm::new`] reproduces the
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
        "##" => Prim::Return,

        _ => Prim::Host,
    }
}

/// The dispatch table: one [`Prim`] per ordinal, dense from zero.
///
/// Dense rather than a map because the whole point is to get a `Copy` answer
/// out of an index. Ordinals run to about two thousand for the 32-bit kernel's
/// second table and to 255 for the 16-bit kernel, so this is at most a couple
/// of kilobytes with the gaps marked [`Prim::Absent`] — and an ordinal in a gap
/// is exactly the "no kernel word has this" case the interpreters report.
///
/// **The match order is the interpreter's own.** The branch words and the
/// inline runtimes are settled by ordinal first — the binding's
/// [`motionvm_formats::Inline`] set — and only what is left over is looked up
/// by name. That is how the 32-bit `step_primitive` has always read, and a
/// kernel table whose name and ordinal disagreed has always run the ordinal.
pub(crate) fn dispatch_table(binding: &Binding) -> Vec<Prim> {
    let inline = &binding.inline;
    let top = binding.words.last().map_or(0, |&(o, _)| o) as usize;
    let mut table = vec![Prim::Absent; top + 1];
    for &(ordinal, ref name) in &binding.words {
        table[ordinal as usize] = if inline.is_branch(ordinal) {
            Prim::Branch
        } else if ordinal == inline.put_lit {
            Prim::PutLit
        } else if ordinal == inline.put_adr {
            Prim::PutAdr
        } else if ordinal == inline.put_const {
            Prim::PutConst
        } else if ordinal == inline.put_string {
            Prim::PutString
        } else if ordinal == inline.put_string_adr {
            Prim::PutStringAdr
        } else {
            prim_of(name)
        };
    }
    table
}

/// True is 1, in every word that produces a flag — not the -1 many Forths use.
/// Measured: `1 1 = R !` stored 1.
///
/// Worth stating because the handlers look otherwise at a glance: each opens
/// with `movl $0xffffffff,-4(%ebp)` before its stack-depth check. That -1 is
/// the "arguments are there" flag, not the result — the result goes into
/// -8(%ebp) further down, and it is 1. Misreading the first one for the second
/// is what the oracle test caught.
pub(crate) fn flag(b: bool) -> i32 {
    i32::from(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::m32::{IMPLEMENTED, branch};
    use motionvm_formats::m32::le::{INLINE, inline};

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
        let mut words: Vec<(u32, String)> =
            entries.iter().map(|&(o, n)| (o, n.to_string())).collect();
        words.sort_by_key(|&(o, _)| o);
        dispatch_table(&Binding {
            words,
            inline: INLINE,
        })
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
