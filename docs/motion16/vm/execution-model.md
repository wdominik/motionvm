[← Documentation index](../../README.md)

# The Virtual Machine — Execution Model

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway and in `BMZ.EXE` with Hilfe für Amajambere, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit-ds2).*

The 16-bit engine runs the same threaded-code design as the 32-bit one — a
cell is either a kernel word or a call, literals and branches carry an
operand in the next cell, `VAR` and `CONST` are complete word behaviors —
but it is a different *machine*: 16-bit cells, one flat address space, word
ids instead of packed addresses, and a kernel word as the return.

## State

| State | Width |
|---|---|
| Cell | 16 bits |
| Data stack | 16-bit cells; the scripts do signed arithmetic on them |
| Return stack | return addresses and whatever `>R` pushes |
| Instruction pointer | a cell address in the flat memory |
| Word table | `table[word id] → address of the word's body`, filled by `=>GET`, cleared by `=>ERASE` |

There is **one** address space, not `module:offset`. After `=>GET` a word id
alone finds the body. That is what the modules demand: sixteen macro modules
each define id 549, and `INCLLOC` runs whichever one it has just loaded with
`LOCINIT EXECUTE` (`LOCINIT` is `CONST 549`) — see
[Script modules](../formats/script-modules.md).

**The arena and the word table**, read from `=>GET` (file `0x176ac`) and
`=>ERASE` (file `0x1690a`): the scripts live in one far segment — the
*arena*, its base at `DS:0x8154`, its top in cells at `DS:0x8168` — and the
word table at `DS:0x815c` holds, per word id, the **absolute arena cell** of
the word's body, 0 when unbound. A script address — what `_PutAdr` pushes,
what `@` and `!` take — is a byte offset into that arena. `=>GET` reads the
module out of the container, copies its 0x22-byte header into a slot table
at `DS:0x7eac` (count at `DS:0x11c6`; the slot keeps the id range, the
module number at +0x20 and a zero-terminated list of the modules it
requires, each of which must be resident or the load fails with error 10),
copies the module's relative offsets into the word table, adds the arena
top to each, and **appends the body at the top**. `=>ERASE` takes a module
out by moving everything above it down, fixing the word table and the
interpreter's own pointer — nothing else: an address a script holds in a
variable is not fixed — and clears every id of the module's range. A module
that is resident when `=>GET` names it again is erased first. The error
codes are 6 (no such module), 7 (erasing the running module), 9 (a bad
item) and 10 (a requirement missing). How the rebuild places modules
differs, and why it cannot be seen: [departures](../../departures.md#the-16-bit-machine).

## The step rule

Fetch one cell `c` at the instruction pointer, advance by one cell, then:

| Cell | Meaning |
|---|---|
| `c & 0x8000` set | **Kernel word** with ordinal `c & 0x7fff`, 1-based. Call its handler. Some handlers read one or more cells that follow ([threaded code](threaded-code.md)) |
| otherwise | **Call** the word with global id `c`: push the address of the next cell on the return stack and continue at `table[c]` |

**Return is a kernel word**: `##`, ordinal 1, cell `0x8001`. The 32-bit
engine's rule that a zero cell returns does not apply. The two words that
push and return, `_PutAdr` (ordinal 37) and `_PutConst` (38), return on
their own; the cell after them is data.

The value `0xffff` occurs in data — route tables, the font reference table,
`-1` literals — and is not a kernel word (ordinal 32767 does not exist).

## Memory words

`@` and `!` take **byte addresses**, and `_PutAdr` pushes the byte address
of the cell after it; `GET` copies a block to a byte address. The scripts do
address arithmetic constantly (`ARR n + @`), so the byte reading is what the
modules require. The two handlers differ on alignment: `!` (file `0x15f63`)
shifts bit 0 out of the address before adding the arena base, `@` (file
`0x1c9af`) reads at the address as given. The kernel's own words — the
inventory, the walk, the order machine — turn a script address into a
pointer through one helper (file `0x17690`) that masks bit 0 the way `!`
does.

## Word ids as callbacks

Where the 32-bit engine passes a packed address, this one passes a **word
id**: `400 SCRCTRL` installs `CTRL` (id 400) as the frame handler, `1086
SCRCTRL` installs `ICTRL`, and `NEWSETDESC`'s sixth argument is a word id or
−1. `EXECUTE ( id -- )` runs an id.

## Blocking words

`ANIMPLAY` runs the frame loop inside its handler and does not return until
the game is over: the boot word calls it once, and everything after it is
teardown ([boot and frame loop](../engine/boot-and-loop.md)). `FADEIN` and
`FADEOUT` also hold it: both spin their ring loops inside the handler
([descriptors and screens](../engine/descriptors.md#transitions)).

## Interpreter and handlers

The first reading of the engine places the interpreter loop at file offset
`0x174f7`, with a nested-run sentinel `0xfffd`, and the handler of `##` at
`12c8:10e2` (file `0x16f62`), `_PutLit` at `1977:00fd` (file `0x1ca6d`),
`=>GET` at `1400:04ac` (file `0x176ac`). The handler addresses are what the
kernel table holds ([kernel words](kernel-words.md)); the loop itself and
the sentinel are **unread** since that first pass.

## Open questions

- The interpreter loop's bytes at `0x174f7`; the nested-run sentinel
  `0xfffd`.
- What `C@`/`C!` — absent from this kernel — are replaced by.
- Which words besides `ANIMPLAY`, `->SCRX` and `->SCRY` block — the two
  scrolls run frames of their own ([descriptors](../engine/descriptors.md)).

## See also

- [Threaded code](threaded-code.md) — cell encoding, ordinals, operands
- [Kernel words](kernel-words.md) — the 233 words and their handlers
- [Execution model (MOTION 32-bit)](../../motion32/vm/execution-model.md) — the 32-bit machine for comparison
