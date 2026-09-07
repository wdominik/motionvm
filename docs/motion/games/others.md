[← Documentation index](../README.md)

# The other MOTION game

*One MOTION game whose files the readers open and measure, and which no page here describes and the player does not play: Compaq, on the 16-bit engine. What is on this page is what the files say and what `motionvm-motion-tools` reads out of them, with the game's own directory name as its key; nothing here was played.*

MOTION made more games than the seven motionvm plays, and one more is on
hand. It matters to this documentation twice over. The format pages measure
over its container, which is how a claim about a *generation* rather than
about one game is made at all — the two framings of the 16-bit container,
which segments are packed, where a volume's items begin. And it is a case
the openers refuse: a directory that has the generation's shape and is not
one of the seven is exactly what the detection has to say no to, by name.

Checker 2000 stood on this page until it was played; it has
[pages of its own](checker/README.md) now, and its binary
[a page](../motion32/engine/engine-r78.md) beside Dunkle Schatten 2's.

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

What stops the reader short is the binary: `COMPAQ.EXE` is none of the five
players whose kernel tables have been read, so the modules come out as bytes
and not as listings — the tool says so, naming the five.

## What it is not

No page describes it as a game — its story, its locations, its modules —
and motionvm does not open it: the 16-bit opener claims a directory by the
player beside its container, which this is not. What it is is evidence: a
claim on a format page that names a generation was measured over it as well
as over the seven, and a reader that reads it is a reader that reads the
format rather than one game's use of it.

## See also

- [Documentation index](../README.md)
- [Checker 2000](checker/README.md) — the game that left this page
- [The DATA container](../motion16/formats/container.md) — the earlier framing
- [Victor Loomes](vloomes/README.md) — the other earlier-framing game
