[← Documentation index](README.md)

# Glossary

*The words this documentation uses in a fixed sense, for both generations of the engine, each with the page that defines it. A term that belongs to one generation says so.*

- **Band** — one step of a curtain: the rows a `FADEOUT` or `FADEIN` fills
  per tick, and the unit a step of the engine is while one runs.
  [Transitions](motion32/engine/transitions.md), [16-bit](motion16/engine/transitions.md).
- **Block** — a resource of the `Block` segment: mixed binary data — a song,
  an animation catalog, a location's tables. [32-bit](motion32/formats/blocks.md),
  [16-bit](motion16/formats/blocks.md).
- **Build** — one shipped binary of the 16-bit engine: `ENVIRO.EXE`,
  `HPPLAY.EXE`, `BMZ.EXE`, `LL.EXE`, each with its own kernel table.
  [Kernel words](motion16/vm/kernel-words.md).
- **Capability** — a named boolean on the [profile](#profile) that says how a
  build behaves where builds differ, each measured at a disassembly address.
  `ARCHITECTURE.md`.
- **Cell** — the unit of the Forth machine: 32 bits on the 32-bit engine, 16
  on the 16-bit one. [Execution model](motion32/vm/execution-model.md).
- **Container** — the resource archive a game ships: `NNN.RSC` banks on the
  32-bit engine, `DATA.-n-` volumes on the 16-bit one.
  [32-bit](motion32/formats/container.md), [16-bit](motion16/formats/container.md).
- **Contract** — `motionvm-playable`: what the window asks of a game, and
  the only thing that crosses between them. [Writing a family](../writing-a-family.md).
- **Core table, domain table** — the two kernel word tables of the 16-bit
  engine, ordinals 1–82 and 105 up. [Kernel words](motion16/vm/kernel-words.md).
- **Curtain** — the fade `FADEOUT` and `FADEIN` draw, band by band, over the
  active screen. [Transitions](motion32/engine/transitions.md).
- **Departure** — a place where motionvm knowingly does something else than
  the original, recorded with its reason. [Departures](departures.md).
- **Descriptor** — one thing on a screen: a sprite, an image, a text, or a
  group, with a position, a level and a wait; the scene graph is a list of
  them. [32-bit](motion32/engine/descriptors.md), [16-bit](motion16/engine/descriptors.md).
- **Digest** — a hash of what a suite composed or played, held against a
  table in the tree so that nothing moves unnoticed. [Debugging](debugging.md),
  [Verification](verification.md).
- **Domain table** — see *core table*.
- **DRO** — DOSBox-X's recording of every OPL register write, the file a
  register stream is held against. [The verification method](verification-method.md).
- **Family** — an engine and its games behind one implementation of the
  contract; MOTION is the one there is. `ARCHITECTURE.md`.
- **Framing** — which of the two 16-bit container headers a file has, sixteen
  bytes apart: the earlier one of 1993/94, the later one of 1995 on.
  [The DATA container](motion16/formats/container.md#the-earlier-framing).
- **`.FRZ`, `.anm`, `.blk`** — the three files a savegame slot is made of:
  the resident modules' memory, the display state, the location number.
  [Savegames](motion32/engine/savegames.md), [motionvm's savegames](savegames.md).
- **F12 picture** — the frame the window writes as an indexed PNG, the bytes
  the engine composed, for a comparison against the original.
  [Debugging](debugging.md), [The verification method](verification-method.md).
- **Generation** — which of the two engines made a game: MOTION 32-bit
  (`ENGINE.EXE`) or MOTION 16-bit (the four builds). [Index](README.md#the-engine-and-its-two-generations).
- **GFXCRUNCH** — the LZW codec both generations' packed sprites, fonts and
  containers use. [The GFXCRUNCH LZW codec](formats/lzw.md).
- **HMI** — the music format of the 32-bit games, inside blocks, played
  through the OPL3 driver of `HMIMDRV.386`. [HMI songs](motion32/formats/hmi.md),
  [The FM driver](motion32/engine/fm-driver.md).
- **Hot area** — a rectangle of a location's item table the pointer is
  tested against, standing for an item or an exit. [Interaction](motion32/engine/interaction.md).
- **Kernel word** — a word the engine binary implements natively, called
  from bytecode by ordinal; the tables list them. [32-bit](motion32/vm/kernel-words.md),
  [16-bit](motion16/vm/kernel-words.md).
- **Ledger** — one of the family's three records kept in one place each:
  [open questions](open-questions.md), [departures](departures.md),
  [verification](verification.md).
- **Location** — one place of a game, entered by number through its
  location table, with a scene macro and a task manager of its own.
  [Game structure (Dunkle Schatten 2)](games/ds2/game-structure.md).
- **Macro** — a location's scene word, run on entry: it builds the screens
  and descriptors and starts the tune. [Module map](games/ds2/module-map.md).
- **Module** — one compiled Forth script, numbered, loaded by `=>GET`; a
  game is a set of them. [Script modules](motion32/formats/script-modules.md),
  [16-bit](motion16/formats/script-modules.md).
- **Occupancy** — the per-slot word of a 16-bit container that says which
  volume a resource is on, or whether it is there at all.
  [The DATA container](motion16/formats/container.md#occupancy-table-offset-38).
- **Open question** — something unknown, unverified or hypothetical about
  the original, recorded until it is read. [Open questions](open-questions.md).
- **Order block** — `_ORDER`, the record the interaction machine runs on:
  the verb, the target, the mode, the pointer snapshot, the conversation
  state. [Interaction](motion32/engine/interaction.md).
- **Ordinal** — the number a kernel word is called by from bytecode, an index
  into its table. [Threaded code](motion32/vm/threaded-code.md).
- **Profile** — what a build decides when a game opens: the display, the
  capabilities, the savegame layout; built whole before the engine exists.
  `ARCHITECTURE.md`.
- **PSM 2** — the music format of the 16-bit games, modules played through
  `MUSADL.DRV`. [PSM 2 music](motion16/formats/psm-music.md).
- **Relocated image** — `ENGINE.EXE`'s LE image with its fixups applied, the
  address space every 32-bit address in this documentation refers to.
  [ENGINE.EXE](motion32/engine/engine-exe.md).
- **Roster** — the list of games a family plays, from which the window's
  help and its refusals are built. [Index](README.md).
- **Screen** — a drawing surface of a given size and origin, one of several
  a game keeps: the scene, the inventory bar, the dialogue. [Screens](motion32/engine/screens.md),
  [16-bit](motion16/engine/screens.md).
- **Segment** — one of the seven resource kinds of a 16-bit container — gfx,
  blk, scr, pal, fnt, frt, txt — each with its own slots.
  [The DATA container](motion16/formats/container.md).
- **Slug** — the key a game's files are kept under: `ds2`, `enviro`,
  `jeffjet`, `hfa`, `vloomes`; it names a directory, a variable, a page,
  never a sentence. `CONTRIBUTING.md`.
- **Speaker table** — the conversation's list of speaking figures, each with
  the word its animation hears and its colors. [Dialogue machine](motion32/engine/dialogue-machine.md).
- **Task manager** — a location's `LTMANAGER`: the script that runs the
  scene's phases frame by frame, waiting in ticks of the frame.
  [Game structure (Dunkle Schatten 2)](games/ds2/game-structure.md).
- **Tick** — the engine's unit of time: 200 a second by its own arithmetic,
  eight to a frame at 25 frames a second. [Game loop](motion32/engine/game-loop.md).
- **Verb** — one of the eight actions the verb menu offers on a thing, by
  number; a verb menu is a strip of their icons. [Interaction](motion32/engine/interaction.md).
- **Volume** — one `DATA.-n-` file of a 16-bit container, of up to three.
  [The DATA container](motion16/formats/container.md).
- **Word** — a Forth definition: a kernel word the binary implements, or a
  script word a module defines. [Execution model](motion32/vm/execution-model.md).
