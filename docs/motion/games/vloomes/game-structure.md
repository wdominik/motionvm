[← Documentation index](../../README.md)

# Game Structure

*Victor Loomes – Das Spiel — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

How the game's 36 script modules organize into a running adventure: the
setting, the location scheme and its two module series, the verbs, the
characters, and saving. For what each module contains, see the
[module map](module-map.md); for the family's boot sequence and the shape all
five 16-bit games share, [boot and frame loop](../../motion16/engine/game-loop.md).

## Setting

The player is the detective **Victor Loomes**, and the plot travels: the
container's own strings name *Chicago 1927*, *Frankfurt 1993* and *Frankfurt
2000*, the object table in module 604 carries the flag `FRANK93`, and one of
the rooms says outright that a machine is what one changes *"zwischen den
drei Zeitzonen"* with. The client is in two of the three years — the object
table names two counter clerks, `LBS93LADY` and `LBS00LADY`, and locations 9
and 11 are the two rooms whose scene modules place a `_D_LADY` and give her
her own speech words. What the game is about, who made it and the competition
it was distributed for are on [the game's own page](README.md).

The dialogue is German in CP437. The engine's own prompts are the informal
*Du*, as they are in Die Enviro-Kids greifen ein and unlike the two builds
between them, and so is the in-game help in text table 7.

## Startup

`RUN` is **module 100, word id 449** — the pair the container header names. The
later games put `RUN` at 401 and `CTRL` at 400; here those two ids belong to
`?LPD` and `LPD`, so even the boot word's number is this generation's rather
than the family's.

In order, and read from the compiled word: fetch the resident library —
600, 602, 603, 604, 605, 606, 607, **608**, 609, with no 601 because this game
has none; store the ids of module 608's `DO_ORDER` and `DO_TAKE` into the
`_DO_ORDER` and `_DO_TAKE` hooks; `AO 0!`; `BUFON`; `-1 16 SETSHADE`;
`NEWANIM`; `TOGFX`; `0 SFT` and `2 +FONT _SHFONT !`; twelve text templates
through `XDEFTDT`; then `610 =>GET INTRO 610 =>ERASE` — the intro plays and its
module is dropped. Palette 0 goes in, sprite 399 becomes the pointer, and
`RUN` builds its **three screens** (below). Then `INCLPERM`, `INITDATA`,
`SCHLÜSSEL ADDTOINV` — the player starts holding the key — `1 INCLORT`, and
`ANIMPLAY`.

**`RUN` enters location 1 itself.** There is no `STARTLOC` variable to read and
no start-up page: `1 INCLORT` is the cell before `ANIMPLAY`, so the first room
is a literal in the boot word. The save slots are not probed here either —
`CTRL` owns that, and probes them when the player asks (below).

The intro is module 610's `INTRO` (id 569) with its own frame handler `ICTRL`
(id 567) on a screen of its own, and it is the only controller running while it
is up. It installs palette 16, zooms a logo sprite in, puts the competition
slide up, and runs `0 7 STARTTUNE ANIMPLAY ENDTUNE` — block 7, the 552-byte
jingle, started with a loop count of zero, so it plays once
([PSM 2 music](../../motion16/formats/psm-music.md)).

## The three screens

`RUN` builds three and gives them **three different controllers**, which no
other game in the corpus does:

| Screen | Size | Window | Controller |
|---|---|---|---|
| `_MS` — the room | 960×280 | 320×140 at y 25 | `432`, `CTRL` |
| `_PS` — the strip along the top | 320×25 | 320×25 at y 0 | `1852`, `PANCTRL` (module 602) |
| `_IS` — the inventory bar | 320×35 | 320×35 at y 165 | `-1`, none |

25 + 140 + 35 is the 320×200 the engine's `TOGFX` mode gives, and the room is
drawn on a screen three times as wide as its window and scrolled. `RUN`
then selects the room with `_MS @ ACTSCR`.

That `SCRCTRL` belongs to a screen and not to the engine is this game's
finding: read as one id for the whole engine, the last call wins, that call is
the inventory bar's `-1 SCRCTRL`, and `ANIMPLAY` finds no controller at all
([`LL.EXE`](../../motion16/engine/ll-exe.md)).

## Locations

Thirteen, numbered 1 to 13, and each owns **two** modules where the later games'
own three:

| Kind | Id | Lifetime |
|---|---|---|
| Scene | module 100+N | resident while the location is active |
| Macro | module 20+N, word id 549, named `ZEIT` | loaded, run through `LOCINIT EXECUTE`, erased |
| Item table | block 200+N, 1120 bytes | loaded into `_PKITEM` |
| Walk routes | block 300+N, 542 bytes | loaded into `_ROUTE` |
| Extended routes | block 400+N, 360 bytes | loaded into `_XROUTE` |
| Click areas | block 20+N, 202 bytes | loaded into `_KLICKAREA` |

There is no third module: the later games' click modules 500+N have no
counterpart, because a hot area here names an item record in the block rather
than a word. The macro carries the id-549 discipline the later games follow
with sixteen and twenty macros — all twelve of this game's define exactly one
word, id 549, named `ZEIT`.

**Location 3 has neither module of its own.** `INCLORT` special-cases it and
loads location 2's pair, which is why modules 23 and 103 do not exist. Its
blocks do: 203, 303, 403 and 23 are all shipped, so the two locations share
their scripts and not their geometry.

Moving between them goes through one variable, as it does in every game of the
family, under this game's own names. **`AO`** (module 605, id 1483) is the
location the game is in, **`?AO`** (1484) is its fetch, and **`NAO`** (1485) is
the one it has been asked for. `CTRL` ends every frame on
`NAO @ IF NAO @ INCLORT NAO 0! THEN`, and the scripts store their exits in
`NAO`; that is also the only way in from outside, so motionvm's `--loc N`
writes `NAO` and the game acts on it the next frame.

`INCLORT` (module 100, id 427) is the switch. It runs and clears the outgoing
room's `_EXITCALL`, fades out unless `FADED` is up, drops the old pair, clears
the ten `_CALL_*` hooks and the walker state, stores the new number into
`AO`, fetches `?AO + 100` and `?AO + 20`, `GET`s the four blocks, and finishes
on `LOCINIT EXECUTE` — `LOCINIT` is the constant 549, so that call is the
macro.

### The two transit lines

Two bands of numbers ride on top of the thirteen. `INCLORT` tests `?AO` before
it loads anything: 101 through 139 sets `AKTBAHN` to `?AO − 100` and enters
location **7**; 141 through 179 sets `AKTFBAHN` to `?AO − 140` and enters
location **6**. Both rooms are the inside of a line, and their scene modules
carry the same three variables for it — `_BAHNLOCK`, `_BAHNENTER`, `_BAHNBUF` —
and a `DO_BAHN`.

The blocks follow the stop rather than the room. For location 7 the four come
from `AKTBAHN + 220`, `+ 320`, `+ 420` and `+ 40`; for location 6 from
`AKTFBAHN + 240`, `+ 340`, `+ 440` and `+ 60`. Twenty stops of each are
shipped.

### Location 13

The thirteenth room is the time machine, and it is the smallest module in the
game: seven words, 330 bytes, no verb handlers, and a `CALCMOVE` that is one
cell. Its two words shrink a descriptor by ten per frame and, once it is under
half size, stop the palette cycle, write 13 into `_OLDROOM` and put
`DESTINATION @` into `NAO`. `DESTINATION` is a flag in module 604 that
locations 4, 5 and 10 set before they send the player here, so the room is a
transit whose exit belongs to whoever entered it.

It is also the only location that asks for a cycling palette: `SETCYCLE` occurs
in modules 33 and 113 and nowhere else in the game. The room's macro arms
`1 127 32 SETCYCLE`, and from then on the kernel's tick (`0104:536d`, once a
frame after the blit) moves entries 32 through 127 up by one more than the
frame before, wrapping inside the range; `DO_TIME_1` and `DO_TIME_2` disarm
it with `1 0 0 SETCYCLE` once the descriptor they shrink is under half size.

## Hotspots, items, verbs

**Seven verbs, not eight.** Module 605 holds them as constants: `O:EXAMINE` 1,
`O:USE` 2, `O:HANDLE` 3, `O:TAKE` 4, `O:TALK` 5, `O:GIVE` 6, `O:INFO` 7. The
later games have the same names in a different order with `LEAVE` 8 added; here
there is no eighth constant, and leaving is a room's own `.CALC_LEAVE` where
a room has one.

A click lands in a rectangle from block 20+N — records of ten bytes, four
corners and a fifth field, reached through `KA`, `KAX1`…`KAY2` in module 600 —
which names an item record in block 200+N. That record is 28 bytes and its
fourteen `u16` fields are the accessor names in modules 599 and 600:
`X1 Y1 X2 Y2 T IT RR DX DY EX OB XO YO DIR`.

The right button opens the command menu the help calls the *Befehls-Menü*, and
what it produces is an **order**: `_OBJORDER` gets the verb, `_OBJACT` the
object, and module 608's `DO_ORDER` dispatches through the seven constants to
`DO_EXAMINE`, `DO_USE`, `DO_HANDLE`, `DO_TAKE`, `DO_TALK`, `DO_GIVE` and
`DO_INFO`. Each of those does the shared work and then runs **the room's own
handler by id**: the location macro stores its scene module's word ids into
`_CALL_EXAMI`, `_CALL_USE`, `_CALL_TAKE`, `_CALL_GIVE`, `_CALL_INFO`,
`_CALL_TALK_`, `_CALL_XUSE`, `_CALL_DIALO`, `_CALL_ENDDI`, `_STEADYFUNC` and
their siblings in module 605, and `DO_*` calls them with `EXECUTE`. That is
this generation's answer to the later games' `MYCALC*` naming convention, and
it is why a scene module's words need no fixed names.

The objects themselves are module 604's: an 80-entry table of ten bytes, of
which ids 2 to 61 are named constants — `BU_LAMP`, `PISTOLE`, `PAß`, `ZETTEL`,
`UNIFORM`, `VERTRAG`, `DIAMANT`, `SCHLÜSSEL` and the rest, characters among
them. The story state beside it is `KOND`, 200 conditions, and `SBIT`, a bit
array reached through `SETBIT`/`CLRBIT`/`GIVBIT`; the named condition words
`AKTBAHN`, `AKTFBAHN`, `DESTINATION`, `FRANK93`, `AKTIENWERT` and their
forty-odd siblings are one-line readers over it. The inventory is module 603's `_INV`,
with `ADDTOINV` and `SUBTOINV` in module 608, and the strip along the bottom is
drawn by `INVCTRL`.

## Talking

Fourteen speaking characters, as constants `I_RACHEL` through `I_HÄNDLER` in
module 605, eleven of which own a talk-state table (`_T_RACHEL`, `_T_BARDAME`,
`_T_WIBI`, …) in the same module.

The dialogue machine is module 608's: `INIT_DIALOG` (303 cells), the choice
list `INIT_CHOICE`, `NEXTCHOICE`,
`SET_ANSWER` and `DO_CHOICE`, and `EXIT_DIALOG`. Module 606 holds the reply
buffer `_ERBUF` and its bookkeeping, and each scene module names its own
speakers' text words (`SETRACHTEXT`, `XT_WIBI`, `XTD_PROF`) and blink and mouth
words per character.

## Walking

Module 609 is the walk machinery — `WALKER` at 927 cells, `NPCWALKING` at
569 — over the route tables in blocks
300+N and 400+N and the walk records `_FIGINFO` and `_NPCINFO`. The route
itself is laid out by the kernel's `CROUTE`, which this game calls directly
with its five pointers on the stack; the later games reach the same routine
through `DOWALK`, a word this build's kernel does not have
([`LL.EXE`](../../motion16/engine/ll-exe.md)). This build's routine differs
in two things, both read off the binary when the game opens: a zero shrink
is copied as it stands rather than taken as 1000, and a closing pass folds a
one- or two-step heading flip between longer runs into the heading around
it (`0104:516d`).

## Saving

Five slots, 701 to 705, and **`CTRL` is the menu**: there is no module 650. A
click in the strip at the top right becomes one of `CTRL`'s own key codes — 317
above the middle, 318 below — and each puts a `REQUEST` box up over text table
6, which holds *Spielstand sichern*, *Spielstand laden* and the five slot
labels *A* to *E*. Saving runs `PUT`, `PUTANIM` and `=>PUTAS` over the slot
number; loading probes first, `706 701 DO I =>EXIST … LOOP` into `_LOADTABLE`,
and offers only what answered.

The file names are the engine's — `701.blk`, `701.anm`, `701.FRZ` — and so is
the layout motionvm writes, which is why the game gets a savegame directory of
its own ([departures](../../departures.md#savegames)).

## Open questions

- **The transit bands are wider than the stops that ship.** `INCLORT` accepts
  `?AO` up to 139 for the first line and 179 for the second, which would index a
  thirty-ninth stop on each; the container holds blocks for twenty — item tables
  221–240 and 241–260, and their three companions. Whether the wider test is a
  loose bound or the remains of a longer line is unread.
- **`_TIMEZONE` is declared and never touched.** It is a variable in module
  605 (id 1487), it is named for the three years the plot turns on, and no
  cell in any of the 36 modules reads or writes it — the zones are moved
  between through `DESTINATION` and the room numbers instead.

## See also

- [Module map](module-map.md) — every module
- [Resource inventory](inventory.md) — the counts
- [Other shipped files](other-files.md) — the four-program launcher chain, `GFX.INF`, the older sound setup
- [`LL.EXE`](../../motion16/engine/ll-exe.md) — the build this game reads its kernel table out of
- [Boot and frame loop](../../motion16/engine/game-loop.md) — the family's `RUN`, `CTRL` and location loader
- [Game structure (Hilfe für Amajambere)](../hfa/game-structure.md) — the same template two years later, with three modules to a location
