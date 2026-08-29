[← Documentation index](../../README.md)

# Module Map

*Hilfe für Amajambere — this page describes the game's own script modules. The engine they run on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Seventy-six modules: one boot module, twenty locations with three modules each,
and fifteen of shared library. The numbering is the authoring template both
sibling games follow, for more locations than either has. What each location
owns besides its modules — its four blocks — is in the
[resource inventory](inventory.md); how they are loaded and dropped is in
[boot and frame loop](../../motion16/engine/boot-and-loop.md).

## Infrastructure modules

| Module | Ids | Words | Bytes | Contents |
|---:|---|---:|---:|---|
| 100 | 400–401 | 2 | 3040 | **Boot and frame handler** — `RUN` (id 401), the word the container header names, and `CTRL` (id 400), the in-game per-frame handler installed with `400 SCRCTRL`. Never erased |
| 600 | 800–870 | 71 | 1760 | Screen and descriptor helpers |
| 601 | 900–1014 | 115 | 5554 | **The game's state** — `STARTLOC`, `LASTLOC`, `ACTLOC`, `NEXTLOC`, the `_LOCLINK` map, `_LOADTABLE`, and the table constants `A_LDITEM` 35, `S_LDITEM` 32, `A_FITEM` 50, `S_FITEM` 10. The location variables the frontend moves through are these |
| 602 | 1025–1028 | 4 | 278 | Inventory arithmetic — `CALCINV`, `ADDITEM`, `SUBITEM` |
| 603 | 1200–1322 | 123 | 4480 | **Verbs and hotspot classes** — `TAKE` 1 … `LEAVE` 8, `TAKEABLE` 3, `HANDLEABLE` 6, `USEABLE` 10, `TALKABLE` 18, `LEAVEONLY` 128, and the verb machinery over them |
| 604 | 1400–1431 | 32 | 2536 | Walking — `WALKQUEUE`, `_DESTX`, `_DESTY` |
| 605 | 1100–1103 | 4 | 602 | **The location loader** — `INCLLOC` (id 1102, 171 cells), which erases the old location's three modules, loads the new one's, and reads its four blocks; plus `SETINFOTEXT` and `NEWPERS` |
| 606 | 1600–1671 | 72 | 7390 | Per-character conversation state — `_DAVESTR`, `_DAVEINFO`, `_DAVEPIPE` and their siblings |
| 607 | 1780–1922 | 143 | 10878 | **Items and documents** — the named inventory words (`MAP`, `SOJA`, `ZETTEL`, `GLAS`, `KOHLE`, `BLEI`, `STIEFEL`, `REAGENZ`, …), `_DOC_TABLE`, `_DOSAVE`, `_DIALFIELD` |
| 609 | 1452–1456 | 5 | 302 | Standing positions the characters are placed by |
| 610 | 1050–1067 | 18 | 892 | **The intro**, fetched and erased by `RUN` |
| 611 | 1080–1080 | 1 | 2076 | `INCLPERM` — the permanent character definitions, fetched and erased by `RUN` |
| 612 | 1450–1451 | 2 | 1094 | `DEF_CHARS` and `DEF_ITEMS`, fetched and erased by `RUN` |
| 614 | 895–897 | 3 | 4866 | Conversation layout — `CSET`, `XCALCTALK` |
| 650 | 420–431 | 12 | 2520 | **The menu** — `SHOW_FILES` (id 421) with the `706 701 DO I =>EXIST … LOOP` slot probe, `SHOW_DOC`, `REMOVE_DOC`. `RUN` fetches it because this game starts on location 20; the sibling games fetch it only when a save exists |
| 651 | 1680–1680 | 1 | 2818 | The walk-direction table, read once by `RUN` through `DO_XDIR` and erased immediately |

608 and 613 are unused, as they are in Die Enviro-Kids greifen ein.

## Location modules

Twenty locations, 1 to 20, with no gap. Every one has the same three modules.

The **scene** module 100+N defines its words from id 560 up — the room's
screen-state and sprite variables, its talk words, and the handlers the verb
machinery calls. It stays resident while the location is active. Location 20,
the game's front page, is the exception that defines from 550.

The **macro** module 300+N defines one word, always id 549, which builds the
room — palette, backgrounds and sprites, buffers and screens. `INCLLOC` loads
it, runs it through `LOCINIT EXECUTE`, and drops it again.

The **click** module 500+N defines one word per hotspot, from id 750 up, named
`LD_`*noun*. They are the entries the click areas in block 800+N point at.

| N | Scene 100+N | Macro 300+N | Click 500+N — hotspot labels |
|---:|---:|---:|---|
| 1 | 16094 B, 118 words | 3272 B | `LD_JOHN` `LD_FAHRER` `LD_SAX` `LD_KALA` `LD_2FRESS` … (+15) |
| 2 | 318 B, 4 words | 490 B | `LD_DORF1` `LD_MERCY` `LD_ROBERT` |
| 3 | 1362 B, 19 words | 944 B | `LD_BRUNNEN` `LD_CHIEF` `LD_2OBJECTD` `LD_FELD3` `LD_ELSA` |
| 4 | 616 B, 5 words | 634 B | `LD_0OBJECTH` `LD_MIHA` `LD_FELD` `LD_3OBJECTH` `LD_ANTHONY` … (+2) |
| 5 | 1208 B, 16 words | 684 B | `LD_1OBJECTM` `LD_2OBJECTM` `LD_3OBJECTM` `LD_SOJA` `LD_DEAL` … (+1) |
| 6 | 3970 B, 7 words | 748 B | `LD_KARTE` `LD_DANIEL` `LD_BOMBA` `LD_SAX` |
| 7 | 90 B, 1 word | 262 B | `LD_KARTE` `LD_1OBJECTK` `LD_FLUG` |
| 8 | 434 B, 4 words | 404 B | `LD_0OBJECTH` `LD_1OBJECTH` `LD_2OBJECTH` `LD_3OBJECTH` `LD_ZAUN` |
| 9 | 6366 B, 53 words | 1224 B | `LD_FLUG` `LD_JEEP` `LD_KARTE` `LD_SAX` |
| 10 | 798 B, 11 words | 644 B | `LD_CHIEF` `LD_PFERCH` `LD_FASS` `LD_1FRESS` `LD_2FRESS` … (+2) |
| 11 | 1842 B, 2 words | 258 B | `LD_ZURÜCK` `LD_MIHA` `LD_KIJI` `LD_GOMBE` `LD_MAJI` … (+3) |
| 12 | 2278 B, 15 words | 408 B | `LD_HOFJOSEF` `LD_JOSEFU` |
| 13 | 306 B, 3 words | 334 B | `LD_HOF` `LD_ANTHONY` |
| 14 | 204 B, 2 words | 344 B | `LD_GOMBE` `LD_KUH1` `LD_KUH2` `LD_KUH3` |
| 15 | 2842 B, 19 words | 828 B | `LD_MIHA` `LD_1TOR` `LD_KINDER` `LD_2FAHNE` `LD_2TOR` … (+2) |
| 16 | 9666 B, 68 words | 1922 B | `LD_MAP` `LD_JOHN` `LD_REAGENZ` `LD_REGAL` `LD_TELEFON` … (+18) |
| 17 | 3150 B, 27 words | 552 B | `LD_0OBJECTB` `LD_1OBJECTB` `LD_KARTE` `LD_DANIEL` `LD_BOMBA` |
| 18 | 90 B, 1 word | 256 B | `LD_KIJI` `LD_FELD` |
| 19 | 1104 B, 12 words | 648 B | `LD_HERD` `LD_HOCKER` `LD_KALA` `LD_KILI` `LD_PRINCE` … (+3) |
| 20 | 4164 B, 53 words | 1434 B | `LD_BLA` |

Locations 11 and 20 are the two that are not rooms in the ordinary sense.
Eleven is the map the player travels from — its hotspots are the other places,
`LD_MIHA`, `LD_KIJI`, `LD_GOMBE`, `LD_MAJI` — and twenty is the front page
`RUN` opens on, with one hotspot and the menu over it.

Location 7 is the one the game ships no item table for; see
[game structure](game-structure.md#open-questions).

## See also

- [Game structure](game-structure.md) — what these modules add up to
- [Resource inventory](inventory.md) — the blocks each location owns
- [Boot and frame loop](../../motion16/engine/boot-and-loop.md) — `RUN`, `CTRL`, `INCLLOC`
- [Module map (Jeff Jet - Abenteuer InfoHighway)](../jeffjet/module-map.md) — the same template, thirteen locations
