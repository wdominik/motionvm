# MOTION — Technical Documentation

Reverse-engineering documentation for **MOTION**, the DOS adventure authoring
system by DigiTales (Stefan Hoffmann), and for the games built with it that
motionvm plays today:

| Game | Short name | Engine generation |
|---|---|---|
| *Im Netzwerk gefangen – Dunkle Schatten 2* (1996, Art Department Werbeagentur GmbH, commissioned by the Bundesministerium des Innern) | **DS2** | **MOTION 32-bit** — `ENGINE.EXE` V0.06.06/R109, dated 1996-10-22 |
| *Die Enviro-Kids greifen ein* (1996, Art Department Werbeagentur GmbH, commissioned by the Ministerium für Umwelt, Raumordnung und Landwirtschaft des Landes Nordrhein-Westfalen — the shipped files name no client) | **ENVIRO** | **MOTION 16-bit** — `ENVIRO.EXE`, dated 1996-08-27 |
| *Jeff Jet - Abenteuer InfoHighway* (Promotion Software GmbH, commissioned by the Hewlett Packard GmbH — the in-game credits name both; `HP.BAT` signs off 1995, the shipped files are re-stamped 1998-04-10) | **JEFFJET** | **MOTION 16-bit** — `HPPLAY.EXE`, an older build of the same player |

## The engine and its two generations

MOTION is not a game. It is an authoring system: a Forth compiler and a
runtime, in the 32-bit generation also an IDE and a debugger, all in one
binary. A game made with it is *data* — compiled Forth script modules plus
sprites, palettes, fonts, texts and music in resource containers. The engine
binary reads that data and runs it.

Those three were built with two generations of that system, and so were the
MOTION games this documentation cites but does not describe. The generations
share the *language* and most of the *vocabulary*; they do not share the
*machine*:

| | MOTION 16-bit | MOTION 32-bit (DS2) |
|---|---|---|
| Binary | `ENVIRO.EXE` and `HPPLAY.EXE`: 16-bit real-mode MZ, Turbo-C, needs EMS; player only, no compiler | `ENGINE.EXE`: 32-bit LE for DOS/4GW, Watcom C/C++32; IDE, compiler, debugger and player |
| Container | `DATA.-n-`, one volume per floppy, seven segments in one id space, items packed or plain | `NNN.RSC` files, merged by type and id |
| Cell | 16 bits | 32 bits |
| Kernel call | `0x8000 \| ordinal`, ordinals 1-based in two tables | `0x4000xxxx`, ordinals in steps of five |
| Word call | A global 16-bit word id through a table `=>GET` fills | `(module << 16) \| offset` |
| Return | The kernel word `##` (cell `0x8001`) | The cell 0 |
| Graphics | Raw `u16 w, u16 h, u16 0` + pixels, no palette of its own | `32BITGFX` + GFXCRUNCH LZW, palette per sprite |
| Video | 320×200×256 | 640×480×256 |
| Music | PSM 2 (Parsec) through `MUSADL.DRV` | HMI SOS, Ad Lib banks |
| Boot | Module and word id in the container header (module 100, `RUN`) | `SYSTEM.RSC`: `4 =>GET` / `START` |

What transfers between the two: the compiler-output semantics (literals,
`VAR`/`CONST`, the branch and loop runtimes, inline strings), the names of
most kernel words (161 of the 16-bit kernel's 233 names occur verbatim in the
32-bit kernel), the descriptor/screen model, the three-argument fades, the
save-slot scheme and the module operators. What does not: ordinals, addresses,
the return cell, the container, the asset encodings, the music format, and —
in the 16-bit generation — off-screen buffer words that the 32-bit game never
exercises.

## Global conventions

- All multi-byte values are **little-endian**; `u8`/`u16`/`u32` denote
  unsigned integers of that width, `i16`/`i32` signed ones.
- Text is encoded in **code page 437** (German umlauts at `0x80..0xFF`).
- A **cell** is the unit of the Forth VM: **32 bits** under `motion32/`,
  **16 bits** under `motion16/`. A page says which generation it describes in
  the line under its title.
- Stack effects are written in Forth notation: `( a b -- c )`.
- **Addresses.** 32-bit engine addresses (e.g. `0xdb067`) refer to
  `ENGINE.EXE`'s **relocated** image (see
  [ENGINE.EXE](motion32/engine/engine-exe.md)). 16-bit engine addresses are
  **file offsets** into `ENVIRO.EXE` (e.g. `file 0x2064e`); where a far
  pointer is meant it is written `segment:offset` of the load image, which
  begins at file offset `0x3200` (see [ENVIRO.EXE](motion16/engine/enviro-exe.md)).
- Resources are addressed by *(type, id)*; script module ids equal module
  numbers.
- **Provenance.** A page under `motion32/` describes the 32-bit engine as
  measured on DS2's files; a page under `motion16/` describes the 16-bit engine
  as measured on ENVIRO's, and on Jeff Jet's where the two builds differ.
  Pages under `games/` describe one game's own data and script library. A
  sentence that names no game holds for the whole generation; a count ("all 86
  modules", "all 1586 sprites") is always a count over one game's corpus, and
  says which.

## Documentation map

### MOTION 32-bit (DS2)

| Page | Covers |
|---|---|
| [RSC containers](motion32/formats/rsc-container.md) | The resource archives `001.RSC`–`003.RSC` |
| [GFX8 sprites](motion32/formats/gfx8-sprites.md) | 8-bit paletted graphics |
| [The GFXCRUNCH LZW codec](motion32/formats/lzw.md) | Compression shared by sprites and fonts |
| [Palettes](motion32/formats/palette.md) | 6-bit VGA palettes, `000.PAL` |
| [Text tables](motion32/formats/text-tables.md) | String tables |
| [Blocks](motion32/formats/block.md) | Mixed binary data: HMI music and game data |
| [Script modules](motion32/formats/script-modules.md) | Compiled Forth modules (`.SCR`) |
| [HMI songs](motion32/formats/hmi.md) | The music format inside the blocks |
| [Ad Lib banks](motion32/formats/adlib-bank.md) | The FM patches the music is played with |
| [Driver archives](motion32/formats/driver-archive.md) | The `.386` bundles the sound drivers ship in |
| [Fonts](motion32/formats/fonts.md) | Bitmap fonts (`.FNT`) |
| [Font reference table](motion32/formats/font-reference-table.md) | `000.FRT`, character → glyph |
| [Execution model](motion32/vm/execution-model.md) | Interpreter, stacks, the address model, blocking words |
| [Threaded code](motion32/vm/threaded-code.md) | Cell encoding, ordinals, inline operands, branches |
| [Word semantics](motion32/vm/word-semantics.md) | Behavior of the core Forth words (with the deviations from standard Forth) |
| [Kernel words](motion32/vm/kernel-words.md) | The 356-word kernel, calling convention, arities |
| [ENGINE.EXE](motion32/engine/engine-exe.md) | The LE binary, fixups, what lives where |
| [Game loop](motion32/engine/game-loop.md) | The 25 fps frame cycle, tasks, input |
| [Interaction machine](motion32/engine/interaction.md) | Clicks, the verb menu, `DOORDER`/`EXECORDER` |
| [Dialogue machine](motion32/engine/dialogue-machine.md) | The native conversation apparatus |
| [Screens](motion32/engine/screens.md) | Display layers, the drawn buffer, compositing |
| [Descriptors](motion32/engine/descriptors.md) | The scene graph: sprites, images, text |
| [Text rendering](motion32/engine/text-rendering.md) | Metrics, fonts, placement |
| [Transitions](motion32/engine/transitions.md) | The FADEOUT/FADEIN curtain |
| [Audio](motion32/engine/audio.md) | Music, samples, drivers, and the nine sound words |
| [The FM driver](motion32/engine/fm-driver.md) | `fmmidi3.com`, register for register |
| [Walking](motion32/engine/walking.md) | The figure-movement system (`DOWALK` and the script words) |
| [Savegames](motion32/engine/savegames.md) | The three files a slot is made of, and what a save keeps |

### MOTION 16-bit

| Page | Covers |
|---|---|
| [The DATA container](motion16/formats/data-container.md) | `DATA.-n-`: header, volumes, the occupancy bitmask, the offset tables, packed items, the seven segments |
| [Sprites](motion16/formats/sprites.md) | Raw 8-bit graphics without a palette of their own |
| [Fonts](motion16/formats/fonts.md) | Raw bitmap fonts and the font reference table |
| [Text tables](motion16/formats/text-tables.md) | String tables with relative offsets |
| [Blocks](motion16/formats/blocks.md) | PSM 2 songs, animation catalogs, per-location tables |
| [PSM 2 music](motion16/formats/psm-music.md) | The music modules and the `MUSADL.DRV` driver that plays them |
| [Script modules](motion16/formats/script-modules.md) | Compiled Forth modules with 16-bit cells and global word ids |
| [Execution model](motion16/vm/execution-model.md) | Interpreter, stacks, the flat address space, the word table |
| [Threaded code](motion16/vm/threaded-code.md) | Cell encoding, ordinals, inline operands, branches |
| [Kernel words](motion16/vm/kernel-words.md) | The 233-word kernel, its two tables, what the game uses |
| [ENVIRO.EXE](motion16/engine/enviro-exe.md) | The later build of the player: the MZ binary, what lives where |
| [HPPLAY.EXE](motion16/engine/hpplay-exe.md) | The earlier build: five words fewer, every ordinal from 124 up shifted |
| [Boot and frame loop](motion16/engine/boot-and-loop.md) | `RUN`, `SCRCTRL`, `ANIMPLAY`, location changes, shutdown |
| [Descriptors and screens](motion16/engine/descriptors.md) | What the scripts' call sites establish about the kernel's display words |
| [Text rendering](motion16/engine/text-rendering.md) | The drawer's two passes, the gaps, justification |
| [Off-screen buffers](motion16/engine/buffers.md) | `BUFON`, `SETBUF`, `SDBUF`, `KILLNBUF` — load-bearing here |

### Dunkle Schatten 2

| Page | Covers |
|---|---|
| [Game structure](games/ds2/game-structure.md) | Startup, locations, story flags, saving |
| [Module map](games/ds2/module-map.md) | What each of the 86 script modules does |
| [Resource inventory](games/ds2/inventory.md) | What the containers hold, by the numbers |
| [Other files](games/ds2/other-files.md) | Bootstrap, sound drivers, readmes, … |
| [Globals](games/ds2/library/globals.md) | Module 2: shared state, core helpers, constants |
| [Shell](games/ds2/library/shell.md) | Module 4: boot, the control handler, menus, saving |
| [Game library](games/ds2/library/game-library.md) | Module 5: verbs, records, hit testing, tasks |
| [Text and speech](games/ds2/library/text-and-speech.md) | Captions, character speech, dialogue prep |
| [Animation](games/ds2/library/animation.md) | Module 6's runtime and the GT sequence animator |
| [Objects and flags](games/ds2/library/objects-and-flags.md) | Module 11: the object table, story flags, dialogue data |
| [Dialogue](games/ds2/library/dialogue.md) | Module 13: global verb handlers, global tasks, the info book |
| [The BBS](games/ds2/library/bbs.md) | Module 216: the in-game network terminal |

### Die Enviro-Kids greifen ein

| Page | Covers |
|---|---|
| [Game structure](games/enviro/game-structure.md) | Setting, the location scheme, the three module series, verbs, saving |
| [Module map](games/enviro/module-map.md) | What each of the 65 script modules does |
| [Resource inventory](games/enviro/inventory.md) | What `DATA.-1-` holds, by the numbers |
| [Other files](games/enviro/other-files.md) | Sound setup and drivers, the launcher, readme, leftovers |

### Jeff Jet - Abenteuer InfoHighway

| Page | Covers |
|---|---|
| [Game structure](games/jeffjet/game-structure.md) | Setting, the thirteen locations, the three module series, verbs, saving |
| [Module map](games/jeffjet/module-map.md) | What each of the 55 script modules does |
| [Resource inventory](games/jeffjet/inventory.md) | What the two `DATA.-n-` volumes hold, by the numbers |
| [Other files](games/jeffjet/other-files.md) | The launcher, the sound stack, the two splash pictures |

### Reference

| Page | Covers |
|---|---|
| [Open questions](open-questions.md) | Everything unknown, unverified, or hypothetical, in one place — for both generations |
| [Departures](departures.md) | Every place motionvm knowingly does something else, and why |

## The shipped files

### Dunkle Schatten 2

| File(s) | Format |
|---|---|
| `001.RSC`, `002.RSC`, `003.RSC` | [Resource containers](motion32/formats/rsc-container.md) |
| `SYSTEM.RSC` | Plain-text Forth bootstrap ([other files](games/ds2/other-files.md)) |
| `ENGINE.EXE` | [The MOTION 32-bit engine](motion32/engine/engine-exe.md) |
| `DOS4GW.EXE`, `_RUNVM.VMC` | DOS extender and its memory config |
| `000.PAL` | [Palette](motion32/formats/palette.md) |
| `000.FNT` | [System font](motion32/formats/fonts.md) |
| `000.FRT` | [Font reference table](motion32/formats/font-reference-table.md) |
| `002.SCR`, `011.SCR` | [Script modules](motion32/formats/script-modules.md) |
| `DS2.BAT`, `DS2.ICO` | Launcher and icon |
| `DRUM.BNK`, `MELODIC.BNK` | [Ad Lib instrument banks](motion32/formats/adlib-bank.md) |
| `SNDSETUP.*`, `HMI*.386`, `LOADPATS.EXE`, `PATCHES.INI`, `TEST.*` | Sound setup and drivers ([other files](games/ds2/other-files.md)) |
| `LIESMICH.DOK`, `LIESMICH.TXT` | German readme files |
| `RSC.INF` | Resource metadata (not analyzed) |

### Die Enviro-Kids greifen ein

| File(s) | Format |
|---|---|
| `DATA.-1-` | [The DATA container](motion16/formats/data-container.md) — the whole game |
| `ENVIRO.EXE` | [The MOTION 16-bit engine](motion16/engine/enviro-exe.md) |
| `KIDS.BAT` | Launcher ([other files](games/enviro/other-files.md)) |
| `SOUND.EXE`, `MUSADL.DRV`, `DMABLAST.DRV`, `DMASB16M.DRV`, `DMASB16S.DRV`, `DMASB2P.DRV`, `DETECTOR.DRV` | PSM 2 sound setup and drivers ([other files](games/enviro/other-files.md)) |
| `README.TXT` | German readme (EMS setup) |
| `A.DAT`, `32RTM.EXE`, `DPMI32VM.OVL` | Unreferenced by the game ([other files](games/enviro/other-files.md)) |

### Jeff Jet - Abenteuer InfoHighway

| File(s) | Format |
|---|---|
| `DATA.-1-`, `DATA.-2-` | [The DATA container](motion16/formats/data-container.md) — the whole game, on two volumes, packed |
| `HPPLAY.EXE` | [The older build of the MOTION 16-bit player](motion16/engine/hpplay-exe.md) |
| `HP.BAT` | Launcher ([other files](games/jeffjet/other-files.md)) |
| `SOUND.EXE`, `MUSADL.DRV`, `DMABLAST.DRV`, `DMASB16M.DRV`, `DMASB16S.DRV`, `DMASB2P.DRV`, `DETECTOR.DRV` | PSM 2 sound setup and drivers, byte-identical to the other 16-bit game's ([other files](games/jeffjet/other-files.md)) |
| `HPLOGO.EXE`, `PROMSOFT.EXE` | Graphic Workshop splash pictures, not MOTION ([other files](games/jeffjet/other-files.md)) |
