[← Documentation index](../../README.md)

# HPPLAY.EXE — The Third Build of the MOTION 16-bit Player

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere, in `STERN.EXE` with Falsches Spiel mit Eddie M. and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

`HPPLAY.EXE` (165 702 bytes) is the same player as
[`ENVIRO.EXE`](enviro-exe.md), built earlier — and earlier than
[`BMZ.EXE`](bmz-exe.md) too, which puts it third of the five, behind
[`LL.EXE`](ll-exe.md) and [`STERN.EXE`](stern-exe.md). Everything the other page says
about the binary holds here — Turbo C real mode, an 800-paragraph header, two
kernel tables of far function pointers, a data stack behind a far pointer in
DGROUP — and this page is only what differs. Where a subsystem page cites an
`ENVIRO.EXE` address, the same code is in this image at its own address; the
five are not interchangeable.

## The binary

| Field | `HPPLAY.EXE` | `ENVIRO.EXE` |
|---|---|---|
| Size | 165 702 | 167 430 |
| Header | `0x3200`, load image 152 902 bytes | `0x3200`, 154 630 bytes |
| Relocations | 3157 | 3160 |
| Entry | `CS:IP = 0000:0000`, `SS:SP = 2546:00e6` | `SS:SP = 25b2:00e6` |
| Data segment | `0x1bce` | `0x1c24` |
| Turbo C banner | file `0x1eee4` | file `0x1f444` |
| Memory asked for | 2 MB EMS, 590 000 bytes base | 1 MB EMS, 550 000 bytes base |

The file's date, 1998-04-10, is a re-stamp: every file of the installation
carries it, including the six `*.DRV` and `SOUND.EXE`, which are byte-identical
to the 1995-07-10 copies Die Enviro-Kids greifen ein ships. `HP.BAT` ends
`@echo (c) 1995 Hewlett Packard`.

Twice the expanded memory for a container a fifth the size, because it is
packed: 2 459 890 bytes of `DATA.-1-` and `DATA.-2-` unfold to 8 412 811 (see
[the DATA container](../formats/container.md)).

## Which build is older

Its kernel table is a **strict subset** of the other's: 228 words against 233,
with five deletions and no additions —

| Missing here | Ordinal in `ENVIRO.EXE` |
|---|---:|
| `SETMOUSEX` | 124 |
| `SETMOUSEY` | 125 |
| `SETMOUSELB` | 126 |
| `SETMOUSERB` | 127 |
| `?SAMPLE` | 255 |

Four of the five sit *inside* the domain table rather than after it. A word is
appended to a live ordinal space, not inserted into the middle of one, so the
build that has them is the later one — and the build without them is the one
whose ordinals shift.

The same 146 domain words, name for name, are `STERN.EXE`'s — and that
build's core table is `LL.EXE`'s eighty, without the two words this one
appends. So the domain table was finished before the core table grew, and
`STERN.EXE` is the build between `LL.EXE` and this one
([STERN.EXE](stern-exe.md)).

`HPPLAY.EXE` also still carries a table of eleven interpreter error messages
that `ENVIRO.EXE` has stripped: far pointers at file `0x1ffce` into strings at
`0x2013e` and around it — *"Line #i: Unterminated Comment!"*,
*"Line #i: Code-Block not available"*, *"Line #i: Illegale Opcode ( #i )"*,
*"Line #i: Fehler diverser Natur (FDN)"* and seven more. They speak of source
lines and code blocks, which is compiler vocabulary in a player that has no
compiler; the later build drops them.

## Why the table must be read from the game's own binary

**127 of the 146 domain words sit at a different ordinal here.** From 124 up,
every word is four below its namesake in `ENVIRO.EXE`: `DOWALK` is 239 here
and 243 there, `PLAYSAMPLE` 250 and 254. A table baked from one game and
applied to another would bind, and would then run the bytecode calling the
wrong words from ordinal 124 on.

Nothing in the format announces the shift, so the only safe rule is the one the
engine follows: scan the table out of the binary that ships with the game.

## What lives where

| File offset | What |
|---|---|
| `0x1f42e` | Kernel table 2 — 146 domain words, ordinals 105–250 ([kernel words](../vm/kernel-words.md)) |
| `0x20236` | Kernel table 1 — 82 core words, ordinals 1–82, name for name and order for order `ENVIRO.EXE`'s |
| `0x400c` | The container item loader: the per-segment packed flag, the volume bitmask, the read ([the DATA container](../formats/container.md)) |
| `0x1934d` | The GFXCRUNCH decoder (`1614:000d`) |
| `0x1951d` | The unpacked length of an item, from its header (`1614:01dd`) |
| `0x1ffce` | The eleven interpreter error messages, by far pointer |

## What the game asks of it

Jeff Jet's bytecode uses **150** of the 228 words. Against Die Enviro-Kids
greifen ein's usage the difference is three words in and two out: it calls `GDOX` and `GDOY`,
which that game does not, and does not call `-FONT`, `SDBLK` or `SDH%SHR`,
which it does. `PLAYSAMPLE` and `XGFXSAMPLE` are in the table and called zero
times — the game ships the four `DMA*.DRV` digital drivers and never reaches
them.

## The hole `?XINSIDE` does not skip

This build's `?XINSIDE` takes a hot area whose four corners are all zero as
a rectangle at the origin, where `ENVIRO.EXE` and `BMZ.EXE` pass over it —
71 instructions against their 101, with no `cmpw $0` in the handler at all.
`LL.EXE` and `STERN.EXE` side with this build: the split is three against
two, and it follows the order of the tables — the two later builds test, the
three earlier do not — but not the games' dates, Jeff Jet's being the youngest
by two years. It is the one behavior read out of each binary separately and
wired as its own capability rather than assumed from the generation.

## Open questions

- **Why the hole case was added** after this build: `?XINSIDE` here takes
  an all-zero hot area as a rectangle at the origin, as in the two builds
  before it; what the tables looked like that made the two later builds
  test for one is not established
  ([ledger](../../open-questions.md#motion-16-bit)).
- **The handlers not read.** This build has been read only where it differs
  from `ENVIRO.EXE` — the table, the ordinals, `?XINSIDE`, the error
  messages — so what is open there is open here: the descriptor drawer, the
  fades, the sound driver interface beyond its timer
  ([ENVIRO.EXE](enviro-exe.md#open-questions)).
- **Jeff Jet's two contradictory GFX slots** are its container's question,
  not this binary's ([the DATA container](../formats/container.md)).

## See also

- [ENVIRO.EXE](enviro-exe.md) — the latest build, and everything the five share
- [BMZ.EXE](bmz-exe.md) — the fourth build, whose ordinals are those of Die
  Enviro-Kids greifen ein
- [STERN.EXE](stern-exe.md) — the build before this one, with this domain table over `LL.EXE`'s core table
- [LL.EXE](ll-exe.md) — the oldest build, whose domain table binds at 102
- [Kernel words](../vm/kernel-words.md) — the two tables, entry by entry
- [The DATA container](../formats/container.md) — the volumes and the packing this build reads
- [Other files (Jeff Jet)](../../games/jeffjet/other-files.md) — what else the installation holds
