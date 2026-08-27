[← Documentation index](../../README.md)

# Script Modules

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein and in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, which is an older build of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names the other game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit-ds2).*

A SCR item is one compiled Forth module: a short header, a table of body
offsets, and a dictionary in which each word's 16-byte header sits
immediately before its body. The module number is the SCR slot minus 3500.
There is no magic string — the 32-bit engine's `USERDEF` header, its `DP`
and `LAST` fields and its packed-address call cells do not exist here.

All values little-endian; a cell is 16 bits.

## Layout

```
u16 firstID            ; the lowest global word id defined in this module
u16 lastID             ; the highest
u16 nwords             ; how many words
u16 firstID, lastID, nwords   ; the same three again
u8  zeros[20]
u16 moduleNumber       ; at 0x20, equals the slot minus 3500
u16 bodyOffset[nwords] ; in CELLS from the start of the dictionary
; the dictionary starts at 0x22 + 2 * nwords; for each word, in order:
u8  namelen            ; the name's original length — may exceed 11
char name[11]          ; the first eleven characters, NUL-padded
u16 globalWordID
u16 link
u16 body[]             ; threaded code, or a VAR/CONST data cell (plus ALLOT cells)
```

`bodyOffset[i]` points at the **body**; the header is the 16 bytes before
it. The last word's body runs to the end of the item.

Measured over all 65 ENVIRO modules: the repeated triple matches the first,
the 20 bytes are zero, `moduleNumber` equals the slot, the first word's id
is `firstID`, the last word's is `lastID`, and there are `nwords` words.

## Words

A body's first cell says what kind of word it is (see
[Threaded code](../vm/threaded-code.md)):

| First cell | Kind | In ENVIRO |
|---|---|---:|
| `0x8025` (`_PutAdr`) | `VAR` — the data cell follows, then any `ALLOT` cells | 792 |
| `0x8026` (`_PutConst`) | `CONST` — the value follows | 310 |
| anything else | a colon definition — threaded code ending in `##` | 672 |

1774 words in all. A VAR's extra cells are data and are never fetched as
code, because `_PutAdr` returns; a disassembler that walks past it prints
nonsense.

Names are capped at eleven characters, as in the 32-bit engine, and the
length byte keeps the original count: 151 of ENVIRO's names are longer than
their stored eleven characters (`BIRKENSTRAß`, `LD_7MÜLLMAN`, `MYCALCEXAMI`).
Names are CP437 and carry umlauts.

## Word ids are global and not unique

A word id is a 16-bit number in one game-wide space — 400 to 1944 in ENVIRO
— and a call cell in threaded code holds nothing but that id (see
[Execution model](../vm/execution-model.md)). Ids are allocated per module at
authoring time, and modules that are never loaded together reuse them:

- every **location macro** module 301–315 and 317 defines exactly one
  word, and it has id **549** in all sixteen; the loader runs it through
  `LOCINIT`, which is `CONST 549`, after `=>GET` of the module;
- every **scene** module 101–115 and 117 starts its ids at 560;
- every **click-handler** module 501–515 and 517 starts at 750.

111 ids are defined in more than one module, the busiest in sixteen. So
`=>GET` has to bind a module's ids when it loads it — `table[id] = address of
the body` — and `=>ERASE` has to free them; which module's word an id names
depends on what is resident.

## What is occupied

65 of the 700 SCR slots hold a module; the other 635 are empty (their
offset equals the next slot's). The occupied numbers:

```
100   101–115 117   301–315 317   501–515 517
600–607 609–612 614–615   650 651
```

Gaps 116, 316, 516, 608 and 613 are real — location 16 does not exist, and
nothing loads 608 or 613. What each module holds is in the
[module map](../../games/enviro/module-map.md).

## Open questions

- The meaning of `link` in the word header; in every module it is read but
  not yet correlated with anything.
- The placement of modules in the engine's 64 KiB address space at load
  time — see [Execution model](../vm/execution-model.md).

## See also

- [Threaded code](../vm/threaded-code.md) — what the body cells mean
- [Execution model](../vm/execution-model.md) — how ids resolve at run time
- [Module map](../../games/enviro/module-map.md) — ENVIRO's 65 modules
- [Script modules (MOTION 32-bit)](../../motion32/formats/script-modules.md) — the 32-bit layout
