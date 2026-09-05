[← Documentation index](../README.md)

# The other MOTION games

*Three MOTION games whose files the readers open and measure, and which no page here describes and the player does not play: Checker 2000 on the 32-bit engine, Compaq and Eddy M. on the 16-bit one. What is on this page is what the files say and what `motionvm-motion-tools` reads out of them, with the game's own directory name as its key; nothing here was played.*

MOTION made more games than the five motionvm plays, and three of them are
on hand. They matter to this documentation twice over. The format pages
measure over their containers, which is how a claim about a *generation*
rather than about one game is made at all — the two framings of the 16-bit
container, which segments are packed, where a volume's items begin. And they
are the cases the openers refuse: a directory that has the generation's
shape and is not one of the five is exactly what the detection has to say
no to, by name.

## Checker 2000 — `CHECKER`

A 32-bit game: four `NNN.RSC` containers beside an `ENGINE.EXE` of its own,
`V0.04.15/R78` where Dunkle Schatten 2's is `V0.06.06/R109`, with the same
DOS/4GW extender, the same HMI sound stack (`HMIDRV.386`, `HMIMDRV.386`,
`HMIDET.386`, `MELODIC.BNK`, `DRUM.BNK`, `TEST.HMI`) and a `SYSTEM.RSC` of
43 bytes. Its readme asks for a VESA card, a mouse and 4 MB.

What the readers make of it, through `motionvm-motion-tools`:

| Bank | Holds |
|---|---|
| `001.RSC` | 33 sprites, 264 text tables, 80 blocks, 119 script modules, 65 palettes |
| `002.RSC` | 1321 sprites, 3 fonts |
| `003.RSC` | 88 sprites, 88 palettes |
| `004.RSC` | 43 blocks |

Every one of the 119 modules disassembles through its own binary's kernel
table — 332 words, against the 356 of the later build — and the modules
reach for 138 of them, 91 % of the uses being the interpreter's own. The
authoring template is plainly the same: `STARTUP`, `START` and `INCLLOC` are
exported from modules 3, 4 and 5 exactly as in Dunkle Schatten 2, which is
what says those three words are the template's and not one game's. What its
module 2 does *not* have is `_STARTLOC` and `_NEXTLOC`, the script variables
Dunkle Schatten 2's compiler named and the engine reads — and that is the
signature the 32-bit opener asks for. Without it this directory passed
detection, put Dunkle Schatten 2's name on the window and failed inside the
VM; now it is refused before that with a sentence that names the game whose
script the directory does not hold ([ENGINE.EXE](../motion32/engine/engine-exe.md),
[script modules](../motion32/formats/script-modules.md)).

## Compaq — `COMPAQ`

A 16-bit game of the earlier framing, and the only other one: `COMPAQ.EXE`
(138 454 bytes, stamped 1994-03-02), one `DATA.-1-` of 445 972 bytes and a
`GFX.INF` of 4800 bytes — the sprite-dimension side table that only the two
earlier-framing games ship, because their sprites are stored packed and a
width cannot be read out of the container without unpacking the item
([Victor Loomes' other files](vloomes/other-files.md)).

The container reader tells the framing apart by two identities that have to
hold at once — the offset table's first entry is where the tables end,
14 848 here as in Victor Loomes, and its last entry is the file's length,
445 972 — and no later container can satisfy the first. Once known, the
framing's three consequences are measured on this game as on Victor Loomes:
no packing field, so which segments are packed is the generation's — all
311 of the 1200 sprite slots in use and all 6 fonts are packed, under the
ten-byte header, the font reference table under the eight-byte one, and the
4 modules, 4 palettes and 5 text tables stored plainly; and the word the
later framing uses for a volume count holds 2 over one file
([the DATA container](../motion16/formats/container.md#the-earlier-framing)).
It ships no blocks at all, so no music. Its boot is module 100, word 1000.

What stops the reader short is the binary: `COMPAQ.EXE` is none of the four
players whose kernel tables have been read, so the modules come out as bytes
and not as listings — the tool says so, naming the four.

## Eddy M. — `EDDIEM`

A 16-bit game of the later framing on **three** volumes, the most any game
on hand has: `DATA.-1-`, `DATA.-2-` and `DATA.-3-` (216 697, 1 265 145 and
1 195 841 bytes, stamped October 1994), a player called `STERN.EXE` (167 334
bytes, 1994-10-06), the PSM 2 sound stack the 1995/96 games ship
(`SOUND.EXE`, `MUSADL.DRV`, the four `DMA*.DRV`, `DETECTOR.DRV`), a
`MAKEBOOT.EXE` its readme explains — a boot floppy for a machine short of
EMS or base memory — and a `MOUSE.SYS`.

It is the game that gives the later framing its third volume, and its
occupancy words are the reason the reader takes them as a bitmask: the
header declares 3, one spare offset-table entry, and every segment packed —
1772 of 2500 sprite slots, 119 blocks, 62 modules, all 25 palettes, 2 fonts,
the font reference table and 51 text tables, packed to the last. Three of
the blocks are PSM 2 songs. It is also the one game whose second and third
volumes leave 24 bytes between the tables' end and the first item, which is
why that offset is documented as where items *may* begin and not where they
must ([the DATA container](../motion16/formats/container.md#occupancy-table-offset-38)).
The packing flags and the shape of the items agree in all 28 segments of
the four multi-volume-capable games, this one included. Its boot is module
100, word 411.

As with Compaq, the binary is the edge: `STERN.EXE` is not one of the four
read players, so its modules are bytes to the tool.

## What they are not

No page describes any of the three as a game — its story, its locations,
its modules — and motionvm does not open them: the 32-bit opener refuses
Checker 2000 by its missing signature, and the 16-bit opener claims a
directory by the player beside its container, which none of these is. What
they are is evidence: a claim on a format page that names a generation was
measured over them as well as over the five, and a reader that reads them
has read the generation rather than one game's habit of it.

## See also

- [The DATA container](../motion16/formats/container.md) — both framings, measured over all six 16-bit games
- [`GFX.INF`](vloomes/other-files.md) — the side table the earlier framing's games ship
- [ENGINE.EXE](../motion32/engine/engine-exe.md) — the binary Checker 2000 has an earlier build of
- [motionvm-motion-tools](../tools.md) — what read these directories
