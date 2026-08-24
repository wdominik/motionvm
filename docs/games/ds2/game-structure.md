[← Documentation index](../../README.md)

# Game Structure

*Dunkle Schatten 2 — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 32-bit](../../README.md#motion-32-bit-ds2).*

How the game's 86 script modules organize into a running adventure: the
startup sequence, locations and the location loader, the story-flag
system, and the save mechanism. For what each module contains, see the
[module map](module-map.md).

## Startup

The bootstrap loads module 4 and runs its `START` word, whose body begins:

```
2 =>GET  5 =>GET  6 =>GET  11 =>GET  13 =>GET  3 =>GET   STARTUP   3 =>ERASE  …
```

— load the resident modules (2 globals, 5 game library, 6 animation, 11
inventory, 13 dialogue), load module 3, run its `STARTUP`, and unload
module 3 again once initialization is done. In order, `STARTUP`:

- Queries `?SOUND` (stores the result in `_SPEECH`).
- Resets animation and font state, registers fonts:
  `5 +FONT _F1 !`, `6 +FONT _F2 !`, `8 +FONT _F3 !`.
- Defines the nine text templates: for every template number 1–9 the same
  argument tuple is pushed — `8 2 2 -1 -1 -1 -1 <font-6 handle> n DEFTDT`.
  (What the eight values mean is still open; only the tuple itself is
  known.)
- Sets the video mode: `640x480x256 SETRES TOGFX`.
- Builds the **status-bar screen** (640×80, shown at display row 400) and
  populates it — see below.
- Builds the **main screen** (640×400 at (0, 0)) and the off-display
  dialogue surface, and creates the info/text descriptors on them.
- Loads the location table: `200 _LOCTABLE 99 GET` (see
  [Blocks](../../motion32/formats/block.md)).
- Installs the script callbacks into the `_ORDER` interface record — the
  addresses through which the native engine calls back into the game
  library (see [Game library](library/game-library.md)).

### The status bar

`STARTUP` lays the status bar out with concrete sprites and positions
(`XYLSITEM. ( x y level sprite -- handle )` for sprites, `XYLBITEM.` for
full images):

| Element | Position | Sprite/image |
|---|---|---|
| Inventory scroll arrows | (0, 16), (32, 16) | 15, 17 |
| Eight inventory slots | x = 64, 128, …, 512, y = 16 | filled at run time |
| Busy indicator | (112, 6), level 1 | 27 |
| Five stars (score display) | x = 239, 319, 399, 479, 559, y = 8 | 22–26 |
| Inventory icon | (576, 0) | image 20 |
| Overlay images | (0, 0), (320, 0) | 28, 29 |
| Selection frames | level 2 | 44 |

## Locations and the location loader

Locations are numbered from 1. Location *N* owns modules *100+N*
(descriptions), *200+N* (scenes/animation/task handler), *300+N* (the
scene macro) — see the [module map](module-map.md) — and four data blocks
(routes, extended routes, click areas, items) — see
[Blocks](../../motion32/formats/block.md).

`INCLLOC`, the location switch in module 5, performs:

1. `SETBUSY`; clear `_LTHANDLER` and `_ANHANDLER`.
2. Issue cache-eviction hints for sprite, script, block, palette, and text
   ranges (`XGFXSTAT-`, `XSCRSTAT-`, `XBLKSTAT-`, `XPALSTAT-`,
   `XTXTSTAT-`).
3. If a location is active: blank the info texts, stop the music
   (`_ACTMUSIC @ ENDTUNE`), remove the old room's descriptors
   (`_BG @ KILLNDESC`), fade out (`1 50 8 FADEOUT` — skipped during
   startup), and **unload** the old location's three modules
   (`=>ERASE` on *100+N*, *200+N*, *300+N*); reset the per-verb handler
   variables (`_LC_HANDLE`, `_LC_TAKE`, `_LC_EXAMINE`, `_LC_TALK`,
   `_LC_USE`, `_LC_GIVE`, `_LC_INFO`, `_LC_LEAVE`); remember the old
   location in `_LASTLOC`.
4. **Load** the new location's three modules (`=>GET`), then copy its four
   data blocks into module-2 arrays with `GET`.
5. Execute the scene macro through the location table:
   `(N−1)·4 _LOCTABLE + @ EXECUTE` — the table is 1-based.
6. If the screen is inactive (it is, after a fade-out): fade the new room
   in (`1 50 8 FADEIN`).

Location changes are *requested* by storing the target into `_NEXTLOC`
(module 2); the native loop consumes it (see [Game loop](../../motion32/engine/game-loop.md)).
Literal targets used by the scripts: locations 1, 2, 5, 6, 9, 13, 15, 16
(other changes compute their target).

### The scene macro

Each location module *300+N* defines a single word, `START_MACRO`:

```
0 0 2 1010 0 0 NEWSETDESC _BG ! SDACTIVE     \ background, level 2
334 159 80 1011 0 0 NEWSETDESC DROP 1011 SDSPR
…
```

Backgrounds sit on low levels, items and characters on higher levels (see
[Descriptors](../../motion32/engine/descriptors.md)). After the scene is built, `?SLDONE` story
flags decide which sequences trigger; on a fresh game all flags are unlit
and the intro sequences run. In location 1 one of them begins with a
full-screen black fade, so a still frame of that room is legitimately
almost entirely black.

## Location 23 — the title macro

Location 23 is not a room but the **title sequence**:

- Shows sprite 86 (the 320×200 title logo) scaled to 200 %
  (`2000 SD%SHR`), filling the 640×400 main screen.
- Sets its background descriptor explicitly `SDINACTIVE`.
- Uses palette 96 for the logo and palette 91 for the title artwork.
- Its task handler `LTMANAGER` lives in module 223 at byte offset
  `0x19c8`.

One intro phase is, verbatim:

```
FADEOUT  _BG @ ACTDESC SDACTIVE  _BG2 @ ACTDESC SDINACTIVE
91 SETPAL  FADEIN  NEXTLTP
```

— a complete image-and-palette swap inside a single blocking sequence (see
[Transitions](../../motion32/engine/transitions.md)).

The intro text comes from table 6: entry 108 is the four-line production
credit, entry 109 the seven-line backstory (selected as `109 SDTXT` and
`110 SDTXT` — the indexing is 1-based, see
[Text rendering](../../motion32/engine/text-rendering.md)).

The intro ends by handing over to the first playable room:

```
1 _KINTRO !   2 _NEXTLOC !
```

Location 2 is the park.

## Story flags

The story state is an array of cells, `_SLDONE`, in the inventory module
(module 11), addressed by flag number:

```
: ?SLDONE   ( n -- f )   4 * _SLDONE +@ ;
: ->SLDONE  ( n -- )     DUP _SL !  4 * _SLDONE +  1 SWAP !  … ;
```

`->!SLDONE` clears a flag; parallel families exist for dialogue-done
flags (`?DDONE`/`->DDONE`) and hint flags. Flag numbers observed with
literal arguments run from 1 to 110; many flags are queried in one place
and set in another with computed values, so the literal lists (37 queried,
20 set) are a lower bound on the flags in use.

## Saving and loading

The game menu has **five slots, numbered 701–705** — not modules, but
filename stems for three files each: a 4-byte block holding the location
number (`PUT`), the descriptor tree (`PUTANIM`), and the memory of every
resident module (`=>PUTAS`). Loading reads the location number first,
enters that location the ordinary way, and only then lays the saved
memory over it. Persisting module memory captures all the variables at
once — which is exactly the state of a Forth game whose variables live
there. The exact flow is in [Shell](library/shell.md), the file
formats in [Savegames](../../motion32/engine/savegames.md).

## No scripted pauses

The task system has a wait counter (`_LOCTASKWAI`), but across all 86
modules it is never set to a positive value — only reset, tested, and
decremented. All pacing in the original comes from load times and the
blocking transition/animation words, not from scripted delays. See
[Game loop](../../motion32/engine/game-loop.md).

## Open questions

- The meaning of `DEFTDT`'s eight per-template arguments (the values are
  known: `8 2 2 -1 -1 -1 -1 <font>`).
- The full story-flag assignment (which flag means which plot event).
- The stale location-table entries (location 12; module 330) — see
  [Blocks](../../motion32/formats/block.md).
- The exact semantics of `=>PUTAS`/`=>GETAS`.

## See also

- [Module map](module-map.md)
- [Game loop](../../motion32/engine/game-loop.md)
- [Screens](../../motion32/engine/screens.md), [Descriptors](../../motion32/engine/descriptors.md)
- [Blocks](../../motion32/formats/block.md) — the location table and data records
