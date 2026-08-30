[← Documentation index](../../../README.md)

# Module 4 — Boot, Control Handler, and Menus

*Dunkle Schatten 2 — this page describes one of the game's own script modules. The engine it runs on is documented under [MOTION 32-bit](../../../README.md#motion-32-bit).*

Module 4 (9 words) is the game's shell: the bootstrap `START`, the
per-frame control handler `ICTRL` (the de-facto main loop — see
[Game loop](../../../motion32/engine/game-loop.md)), the menu system, the document
viewer, and the speech pump.

## START — the bootstrap

```
2 =>GET  5 =>GET  6 =>GET  11 =>GET  13 =>GET      \ resident modules
3 =>GET  STARTUP  3 =>ERASE                        \ init, then discard
12 =>GET  DS_INIT  12 =>ERASE                      \ item setup, discard
0 ERRORLEVEL   1 SPEEDMODE
<ICTRL> CTRL                                       \ install the control handler
25 DELAY   1 SETPAL   390 FATMOUSE
_STARTLOC @ 1 = IF … THEN                          \ (not taken; _STARTLOC = 23)
1 2 3 4 5 6 7 8 9 10 ANIMPLAY                      \ ← the game runs in here
3 =>GET  ENDGAME  3 =>ERASE
```

The essential trick: `CTRL` registers `ICTRL` as the engine's control
callback, and then **the whole game runs inside one blocking `ANIMPLAY`
call** — the engine's internal frame loop invokes the callback every
frame. When `ANIMPLAY` ever returns, the game is over and `ENDGAME`
runs. The skipped branch would auto-open the load menu when save games
exist and `_STARTLOC` is 1 (a development configuration; shipped is 23,
the title).

## ICTRL — the per-frame control handler

3236 cells; everything interactive funnels through here, once per
frame:

1. **Input sampling** — `?KEY` into `_AKTKEY`; mouse into `_MX/_MY`
   (with previous values in `_LMX/_LMY`), split into picture-area
   coordinates (`_MMX/_MMY`, −1 outside) and status-bar coordinates
   (`_IMX/_IMY`, y−400); buttons via `MOUSELK`/`MOUSERK` into
   `_MLK`/`_MRK`; `_MPRESSED` implements the click debounce.
2. **Click-to-advance** — a fresh click outside dialogue states resets
   the wait of the caption descriptors (`_IINFO`, `_TI1`, `_TI2`).
3. **Free-play input** (no menu open): arrow highlighting, inventory
   scrolling (±8 per click on the arrows), `?WALK` on picture-area
   clicks, and Escape / the inventory icon opening the menu. Then
   `SCANITEM`, copying the input snapshot into `_ORDER` (+140…+164),
   and dispatching the player's order through the engine's `DOORDER`
   (bracketed by a re-entrancy counter).
4. **Menu modes** (`_INVMODE` selects the UI state): hit boxes for the
   six main-menu buttons, the five save/load slots
   (`slot = (_IMX−239)/80`), the 20 document tabs, the quit
   confirmation, and the options rows.
5. **Per-frame tail** — keep the inventory icon's image current; **if
   no location is active yet, enter `_STARTLOC`** (this is how the game
   reaches the title); run `GLOBTASK` and the location's `_LTHANDLER`;
   consume `_NEXTLOC` (enter the location, clear the variable); run
   `_ANHANDLER` (the [animation driver](animation.md)); update the
   debounce latch.
6. **Debug layer** (`_DEBUGON`): number keys teleport to locations
   1–22; `m` shows the mouse position; an overlay shows task/busy
   state and the current order; `g` opens a numeric sprite viewer; `i`
   opens a **live Forth input line** (fed to the kernel's
   `INTERPRET$`); one key toggles a 320×200 debug screen mode.

## The menu (CALLMENU and DO_INVSEL)

`CALLMENU ( n -- )` starts a button-press animation: it stores the
button code in `_MENCZW`, puts the highlight sprite (44/45) on the
button, and registers `DO_INVSEL` as the descriptor's completion word.
`DO_INVSEL` then performs the action:

| Button | Action |
|---|---|
| Continue | close the menu (`KILL_MENU`), back to the game |
| Load | overlays 30/31, `SHOW_FILES`, save-slot UI (`_INVMODE 3`) |
| Save | the same with the save marker (`_INVMODE 4`) |
| Documents | overlays 56/57, page 1, `SHOW_DOC` (`_INVMODE 5`) |
| Options | overlays 58/59; highlight bars at `517+(mode−1)·40` for text and walk speed (`_INVMODE 7`) |
| Quit | overlays 52/53, confirmation (`_INVMODE 6`); confirming fades out and shows the ending screens (palette 93, image 64, texts from table 6) |

Options map: text speed `_TSMODE` 1/2/3 → `_TSPEED` 150/250/400; walk
speed `_GSMODE` 1–3 → `STEPMULTI`.

### When the menu fades, and on which screen

Worth setting out, because it is the densest concentration of fades in the
game and because almost all of them go **one way**. `CALLMENU` and `SHOW_DOC`
fade nothing themselves; the fades are in `DO_INVSEL`, in `ICTRL`'s own
`_INVMODE` branches, and in module 5's `KILL_MENU`/`DOINVBACK`/`DOINV2`.

| Moment | Fades | Screen |
|---|---|---|
| Opening the bar (`_IMX ≥ 576`, 0x02dc0) | `FREEZESCR` on the picture, then `FADEOUT`, `FADEOUT`, `FADEIN` | the bar |
| Escape → quit page (0x02c40) | `FREEZESCR`, `FADEOUT`, `FADEIN` | the bar |
| Load / save / options page (0x01af4, 0x01bb4, 0x01dec) | `FADEIN` only | the bar |
| Documents page (0x01c44–0x01ca8) | `FADEIN`, `FADEOUT`, `FADEIN` | bar, then picture, then picture |
| **Turning a help page** (0x03bc0/0x03be8, 0x03cb4/0x03cdc) | `FADEIN`, `FADEIN` — **no `FADEOUT`** | bar, then picture |
| Moving an options slider (0x03ef8, 0x040a4) | `FADEIN` only | the bar |
| Clicking a save slot (0x036a0, 0x0373c) | `FADEIN`, write, `FADEIN` | the bar |
| Closing the menu (`KILL_MENU`) | `UNFREEZESCR` on the picture, `FADEOUT`, `FADEIN` | the bar |

Two things follow. First, the screen a fade works on is whatever `ACTSCR`
last selected, and several of these sites do not select one at all — they
inherit it, and `SHOW_DOC` ends by selecting `_IS` (0x014b0), so everything
after it inherits the bar. Second, a bare `FADEIN` reveals its picture over
what is already on screen; nothing here blanks anything. See
[Transitions](../../../motion32/engine/transitions.md).

`DO_INVSEL` is also the one place several fades run back to back, because it
arrives as a descriptor callback rather than from `ICTRL` directly.

One asymmetry with module 13's viewer, recorded and not "fixed": `GLOBTASK`'s
document reader (task 1100) does `FADEOUT` → `SETPAL` (191–197, one per page)
→ `FADEIN` on every page turn, while module 4's help pages have no per-page
palette at all and never fade out. Both are the original.

## Saving and loading (the exact flow)

Save (slot 0–4, clicked in `_INVMODE 4`):

```
4 _AKTLT (slot+701) PUT            \ 4 bytes: the current location number
(slot+701) DUP PUTANIM  DUP =>PUTAS  DROP
SHOW_FILES  1 _DOSAVE !
```

Load:

```
4 _AKTLT (slot+701) GET            \ read the saved location number
1 _?STARTUP !  _AKTLT @ INCLLOC  0 _?STARTUP !
(slot+701) DUP GETANIM  DUP =>GETAS  DROP
_GSMODE @ STEPMULTI
```

A save game is therefore three **files** whose names are built from
`701+slot` (five slots, 701–705):

| file | written by | holds |
|---|---|---|
| `NNN.blk` | `PUT` | four bytes: the location number |
| `NNN.anm` | `PUTANIM` | the descriptor tree, five globals and a 180-byte table |
| `NNN.FRZ` | `=>PUTAS` | the memory of every resident module |

The module image is where the substance is: it captures every variable
without anything enumerating them, because script variables live in
module memory. That is why the flow above names exactly one value by
hand — the location number, which loading needs *before* it can restore
anything, so that `INCLLOC` has put the right modules in place first.

`701+slot` is a filename stem and nothing more. **No module 701 is ever
created**, and `=>PUTAS` writes *all* resident modules into the one file
regardless of the number it is given. The only sense in which 701 is a
resource id is that `PUT` registers it in the engine's BLOCK catalog for
the four-byte `.blk`. `SHOW_FILES` finds occupied slots by asking `EXIST`,
which is a plain file test on `NNN.blk` and answers −1 or 0.

See [Savegames](../../../motion32/engine/savegames.md) for the file formats, and
[Departures](../../../departures.md) for what motionvm does differently.

## SHOW_DOC — the document viewer

A flat repainter for 20 help/document pages in two tab rows: it
deactivates all twenty text/graphic descriptors, rebuilds the chrome
(sprite 61, running title from text table 6 entry `page+71`, a
right-aligned chapter heading), then re-activates exactly the
descriptors the current page needs with hard-coded positions — pages mix
text entries 39–71 with illustration sprites (287–299, 390–394, 623).

### What a page is made of

Three layers, and only the middle one belongs to `SHOW_DOC`:

| Layer | Level | Where from |
|---|---|---|
| The paper — image **60** twice, at (0,0) and (320,0) | 128 | `_ANL1`/`_ANL2`, activated by **`DO_INVSEL` case 1005** |
| Sprite **61** at (20,35) — a 600×3 rule of index 9, not a background | 129 | `SHOW_DOC` |
| The texts and illustrations | 129 | `SHOW_DOC` |

Image 60 is a plain 320×400 field of **index 2**; two of them tile the
640×400 picture. Sprite 61 is the line under the heading and nothing
more.

The split matters for anything driving this from outside: **setting
`_INVMODE` to 5 directly opens the viewer without its paper**, because
the paper is switched on by `DO_INVSEL`, not by `SHOW_DOC`.

`SHOW_DOC` contains **no `SDCOL`, no `SDTDT` and no `SDFNT`** — 36 ×
`6 SDTB` and nothing about appearance. Its texts therefore run on the
descriptor's zeroed color field, index 0, black on the paper, with no
outline; see [Text rendering](../../../motion32/engine/text-rendering.md). The one
script in the game that ever colors one of these descriptors is
`223:LTMANAGER`, which sets `_ANT1` white for the title text and back to
0 when the title is over.

## SAMPLE_TIMING — the speech pump

The per-frame speech driver (10 speaker slots, cue table, `?STIME`
polling). Fully described under [Audio](../../../motion32/engine/audio.md); dead in
the shipped game.

## Module 399 — a shipped authoring tool

Module 399 (4 words: `_ITEM`, `_SCR`, `ICTRL`, `SHOW`) is a **sprite
inspector from the authoring environment, shipped by accident**: its
`SHOW ( item -- )` cuts an image via the authoring word `XYCUT` and
displays it; its own `ICTRL` stub replaces the game's handler (Escape
quits). It is never loaded by the game.

## See also

- [Game loop](../../../motion32/engine/game-loop.md) — how ICTRL fits the frame cycle
- [Game library](game-library.md) — the words ICTRL calls
- [Audio](../../../motion32/engine/audio.md) — SAMPLE_TIMING
- [Module map](../module-map.md)
