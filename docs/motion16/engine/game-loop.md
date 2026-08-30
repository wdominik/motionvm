[← Documentation index](../../README.md)

# Boot and Frame Loop

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

The engine starts the VM at the module and word the container header names
— module 100, word id 401, `RUN` in Die Enviro-Kids greifen ein — and everything
else is the
game's Forth. What follows is read from the compiled words `RUN` and `CTRL`
(module 100), `STARTINTRO` and `ICTRL` (module 610), `INCLLOC` and
`NEWPERS` (module 605); the kernel handlers they call are unread, and their
effects below are what the call sites establish.

## `RUN` (module 100, id 401)

In order:

1. `=>GET` modules 600, 601, 602, 603, 604, 605, 606, 607, 609 — the
   resident library.
2. Push the constants 1…10, `=>GET 651`, store 1685 into `DO_XDIR`,
   `=>ERASE 651` — the walk-direction tables are copied and dropped.
3. `TOGFX` — into graphics mode. The game never calls a resolution word;
   320×200×256 is the mode `TOGFX` enters.
4. `0 SETPAL`, `NEWANIM`, **`BUFON`** — buffer compositing on from the start
   ([off-screen buffers](buffers.md)).
5. `399 FATMOUSE` — sprite 399 (16×15) becomes the pointer. It stays
   invisible: `SHOWMOUSE` and `HIDEMOUSE` (`14ee:0874`, `14ee:094b`) move
   a show counter that starts at zero — the pointer shows from one up —
   and both are inert until a shape has armed the pointer (`ds:0x16AA`).
   `RUN`'s one `SHOWMOUSE` comes after `STARTINTRO`, so the intro runs
   without a pointer.
6. `=>GET 610`, `STARTINTRO`, `=>ERASE 610` — the intro plays and its module
   is dropped.
7. `125 _TSPEED !`, `1 SETPAL`, `396 0 0 FXATMOUSE`, `SHOWMOUSE`.
8. `=>GET 611` `INCLPERM` `=>ERASE 611`; `=>GET 612` `DEF_CHARS` `DEF_ITEMS`
   `=>ERASE 612` — one-time initialization.
9. Fonts: `0 SFT`, `2 +FONT _SHFONT !`, `7 +FONT _MEMO !`.
10. `XDEFTDT` for templates 1–9, each with one number (22, 16, 32, 61, 54,
    38, 27, 42, 6). `XDEFTDT` (module 603) is `>R 1 1 -1 -1 -1 -1 _SHFONT @
    R> DEFTDT`, so the kernel's `DEFTDT` receives nine values — the number,
    `1 1 -1 -1 -1 -1`, the shadow font's handle and the template id — the
    same count as in the 32-bit game, whose first value is a constant 8.
    The leading number is the shadow color — the last value the handler
    pops (`05f1:2ee7`); see [text rendering](text-rendering.md).
11. `15 DELAY`, `_MS SSACT`, **`400 SCRCTRL`** — `CTRL` becomes the frame
    handler — then `1 50 8 FADEOUT`.
12. `UPLOAD_GAME` (module 606).
13. `1 STARTLOC !`, `STARTLOC @ INCLLOC` — enter location 1, the dump.
14. If `STARTLOC` is 1: `0 _?STARTUP !`, then `706 701 DO I =>EXIST … LOOP`
    probes the five save slots; if any exists, module 650 is loaded and a
    page with *Laden* and *Neustart* comes up over the location, and `RUN`
    waits for the click **in a loop of its own** — `BEGIN … MOUSELK …
    MOUSEY … MOUSEX … _?STARTUP @ 2 < UNTIL`, outside any `ANIMPLAY` —
    until `_?STARTUP` is set (*Neustart* at x 246–305, *Laden* at 12–56, y
    172–189). The loop polls the pointer; how the rebuild keeps such a loop
    turning is a [departure](../../departures.md#the-16-bit-machine). The
    five slots are the same file names the other games use (`701.blk` and
    so on), so no two games may share a save directory.
15. **`ANIMPLAY`** — the frame loop. It does not return until the game is
    over.
16. `HIDEMOUSE`, `GFXTO` (back to text mode), `=>ERASE` of 609, 607, 606,
    605, 604, 603, 602, 601, 600, `##`.

So the process is `RUN` → intro → location 1 → `ANIMPLAY` → teardown, and
the whole game happens inside step 15.

## The frame loop

`ANIMPLAY` is the blocking word. Each frame it runs the word installed with
`SCRCTRL ( word-id -- )`:

- during the intro `1086 SCRCTRL` — `ICTRL` in module 610 (1657 cells);
- in the game `400 SCRCTRL` — `CTRL` in module 100 (1219 cells), always
  resident.

`ANIMPLAY` takes no arguments here; the 32-bit game pushes ten values
before it that its handler never pops. `QUITANIM` is in the kernel and
called from four sites. `NEWANIM` is called once, before `BUFON`.

**The loop, read** (`ANIMPLAY` at file `0x4d9d`): `SCRCTRL ( word-id -- )`
stores the id on the *active screen* (`+0x14` of the 0x13ae-byte screen
block at `DS:0x305a`; two screens), and each frame the loop, for screen 0
then screen 1, first runs the callbacks of the descriptors whose `SDWAIT`
countdown stands at 0 (a countdown above 0 is decremented instead; -1 is
off), then the screen's controller word, then the restore-and-draw step
([buffers](buffers.md)); then it waits until the `DELAY` tick count has
elapsed since the frame began, blits the dirty rectangles of both screens
to the display, steps a running palette fade, and goes round again while
`QUITANIM` has not cleared its flag — the frame in which `QUITANIM` ran is
still shown. `FREEZESCR` stops the callbacks and keeps the controllers.
Nothing in the loop polls input: the controller reads the mouse words
itself.

**Every screen's controller runs, including a screen that is switched off.**
The same loop in `LL.EXE` walks three slots (`0104:5528`, `cmp $3,%si`) and
for each one whose word id is not `0xFFFF` makes that screen current and runs
it (`0104:55ec` to `0x561f`). The activity test sits earlier in the loop
(`0104:5543`) and jumps to exactly that point, so what an inactive screen
loses is the descriptor work and not its controller. Victor Loomes is the game
that shows why it matters: its menu is a screen of its own, switched off while
the game plays, and the word that watches for the pointer reaching the top of
the display and switches the screen back on — `PANCTRL`, module 602 id 1852 —
is the controller *of that hidden screen*.

`CTRL` reads `?KEY` (stored in `_AKTKEY`) and the mouse, tracks `_MX`/`_MY`
against the previous frame, and splits the display at **y = 165**: above it
the world (`_MMX`/`_MMY`), below it the inventory and verb bar
(`_IMX`/`_IMY`). Then it runs the order and hover machinery (`_ORDER`,
`SMDESC`, `SDWAIT`), fades in when the screen is active and `ZEIM` is set,
and descends into a `_SYS_LEVEL`/`_INVMODE` tree for the inventory, the
verb bar and walking. A debug key prints the mouse coordinates with `.` and
`EMIT` — the only uses of those two words in the game.

**`DELAY n` sets the frame period to `200 / n` ticks of a 200 Hz clock**,
the same rule as the 32-bit engine's — read here, not assumed: the handler
(file `0xa901`) stores `200 / n` (`DS:0x05a8`, 20 in the file), and the
loop's wait compares it with a counter the sound driver's timer interrupt
advances once a millisecond, divided by five. `15 DELAY`, in `RUN` and
again in `STARTINTRO`, is therefore 13 ticks — 65 ms, a little over 15
frames a second.

## Saves

Five slots, 701–705, three files each, the same names as the 32-bit game's
— `NNN.blk`, `NNN.anm`, `NNN.FRZ` — written and read by the same words in
the same order. The save page (`DOINVSAVE` in module 650, reached from
the inventory bar's menu) runs

```text
ACTLOC @ _LOADTABLE !   2 _LOADTABLE slot PUT   slot PUTANIM
650 =>ERASE   slot =>PUTAS   650 =>GET   SHOW_FILES
```

— the location number into `.blk`, the display into `.anm`, the resident
modules into `.FRZ`, with the menu module erased for the duration so that
it is not saved — and `CTRL`'s load path (module 100, cells 664–790), on
a click on a slot's star, runs `2 _LOADTABLE slot GET`, `_LOADTABLE @
INCLLOC`, `slot GETANIM`, `slot =>GETAS`, `_LASTPAL @ SETPAL`, then
`UPLOAD_GAME` and the fades. `=>EXIST` (file `0x16f8c`) is a file test on
the `.blk`; `RUN` probes all five after the first location and puts its
page up if any is there (`RUN`, step 14).

What the 16-bit handlers write is read as far as `=>PUTAS` (`12c8:1169`,
file `0x16fe9`): one run of the arena from the first resident module's
header to the last module's final `##`, as it stands — addresses and all,
which is what makes the original's `=>GETAS` purely positional, and what a
rebuild that places modules elsewhere cannot take back. So the layout of
`.FRZ` and `.anm` is motionvm's own for this game as for the other, under
magics of its own (`ENVFRZ`, `ENVANM`): the resident modules' images, the
descriptors with their buffers, the screens, the palette, the off-screen
buffers; only `.blk` — two raw bytes here, four there — coincides with the
original's. And every game keeps its saves apart: all of them name their
slots alike and each asks at start-up whether a slot exists — and between the
two 16-bit games even the magics match, so one would open the other's slot
rather than refuse it. See the
[savegame departure](../../departures.md#savegames).

`NEWANIM` (`05f1:000a`) is the display system's initializer: it sets up
the screen and descriptor state, registers **the container's font 0 as
the first font** — the face every descriptor wears until `SDFNT` picks
another — and probes an optional `gfx.inf` beside the game data, going
on without one. `UPLOAD_GAME` (module 606) is a cache warmer: four
`XGFXSTAT+` ranges over sprites 0–59 and two `TXTSTAT` hints, gated on
`_FASTSTART` — all of it advisory to a loader that loads lazily anyway.

## Open questions

- `DEFTDT`'s first value is read — the shadow color; what remains here is
  the speed class the clock divides by (`DS:0x10ce`, 0 in the file, set
  once at start-up).
- Who fills the mouse record at `DS:0x16c0` that `MOUSEX`/`MOUSEY` copy —
  an interrupt 33h callback or a poll.
- The 16-bit `.anm`'s bytes (`PUTANIM` at file `0xc46d`, `GETANIM` at
  `0xcb1e`) — unread, and not needed while the layout is motionvm's own.

## See also

- [Execution model](../vm/execution-model.md) — `SCRCTRL` and word-id callbacks
- [Off-screen buffers](buffers.md) — why `BUFON`/`SETBUF`/`SDBUF` cannot be stubs
- [Descriptors and screens](descriptors.md) — the words the intro calls
- [Game structure](../../games/enviro/game-structure.md) — the locations and module series
- [Game loop (MOTION 32-bit)](../../motion32/engine/game-loop.md) — the 25 fps loop of the 32-bit engine
