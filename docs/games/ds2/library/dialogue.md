[← Documentation index](../../../README.md)

# Module 13 — Dialogue and Global Verb Handlers

*Dunkle Schatten 2 — this page describes one of the game's own script modules. The engine it runs on is documented under [MOTION 32-bit](../../../README.md#motion-32-bit).*

Module 13 (30 words) holds the game-wide object behaviors: what happens
when the player examines, combines, or reads things regardless of the
room — plus the global task handler that drives multi-step sequences.

## Dialogue helpers

| Word | Effect |
|---|---|
| `SAYKARSTEN ( txt -- )` | say entry `txt` of text table 3 as the main character (see [Text and speech](text-and-speech.md)) |
| `FORCE_ORDER ( verb obj1 obj2 -- )` | inject an interaction as if the player had clicked: fills `_ORDER` (+0/+4/+8), sets the synthetic state 97, and dispatches through the engine's `DOORDER` |
| `?DIALON ( -- f )` | true while a dialogue is on screen (interaction state 12–18) |
| `MYWAIT ( n -- )` | a raw **busy-wait** of n·10000 empty loop iterations — CPU-speed dependent, one of the few timing sins in the code |

## The global verb handlers (`PCALC*`)

Installed into the `_P_CALC*` vectors by module 12's `INIT_PORDER`; they
are the second stage of the
[verb dispatch chain](game-library.md). Contract: return 0 = handled
(EXAMINE), 2 = not handled (TAKE/HANDLE/USE), −1 (TALK).

- `PCALCTAKE ( obj -- 0 )` — empty: **all takes are location-specific**.
- `PCALCEXAMINE ( obj -- r )` — a long dispatch on the object id: most
  entries either say a line (`SAYMC`) or arm a global task (below).
  Examples: examining the school notebook arms task 1002; the newspaper
  1004; the photo 1007; the e-mail 1008; examining the bicycle wheel in
  the right room yields the inner tube (`1SCHLAUCH ADDITEM`).
- `PCALCHANDLE ( obj -- r )` — handling behaviors: splitting the
  paper-wrapped stone into paper + stone, turning the paper into the
  leaflet, picking up the tube, plus canned info-line texts for a few
  items.
- `PCALCUSE ( obj2 obj1 -- r )` — the **combination table**, essentially
  one puzzle chain: inner tube + full bucket → find the leak; tube +
  pump → inflate (and the object record is re-registered with new name
  text 217, sprite 644, and description 256 — object state changes are
  *re-registrations*); tube + repair kit → patched tube; patched tube +
  wheel → repaired bike. Plus flashlight + batteries. Every other pair
  falls through to one of ~14 refusal lines.
- `PCALCTALK ( obj -- -1 )` — empty: talk is always location-specific.
- `PCALCGIVE` / `PCALCINFO` — return the strings `"DGIVE"` / `"DINFO"`,
  the action-name keys found in the dialogue definition blocks.
- `PCALCLEAVE ( loc -- )` — enter the location, except for two blocked
  exits (31, 32) which do nothing.

## Dialogue consequences (`DC_*`)

Fourteen small words named after dialogue ids apply the world changes a
finished conversation causes — adding or removing inventory (e.g.
`DC_18`: gain the second disk; `DC_45`: gain power supply + answering
machine + remote), setting story variables (`DC_03b`: `_KGABY := 1`), or
setting dialogue-done flags (`DC_13`). They are reached from the
conversation data, most plausibly through the
[dialogue machine's](../../../motion32/engine/dialogue-machine.md) branch actions 5/6,
which call a script word **by name** (the lookup path is still
unmapped).

## `GLOBTASK` — the global task handler

Runs every frame from the [control handler](shell.md) while no menu is
open, and only when the active task id is **> 1000**
(ids < 1000 belong to the per-location handlers). Each task is a phase
machine gated on `?LTWAIT`/`NEXTLTP`. The shipped tasks:

| Task | Sequence |
|---|---|
| 1001 | Story beat after examining the message: a line, then location change to 5 |
| 1002 | Reading the school notebook (two lines, state advance) |
| 1003 | The internet page: full-screen image 2100, fade, story flags around the logo |
| 1004 | Reading the newspaper: palette 27, full image 2214, first read adds the arson-article overlay 2215 and sets story flag 81; later reads set flag 96 — then the paper is consumed (`SUBITEM`) |
| 1005 | The leaflet: a five-line read with a conditional continuation |
| 1006 | The lighter: one line |
| 1007 | The photo: full-screen 2221, palette 32 |
| 1008 | The e-mail: text depends on dialogue flag 77 |
| 1009 | The folder: eight lines, then the photo joins the inventory if absent |
| 1100 | The **document viewer**: background sprite 32, pages 0–6 as full images 32–38 with palettes 191–197, left/right click paging, right-click exit |
| 1101 | The **info-book browser**: background block 66, palette 95, eight category tabs each unlocked by a dialogue-done flag, per-category page counts (see below) |

## The info book (`SHOW_RUBRIK`)

The in-game reference book. Eight categories, each visible only after
the matching dialogue flag (`?DDONE` 15, 16, 28, 40, 41, 52, 64, 90) is
set; tabs at fixed x positions with selected/unselected sprite pairs
(67/68 … 81/82). Body text comes from **text table 7**, with a per
category base entry and page offset; page counts per category are
0/6/6/2/1/3/2/7/4.

## See also

- [Dialogue machine](../../../motion32/engine/dialogue-machine.md) — the native
  conversation apparatus these words feed
- [Game library](game-library.md) — the dispatch chain these plug into
- [Objects and flags](objects-and-flags.md) — the ids and flags used here
- [Shell](shell.md) — where `GLOBTASK` is invoked
