[← Documentation index](../../README.md)

# SCRIPT — Compiled Forth Modules

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit-enviro).*

The game logic is compiled Forth, stored — in Dunkle Schatten 2 — as 86 script modules ("scriptor
files") in `001.RSC`. The same format appears as standalone `.SCR` files on
disk (`002.SCR`, `011.SCR`). A module's resource id always equals its module
number.

This page describes the container and dictionary structure. The encoding of
compiled code inside word bodies is covered in
[Threaded code](../vm/threaded-code.md).

All multi-byte values are little-endian.

## Module layout

| Offset | Type | Description |
|---|---|---|
| `0x00` | `char[12]` | Magic `USERDEF\0#F0\0` |
| `0x10` | `u32` | Module number (equals the resource id) |
| `0x14` | `u32` | Address from the authoring tool's address space; meaningless at runtime |
| `0x18` | `u32` | Same |
| `0x1c` | `u32` | Size of the module-memory region — a **capacity**, not a used size, in the copies stored in the RSC container (`0xfff0` there) |
| `0x20` | `u32` | Size of the second region (16004 in every shipped module) |
| `0x24` | `u32` | **DP**: where module memory ends, in cells from `0x30` |
| `0x28` | `u32` | **LAST**: the dictionary's link field |
| `0x30` | `u32` | Entry point, packed as `(module << 16) | offset` |
| `0x50` | … | Dictionary |

**Two regions follow the header, not one.** `DP * 4` bytes of module
memory from `0x30` on — code, dictionary and data, everything an address
can reach — and then exactly 16004 further bytes that the engine
allocates as a separate block. `=>GET` (`0x64999`) allocates the first at
precisely `DP * 4` and then overwrites `0x1c` with that size, which is
why the container copies can carry `0xfff0` there: it is the authoring
tool's capacity, and the runtime never believes it.

Measured across all 86 shipped modules: `0x20` is 16004 in every one,
`0x1c` is `0xfff0` in every one, and `0x30 + DP*4 + 16004` never exceeds
the item. Items may carry padding past the second region — nothing in 60
of them, up to 4208 bytes in the other 26, zero except in module 10,
which has no words and keeps 4208 bytes of data there. So a reader must
use the item's own length, not the sum of the two regions.

Consequences for parsing:

- Sizes at `0x1c` are measured **from `0x30`**, not from the dictionary
  start. File offset `0x30` is the base for all addressing inside a module
  (see [Execution model](../vm/execution-model.md)).
- **Words run to `0x30 + DP*4`, not to `0x50 + 16004`.** Stopping at the
  latter sees only 2630 of the game's **3302 words** (module 202: 65 of
  106), because the second region is appended *after* module memory
  rather than carved out of it. Running *past* DP is the opposite error
  and just as real: it walks the second region as if it were threaded
  code, giving the last word of 38 modules a tail of cells that are not
  code — ten extra in module 2's `2OVER`, among them a call to `0:0x84`,
  and in module 101 an invented closing `EXIT` for the constant
  `LD_TASCHE`, which needs none (`_PutConst` at `0x62484` returns through
  `0x611df`, where `_PutLit` only steps the instruction pointer on).
- An empty module has `DP == 8` — `create_module` bumps it by eight on
  creation — so its memory ends at exactly `0x50` and the whole module is
  `0x50 + 16004 = 16084` bytes, which is what the original compiler emits
  for a source file with nothing in it. Three shipped modules define no
  words: 123 and 130 are exactly that size, and module 10 carries only
  data, past the second region.

## Word definitions

The dictionary at `0x50` is a sequence of word definitions, each with a
16-byte header followed by the body:

| Offset | Type | Description |
|---|---|---|
| `+0` | `u8` | Name length *before* truncation |
| `+1` | `u8[11]` | Name, truncated to 11 bytes, NUL-padded |
| `+12` | `u32` | Hash-chain link (see below) |
| `+16` | … | Body: a sequence of 32-bit cells |

### Name truncation

Names longer than 11 characters are stored truncated while the length byte
keeps the original count: `TELEFONKARTE` ("phone card") is stored as
`TELEFONKART` with a declared length of 12. Valid declared lengths are 1–32;
the stored bytes are printable (≥ `0x20`) with the remainder of the 11-byte
field zeroed.

### The link field is not a flag word

The `u32` at `+12` looks like a flag field at first — it is 11 on the first
word of a freshly compiled module and 6 on the following ones — but in
dictionaries that grew over many compilations it takes over a hundred
distinct values. It behaves like a **hash-chain link** and must not be used
to classify headers.

## The dictionary is not contiguous

A module's dictionary typically holds a first group of words, then padding,
then further groups. A parser that stops at the first non-header position
silently loses most of a module. The correct rule on encountering something
that is not a valid word header is: **skip 4 bytes and keep scanning**.

## Where a body ends

A zero cell in a body is a return — but an *early* return can occur in the
middle of a body, so a single zero cell does not terminate the walk. A body
ends where:

- the next valid word header begins, or
- a return is followed by at least **16 bytes (4 cells) of zeros** — real
  code never contains that many consecutive zero cells, padding does.

Walking a body also requires consuming inline operands as the interpreter
would ([Threaded code](../vm/threaded-code.md)): the string operand of
`_PutString` looks exactly like a counted name and would otherwise be
mistaken for the next word's header. String operands are NUL-terminated and
padded to the next 4-byte boundary (measured past the NUL).

## How words are addressed

A word is addressed from other code as `(module << 16) | offset` where the
offset counts **cells from file offset `0x30`** — not bytes, and not from
the dictionary start. Three sibling words with bodies at file offsets
`0x60`, `0x7c`, `0x98` (28 bytes apart) are called as offsets `0x0c`,
`0x13`, `0x1a` (7 cells apart).

Note that the *same* packed format means **byte** offsets when used as a
data address. This duality is load-bearing and is covered in
[Execution model](../vm/execution-model.md).

## Open questions

- What the 16004-byte second region holds. It is allocated separately at
  runtime (`0x64999`), contains no word headers in any shipped module,
  and nothing has been observed reading it.
- The exact hash function behind the link field at `+12`.
- The authoring-tool addresses at `0x14`/`0x18`.

## See also

- [Threaded code](../vm/threaded-code.md) — cell encoding inside bodies
- [Execution model](../vm/execution-model.md) — addressing and module memory
- [RSC containers](rsc-container.md)
