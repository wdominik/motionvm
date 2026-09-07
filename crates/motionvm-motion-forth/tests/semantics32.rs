//! What the interpreter does, held against what the original was measured to do.
//!
//! Nothing here is new knowledge: every assertion is a fact already recorded
//! in a doc comment in `src/lib.rs` or `src/prims.rs`, usually with the address
//! in `ENGINE.EXE` it was read from or the experiment that settled it. Written
//! down as tests, prose that can quietly go stale becomes something that
//! fails. [`NullHost`] exists to make that possible without an engine.
//!
//! **No game data.** Everything runs on bytecode built in memory, so this file
//! is the one part of the suite that works on any machine.
//!
//! There is deliberately no `ds2_modules.rs` beside the five 16-bit
//! `*_modules.rs` suites: the 32-bit machine's runs against the real game's
//! bytecode go through the engine's `ds2_*` suites, which drive the same
//! modules with the kernel words answered rather than stubbed.
//!
//! Four things are deliberately *not* asserted as original behavior, because
//! they have never been measured: what `/LOOP` does at its limit, where `LEAVE`
//! continues, what the original makes of a negative `/` or `MOD`, and what it
//! does on division by zero. Where a test touches those it says it is pinning
//! *our* choice, not the original's.

use motionvm_motion_formats::Binding;
use motionvm_motion_formats::m32::le::{INLINE, TAG_KERNEL, inline};
use motionvm_motion_formats::m32::scr::ScrModule;
use motionvm_motion_forth::cell;
use motionvm_motion_forth::m32::{CELL, Vm, branch};
use motionvm_motion_forth::{Address, Error, NullHost};

// ---------------------------------------------------------------- the machine

/// The interpreter's own words, at the ordinals it dispatches them on.
///
/// `step_primitive` looks the name up *before* it looks at the number — that
/// lookup is the only thing rejecting an ordinal no kernel word has — so even
/// the words it then dispatches numerically have to be in the table. Their
/// ordinals are not free: `step_branch` and the inline arms match on these
/// exact values.
const FIXED: &[(&str, u32)] = &[
    ("_PutLit", inline::PUT_LIT),
    ("_PutAdr", inline::PUT_ADR),
    ("_PutConst", inline::PUT_CONST),
    ("_PutString", inline::PUT_STRING),
    ("_PutStringAdr", inline::PUT_STRING_ADR),
    ("_CheckIf", inline::CHECK_IF),
    ("_CheckEIf", inline::CHECK_EIF),
    ("_CheckElse", inline::CHECK_ELSE),
    ("_AddLoop", inline::ADD_LOOP),
    ("_ULoopEnd", inline::U_LOOP_END),
    ("_Until", inline::UNTIL),
    ("_LoopBreak", inline::LOOP_BREAK),
    ("_Repeat", inline::REPEAT),
    ("_LoopEnd", inline::LOOP_END),
];

/// The kernel words these tests name by hand.
///
/// Which ordinal each one gets does not matter — nothing outside this file
/// knows or cares — so they are handed out in order, stepping over the indices
/// [`FIXED`] has claimed. What *is* real is the formula: a table-0 word's
/// ordinal is `5 * index + 104`, measured from the shipped kernel — see
/// [`motionvm_motion_formats::m32::le`], which recovers the real table.
const NAMES: &[&str] = &[
    "DUP",
    "DROP",
    "SWAP",
    "OVER",
    "ROT",
    "+",
    "-",
    "*",
    "/",
    "MOD",
    "=",
    "!=",
    "<",
    ">",
    "<=",
    ">=",
    "0=",
    "0>",
    "0<",
    "NOT",
    "AND",
    "OR",
    "&",
    "|",
    "<<",
    ">>",
    "~",
    "@",
    "!",
    "C@",
    "C!",
    ">R",
    "R>",
    "I",
    "J",
    "RANDOM",
    "EXECUTE",
    "_LoopStart",
    "NOSUCHWORD",
];

/// The index a table-0 ordinal sits at.
fn index_of(ordinal: u32) -> usize {
    cell::index((ordinal - 104) / 5)
}

/// The kernel, bound: the interpreter's own words at their real ordinals,
/// everything else at whatever table-0 index is still free, and the inline
/// set the real kernel measures as.
fn kernel() -> Binding {
    let taken: Vec<usize> = FIXED.iter().map(|(_, o)| index_of(*o)).collect();
    let mut words: Vec<(u32, String)> = FIXED.iter().map(|(n, o)| (*o, (*n).to_string())).collect();
    let mut index = 0usize;
    for name in NAMES {
        while taken.contains(&index) {
            index += 1;
        }
        words.push((104 + 5 * cell::narrow(index), (*name).to_string()));
        index += 1;
    }
    words.sort_by_key(|&(o, _)| o);
    Binding {
        words,
        inline: INLINE,
    }
}

/// The ordinal of a named word in the synthetic table.
fn ordinal(name: &str) -> u32 {
    kernel()
        .ordinal(name)
        .unwrap_or_else(|| panic!("{name} is in neither FIXED nor NAMES"))
}

/// A cell that executes a kernel word.
fn prim(name: &str) -> u32 {
    TAG_KERNEL | ordinal(name)
}

/// A cell that executes an interpreter word by its own ordinal.
fn op(ordinal: u32) -> u32 {
    TAG_KERNEL | ordinal
}

/// A module holding `cells` as its memory, from address 0.
///
/// `Module::load` takes everything from file offset 0x30 as cells, so the item
/// is that much padding followed by the bytecode. No word headers: these tests
/// call by address, not by name.
fn module(cells: &[u32]) -> (Vec<u8>, ScrModule) {
    let mut item = vec![0u8; 0x30];
    for c in cells {
        item.extend_from_slice(&c.to_le_bytes());
    }
    let parsed = ScrModule {
        module: 1,
        entry: 0,
        entries: Vec::new(),
        dp_cells: cells.len(),
        second_area_len: 0,
        declared_mem_len: 0,
        tail_len: 0,
    };
    (item, parsed)
}

/// A machine with `cells` loaded as module 1 and `data` on the stack.
fn machine(cells: &[u32], data: &[i32]) -> Vm {
    let mut vm = Vm::new(&kernel());
    let (item, parsed) = module(cells);
    vm.load(&item, &parsed);
    vm.data.extend_from_slice(data);
    vm
}

/// Runs `cells` from address 0 and answers the data stack it left.
///
/// A body ends at its first zero cell, which is what the interpreter treats as
/// a return, so every program here finishes with one.
fn run(cells: &[u32], data: &[i32]) -> Vec<i32> {
    let mut vm = machine(cells, data);
    vm.call(Address::new(1, 0), &mut NullHost)
        .unwrap_or_else(|e| panic!("{e}"));
    vm.data
}

/// Runs `cells` and answers the error it stopped on.
fn fails(cells: &[u32], data: &[i32]) -> Error {
    let mut vm = machine(cells, data);
    match vm.call(Address::new(1, 0), &mut NullHost) {
        Err(e) => e,
        Ok(()) => panic!("expected a failure, got the stack {:?}", vm.data),
    }
}

// ------------------------------------------------------------ branch arithmetic

/// Where each branch word lands, from the table in `src/lib.rs`.
///
/// Needs no machine at all: `branch::target` is arithmetic on the operand's own
/// position, over the inline set the shipped kernel binds to. The eight vectors are the ones recorded there, taken from putting
/// each construct through the original's compiler and reading the cells back.
/// Distances are in cells, and the direction is fixed per word — which is the
/// whole reason this is a table rather than a sign in the operand.
#[test]
fn branch_targets_match_the_compiler() {
    // : T 1 IF 2 ELSE 3 ENDIF ;
    assert_eq!(
        branch::target(&INLINE, inline::CHECK_IF, 3, 5),
        8,
        "IF to the ELSE arm"
    );
    assert_eq!(
        branch::target(&INLINE, inline::CHECK_ELSE, 7, 3),
        10,
        "ELSE past the ENDIF"
    );
    // : T 1 =IF 2 ENDIF ;
    assert_eq!(branch::target(&INLINE, inline::CHECK_EIF, 3, 3), 6, "=IF");
    // : T BEGIN 1 UNTIL ;
    assert_eq!(
        branch::target(&INLINE, inline::UNTIL, 3, 3),
        0,
        "UNTIL back to BEGIN"
    );
    // : T BEGIN 1 WHILE 2 REPEAT ;
    assert_eq!(
        branch::target(&INLINE, inline::LOOP_BREAK, 3, 5),
        8,
        "WHILE out"
    );
    assert_eq!(
        branch::target(&INLINE, inline::REPEAT, 7, 7),
        0,
        "REPEAT back"
    );
    // : T 0 10 DO 1 LOOP ;  /  : T 0 10 DO 1 2 +LOOP ;
    assert_eq!(
        branch::target(&INLINE, inline::LOOP_END, 8, 3),
        5,
        "LOOP body"
    );
    assert_eq!(
        branch::target(&INLINE, inline::ADD_LOOP, 10, 5),
        5,
        "+LOOP body"
    );
}

/// Forward and backward are a property of the word, not of the operand — and
/// which word is which comes off the bound kernel, not off a literal.
#[test]
fn only_four_branch_words_go_forward() {
    for f in [
        inline::CHECK_IF,
        inline::CHECK_EIF,
        inline::CHECK_ELSE,
        inline::LOOP_BREAK,
    ] {
        assert!(INLINE.is_forward(f), "{f} should go forward");
    }
    for b in [
        inline::UNTIL,
        inline::REPEAT,
        inline::LOOP_END,
        inline::ADD_LOOP,
        inline::U_LOOP_END,
    ] {
        assert!(!INLINE.is_forward(b), "{b} should go backward");
        assert!(INLINE.is_branch(b), "{b} is still a branch");
    }
    assert!(
        !INLINE.is_branch(inline::PUT_LIT),
        "_PutLit is not a branch"
    );
}

// -------------------------------------------------------------- the address model

/// A call cell counts cells; an address counts bytes.
///
/// The two are four apart and the difference is invisible for a plain variable,
/// which is why it went unnoticed until the game did arithmetic on an address.
/// The location table's first entry, `0x012d0030`, only points at a word body
/// when its `0x30` is read as bytes.
#[test]
fn a_call_cell_counts_cells_and_an_address_counts_bytes() {
    assert_eq!(Address::from_call(0x0002_000c), Address(0x0002_0030));
    assert_eq!(Address::from_call(0x0002_000c).offset(), 0x30);
    assert_eq!(Address::new(2, 0x30).to_string(), "2:0x30");
    assert_eq!(Address::new(2, 0x30).next(), Address::new(2, 0x34));
    assert_eq!(Address::new(2, 0x30).plus_cells(3), Address::new(2, 0x3c));
    assert_eq!(CELL, 4);
}

/// The offset half is masked to sixteen bits; the module half is not.
///
/// Deliberate, and the one place the two notions of "a legal module" differ:
/// `Engine::callable` masks an untrusted value to fourteen bits, this does not
/// mask a module number its caller already knows.
#[test]
fn new_masks_the_offset_but_not_the_module() {
    assert_eq!(Address::new(1, 0x1_0004).0, 0x0001_0004);
    assert_eq!(Address::new(1, 0x1_0004).module(), 1);
    assert_eq!(Address::new(1, 0x1_0004).offset(), 4);
}

// ------------------------------------------------------------------- arithmetic

/// The vectors measured against the original, one per arithmetic word.
///
/// `3 5 -` is −2 and not 2: the handler subtracts the top of stack from the
/// one below it, which is standard Forth and worth pinning because the
/// opposite reading produces plausible numbers everywhere it is wrong.
#[test]
fn arithmetic_matches_the_original() {
    assert_eq!(run(&[prim("+"), 0], &[3, 5]), [8]);
    assert_eq!(run(&[prim("-"), 0], &[3, 5]), [-2], "3 5 - is -2");
    assert_eq!(run(&[prim("*"), 0], &[3, 5]), [15]);
    assert_eq!(run(&[prim("/"), 0], &[20, 4]), [5]);
    assert_eq!(run(&[prim("MOD"), 0], &[10, 3]), [1]);
}

/// Division by zero stops rather than answering something.
///
/// **This is our choice, not a measurement.** What a Watcom-built DOS binary
/// does with `idiv` by zero is a trap, and what the original engine made of
/// that is not established. Refusing is the one answer that cannot be quietly
/// wrong.
#[test]
fn division_by_zero_is_refused() {
    assert!(matches!(
        fails(&[prim("/"), 0], &[1, 0]),
        Error::DivideByZero { .. }
    ));
    assert!(matches!(
        fails(&[prim("MOD"), 0], &[1, 0]),
        Error::DivideByZero { .. }
    ));
}

/// Both halves of the shared arm report the word that was running.
///
/// `/` and `MOD` share one arm, so both pops have to name the word that is
/// actually running. A fixed `/` there reports an underflow in `MOD` against a
/// word that was never on the stack — an error message that sends the reader to
/// the wrong place.
#[test]
fn an_underflow_names_the_word_that_ran() {
    match fails(&[prim("MOD"), 0], &[1]) {
        Error::StackUnderflow { word, .. } => assert_eq!(word, "MOD"),
        other => panic!("wanted an underflow, got {other}"),
    }
}

// ------------------------------------------------------------ truth and logic

/// This engine's truth value is 1, not the −1 many Forths use.
///
/// Measured in the original: `1 1 = R !` stored 1. The `-1` that appears at the
/// top of every comparison handler is set *before* the stack-depth check — it
/// is the "arguments are there" flag, not the result. Mistaking the first for
/// the second is what the oracle caught.
#[test]
fn true_is_one() {
    assert_eq!(run(&[prim("="), 0], &[1, 1]), [1]);
    assert_eq!(run(&[prim("="), 0], &[1, 2]), [0]);
    assert_eq!(run(&[prim("<"), 0], &[3, 5]), [1], "3 5 < is 1");
    assert_eq!(run(&[prim(">"), 0], &[3, 5]), [0], "3 5 > is 0");
    assert_eq!(run(&[prim("0="), 0], &[0]), [1]);
    assert_eq!(run(&[prim("NOT"), 0], &[7]), [0], "NOT is logical");
}

/// `AND` and `OR` are **logical**; `&` and `|` are the bitwise pair.
///
/// The handlers compare both operands against zero and push 1 or 0. Reading
/// them as bitwise is what kept the game in its intro: `ICTRL` asks
/// `_NEXTLOC @ _INVMODE @ 1 <= AND`, and with location 2 pending, `2 & 1` came
/// out 0 and the next location never arrived.
#[test]
fn and_or_are_logical_not_bitwise() {
    assert_eq!(run(&[prim("AND"), 0], &[2, 1]), [1], "2 AND 1 is 1, not 0");
    assert_eq!(run(&[prim("AND"), 0], &[2, 0]), [0]);
    assert_eq!(run(&[prim("OR"), 0], &[0, 4]), [1]);
    assert_eq!(
        run(&[prim("&"), 0], &[2, 1]),
        [0],
        "and & really is bitwise"
    );
    assert_eq!(run(&[prim("|"), 0], &[2, 1]), [3]);
    assert_eq!(run(&[prim("~"), 0], &[0]), [-1], "~ is the complement");
}

// ----------------------------------------------------------------- the stack

#[test]
fn the_stack_words_do_what_they_say() {
    assert_eq!(run(&[prim("DUP"), 0], &[7]), [7, 7]);
    assert_eq!(run(&[prim("DROP"), 0], &[7, 8]), [7]);
    assert_eq!(run(&[prim("SWAP"), 0], &[7, 8]), [8, 7]);
    assert_eq!(run(&[prim("OVER"), 0], &[7, 8]), [7, 8, 7]);
    assert_eq!(run(&[prim("ROT"), 0], &[1, 2, 3]), [2, 3, 1]);
}

/// `I` is whatever is on top of the return stack, loop counter or not.
///
/// Measured: `4711 >R I` gives 4711.
#[test]
fn i_reads_the_return_stack_and_not_only_a_loop() {
    // `R>` puts it back before the return: a `>R` left standing would make the
    // body's terminating cell return *to* 4711 rather than to the caller.
    let cells = &[prim(">R"), prim("I"), prim("R>"), prim("DROP"), 0];
    assert_eq!(run(cells, &[4711]), [4711]);
}

// ------------------------------------------------------------------- memory

/// A read from a module that is not loaded answers zero and is counted.
///
/// Not an error, on purpose. Location 5's `KRSCHRZIEHE` reads a word that was
/// never compiled, and refusing to answer stopped the game on five of the
/// game's locations. Zero is the only value that can be justified, and the read
/// is recorded so the silence is measurable rather than invisible.
#[test]
fn a_read_from_an_unloaded_module_answers_zero_and_is_counted() {
    let mut vm = machine(&[0], &[]);
    let stray = Address::new(99, 0x10);
    assert_eq!(vm.fetch(stray).expect("a stray read is not an error"), 0);
    assert_eq!(vm.mem.loose().get(&stray), Some(&1));

    // A write to the same place is swallowed the same way.
    vm.store(stray, 5)
        .expect("a stray write is not an error either");
    assert!(vm.mem.loose().contains_key(&stray));
}

/// `=>ERASE` gives a module's memory back: once it is unloaded, a call into
/// it stops, a read answers zero and is counted, and loading it again starts
/// it over from the container's image.
#[test]
fn an_unloaded_module_is_gone_until_it_is_loaded_again() {
    let mut vm = machine(&[prim("DUP"), 0], &[]);
    let cell = Address::new(1, CELL);
    vm.store(cell, 77).expect("a write into a loaded module");
    assert_eq!(vm.fetch(cell).expect("read back"), 77);

    assert!(vm.unload(1), "module 1 was loaded");
    assert!(!vm.unload(1), "and is given back once");
    assert!(vm.module(1).is_none());
    assert!(
        matches!(
            vm.call(Address::new(1, 0), &mut NullHost),
            Err(Error::OutOfRange { .. })
        ),
        "a call into it stops"
    );
    assert_eq!(vm.fetch(cell).expect("a stray read"), 0);
    assert_eq!(vm.mem.loose().get(&cell), Some(&1));

    let (item, parsed) = module(&[prim("DUP"), 0]);
    vm.load(&item, &parsed);
    assert_eq!(
        vm.fetch(cell).expect("loaded again"),
        0,
        "the image is the container's, not the one written to"
    );
}

/// Instruction fetch stays strict where data fetch is lenient.
///
/// The leniency above is about *data*. Running into an unloaded module is a
/// jump to nowhere and has to stop.
#[test]
fn a_call_into_an_unloaded_module_stops() {
    // A call cell naming module 9, which was never loaded.
    let call = 0x0009_0000;
    assert!(matches!(
        fails(&[call, 0], &[]),
        Error::NoSuchModule { module: 9, .. }
    ));
}

/// `C@` and `C!` are real byte accesses, not the low byte of a cell.
#[test]
fn byte_access_is_byte_access() {
    let mut vm = machine(&[0x0403_0201, 0], &[]);
    assert_eq!(vm.mem.fetch_byte(Address::new(1, 0)).unwrap(), 0x01);
    assert_eq!(vm.mem.fetch_byte(Address::new(1, 2)).unwrap(), 0x03);
    vm.mem.store_byte(Address::new(1, 2), 0xff).unwrap();
    assert_eq!(vm.fetch(Address::new(1, 0)).unwrap(), 0x04ff_0201);
}

// ------------------------------------------------------------- inline runtimes

/// `_PutLit` pushes the cell after it and steps over it.
#[test]
fn put_lit_pushes_its_operand() {
    assert_eq!(run(&[op(inline::PUT_LIT), 42, 0], &[]), [42]);
}

/// `_PutAdr` pushes the address of the cell after it, and returns.
///
/// Returning is the point: across all 86 shipped modules it occurs only as the
/// first cell of a body, which is how a variable answers with its own address.
#[test]
fn put_adr_pushes_the_following_address_and_returns() {
    // Cell 1 is at byte offset 4. The trailing cells are never reached.
    let left = run(&[op(inline::PUT_ADR), 0, prim("DUP"), 0], &[]);
    assert_eq!(left, [cell::signed(Address::new(1, CELL).0)]);
}

/// `_PutConst` answers with the following cell and returns.
#[test]
fn put_const_answers_and_returns() {
    assert_eq!(
        run(&[op(inline::PUT_CONST), 4711, prim("DUP"), 0], &[]),
        [4711]
    );
}

// ----------------------------------------------------------------- control flow

/// `IF` branches when the flag is **false**, and the distance is in cells.
#[test]
fn if_skips_its_arm_when_the_flag_is_false() {
    // 0 IF 111 ENDIF  →  the literal is skipped
    let taken = &[
        op(inline::PUT_LIT),
        1,
        op(inline::CHECK_IF),
        3,
        op(inline::PUT_LIT),
        111,
        0,
    ];
    assert_eq!(run(taken, &[]), [111], "a true flag falls through");

    let skipped = &[
        op(inline::PUT_LIT),
        0,
        op(inline::CHECK_IF),
        3,
        op(inline::PUT_LIT),
        111,
        0,
    ];
    assert_eq!(run(skipped, &[]), [], "a false flag branches past the arm");
}

/// `=IF` pops two and runs the arm when they are equal.
///
/// Measured in the original: `3 3 =IF 111 R ! ENDIF` stored 111, and
/// `3 4 =IF …` left R untouched.
#[test]
fn eif_runs_its_arm_only_when_the_two_agree() {
    let body = |a: i32, b: i32| {
        vec![
            op(inline::PUT_LIT),
            cell::unsigned(a),
            op(inline::PUT_LIT),
            cell::unsigned(b),
            op(inline::CHECK_EIF),
            3,
            op(inline::PUT_LIT),
            111,
            0,
        ]
    };
    assert_eq!(run(&body(3, 3), &[]), [111], "3 3 =IF runs the arm");
    assert_eq!(run(&body(3, 4), &[]), [], "3 4 =IF does not");
}

/// **`WHILE` leaves the loop when the flag is true** — the opposite of standard
/// Forth.
///
/// Measured against the original: `0 BEGIN DUP 3 >= WHILE 1 + REPEAT` counts up
/// to 3. Read the standard way — `DUP 3 <` — it stops immediately at 0, and the
/// difference is not subtle: with the standard reading the game never got past
/// its first counted wait.
#[test]
fn while_leaves_the_loop_when_the_flag_is_true() {
    // 0 BEGIN DUP 3 >= WHILE 1 + REPEAT
    //  cell: 0 _PutLit 1 0 | 2 DUP | 3 _PutLit 4 3 | 5 >= | 6 _LoopBreak 7 <d> |
    //        8 _PutLit 9 1 | 10 + | 11 _Repeat 12 <d> | 13 end
    let cells = &[
        op(inline::PUT_LIT),
        0,
        prim("DUP"),
        op(inline::PUT_LIT),
        3,
        prim(">="),
        op(inline::LOOP_BREAK),
        6, // operand at cell 7, forward 6 → cell 13, past the REPEAT
        op(inline::PUT_LIT),
        1,
        prim("+"),
        op(inline::REPEAT),
        10, // operand at cell 12, backward 10 → cell 2, the BEGIN
        0,
    ];
    assert_eq!(run(cells, &[]), [3], "it counts up to 3 and stops");
}

/// `DO` takes its arguments as `limit start`, and `LOOP` steps by one.
///
/// Measured by running a counted loop in the original: `0 5 0 DO I + LOOP`
/// leaves 10, which is 0+1+2+3+4.
#[test]
fn a_counted_loop_sums_its_index() {
    // 0 5 0 DO I + LOOP
    let cells = &[
        op(inline::PUT_LIT),
        0, // the accumulator
        op(inline::PUT_LIT),
        5, // limit
        op(inline::PUT_LIT),
        0, // start
        prim("_LoopStart"),
        prim("I"),
        prim("+"),
        op(inline::LOOP_END),
        3, // operand at cell 10, backward 3 → cell 7, the body
        0,
    ];
    assert_eq!(run(cells, &[]), [10]);
}

// --------------------------------------------------------------------- random

/// `RANDOM` is repeatable, which is what deterministic rendering rests on.
///
/// **Not the original's generator.** What that was is unknown; this pins ours,
/// so that a change to it shows up here rather than as a mysteriously
/// different picture three tests away.
#[test]
fn random_is_repeatable_from_a_fixed_seed() {
    let five = |n: i32| {
        let cells = &[prim("RANDOM"), 0];
        (0..5)
            .scan(machine(cells, &[]), |vm, _| {
                vm.data.push(n);
                vm.call(Address::new(1, 0), &mut NullHost).ok()?;
                vm.data.pop()
            })
            .collect::<Vec<_>>()
    };
    let first = five(1000);
    assert_eq!(first.len(), 5, "five draws");
    assert_eq!(first, five(1000), "and the same five every time");
    assert!(first.iter().all(|&v| (0..1000).contains(&v)));

    // `n` at or below zero answers zero rather than dividing by it.
    assert_eq!(run(&[prim("RANDOM"), 0], &[0]), [0]);
    assert_eq!(run(&[prim("RANDOM"), 0], &[-5]), [0]);
}

/// A seed decides the sequence, so a player's run is not everyone's.
///
/// The other half of the test above: repeatable *without* a seed is what the
/// suite relies on, and different *with* one is what a player gets, because
/// nothing in the original handed out one sequence to everybody. Both have to
/// hold at once or neither is worth anything.
#[test]
fn a_seed_decides_which_sequence_random_draws() {
    let five = |seed: Option<u64>| {
        let cells = &[prim("RANDOM"), 0];
        let mut vm = machine(cells, &[]);
        if let Some(seed) = seed {
            vm.seed(seed);
        }
        (0..5)
            .scan(vm, |vm, _| {
                vm.data.push(1000);
                vm.call(Address::new(1, 0), &mut NullHost).ok()?;
                vm.data.pop()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(five(Some(4711)), five(Some(4711)), "one seed, one sequence");
    assert_ne!(five(Some(4711)), five(Some(4712)), "two seeds, two");
    assert_ne!(
        five(Some(4711)),
        five(None),
        "a seeded run is not the unseeded one"
    );
    assert!(five(Some(4711)).iter().all(|&v| (0..1000).contains(&v)));
}

// ----------------------------------------------------------------- the host seam

/// A word the interpreter does not own is offered to the host, and a host that
/// declines produces a named error rather than silence.
#[test]
fn an_unowned_word_reaches_the_host_and_its_refusal_is_named() {
    match fails(&[prim("NOSUCHWORD"), 0], &[]) {
        Error::Unimplemented { name, .. } => assert_eq!(name, "NOSUCHWORD"),
        other => panic!("wanted Unimplemented, got {other}"),
    }
}

/// An ordinal that is in no kernel table stops the machine.
#[test]
fn an_unknown_ordinal_stops() {
    assert!(matches!(
        fails(&[op(9999), 0], &[]),
        Error::UnknownOrdinal { ordinal: 9999, .. }
    ));
}

/// A program that never returns is stopped by the step limit.
#[test]
fn a_runaway_program_is_stopped() {
    // `0 BEGIN 0 UNTIL` — a false flag sends it back to the BEGIN every time,
    // and each turn pushes its own flag, so the stack stays level and nothing
    // but the limit stops it. The operand sits at cell 3 and the jump is
    // backward by 3, i.e. to cell 0.
    let cells = &[op(inline::PUT_LIT), 0, op(inline::UNTIL), 3, 0];
    let mut vm = machine(cells, &[]);
    vm.set_step_limit(500);
    assert!(matches!(
        vm.call(Address::new(1, 0), &mut NullHost),
        Err(Error::StepLimit(500))
    ));
}

// ------------------------------------------------------- the module-table walk

/// Two modules, each defining one word of the same name at the same offset,
/// and the order the caller asks for decides which is found.
///
/// The original resolves a dialogue action's word name by walking its module
/// table at `0xEE6D0` in entry order — the order `=>GET` took slots in — and
/// answering the first match. Nothing in the interpreter knows that order, so
/// [`m32::Memory::lookup`] takes it; this is what says the order is really
/// used rather than merely accepted.
#[test]
fn a_lookup_answers_the_first_module_of_the_order_it_is_given() {
    /// A module holding one word `SAME` whose body is a single literal.
    fn named(number: u32, value: u32) -> (Vec<u8>, ScrModule) {
        // `_PutLit`, the literal, and the zero cell that returns.
        let cells = [TAG_KERNEL | inline::PUT_LIT, value, 0];
        let mut item = vec![0u8; 0x30];
        for c in cells {
            item.extend_from_slice(&c.to_le_bytes());
        }
        let parsed = ScrModule {
            module: number,
            entry: 0,
            entries: vec![motionvm_motion_formats::m32::scr::Entry {
                name: "SAME".into(),
                declared_len: 4,
                flags: 11,
                // `call_offset` is `(offset + 16 - 0x30) / 4`: a header at
                // 0x20 ends exactly at the address base, so the body this
                // entry names is the module's first cell.
                offset: 0x20,
                body: cells.to_vec(),
            }],
            dp_cells: cells.len(),
            second_area_len: 0,
            declared_mem_len: 0,
            tail_len: 0,
        };
        (item, parsed)
    }

    let mut vm = Vm::new(&kernel());
    for (number, value) in [(7u32, 70u32), (9, 90)] {
        let (item, parsed) = named(number, value);
        vm.load(&item, &parsed);
    }

    // Module number order and slot order agree here…
    let seven = vm
        .mem
        .lookup("SAME", &[7, 9])
        .expect("module 7 defines SAME");
    assert_eq!(seven.module(), 7);
    // …and here they do not: the later-numbered module took the earlier slot.
    let nine = vm
        .mem
        .lookup("SAME", &[9, 7])
        .expect("module 9 defines SAME");
    assert_eq!(
        nine.module(),
        9,
        "the walk followed module numbers instead of the order it was given"
    );

    // A module named in the order and not loaded is stepped over rather than
    // ending the walk, the way an empty slot is.
    let past = vm
        .mem
        .lookup("SAME", &[3, 9, 7])
        .expect("SAME is still found");
    assert_eq!(past.module(), 9);
    // And a module loaded but not named is not reached at all.
    assert!(vm.mem.lookup("SAME", &[3]).is_none());
}
