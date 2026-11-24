[← Documentation index](../README.md)

# PARSEC — Technical Documentation

Reverse-engineering documentation for **PARSEC**, the DOS adventure engine
Norbert Schmidt wrote, and for [*Dunkle Schatten*](games/ds1/README.md)
(1994), the first game built with it: commissioned by the Bundesministerium des
Innern, made by The Art Department WA GmbH, Bochum, on a concept by Platzer
Kommunikation GmbH, München. Its programs are `XP00.EXE`, the title
program, and `XP01.EXE`, the game — both 1994-10-24.

Every program the install ships carries the line *A PARSEC Production*,
and the game's own credits name the programmer, *Programmierung · Norbert
Schmidt*, and the music system beside him, *PSM 2.00 Soundsystem ·
Copyright (C) 1988-1994 · Norbert Schmidt & Jochen Heß*. The game's page
carries who made it and how each entry is evidenced.

A second title is built on the same library: [*Captain
Zins*](games/capzins/README.md) (1994), whose `XP01.EXE` is linked against
another build of it. The pages below are written from Dunkle Schatten's
programs, and their addresses are that build's unless a sentence names
Captain Zins.

The game is installed here into `DS1`, and that name in lower case, `ds1`,
is a key rather than a name: what is kept for the game is filed under it —
`MOTIONVM_GAMEDATA_DS1`, `saves/ds1/`, `docs/parsec/games/ds1/` — while
prose calls the game by its title. Captain Zins is installed into
`CAPZINS`, and its key is `capzins`.

## The engine

PARSEC is not an interpreter. Dunkle Schatten is **compiled C**: the
location handlers, the puzzles, the verbs and the order of a scene are code
in the game's own executable, linked against a library that loads rooms,
walks figures, runs animations, draws text and drives the music. What comes
out of files is the material: pictures, walk graphs, hotspots, animation
streams, dialogue trees and songs, in FGROUP and RIB containers under
`RESOURCE/`. So the game's program is read as a program — its handlers,
its tables and its strings — and what it does is what its code says.

| | Dunkle Schatten |
|---|---|
| Programs | `XP00.EXE`, `XP01.EXE`, `XP02.EXE`: 16-bit real-mode MZ, Turbo C, packed by a `PRSC` stub in front of LZEXE 0.91 |
| Containers | `RESOURCE/*.DAT`: FGROUP tables of RIB bundles, packed with a backwards LZ codec |
| Rooms | One FGROUP, `DSROOM.DAT`, a slot per location, each slot six resources around an `RDV1.00` tiled playfield |
| Video | 320×200×256, palettes as DAC banks |
| Sound | PSM 2 through `MUSADL.DRV` on the OPL and `DMABLAST.DRV`'s mixer on the DAC — the music with its digital voices, and the sound effects — out of the `PSMP2L.DAT` library |
| Boot | `SCHATTEN.BAT` runs `XP00`, and `XP01` when `XP00` leaves errorlevel 0 |

## Global conventions

- All multi-byte values are **little-endian**, except the 16-bit operands
  of the RIB LZ stream, which the codec reads backwards.
- Text is encoded in **code page 437**; umlauts and `ß` are shown as
  Unicode.
- **Addresses.** A linear address such as `0x7BFB` is an offset into the
  **unpacked** load image of `XP01.EXE` — 150 096 bytes, `DGROUP` `21EFh` —
  never into the packed file. A far address such as `0BB6h:027Ah` is a
  segment and an offset of that image, the segment counted from the image's
  start, so `0BB6h:027Ah` is `0xBDDA`. A word in brackets, `[1C76h]`, or a
  `DS:` offset such as `DS:13E8h` is in `DGROUP`. Addresses in `XP00.EXE`
  — 64 176 bytes unpacked, `DGROUP` `0E3Ah` — are marked as that program's.
  See [the program files](formats/executables.md).
- A **location** is a row of the program's location table, and its number
  is a slot number of `DSROOM.DAT`; a **row of kind 3** is one drawn on the
  forest picture of slot 20.
- A count ("all 31 object maps", "28 slots") is a count over Dunkle
  Schatten's files.

## Documentation map

### Formats

| Page | Covers |
|---|---|
| [Containers](formats/containers.md) | FGROUP and RIB: the shared directory shape, `NULLPTR` slots, the table-less RIB, which `RESOURCE/*.DAT` file has which shape |
| [The RIB LZ codec](formats/lz.md) | In place and backwards: the token table, 16-bit operands high byte first, the stored-payload path |
| [The program files](formats/executables.md) | `XP00`, `XP01` and `XP02.EXE`: the `PRSC` stub and LZEXE 0.91, the unpacked sizes and `DGROUP`, what the images hold as data |
| [Graphics](formats/graphics.md) | 320×200, DAC banks, pictures and cels, `CMBMOU.DAT` with the pointers and the font, `PLAY00.DAT`, `PANELS.DAT` |
| [Rooms](formats/rooms.md) | `DSROOM.DAT`: 60 slots, slot 20 for rows of kind 3, the six slot resources, `RDV1.00` layers, a tile as its table's entry k − 1, draw order, object and person records, walk graphs |
| [Object maps](formats/object-map.md) | The `OMP` map a forest or maze row binds the forest picture with: header and entries, the live byte, the flag bits |
| [Animations](formats/animations.md) | `AN` blocks: the directory, the entry with its working state, `ANIS`, chains of part nodes |
| [Hotspots](formats/hotspots.md) | `MBX`: 46-byte records, the verb mask, the string pool and per-verb answers, the mouse objects and the verb strip's seventeen |
| [Dialogue](formats/dialogue.md) | The banks of `DIALOG.DAT` and each room's own, how a scene number picks a bank, the `II` record, where a text ends |
| [Audio](formats/audio.md) | `PSMDRV.DAT` with `MUSADL.DRV` as its resource 5, and the `PSMP2L.DAT` library: its tables, module heads, the Ad Lib and the digital path |

### The engine

| Page | Covers |
|---|---|
| [Overview](engine/overview.md) | The programs, the subsystems, how a location comes up and how a click travels |
| [Boot and the title program](engine/boot.md) | The batch file, `XP00`: logos, menu, backstory, its music, what it leaves `XP01`, and how `XP01` begins |
| [Game loop](engine/game-loop.md) | The location table, the main loop, the onward path through rows of kind 0, the tick account, the screen's division |
| [Objects](engine/objects.md) | The object records and the draw chain, the entry function, and the forest and the maze: object maps bound to slot 20 and the slot held between forest rows |
| [Tasks](engine/tasks.md) | The task slots that speak, show and wait: what a scene is made of and how long a page stays |
| [The animator](engine/animator.md) | The byte code that turns an `AN` stream into frames, waits and codes |
| [Walking](engine/walking.md) | The walk graph, the flood, a leg's cost, scale, gait and the snap |
| [Verbs](engine/verbs.md) | The interaction mode word `[288Fh]`, the hover rules, the verb strip, what a click reaches |
| [Text](engine/text.md) | The control bytes below `20h`, the substitutions, the speech box and its clamp, the captions |
| [The pointer](engine/cursors.md) | The pointer library: descriptor selection, the random animation, the mouse timer and the input state |
| [Options and the saved game](engine/options.md) | The game options, the confirmations, `SAVEGAME.SAV` and `SAVEGAME.CFG` with the player's record, location 60 |
| [Music](engine/music.md) | What the program asks of the PSM 2 driver: the piece per location, play on entry, the fade on exit, the music switch, the title program's piece |
| [The way out](engine/ending.md) | What `XP01` shows after its main loop: Dunkle Schatten's end and quit, Captain Zins's parade |
| [Palettes](engine/palettes.md) | Palette state, the DAC banks per resource, the panel's colors and the transition ramps |

### Dunkle Schatten

| Page | Covers |
|---|---|
| [The game](games/ds1/README.md) | What it is about, who made it, and why it exists |
| [Game structure](games/ds1/game-structure.md) | The sixty-one rows, the opening, the map, the days, the forest and the maze, saving |
| [Other files](games/ds1/other-files.md) | The batch files, the `.DOK` viewer, the end sequence's file and the rest |

### Captain Zins

| Page | Covers |
|---|---|
| [The game](games/capzins/README.md) | The program, its library build, the start, what is built and what is not |

### Reference

| Page | Covers |
|---|---|
| [Open questions](open-questions.md) | What is not yet known about the original |
| [Departures](departures.md) | Every place motionvm knowingly does something else, and why |
| [Verification](verification.md) | What is held against the original's own output, and what only against the game's files |
| [motionvm-parsec-tools](tools.md) | The command-line inspector: what it reads out of the game's files and what it writes |

## The shipped files

| File(s) | Format |
|---|---|
| `XP00.EXE`, `XP01.EXE` | [The title program and the game](formats/executables.md) |
| `XP02.EXE`, `INFO.DOK`, `FILES.DOK`, `INFO.BAT`, `FILES.BAT` | The `.DOK` viewer and the two German texts it shows ([other files](games/ds1/other-files.md)) |
| `RESOURCE/DSROOM.DAT` | [Rooms](formats/rooms.md) — every location |
| `RESOURCE/INTROS.DAT` | [Containers](formats/containers.md) — the title program's pictures |
| `RESOURCE/EXTROS.DAT` | [Containers](formats/containers.md) — the sequences on the way out of `XP01`: the end of the game and quitting ([other files](games/ds1/other-files.md)) |
| `RESOURCE/PANELS.DAT`, `RESOURCE/CMBMOU.DAT`, `RESOURCE/PLAY00.DAT` | [Graphics](formats/graphics.md) — the verb strip, the pointers and the font, the player's cels |
| `RESOURCE/DIALOG.DAT` | [Dialogue](formats/dialogue.md) — the two global banks |
| `RESOURCE/PSMP2L.DAT`, `RESOURCE/PSMDRV.DAT` | [Audio](formats/audio.md) — the music library and the sound drivers |
| `RESOURCE/PRGVER.DAT` | Named by both programs ([other files](games/ds1/other-files.md)) |
| `SCHATTEN.BAT`, `DS1.BAT`, `DS1.ICO` | Launchers and icon ([other files](games/ds1/other-files.md)) |
| `PSMCFG4.EXE`, `CONFIG.DAT` | Shipped beside the programs ([other files](games/ds1/other-files.md)) |

Captain Zins ships the same kinds of file, its own where the names differ:

| File(s) | Format |
|---|---|
| `XP00.EXE`, `XP01.EXE` | [The title program and the game](games/capzins/README.md#the-program) |
| `XP02.EXE`, `LIESMICH.DOK`, `INFO.BAT` | The `.DOK` viewer and the readme it shows ([the game](games/capzins/README.md#the-program)) |
| `RESOURCE/CZROOM.DAT` | [Rooms](formats/rooms.md) — every location, in the FGROUP container `DSROOM.DAT` is |
| `RESOURCE/INTROS.DAT` | [Containers](formats/containers.md) — the title program's pictures |
| `RESOURCE/EXTROS.DAT` | [Containers](formats/containers.md) — the parade after the main loop ([the way out](engine/ending.md#captain-zins)) |
| `RESOURCE/PANEL0.DAT`, `RESOURCE/PANEL1.DAT`, `RESOURCE/PLAY00.DAT`, `RESOURCE/PLAY01.DAT` | [Graphics](formats/graphics.md) — two sets of the verb strip and the player's cels, the first for locations 1 and 2, the second above them ([its library build](games/capzins/README.md#its-library-build)) |
| `RESOURCE/CMBMOU.DAT` | [Graphics](formats/graphics.md) — the pointers and the font |
| `RESOURCE/DIALOG.DAT` | [Dialogue](formats/dialogue.md) — the global banks |
| `RESOURCE/PSMP2L.DAT`, `RESOURCE/PSMDRV.DAT` | [Audio](formats/audio.md) — the music library and the sound drivers |
| `ZINS.BAT` | The launcher ([the game](games/capzins/README.md#the-program)) |
