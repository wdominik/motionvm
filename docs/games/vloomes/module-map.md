[← Documentation index](../../README.md)

# Module Map

*Victor Loomes – Das Spiel — this page describes the game's own script modules. The engine they run on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Thirty-six modules, 125 120 bytes, and a numbering the later games keep only
half of: the boot module is 100 and the resident library lives at 599–610, but
a location owns **two** modules rather than three — *100+N* for its scene and
*20+N* for its macro — and there is no module 601. Word counts and sizes are
measured from the modules; a module's id range is the global word ids it
defines. What each location owns besides its modules — its four blocks — is in
the [resource inventory](inventory.md); how they are loaded and dropped is in
[game structure](game-structure.md#locations).

## Infrastructure modules

| Module | Ids | Words | Bytes | Contents |
|---:|---|---:|---:|---|
| 100 | 400–449 | 50 | 8692 | **Boot, frame handler and location switch** — `RUN` (id 449, the word the container header names), `CTRL` (432, installed with `432 SCRCTRL` and also the save and menu pages), `INCLORT` (427, the location switch), `INCLPERM` (425), the descriptor builders `XYLITEM.`/`XYLBITEM.`/`XYLSITEM.`/`XYLFITEM.`, `SIMPLE_FIG`, `HANDLE_GLOB`, the general menu's tables `_ANLTABLE` and `_ANLXTABLE`, and the constants `LOCINIT` 549, `SIZE_PERINF` 30, `STEPWIDTH` 6. Never erased |
| 599 | 1940–1968 | 29 | 830 | **The item record's fields** — `PIX1` … `PIDIR`, fourteen one-line words adding 0, 2, 4 … 26 to a record address, and the fourteen `->PI*` that store through them plus `->PIALL`, which fills eight fields from the return stack. Fourteen `u16` is the 28 bytes module 600 declares |
| 600 | 1900–1931 | 32 | 3124 | **The per-location arrays** — `_PKITEM` with `A_PITEM` 40 and `SIZE_PKITEM` 28, `PKITEM ( n -- addr )` and the by-index readers `PIX1->`…`PIDIR->`; and `_ROUTE`, `_XROUTE`, `_KLICKAREA` with `A_ROUTES` 30, `S_XROUTES` 12, `A_KLICKAREA` 20, `S_KLICKAREA` 10; `RECTNR`, `XRECTNR`, `KA`, `KAX1`…`KAY2` |
| 602 | 1850–1852 | 3 | 194 | **The top strip's controller** — `PANCTRL` (id 1852), the word `RUN` gives the second screen with `1852 SCRCTRL`, plus `_PANON` and `_BOX` |
| 603 | 510–525 | 16 | 1108 | **The inventory** — `_INV`, `TOINV`, `INVTO`, `?INVINCL`, `?INVEMPTY`, `CALCINV`, `INVSTART`, `RESETINV`, `INVCTRL`, and the arrow state `_GOUP`/`_GODOWN`. The later builds moved this work into the kernel; here it is script |
| 604 | 1000–1136 | 137 | 4766 | **Objects and story state** — the object table `OBJECT` (`A_OBJECT` 80, `S_OBJECT` 10) with ids 2–61 as named constants (`BU_LAMP`, `PISTOLE`, `PAß`, `ZETTEL`, `UNIFORM`, `VERTRAG`, `DIAMANT`, `SCHLÜSSEL`, `WILD_BILL`, `PROFESSOR`, `LBS93LADY`, `LBS00LADY`, …); the condition array `KOND` (`A_KOND` 200) and the bit array `SBIT` with `SETBIT`/`CLRBIT`/`GIVBIT`; and the fifty-odd named readers over them — `AKTBAHN`, `AKTFBAHN`, `DESTINATION`, `FRANK93`, `AKTIENWERT`, `ROHRORDER` |
| 605 | 1400–1565 | 166 | 4586 | **The game's state** — the seven verb constants `O:EXAMINE` 1 … `O:INFO` 7; the three screens `_MS`, `_PS`, `_IS`; the ten `_CALL_*` hooks a location fills, and `_STEADYFUNC`, `_INIT_EX_FI`, `_EXIT_EX_FI` and `_SET_TALKMO` beside them; the location variables **`AO`** (1483), **`?AO`** (1484) and **`NAO`** (1485); `_TIMEZONE`; the fourteen character ids `I_RACHEL` … `I_HÄNDLER` and eleven `_T_*` talk-state tables; the text-speed pair `_TSPEED`/`_TSMODE`; `XSETPAL`, `LOCKSYS`, `MXYSCR` |
| 606 | 1700–1819 | 120 | 4014 | **Core helpers** — the small integers as constants (0–20, the even numbers to 30, −1, 100, 1000, and the screen edges 320, 159, 160, 457, 446), the store and arithmetic shorthands (`+@`, `+!`, `0!`, `1+`, `2*`, `DUP0`…`DUP6`, `==>`), the mouse shadow `__MOUSEX`/`__MOUSEY`, `FATMOUSE`, `SETBUSY`/`SETNOBUSY`, and the reply buffer `.ERBUF`/`UPD_ERBUF` |
| 607 | 900–907 | 8 | 1274 | Eight words, all about the moving figure's frame: `MONI`, `MOVING`, `MOVEMONI`, `CALCMONI`, `GDXX`, `GDOXX`, `GDYY`, `GDOYY` |
| 608 | 700–768 | 69 | 5752 | **The verbs and the dialogue machine** — `DO_ORDER` (746) dispatching to `DO_EXAMINE`, `DO_USE`, `DO_HANDLE`, `DO_TAKE` (723), `DO_TALK`, `DO_GIVE`, `DO_INFO`, each ending on the room's own `_CALL_*` hook; `CREATE_ORDE`, `GIVE_ORDER`, `BREAK_ORDER`; `ADDTOINV`, `SUBTOINV`; and the conversation path `INIT_DIALOG` (303 cells), `INIT_CHOICE`, `NEXTCHOICE`, `SET_ANSWER`, `DO_CHOICE`, `EXIT_DIALOG`. This is the module the later games have no number for — they use 608 for nothing |
| 609 | 1600–1697 | 98 | 8684 | **Walking** — `WALKER` (927 cells) and `NPCWALKING` (569), the walk records `_FIGINFO` and `_NPCINFO`, `SETWALK`/`XYSETWALK`, `INITFIG`, `STOPFIG`, and the `WR*` accessors over an NPC's walk record |
| 610 | 550–569 | 20 | 2886 | **The intro** — `INTRO` (569) and its own frame handler `ICTRL` (567), `ZOOM`, `FADE_IN`, `FADE_OUT`, `ENDINTRO`. `RUN` fetches it, runs it and erases it, and its ids 550–569 are the ones a scene module reuses |

There is **no module 601, 611, 612, 614, 650 or 651**. The location variables
the later games keep in 601 are in 605 here; the permanent setup their 611
carries as `INCLPERM` is a word of module 100's under the same name — the
buffers, the walking figure, the mirror aliases and the talking head's
descriptors; the walk-direction table their 651 hands over at startup does not
exist; and the save and load pages their 650 holds are `CTRL`'s own.

## Location modules

Thirteen locations, 1 to 13, with twelve pairs between them.

The **scene** module 100+N defines its words from id 550 up: the room's
descriptor handles (`_D_TÜR`, `_D_LAMPE`, `_D_LADY`), its per-object state, its
speech words, and the `CALC_*` handlers the verb machinery reaches through the
`_CALL_*` hooks — `CALC_TAKE`, `CALC_EXAMIN`, `CALC_USE`, `CALC_XUSE`,
`CALC_GIVE`, `CALC_INFO`, `CALC_DIALOG`, `CALC_ENDDIA` and `CALCMOVE`. It stays
resident while the location is active.

The **macro** module 20+N defines one word, always id 549 and always named
`ZEIT`, which builds the room: palette through `XSETPAL`, backgrounds and
sprites through `XYLBITEM.`/`XYLSITEM.`, the story flags that decide what is
visible, the walker's starting place, the tune, and finally the ids of the
scene module's handlers into the `_CALL_*` hooks. `INCLORT` loads it, runs it
through `LOCINIT EXECUTE`, and drops it again.

| N | Scene 100+N | Macro 20+N | What the scene module names |
|---:|---:|---:|---|
| 1 | 6564 B, 72 words | 1488 B | `_D_TÜR` `_D_LAMPE` `_D_TELE` `_D_SCHRT` `_D_RACHEL1` `_VL_CHARLIE`, and the six takes `TAKE_KARTE` `TAKE_TINTE` `TAKE_PISTOL` `TAKE_PASS` `TAKE_UMSCHL` `TAKE_PAPIER` — the room `RUN` enters |
| 2 | 15 510 B, 88 words | 2796 B | `_D_BADA` `_D_GAST` `_D_WIBI` `_D_SAXO` `_D_POLY` `_D_DIAMONDS` `_D_GOLD` `_D_BULLET`, `TAKE_GLAS` `GIVE_GLAS` `DO_RASI` `DO_SAXO` `DO_BADA` — the largest module in the game |
| 3 | *shares 2's* | *shares 2's* | No modules of its own; `INCLORT` fetches 102 and 22 for it. Its four blocks are shipped |
| 4 | 8930 B, 72 words | 2448 B | `_D_ROSE` `_D_TÜTE` `_D_MASCHINE` `_D_AUTO` `_D_ZEIT` `_D_PROF` `_D_BULLET`, `DO_AUTO` `DO_PROF` `DO_MASCHINE` `DO_COMIC` |
| 5 | 2432 B, 29 words | 670 B | `_D_UNIF` `_D_ZM` `_D_KANISTER` `_D_KISTE`, `TAKE_KANIST` `PUT_KISTE` `DO_KISTE` |
| 6 | 6830 B, 66 words | 3284 B | `_BAHNLOCK` `_BAHNENTER` `_BAHNBUF` `_CAMEBYBAHN` `_D_JUNGE` `_D_SCHILD` `_D_DOSE` `_D_SÄULE`, `DO_BAHN` — the second transit line's interior, entered as `?AO` 141–179 |
| 7 | 2410 B, 33 words | 2602 B | `_BAHNLOCK` `_BAHNENTER` `_BAHNBUF` `_D_COMIC` `_D_PLAKAT`, `DO_PLAKAT` `DO_BAHN` — the first line's interior, entered as `?AO` 101–139 |
| 8 | 2988 B, 24 words | 394 B | `_SCRPART`, `SETDBÖRSETE` `XTD_BÖRSE` — one speaker and no items |
| 9 | 3086 B, 31 words | 508 B | `_TÜR_AUF` `_D_TÜR` `_D_LADY`, `ÖFFNE_TÜR` `SETLADYTEXT` `XTD_LADY` |
| 10 | 4640 B, 43 words | 984 B | `_CLOUDSTOP` `_D_CHEM` `_D_MANTEL` `_D_W1`…`_D_W5`, `DO_CHEM` `DO_W1`…`DO_W5` `DO_MANTEL` |
| 11 | 3396 B, 32 words | 526 B | `_DOORMODE` `_D_TÜR` `_D_LADY` `_D_CONTRACT`, `SETLADYTEXT` |
| 12 | 5230 B, 42 words | 832 B | `_D_KNACK` `_D_JUNGE` `_D_HÄNDLER` `_D_TONARM`, `DO_HÄNDLER` and the five `_HÄ_DIA*` |
| 13 | 330 B, 7 words | 332 B | `_TIMEMODE` `_D_TIME` `_TIME%`, `DO_TIME_1` `DO_TIME_2` — the time machine, and the only place `SETCYCLE` is called |

Ids are reused across the pairs: every scene module starts at 550, every macro
defines 549, and module 610's 550–569 are the same numbers a scene module
takes once the intro is erased. Modules are therefore loaded and dropped
whole, and a word is addressed by the module it came from.

## See also

- [Game structure](game-structure.md) — what these modules add up to
- [Resource inventory](inventory.md) — the blocks each location owns
- [`LL.EXE`](../../motion16/engine/ll-exe.md) — the build whose kernel these modules bind against
- [Boot and frame loop](../../motion16/engine/game-loop.md) — the family's `RUN`, `CTRL` and location loader
- [Module map (Hilfe für Amajambere)](../hfa/module-map.md) — the later template, three modules to a location
