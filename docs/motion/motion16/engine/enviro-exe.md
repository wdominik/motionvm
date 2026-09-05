[← Documentation index](../../README.md)

# ENVIRO.EXE — The Latest Build of the MOTION 16-bit Player

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere, in `STERN.EXE` with Falsches Spiel mit Eddie M. and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

`ENVIRO.EXE` (167 430 bytes, dated 1996-08-27) is the 16-bit MOTION
**player**: the Forth interpreter, the kernel words and the drivers behind
them, and nothing of the authoring system. It never names MOTION or
DigiTales; the product banner, the IDE, the compiler and the debugger that
the 32-bit `ENGINE.EXE` still carries are absent. The game it plays is
entirely in `DATA.-1-`.

Every address on this page and on the pages it links to is this image's. The
same player shipped four times more, earlier and under other names, as
`BMZ.EXE`, `HPPLAY.EXE`, `STERN.EXE` and `LL.EXE` — see their own pages
([BMZ.EXE](bmz-exe.md), [HPPLAY.EXE](hpplay-exe.md),
[STERN.EXE](stern-exe.md), [LL.EXE](ll-exe.md)) for what differs, ordinals
first.

## The binary

A plain DOS MZ executable, real mode, built with Turbo C (the runtime's
*"Turbo-C - Copyright (c) 1988 Borland Intl."* is at file `0x1f444`):

| Field | Value |
|---|---|
| Header | 800 paragraphs = `0x3200` bytes; the load image follows at file `0x3200` and is 154 630 bytes |
| Relocations | 3160 entries |
| Entry | `CS:IP = 0000:0000` of the load image; `SS:SP = 25b2:00e6` |

A far pointer `segment:offset` in the image is file offset
`0x3200 + segment × 16 + offset`; that is how the kernel table's name and
handler pointers below are resolved.

The program needs expanded memory: its messages ask for *"mindestens 1 MB
EMS-Erweiterungsspeicher"* and *"550.000 freien Basis-Speicher"*, and the
shipped `README.TXT` tells the player to load `EMM386.EXE` with `RAM` —
the opposite of the 32-bit engine, whose DOS/4GW extender wants EMS off.

## What lives where

| File offset | What |
|---|---|
| `0x1f9f6` | Kernel table 2 — 151 domain words, ordinals 105–255 ([kernel words](../vm/kernel-words.md)) |
| `0x2064e` | Kernel table 1 — 82 core words, ordinals 1–82 |
| `0x1fec5`–`0x20aae` | The kernel words' name strings |
| `0x174f7` | The interpreter loop, by a first reading that is not re-checked ([execution model](../vm/execution-model.md)) |
| `0x16f62` | The handler of `##` (`12c8:10e2`); every other handler address is in the kernel table |

Strings the player carries, read out of the image:

- The container and its volumes: `data.-1-`, `DATA.-#i-`, *"Bitte
  Diskette #d einlegen!"*, *"Datenblock <#s> nicht …"* — the format is
  multi-volume and the loader prompts for a disk it cannot find.
- Authoring-time names: `#F0R3i.txt`, `#F0R4i.gfx`, `#F0R3i.pal`,
  `#F0R3i.frt`, `#F0R3i.fth`, `#F0R3i.blk`, `gfx.inf`, *"GFX-File #s not
  found"*, *"Font-Maximum reached"*. `#F0R3i` is a printf-style pattern —
  a three-digit number — and the same pattern names the 32-bit engine's
  resources. None of these files is shipped; the player reads the container.
- Save files: `#F0R3i.frz`, `#F0R3i.anm`, `#F0R3i.blk` — a slot is three
  files, the same family the 32-bit engine writes (see [boot and frame
  loop](game-loop.md)).
- Sound: `psmcfg4.dat` and the driver names `DMABLAST.DRV`, `DMASB2P.DRV`,
  `DMASB16M.DRV`, `DMASB16S.DRV`, `DETECTOR.DRV`, `MUSADL.DRV` — the PSM 2
  stack ([other files](../../games/enviro/other-files.md)).

Absent from the image: `32RTM`, `DPMI`, `A.DAT` — the three leftovers in
the game directory are not referenced by the player.

## Relation to `ENGINE.EXE`

The two binaries share their kernel vocabulary — 161 of the 16-bit kernel's
233 names occur verbatim in the 32-bit kernel — and the compiler-output
conventions the interpreter executes, and nothing else that can be read
across: different compiler, different address width, different table
order, different container. `ENVIRO.EXE` contains no Forth compiler: the
strings `IF`, `ELSE`, `DO`, `BEGIN` do not occur, `=>TABLES` is in the
kernel but unused, and `#F0R3i.fth` is the only trace of source files.

## Reading the handlers

The handlers are Turbo C far functions: arguments pushed right to left,
far pointers as segment then offset, a 16-bit result in `ax` and a pointer
in `dx:ax`; the data segment is `0x1c24` (the kernel tables' name pointers
point into it). The kernel keeps its **data stack** behind a far pointer at
`DS:0x8160` — 2-byte cells, growing down — and every handler takes its
arguments through `12c8:0652` (pop into `ax`) and leaves results through
`12c8:0663` (push the argument); a stack effect is the count of those two
calls. Three more helpers recur: `016a:04ca` answers the **current
descriptor** (`DS:0x305a + screen × 0x13ae + 0x24 + descriptor × 0x2e`,
the screen from `DS:0x5de2` and the descriptor from `DS:0x3058`),
`1400:0490` turns a **script address** into a pointer (the arena base at
`DS:0x8154` plus the address with bit 0 cleared), and `1400:028f` **runs a
script word by id** through the interpreter — which is how the order
machine and `MOUSEINFO` call back into the game. What the reading settled
is on the pages it belongs to: [execution model](../vm/execution-model.md),
[boot and frame loop](game-loop.md), [buffers](buffers.md),
[descriptors and screens](descriptors.md).

## Open questions

- The handlers not read: the descriptor drawer (`016a:0aac`), the fades,
  the sound driver interface beyond its timer.

## See also

- [BMZ.EXE](bmz-exe.md) — the fourth build, this table less its last word
- [HPPLAY.EXE](hpplay-exe.md) — the third build, whose ordinals are shifted
- [STERN.EXE](stern-exe.md) — the second build, `LL.EXE`'s core table under `HPPLAY.EXE`'s domain table
- [LL.EXE](ll-exe.md) — the oldest build, whose domain table binds at 102
- [Kernel words](../vm/kernel-words.md) — the two tables, entry by entry
- [Execution model](../vm/execution-model.md) — what the interpreter does with a cell
- [ENGINE.EXE (MOTION 32-bit)](../../motion32/engine/engine-exe.md) — the 32-bit binary
