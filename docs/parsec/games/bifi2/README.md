[← PARSEC documentation](../../README.md)

# Bi-Fi 2

*Bi-Fi 2 - Action in Hollywood (1994), a title on PARSEC's `parsec1994` generation, linked against the oldest build of its library that is read.*

## The program

`XP01.EXE` is the game: 164 576 bytes unpacked, `DGROUP` `250Ah`.
Addresses on this page are offsets into that unpacked image, linear
(`seg · 16 + off`), unless a sentence names `XP00.EXE`. `BIFI.BAT` runs
`xp00.exe`, the title program, and `xp01.exe` only when it left
errorlevel 0. `LIESMICH.DOK` and `BIFI.DOK` spell the title as *Bi-Fi 2 -
Action in Hollywood*.

The game is installed here into `BIFI2`, and its key is `bifi2`:
`MOTIONVM_GAMEDATA_BIFI2`, `saves/bifi2/`, `docs/parsec/games/bifi2/`.

The rooms are slots of `RESOURCE/BFROOM.DAT` (`push 0B6Ah`, `0x00A27`),
the FGROUP container Dunkle Schatten's `DSROOM.DAT` is. Beside it the
program opens `PANELS.DAT` (`0x073CD`), `CMBMOU.DAT` (`0x01370`),
`PLAYER.DAT` (`0x0B75F`), `DIALOG.DAT` (`0x009D5`), `PSMP2L.DAT`
(`0x0E5EE`) and `PSMDRV.DAT`, whose resource 5 is the music driver
(`0x0129E`); the title program and the game's end read `INTROS.DAT`.

## The title program

`XP00.EXE` is 62 592 bytes unpacked, `DGROUP` `0DD7h`: an animation, two
stills, the title picture and a menu of three lines, with module 0 of
`PSMP2L.DAT` under them and no backstory
([boot](../../engine/boot.md#bi-fi-2s-title-program)).

## The locations

The location table at `2181h:0000h` has 92 rows of `44h` bytes — the
kind at `+7`, sixteen exits from `+9` with their destinations from
`+19h`, the handler at `+33h`, the name at `+37h`, the rule table at
`+3Bh`, the hook at `+3Fh`, the animation hook at `+43h` and a record
hook at `+47h`. The kinds mean other things than Dunkle Schatten's:

| Kind | Rows |
|---|---|
| 0 | 0, 37 and 38: no places, only the walk on `0060h:04BFh` (`0x00ABF`) |
| 1 | the rooms: the Vorzimmer (5), Rambowsky (7), Jones 1 and Jones 2 (8, 9), the Enterhaken - Brücke and - Maschinenraum (10, 11), the Bi-Fi (12), the Dino Zoo (14), the Technik (15), the Requisite (16), the Denkfabrik (17), the Tontechnik (20), the SFX (21), the Maske (22), the Kantine (23), Schlönstadt's Büro (24), the VIP-Lounge (25), the Vorführraum (26), Öliens (27), the Öliens-Halle (28), the 08-15 (29) |
| 2 | the Flure, one a floor — the Flur Erdgeschoß (18), 1. Stock (19), 2. to 6. Stock (31 to 35), 7. Stock (30), all on one handler `14F9h:00EBh` — and the Lift (36), all in room slot `25h` |
| 4 | the labyrinth, the rows among 40 to 90 that name the one handler `1746h:0226h`, row 69 named *Ausgang* and row 79 *Skelett*; Schlönstadt's Gefängnis (41) and row 46, the cell before its Eisengitter — a row of kind 4 shows slot `26h`, the maze's shared picture, with its own slot's object map bound to it (`0x02EE7`, `0x02F01`) |
| 5 | the Kaffee (1) |
| 6 | row 91, the restore (`0EE6h:0523h`, `0x0F383`), the row a loaded game is sent to (`mov word [200Ch], 5Bh`, `0x0F376`) |
| 7 | rows 2, 3, 6, 13 and 39, without names, each a handler and an animation hook of its own |

The Pforte, row 4, is the gate the Kaffee's first scene leads to.

The global rules are 48 rows at `2336h:0130h` (`0x23490`), every handler
in segment `0FE3h`; the walk verb's rule is verb `0Ch`, row 46. The shared
hook is `09B6h:186Ch` (`0x0B3CC`). Four locations install gates of their
own: the Kaffee's `107Ah:0008h`, the Dino Zoo's `1407h:0067h`, the
Flure's `14F9h:01B4h` and the Tontechnik's `1523h:0009h`; and the door's
gate the walk verb's rule installs, `0FE3h:000Eh`, is the program's own —
doors `64h` to `6Dh` wait for an animation, and the way out trades item
`2Dh` for `2Ch` (`0x0FE88`, `0x0FF17`).

## The start

The location word `[200Ch]` is 0 in the image and the start's load leaves
it so without a saved game. The main loop calls the walk on for a row of
kind 0 (`0x0074B`–`0x00768`), and this build's walk on is its own: from
the row in `[200Ch]` it steps over kinds 0, 4, 6 and 7 (`dec bx ; cmp bx,
4 ; ja`, `0x00AD4`–`0x00AD8`), enters a row of kind 5 at once with entry 0
(`0x00AE1`–`0x00AEB`), and a row of kinds 1 to 3 with the first exit byte
of the row the main loop last ran (`0x00B25`–`0x00B3C`). From row 0 that
is row 1, the Kaffee, on entry 0.

The Kaffee is the conversation a fresh game opens with and the story
comes back to between its chapters: the entry it is entered by, 0 to 8,
is the scene that plays (`0x107D7`), and the row at `DS:182Ch` names the
location each entry goes on to — the Pforte, location 4, from entry 0
(`0x10920`).

## Its library build

The dispatch, the record fields, the rule format and the handler's shape
are Dunkle Schatten's; many of the library's routines are an earlier
version, and which one each is can be read off its body. Against Dunkle
Schatten's — and against Captain Zins's too, unless a line says the two
agree — this build:

- loads one strip file and one figure file whose names carry no digit,
  `PANELS.DAT` and `PLAYER.DAT`; the figure's loader `0B73h:0003h` is asked
  with −2 and keeps a file already loaded (`0x0B74E`), hands no palette on
  and steps past none (`0x0B7A8`), so the figure's cels count from
  resource 0 where the other two builds' count from resource 1; the
  strip's loader `073Ch:0007h` is called once, from the start (`0x00657`),
  and keeps no range of the carried list (`0x073C7`); giving or taking an
  item scrolls the strip by rows of four (`0x07E7B`–`0x07E8B`);
- darkens by the pixel: the options set bit 7 of every pixel of the
  playfield (`0DE4h:0123h`, `0x0DF88`) against a second palette half the
  palette loader fills with a converted copy of the first (`0104h:090Eh`,
  `0x0194E`–`0x019B0`), rather than through a flattening table; the
  panel's variants go to that upper half (`0x0744A`), and the options'
  inks are Dunkle Schatten's less `80h`, on the grounds `51h` and `50h`;
- holds the status band dark `48h` rows high from row `80h`, the strip
  below the line with it (`0751h:0067h`, `0x07987`, `0x078B0`), sets the
  line in `74h` with `7Ah`, `7Ch` and `74h` for its controls (`0x07843`,
  the table at `0x079A2`), and puts the strip's picture on row `8Ch`
  (`0x0761F`); under a choice menu the strip's redraw `0751h:04A2h` fills
  the line's `0Ch` rows with color 0 (`0x07A62`) and holds the band dark
  the same way (`0x07A7C`);
- composes the status line again once its age is past 2 (`0x0B105`), as
  Captain Zins's does, and like Captain Zins's leaves the age alone when
  mode 8 has asked its gate (`0x0A55F`);
- has Escape choose verb `0Eh` (`0x0AC6D`) and no arm of its own for the
  sixth verb; the options' gate, as Captain Zins's, opens no menu while
  the bits `[2D46h]` hold 8 — in the Lift it ends at once, anywhere else it
  plays a scene and waits (`0x09FDA`) — and it keeps the ticks the menu was
  up for (`0x0A019`);
- wants a second object by its rules, not by its verb: a search with
  phase 8 and, when it answers −1 or 2 for verb 4, a probe for a verb-5
  rule (`09B6h:0768h`, `0x0A2C8`–`0x0A30C`); its script's own interaction takes ids below `30h`
  for items and chooses with verb 3 (`0x0B238`, `0x0B2F8`);
- starts a task without testing its armed bit (`0x09A1B`), shows a chain
  head's parts live and not hidden (`0x0FC5D`), leaves the head when it
  hides a chain (`0x0FDB2`), and does not set task 0's speakers for a
  hotspot's own line (`0x098C3`);
- answers an object's state for kind 4 by bit 1 alone (`0x0FBFF`), keeps
  an object in the state row as 1 or its frame plus two, with no hidden
  state (`0x0FB04`–`0x0FB18`), and reads kind 1 back and kind 4 fresh
  (`0x0F855`);
- keeps no records of kind 8 apart (`0x0F5C1`); the animator's step has
  arms up to state `0Ah` (`0x040AE`) and hands a higher one to the row's
  record hook (`0x041E0`), which the Vorführraum alone names;
- does not fall back on node `7Fh` when the entry's node is not in the room
  (`0x09F64`); it remembers the door it was entered by for the doors `6Eh`
  to `77h` of kind 4 and for the single ids `78h` and `7Bh`, and shows
  `79h` without remembering it (`0x09EEE`);
- shares a room slot between rows of kinds 2 (slot `25h`) and 4 (slot
  `26h`) and keeps the sound between rows of kind 7 (`0x02EE2`); brings a
  room up by location (1 and `24h`) and then by the pair's kinds 2, 4 and 7
  (`0x09C39`, `0x09C4E`); leaving, spares the music and the transition
  between two places of kinds 2, 4 or 7 (`0x09D18`), fades over 2 000 ms
  as Captain Zins's does, and does not clear the status line (`0x09D48`);
- forgets the hover with `0:0` in its mode reset (`0x09C08`), as Captain
  Zins's takes `0:0` for "no hover"; keeps no
  callback, has no routine that walks the figure to a node — the logic
  writes the walk target and calls the walker itself (`0x16FE9`,
  `0x16FF1`) — and keeps a slot word's style without a horizontal field
  (`0x084F4`);
- takes codes 4 to 8 in its shared hook and hands every code on to the
  location's own hook (`0x0B3DA`, `0x0B557`); its animation signal has
  codes 1 to 9 (the table at `0x0B720`), whose samples depend on the
  object's id;
- names an inventory place by the item's id (`add ax, 0C000h`,
  `0x08657`), as Captain Zins's does;
- keeps songs for room slots below `28h` (`0x0C5C4`) and two-word effect
  masks, as Captain Zins's does, one for every room slot, from
  `2336h:0000h`;
- saves `SAVEGAME.SAV` as `0D15h` bytes under `BiFiSAV` and `SAVEGAME.CFG`
  as `1Dh` bytes under `BiFiCFG`, the settings' check over five words and
  no walk speed (`0x0EF1F`, `0x0F274`); a loaded game goes through row 91.

## The end

The program shows nothing after its main loop: `_main` writes the
settings (`0x007AD`) and exits. The game's end is a location's: the
Kaffee's row at `DS:182Ch` names `FFFFh` for entry 6, and the Kaffee's
handler then plays the sequence `0D84h:0008h` itself (`0x10942`) and
returns with the location word at `FFFFh`, which no other path leaves. So
the end comes before the settings are written
([the way out](../../engine/ending.md#bi-fi-2)).

## What is built

Every row has its handler's logic, with its hook, its animation hook, its
gate and its rule handlers; the 48 global rules; the shared hook; the
title program; and the end. A game is played from a new game to its end
by the host's own inputs alone — the verb buttons, the strip's items and
arrows, the hotspots, the dialogue menus — and the run reaches the end on
a location of −1, with nothing on the way that the runtime reports as not
built (`just playthrough`). The logic is held to the program's code and
that run, not to traces of the original routines
([verification](../../verification.md)).

## What is not built

No part of the game is known to be missing. What is not yet known about
the original is in [open questions](../../open-questions.md#bi-fi-2).
