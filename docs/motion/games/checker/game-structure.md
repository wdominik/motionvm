[← Documentation index](../../README.md)

# Game Structure

*Checker 2000 — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

How the game's 119 script modules organize into a running program: the
startup, the shell and its boards, the registration, the task machine the
story game runs on, and what is saved. For what each module contains, see
the [module map](module-map.md).

## Startup

The bootstrap is the authoring template's: `SYSTEM.RSC` — after a
`->RSCPATH` line of its own, see [other files](other-files.md#systemrsc) —
says `4 =>GET` and `START`, and module 4's `START` begins

```
2 =>GET  5 =>GET  6 =>GET  3 =>GET  STARTUP  3 =>ERASE  …
```

— load the resident modules (2 globals, 5 game library, 6 animation), load
module 3, run its `STARTUP`, unload it. `STARTUP` (395 cells) asks
`?SOUND` and keeps the answer in `_SPEECH`; puts the shell at board 5 with
the board switched on (`1 _IBON !  5 _IBNR !`); registers fonts 5, 6 and 8
as `_F1`–`_F3`; defines nine text templates with `DEFTDT`, each `n 2 2 -1 -1
-1 -1 _F2 @ id`; enters graphics with `TOGFX` — **without a `SETRES`**, and
gets 640×480 because that is the selection the engine's own init seeds
([screens](../../motion32/engine/screens.md#the-video-mode)); makes the one
screen, 640×480 in every dimension, at `0 0 SCRPOS`; and makes the two
caption descriptors `_TI1` and `_TI2` (table 2, text 376, template 1) and
the talking head `_CHKM`.

`START` then goes on:

```
3 ERRORLEVEL  1 SPEEDMODE  700 200 99 526 XYLSITEM. _CHKM !
0x50dac SDWORD  0 SDWAIT  SDAUTOBUF  SDINACTIVE
0x420cc CTRL  8 DELAY  TASK_START  ANIMPLAY
3 =>GET  ENDGAME  3 =>ERASE  2 =>ERASE  5 =>ERASE  6 =>ERASE
```

`ICTRL` (module 4, `0x420cc`) becomes the per-frame controller, `8 DELAY`
sets the frame at 25 ticks of the 200 Hz clock — **eight frames a second**,
where Dunkle Schatten 2's `25 DELAY` runs at twenty-five — `TASK_START`
puts the task machine at `_STASK`, 0, and `ANIMPLAY` is the main loop until
`QUITANIM` ends it; `ENDGAME` then runs and the resident modules are
unloaded.

## The shell

`ICTRL` runs every frame. It reads `?KEY` into `_AKTKEY`; Escape (27) while
no board is up stops the speech, switches the board on at board 1, parks the
task in `_BTASK`, moves the two captions off screen (`-1000 SDX`) and hides
the talking head; key 1348 is `QUITANIM`. Then `TASK_CTRL`.

`TASK_CTRL` (1909 cells) is the game. Every frame it pumps the speech
(`SAMPLE_TIMING`), runs the location's animation handler (`_ANHANDLER`) and,
while `_LOCTASK` is above nought, its task manager (`_LTHANDLER` — each
story location's `LTMANAGER`); counts `_TASKWAIT` down, except while a board
is up; and, when `_TASKWAIT` reaches nought, advances the story by
**`_TASK`**: a chain of `_TASK @ n =` tests, each entering a location with
`INCLLOC`, setting the next wait and stepping `_TASK` on. Task 0 is the
board, location 18; tasks 1 to 98 are the story game; task 99 puts the
board back up at board 3, the highscore; task 100 is `QUITANIM`. With the
board on (`_IBON`) and the game somewhere else (`_ACTLOC @ 18 !=`), the
board's location is entered; a scene can ask for an information page with
`_IBSELECT`, which parks the task and enters the board at that page; leaving
the board sets `1 MUSVOLUME` and takes the task out of `_LTASK`.

The story game's sequence, as `TASK_CTRL` has it — task after task, the
location each enters:

```
 1–10   6  7  8  5 22  5  5 20  5  3
11–20   9  3  9  3  9  3  4 21  4  9
21–30   4 10 10 —  23 10 —  24 10  7
31–40   8  5  3 25  9 26  1 27 11  9
41–50  10 28 10 10 12 29 12  5  5 13
51–60  30 13 11 31 11 13  9  5 32  5
61–70   9  5  9  5  9  5  4 —  33  9
71–80   2  9 14 34 14  9  5  9  9  9
81–90   9  5 35  5 15 10 36 10  5  7
91–98  37  3  4 38  5 39  9 17
```

A dash is a task that sets a wait and no location. The mini-games, locations
20 to 39, are threaded through the story one at a time, each between two
visits to a story location; locations 5 and 9 are the ones the story keeps
coming back to ([module map](module-map.md#location-modules)). One step is
not a step: task 2's entry ends `4 _TASK !` where every other says
`_TASK ++`, so task 3 — location 8, the street with the Quirli billboard —
is never reached from the chain, and the classroom hands straight to the
schoolyard.

### The boards

Location 18 is the shell's own scene, module 218 with its macro in 318.
Which board it shows is `_IBNR`, and `SI2` — fade out, hide the texts,
`SHOWINFO`, fade in — changes it. `SHOWINFO` gives board *n* below 700 the
background `600 + n` and the palette `100 + n`; the information pages from
700 up have fixed pairs (`705`/55, `709`/59, `713`/63, …, and `715`/65 for
750 to 763); a board above 1000 is a regional page, `1000 + 100 × region +
page`, and `SI2` picks its text table by `_REGIONAL` — 257, 261, 264, 258,
263, 259, 260 and 262 for regions 1 to 8 — and the entry by the page.

| Board | What it is |
|---|---|
| 1 | Escape from a scene |
| 2 | The main menu — *Play it!*, *Watch it!*, *Check it!*, *Dance it!*, *Stop it!* |
| 3 | The highscore: five names, dates and scores through `SDINSERT`, below |
| 5 | The registration, below — what `STARTUP` chooses |
| 6 | The *Futurespiel* menu — *Play it!* starts task 1, *Play it?* the instructions, *Load it!* runs `READSAVE`, *Stop it!* |
| 7 | The photo story and the information pages' index |
| 700–763 | The information pages, scrolled with `SDSTARTLINE` fifteen lines at a time |
| 1000– | The regional pages |

The clickable areas of every board are literal `MOUSEX`/`MOUSEY` ranges in
`LTMANAGER`: this build has no `?XINSIDE` ([ENGINE.EXE R78](../../motion32/engine/engine-r78.md)).

The information book's pages stand on a white sheet. `SI2` opens a board
below 700 with `2 50 8 FADEIN` when `_IBMODE` or `_IBMODE2` is set — the
engine's `FADEIN` mode 2, which paints `WHITEBOX`'s box at 25,122, 452 by
317, white with a black frame two in, before its curtain opens, and which
waits between bands on no build — and then activates `_IBTXT` and
`_IBTXT2`, so the page's text is drawn onto the box. A page from 700 up
takes mode 1 and says `25 122 452 317 WHITEBOX` itself, the regional pages
`50 50 540 380 WHITEBOX`; turning a page within a book says it again after
`SDSTARTLINE` moved the window fifteen lines
([transitions](../../motion32/engine/transitions.md#the-mode-argument)).
The first two pages of *Schul-Check* were held against the original
([verification](../../verification.md)).

### The registration

Board 5 shows two texts of table 4 — `#s<`, the name with a cursor after
it, and `#s`, the postcode — with the buffers `_NAME0` and `_INSERT2` in
their insert slots. `LTMANAGER` reads `_AKTKEY`: while `_INITSTAT` is 1,
letters (upper case as they are, lower case raised by 32), the three
umlauts' CP437 codes and space go into `_NAME0` at `_IPPT`, up to ten of
them, and `_NAME0 0 SDINSERT` shows the buffer again; backspace takes one
back; Enter moves the cursor text to the postcode. While `_INITSTAT` is 2,
digits go into `_INSERT2`, up to five; Enter with three or more closes the
entry, reads the **first three digits** into `_PLZ` and looks the region up:

| Region | First three digits |
|---|---|
| 1 | 101–109, 111, 120–126, 130–131, 133–136, 140–144 |
| 2 | 061–069, 555 |
| 3 | 019, 038, 049, 170–174, 180–186, 190, 192–194, 239, 888 |
| 4 | 015, 030–032, 144–149, 152–153, 157–159, 162–163, 165, 167–169, 172, 193, 777 |
| 5 | 294, 384, 388, 391–396, 666 |
| 6 | 080–086, 090–096, 333 |
| 7 | 010–019, 026–029, 444 |
| 8 | 040–048, 222 |

A postcode outside all of them leaves `_REGIONAL` at nought and shows board
715; any other goes to the main menu. The three-digit codes 111 to 888 are
not postcodes and look like the authors' shortcuts into each region.

How the texts show the buffers is the engine's: every set slot is handed to
the `#`-formatter as an address, and the formatter reads the string there
when the text is laid out — so a keystroke shows without a new `SDINSERT`,
though the script makes one anyway
([text rendering](../../motion32/engine/text-rendering.md#the-layout-the-line-window-and-the-inserts)).

### The pointer

The game never says `SHOWMOUSE`, and the shell's `LTMANAGER`s call
`XATMOUSE` only inside the story locations — 718 over a clickable area, 719
elsewhere. What the player points with on the boards is the engine's own
arrow, which `TOGFX` installs and shows on entering graphics: white inside a
dark teal line as the two nearest entries of the system palette, entries 1
and 52 of `ENGINE.RSC`'s palette 0, kept as indices — so once `SHOWINFO` has
put a board's palette in force the arrow wears whatever those two entries
are there, a dark red inside a blue-green line on the registration and the
menu. `NORMMOUSE` — which `ICTRL` says on Escape from a scene and
`TASK_CTRL` when a scene asks for an information page and when the board
hands the story back — resolves the two colors again against the palette of
that moment ([the 32-bit pointer](../../motion32/engine/interaction.md#the-engines-own-arrow)).
Both boards were held against the original with the pointer in the picture
([verification](../../verification.md)).

## Saving

This build has no `PUTANIM` and `GETANIM`, and the game saves no scene.
`SAVESAVE` (module 218) packs 184 bytes — the parked task `_BTASK`, the
score `_GSCORE`, the five highscore names, dates and scores and the player's
own name — into a buffer and writes it as **block 98** with `PUT`;
`READSAVE` reads it back with `98 GET` and *Load it!* puts `_BTASK` into
`_TASK`, so a loaded game resumes at the start of the task it was saved in,
not at the frame. `START_VOR`, the board's macro, runs `READSAVE` the first
time the board comes up, so a fresh copy's very first `98 GET` misses — and
the original answers that with two of the engine's error boxes before the
registration appears, which the [ledger](../../departures.md#the-virtual-machine)
keeps as a departure. motionvm writes the block into the game's save
directory as `098.blk`, which is where its `GET` looks first
([savegames](../../savegames.md)).

The dates come from `GIVEDATE`, the DOS date, spelled out by `DATECONV`
into a `dd.mm.yyyy` buffer; the scores by `SCORECONV` into four digits.
Both go on the highscore board through `#s`, which is why the texts there
are `#s` and not `#i` ([text tables](../../motion32/formats/text-tables.md)).

## Speech

The game is voiced, and it is voiced *instead of* subtitled. `STARTUP`
asks `?SOUND` once — whether the sound layer's digital driver came up —
and keeps the answer in `_SPEECH`; every scene that talks reads that cell
as its macro enters it and puts its task manager on one of two paths:

- **Speech** (`_SPEECH` set; `_LOCTASK` 1001, 1050, 1100, …): `->SPEECHSEQ`
  plays the scene's WAV file out of the `WAVS/` directory `SMPPATH` names
  through `->STARTSAMPLE`, at the layer's full volume and streamed when
  it is over 128 KB — `1_2.WAV` for the classroom's first two lines,
  `4_5.WAV` for the schoolyard's, one file a passage, 73 in all — and
  stores a cue table beside it; `->SPEAKER` binds a figure's mouth to a
  speaker slot. `SAMPLE_TIMING` (module 4) runs every frame while
  `_SPEECH` is set: it reads `?STIME` of the running file, halves it to
  hundredths, and fires each cue's start word when its time comes and the
  speaker's stop word at the cue's end — the talking head's mouth. The
  manager waits at the next step until `SPEECHSEQ->` answers 0, which is
  `?STIME` answering −1 with the DAC run dry, then waits five frames and
  hands the story on. No caption is drawn on this path.
- **Captions** (`_SPEECH` clear; `_LOCTASK` 1, 50, 100, …): the same lines
  are put up as text through `SETT1` and `SETT2` on the two caption
  descriptors, each timed by `TSX` from its length — under 20 characters
  is 20 frames, over 100 a third of the count, over 50 a half, then
  `_TSPEED` percent, 150 here — and the manager moves on when the caption
  has gone. Nothing is played.

So a copy with a sound card never shows the lines it speaks, and a copy
without one never speaks them; 71 sites in fourteen location modules make
the choice. One passage ducks the music under the voice (`0 MUSVOLUME`,
location 3 at task 92), and every task step restores it (`1 MUSVOLUME`).
Beside the voice the scenes play **sound effects** through `STARTSAMPLE`,
16-bit blocks at a quarter of the layer's volume, and these sound *with*
the speech: the layer keeps 34 sample slots and mixes every one that is
taken. Nearly every effect plays once; the beach (location 12, module 212)
starts block 17 with a loop count of −1 under each of its two dialogues and
stops it with `STOPSAMPLE` when the passage is over, so the block runs
under the voices as an ambience, and the schoolyard (module 205) starts a
file the same way on its speech path, which the stream path plays once
whatever the count.
motionvm's sound layer is its audio sink: with one attached `?SOUND`
answers 1 and the game speaks, without one it captions
([audio](../../motion32/engine/audio.md#the-speech-system)). The
classroom's speech — its onset six frames after the curtain, its end, the
scene's step seven frames after that, and the next file starting the
frame the last one ran dry — was held against a recording of the original
([verification](../../verification.md)).

## See also

- [The game](README.md)
- [Module map](module-map.md)
- [Other shipped files](other-files.md)
- [Dunkle Schatten 2's game structure](../ds2/game-structure.md) — the same template's other use
