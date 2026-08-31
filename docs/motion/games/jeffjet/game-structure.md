[← Documentation index](../../README.md)

# Game Structure

*Jeff Jet - Abenteuer InfoHighway — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

How the game's 55 script modules organize into a running adventure: the
setting, the location scheme and its three module series, the verbs, the
characters, and saving. For what each module contains, see the
[module map](module-map.md); for the boot sequence and the location loader
word by word, [boot and frame loop](../../motion16/engine/game-loop.md).

## Setting

A German point-and-click adventure made for Hewlett-Packard: Jeff has written
a program that scans a person into their own computer through an HP scanner,
tries it on himself, and strands himself and Donna inside the machine. The
twelve rooms that follow are the machine's parts seen from within — the
scanner, the memory, the processor, the printer's composing room, the
InfoHighway itself — and the way out is the HP DeskJet, which prints them back.
The product placement is the setting: the text corpus names *Hewlett Packard*
nine times, *Drucker* twenty-two and *Modem* eighteen.

The dialogue is informal *Du*. The player's own text — the engine's prompts —
is formal *Sie*, which is one of the few places where this build's DGROUP was
edited away from Die Enviro-Kids greifen ein's.

What the game is about, who made it and why it exists are on
[the game's own page](README.md).

## Startup

`RUN` (module 100, word id 401 — the pair the container header names) loads the
resident library, reads the walk-direction table out of module 651 and drops it
again, enters graphics with `TOGFX`, installs palette 0, and then, unless
`_FASTSTART` is set, plays the intro from module 610 and erases it. It defines
the nine text templates, installs `CTRL` as the frame handler with
`400 SCRCTRL`, fades out, runs `UPLOAD_GAME`, and enters `STARTLOC` — which is
**13**, the room Jeff starts in.

Only then, and only when the location it entered is 13, does it probe the save
slots: `706 701 DO I =>EXIST … LOOP`. If any of the five answers, `_?STARTUP`
goes up, module 650 is loaded and the start-up page comes up over the room with
`3 _INVMODE !` and `SHOW_FILES`. Either way the last thing `RUN` does is
`ANIMPLAY`, and from there every frame belongs to `CTRL`.

## Locations

Thirteen, numbered 1 to 13 with no gap, and each owns three modules and four
blocks:

| Kind | Id | Lifetime |
|---|---|---|
| Scene | module 100+N | resident while the location is active |
| Macro | module 300+N, word id 549, named `ZEIT` | loaded, run through `LOCINIT EXECUTE`, erased |
| Click | module 500+N, ids from 750 | resident while the location is active |
| Item table | block 200+N, 1120 bytes | loaded into `_LDITEM` |
| Walk routes | block 400+N, 452 bytes | loaded into `_ROUTE` |
| Extended routes | block 600+N, 300 bytes | loaded into `_XROUTE` |
| Click areas | block 800+N, 182 bytes | loaded into `_KLICKAREA` |

Moving between them goes through one variable. `CTRL` runs
`NEXTLOC @ -1 != IF NEXTLOC @ INCLLOC -1 NEXTLOC ! THEN` every frame, and the
scripts store their exits there; `INCLLOC` (module 605) does the rest. That is
also the only way in from outside: motionvm's `--loc N` writes `NEXTLOC` and
the game honors it on the next frame.

The room is drawn on a screen larger than the display and scrolled — 960×544
seen through a 320×165 window in location 13 — with the verb and inventory
strip on a second screen of 320×35 beneath it. The two add up to the 320×200
the engine's `TOGFX` mode gives.

## Hotspots, items, verbs

Eight verbs, the same eight Die Enviro-Kids greifen ein has, as constants in module
603: `TAKE`, `EXAMINE`, `HANDLE`, `USE`, `TALK`, `GIVE`, `INFO`, `LEAVE`, with
the classes `TAKEABLE`, `HANDLEABLE`, `USEABLE`, `TALKABLE`, `LEAVEONLY` and
the combinations `TAKE_HANDLE`, `TAKE_USE`, `TAKE_USE_HA`.

A click lands in an area from block 800+N, which names a word in module 500+N —
`LD_SCANNER`, `LD_MODEM`, `LD_PINGUIN` — that fills an item record in
`_LDITEM`. The verb machinery in module 606 (`CALCTAKE` … `CALCLEAVE`) then
calls the room's own `MYCALC*` handler in module 100+N, which is where a room
says what happens. Items carried are constants in module 607: `SUMM`,
`FAPPARAT`, `MAGNET`, `WALKIE`, `WODKA`, `DIAMANT`, `PERLE`, `KOHLE`,
`DISKETTE`, `KOSTÜM`, the five letters `BUCH_D` `BUCH_R` `BUCH_E` `BUCH_C`
`BUCH_K`, and thirty more.

The noun under the pointer is a string from text table 11 — 132 of them,
*Fotoapparat*, *Tür*, *Fotos* — and what Jeff says about it comes from tables
20 and 21.

## Talking

Two characters walk: Jeff and Donna. Module 606 keeps a message pipe for each
(`_DAVEPIPE`, `_DONNAPIPE`) with its own string, info and inventory variables,
and module 614 holds the dialogue helpers `->DIAL`, `CSET`, `XCALCTALK`. The
dialogue fields are variables in module 607.

## Walking

Module 604: `XYWALK` and `DESTWALK` take a destination, `WALKER` steps it, and
the kernel's `DOWALK` does the moving; the direction table is module 651's one
word, whose id `RUN` copies into `DO_XDIR` before dropping the module. Routes
come from blocks 400+N and 600+N.

## Saving

Five slots, 701 to 705, three files each — `.blk` for the location, `.anm` for
the display, `.FRZ` for the resident modules. Module 650 is the page:
`DOINVSAVE` writes with `PUT`, `PUTANIM`, `=>PUTAS` and the load path reads
with `GET`, `INCLLOC`, `GETANIM`, `=>GETAS`, which is Die Enviro-Kids greifen ein's
scheme exactly, down to the slot numbers.

That is also why motionvm gives the game a savegame directory of its own: the
files are named alike and the format's magic is the engine's, not the game's,
so one game's slot in another's directory would be opened rather than
refused ([departures](../../departures.md)).

## Open questions

- What the first field of the animation catalogs 125–182 means. It is `0xFFFF`
  in several of them where Die Enviro-Kids greifen ein's catalogs hold small integers.
- Whether anything in the game reaches the two font-reference entries that
  point past its fonts. Nothing in the shipped strings does.

## See also

- [Module map](module-map.md) — every module
- [Resource inventory](inventory.md) — the counts
- [Other shipped files](other-files.md) — the launcher, the sound stack, the two splash pictures
- [Boot and frame loop](../../motion16/engine/game-loop.md) — `RUN`, `CTRL`, `INCLLOC`
- [Game structure (Die Enviro-Kids greifen ein)](../enviro/game-structure.md) — the same template, one game earlier
