[← Documentation index](../../README.md)

# Module Map

*Dunkle Schatten 2 — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

The game ships 86 script modules. Their numbering is systematic: fixed
infrastructure lives below 100, and each location *N* owns up to three
modules — *100+N*, *200+N*, *300+N* — that are loaded when the location is
entered and unloaded when it is left (see
[Game structure](game-structure.md)).

## Infrastructure modules

| Module | Words | Contents |
|---|---|---|
| 2 | 263 | **Global state** — see [Globals](library/globals.md): the shared variables plus the core helpers (`+@`, `+!`, `SMDESC`, `XYL…ITEM.`, …) |
| 3 | 2 | `STARTUP` and `ENDGAME` — loaded, run, and unloaded again |
| 4 | 9 | **Boot and shell** — see [Shell](library/shell.md): `START`, the control handler `ICTRL` (the de-facto main loop), the menu (`CALLMENU`/`DO_INVSEL`), the document viewer (`SHOW_DOC`), the speech pump (`SAMPLE_TIMING`) |
| 5 | 178 | **The game library** — see [Game library](library/game-library.md): verbs, records, hit testing, inventory, task machine, `INCLLOC`, [text/speech](library/text-and-speech.md), [walking](../../motion32/engine/walking.md), the [GT animator](library/animation.md) |
| 6 | 136 | **The animation runtime** — see [Animation](library/animation.md) |
| 7 | 57 | `TI1`–`TI50`: constants 1–56 enumerating story beats (with `a`/`b`/`c` sub-beats; `TI41a` is defined twice — an original copy-paste bug). **Dead code**: nothing loads or references module 7 |
| 9, 19 | 13 | The **Karsten** walk set — see [Walking](../../motion32/engine/walking.md); module 19 is a byte-identical copy of 9 (same source at a different load base), and module 212 embeds a third |
| 10 | 0 | No words — data only (carried past the dictionary) |
| 11 | 405 | **Objects and story state** — see [Objects and flags](library/objects-and-flags.md): 74 object constants, the flag arrays, ~85 story variables, 119 dialogue records |
| 12 | 3 | One-time initialization (`DS_INIT`) — loaded, run, unloaded ([Objects and flags](library/objects-and-flags.md)) |
| 13 | 30 | **Dialogue and global verb handlers** — see [Dialogue](library/dialogue.md) |
| 14 | 1 | `_ANSIZE` only |
| 123, 130 | 0 | Entirely empty |
| 399 | 4 | A **sprite inspector from the authoring environment, shipped by accident** — see [Shell](library/shell.md); never loaded by the game |

Every word of the infrastructure modules is documented in the
[script library reference](library/game-library.md) and its sibling
pages.

## Location modules

For location *N* (see [Game structure](game-structure.md) for the location
list):

| Series | Role | Present for |
|---|---|---|
| 100+N | `LD_…` **description handlers** — one word per clickable thing in the room (`LD_TÜR` "door", `LD_GRAB1` "grave 1", …) | 101–122 |
| 200+N | **Scenes and animation**: `_SC_…` scene builders, `_AN…`/`_AC…` animation definitions, character walk sets, dialogue words; 18 of them also define the location's task handler `LTMANAGER` | 201–223, 230 |
| 300+N | The scene macro `START_MACRO` that builds the room | 301–323, 330 |

The `LTMANAGER`-defining modules are 203, 205, 206, 209–215, 217–223, and
230; the other locations run without a per-location task handler.

Module **216** implements the in-game **bulletin-board system** (the
"network" of the title) — see [The BBS](library/bbs.md).

## Save games are not modules

They look like it at the call site and they are not. `701 + slot` is a
**filename stem**, used for three files per slot; no module 701 is ever
created, and `=>PUTAS` writes every resident module into the one file
whatever number it is handed. Which modules those are is decided by
`=>GET` and `=>ERASE`, and in a running game it comes out as

```
4  2  5  6  11  13  L+100  L+200  L+300
```

— the boot set, plus the current location's three, in descriptor-slot
order. See [Savegames](../../motion32/engine/savegames.md) and [Shell](library/shell.md).

## Open questions

- The duplication of the Karsten walk set (modules 9, 19, and again inside
  212).
- The precise semantics of `=>PUTAS`/`=>GETAS` (module save/load "as" a
  given id — inferred from usage, not confirmed).

## See also

- [Script library reference](library/game-library.md) — per-word
  documentation of modules 2, 4, 5, 6, 11, 12, 13, 216
- [Game structure](game-structure.md)
- [Script modules](../../motion32/formats/script-modules.md)
- [Blocks](../../motion32/formats/blocks.md) — the parallel per-location data blocks
