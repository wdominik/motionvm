[← Documentation index](../../README.md)

# Module Map

*Jeff Jet - Abenteuer InfoHighway — this page describes the game's own script modules. The engine they run on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

The game ships 55 script modules, numbered on the same template Die
Enviro-Kids greifen ein follows: the boot module is 100, the resident library
lives at 600–651, and each location *N* owns three modules — *100+N* (scene),
*300+N* (room macro), *500+N* (click handlers) — loaded when the location is
entered (see [Game structure](game-structure.md)). Word counts and sizes are
measured from the modules; a module's id range is the global word ids it
defines.

## Infrastructure modules

| Module | Ids | Words | Bytes | Contents |
|---:|---|---:|---:|---|
| 100 | 400–401 | 2 | 2912 | **Boot and frame handler** — `RUN` (id 401), the word the container header names, and `CTRL` (id 400), the in-game per-frame handler installed with `400 SCRCTRL`. Never erased |
| 600 | 800–870 | 71 | 1760 | **Core helpers** — the small integers as constants (0–20, the even numbers to 30, −1, 100, 1000), store and arithmetic shorthands (`+@`, `+!`, `0!`, `1+`, `2*`, `DUP0`…`DUP6`, `++`, `--`, `==>`, `,,`), and the descriptor builders `XYLSITEM.`, `XYLBITEM.`, `NEWTEXT.`. Word for word the other game's module 600 |
| 601 | 900–1014 | 115 | 5554 | **Global state** — `STARTLOC`, `ACTLOC`, `LASTLOC`, `NEXTLOC`, `_LOCLINK`, `LOCINIT`; the screens `_MS`/`_IS`; the mouse and key variables; `_ORDER`; the per-location arrays `_LDITEM`/`_ROUTE`/`_XROUTE`/`_KLICKAREA` and the inventory `_FITEM` with their size constants; the click-class flags `CLTAKE`…`CLLEAVE`; `_SHFONT`, `_INVMODE`, `DO_XDIR` |
| 602 | 1025–1028 | 4 | 278 | `CALCARROWS`, `CALCINV`, `ADDITEM`, `SUBITEM` — inventory bookkeeping |
| 603 | 1200–1320 | 121 | 4408 | **Verbs, records, tasks** — the verb constants `TAKE` `EXAMINE` `HANDLE` `USE` `TALK` `GIVE` `INFO` `LEAVE` and the classes `TAKEABLE` … `LEAVEONLY`; the item-record accessors `.LDITEM.`, `->LDX1` … `->LDORDER`; the inventory accessors; `SMDESC`, `SSACT`, `XDEFTDT`, `XSETPAL`, `FATMOUSE` |
| 604 | 1400–1431 | 32 | 2536 | **Walking** — `XYWALK`, `DESTWALK`, `WALKER`, `PSETWALK`, `PINITFIG`, the walk queue, the freeze pair `FREEZE_PERS`/`UNFREEZE_PE`; the script side of the kernel's `DOWALK` |
| 605 | 1100–1103 | 4 | 708 | `INCLLOC` (the location loader), `NEWPERS`, `SETINFOTEXT`, `TSC` |
| 606 | 1600–1671 | 72 | 10294 | **Speech and the verbs' work** — the message pipes for the two characters (`_DAVEPIPE`, `_DONNAPIPE`, `_MESSPIPE`), `UPLOAD_GAME`, the verb calculators `CALCTAKE` … `CALCLEAVE`, the walk-sprite sets (`_KBVORN`, `_KBLINKS`, `DAVE_SMALL`, `_DONNAVORN`, …), `WALK_DONNA` |
| 607 | 1780–1974 | 195 | 17096 | **Items, story state, documents** — the item constants (`SUMM`, `FAPPARAT`, `SAPPARAT`, `MAGNET`, `WALKIE`, `ROHR`, `WODKA`, `EIS`, `DIAMANT`, `PERLE`, `KOHLE`, `DISKETTE`, `KOSTÜM`, the five `BUCH_*` letters, …), the dialogue fields, `_DOC_TABLE`, `_DOSAVE`, `_?STARTUP` |
| 609 | 1452–1456 | 5 | 212 | `GLOBAL_TASK`, `XSAYDAVID`, `->DAVID` |
| 610 | 1050–1065 | 16 | 1268 | **The intro** — `STARTINTRO` and its frame handler `ICTRL`, the two Jeff figures and the caption variables. Loaded by `RUN`, run, erased |
| 611 | 1080–1080 | 1 | 2104 | `INCLPERM` — loaded, run once, erased |
| 612 | 1450–1451 | 2 | 1726 | `DEF_CHARS`, `DEF_ITEMS` — loaded, run once, erased |
| 614 | 895–897 | 3 | 2370 | Dialogue helpers `->DIAL`, `CSET`, `XCALCTALK` |
| 650 | 420–431 | 12 | 2506 | **The save/load menu** — `SHOW_FILES`, `DOINVSAVE`, `DOINVDOC`, `REMOVE_FILE`, `KILL_MENU`; loaded when a save exists at startup and from the inventory |
| 651 | 1680–1680 | 1 | 2858 | `XDIR`, the walk-direction table — loaded by `RUN`, its id copied into `DO_XDIR`, erased at once |

Modules 608 and 613 do not exist, and neither does a newspaper module: where
the other game has 615, this game has nothing.

## Location modules

Thirteen locations, 1 to 13, with no gap. Every one has the same three
modules.

The **scene** module 100+N defines its words from id 550 up: the room's
screen-state and sprite variables (`_SCRSTAT`, `_BG1`…`_BG6`, the `_SC_*`
per-object flags), the room's talk words, and the `MYCALC*` handlers
(`MYCALCMOVE`, `MYCALCTAKE`, `MYCALCUSE`, `MYCALCLEAVE`, `MYCALCGIVE`,
`MYCALCINFO`) the verb machinery calls. It stays resident while the location
is active.

The **macro** module 300+N defines one word, always id 549 and always named
`ZEIT`, which builds the room — palette, backgrounds and sprites through
`XYLBITEM.`/`XYLSITEM.`, buffers and screens. `INCLLOC` loads it, runs it
through `LOCINIT EXECUTE`, and drops it again.

The **click** module 500+N defines one word per hotspot, from id 750 up, named
`LD_`*noun*. They are the entries the click areas in block 800+N point at.

| N | Scene 100+N | Macro 300+N | Click 500+N — hotspot labels |
|---:|---:|---:|---|
| 1 | 6632 B, 57 words | 938 B | `LD_SUMM` `LD_FAPPARAT` `LD_SPEAK` `LD_MALER` `LD_FOTO` `LD_RAM` `LD_DRUCK` `LD_WASCH` `LD_FOTOS` `LD_RADIO` `LD_SCHRANK` `LD_BELICHTE` `LD_DONNA` |
| 2 | 5052 B, 31 words | 764 B | `LD_SFOTO` `LD_VIRUS` `LD_ARBEITER` `LD_SPEAK` `LD_SPEICHER` `LD_PROZ` `LD_SAPPARAT` `LD_DONNA` |
| 3 | 3152 B, 26 words | 718 B | `LD_ROHR` `LD_SPEAK` `LD_SCAN` `LD_PROZ` `LD_RAM` `LD_KOSTÜM` `LD_DONNA` |
| 4 | 5918 B, 54 words | 2036 B | `LD_MAGNET` `LD_THRON` `LD_ROHR` `LD_MODEM` `LD_PLADDE` `LD_CPU` `LD_TALKIE` `LD_RAM` `LD_MFELD` `LD_DONNA` |
| 5 | 3440 B, 28 words | 652 B | `LD_BUCH` `LD_DPLATTE` `LD_SPEAK` `LD_SCAN` `LD_MODEM` `LD_SETZER` `LD_TYPEN1` `LD_TYPEN2` `LD_1LDRUCKE` `LD_2LDRUCKE` `LD_DONNA` |
| 6 | 6378 B, 52 words | 1512 B | `LD_AUTOBAHN` `LD_1SCHRANK` `LD_2SCHRANK` `LD_SPEAK` `LD_WÄRTER` `LD_HAUS` `LD_ROHR` `LD_ROHR2` `LD_DRUCK` `LD_PROZ` `LD_BITS1` `LD_BITS2` `LD_LASTER1` `LD_LASTER2` `LD_DONNA1` `LD_DONNA2` |
| 7 | 5590 B, 38 words | 1004 B | `LD_SCAN` `LD_DRUCK` `LD_COMPI` `LD_MODEM` `LD_DOSEN` `LD_WODKA` `LD_PINGUIN` `LD_FRIDGE` `LD_FENSTER` `LD_TÜRA` `LD_TUER_P` `LD_FORSCHER` `LD_DOSE` |
| 8 | 1932 B, 22 words | 660 B | `LD_HUND` `LD_PINGUIN` `LD_BLOCK` `LD_EIS` `LD_WEHE` `LD_WAND` `LD_TÜR` |
| 9 | 9816 B, 59 words | 1714 B | `LD_SCAN` `LD_DRUCK` `LD_MODEM` `LD_COMPI` `LD_TUSSI` `LD_VASE` `LD_TISCH` `LD_BÜRSTE` `LD_SCHERBE` `LD_LACK` `LD_DÜNE` `LD_SÄULE` `LD_TÜTE` `LD_PLANE` `LD_PINSEL` `LD_STATUE` `LD_DIAMANT` `LD_PROF` `LD_GRABUNG` `LD_PROF1` |
| 10 | 9748 B, 100 words | 2502 B | 33 labels — `LD_HELM` `LD_ANZUG` `LD_SCHUHE` `LD_SCHLEUSE` `LD_MUSCHEL` `LD_NEPTUN` `LD_PERLE` `LD_WRACK` … the largest room in the game |
| 11 | 6022 B, 51 words | 1426 B | `LD_SCAN` `LD_DRUCK` `LD_COMPI` `LD_MODEM` `LD_SCHLEUSE` `LD_CODE` `LD_TEFLON` `LD_FENSTER` `LD_RATTE` `LD_SCHAF` `LD_HEIZ` `LD_TUER` `LD_FADEN` `LD_TOM` |
| 12 | 2726 B, 22 words | 616 B | `LD_KESSEL` `LD_KOHLE` `LD_STANGE` `LD_LABOR` `LD_HEIZER` `LD_SUMM` `LD_KOHLE2` |
| 13 | 1830 B, 23 words | 516 B | `LD_TÜR` `LD_SCANNER` `LD_SCHUB` `LD_COMPUTER` `LD_MODEM` `LD_DRUCKER` |

Location 13 is where the game begins: it is the room Jeff sits in, with his
own scanner, computer, modem and printer, and it is the only one whose
hotspots are all ordinary furniture. The other twelve are inside the machine.

## See also

- [Game structure](game-structure.md) — how the three module series are used
- [Resource inventory](inventory.md) — the counts behind this page
- [Boot and frame loop](../../motion16/engine/boot-and-loop.md) — `RUN`, `CTRL`, `INCLLOC`
- [Module map (Die Enviro-Kids greifen ein)](../enviro/module-map.md) — the same template, sixteen locations
