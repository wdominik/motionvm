[← Documentation index](../README.md)

# Boot and the Title Program

*PARSEC — Dunkle Schatten (1994): the batch file, the title program `XP00` and how `XP01` begins. `XP00` addresses are linear offsets into the unpacked `XP00.EXE` (64 176 bytes, `DGROUP` `0E3Ah`) and say so; the rest are into the unpacked `XP01.EXE` (150 096 bytes).*

`DS1.BAT` and `SCHATTEN.BAT` run `xp00.exe`, and `xp01.exe` only when it
left errorlevel 0. Between the two programs there is nothing but that exit
code and `SAVEGAME.SAV`: a new game is one whose saved game the title
program deleted.

## The title program

`XP00`'s `_main` (`005Ch:0008h`, XP00 `0x5C8`) probes `SAVEGAME.SAV`
(XP00 `0x5EC`) and calls the title routine `0323h:08F5h` (XP00 `0x3B25`),
which runs, in order:

1. The blank, `INTROS.DAT` opened, the screen on.
2. The intro (XP00 `0x3700`): the logo of slots 4 and 5 (XP00 `0x3631`),
   faded in, held 150 hundredths and faded out; then the *freelance* logo
   of slot 12 drawn through eight logo masks (`03B5h:0003h`, XP00
   `0x3B53`) — 287 columns of 123 rows that slide in from the right,
   come up mask by mask, are held a second and fade out at half the pace.
   With `/FXOFF` on the command line (XP00 `0x5FD`) the plain logo of
   slots 6 and 7 stands in its place. Any event while the first logo
   fades in skips the intro (XP00 `0x323C`).
3. The menu (XP00 `0x3785`), over the picture of slots 8 and 9: three
   lines, each centered on column 160 — *Neues Spiel beginnen*, *Altes
   Spiel fortsetzen*, *Spiel sofort beenden*. The second is drawn in its
   own ink when there is no saved game. Escape is the third line
   (XP00 `0x3A80`).
4. For *Neues Spiel beginnen*, the backstory (XP00 `0x336E`), slots 10
   and 11: six pages of two lines, each waiting for an event (XP00
   `0x34FD`), then the picture faded out.
5. Back in `_main`: a new game deletes `SAVEGAME.SAV` when it was there
   (XP00 `0x625`–`0x649`); continuing leaves it for `XP01`; quitting
   leaves the exit code −1 (XP00 `0x64E`), and the batch file runs nothing
   more — the settings are not written either.

All pictures come out of `RESOURCE\INTROS.DAT`, picture and palette in
neighboring slots ([containers](../formats/containers.md),
[graphics](../formats/graphics.md)).

### The title program's clock

`XP00` has no frame pass. Its palette loops wait for the start of the
vertical retrace before every DAC write (`0A35h:00B8h`, XP00 `0xA408`:
port `3DAh` bit 3), so a fade advances one step a retrace; the menu's fade
takes 66 of them. Everything else counts `[13C8h]`, the hundredths word
of the record the timer counts into (`DS:13BAh`, registered at XP00
`0x913`–`0x917`). The timer module is `XP01`'s: the same 8 156 bytes stand
at XP00 `0x0B16C` and at `0x18FCC`. The retrace rate is the card's, not
the program's: VGA mode 13h scans 449 lines of 800 dots at 25.175 MHz,
70.086 frames a second.

### The title program's music

The title plays module 0 of `PSMP2L.DAT` from the menu on: loaded as the
menu is built (XP00 `0x37A1`), started endlessly once the menu has faded
in (XP00 `0x3854`), faded over 2 000 ms when a line is chosen (XP00
`0x3AA2`, which returns at once) and stopped once the menu has faded out
(XP00 `0x3B07`), some retraces after the fade began, the digital
driver's `StopAll` behind the stop (XP00 `0x3B0C`). The music library is
`XP01`'s, its switch at 1 in the image and no options to change it, so
each call reaches the drivers — the song's digital part goes to the
digital driver, installed with three channels (XP00 `0x9A2`), before the
play (XP00 `0x0E4F`); without a sound card each is a no-op (XP00
`0x0D80`, `0x0F85`), and the pictures and their timing do not depend on
them. The drivers go with the program, and `XP01` loads and installs its
own ([music](music.md)).

### Captain Zins's title program

`ZINS.BAT` runs `XP00` the same way, and its `_main` is Dunkle Schatten's
instruction for instruction, but for its answers: the title routine
`0323h:1169h` (`0x4399`) answers 3 for a new game, 4 for the saved one
and 5 for quitting (`0x614`–`0x623`). The library under it is the same —
256 functions byte for byte, 94 more but for their addresses — and so are
its clock and its fades, so both programs are played on one rebuilt
machine. What the routine runs is its own:

1. an animation, `INTROS.DAT` slot 0 (`0323h:0459h`, `0x3689`): a palette
   of 256 colors without a header and 39 frames of deltas — after a
   width and a height, an offset on the screen and runs of a gap and a
   count of bytes (`0221h:00A0h`, `0x22B0`) — ten hundredths each; it
   polls nothing, and nothing skips it;
2. Dunkle Schatten's intro, the wave in slot 13 (`push 0Dh`, `0x43FC`);
3. the credits (`0323h:0EDCh`, `0x410C`): 88 lines at `DS:0826h` laid out
   by their first character — `c`, `n` and `p` centered in three inks,
   the others gaps — and rolled a line each three hundredths through the
   window (9, 7)–(`0CAh`, `0A8h`) over slot 10, slot 12's frame over them,
   round and round until an event; module 0;
4. the menu (`0323h:088Ah`, `0x3ABA`), Dunkle Schatten's routine with
   five lines from `82h`, a fade in by 8, and between the fade and the
   lines a wait of up to 500 hundredths and a dim of the picture to `80h`;
   module 2. Lines 1 and 2 open a viewer (`0323h:0520h`, `0x3750`) on the
   guide's ten pages or the five about GriPS — slot 1's font, a character
   each 6 pixels with a shadow, three boxes to page on, back and out —
   with module 1, and come back to the menu.

There is no backstory. The clock of a rolling or animating step is set to
0 the moment the last wait ended, so each step is its hundredths to the
count, wherever the retrace falls: a capture of the original rolls the
credits 33⅓ lines a second.

### Bi-Fi 2's title program

`BIFI.BAT` runs `XP00` the same way. Its addresses here are offsets into
its own unpacked `XP00.EXE`, 62 592 bytes, `DGROUP` `0DD7h`. `_main`
(`005Ch:0008h`, `0x5C8`) reads the mouse setting (`0x6A4`), probes
`SAVEGAME.SAV` (`0x626`–`0x646`), opens the music library (`0x651`) and
calls the title routine `0311h:059Dh` (`0x36AD`), which runs, in order:

1. the pointer hidden, `INTROS.DAT` opened;
2. an animation, slot 8 (`0311h:0134h`, `0x3244`): its palette written at
   once and 36 frames of deltas, five hundredths each, which nothing
   skips;
3. module 0 of `PSMP2L.DAT`, started endlessly as the animation ends
   (`0x3709`, `0x371B`);
4. two stills through the logo routine (`0311h:02C1h`, `0x33D1`, Dunkle
   Schatten's `0x3631`), slots 0/1 and then 2/3, each by step 4 and held
   `96h` hundredths — no wave;
5. the title picture, slots 4/5, faded in by 2, which nothing skips;
6. the menu (`0311h:0342h`, `0x3452`): three lines from row `9Ah`, `0Fh`
   apart — a new game, the saved one, quitting — the second gray without
   a saved game, Escape the third; its inks `7Eh`, `74h` hovered and
   `70h` gray, in `CMBMOU.DAT`'s bank, which here is `60h`–`7Fh`. A line
   chosen fades the music over 2 000 ms (`0x3812`), the picture out by 8,
   and stops the song once it is out (`0x3860`, `0x3865`);
7. back in `_main`: the new game deletes `SAVEGAME.SAV` (`0x66F`), without
   looking at the probe; the first two lines leave exit code 0, the third
   −1 (`0x68A`).

The blank fills the screen with 0 (`0x2834`), where Dunkle Schatten's
fills it with `0FEh` (`0x28F9`). With `/JUKEBOX` on the command line and a
music driver, the third line reads *Ab geht's zur Jukebox!* (`0x61C`) and
runs a jukebox (`0311h:0BB5h`) before the exit. There is no backstory.

## How `XP01` begins

`_main` runs from `0x4F19` to the options (`0x4F87`):

1. The main loop's words set: the place left, `[1F4Ch]`, to −1
   (`0x4F19`); the entry word to 0.
2. The start's load, `147Ah:0072h`: the save buffer cleared (`0x14821`);
   `SAVEGAME.CFG` read and taken when it is exactly 31 bytes and its magic
   and check value hold (`0x14847`); `SAVEGAME.SAV` read when it is exactly
   `8B1h` bytes (`0x148ED`). A saved game whose sum, xor or magic does not
   hold ends the program with a message (`0x149C7`); one that holds is
   restored at once, through location 60's handler (`0x149D3`)
   ([options and saving](options.md)).
3. A fresh game when nothing was loaded: `0x4F2A` asks whether the word
   `[1F4Ah]` is still zero, and when it is, `0x4F37` and `0x4F3D` set
   location **2**, the Zimmer, and entry `7Fh`. Location 0 is not where a
   game opens: its row is not a place.
4. The game options, always (`0x4F87`): the menu dissolves in over a
   hundred hundredths (`0x120CE`), keys typed before that are flushed
   (`0x12106`), and Escape closes it.
5. The main loop (`0x4FC0`, [game loop](game-loop.md)), which runs while
   `[1F4Ah]` is not below zero (`0x4FE9`). Quitting from the options sets
   it to −2 (`0x12403`).

The pointer the options come up with stands at (160, 100), where the
pointer library's image puts it: nothing the mouse did while `XP00` ran
reaches `XP01` ([the pointer](cursors.md)). Captain Zins's and Bi-Fi 2's
`XP01` begin with the pointer there too.

## See also

- [Game loop](game-loop.md) — the location table and the main loop
- [Options and saving](options.md) — the start's load, and the files
- [Music](music.md) — the songs `XP01` plays
- [Graphics](../formats/graphics.md) — the pictures and palette banks the title program shows
