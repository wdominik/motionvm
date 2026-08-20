[← Documentation index](../README.md)

# The Virtual Machine — Execution Model

The game runs on a threaded-code Forth interpreter built into the engine.
Compiled script modules ([format](../formats/script-modules.md)) contain
word bodies made of 32-bit cells; the interpreter walks them cell by cell.

## Interpreter state

| State | Description |
|---|---|
| Instruction pointer | The address of the next cell to execute |
| Data stack | 32-bit signed cells; all values, flags, and addresses live here |
| Return stack | Return addresses of calls; also used by `>R` / `R>` |
| Loop stack | One `{index, limit}` frame per active `DO` loop |

Because the entire position — instruction pointer, return stack, loop
stack — is explicit state, execution can be suspended and resumed at any
cell boundary. The engine relies on this (see
[Blocking words](#blocking-words) below).

## The step rule

Each step fetches one cell at the instruction pointer, advances the pointer,
and dispatches on the value:

| Cell | Meaning |
|---|---|
| `0x00000000` | **Return.** Pop the return stack; if it is empty, the top-level call is done. Compiled `;` produces this cell, and it also occurs mid-body as an early return |
| `0x4000xxxx` | **Kernel word.** The low 16 bits are the ordinal of a built-in word (see [Threaded code](threaded-code.md)). Some kernel words consume inline operands that follow the cell |
| anything else | **Call.** The cell is `(module << 16) | cell offset`; push the current position on the return stack and jump |

A call to module 0, offset 0 renders as `EXIT` — module 0 is the kernel's
base module and offset 0 there is the return that ends every definition.

## The address model

A single 32-bit cell addresses all script memory:

```
address = (module << 16) | offset      // offset in BYTES from file offset 0x30
```

- The module number occupies the upper 16 bits (interpreted as signed).
- The offset counts **bytes from file offset `0x30`** of the module image
  and is capped at 64 KB.
- `@` (fetch) truncates the offset to a cell boundary (`offset & ~3`).

### Two units in the same format

The packed `(module << 16) | offset` form means **two different things**
depending on where it appears:

| Use | Unit |
|---|---|
| Call cell in threaded code | **cells** from `0x30` |
| Data address (`@`, `!`, tables, computed addresses) | **bytes** from `0x30` |

Both count from the same base, file offset `0x30`. The distinction is
load-bearing: for a plain variable both readings hit the same cell, but the
moment code does arithmetic on an address — which the game does constantly —
they diverge. Three telling cases:

- `ARR 4 + !` writes the *next* cell after `ARR`, so `+ 4` moved one cell:
  bytes.
- The location table entry `0x012d0030` only points at a word body if
  `0x30` is read as a byte offset.
- Sibling words with bodies 28 bytes apart call each other with offsets 7
  apart: cells.

Branch distances of the control-flow words also count cells (see
[Threaded code](threaded-code.md)).

### How the engine decodes an address

The handler of `@` decodes a data address as follows (addresses are
locations in the engine's relocated image):

```
module = address >> 16                     // arithmetic shift, signed
slot   = [0xEE6C0 + 4*module]              // module number -> module slot
base   = [0xEE6D0 + 0x30*slot + 0x14]      // slot -> module memory pointer
value  = [base + (address & 0xFFFF & ~3)]
```

A consequence worth stating explicitly: script code **cannot address memory
outside the module table**. `@` and `!` can only ever reach loaded module
images; there is no escape hatch into engine memory.

## Module memory

- A module's addressable memory is its **entire image from `0x30`
  onward** — not just the dictionary. Modules carry data past the dictionary
  (dialogue tables, location data), and code reaches it with ordinary
  addresses.
- Module memory is **code and data at once**. Variables compiled with `VAR`
  store their value in a cell inside the word body itself; resource blocks
  loaded with `GET` are copied into module memory and then read back with
  `@`/`C@`. A stray store does not fault — it rewrites code.
- `C@`/`C!` provide true byte granularity within cells.
- **Module 0** is the kernel's base module. It is created at runtime and is
  not a file; roughly 2600 call cells across the game modules point into it
  and remain unresolved (open question).

### `@` never checks

The handler at `0x626fc` computes
`[[[0xEE6D0 + [[0xEE6C0] + (v>>16)·4]·0x30] + 0x14] + ((v & 0xffff) & ~3)]`
and reads — no validation of any kind. Scripts rely on it: location 5's macro
tests `KRSCHRZIEHE @ 0 =` — `KRSCHRZIEHER`, stored truncated to eleven bytes
(see [Script modules](../formats/script-modules.md)). But that word is the
*item number* 24 from module 11, not the flag `_?KRSCHRZIEHER` the author
meant, so the read lands on address 24 — module 0. Location 6 reads through
Gaby's not-yet-built shadow record the same way. Refusing to answer stopped
five of the game's sixteen locations, which is how much of the game rests on
the handler asking no questions. What motionvm answers instead is a
[departure](../departures.md).

## Byte-exact access to packed records

Some native subsystems keep records at strides that are **not divisible
by four** — a conversation's answers are 0x12 bytes apart, its branches
0x22 (see [Dialogue machine](../engine/dialogue-machine.md)). The
engine resolves only the *base* of such a record to a cell boundary and
then indexes byte-wise. Cell-granular reads that truncate every access
to a cell boundary silently return neighboring bytes for every second
entry; any reimplementation must read these structures byte-exact.

## Reentrancy

Native kernel handlers **re-enter the interpreter** from inside
themselves: the pointer-info word calls script words eight times, the
click dispatcher twenty-two times, mid-handler included. Each nested
run gets its own return stack while the **data stack stays shared** —
arguments and results pass over it, and a callback that is the last act
of a branch is exactly a tail call. Nested runs never treat blocking
words as suspension points: suspension models a blocking word in the
*outer* instruction stream, while a nested call runs to completion and
returns into the handler.

Callback addresses handed to the engine (completion words, controller)
are **validated before storing**: the module (bits 16…29) must be
loaded and the offset must lie inside it; a third check inspects the
word header behind the address. Invalid values are dropped silently —
necessary, because scripts do pass plain numbers where addresses are
expected.

## Blocking words

Four kernel words run their own timing loop **inside** the word; the
bytecode does not continue until they finish:

| Word | Purpose |
|---|---|
| `ANIMPLAY` | Play an animation |
| `ANIMSIM` | Play an animation (variant) |
| `FADEOUT` | Close the screen curtain |
| `FADEIN` | Open the screen curtain |

This is not incidental. One intro phase calls `FADEOUT`, swaps the
background, calls `SETPAL 91` and `FADEIN` — all in one word invocation.
While `FADEOUT`'s curtain is running, the cells after it have **not**
executed: the descriptors are unchanged and the palette swap has not
happened. The old picture, in the old palette, is what the curtain closes
over — no snapshot or saved palette is involved, because the state that
would need saving simply has not changed yet.

Any reimplementation must therefore be able to stop the interpreter *mid
word* — after the blocking word, before the next cell — and resume later.
The explicit interpreter state (see above) is what makes that possible.

## Open questions

- Contents and layout of module 0; until it is reconstructed, ~2600 call
  targets stay unresolved.
- Whether any word besides the four listed blocks in the same way.

## See also

- [Threaded code](threaded-code.md) — cell encoding, ordinals, operands
- [Word semantics](word-semantics.md) — behavior of the core words
- [Script modules](../formats/script-modules.md) — the on-disk format
- [Transitions](../engine/transitions.md) — the curtain in detail
- [Departures](../departures.md) — what an unresolved read answers
