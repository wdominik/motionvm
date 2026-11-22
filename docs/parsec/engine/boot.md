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

## See also

- [Game loop](game-loop.md) — the location table and the main loop
- [Options and saving](options.md) — the start's load, and the files
- [Music](music.md) — the songs `XP01` plays
- [Graphics](../formats/graphics.md) — the pictures and palette banks the title program shows
