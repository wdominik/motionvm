[← Documentation index](../../README.md)

# Threaded Code — Cell Encoding

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

A word body is a sequence of 16-bit cells. Each cell is one of:

| Cell | Meaning |
|---|---|
| `0x8000 \| ordinal` | A kernel word. Ordinals are 1-based; 1–82 are the core table, 105–255 the domain table, 83–104 do not occur ([kernel words](kernel-words.md)) |
| any other value | A call to the word with that global id (400–1944 in Die Enviro-Kids greifen ein) |
| the cell after certain kernel words | That word's operand — a literal, a branch distance, a data cell, or the bytes of a string |

The 32-bit engine's `0x4000xxxx` tag, its five-per-index ordinals and its
zero-cell return have no counterpart here; look kernel words up **by name**
when comparing the two, never by number.

## Inline operands

Measured by decoding all 65 modules of Die Enviro-Kids greifen ein with exactly
this set — the walk
meets no unknown ordinal and every body ends where the next word's header
begins:

| Word | Ordinal | Operand |
|---|---:|---|
| `_PutLit` | 80 | one cell, the signed literal; pushes it and continues |
| `_PutAdr` | 37 | one cell of **storage**: pushes that cell's byte address and **returns** — a `VAR`; `ALLOT` cells follow as data |
| `_PutConst` | 38 | one cell: pushes its value and **returns** — a `CONST` |
| `_CheckIf` | 41 | signed forward distance (`IF`) |
| `_CheckEIf` | 42 | signed forward distance (`=IF`) |
| `_ChElseDup` | 43 | forward; in the kernel, unused by Die Enviro-Kids greifen ein |
| `_CheckElse` | 44 | signed forward distance (`ELSE`) |
| `_LoopBreak` | 51 | signed forward distance (`WHILE`) |
| `_Until` | 50 | signed backward distance |
| `_Repeat` | 52 | signed backward distance |
| `_LoopEnd` | 40 | signed backward distance (`LOOP`) |
| `_AddLoop` | 47 | backward (`+LOOP`); unused by Die Enviro-Kids greifen ein |
| `_ULoopEnd` | 48 | backward (`/LOOP`); unused by Die Enviro-Kids greifen ein |
| `_PutString` | 78 | a NUL-terminated CP437 string padded to a cell boundary; 2 sites |
| `_PutStringAdr` | 81 | the same payload; pushes the string's address and continues; 162 sites — `LL.EXE`'s kernel does not have the word at all |

`_LoopStart` (39, `DO`) takes no operand.

The string payload occupies `(len + 2) / 2` cells counted from the cell after
the opcode — the terminator is included and the length rounded up to a cell,
so a string of even length costs one cell more than its characters. This is
the compiler's rule; whether the 16-bit `_PutStringAdr` handler skips the
same number of cells (the 32-bit handler skips one fewer when the length
divides by four, see
[threaded code (MOTION 32-bit)](../../motion32/vm/threaded-code.md)) is
unread.

## Branches

A branch's target is **the operand cell's own index plus or minus the
distance** — the same rule as in the 32-bit engine:

```
forward:   target = index_of_operand + distance
backward:  target = index_of_operand − distance
```

Measured: `CTRL` (module 100) opens `?KEY DUP _CheckIf 95 …` with the
operand at cell 3, and cell 98 is where the `ELSE` branch of that test
lands.

`DO … LOOP` compiles to `_LoopStart` (no operand) and `_LoopEnd` with a
backward distance; the loop index is `I`. `DO` takes `limit index` in that
order: the save-slot probe in `RUN` is `706 701 DO I =>EXIST … LOOP`. Both
cells go onto the **return stack**, index over limit — see the state table
in [Execution model](execution-model.md#state) for the handlers and for what
`I'`, `LEAVE` and Victor Loomes' `STOPLOOP` do with them.

## `VAR`, `CONST`, `ALLOT`

A variable is a two-cell word, `_PutAdr` followed by its value cell; a
constant is `_PutConst` followed by the value. Executing either pushes and
returns, so `ALLOT` storage after a variable's value cell is never fetched
as code:

```
_PutConst 549        \ LOCINIT
_PutAdr   13         \ STARTLOC — initial value 13; RUN stores 1 before use
_PutAdr   0  0 0 …   \ _LOCLINK — a VAR with 16 ALLOT cells
```

The modules of Die Enviro-Kids greifen ein define 792 variables and 310 constants
this way. Module
600 defines the small integers as constants — 0 to 20, the even numbers to
30, −1, 100 and 1000 — which is why a decoded listing shows bare numbers
that are *calls*, not literals.

## Open questions

- Whether `_PutStringAdr`'s handler follows the compiler's padding rule.
- `_ChElseDup`'s exact semantics (the `ELSEDUP` runtime); no site in
  Die Enviro-Kids greifen ein reaches it.

## See also

- [Execution model](execution-model.md) — the step rule and the word table
- [Kernel words](kernel-words.md) — every ordinal
- [Script modules](../formats/script-modules.md) — where bodies live
