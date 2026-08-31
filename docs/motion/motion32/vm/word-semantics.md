[← Documentation index](../../README.md)

# Word Semantics

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

The behavior of the core Forth words as the original engine implements
them. This dialect deviates from standard Forth in several places; every
deviation listed here has been confirmed against the original engine's
actual behavior, with the observed results given inline.

All arithmetic is 32-bit two's complement.

## Truth values

Comparison words push **1** for true, not the standard Forth −1.

```
1 1 =        → 1
3 5 <        → 1
3 5 >        → 0
```

## Stack manipulation

| Word | Effect |
|---|---|
| `DUP` | `( a -- a a )` |
| `DROP` | `( a -- )` |
| `SWAP` | `( a b -- b a )` |
| `OVER` | `( a b -- a b a )` |
| `ROT` | `( a b c -- b c a )` |

## Arithmetic and comparison

| Word | Effect | Notes |
|---|---|---|
| `+` `-` `*` | `( a b -- n )` | wrapping 32-bit |
| `/` | `( a b -- a/b )` | truncating; `20 4 /` → 5 |
| `MOD` | `( a b -- a%b )` | `10 3 MOD` → 1 |
| `=` `!=` `<` `>` `<=` `>=` | `( a b -- f )` | signed; true = 1 |
| `0=` `0>` `0<` | `( a -- f )` | |
| `3 5 -` | | → −2 (operand order as in standard Forth) |

Behavior of `/` and `MOD` with negative operands is unverified, as is
division by zero.

## Logic and bit operations

| Word | Effect | Notes |
|---|---|---|
| `NOT` | `( a -- f )` | **logical** not (`a == 0`), not bitwise |
| `AND` / `OR` | `( a b -- f )` | **logical, confirmed in the handlers**: both operands are compared against zero and 1 or 0 is pushed — no `and`/`or` instruction touches the values. Treating them as bitwise is a trap that fails silently: `_NEXTLOC @ _INVMODE @ 1 <= AND` with location 2 pending computes `2 & 1 = 0` bitwise, and the game never changes scene |
| `&` / `\|` | `( a b -- n )` | the **bitwise** pair (real `and`/`or` in the handlers); game code combines capability bit masks with `\|` |
| `<<` `>>` | `( a b -- n )` | `>>` is an arithmetic (sign-preserving) shift |
| `~` | `( a -- ~a )` | bitwise complement |

A handler-reading note on truth values: every comparison/logic handler
begins by setting a local to `0xffffffff` — that is an "arguments
present" flag set before the stack-depth check, **not** the result. The
result is computed separately and is **1**, for `=`, `<`, `NOT`, `0=`,
`AND`, and all the rest alike.

## Memory access

| Word | Effect | Notes |
|---|---|---|
| `@` | `( addr -- n )` | cell fetch; offset truncated to a cell boundary |
| `!` | `( n addr -- )` | cell store; **pops the address first, then the value** |
| `C@` | `( addr -- b )` | true byte fetch, not the low byte of a cell |
| `C!` | `( b addr -- )` | byte store |

Addresses are packed `(module << 16) | byte offset` values — see
[Execution model](execution-model.md).

## Return stack

`>R` and `R>` move values to and from **the same return stack the
interpreter uses for calls**. A mismatched pair therefore corrupts control
flow rather than raising an error. `I` is literally the **top of the
return stack**: `4711 >R I` yields 4711, and inside a loop a pending
`>R` shadows the loop index.

## Loops

### `DO … LOOP`

`DO` follows standard Forth: `( limit start -- )`, i.e. `limit` is pushed
first. Confirmed: `0 5 0 DO I + LOOP` leaves **10** (0+1+2+3+4).

- `I` pushes the innermost loop index, `J` the next-outer one.
- `LOOP` increments the index by 1 and exits when `index >= limit`.
- `+LOOP` pops the step from the stack: `0 3 0 DO I 2 * + LOOP 100 +`
  leaves 106. The termination rule depends on the step's sign: exit when
  `index >= limit` for a non-negative step, `index <= limit` for a negative
  one.
- `/LOOP` compiles to its own runtime (`_ULoopEnd`) but its termination
  rule has not been measured separately (see open questions).
- `LEAVE` abandons the current loop frame. Whether it also jumps to the
  cell after the loop end, or merely drops the frame and lets execution
  continue, is unverified.

### `BEGIN … UNTIL`

`UNTIL` pops a flag and loops back **while the flag is false**. Confirmed:
`0 BEGIN 1 + DUP 4 >= UNTIL` leaves 4.

### `BEGIN … WHILE … REPEAT` — inverted!

`WHILE` **exits the loop when the flag is true** — the opposite of standard
Forth. The kernel's internal name for it, `_LoopBreak`, describes the
actual behavior. Confirmed by a decisive pair:

```
0 BEGIN DUP 3 >= WHILE 1 + REPEAT      → 3   (counts up, exits at 3)
0 BEGIN DUP 3 <  WHILE 1 + REPEAT      → 0   (exits immediately)
```

Under the standard Forth reading the results would be swapped. This is the
single most dangerous word to get wrong: the standard meaning looks
self-evident and produces plausible-looking behavior for a while.

### `=IF`

`=IF` compares **two** values, consuming both, and runs its arm when they
are equal. Confirmed:

```
7 R !  3 3 =IF 111 R ! ENDIF     → R = 111
7 R !  3 4 =IF 111 R ! ENDIF     → R = 7
```

## The four "Put" runtimes

| Word | Behavior | Corpus evidence |
|---|---|---|
| `_PutLit` | Inline instruction: push the following cell, continue | 12,578 uses, freely mid-body |
| `_PutAdr` | **Complete word behavior**: push the address of the following cell (the variable's storage), then return | 1,491 uses, always the first cell of a body |
| `_PutConst` | **Complete word behavior**: push the following cell, then return | 486 uses, always the first cell of a body |
| `_PutStringAdr` | Inline instruction: push the address of the following **string**, then step over it and continue | 147 uses, never the first cell of a body |

`_PutStringAdr` (handler `0x665da`) is the `_PutLit` shape with a
variable-length operand: it pushes an ordinary packed address pointing at the
text itself, so `CompareString` and `ADDMESSPIPE` can take it straight, and
then advances by `(strlen + 3) >> 2` cells — the terminator is *not* counted.

That is one cell less than the compiler emitted whenever the length divides
by four. In the whole game exactly one string is affected, `"GANRUFBA"`, at
four sites; there the handler steps onto the padding cell, which is zero and
therefore a return. The long way round through the `_CheckElse` chain reaches
the same return with the same value on the stack, so the two are
indistinguishable from outside — which is presumably why it survived. Reading
a body statically needs the compiler's `(len + 1 + 3) & ~3` rather than the
handler's count; see [Departures](../../departures.md).

`_PutAdr`/`_PutConst` are the compiled forms of `VAR` and `CONST`: the
entire definition consists of the runtime cell plus its operand. That the
variable's storage cell lives *inside* the word body is why module memory
must be writable (see [Execution model](execution-model.md)).

## Miscellaneous

| Word | Effect | Notes |
|---|---|---|
| `EXECUTE` | `( addr -- )` | call the word at the packed address |
| `RANDOM` | `( n -- r )` | random value; exact generator and range behavior unverified |

## Open questions

- `/LOOP` (`_ULoopEnd`): only its branch distance encoding is measured. The
  `U` prefix suggests an unsigned termination rule, distinct from `+LOOP`'s
  signed one; unconfirmed.
- `LEAVE`'s exact continuation point.
- `/` and `MOD` with negative operands; division by zero.
- `RANDOM`'s generator and distribution.

## See also

- [Threaded code](threaded-code.md) — how these words are encoded
- [Execution model](execution-model.md) — stacks and addressing
- [Kernel words](kernel-words.md) — the full word inventory
- [Departures](../../departures.md) — `_PutStringAdr` when walking a body
