# MOTION — Technical Documentation

Reverse-engineering documentation for **MOTION**, the DOS adventure authoring
system by DigiTales (Stefan Hoffmann), and for the games built with it that
motionvm plays today:

| Game | Year | Commissioned by | Made by | Directory | Engine |
|---|---|---|---|---|---|
| [*Im Netzwerk gefangen – Dunkle Schatten 2*](games/ds2/README.md) | 1996 | Bundesministerium des Innern | DigiTales GmbH, Hamburg, produced by Art Department WA GmbH, Bochum | `DS2` | **32-bit** — `ENGINE.EXE` V0.06.06/R109, 1996-10-22 |
| [*Die Enviro-Kids greifen ein*](games/enviro/README.md) | 1996 | Ministerium für Umwelt, Raumordnung und Landwirtschaft NRW | Art Department Werbeagentur GmbH | `ENVIRO` | **16-bit** — `ENVIRO.EXE`, 1996-08-27: the latest build |
| [*Jeff Jet - Abenteuer InfoHighway*](games/jeffjet/README.md) | 1995 | Hewlett Packard GmbH | Promotion Software GmbH, Tübingen | `JEFFJET` | **16-bit** — `HPPLAY.EXE`: the second build |
| [*Hilfe für Amajambere*](games/hfa/README.md) | 1995 | Bundesministerium für wirtschaftliche Zusammenarbeit und Entwicklung | ART DEPARTMENT WA GmbH, Bochum | `HFA` | **16-bit** — `BMZ.EXE`, 1995-06-05: the third build |
| [*Victor Loomes – Das Spiel*](games/vloomes/README.md) | 1993 | LBS (Landesbausparkasse) | Promotion Software GmbH, Reutlingen | `VLOOMES` | **16-bit** — `LL.EXE`, 1993-05-20: the oldest build, and the earlier framing of the container |

Each game's own page carries what it is about, who made it and how each of
those entries is evidenced — a credits table, a license file, a launcher's
sign-off, or, where the files say nothing, the published record named as such.
Jeff Jet's year is its launcher's sign-off; its shipped files are re-stamped
1998-04-10.

**Two of the five name the engine**, at the two ends of the corpus. Victor
Loomes' credits close on *Erstellt unter · Motion 1.0 · Michel "Babe" Stigler
· EGO Software* (text table 8), and Dunkle Schatten 2 names it in its
programming entry: *Basierend auf: … "Motion"-Präsentations-System von
S. Hoffmann* (text table 6, entry 67), on the same page that gives
*Entwicklung und Gestaltung / DigiTales GmbH, Hamburg*. The later game therefore names the engine, its
author and his company outright, which is where the attribution above comes
from; the earlier one names a **version**, `1.0`, three years before the
builds this documentation is written from. What Victor Loomes' last two
entries belong to is not settled — the same table gives `Graphik` two names,
so a label there can take more than one value.

The directory is the name the original was installed into, and it is a key
rather than a name: everything kept per game is filed under it —
`MOTIONVM_GAMEDATA_<SLUG>`, `saves/<slug>/`, `docs/motion/games/<slug>/` — while prose
calls a game by its title.

## The engine and its two generations

MOTION is not a game. It is an authoring system: a Forth compiler and a
runtime, in the 32-bit generation also an IDE and a debugger, all in one
binary. A game made with it is *data* — compiled Forth script modules plus
sprites, palettes, fonts, texts and music in resource containers. The engine
binary reads that data and runs it.

Those five were built with two generations of that system, and so were the
three [other MOTION games](games/others.md) on hand — *Checker 2000*,
*Compaq* and *Eddy M.* — whose files the readers open and the format pages
measure over, and which no page describes as a game. The generations
share the *language* and most of the *vocabulary*; they do not share the
*machine*:

| | MOTION 16-bit | MOTION 32-bit |
|---|---|---|
| Binary | `ENVIRO.EXE`, `BMZ.EXE`, `HPPLAY.EXE` and `LL.EXE`: 16-bit real-mode MZ, Turbo-C, needs EMS; player only, no compiler | `ENGINE.EXE`: 32-bit LE for DOS/4GW, Watcom C/C++32; IDE, compiler, debugger and player |
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
  measured on the files of Dunkle Schatten 2; a page under `motion16/`
  describes the 16-bit engine
  as measured on the files of Die Enviro-Kids greifen ein, and on Jeff Jet's,
  Hilfe für Amajambere's or Victor Loomes' where the builds differ.
  Pages under `games/` describe one game's own data and script library. A
  sentence that names no game holds for the whole generation; a count ("all 86
  modules", "all 1586 sprites") is always a count over one game's corpus, and
  says which.

## Documentation map

### MOTION 32-bit

| Page | Covers |
|---|---|
| [Resource containers](motion32/formats/container.md) | `NNN.RSC`: the resource archives `001.RSC`–`003.RSC` |
| [Sprites](motion32/formats/sprites.md) | GFX8: 8-bit paletted graphics |
| [The GFXCRUNCH LZW codec](formats/lzw.md) | Compression shared by both generations' sprites, fonts and packed containers |
| [Palettes](motion32/formats/palette.md) | 6-bit VGA palettes, `000.PAL` |
| [Text tables](motion32/formats/text-tables.md) | String tables |
| [Blocks](motion32/formats/blocks.md) | Mixed binary data: HMI music and game data |
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
| [Resource containers](motion16/formats/container.md) | `DATA.-n-`: header, volumes, the occupancy bitmask, the offset tables, packed items, the seven segments |
| [Sprites](motion16/formats/sprites.md) | Raw 8-bit graphics without a palette of their own |
| [Fonts](motion16/formats/fonts.md) | Raw bitmap fonts and the font reference table |
| [Text tables](motion16/formats/text-tables.md) | String tables with relative offsets |
| [Blocks](motion16/formats/blocks.md) | PSM 2 songs, animation catalogs, per-location tables |
| [PSM 2 music](motion16/formats/psm-music.md) | The music modules and the `MUSADL.DRV` driver that plays them |
| [Script modules](motion16/formats/script-modules.md) | Compiled Forth modules with 16-bit cells and global word ids |
| [Execution model](motion16/vm/execution-model.md) | Interpreter, stacks, the flat address space, the word table |
| [Threaded code](motion16/vm/threaded-code.md) | Cell encoding, ordinals, inline operands, branches |
| [Kernel words](motion16/vm/kernel-words.md) | The 16-bit kernel — 233 words in `ENVIRO.EXE`, 232 in `BMZ.EXE`, 228 in `HPPLAY.EXE`, 204 in the oldest `LL.EXE` — its two tables, what the games use |
| [ENVIRO.EXE](motion16/engine/enviro-exe.md) | The latest build of the player: the MZ binary, what lives where |
| [HPPLAY.EXE](motion16/engine/hpplay-exe.md) | The second build: five words fewer, every ordinal from 124 up shifted |
| [BMZ.EXE](motion16/engine/bmz-exe.md) | The third build: one word fewer, and no ordinal moved |
| [LL.EXE](motion16/engine/ll-exe.md) | The oldest build: a load image that starts elsewhere, and a domain table that binds at 102 |
| [Boot and frame loop](motion16/engine/game-loop.md) | `RUN`, `SCRCTRL`, `ANIMPLAY`, location changes, saving and loading, shutdown |
| [Descriptors](motion16/engine/descriptors.md) | The scene graph: what a descriptor shows, the setters, the structures as far as read |
| [Screens and the draw chain](motion16/engine/screens.md) | Screens, the graphics mode, and how the drawer gets from a descriptor to pixels |
| [Transitions](motion16/engine/transitions.md) | `FADEIN` and `FADEOUT`, and the frames a curtain occupies |
| [Interaction](motion16/engine/interaction.md) | The pointer and the keys, the figure's walk, the inventory bar and the order table |
| [Text rendering](motion16/engine/text-rendering.md) | The drawer's two passes, the gaps, justification |
| [Off-screen buffers](motion16/engine/buffers.md) | `BUFON`, `SETBUF`, `SDBUF`, `KILLNBUF` — load-bearing here |

### Dunkle Schatten 2

| Page | Covers |
|---|---|
| [The game](games/ds2/README.md) | What it is about, who made it, and why it exists |
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
| [The game](games/enviro/README.md) | What it is about, who made it, and why it exists |
| [Game structure](games/enviro/game-structure.md) | Setting, the location scheme, the three module series, verbs, saving |
| [Module map](games/enviro/module-map.md) | What each of the 65 script modules does |
| [Resource inventory](games/enviro/inventory.md) | What `DATA.-1-` holds, by the numbers |
| [Other files](games/enviro/other-files.md) | Sound setup and drivers, the launcher, readme, leftovers |

### Jeff Jet - Abenteuer InfoHighway

| Page | Covers |
|---|---|
| [The game](games/jeffjet/README.md) | What it is about, who made it, and why it exists |
| [Game structure](games/jeffjet/game-structure.md) | Setting, the thirteen locations, the three module series, verbs, saving |
| [Module map](games/jeffjet/module-map.md) | What each of the 55 script modules does |
| [Resource inventory](games/jeffjet/inventory.md) | What the two `DATA.-n-` volumes hold, by the numbers |
| [Other files](games/jeffjet/other-files.md) | The launcher, the sound stack, the two splash pictures |

### Hilfe für Amajambere

| Page | Covers |
|---|---|
| [The game](games/hfa/README.md) | What it is about, who made it, and why it exists |
| [Game structure](games/hfa/game-structure.md) | Setting, the twenty locations, the three module series, verbs, saving |
| [Module map](games/hfa/module-map.md) | What each of the 76 script modules does |
| [Resource inventory](games/hfa/inventory.md) | What the two `DATA.-n-` volumes hold, by the numbers |
| [Other files](games/hfa/other-files.md) | The launcher, the sound stack, the integrity chain, the readmes |

### Victor Loomes – Das Spiel

| Page | Covers |
|---|---|
| [The game](games/vloomes/README.md) | What it is about, who made it, and why it exists |
| [Game structure](games/vloomes/game-structure.md) | Setting, the thirteen locations, the two module series, verbs, saving |
| [Module map](games/vloomes/module-map.md) | What each of the 36 script modules does |
| [Resource inventory](games/vloomes/inventory.md) | What the one `DATA.-1-` holds, by the numbers |
| [Other files](games/vloomes/other-files.md) | The four-program launcher chain, `GFX.INF`, the older sound setup |

### The other MOTION games

| Page | Covers |
|---|---|
| [The other MOTION games](games/others.md) | Checker 2000, Compaq and Eddy M.: what their files are, what the readers make of them, and why the player refuses them |

### Reference

| Page | Covers |
|---|---|
| [Glossary](glossary.md) | The words this documentation uses in a fixed sense, each with the page that defines it |
| [Open questions](open-questions.md) | Everything unknown, unverified, or hypothetical, in one place — for both generations |
| [Departures](departures.md) | Every place motionvm knowingly does something else, and why |
| [motionvm's savegames](savegames.md) | The layout of the three files motionvm writes per slot, and what a version change does |
| [Verification](verification.md) | What is held against the original engine's own output, and what only against the games' files |
| [The verification method](verification-method.md) | How a capture of the original is made and held against a frame or a register stream: the emulator's settings, the reduction, `compare-frame` and `compare-dro` |
| [Debugging and diagnostics](debugging.md) | The switches, keys, reports and rigs for working on the engine: what a run says about itself, and the suites that hold it still |
| [motionvm-motion-tools](tools.md) | The command-line inspector: what it reads out of a game's containers and what it writes |

## The shipped files

### Dunkle Schatten 2

| File(s) | Format |
|---|---|
| `001.RSC`, `002.RSC`, `003.RSC` | [Resource containers](motion32/formats/container.md) |
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
| `DATA.-1-` | [Resource container](motion16/formats/container.md) — the whole game |
| `ENVIRO.EXE` | [The MOTION 16-bit engine](motion16/engine/enviro-exe.md) |
| `KIDS.BAT` | Launcher ([other files](games/enviro/other-files.md)) |
| `SOUND.EXE`, `MUSADL.DRV`, `DMABLAST.DRV`, `DMASB16M.DRV`, `DMASB16S.DRV`, `DMASB2P.DRV`, `DETECTOR.DRV` | PSM 2 sound setup and drivers ([other files](games/enviro/other-files.md)) |
| `README.TXT` | German readme (EMS setup) |
| `A.DAT`, `32RTM.EXE`, `DPMI32VM.OVL` | Unreferenced by the game ([other files](games/enviro/other-files.md)) |

### Jeff Jet - Abenteuer InfoHighway

| File(s) | Format |
|---|---|
| `DATA.-1-`, `DATA.-2-` | [Resource container](motion16/formats/container.md) — the whole game, on two volumes, packed |
| `HPPLAY.EXE` | [The second build of the MOTION 16-bit player](motion16/engine/hpplay-exe.md) |
| `HP.BAT` | Launcher ([other files](games/jeffjet/other-files.md)) |
| `SOUND.EXE`, `MUSADL.DRV`, `DMABLAST.DRV`, `DMASB16M.DRV`, `DMASB16S.DRV`, `DMASB2P.DRV`, `DETECTOR.DRV` | PSM 2 sound setup and drivers, byte-identical to the other 1995/96 games' ([other files](games/jeffjet/other-files.md)) |
| `HPLOGO.EXE`, `PROMSOFT.EXE` | Graphic Workshop splash pictures, not MOTION ([other files](games/jeffjet/other-files.md)) |

### Hilfe für Amajambere

| File(s) | Format |
|---|---|
| `DATA.-1-`, `DATA.-2-` | [Resource container](motion16/formats/container.md) — the whole game, on two volumes, stored plainly |
| `BMZ.EXE` | [The third build of the MOTION 16-bit player](motion16/engine/bmz-exe.md) |
| `AFRIKA.BAT` | Launcher ([other files](games/hfa/other-files.md)) |
| `SOUND.EXE`, `MUSADL.DRV`, `DMABLAST.DRV`, `DMASB16M.DRV`, `DMASB16S.DRV`, `DMASB2P.DRV`, `DETECTOR.DRV` | PSM 2 sound setup and drivers, byte-identical to the other 1995/96 games' ([other files](games/hfa/other-files.md)) |
| `VRCHKSUM.EXE`, `ORIGINAL.BIN`, `ORIGINAL.REP`, `ORIGINAL.SCR` | The installation's integrity chain, never run here ([other files](games/hfa/other-files.md)) |
| `CONFIG.DAT` | Installer output ([other files](games/hfa/other-files.md)) |
| `INFO.TXT`, `FREEWARE.TXT`, `LIESMICH.DOK` | German readme, license, ministry reply card |

### Victor Loomes – Das Spiel

| File(s) | Format |
|---|---|
| `DATA.-1-` | [Resource container](motion16/formats/container.md) — the whole game, on one volume, in the earlier framing |
| `LL.EXE` | [The oldest build of the MOTION 16-bit player](motion16/engine/ll-exe.md) |
| `GFX.INF` | The sprite-dimension side table, which this game ships and the later ones only name ([other files](games/vloomes/other-files.md)) |
| `LBS.BAT`, `LQ.EXE`, `LP.EXE`, `LC.EXE`, `LOGOMSFX.CMF` | The four-program launcher chain and the jingle the last of them plays ([other files](games/vloomes/other-files.md)) |
| `PSMCFG.EXE`, `MUSADL.DRV`, `DETECTOR.DRV` | The older PSM 2 sound setup and its drivers ([other files](games/vloomes/other-files.md)) |
