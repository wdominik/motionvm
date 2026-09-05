[← Documentation index](../../README.md)

# BMZ.EXE — The Third Build of the MOTION 16-bit Player

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

`BMZ.EXE` (166 806 bytes) is the same player as
[`ENVIRO.EXE`](enviro-exe.md), built earlier — and later than
[`HPPLAY.EXE`](hpplay-exe.md), which puts it third of the four.
Everything the `ENVIRO.EXE` page says about the binary holds here — Turbo C real
mode, an 800-paragraph header, two kernel tables of far function pointers, a
data stack behind a far pointer in DGROUP — and this page is only what differs.
Where a subsystem page cites an `ENVIRO.EXE` address, the same code is in this
image at its own address; the four are not interchangeable.

The name is the client's: the game was made for the Bundesministerium für
wirtschaftliche Zusammenarbeit und Entwicklung.

## The binary

| Field | `BMZ.EXE` | `ENVIRO.EXE` | `HPPLAY.EXE` |
|---|---|---|---|
| Size | 166 806 | 167 430 | 165 702 |
| Header | `0x3200`, load image 154 006 bytes | `0x3200`, 154 630 | `0x3200`, 152 902 |
| Relocations | 3162 | 3160 | 3157 |
| Entry | `CS:IP = 0000:0000`, `SS:SP = 258b:00e6` | `SS:SP = 25b2:00e6` | `SS:SP = 2546:00e6` |
| Data segment | `0x1c03` | `0x1c24` | `0x1bce` |
| Turbo C banner | file `0x1f234` | file `0x1f444` | file `0x1eee4` |
| Memory asked for | 1 MB EMS, 550 000 bytes base | 1 MB EMS, 550 000 | 2 MB EMS, 590 000 |

The dates are the build's own and not a re-stamp: 1995-06-05 for the binary,
1995-06-06 for the containers, against Jeff Jet's blanket 1998-04-10. This is
the oldest-dated of the three later builds — only `LL.EXE`, two years earlier,
predates it — and the memory it asks for is
`ENVIRO.EXE`'s — Jeff Jet needs twice the expanded memory because its container
is packed, and this game's is not.

Its prompts are formal *Sie* (`passen Sie`, file `0x1f4b1`), as `HPPLAY.EXE`'s
are; the later `ENVIRO.EXE` was edited to *Du*.

## Where this build sits between the other two

Its kernel table is `ENVIRO.EXE`'s **less exactly one word**: 232 against 233,
with `?SAMPLE` — the last, ordinal 255 — missing and nothing else changed. And
it is `HPPLAY.EXE`'s **plus four**: the `SETMOUSE*` group that build lacks.

| Build | Domain words | Core words | Total |
|---|---:|---:|---:|
| `HPPLAY.EXE` | 146 | 82 | 228 |
| `BMZ.EXE` | 150 | 82 | 232 |
| `ENVIRO.EXE` | 151 | 82 | 233 |

That is the order they were built in, and the argument is the one the
[`HPPLAY.EXE` page](hpplay-exe.md) makes: a word is appended to a live ordinal
space, not inserted into the middle of one, so the build that has a mid-table
word is the later one. `SETMOUSEX`–`SETMOUSERB` sit at 124–127, inside the
domain table: `HPPLAY.EXE` does not have them, this build does, so this build is
later. `?SAMPLE` is appended after 254: this build does not have it,
`ENVIRO.EXE` does, so `ENVIRO.EXE` is later still.

The consequence for the ordinals is worth stating plainly, because it is the
opposite of Jeff Jet's. **Nothing is shifted here.** Every ordinal from 105 to
254 names in this build what it names in `ENVIRO.EXE` — `DOWALK` is 243 in
both, where `HPPLAY.EXE` has it at 239 — and only 255 is absent.

Like `HPPLAY.EXE`, this build still carries the interpreter's error messages
that `ENVIRO.EXE` has stripped: fifteen of them here, as far pointers at file
`0x203ec` into strings from `0x2043b` — *"Line #i: Unterminated Comment!"*,
*"Line #i: Reference-Block not available"*, *"Line #i: Illegale Opcode ( #i )"*,
*"Line #i: Fehler diverser Natur (FDN)"* and eleven more. They speak of source
lines and code blocks, which is compiler vocabulary in a player that has no
compiler.

## Why the table must be read from the game's own binary

The shift `HPPLAY.EXE` shows is the loud case. This build is the quiet one: a
table baked from `ENVIRO.EXE` would bind this game's bytecode and name every
word of it *correctly*, right up to ordinal 255 — which this build does not have
and this game never calls. The mistake would cost nothing until a game did call
it, and then it would call a word that is not there.

Nothing in the format announces either case, so the rule is the same for all
three: scan the table out of the binary that ships with the game.

## What lives where

| File offset | What |
|---|---|
| `0x1f7e6` | Kernel table 2 — 150 domain words, ordinals 105–254 ([kernel words](../vm/kernel-words.md)) |
| `0x2067a` | Kernel table 1 — 82 core words, ordinals 1–82, name for name and order for order the other builds' |
| `0x203ec` | The fifteen interpreter error messages, by far pointer into `0x2043b` and on |
| `0x16ba7` | `GET` (`12bb:0df7`), the block reader — and at `12bb:0e91` the null test that answers a missing block with error `0xE`, *Fehler diverser Natur* |

## What the game asks of it

Hilfe für Amajambere's bytecode uses **143** of the 232 words. Against the union
of what Die Enviro-Kids greifen ein and Jeff Jet use it calls two more — `&`
and `GFXVFLIP` — and both
were implemented already. `PLAYSAMPLE` and `XGFXSAMPLE` are in the table and
called zero times, as in both sibling games; `?SAMPLE`, which `ENVIRO.EXE` added
after this build, does not exist here to be called.

## Open questions

- **Why the hole case was added.** This build passes an all-zero hot area
  over where `HPPLAY.EXE` does not, and Jeff Jet's is the later date; what
  the tables looked like that made it worth adding is not established
  ([ledger](../../open-questions.md#motion-16-bit)).
- **The handlers not read.** Read only where it differs from `ENVIRO.EXE`
  — one word fewer, the error messages it still carries — so the descriptor
  drawer, the fades and the sound interface stand on that binary's reading
  ([ENVIRO.EXE](enviro-exe.md#open-questions)).
- **Hilfe für Amajambere's location 7**, whose item table is not in the
  container: `GET`'s null test at `12bb:0e91` and its error `0xE` are read;
  whether the room was cut late or its table lost in mastering is the
  game's question ([ledger](../../open-questions.md#motion-16-bit)).

## See also

- [ENVIRO.EXE](enviro-exe.md) — the latest build, and everything the four share
- [HPPLAY.EXE](hpplay-exe.md) — the earlier build, whose ordinals are shifted
- [LL.EXE](ll-exe.md) — the oldest build, whose domain table binds at 102
- [Kernel words](../vm/kernel-words.md) — the two tables, entry by entry
- [Other files (Hilfe für Amajambere)](../../games/hfa/other-files.md) — what else the installation holds
