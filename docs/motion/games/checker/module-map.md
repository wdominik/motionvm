[← Documentation index](../../README.md)

# Module Map

*Checker 2000 — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

The game ships 119 script modules, numbered as [Dunkle Schatten 2](../ds2/module-map.md)'s
are and by the same authoring template: fixed infrastructure below 100, and
each location *N* owning up to three modules — *100+N*, *200+N*, *300+N* —
that `INCLLOC` loads when the location is entered and unloads when it is
left ([game structure](game-structure.md)). Locations run from 1 to 39 with
16 and 19 missing; 18 is the shell's board rather than a scene.

## Infrastructure modules

| Module | Words | Contents |
|---|---|---|
| 2 | 92 | **Global state**: the screen and background handles (`_SCREEN`, `_BG`), the two caption descriptors and the talking head (`_TI1`, `_TI2`, `_CHKM`), the three fonts (`_F1`–`_F3`), the task machine's cells (`_TASK`, `_SUBTASK`, `_TASKWAIT`, `_LOCTASK`, `_LTHANDLER`, `_ANHANDLER`), the speech cells (`_SPEECH`, `_SMP1`–`_SMP4`, `_ACTSPEECH`, `_TSPEED`), the speaker and route tables, the shell's board cells (`_IBNR`, `_IBON`, `_IBSELECT`, `_BTASK`, `_LTASK`), the registration and highscore (`_PLZ`, `_REGIONAL`, `_NAME0`–`_NAME5`, `_DATE1`–`_DATE5`, `_SCORE1`–`_SCORE5`, `_GSCORE`), and the core helpers Dunkle Schatten 2 also has — `++`, `--`, `,,`, `+@`, `+!`, `0!`, `SMDESC`, `0SDWAIT`, `XYLSITEM.` |
| 3 | 2 | `STARTUP` and `ENDGAME` — loaded, run, and unloaded again |
| 4 | 7 | **Boot and shell**: `START`, the control handler `ICTRL`, the task machine `TASK_CTRL` (1909 cells — the story's whole sequence is in it) and its entry `TASK_START`, the speech pump `SAMPLE_TIMING` and `_ACTSPEAKER` |
| 5 | 23 | **The game library, the part this game uses**: `INCLLOC`, the walk queue (`WALKQUEUE`, `FOLLOWMAN`, `PINITFIG`, `PSETWALK`), the caption words (`SETT1`, `SETT2`, `?READYT1`, `?READYT2`, `TSX`, `TS`), the speech sequence (`->SPEECHSEQ`, `SPEECHSEQ->`, `->SPEAKER`) and the talking head (`TON_CHKM`, `TOFF_CHKM`, `SAY_CHKM`, `SAY_INSERT`, `DOCHKM`) |
| 6 | 133 | **The animation runtime** — the same word set as Dunkle Schatten 2's [module 6](../ds2/library/animation.md): `INITANI`, `SHOWANI`, `STARTANI`, `STOPANI`, `FREEZEANI`, the `SDANI*`/`GDANI*` setters and getters, the range and pointer words |
| 7 | 241 | **The story's figures and props**: state cells (`_HUNDSTAT`, `_BALLSTAT`, `_SCHWSTAT`, `_JENSTAT`, `_MALSTAT`, …) and the animation and route definitions of Jenny, Malte, Tom, the dog, the ball, the broom, the bucket. Loaded by location 5 and erased when it is left — `INCLLOC` has `_ACTLOC @ 5 = IF 7 =>ERASE` |
| 99 | 4 | **A test module from the authoring environment**: `TEST1` (`TEMAKE` in a loop), `V1`, `_GFXNR`, and `X3` — `640x480x256 SETRES TOGFX … ->SCREEN KEY GFXTO`, the one place in the game's files where `SETRES` occurs. Never loaded by the game |

## Location modules

For location *N*:

| Series | Role | Present for |
|---|---|---|
| 100+N | The **description handler** — one word, `_BRINK`, or `EMPTY_BRINK` in 102, or none at all in 103 and 120–139 | 101–115, 117, 118, 120–139 |
| 200+N | **Scenes and games**: the cast's animation and route definitions, the dialogue tables, and — for the story locations — the task manager `LTMANAGER` | 201–215, 217, 218, 220–239 |
| 300+N | The **scene macro** that builds the location: the background descriptor, `SETPAL`, the tune, `_LTHANDLER`, `_LOCTASK` | 301–315, 317, 318, 320–339 |

The 200-series splits in two by what its words are named for. **201 to 218**
are the story: they export the cast (`TOM`, `JENNY`, `MALTE`, `VICKY`,
`MEISTER`, `LEHRER`, `REPORTER`, `BRINK`, `HILDE`, `KOLLEGE`), their walk
sets (`W1`–`W5`), dialogue tables (`_DTABLE1`, `_DTABLE2`) and each a
`LTMANAGER`. **220 to 239** are the twenty mini-games, and they export a
clock and a score (`_TIME`, `_SCORE`), the game's own state (`_CARDS`,
`_UCARDS`, `_QUEST`, `_TDIGIT`, `_KLAMMER`, `_HEIGHT`, `_STRNGTH`, …) and a
score display (`_GDISP`, `_GSCO`, `_SCORESTAT`, `_BLINKSTAT`), and no
`LTMANAGER`. Module 218 is the shell's own location — the boards, the
registration, the highscore, saving — and the largest of the story series
at 82 words ([game structure](game-structure.md#the-boards)).

The scene macros name their locations, and the 300-series is the shortest
list of what the story game consists of:

| Location | Macro | Background | Palette | Tune |
|---|---|---|---|---|
| 1 | `START_BRINK` | 101 | 1 | |
| 2 | `START_OMA` | 102 | 2 | |
| 3 | `START_KELLER` | — | 3 | 62 |
| 4 | `START_AOK` | 104 | 4 | |
| 5 | `START_BRINK` | 106 | 5 | 64 |
| 6 | `START_VOR` | 462 | 41 | 60 |
| 7 | `START_KLASSE` | 470 | 7 | |
| 8 | `START_BANK` | 160 | 8 | |
| 9 | `START_ZW` | 161 | 16 | |
| 10 | `START_MALTE` | 162 | 10 | |
| 11 | `START_KRANK` | 163 | 11 | |
| 12 | `START_STRAND` | 164 | 12 | |
| 13 | `START_GARAGE` | 165 | 13 | |
| 14 | `START_MACRO` | 166 | 14 | |
| 15 | `START_BILD` | 167 | 15 | |
| 17 | `START_VOR` | 368 | 17 | |
| 18 | `START_VOR` | 462 | 41 | 64 |
| 20–39 | `START_GAMEB`, `START_GAMEC`, `START_GAMEA`, `START_GAMED` … `START_GAMET` | 2300, 2400, 2500, 4450, 2700, 2800, 4800, 2900, 2950, 2450, 2850, 2350, 3100, 2250, 2050 where the macro places one | 20–39 | |

The background column is the graphic id of the first `NEWSETDESC` the
macro makes; a dash is a macro that builds its picture some other way. The
names are the authors': *Keller*, *Klasse*, *Bank*, *Krank*, *Strand*,
*Garage*, *Oma* are a cellar, a classroom, a bench or a bank, a sickroom, a
beach, a garage and a grandmother, and what each scene is about is in text
table 2's dialogue, not read here. Location 18's macro is the shell's board
and is what `STARTUP` sends the game into first ([game structure](game-structure.md)).

## What the modules use

Every one of the 119 modules disassembles through its own binary's kernel
table, and the modules reach for 138 of its 375 words, 91 % of the uses being
the interpreter's own — `_PutLit`, `@`, `!`, the branches. The domain words
they lean on most are `SDSPR` (749 uses), `SDINACTIVE`, `SDACTIVE`,
`SDAUTOBUF`, `ACTDESC` and `XGFXVFLIP`; the words this game reaches that
Dunkle Schatten 2 never does are the timer set (`OPENTIMER`, `SETTIMER`,
`GIVETIMER`, `CLOSETIMER`), the sample words (`STARTSAMPLE`,
`->STARTSAMPLE`, `STOPSAMPLE`, `?STIME`, `MUSVOLUME`), the scroll slide
(`->SCRX`, `->SCRY`, `SCRY`), `WHITEBOX`, `GIVEDATE`, `TEXT->PRINTER` and
`SDINSERT` ([kernel words](../../motion32/vm/kernel-words.md)).

## See also

- [The game](README.md)
- [Game structure](game-structure.md)
- [Other shipped files](other-files.md)
- [Dunkle Schatten 2's module map](../ds2/module-map.md) — the same template, fully read
