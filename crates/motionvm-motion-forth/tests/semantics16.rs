//! What the 16-bit interpreter does, held against what the modules of Die
//! Enviro-Kids greifen ein establish — and, where only a handler could say,
//! against the 32-bit engine's measured rule taken as the hypothesis.
//!
//! **No game data.** Every module here is assembled in memory in the 16-bit
//! layout, and the kernel is the real core table of `ENVIRO.EXE` by name and
//! ordinal, typed out — so this file runs on any machine. What it pins as
//! *measured* is the shape of the machine the modules demand: the return
//! word, the word-id table, the variable and constant runtimes, the branch
//! arithmetic, `DO`'s argument order, the 16-bit wrap. What it pins as *our
//! choice* it says so.

use motionvm_motion_formats::m16::scr::ScrModule;
use motionvm_motion_formats::{Binding, Inline};
use motionvm_motion_forth::cell;
use motionvm_motion_forth::m16::{Vm, word_address};
use motionvm_motion_forth::{Address, Error, Host, NullHost, Result, Run};

// ---------------------------------------------------------------- the kernel

/// The core table of `ENVIRO.EXE`, in table order: ordinal = index + 1.
const CORE: [&str; 82] = [
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
    ":",
    "VAR",
    "@",
    "!",
    "ALLOT",
    "CONST",
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
    ">>",
    "<<",
    "~",
    "&",
    "|",
    ">R",
    "R>",
    "I",
    "I'",
    "_PutAdr",
    "_PutConst",
    "_LoopStart",
    "_LoopEnd",
    "_CheckIf",
    "_CheckEIf",
    "_ChElseDup",
    "_CheckElse",
    "KEY",
    "?KEY",
    "_AddLoop",
    "_ULoopEnd",
    "LEAVE",
    "_Until",
    "_LoopBreak",
    "_Repeat",
    ",",
    "CREATE",
    "<N>",
    "'",
    "EXECUTE",
    "=>START",
    "=>END",
    "=>TABLES",
    "=>INFO",
    "=>ERASE",
    "=>EXIST",
    "=>PUTAS",
    "=>GETAS",
    "=>PUT",
    "=>GET",
    "=>AT",
    "=>VALID",
    "=>RESET",
    "PUT",
    "GET",
    "ENDEBUG",
    "DISDEBUG",
    "WORD",
    "EMIT",
    ".",
    "_PutString",
    "RANDOM",
    "_PutLit",
    "_PutStringAdr",
    "$->",
];

/// The binding of that table plus two domain words, as the game binds them.
fn binding() -> Binding {
    let mut words: Vec<(u32, String)> = CORE
        .iter()
        .enumerate()
        .map(|(i, n)| (cell::narrow(i) + 1, n.to_string()))
        .collect();
    words.push((105, "TOGFX".into()));
    words.push((106, "GFXTO".into()));
    let inline = Inline::by_name(&words).expect("every inline word is in the core table");
    Binding { words, inline }
}

fn ordinal(name: &str) -> u16 {
    let o = binding()
        .ordinal(name)
        .unwrap_or_else(|| panic!("{name} is not in the kernel"));
    0x8000 | cell::module(o)
}

// --------------------------------------------------------------- the assembler

/// A cell of a word body, as the tests write it.
#[derive(Clone, Copy)]
enum C {
    /// A kernel word by name.
    K(&'static str),
    /// A call to a global id.
    Id(u16),
    /// A raw cell: a literal's value, a branch distance, a data cell.
    N(i16),
}

/// Assembles a 16-bit module from words given as `(id, name, cells)`.
fn module(number: u16, words: &[(u16, &str, &[C])]) -> Vec<u8> {
    let first = words.first().map_or(0, |w| w.0);
    let last = words.last().map_or(0, |w| w.0);
    let n = cell::flat(words.len());
    let mut v = Vec::new();
    for x in [first, last, n, first, last, n] {
        v.extend_from_slice(&x.to_le_bytes());
    }
    v.extend_from_slice(&[0u8; 20]);
    v.extend_from_slice(&number.to_le_bytes());
    // Body offsets in cells from the dictionary start: each word is a
    // 16-byte header (8 cells) and then its cells.
    let mut offsets = Vec::new();
    let mut cursor = 0u16;
    for (_, _, cells) in words {
        cursor += 8;
        offsets.push(cursor);
        cursor += cell::flat(cells.len());
    }
    for o in &offsets {
        v.extend_from_slice(&o.to_le_bytes());
    }
    for (id, name, cells) in words {
        v.push(u8::try_from(name.len()).unwrap());
        let mut nm = name.as_bytes().to_vec();
        nm.resize(11, 0);
        v.extend_from_slice(&nm);
        v.extend_from_slice(&id.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes());
        for c in *cells {
            let cell: u16 = match c {
                C::K(n) => ordinal(n),
                C::Id(id) => *id,
                C::N(n) => cell::unsigned16(*n),
            };
            v.extend_from_slice(&cell.to_le_bytes());
        }
    }
    v
}

/// A machine with one assembled module loaded.
fn machine(number: u16, words: &[(u16, &str, &[C])]) -> Vm {
    let b = binding();
    let mut vm = Vm::new(&b);
    load(&mut vm, number, words);
    vm
}

fn load(vm: &mut Vm, number: u16, words: &[(u16, &str, &[C])]) {
    let item = module(number, words);
    let parsed = ScrModule::parse(&item).expect("the assembled module parses");
    vm.load(&item, &parsed).expect("the module fits");
}

/// Runs `word` of `module` to completion under [`NullHost`] and answers the
/// data stack.
fn run(vm: &mut Vm, module: u16, word: &str) -> Result<Vec<i32>> {
    let at = word_address(vm, module, word).unwrap_or_else(|| panic!("no word {word}"));
    vm.call(at, &mut NullHost)?;
    Ok(vm.data.clone())
}

const RET: C = C::K("##");

// --------------------------------------------------------------- the machine

#[test]
fn the_return_is_the_kernel_word_and_a_zero_cell_is_a_call_to_id_0() {
    // Measured: every colon definition ends with 0x8001.
    let mut vm = machine(1, &[(400, "FIVE", &[C::K("_PutLit"), C::N(5), RET])]);
    assert_eq!(run(&mut vm, 1, "FIVE").unwrap(), [5]);
    // A zero cell is not a return here; it names word id 0, which nothing
    // binds.
    let mut vm = machine(1, &[(400, "ZERO", &[C::N(0)])]);
    match run(&mut vm, 1, "ZERO") {
        Err(Error::UnboundWord { id: 0, .. }) => {}
        other => panic!("expected an unbound id 0, got {other:?}"),
    }
}

#[test]
fn a_variable_pushes_the_address_of_its_cell_and_returns() {
    // `VAR X` is `_PutAdr value`; `_PutAdr` pushes the byte address of the
    // cell after it and returns — measured from every variable body in the
    // modules and from the `ARR n + @` idiom.
    let mut vm = machine(
        1,
        &[
            (400, "X", &[C::K("_PutAdr"), C::N(13)]),
            (401, "GETX", &[C::Id(400), C::K("@"), RET]),
            (
                402,
                "SETX",
                &[C::K("_PutLit"), C::N(-7), C::Id(400), C::K("!"), RET],
            ),
            (403, "K", &[C::K("_PutConst"), C::N(549), C::N(0)]),
        ],
    );
    assert_eq!(run(&mut vm, 1, "GETX").unwrap(), [13]);
    vm.data.clear();
    run(&mut vm, 1, "SETX").unwrap();
    vm.data.clear();
    assert_eq!(
        run(&mut vm, 1, "GETX").unwrap(),
        [-7],
        "stored and read back, signed"
    );
    vm.data.clear();
    // The address `X` leaves is the flat address of its data cell, and it
    // lies inside module 1.
    let addr = cell::low16(run(&mut vm, 1, "X").unwrap()[0]);
    assert_eq!(vm.mem.locate(addr).map(|a| a.module()), Some(1));
    vm.data.clear();
    // A constant pushes its value and returns without touching the cell
    // after it.
    assert_eq!(run(&mut vm, 1, "K").unwrap(), [549]);
}

#[test]
fn arithmetic_wraps_at_sixteen_bits_and_flags_are_one() {
    let mut vm = machine(
        1,
        &[
            (
                400,
                "BIG",
                &[
                    C::K("_PutLit"),
                    C::N(30000),
                    C::K("_PutLit"),
                    C::N(30000),
                    C::K("+"),
                    RET,
                ],
            ),
            (
                401,
                "NEG",
                &[
                    C::K("_PutLit"),
                    C::N(-5),
                    C::K("_PutLit"),
                    C::N(3),
                    C::K("*"),
                    RET,
                ],
            ),
            (
                402,
                "CMP",
                &[
                    C::K("_PutLit"),
                    C::N(2),
                    C::K("_PutLit"),
                    C::N(3),
                    C::K("<"),
                    C::K("_PutLit"),
                    C::N(4),
                    C::K("_PutLit"),
                    C::N(4),
                    C::K("="),
                    C::K("AND"),
                    RET,
                ],
            ),
            (
                403,
                "LOG",
                &[
                    C::K("_PutLit"),
                    C::N(2),
                    C::K("_PutLit"),
                    C::N(1),
                    C::K("AND"),
                    C::K("_PutLit"),
                    C::N(2),
                    C::K("_PutLit"),
                    C::N(1),
                    C::K("&"),
                    RET,
                ],
            ),
            (
                404,
                "DIV",
                &[
                    C::K("_PutLit"),
                    C::N(-7),
                    C::K("_PutLit"),
                    C::N(2),
                    C::K("/"),
                    C::K("_PutLit"),
                    C::N(7),
                    C::K("_PutLit"),
                    C::N(3),
                    C::K("MOD"),
                    RET,
                ],
            ),
        ],
    );
    // Our choice, stated in the machine's docs: the stack holds 16-bit
    // cells sign-extended, so the sum wraps.
    assert_eq!(run(&mut vm, 1, "BIG").unwrap(), [-5536]);
    vm.data.clear();
    assert_eq!(run(&mut vm, 1, "NEG").unwrap(), [-15]);
    vm.data.clear();
    // True is 1, read at `12c8:03f2` and its siblings.
    assert_eq!(run(&mut vm, 1, "CMP").unwrap(), [1]);
    vm.data.clear();
    // `AND` is logical, `&` bitwise: 2 AND 1 is 1, 2 & 1 is 0.
    assert_eq!(run(&mut vm, 1, "LOG").unwrap(), [1, 0]);
    vm.data.clear();
    // Read: `/` divides the low words unsigned (`12c8:0030`), so −7 is
    // 0xfff9 and the quotient 32764; `MOD` the same (`12c8:0072`).
    assert_eq!(run(&mut vm, 1, "DIV").unwrap(), [32764, 1]);
}

/// `/` and `MOD` divide the low words unsigned (`12c8:0030`, `12c8:0072`:
/// `xor dx, dx` then `div`), so a negative dividend is a large one; `/`
/// answers 0 for a zero divisor where `MOD` is the processor's own fault.
#[test]
fn division_is_unsigned_and_a_zero_divisor_answers_zero_or_faults() {
    let word = |a: i16, b: i16, op: &'static str| -> [C; 6] {
        [
            C::K("_PutLit"),
            C::N(a),
            C::K("_PutLit"),
            C::N(b),
            C::K(op),
            RET,
        ]
    };
    let (neg_div, neg_mod, by_zero, mod_zero) = (
        word(-6, 2, "/"),
        word(-7, 4, "MOD"),
        word(5, 0, "/"),
        word(5, 0, "MOD"),
    );
    let mut vm = machine(
        1,
        &[
            (400, "NEGDIV", &neg_div),
            (401, "NEGMOD", &neg_mod),
            (402, "BYZERO", &by_zero),
            (403, "MODZERO", &mod_zero),
        ],
    );
    // 0xfffa / 2 = 0x7ffd, and 0xfff9 % 4 = 1: what an unsigned divide of
    // the cell's bits gives, not −3 and −3.
    assert_eq!(run(&mut vm, 1, "NEGDIV").unwrap(), [0x7ffd]);
    vm.data.clear();
    assert_eq!(run(&mut vm, 1, "NEGMOD").unwrap(), [1]);
    vm.data.clear();
    assert_eq!(run(&mut vm, 1, "BYZERO").unwrap(), [0]);
    vm.data.clear();
    assert!(
        matches!(run(&mut vm, 1, "MODZERO"), Err(Error::DivideByZero { .. })),
        "MOD by zero is a fault"
    );
}

#[test]
fn branches_land_on_the_operand_cell_plus_or_minus_the_distance() {
    // `1 IF 10 ELSE 20 ENDIF`: _CheckIf's operand is at cell 3 and the ELSE
    // arm starts 5 further on; _CheckElse's operand at 7 jumps 3 past the
    // ENDIF. The same distances the 32-bit compiler writes, and the shape of
    // `CTRL`'s opening `_CheckIf 95` landing on 98.
    let body: &[C] = &[
        C::K("_PutLit"),
        C::N(1), // 0 1
        C::K("_CheckIf"),
        C::N(5), // 2 3  -> 8
        C::K("_PutLit"),
        C::N(10), // 4 5
        C::K("_CheckElse"),
        C::N(3), // 6 7  -> 10
        C::K("_PutLit"),
        C::N(20), // 8 9
        RET,      // 10
    ];
    let mut vm = machine(1, &[(400, "IFT", body)]);
    assert_eq!(run(&mut vm, 1, "IFT").unwrap(), [10]);
    let mut body_f = body.to_vec();
    body_f[1] = C::N(0);
    let mut vm = machine(1, &[(400, "IFF", &body_f)]);
    assert_eq!(run(&mut vm, 1, "IFF").unwrap(), [20]);
}

#[test]
fn until_repeats_on_false_and_while_leaves_on_true() {
    // `0 BEGIN 1 + DUP 3 = UNTIL`: counts to 3. _Until's operand at cell 9
    // jumps back 7 to cell 2.
    let until: &[C] = &[
        C::K("_PutLit"),
        C::N(0), // 0 1
        C::K("_PutLit"),
        C::N(1),
        C::K("+"), // 2 3 4
        C::K("DUP"),
        C::K("_PutLit"),
        C::N(3),
        C::K("="), // 5 6 7 8
        C::K("_Until"),
        C::N(8), // 9 10 -> 2
        RET,
    ];
    let mut vm = machine(1, &[(400, "UNT", until)]);
    assert_eq!(run(&mut vm, 1, "UNT").unwrap(), [3]);
    // `0 BEGIN DUP 3 >= WHILE 1 + REPEAT`: the 32-bit engine's WHILE leaves
    // on *true*, measured there, the hypothesis here — so this counts to 3.
    let while_: &[C] = &[
        C::K("_PutLit"),
        C::N(0), // 0 1
        C::K("DUP"),
        C::K("_PutLit"),
        C::N(3),
        C::K(">="), // 2 3 4 5
        C::K("_LoopBreak"),
        C::N(6), // 6 7  -> 13, past REPEAT's operand
        C::K("_PutLit"),
        C::N(1),
        C::K("+"), // 8 9 10
        C::K("_Repeat"),
        C::N(10), // 11 12 -> 2
        RET,      // 13
    ];
    let mut vm = machine(1, &[(400, "WHL", while_)]);
    assert_eq!(run(&mut vm, 1, "WHL").unwrap(), [3]);
}

#[test]
fn do_loop_takes_limit_then_index_and_i_reads_the_return_stack() {
    // `706 701 DO I LOOP` — RUN's save-slot probe — runs I for 701..706.
    // _LoopEnd's operand at cell 7 jumps back 2 to cell 5.
    let body: &[C] = &[
        C::K("_PutLit"),
        C::N(706),
        C::K("_PutLit"),
        C::N(701),          // 0..3
        C::K("_LoopStart"), // 4
        C::K("I"),          // 5
        C::K("_LoopEnd"),
        C::N(2), // 6 7 -> 5
        RET,
    ];
    let mut vm = machine(1, &[(400, "SLOTS", body)]);
    assert_eq!(run(&mut vm, 1, "SLOTS").unwrap(), [701, 702, 703, 704, 705]);
    // `>R I R>` is a copy of the top of the return stack, as `XYLSITEM.`
    // uses it — and a negative value comes back negative.
    let body: &[C] = &[
        C::K("_PutLit"),
        C::N(-2),
        C::K(">R"),
        C::K("I"),
        C::K("R>"),
        RET,
    ];
    let mut vm = machine(1, &[(400, "RCOPY", body)]);
    assert_eq!(run(&mut vm, 1, "RCOPY").unwrap(), [-2, -2]);
}

#[test]
fn a_loop_index_steps_in_sixteen_bits_and_wraps_where_the_add_does() {
    // `-32768 32767 DO I 1 +LOOP`: the body runs once with I at the top of
    // the range, the step carries the index round to -32768 — a 16-bit `add`,
    // not a 32-bit one — and the loop leaves because the limit is not above
    // it. A machine stepping in 32 bits would either run on or refuse.
    let body: &[C] = &[
        C::K("_PutLit"),
        C::N(-32768),
        C::K("_PutLit"),
        C::N(32767),        // 0..3
        C::K("_LoopStart"), // 4
        C::K("I"),          // 5
        C::K("_PutLit"),
        C::N(1), // 6 7
        C::K("_AddLoop"),
        C::N(4), // 8 9 -> 5
        RET,
    ];
    let mut vm = machine(1, &[(400, "EDGE", body)]);
    assert_eq!(run(&mut vm, 1, "EDGE").unwrap(), [32767]);
}

#[test]
fn execute_runs_a_global_id_and_an_unbound_id_is_an_error() {
    let mut vm = machine(
        1,
        &[
            (400, "SEVEN", &[C::K("_PutLit"), C::N(7), RET]),
            (
                401,
                "GO",
                &[C::K("_PutLit"), C::N(400), C::K("EXECUTE"), RET],
            ),
            (
                402,
                "GONE",
                &[C::K("_PutLit"), C::N(999), C::K("EXECUTE"), RET],
            ),
        ],
    );
    assert_eq!(run(&mut vm, 1, "GO").unwrap(), [7]);
    vm.data.clear();
    match run(&mut vm, 1, "GONE") {
        Err(Error::UnboundWord { id: 999, at }) => assert_eq!(at.module(), 1),
        other => panic!("{other:?}"),
    }
}

#[test]
fn ids_bind_to_the_module_loaded_last_and_unbind_when_it_goes() {
    // Sixteen of ENVIRO's modules define id 549; the loader runs whichever is
    // resident. Load two, erase one, and the id follows.
    let b = binding();
    let mut vm = Vm::new(&b);
    load(
        &mut vm,
        301,
        &[(549, "MUELL", &[C::K("_PutLit"), C::N(1), RET])],
    );
    load(
        &mut vm,
        302,
        &[(549, "BIRKEN", &[C::K("_PutLit"), C::N(2), RET])],
    );
    load(
        &mut vm,
        601,
        &[
            (548, "LOCINIT", &[C::K("_PutConst"), C::N(549)]),
            (550, "INIT", &[C::Id(548), C::K("EXECUTE"), RET]),
        ],
    );
    assert_eq!(
        run(&mut vm, 601, "INIT").unwrap(),
        [2],
        "the module loaded last holds the id"
    );
    vm.data.clear();
    // Erasing the module that holds the id leaves the id unbound — the table
    // has one slot per id, and what the original does with two resident
    // modules defining the same id is unknown; ENVIRO never has two.
    assert!(vm.unload(302));
    assert!(matches!(
        run(&mut vm, 601, "INIT"),
        Err(Error::UnboundWord { id: 549, .. })
    ));
    vm.data.clear();
    assert!(vm.unload(301));
    // The space is reused: loading again lands where the first one sat.
    let base_again = {
        let item = module(301, &[(549, "MUELL", &[C::K("_PutLit"), C::N(1), RET])]);
        vm.load(&item, &ScrModule::parse(&item).unwrap()).unwrap()
    };
    assert_eq!(base_again, 0x100, "first-fit from the first base");
    assert_eq!(
        run(&mut vm, 601, "INIT").unwrap(),
        [1],
        "and the id is bound again"
    );
}

#[test]
fn a_module_that_does_not_fit_is_refused_not_placed() {
    let b = binding();
    let mut vm = Vm::new(&b);
    // A word with 30 000 data cells is a 60 KB module; two of them do not
    // fit a 64 KiB space, and the second is refused whole.
    let big = vec![C::N(0); 30_000];
    let mut words = vec![C::K("_PutAdr")];
    words.extend(big);
    let item = module(1, &[(400, "HUGE", &words)]);
    let parsed = ScrModule::parse(&item).unwrap();
    vm.load(&item, &parsed).unwrap();
    let item2 = module(2, &[(401, "HUGE2", &words)]);
    let parsed2 = ScrModule::parse(&item2).unwrap();
    assert!(matches!(
        vm.load(&item2, &parsed2),
        Err(Error::OutOfMemory { module: 2, .. })
    ));
    assert!(!vm.mem.is_loaded(2));
}

#[test]
fn a_host_word_gets_the_machine_and_an_unknown_one_is_named() {
    /// A host that answers `TOGFX` by pushing a marker.
    struct Togfx;
    impl Host<Vm> for Togfx {
        fn word(&mut self, ordinal: u32, vm: &mut Vm) -> Result<bool> {
            if vm.ordinal_name(ordinal) == Some("TOGFX") {
                vm.data.push(0x7777);
                return Ok(true);
            }
            Ok(false)
        }
    }
    let mut vm = machine(1, &[(400, "GFX", &[C::K("TOGFX"), C::K("GFXTO"), RET])]);
    let at = word_address(&vm, 1, "GFX").unwrap();
    let err = vm.call(at, &mut Togfx).unwrap_err();
    assert_eq!(vm.data, [0x7777]);
    match err {
        Error::Unimplemented { name, ordinal, .. } => {
            assert_eq!((name.as_str(), ordinal), ("GFXTO", 106));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_string_operand_is_skipped_whole_and_its_address_pushed() {
    // `_PutStringAdr "Hi"` occupies (2 + 2) / 2 = 2 cells: "Hi\0" and a pad
    // byte. The word after it must still run.
    let mut vm = machine(
        1,
        &[(
            400,
            "S",
            &[
                C::K("_PutStringAdr"),
                C::N(i16::from_le_bytes(*b"Hi")),
                C::N(0),
                C::K("_PutLit"),
                C::N(9),
                RET,
            ],
        )],
    );
    let stack = run(&mut vm, 1, "S").unwrap();
    assert_eq!(stack[1], 9);
    let addr = cell::low16(stack[0]);
    assert_eq!(vm.mem.read_bytes(addr, 2).unwrap(), b"Hi");
}

#[test]
fn a_pause_yields_and_resumes_where_it_stood() {
    /// Pauses after `TOGFX`, once.
    struct Pauser(bool);
    impl Host<Vm> for Pauser {
        fn word(&mut self, ordinal: u32, vm: &mut Vm) -> Result<bool> {
            if vm.ordinal_name(ordinal) == Some("TOGFX") {
                self.0 = true;
                return Ok(true);
            }
            Ok(false)
        }
        fn wants_pause(&mut self) -> bool {
            std::mem::take(&mut self.0)
        }
    }
    let mut vm = machine(
        1,
        &[(400, "P", &[C::K("TOGFX"), C::K("_PutLit"), C::N(4), RET])],
    );
    let at = word_address(&vm, 1, "P").unwrap();
    let mut host = Pauser(false);
    vm.start(at).unwrap();
    assert_eq!(vm.resume(&mut host).unwrap(), Run::Yielded);
    assert!(vm.data.is_empty());
    assert_eq!(vm.resume(&mut host).unwrap(), Run::Done);
    assert_eq!(vm.data, [4]);
    // And `call` refuses a pausing host rather than losing the position.
    vm.data.clear();
    assert!(matches!(
        vm.call(at, &mut Pauser(false)),
        Err(Error::Suspended { .. })
    ));
}

#[test]
fn an_address_reports_as_module_and_offset() {
    let vm = machine(7, &[(400, "W", &[RET])]);
    let at = word_address(&vm, 7, "W").unwrap();
    assert_eq!(at.module(), 7);
    // The body of the first word sits 16 bytes past the 0x22 + 2 header.
    assert_eq!(at.offset(), 0x22 + 2 + 16);
    assert_eq!(vm.mem.flat(at), Some(0x100 + 0x22 + 2 + 16));
    assert_eq!(
        vm.mem.locate(0x100 + 0x22 + 2 + 16),
        Some(Address::new(7, 0x22 + 2 + 16))
    );
}
