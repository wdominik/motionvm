[← Documentation index](../../README.md)

# Module Map

*Die Enviro-Kids greifen ein — this page describes the game's own script modules. The engine they run on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

The game ships 65 script modules. Their numbering is systematic: the boot
module is 100, the resident library lives at 600–651, and each location *N*
owns three modules — *100+N* (scene), *300+N* (room macro), *500+N* (click
handlers) — loaded when the location is entered (see
[Game structure](game-structure.md)). Word counts and sizes are measured
from the modules; a module's id range is the global word ids it defines.

## Infrastructure modules

| Module | Ids | Words | Bytes | Contents |
|---:|---|---:|---:|---|
| 100 | 400–401 | 2 | 3226 | **Boot and frame handler** — `RUN` (id 401), the word the container header names, and `CTRL` (id 400), the in-game per-frame handler installed with `400 SCRCTRL`. Never erased |
| 600 | 800–870 | 71 | 1760 | **Core helpers** — the small integers as constants (0–20, the even numbers to 30, −1, 100, 1000), store and arithmetic shorthands (`+@`, `+!`, `0!`, `1+`, `2*`, `DUP0`…`DUP6`, `++`, `--`, `==>`, `,,`), and the descriptor builders `XYLSITEM.`, `XYLBITEM.`, `NEWTEXT.` |
| 601 | 900–1015 | 116 | 5676 | **Global state** — `STARTLOC`, `ACTLOC`, `LASTLOC`, `NEXTLOC`, `_LOCLINK` (a 16-cell array), `LOCINIT` (`CONST 549`); the screens `_MS`/`_IS`; the mouse and key variables `_MX`/`_MY`/`_MLK`/`_MRK`/`_AKTKEY`; `_ORDER` (260 bytes); the per-location arrays `_LDITEM`/`_ROUTE`/`_XROUTE`/`_KLICKAREA` and the inventory `_FITEM` with their size constants; the click-class flags `CLTAKE`…`CLLEAVE`; the buffer and descriptor counters `_LPB`/`_LPD`; `_SHFONT`, `_MEMO`, `_INVMODE`, `DO_XDIR` |
| 602 | 1025–1028 | 4 | 278 | `CALCARROWS`, `CALCINV`, `ADDITEM`, `SUBITEM` — inventory bookkeeping around the kernel's `CCALCINV`/`ADDTOINV`/`SUBFROMINV` |
| 603 | 1200–1321 | 122 | 4544 | **Verbs, records, tasks** — the verb constants `TAKE` `EXAMINE` `HANDLE` `USE` `TALK` `GIVE` `INFO` `LEAVE` and the classes `TAKEABLE` … `LEAVEONLY`; the item-record accessors `.LDITEM.`, `->LDX1` … `->LDORDER`, `LDX1->` …; the inventory accessors `.FITEM.`, `->FNAME` … `->FDIR`; `SMDESC`, `SSACT`, `XDEFTDT`, `XSETPAL`, `FATMOUSE`/`FXATMOUSE`; `SCANITEM`/`FSCANITEM`, `ORDERMAKE`, `FORCE_ORDER`; the location-task words `SETLOCTASK`, `?LTPHASE`, `NEXTLTP`; `SAYDAVID`, `SETBUSY` |
| 604 | 1400–1434 | 35 | 2628 | **Walking** — `XYWALK`, `DESTWALK`, `WALKER`, `PSETWALK`, `PINITFIG`, `WALKON`/`WALKOFF`, the walk queue; the script side of the kernel's `DOWALK` |
| 605 | 1100–1103 | 4 | 626 | `INCLLOC` (the location loader), `NEWPERS` (a person descriptor with its own 100×140 off-screen buffer), `SETINFOTEXT`, `TSC` |
| 606 | 1600–1669 | 70 | 11256 | **Speech and verbs' work** — the Dave/Evi message pipes (`_DAVEPIPE`, `_MESSPIPE`), `->DSTR`, `UPLOAD_GAME` (run once before the first location), the verb calculators `CALCTAKE` … `CALCLEAVE`, the walk-sprite sets (`EVI_BIG`, `DAVE_SMALL`, `_KB*`), `WALKSPR`, `DO_DIR`, `EXITDIALOG`, `DAY2`–`DAY4` |
| 607 | 1686–1944 | 259 | 13688 | **Items, faces, story state** — the `IT_*` item constants (`IT_ANGEL`, `IT_DOSE`, `IT_ZANGE`, `IT_KANISTER`, … `IT_GAZ4`), the dialogue variables `dial01`–`dial57`, the face words `FPOSTBOT`, `FSUE`, `FWAMI`, `FSASCHA`, …, the story flags (`DOSE_TAKEN`, `MüLL_OPEN`, `PRALFLAG`, …), `_DOC_TABLE`, `_DOSAVE`, `_?STARTUP` |
| 609 | 1452–1456 | 5 | 1870 | `GLOBAL_TASK`, `XSAYDAVID`, `->DAVID` |
| 610 | 1050–1087 | 38 | 4660 | **The intro** — `STARTINTRO` and its frame handler `ICTRL` (id 1086), the motif variables `_MOT`–`_MOT5`. Loaded by `RUN`, run, erased |
| 611 | 1092–1092 | 1 | 2104 | `INCLPERM` — loaded, run once, erased |
| 612 | 1449–1450 | 2 | 1762 | `DEF_CHARS`, `DEF_ITEMS` — loaded, run once, erased |
| 614 | 895–897 | 3 | 2380 | Dialogue helpers `->DIAL`, `CSET`, `XCALCTALK` |
| 615 | 1110–1154 | 45 | 3480 | The newspaper and clock — `ZEIT_ON`, `SHOW_ART`, the article variables `_ART1`–`_ART3`, `_ZEP1`–`_ZEP4` |
| 650 | 420–431 | 12 | 2516 | **The save/load menu** — `SHOW_FILES`, `DOINVSAVE`, `DOINVDOC`, `REMOVE_FILE`, `KILL_MENU`; loaded when a save exists at startup and from the inventory |
| 651 | 1670–1685 | 16 | 2252 | Walk-direction tables (`MGG` `MGB` `MKG` `MKB` `EGG` …, `?GDIROFF`) — loaded by `RUN`, copied into `DO_XDIR`, erased at once |

Modules 608 and 613 do not exist.

## Location modules

Every location *N* (1–15 and 17; there is no location 16) has the same
three modules. The **scene** module 100+N defines its words from id 560 up:
the room's screen-state and sprite variables (`_SCRSTAT`, `_BG3`, `_ITEM_S`,
person variables such as `_OPI`, `_SUE`, `_MAIK`), the room's talk words,
and the `MYCALC*` handlers (`MYCALCMOVE`, `MYCALCEXAMI`, `MYCALCHANDL`,
`MYCALCUSE`, `MYCALCLEAVE`) the verb machinery calls. It stays resident
while the location is active.

The **macro** module 300+N defines one word, always id 549, which builds
the room — palette, backgrounds and sprites through `XYLBITEM.`/`XYLSITEM.`,
buffers and screens. `INCLLOC` loads it, runs it through `LOCINIT EXECUTE`,
and erases it again.

The **click** module 500+N defines the hotspot handlers, ids from 750:
`LD_<n><label>` words whose labels are the hotspots of the room, and `DONIX`
("do nothing") last.

| N | Scene 100+N | Macro 300+N | Click 500+N — hotspot labels |
|---:|---|---|---|
| 1 | 24 words, 2512 B | `MUELL` — the dump (Mülldeponie) — the first room, 704 B | 11 words: MÜLLMAN, MÜLLMAN, ZURBIRK, SITZ, FAß, LASTWAG, BULLDOZ, WäRTERH, REIFEN, LACHE |
| 2 | 5 words, 726 B | `BIRKENSTRAß` — Birkenstraße, 342 B | 12 words: GEBÜSCH, TANNE, LATERNE, AUTO, ZUMWALD, VILLA, TüR, TüR, TüR, ZUMSTAD, ZURDEPO |
| 3 | 15 words, 1542 B | `WALD` — the forest, 508 B | 10 words: WILDBAC, SCHILD, FLASCHE, OPI, BANK, PAPIERK, STAB, STEIN, ZURBIRK |
| 4 | 9 words, 1564 B | `WILDB` — the Wildbach stream, 398 B | 13 words: FABRIK, GEBÜSC, BAUM, WILDBAC, ZURPINI, BAUM, BAUM, SCHILF, BAUMSTU, MOND, WEG, WALD |
| 5 | 45 words, 7190 B | `VISALUX` — the VISALUX factory, 1068 B | 10 words: VISALUX, PMUELL, ZUMEINK, ZETTEL, SCHLOSS, CONTAIN, FABRIKG, FALL, MAUER |
| 6 | 36 words, 3952 B | `RAFFKE` — Raffke's, 1136 B | 23 words: FISCHE, WAFFI, SCHUBLA, VERPACK, VALENTI, VERPACK, VERPACK, VERPACK, UNDEFIN, COMPUTE, COMPUTE, STUHL, SCHNIC, ABLAGE, BLATT, OBJECT, KISTEN, GLAS, SCHREI, FALL, FLASCH |
| 7 | 125 words, 10786 B | `EINKAUFSZEN` — the shopping centre, 2080 B | 19 words: BEFESTI, INDENSU, ZUMSTAD, ZIGARET, FLASCH, BLUMEN, TELEFON, SCHAUFE, MüLLEIM, POSTBO, CAFETIS, VISALUX, MüLLEI, MüLLEI, ANNE, MÜLL, FISCHE, BÜCHER |
| 8 | 53 words, 4212 B | `STADTZ` — the town, first view, 1218 B | 11 words: ZUMEINK, ZURSTAD, ZURBIRK, ZURPINI, SCHILD, HECKE, BRUNNEN, SASCHA, BEET, WILDB |
| 9 | 87 words, 9112 B | `STADTV` — the town, second view, 1372 B | 18 words: ZUMFLU, UMSCHL, SUE, ZUMSTAD, TüR, TüR, TüR, TüR, TüR, SCHILD, TELEFON, WAMI, SESSEL, SCHREIB, WAMIS, FALL, ZANK |
| 10 | 14 words, 2878 B | `PINIEN` — the pines, 714 B | 8 words: GULLI, WAFFENS, STADTZE, ZUMBAUM, RECYCLI, LATERNE, STADTAN |
| 11 | 77 words, 10046 B | `WAFFEN` — Waffi's, 2168 B | 24 words: SEIL, SEIL, STEHLAM, FERNSEH, SCHRANK, PAKET, SCHRANK, BüCHER, ZURPINI, WANDUHR, ANNEST, MIXER, KüHLSCH, MESSER, SALAT, BENACH, ESSTISC, ZIMMER, BESENS, MüLLEI, ÄPFEL, WAFFIS, GLAS |
| 12 | 49 words, 6948 B | `BAUMHAUS` — the treehouse, 1414 B | 13 words: SASCHA, SARAH, FENSTER, ANGEL, SKATEBO, TAPEZIE, KARTON, REGAL, PETROLE, GELäNDE, PIRATEN, STRICKL |
| 13 | 66 words, 9956 B | `REDAKT` — the editorial office, 1770 B | 21 words: MAIK, ANDRUCK, TACKER, SCHERE, BLOCK, DIKTIE, GLAS, SCHREIB, KARTON, TELEFON, PAPIERK, ZIMMERP, ZURPINI, LABOR, FAXGERä, AUGUST, KOPIER, STECKD, LAYOUT, SARAH |
| 14 | 31 words, 3204 B | `RECYCLE` — the recycling yard, 494 B | 11 words: FLASCHE, COMPUTE, KARTON, NOCHMEH, SCHROTT, STUHL, TELEFON, ZURPINI, ANNE, FARBE |
| 15 | 27 words, 2210 B | `SUPER` — the supermarket, 848 B | 11 words: ZUMEINK, EINKAUF, PLASTIK, SUPER, FALL, KüHLTRU, SCHILD, DOSEN, DOSEN, KISTE |
| 17 | 74 words, 2732 B | `STADTZ` — the town, third view (the macro carries the same name as 308), 1400 B | 1 word: `DONIX` only |

Hotspot labels are the eleven-character word names with their `LD_` prefix
and index digit removed; they are German and truncated (`ZURBIRK` = *zur
Birkenstraße*, `MÜLLMAN` = *Müllmann*). Location 17's click module holds
`DONIX` alone — the third town view has no hotspots of its own.

## See also

- [Game structure](game-structure.md) — how the series are loaded and what the locations are
- [Script modules](../../motion16/formats/script-modules.md) — the module format
- [Resource inventory](inventory.md) — the counts
