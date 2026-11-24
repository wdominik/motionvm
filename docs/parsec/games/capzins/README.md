[← PARSEC documentation](../../README.md)

# Captain Zins

*Captain Zins (1994), the second title on PARSEC's `parsec1994` generation.*

## The program

`XP01.EXE` is the game: 74 775 bytes packed, 138 752 unpacked, `DGROUP`
`1FC4h`. Addresses on this page are offsets into that unpacked image,
linear (`seg · 16 + off`). `ZINS.BAT` runs `xp00.exe`, the title program,
and `xp01.exe` only when it left errorlevel 0; `INFO.BAT` runs `xp02 liesmich.dok`, the readme's
viewer. `LIESMICH.DOK` asks for an 80286, VGA or MCGA and an AdLib card at
least.

The title program's credits (`XP00.EXE`, image offset `0xFC05` onward)
open with *Die Dresdner Bank präsentiert* and list the developers by name,
in alphabetical order, with no studio: among them *Norbert Schmidt —
Programmierung, Konzeptumsetzung, Soundprogrammierung*, *Jochen Hess —
Musik, Dialogregie, Zusätzliche Programmierung*, *Michael Bohne —
Spielidee* and *Thomas Scholl — Konzept, Projektleitung*, and the music
system, *PSM 2.00 Soundsystem · Copyright (C) 1988-1994 · Norbert Schmidt
& Jochen Heß*.

The rooms are slots of `RESOURCE/CZROOM.DAT`, the FGROUP container
Dunkle Schatten's `DSROOM.DAT` is. The location table at `1EBBh:0000h`
has twenty rows of `2Ch` bytes; rows 1 to 18 are places, row 0 (kind 0,
"Extro") is the walk on and row 19 (kind 3, no name) the way back into a
loaded game.

## Its library build

The program is linked against another build of the library than Dunkle
Schatten's, and which one it is can be read off the routines' own bodies.
Against Dunkle Schatten's, this build:

- leaves the hover words alone on the input lock, its lifting, hover off,
  the script's exit and the mode reset, and takes `0:0` for "no hover"
  where Dunkle Schatten takes `FFFFh`;
- has no hook call in the task start, no task asked for in the
  dispatcher's head, no screen slot, no forest, and does not remember the
  door it was entered by;
- fades over 2 000 ms on leaving, for every kind, and fades the music in
  over 6 000 ms on entering;
- keeps a callback at `[1980h]` that the entry asks about objects 12h to
  19h, except in location 6, and that the Bits and the Schranke bend to a
  routine of their own;
- loads one of two sets of strip and figure, `PANEL%.DAT` and
  `PLAY0%.DAT`, by whether the location is above 2, splits the carried
  list at item 6 and shows six items to a window;
- keeps 45 effect slots, 14 of them every room's, with two-word masks for
  rooms below `13h`, and songs for room slots below `13h`;
- keeps a slot word's style in fields of its own, `and bx, 0Fh` and the
  table at `cs:04BAh` (`0x08741`).

## The start

`_main` stores no start. `[1A5Ch]` is 0 in the image, the start's load
(`0FBFh:007Ah`, `0x006F5`) leaves it so without a saved game, the options
come up (`0CD4h:045Ah`, `0x00744`), and the main loop calls the walk on for
row 0 (`0x0075E`, `0x00768`), which takes the first row whose kind is
neither 0 nor 3, row 1, the Wohnung, and enters it on `7Fh` (`0x00B4C`,
`0x00B50`).

## What is built

Every row has its handler's logic, with its hook, its animation hook and
its rule handlers;
the twenty-one handlers of the global table at `1F1Dh:0080h`; and the
shared hook `0A01h:18F6h`. Each is held against the original routine
([verification](../../verification.md)). Play begins in the title program
`XP00.EXE` ([boot](../../engine/boot.md#captain-zinss-title-program)), and
when Money's third scene ends the game on a location of −1, the parade of
`RESOURCE\EXTROS.DAT` plays until an event ([the way
out](../../engine/ending.md)); for quitting the program shows nothing.

## What is not built

No part of the game is known to be missing. What is not yet known about
the original is in [open questions](../../open-questions.md#captain-zins).
