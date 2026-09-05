[← Documentation index](../../README.md)

# Module Map

*Falsches Spiel mit Eddie M. — this page describes the game's own script modules. The engine they run on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Sixty-two modules: one boot module, fifteen locations with three modules each,
and sixteen of shared library — 600 to 615 without a gap, where the sibling
games leave 608 and 613 unused. The numbering is the authoring template the
three later games follow, two years earlier. What each location owns besides
its modules — its four blocks — is in the [resource inventory](inventory.md);
how they are loaded and dropped is in
[boot and frame loop](../../motion16/engine/game-loop.md).

## Infrastructure modules

| Module | Ids | Words | Bytes | Contents |
|---:|---|---:|---:|---|
| 100 | 400–411 | 12 | 8058 | **Boot, frame handler and menu** — `RUN` (id 411), the word the container header names, `CTRL` (id 410), installed with `410 SCRCTRL`, and the menu the sibling games keep in a module 650: `SHOW_FILES` with the slot probe, `REMOVE_FILE`, `SHOW_DOC`, `REMOVE_DOC`, `KILL_MENU`, and `GEWINNCODE`, which renders the prize code from the score. Never erased |
| 600 | 800–867 | 68 | 1688 | The small constants 0 to 30, `-1`, 100, 1000, and the helpers `=OR`, `=AND`, `+@`, `+!`, `0!`, `DUP0` to `DUP6`, `NEWTEXT.`, `XYLSITEM.`, `XYLBITEM.` |
| 601 | 900–1019 | 120 | 5810 | **The game's state** — `_FASTSTART`, `STARTLOC` (declared 3), `LASTLOC`, `ACTLOC`, `NEXTLOC`, `MAC`, the `_LOCLINK` map, the table constants `A_LDITEM` 35, `S_LDITEM` 32, `A_FITEM` 50, `S_FITEM` 10, `_LDITEM`, the menu's `_INVMODE`, `_LOADTABLE`, the puzzle's `_PUZZON` and `_PFIELD`, the key queue `_KQ`, the play-time counters `_TENMIN` and `_GESTENMIN` |
| 602 | 1020–1023 | 4 | 278 | Inventory arithmetic — `CALCARROWS`, `CALCINV`, `ADDITEM`, `SUBITEM` |
| 603 | 1200–1307 | 108 | 4048 | **Verbs and hotspot classes** — `TAKE` 1 … `LEAVE` 8, `TAKEABLE` 3, `HANDLEABLE` 6, `USEABLE` 10, `TALKABLE` 18, `LEAVEONLY` 128, the item-record accessors `->LDX1` … `->LDORDER`, and the verb machinery |
| 604 | 1400–1429 | 30 | 2472 | Walking — `WALKQUEUE`, `_DESTX`, `_DESTY`, `XYWALK`, `SETWALK`, `SETSTOP`, `SETSTEPS`, `FREEZE_PERS` |
| 605 | 1100–1103 | 4 | 828 | **The location loader** — `INCLLOC` (id 1102, 285 cells), which stops the tune, erases the old location's modules, loads the new one's and reads its four blocks; plus `TSC`, `SETINFOTEXT` and `NEWPERS` |
| 606 | 1600–1657 | 58 | 9328 | The player figure under the template's name — `_DAVE`, `_DAVESTR`, `_DAVEINFO`, `_DAVEPIPE`, `_DAVEINV` — the message pipe, `UPLOAD_GAME`, and the verb handlers `CALCTAKE` … `CALCLEAVE` |
| 607 | 1780–1936 | 157 | 11212 | **Items, answers and the score's hook** — the named inventory words (`PRÄSENTATIO`, `AUDIOTAPE`, `FAX`, `KLEBER`, `PAPPE`, `DIKTIER`, `STERN`, `GLAS`, `PUZZLE`, `SCHNIPSEL`, `FOTO`, `VISITEN`), `SET_POINTS`, the dialogue answer words, `INTROON` |
| 608 | 1040–1046 | 7 | 1582 | **The photograph** — `INIT_FOTO`, `CALC_FOTO`, `EXIT_FOTO` |
| 609 | 1452–1499 | 48 | 2778 | **The date and the score** — `STNR` with its tables `_SKAL`, `_JTABLE`, `_MTABLE`, the score panel `CALC_PPANEL` and `CALC_POINTS`, `GLOBAL_TASK`, and the dialogue data `_D1` … `_D18` |
| 610 | 1050–1059 | 10 | 1484 | **The intro and the ending** — `STARTINTRO`, `STARTEXTRO`, their controllers `ICTRL` and `ICTRL2`; fetched and erased by `RUN`, and by location 9 |
| 611 | 1080–1080 | 1 | 2066 | `INCLPERM` — the permanent character definitions, fetched and erased by `RUN` |
| 612 | 1450–1451 | 2 | 1172 | `DEF_CHARS` and `DEF_ITEMS`, fetched and erased by `RUN` |
| 613 | 1060–1063 | 4 | 1204 | **The slide puzzle** — `INIT_PUZZLE`, `CALC_PUZZLE`, `EXIT_PUZZLE`, `MIX_PUZZLE` |
| 614 | 895–897 | 3 | 2700 | Conversation layout — `->DIAL`, `CSET`, `XCALCTALK` |
| 615 | 890–892 | 3 | 1088 | **The city map** — `INIT_MAP`, `CALC_MAP`, `EXIT_MAP` |

## Location modules

Fifteen locations, 1 to 15, with no gap. Every one has the same three modules.

The **scene** module 100+N defines its words from id 550 up — the room's
screen-state and sprite variables (`_BG1` …), its talk words (`SAY_`*name*),
its characters' walk words and the handlers the verb machinery calls. It stays
resident while the location is active. Location 10 is the exception that
defines from 560.

The **macro** module 300+N defines one word, always id 549, which builds the
room — palette, backgrounds and sprites, buffers and screens — and, for
location 5 alone, starts a tune. `INCLLOC` loads it, runs it through
`LOCINIT EXECUTE`, and drops it again.

The **click** module 500+N defines one word per hotspot, from id 750 up, named
`LD_`*noun*. They are the entries the click areas in block 800+N point at.

| N | Scene 100+N | Macro 300+N | Click 500+N — hotspot labels |
|---:|---:|---:|---|
| 1 | 10576 B, 78 words | 1502 B | `LD_HEADHUNT` `LD_BITT` `LD_PARKBANK` `LD_SCHILD` `LD_KIOSK` … (+6) |
| 2 | 4408 B, 46 words | 1160 B | `LD_TELEFON` `LD_KAFFEE` `LD_SCHRANK` `LD_PROJEKTO` `LD_DRAUSSEN` … (+2) |
| 3 | 7378 B, 43 words | 1720 B | `LD_BTÜR` `LD_BALKON` `LD_BETT` `LD_TELEFON` `LD_KASSETTE` … (+25) |
| 4 | 4734 B, 57 words | 1520 B | `LD_FASTI` `LD_BIANCA` `LD_KRUSE` `LD_RBÜRO` `LD_LBÜRO` … (+20) |
| 5 | 888 B, 12 words | 630 B | `LD_WOHNUNG` `LD_FAST` `LD_GEWÄCHS` `LD_MICRO` `LD_SENDER` … (+5) |
| 6 | 2768 B, 28 words | 486 B | `LD_PARKBANK` `LD_EXIT1` `LD_EXIT2` |
| 7 | 3040 B, 36 words | 1030 B | `LD_TAMARA` `LD_FLÖTTI` `LD_GUNDEL` `LD_KATI` `LD_FAX` … (+17) |
| 8 | 5182 B, 42 words | 602 B | `LD_EXIT1` `LD_EXIT2` `LD_EINGANG` `LD_BÜSCHE` |
| 9 | 3982 B, 38 words | 886 B | `LD_STRASSE` `LD_STERN` `LD_SCHILD` `LD_TRANS` |
| 10 | 10080 B, 78 words | 1200 B | `LD_MÜLLEIME` `LD_TELEFON` `LD_SPRECH` `LD_SCHIFF` `LD_SITZECKE` … (+9) |
| 11 | 10760 B, 64 words | 674 B | `LD_STRASSE` `LD_BOOT` `LD_AUTO` `LD_SCHUPPEN` |
| 12 | 5576 B, 35 words | 652 B | `LD_TÜR` `LD_FENSTER` `LD_TELEFON` `LD_STISCH` `LD_STÜTZE` … (+2) |
| 13 | 860 B, 17 words | 834 B | `LD_FOTO` `LD_IRENE` `LD_SIMONE` `LD_EXIT` `LD_HITLER` … (+10) |
| 14 | 3696 B, 61 words | 1230 B | `LD_KLÖBI` `LD_BARTI` `LD_TIEDJE` `LD_MARKWORT` `LD_IRENE` … (+8) |
| 15 | 8246 B, 64 words | 1628 B | `LD_ZWERG` `LD_TOMMY` `LD_PAGODE` `LD_JÖRG` `LD_ELVIRA` … (+16) |

Three locations are not rooms in the ordinary sense. Five is the city map the
player travels from — its hotspots are the other places. Three is the flat the
game opens in, whose scene module carries the opening sequence under `INTROON`.
And nine, the street at the publisher's house, is where the ending plays: its
scene fetches module 610 for `STARTEXTRO`, rolls the credits words `_CREDITS`
and `_CRWAIT`, and shows the prize code through `XGEWINNCODE`.

## See also

- [Game structure](game-structure.md) — what these modules add up to
- [Resource inventory](inventory.md) — the blocks each location owns
- [Boot and frame loop](../../motion16/engine/game-loop.md) — `RUN`, `CTRL`, `INCLLOC`
- [Module map (Die Enviro-Kids greifen ein)](../enviro/module-map.md) — the same template, sixteen locations
