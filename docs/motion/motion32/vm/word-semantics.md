[← Documentation index](../../README.md)

# Word Semantics

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2 and V0.04.15/R78 with Checker 2000; what is measured here is measured on those games' files, and an address is R109's unless the page says otherwise. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

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
| `/` | `( a b -- a/b )` | toward zero; `20 4 /` → 5, `-7 2 /` → −3, `7 -2 /` → −3 |
| `MOD` | `( a b -- a%b )` | the dividend's sign; `10 3 MOD` → 1, `-7 2 MOD` → −1, `7 -2 MOD` → 1 |
| `=` `!=` `<` `>` `<=` `>=` | `( a b -- f )` | signed; true = 1 |
| `0=` `0>` `0<` | `( a -- f )` | |
| `3 5 -` | | → −2 (operand order as in standard Forth) |

Both handlers (`/` at `0x61cff`, `MOD` at `0x61da9`) divide with `idiv` on
the sign-extended dividend, which is where the signs above come from; the
negative cases are measured in the original as well. Neither reaches `idiv`
with a zero divisor: each tests it first and, on a zero, reports through
`0x5294d`, sets the flag at `0xdb043` and leaves 0 where the result goes.

The report is the engine's error box: message 0 of its table, *Division
durch Null!*, in a grey *Fehler* panel with an *OK* button at 40,40
(`0x53246`–`0x532e9`), and the engine waits there for the click. The flag
is what the threaded-code loop tests after every word (`0x61173`,
`0x61180`): set, it leaves the word it is running. Measured in the
original, with the click made by script:

- **In a word the frame loop runs** (the `CTRL` word), the rest of that
  word is skipped for the frame and the loop goes on: a counter the word
  advances reached all 200 frames, and the store after the division never
  happened.
- **In the boot script**, the engine ends after the *OK*, without running
  the lines after the failing one.
- **Without the click**, the original waits at the box.

motionvm stops the run instead; see the departures ledger.

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
- `LOOP` (`_LoopEnd`, `0x63fd7`) increments the index by 1 and exits when
  `index >= limit`, signed (`incl`, `cmp`, `jge`).
- `+LOOP` (`_AddLoop`, `0x641bf`) pops the step and adds it:
  `0 3 0 DO I 2 * + LOOP 100 +` leaves 106. It exits by the same `jge`,
  **whatever the step's sign** — a loop counting down ends at the first step
  that leaves its index at or above the limit. Measured: `0 3 DO … -1 +LOOP`
  runs its body once. `0 -3 DO … -1 +LOOP`, a loop counting away from its
  limit, brings no module back from the original within the probe's time —
  what the `jge` predicts, though a timeout cannot prove it.
- `/LOOP` (`_ULoopEnd`, `0x6405e`) is instruction for instruction
  `_LoopEnd`: a step of one, a signed compare, and **nothing taken from the
  data stack**. Measured: `10 0 DO … 2 /LOOP` runs its body ten times, the 2
  staying on the stack each pass.
- `LEAVE` (`0x6426f`) copies the index over the limit and returns. The rest
  of the body runs, and the loop's end — which steps the index past the
  limit — falls out. Measured: `10 0 DO … LEAVE R @ 100 + R ! LOOP` adds the
  100 once; nested, it ends only the innermost loop. Without a loop frame
  under it, the handler's return-stack check (`0x65b7c`) reports, as
  `LOOP`'s does.

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
| `RANDOM` | `( n -- r )` | `r` in `0..n`, from the clock hash below |

## `RANDOM`

Not a generator with a seed: a hash of the clock. The handler (`0x65a4f`)
hands the count on the stack to `0x24219`, which keeps one counter
(`0xE7F4C`, 0 at start) and reads one clock — the raw tick count behind the
pointer at `0xE7F38`, the same count the timers stamp
([game loop](../engine/game-loop.md#where-the-200-hz-comes-from-and-where-it-stops)):

```text
s = s + 331;  if s = 0 then s = 1
r = ((t + s) / s)  xor  ((t − s) mod s)      unsigned, 32 bits
RANDOM = r mod n                              unsigned
```

Two calls in the same tick answer differently because `s` moved, and the
same call a tick later answers differently because `t` did. The divisions
are unsigned: a count of zero would fault the machine, and a negative count
is a divisor above two billion, which leaves `r` as it is. No shipped
module asks either — all 237 sites push a positive literal, 4 at 124 of
them. The 16-bit engine carries the same generator to the word, on 16-bit
values, over its millisecond counter divided by five
([16-bit kernel words](../../motion16/vm/kernel-words.md#random)). motionvm
runs this arithmetic with the master counter as its frames have advanced it
for `t`; what that is a departure from, and why, is on the
[departures](../../departures.md#the-virtual-machine) page.

## Open questions


## See also

- [Threaded code](threaded-code.md) — how these words are encoded
- [Execution model](execution-model.md) — stacks and addressing
- [Kernel words](kernel-words.md) — the full word inventory
- [Departures](../../departures.md) — `_PutStringAdr` when walking a body
