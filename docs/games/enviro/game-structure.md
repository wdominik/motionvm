[← Documentation index](../../README.md)

# Game Structure

*Die Enviro-Kids greifen ein — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit-enviro).*

How the game's 65 script modules organize into a running adventure: the
setting, the location scheme and its three module series, the verb table,
the characters, and saving. For what each module contains, see the
[module map](module-map.md); for the boot sequence and the location loader
word by word, [boot and frame loop](../../motion16/engine/boot-and-loop.md).

## Setting

The town of **Waldbach** — *"ein hübsches kleines Städtchen"* that lives on
tourism, the intro's text table 14 says — with the **VISALUX** factory and a
dump. The player is one of the Enviro-Kids, **Eva** or **Maik** (*"Jetzt bin
ich Eva."* / *"Jetzt bin ich Maik."*, table 20); the script's person and
face words name the cast: Dave, Evi, Sarah, Sascha, Anne, Opi, Waffi,
Fischer, Traffke, Sue, Niemöller, Sepp, Münz, the Postbote, Wami, and the
Müllmann. A newspaper — the articles `_ART1`–`_ART3` in module 615 — and a
clock of days (`DAY2`–`DAY4`, `_TAG`) structure the plot.

The game is played with the mouse on a 320×200 picture whose lower part,
from y = 165, is the inventory and verb bar; the control handler splits the
pointer's coordinates there.

## Startup

The container header names module 100's `RUN`. It loads the library (600–
607, 609), plays the intro from module 610, runs the one-time words
`INCLPERM` (611) and `DEF_CHARS`/`DEF_ITEMS` (612), registers the fonts,
installs `CTRL` as the frame handler, enters location 1 and offers to
continue a saved game if one exists. `STARTLOC` is a variable in module
601 whose compiled initial value is 13 (the editorial office — presumably
the author's test room); `RUN` stores 1 before using it.

## Locations

A location *N* is built from three modules and four blocks, all addressed
by arithmetic on *N*:

| Kind | Id | Lifetime |
|---|---|---|
| Scene module | 100+N | resident while the location is active |
| Room macro module | 300+N | loaded, run through `LOCINIT EXECUTE`, erased |
| Click-handler module | 500+N | resident while the location is active |
| Item/hotspot table | block 200+N → `_LDITEM` | copied into module 601's array |
| Walk routes | block 400+N → `_ROUTE` | likewise |
| Extended routes | block 600+N → `_XROUTE` | likewise |
| Click areas | block 800+N → `_KLICKAREA` | likewise |

Sixteen locations exist, 1–15 and 17; 16 does not. The room macro is one
word per module, always with word id 549, and its name is the room's:

| N | Macro | Room |
|---:|---|---|
| 1 | `MUELL` | The dump (Mülldeponie) — the first room |
| 2 | `BIRKENSTRAß` | Birkenstraße |
| 3 | `WALD` | The forest |
| 4 | `WILDB` | The Wildbach stream |
| 5 | `VISALUX` | The VISALUX factory |
| 6 | `RAFFKE` | Raffke's |
| 7 | `EINKAUFSZEN` | The shopping centre |
| 8 | `STADTZ` | The town, first view |
| 9 | `STADTV` | The town, second view |
| 10 | `PINIEN` | The pines |
| 11 | `WAFFEN` | Waffi's |
| 12 | `BAUMHAUS` | The treehouse |
| 13 | `REDAKT` | The editorial office |
| 14 | `RECYCLE` | The recycling yard |
| 15 | `SUPER` | The supermarket |
| 17 | `STADTZ` | The town, third view (the macro shares its name with 8) |

Rooms are wider than the 320-pixel viewport: the click and route tables
carry world x coordinates up to 627, and the screens scroll (`->SCRX`,
`->SCRY`). `_LOCLINK` in module 601 is a 16-cell array after its initial
value — the room adjacency, read by the exits.

The three series are kept apart by word id as well: scene modules define
their words from id 560, click modules from 750, and every macro is 549 —
ids that collide across modules and are valid because a location's three
modules are the only ones of their series resident at a time
([script modules](../../motion16/formats/script-modules.md)).

## Hotspots, items, verbs

A room's hotspots are the records of its item table (35 × 32 bytes,
fields `X1 Y1 X2 Y2 TEXT ITEXT DR DX DY EXIT FITEM MX MY DIR ORDER` —
[blocks](../../motion16/formats/blocks.md)) and the `LD_*` words of its
click module, one per hotspot, plus `DONIX`. The inventory is a second
table, `_FITEM`, 60 records of 10 bytes (`NAME ORDER GFX INFO DIR`) in
module 601; the item constants `IT_ANGEL`, `IT_DOSE`, `IT_ZANGE`, … are in
module 607.

The verbs are module 603's constants — `TAKE`, `EXAMINE`, `HANDLE`, `USE`,
`TALK`, `GIVE`, `INFO`, `LEAVE` — with the classes `TAKEABLE`,
`HANDLEABLE`, `USEABLE`, `TALKABLE`, `LEAVEONLY` and the combinations
`TAKE_HANDLE`, `TAKE_USE`, `TAKE_USE_HA`. The kernel's `DOORDER` is the
hook the verb bar calls into; `ORDERMAKE` and `FORCE_ORDER` (603), the
`CALCTAKE` … `CALCLEAVE` words (606) and each scene's `MYCALCMOVE`,
`MYCALCEXAMI`, `MYCALCHANDL`, `MYCALCUSE`, `MYCALCLEAVE` do the game's part.

## Talking

There is no dialogue engine in this kernel — the 32-bit engine's
conversation apparatus is absent. Talk is Forth: the message pipes of
module 606 (`_DAVEPIPE`, `_MESSPIPE`), `->DIAL` and `XCALCTALK` in 614, the
`dial01`–`dial57` variables and the face words (`FSUE`, `FWAMI`, …) in 607,
and the kernel's `ADDMESSPIPE`, `SDTXT`/`SDTB` for the words on screen.

## Walking

Module 604 — `XYWALK`, `DESTWALK`, `WALKER`, `PSETWALK`, `PINITFIG`,
`WALKON`/`WALKOFF`, `FREEZE_PERS` — over the kernel's `DOWALK` and
`STEPMULTI`; the direction tables come from module 651 (`XDIR`, `MGG`,
`MKB`, …), copied into `DO_XDIR` at boot; the walk-sprite sets per figure
and direction are in 606 (`EVI_BIG`, `DAVE_SMALL`, `_KBVORN`, …). Each
person is drawn through an off-screen buffer of its own (`NEWPERS`,
[buffers](../../motion16/engine/buffers.md)).

## Saving

Five slots, 701–705. `RUN` probes them with `=>EXIST`; the menu in module
650 (`SHOW_FILES`, `DOINVSAVE`, `REMOVE_FILE`, `KILL_MENU`) saves and loads
with `=>PUTAS`/`=>GETAS` and `PUTANIM`/`GETANIM`. The engine names the three
files of a slot after `#F0R3i.frz`, `#F0R3i.anm`, `#F0R3i.blk`. No save is
on hand; what a save keeps, byte for byte, is open.

## Open questions

- The meaning of the item record's `DR`, `DX`/`DY`, `EXIT`, `ORDER` fields
  and of the route links; the `_LOCLINK` graph.
- What `UPLOAD_GAME`, `INCLPERM`, `DEF_CHARS`/`DEF_ITEMS` initialize.
- The save-file bytes.

## See also

- [Module map](module-map.md) — every module
- [Resource inventory](inventory.md) — the counts
- [Boot and frame loop](../../motion16/engine/boot-and-loop.md) — `RUN`, `CTRL`, `INCLLOC`
- [Game structure (Dunkle Schatten 2)](../ds2/game-structure.md) — the 32-bit game's scheme for comparison
