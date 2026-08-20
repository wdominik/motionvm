# Dunkle Schatten 2 — Technical Documentation

Reverse-engineering documentation for **"Im Netzwerk gefangen – Dunkle
Schatten 2"** ("Caught in the Network – Dark Shadows 2", 1996), a German
point-and-click adventure for DOS, published by Art Department
Werbeagentur GmbH.

## The game and its engine

`ENGINE.EXE` is not the game — it is **MOTION**, a general adventure
authoring system by DigiTales (Stefan Hoffmann), version V0.06.06/R109,
dated 1996-10-22, built with Watcom C/C++32 and shipped as an LE binary for
the DOS/4GW extender. It bundles a complete IDE, a Forth compiler, and a
debugger. The game itself is data: **86 compiled Forth script modules**
plus sprites, palettes, fonts, texts, and music in three resource
containers.

The display mode is 640×480 with 256 colors (VESA required).

**Boot chain:** `DS2.BAT` → `ENGINE.EXE` → `SYSTEM.RSC` (plain-text Forth:
`4 =>GET` / `START`) — load module 4, run its `START` word.

## Global conventions

- All multi-byte values are **little-endian**; `u8`/`u16`/`u32` denote
  unsigned integers of that width, `i16`/`i32` signed ones.
- Text is encoded in **code page 437** (German umlauts at `0x80..0xFF`).
- A **cell** is a 32-bit value — the unit of the Forth VM.
- Stack effects are written in Forth notation: `( a b -- c )`.
- Engine addresses (e.g. `0xdb067`) refer to `ENGINE.EXE`'s **relocated**
  image (see [ENGINE.EXE](engine/engine-exe.md)).
- Resources are addressed by *(type, id)*; script module ids equal module
  numbers.

## Documentation map

### File formats

| Page | Covers |
|---|---|
| [RSC containers](formats/rsc-container.md) | The resource archives `001.RSC`–`003.RSC` |
| [GFX8 sprites](formats/gfx8-sprites.md) | 8-bit paletted graphics |
| [The GFXCRUNCH LZW codec](formats/lzw.md) | Compression shared by sprites and fonts |
| [Palettes](formats/palette.md) | 6-bit VGA palettes, `000.PAL` |
| [Text tables](formats/text-tables.md) | String tables |
| [Blocks](formats/block.md) | Mixed binary data: HMI music and game data |
| [Script modules](formats/script-modules.md) | Compiled Forth modules (`.SCR`) |
| [HMI songs](formats/hmi.md) | The music format inside the blocks |
| [Ad Lib banks](formats/adlib-bank.md) | The FM patches the music is played with |
| [Driver archives](formats/driver-archive.md) | The `.386` bundles the sound drivers ship in |
| [Fonts](formats/fonts.md) | Bitmap fonts (`.FNT`) |
| [Font reference table](formats/font-reference-table.md) | `000.FRT`, character → glyph |
| [Other files](formats/other-files.md) | Bootstrap, sound drivers, readmes, … |

### The virtual machine

| Page | Covers |
|---|---|
| [Execution model](vm/execution-model.md) | Interpreter, stacks, the address model, blocking words |
| [Threaded code](vm/threaded-code.md) | Cell encoding, ordinals, inline operands, branches |
| [Word semantics](vm/word-semantics.md) | Behavior of the core Forth words (with the deviations from standard Forth) |
| [Kernel words](vm/kernel-words.md) | The 356-word kernel, calling convention, arities |

### The engine

| Page | Covers |
|---|---|
| [ENGINE.EXE](engine/engine-exe.md) | The LE binary, fixups, what lives where |
| [Game loop](engine/game-loop.md) | The 25 fps frame cycle, tasks, input |
| [Interaction machine](engine/interaction.md) | Clicks, the verb menu, `DOORDER`/`EXECORDER` |
| [Dialogue machine](engine/dialogue-machine.md) | The native conversation apparatus |
| [Screens](engine/screens.md) | Display layers, the drawn buffer, compositing |
| [Descriptors](engine/descriptors.md) | The scene graph: sprites, images, text |
| [Text rendering](engine/text-rendering.md) | Metrics, fonts, placement |
| [Transitions](engine/transitions.md) | The FADEOUT/FADEIN curtain |
| [Audio](engine/audio.md) | Music, samples, drivers, and the nine sound words |
| [The FM driver](engine/fm-driver.md) | `fmmidi3.com`, register for register |
| [Game structure](engine/game-structure.md) | Startup, locations, story flags, saving |
| [Walking](engine/walking.md) | The figure-movement system (`DOWALK` and the script words) |
| [Savegames](engine/savegames.md) | The three files a slot is made of, and what a save keeps |
| [Module map](engine/module-map.md) | What each of the 86 script modules does |

### The script library (every Forth-defined word)

| Page | Covers |
|---|---|
| [Globals](library/globals.md) | Module 2: shared state, core helpers, constants |
| [Shell](library/shell.md) | Module 4: boot, the control handler, menus, saving |
| [Game library](library/game-library.md) | Module 5: verbs, records, hit testing, tasks |
| [Text and speech](library/text-and-speech.md) | Captions, character speech, dialogue prep |
| [Animation](library/animation.md) | Module 6's runtime and the GT sequence animator |
| [Objects and flags](library/objects-and-flags.md) | Module 11: the object table, story flags, dialogue data |
| [Dialogue](library/dialogue.md) | Module 13: global verb handlers, global tasks, the info book |
| [The BBS](library/bbs.md) | Module 216: the in-game network terminal |

### Reference

| Page | Covers |
|---|---|
| [Resource inventory](inventory.md) | What the containers hold, by the numbers |
| [Open questions](open-questions.md) | Everything unknown, unverified, or hypothetical, in one place |
| [Departures](departures.md) | Every place motionvm knowingly does something else, and why |

## The shipped files

| File(s) | Format |
|---|---|
| `001.RSC`, `002.RSC`, `003.RSC` | [Resource containers](formats/rsc-container.md) |
| `SYSTEM.RSC` | Plain-text Forth bootstrap ([other files](formats/other-files.md)) |
| `ENGINE.EXE` | [The MOTION engine](engine/engine-exe.md) |
| `DOS4GW.EXE`, `_RUNVM.VMC` | DOS extender and its memory config |
| `000.PAL` | [Palette](formats/palette.md) |
| `000.FNT` | [System font](formats/fonts.md) |
| `000.FRT` | [Font reference table](formats/font-reference-table.md) |
| `002.SCR`, `011.SCR` | [Script modules](formats/script-modules.md) |
| `DS2.BAT`, `DS2.ICO` | Launcher and icon |
| `DRUM.BNK`, `MELODIC.BNK` | [Ad Lib instrument banks](formats/adlib-bank.md) |
| `SNDSETUP.*`, `HMI*.386`, `LOADPATS.EXE`, `PATCHES.INI`, `TEST.*` | Sound setup and drivers ([other files](formats/other-files.md)) |
| `LIESMICH.DOK`, `LIESMICH.TXT` | German readme files |
| `RSC.INF` | Resource metadata (not analyzed) |
