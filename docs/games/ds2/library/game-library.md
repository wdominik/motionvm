[← Documentation index](../../../README.md)

# Module 5 — The Game Library

*Dunkle Schatten 2 — this page describes one of the game's own script modules. The engine it runs on is documented under [MOTION 32-bit](../../../README.md#motion-32-bit).*

Module 5 (178 words) is the shared game logic: verb dispatch, hit
testing, inventory, the task machine, location switching, and the
busy/mouse plumbing. Text and speech words are on
[their own page](text-and-speech.md); the walking words are described
with the [walking system](../../../motion32/engine/walking.md); the sequence animator
(`GT`/`DODIR`/`CALLDIR`) is under [Animation](animation.md).

## The `_ORDER` interface — how the engine calls back into Forth

The 520-byte `_ORDER` record is the contract between the native engine
and the script library. At startup the game writes **packed addresses of
module-5 words** into fixed slots of `_ORDER`; the engine's native input
processing (`DOORDER`, `MOUSEINFO`) then calls these words when the
player acts:

| `_ORDER` offset | Installed word |
|---|---|
| +32 / +44 / +56 / +68 / +80 | `CALCTAKE` / `CALCEXAMINE` / `CALCHANDLE` / `CALCUSE` / `CALCTALK` |
| +92 / +104 | `CALCINFO` / `CALCGIVE` — **in that order**; see the note below |
| +116 | `CALCLEAVE` (the second of module 5's two definitions) |
| +176 | `CALCINV` (inventory repaint) |
| +184 | `FXATMOUSE` (cursor-change callback) |
| +272 / +276 | `SETWALK` / `SETSTOP` |
| +288 / +292 | `SETBUSY` / `SETNOBUSY` |
| +300 | `FLASH_ENTRY` (inventory-slot blink) |
| +316 | `FSCANITEM` (return UI to normal; also stored in `_TONORMAL`) |
| +360 | `SETINFOTEXT` |
| +416 | `TS` (text display time) |

The engine also *writes* into `_ORDER`: the current verb (+0), objects
(+4, +8), interaction state (+12), and the per-frame input snapshot
(+140…+164, copied in by the [control handler](shell.md)).

## Verb system

Verb codes: `TAKE=1`, `EXAMINE=2`, `HANDLE=3`, `USE=4`, `TALK=5`,
`GIVE=6`, `INFO=7`, `LEAVE=8` — module 5's own constants, each a
`_PutConst` of that value. Capability bits:
`TAKEABLE=3`, `HANDLEABLE=6`, `USEABLE=10`, `TALKABLE=18`,
`LEAVEONLY=128`, with combinations `TAKE_HANDLE=7`, `TAKE_USE=11`,
`TAKE_USE_HANDLE=15` (built with the **bitwise** `|`).

### `GIVE` and `INFO` name the wrong slots

The constants say `GIVE=6` and `INFO=7`. The dispatchers `STARTUP`
installs at the verb-6 and verb-7 slots are the other way round:

```
CALCINFO -> _ORDER +0x5C   (verb 6)
CALCGIVE -> _ORDER +0x68   (verb 7)
```

So a script pushing the constant `GIVE` reaches `CALCINFO`, and `INFO`
reaches `CALCGIVE`. Both readings of the pair are in the shipped data and
they disagree — an original inconsistency, in the same family as `TI41a`
being defined twice and the dead music branch in `INCLLOC`. The engine
side is what actually runs: its case bodies are `0x7c7e8` for verb 6
(asking for the name `"DINFO"`) and `0x7ca7f` for verb 7 (`"DGIVE"`), and
`STARTUP` feeds them to match. Nothing in the shipped game reaches either
verb from a menu — no flag constant sets bit 5 or 6 — so the swap never
shows (see [Interaction machine](../../../motion32/engine/interaction.md)).

### The dispatch chain

Each `CALC*` dispatcher tries the **location handler** (`_LC_*`, set by
the location's scene module, cleared on every location change), then the
**global handler** (`_P_CALC*`, pointing into
[module 13](dialogue.md)), then a default. Return convention:
**0 = handled, 1 = continue, 2 = failed** — with per-verb wrinkles:

| Word | Chain and default |
|---|---|
| `CALCTAKE ( obj -- r )` | `_LC_TAKE` → `_P_CALCTAKE` → 2 |
| `CALCEXAMINE ( obj -- r )` | `_LC_EXAMINE`; a **0** result retries globally with `O.AktObj1` → default 0 |
| `CALCHANDLE ( obj -- r )` | `_LC_HANDLE`; a **2** result retries globally → default 2 |
| `CALCUSE ( a b -- r )` | `_LC_USE`; a **2** result retries with both objects → default 2 |
| `CALCTALK ( obj -- r )` | −1 short-circuits (leaving nothing — a stack-balance bug); else `_LC_TALK` → `_P_CALCTALK` → −1 |
| `CALCINFO` (verb 6) | `_LC_INFO` → `_P_CALCINFO` → the string `"DINFO"` (a dialogue lookup key) |
| `CALCGIVE` (verb 7) | `_LC_GIVE` → `_P_CALCGIVE` → `"DGIVE"` |
| `CALCLEAVE ( -- )` | `_LC_LEAVE` → `_P_CALCLEAVE` → `INCLLOC` of `O.NextLoc` — the default "leave" simply enters the stored location |

Result helpers for handler code: `TAKEOK`/`TAKECONT`/`TAKEFAIL`,
`EXAMINEOK`/`…CONT`/`…FAIL`, `HANDLEOK`/`…CONT`/`…FAIL` (each
`( x -- 0/1/2 )`), `USEOK`/`USECONT`/`USEFAIL` (`( a b -- 0/1/2 )` —
USE consumes two objects).

### `ORDERMAKE ( -- )`

Repairs the capability masks of all 35 location items after a scene
sets them up: items with an `EXIT` field become `LEAVEONLY|2`,
talk-only items get the hotspot bit added, everything else defaults to
`HANDLEABLE`.

## Record accessors

`.LDITEM. ( n -- addr )` = `n × 64 + _LDITEM` — the location-item
records; `.FITEM. ( n -- addr )` = `n × 20 + _FITEM` — the object
records. Around them sit paired setters/getters per field
(`->LDX1`/`LDX1->` etc. — the field table is in
[Blocks](../../../motion32/formats/blocks.md)) and two bulk setters:

- `->LDALL ( text x1 y1 x2 y2 dr dx dy n -- )` — fill a location item
  (does **not** set the info text; callers follow with `->LDITEXT`).
- `->FALL ( name order gfx info dir n -- )` — fill an object record
  (how every inventory object is registered; see
  [Objects and flags](objects-and-flags.md)).
- `->HITBOX ( item descvar -- )` — copy a descriptor's bounding box
  into an item's click rectangle, keeping a character's clickable area
  glued to its sprite.

## Hit testing and the mouse

- `SCANITEM ( -- )` hands the complete UI state to the native
  `MOUSEINFO` word — a **24-argument** call passing the mouse
  coordinates, the item and object tables with their geometry, the
  info/mouse-over descriptors, the three cursor sprites (390 normal,
  391, 392), the cursor callback `FXATMOUSE`, and the current
  interaction state (see
  [Interaction machine](../../../motion32/engine/interaction.md)). The result (the
  item under the cursor) lands in `_SCANITEM`.
- `FSCANITEM ( -- )` is the "force back to normal" variant (installed
  as `_TONORMAL`); outside the free-play state it just blanks the
  mouse-over line.
- `FORCE_SCANITEM ( -- )` invalidates the last mouse position so the
  next frame rescans.
- `FXATMOUSE ( nr x y -- )` / `FATMOUSE ( nr -- )` set the cursor
  sprite (remembering it in `_BMNR/_BMX/_BMY`); `REST_MOUSE` restores
  it.
- `LDON ( n -- )` / `LDOFF ( n -- )` enable/disable a hotspot by
  **shifting its rectangle by ±10000 pixels** — the most-called pair of
  words in the entire game (34 and 30 modules).

## Busy state

`SETBUSY` / `SETNOBUSY` maintain a nesting counter `_BUSY`: entering
busy raises `_SYS_LEVEL` to 1, hides the mouse-over line, and switches
to the hourglass cursor (sprite 394); leaving restores cursor 390,
re-blanks the info line, and forces a rescan. Over- or underflow of the
counter prints a debug report.

Only the **edges** do any of that: `SETBUSY` acts when the count reaches 1
and `SETNOBUSY` when it reaches 0, so the two are safe to nest. A location
entry nests them exactly once:

```
INCLLOC           SETBUSY     0 -> 1
3xx:START_MACRO   SETBUSY     1 -> 2      (locations that open with a scene)
INCLLOC (tail)    SETNOBUSY   2 -> 1
2xx:LTMANAGER     SETNOBUSY   1 -> 0      (the scene's last phase)
```

so a cutscene holds exactly one raise, and `_SYS_LEVEL` stays at 1 — which
is what closes the menu block behind `_SYS_LEVEL @ 1 <` (module 4, 0x02a40)
while it runs. Entering a location twice without letting its scene finish
orphans a raise and locks the menu for good; that is a way of driving the
game, not a state it reaches by itself (see
[Transitions](../../../motion32/engine/transitions.md) for the fades in the same phases).

## Inventory

- `ADDITEM ( obj -- )` / `SUBITEM ( obj -- )` — add/remove an object id
  in `_GAMEINV` (wrappers around the kernel's `ADDTOINV`/`SUBFROMINV`).
- `CALCINV ( -- )` — repaint the inventory bar: passes the scroll
  offset, object table, arrow/slot descriptors, and the `FLASH_ENTRY`
  callback to the kernel's `CCALCINV`, then `CALCARROWS`. (An earlier,
  empty definition of `CALCINV` sits dead in the dictionary.)
- `CALCARROWS ( -- )` — arrow highlight: sprites 15/16 (up) at x = 0,
  17/18 (down) at x = 32, switching on mouse position.
- `FLASH_ENTRY ( -- )` — blinks the selected slot by toggling sprites
  13/14 (13 is also the empty-slot graphic).

## Task machine

| Word | Effect |
|---|---|
| `SETLOCTASK ( n -- )` | arm task `n`, phase 0, wait 0; set the "task armed" flag `_ORDER+460` |
| `?LOCTASK ( n -- f )` | is task `n` active? |
| `??LOCTASK ( -- f )` | a task is active with id < 1000 (location task) |
| `??GLOBTASK ( -- f )` | a task is active with id > 1000 (global task — see [Dialogue](dialogue.md)) |
| `?LTPHASE ( -- n )` / `->LTPHASE ( n -- )` | read/set the phase |
| `?LTWAIT ( -- n )` / `->LTWAIT ( n -- )` / `!LTWAIT ( -- )` | read/set/decrement the wait counter |
| `NEXTLTP ( -- )` | advance to the next phase |
| `FINISH_LT ( -- )` | clear task, phase, wait, and the armed flag |

Task id exactly 1000 belongs to neither class.

## Location switching — `INCLLOC ( n -- )`

The full sequence is described in
[Game structure](../game-structure.md). Two details worth
pinning here:

- The cache-eviction hints leave deliberate gaps: sprites 20–21,
  101–144 (the protagonist's walk cycle), 282–295, **390–394 (the
  cursors)**, and text tables 1–10 are never evicted.
- The music-start branch at the end is **dead code**: its location-range
  test (`≥ 80 AND ≤ 40`) can never be true, so `INCLLOC` never starts
  music — all music starts live in the scene macros. If the branch ever
  ran it would pick a random tune 60–63, which do not exist. A genuine
  bug in the shipped game.

## Save/menu overlay helpers

- `SHOW_FILES ( -- )` — probes save slots **701–705** with `EXIST`, which is a plain file test on `NNN.blk`
  and fills `_LOADTABLE`; lights one star descriptor (sprites 23–27)
  per occupied slot.
- `REMOVE_FILES ( -- )` — hides the five stars.
- `KILL_MENU ( -- )` — unfreeze the main screen, curtain the status
  bar, restore the inventory icon, fade back in.
- `DOINVBACK ( -- )` — leave a submenu back to the main menu overlay
  (or finish a restore).
- `DOINV2` — an older menu hit-tester superseded by the
  [control handler](shell.md); dead code.

## Open questions

- `FOLLOWMAN`'s entire body is `SDINACTIVE` — and that is not vestigial:
  its **address** is the expiry callback of the `_TI1` caption
  descriptor (passed as `NEWSETDESC`'s sixth argument), the word that
  makes a caption hide itself when its display time runs out. No script
  calls it by name, which long disguised its role.
- `CALCTALK`'s −1 arm returns one value fewer than the other arms.
- Module 5 shadows several module-2 words with identical definitions
  (`TAKEABLE` and friends, `WALKQUEUE`, `_ZW`) — code compiled later
  binds to the module-5 copies.
- `PSINITFIG` reads its queue through the variable *address* instead of
  its contents — inconsistent with every sibling word; likely an
  original bug that happens to be harmless at its single call site.

## See also

- [Globals](globals.md) — the state this library operates on
- [Text and speech](text-and-speech.md), [Animation](animation.md),
  [Walking](../../../motion32/engine/walking.md)
- [Dialogue](dialogue.md) — the global verb handlers
- [Game structure](../game-structure.md)
