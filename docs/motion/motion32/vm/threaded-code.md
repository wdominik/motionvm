[← Documentation index](../../README.md)

# Threaded Code — Cell Encoding

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2 and V0.04.15/R78 with Checker 2000; what is measured here is measured on those games' files, and an address is R109's unless the page says otherwise. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Word bodies are sequences of 32-bit cells. This page describes how the cells
encode kernel words, calls, literals, and control flow. The interpreter's
step rule is described in [Execution model](execution-model.md).

## Cell classification

| Cell | Meaning |
|---|---|
| `0x4000xxxx` | Kernel word; `xxxx` is the ordinal |
| `0x00000000` | Return |
| anything else | Call: `(module << 16) | cell offset` |

Inline operands (below) are raw data cells and must be consumed together
with the word that owns them — a decoder that ignores them will read
operands as code and string bytes as word headers. Note also that a small
integer literal is bit-identical to a call into module 0 at a small offset;
only position (after `_PutLit` etc.) distinguishes them.

### A call cell's offset counts cells; a data address counts bytes

The two halves of a cell are `(module << 16) | offset`, and what `offset`
means depends on which kind of address it is:

| Written | Means | Byte position in the module |
|---|---|---|
| a **call** cell `<202:0x1645>` | cell `0x1645` | `0x1645 · 4 + 0x30` |
| a **data** address `<2:0x28ac>` | byte `0x28ac` | `0x28ac + 0x30` |

Both are `(module << 16) | offset` and neither says which it is; only where
the value came from does. A call cell read as a byte address — or a variable's
address read as a cell index — points somewhere real, four times off, with
nothing to signal it. The mistake shows up as a call into the middle of an
unrelated word, or as a variable that reads plausible garbage.

See [The address model](execution-model.md#the-address-model) for the byte
form.

## The ordinal system

The low 16 bits of a kernel cell are **not a table index**. They are an
offset into the kernel base module's dictionary, where every entry occupies
5 bytes:

```
ordinal = 5 * index + base(table)
```

The kernel's words live in three tables inside `ENGINE.EXE` (see
[Kernel words](kernel-words.md)). The measured bases:

| Table | Contents | Base |
|---|---|---|
| 0 | Core Forth and compiler runtimes | 104 |
| 1 | Compiling words (`:`, `IF`, `DO`, …) | 634 |
| 2 | Domain words (`NEWSCREEN`, `SDIAL`, `ANIMPLAY`, …) | 1039 |

Table 1's base is never seen in a cell — its words run at compile time —
and is read out of the init like the other two: every registered word takes
five bytes of the dictionary and its ordinal is the dictionary pointer plus
four, so the bases are counts. Table 0 begins at 104; `_FNAME`, registered
by itself, is 609; a gap of twenty puts table 1 at 634; the shell's 54 words
follow one by one, `TEST` at 769 and `PROGINFO` at 1034; and table 2 begins
at 1039. Checker 2000's build counts to 104, 559, 584, 719 and 934 the same
way ([ENGINE.EXE R78](../engine/engine-r78.md#the-kernel)). Anchor values
confirming the formula:

| Word | Table, index | Ordinal |
|---|---|---|
| `DUP` | 0, 7 | 139 |
| `DROP` | 0, 10 | 154 |
| `_PutLit` | 0, 14 | 174 |
| `_PutAdr` | 0, 15 | 179 |
| `_PutConst` | 0, 16 | 184 |
| `TOGFX` | 2, 0 | 1039 |
| `NEWSCREEN` | 2, 21 | 1144 |

## Words with inline operands

These words consume the cell(s) following them. Fourteen ordinals are
known to take operands:

| Word | Ordinal | Compiled from | Operand |
|---|---|---|---|
| `_PutLit` | 174 | a literal (`5`) | one cell (the value) |
| `_PutAdr` | 179 | `VAR X` | one cell (the variable's storage) |
| `_PutConst` | 184 | `5 CONST Y` | one cell (the constant's value) |
| `_CheckIf` | 314 | `IF` | one cell (forward distance) |
| `_CheckEIf` | 319 | `=IF` | one cell (forward distance) |
| `_CheckElse` | 329 | `ELSE` | one cell (forward distance) |
| `_AddLoop` | 359 | `+LOOP` | one cell (backward distance) |
| `_ULoopEnd` | 364 | `/LOOP` | one cell (backward distance) |
| `_Until` | 374 | `UNTIL` | one cell (backward distance) |
| `_LoopBreak` | 379 | `WHILE` | one cell (forward distance) |
| `_Repeat` | 384 | `REPEAT` | one cell (backward distance) |
| `_LoopEnd` | 394 | `LOOP` | one cell (backward distance) |
| `_PutString` | 499 | `." …"` | NUL-terminated string, padded to 4 bytes; **no game module uses it** |
| `_PutStringAdr` | 504 | — | the same operand; pushes its address and continues. The engine steps `(strlen + 3) >> 2` cells, one less than the compiler laid down when the length divides by four (see [Word semantics](word-semantics.md)) |

`_LoopStart` (compiled from `DO`) takes **no** operand — the one member of
the loop family that doesn't, which is why guessing by word family fails.

So `VAR X` compiles to `_PutAdr` plus a data cell, and `5 CONST Y` to
`_PutConst 5` — exactly the pattern found at the inventory module's `STIFT`
("pen") and `ZETTEL` ("note") definitions.

## Branch distances

Branch operands count **cells**, and the base of the jump is **the
operand's own position**:

```
forward:   target = operand_position + distance
backward:  target = operand_position - distance
```

Forward-branching words: `_CheckIf`, `_CheckEIf`, `_CheckElse`,
`_LoopBreak`. Backward-branching words: `_Until`, `_Repeat`, `_LoopEnd`,
`_AddLoop`, `_ULoopEnd`.

The compiler's emissions for each construct:

| Source | Emission |
|---|---|
| `: T 1 IF 2 ELSE 3 ENDIF ;` | `_CheckIf 5 … _CheckElse 3` |
| `: T 1 =IF 2 ENDIF ;` | `_CheckEIf 3` |
| `: T BEGIN 1 UNTIL ;` | `_Until 3` |
| `: T BEGIN 1 WHILE 2 REPEAT ;` | `_LoopBreak 5 … _Repeat 7` |
| `: T 0 10 DO 1 LOOP ;` | `_LoopStart` (no operand) `… _LoopEnd 3` |
| `: T 0 10 DO 1 2 +LOOP ;` | `_AddLoop 5` |
| `: T 0 10 DO 1 2 /LOOP ;` | `_ULoopEnd 5` |

## Open questions

- `_LoopStart`'s ordinal: arithmetic suggests 389 (table 0, the only gap
  between `_Repeat` = 384 and `_LoopEnd` = 394), but this has not been
  confirmed.
- `_ChElseDup` takes an unknown operand shape; it does not occur anywhere
  in the game's modules.

## See also

- [Execution model](execution-model.md)
- [Word semantics](word-semantics.md)
- [Kernel words](kernel-words.md) — the tables the ordinals index
- [Script modules](../formats/script-modules.md)
