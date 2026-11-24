[← Documentation index](../README.md)

# The Way Out of `XP01`

*PARSEC — what the game program shows after its main loop, before it exits with code 0. Dunkle Schatten's addresses are linear offsets into the unpacked `XP01.EXE` (150 096 bytes, `DGROUP` `21EFh`); Captain Zins's are marked CZ and are into its own `XP01.EXE` (`DGROUP` `1FC4h`).*

The main loop runs while the location word is not below zero (`0x4FE9`,
CZ `0x007A6`). After it, the settings are written (`0x4FF0`, CZ `0x007AD`),
and the location word the loop ended on chooses what is shown, out of
`RESOURCE\EXTROS.DAT`:

| Location | Dunkle Schatten | Captain Zins |
|---|---|---|
| −1 | the game's end, `12D3h:0636h` (`0x5014`) | the parade, `0E01h:068Bh` (CZ `0x007CB`) |
| −4 | nothing (`0x4FFB`) | nothing |
| any other below zero | the quit, `12D3h:03A7h` (`0x501B`) | nothing (CZ `0x007B2` sends only −1 on) |

Location −1 is the end of the game: in Dunkle Schatten the JZ's last
state sets it (`0x1D7D`), and so does the F1's hook event 28h
(`0x463E`); in Captain Zins Money's hook on the text's script event 12h
(CZ `0x148C2`), at the end of its third scene. Quitting from the options
leaves −2 (`0x12403`, CZ `0x0D73B`); −4 is a developer's key in Dunkle
Schatten (`0x0FD00`). Afterwards the program exits with code 0
(`0590h:095Fh`).

The pictures are the title programs' library: the blank, the fades, the
pictures and rectangles, and in Captain Zins the columns of the
[intro's wave](boot.md#captain-zinss-title-program). The pointer is
hidden throughout: the thaw at each wait (`179Eh:091Ah`) draws it only
when it was shown, and the hide before (`179Eh:0882h`) took that away.
While it is hidden the mouse is not sampled at all — the timer's callback
skips it (`0x18488`) — so a click during a fade nobody polls is not seen.
Thawed, a press sets a button's flag, and the poll reports it and clears
every flag at once (`0x17B39`): two clicks before a poll are one event.
Flushing the keys leaves the buttons as they are (`0x17F13`).

## Dunkle Schatten

`EXTROS.DAT` has four slots: the campaign's picture (slot 0) and its
palette (1), the campaign's banner (2) and its palette (3), each palette
128 colors from DAC entry 0.

Every fade is `12D3h:00DCh`: the palette at a level, a level each
retrace, by 4 or by 8, the last level written once more at the end — the
title programs' fade. Between its steps a check polls; an event is
latched (`[1E84h]`) only through the mask (`[1E82h]`), and the latch
stays until it is cleared.

**The game's end** (`0x13366`):

1. The mask off, the pointer hidden, the blank — `XP01`'s fills the
   screen with 0 (`0x110A2`) — and the banner onto the screen; faded in
   by 4, which nothing skips.
2. The keys flushed, module 4 played endlessly (`0x134A6`), and the
   credits rolled over the banner through a window of 200 by 160 at
   (60, 20): the 55 lines of `DS:0AD6h`, laid out by their first
   character as the [roll](boot.md#captain-zinss-title-program) of
   Captain Zins's title program is, but from `0A0h` and with a `c`
   line on by `0Eh`, an `n` by `0Fh` and a `p` by `10h` (`0x12E73`,
   `0x12F0D`, `0x12EDC`, `0x12F08`). The roll is 1 443 positions long and
   begins again at its end. A step each three hundredths: the clock set
   to 0, the window restored from the banner, the lines in view over it
   in their inks (`0E6h`, `0F6h`, `0F4h`, in the bank no fade touches),
   the window onto the screen, then polled until three hundredths have
   passed (`0x13502`). Any event ends it at once (`0x13517`).
3. The banner without the roll, the music faded over 2 000 ms, the
   banner faded out by 4; the song stopped and the digital driver's
   effects with it (`0x1357F`, `0x13584`), which cuts the music's fade
   short.
4. The blank, the campaign's picture faded in by 8; the mask on, and the
   picture held until an event (`0x136CD`); faded out by 8.

**The quit** (`0x130D7`): the campaign's picture faded in by 8, held
until an event, faded out; then, with no blank between, the banner, the
mask on before its fade (`0x132D4`) — so an event cuts the fade short and
stays latched, and the wait after it is over at once — held until an
event and faded out. The music the location left plays on.

## Captain Zins

`EXTROS.DAT` has 24 slots: the water (slot 0, colors 161–223) and its
bank (1, 64 colors from 160), the pipes on the left and right edges
(2 and 3, cels of 47 and 46 by 200), a bank the portraits share (4, 32
colors from 128), eleven masks (5, a directory of 320-by-200 pictures,
not packed as a whole), and from slot 6 on the eighteen portraits, each a
directory of a palette (128 colors from 0) and its columns, `1 × h` cels.
How many portraits there are is how many slots the group has past six
(CZ `0x0E2E2`).

**The parade** (CZ `0x0E69B`):

1. The pointer hidden, the game's last picture faded out: every DAC entry
   from the shadow, from `100h` down by 2, a level a retrace, 129 of
   them, no last write and no event looked at (CZ `0x0E6BB`).
2. The blank; module 3 played endlessly at once (CZ `0x0E327`); the two
   banks; the water onto the screen; the DAC black, and the water faded
   in the same way from 0 up (CZ `0x0E740`).
3. Pass after pass, from the left first and then from the right in turn
   (CZ `0x0E7B6`): the next mask, round after eleven; the next portrait,
   round after eighteen. The portrait's first color is made black and
   its palette written at the retrace, without a fade (CZ `0x0E8DD`);
   the water and the line under it go onto the screen whole; the keys
   flushed, one poll's event dropped (CZ `0x0E9B5`), the clock set to 0.
4. The ride (CZ `0x0E9C0`): the portrait one column a hundredth across,
   from the left until it is past the right edge, from the right until it
   is past the left — 320 plus its width in hundredths. Each picture of
   it (`0E01h:050Bh`) polls into the latch, draws the columns into an
   empty surface, each stretched over its mask's span at its own screen
   column; copies that surface into the back buffer with 0 seen through;
   draws the pipes at both edges and the line; copies the part this
   picture and the one before covered onto the screen; and restores the
   water in the back buffer where the columns were. The rectangle before
   is kept from pass to pass. The ride waits for no retrace: it draws as
   often as the machine lets it.
5. The scaler is handed each column four bytes in (CZ `0x0E1D5`), past
   its width and height only: the columns are cels, so their flags and
   reserved words — four zeros, seen through — come out as the top of
   each column, and its last four pixels are not drawn.
6. The line: one of the five of `DS:0A52h`, centered at row `0B4h` in
   ink `0E8h`; the next every four passes, the last kept.
7. An event ends the pass and the parade: the music faded over 2 000 ms,
   the screen faded out as in 1., the song stopped (CZ `0x0E490`).

Captain Zins has no picture after the parade and nothing for quitting.

## See also

- [Boot and the title program](boot.md) — the library the sequences are drawn with
- [Game loop](game-loop.md) — the main loop and the location word
- [Departures](../departures.md) — where motionvm plays them differently
- [Verification](../verification.md) — what is held against the original's own output
- [Open questions](../open-questions.md#the-way-out-of-xp01) — what is not
