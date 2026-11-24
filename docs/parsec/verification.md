[← Documentation index](README.md)

# Verification

*What motionvm has been held against for the games it plays on PARSEC, Dunkle Schatten and Captain Zins. Deliberate divergences are a different record and live in [departures](departures.md).*

There are three kinds of check here, as there are for every family in this
project, and the distinctions matter enough to be written down rather than
implied.

A check against the original's **own output** — a screen capture, a
register stream, a trace of one of its routines, a file it wrote — says the
engine *behaves* as the original did. A check against the original's
**files** says only that the readers agree with the data. A check against
**motionvm's own earlier output** says neither, and says the thing the
other two cannot: that nothing has moved since.

## Held against the original's own output

### The location logic, routine by routine

The location handlers of `XP01.EXE`, with their hooks, animation hooks and
rule handlers, and the global rules, are held against
the original routines themselves: **1 612 cases over 136 entry points** —
487 runs of a handler, 884 of a hook, 213 of a rule handler and 28 of an
animation hook. They cover every handler the location table names: the
onward walk of rows 0 and 20, the three later JZ rows through the JZ's
own, the maze's thirty rows through the one they share, and location 60's
restore; the shared hook `0EB9h:1968h`, with 239 of the hook cases; and 25
of the global table's 29 handlers, the four that speak the chosen
hotspot's own text among them. The other four are not traced: the walk
verb's rule and the seventh verb's door, which read records outside what
the traced logic is handed, the eighth verb's rule, which reads the
object's record, and the options' gate, which is library code; the
runtime's own tests hold those. The counts are those of
`tests/ds1_logic_cases.txt`, whose length the trace test holds.

A case is one entry point and one set of inputs: the location, the
routine's address, the bytes and words of the data segment set before the
call, what each engine routine the logic calls answers, the words the
engine writes between pictures, and the picture after which the engine
asks for an exit. The original routine runs on those inputs and is traced
— every call it makes, with its target and its argument words, in order;
every byte of the data segment it changed; and how it ended — and the
trace is reduced to a digest. The engine's own logic for the same entry
runs on the same inputs against a stand-in that records the same things
the same way, and its digest has to be the original's. Each line of the
table says so: *anchored: the original routine, traced on the same
inputs*. The cases are inputs, not data of the game, and live in the
repository with the digests beside them.

Captain Zins's logic is held the same way, against its own program's
routines: **1 159 cases over 124 entry points** — 223 runs of a handler,
571 of a hook, 251 of a rule handler and 114 of an animation hook — over
all eighteen locations, the twenty-one handlers of the global table and the
shared hook. They are the original's traces of generated and hand-made
cases, reduced to one to three per path through the code. Left out are the
cases whose inputs hand the logic a null record it then writes through, and
the options' gate, which is library code. The tracer takes the argument
counts of the script's interaction (three words, `0x0B764`–`0x0B861`) and
of the callback (one, `0x0A015`) from their listings.

What this settles is the logic: which engine routine a location calls, in
which order, with what, and what it leaves in memory. It does not settle
what the engine routines themselves draw or play; that is what the next
checks are for.

### The keyboard pointer

The pointer library's keyboard step and conversion, `179Eh:0AF9h` with
the step `179Eh:14E6h` in its first slot and no mouse driver, run under an
8086 emulator over 264 polls of Dunkle Schatten's `XP01.EXE`: the eight
directions held alone and together, both speed keys, both button keys,
units from 0 to 15, three divisor settings, the switch off, and the
pointer driven into its limits. After every poll the position, what is
carried, both acceleration factors, the key buttons and the changed bits
agree (`tests/ds1_keyboard_cases.txt`). Captain Zins's addresses are held
to its own image's words and keys; its routines are the same code at
other addresses and were not run.

Against `XP01.EXE` under DOSBox-X, Down held for eight BIOS ticks in the
options at the start: the rebuild held for the same 439 ms comes to rest
on the same picture, every RGB pixel; held for 51 or 54 steps it does not
(`tests/ds1_keyboard.rs`, which holds the change's place without the
original's pixels). That Space and Return are the buttons rests on the
hook's handler and the program's list, and on the game's own `INFO.DOK`;
no DOS run has pressed them. The title menu's arrow, held the same way in
Dunkle Schatten's `XP00.EXE`, comes to rest on the original's picture
too (`tests/ds1_keyboard.rs`).

### Pictures

Against lossless recordings of the original under DOSBox-X:

- **The maze**, row 30, reached through a saved game on the map: the
  playfield agrees **pixel for pixel in all 38 frames** of the capture —
  the forest picture of slot 20 with row 30's object map bound to it, an
  object walking from (160, 113) to (223, 113).
- **The Wald**, row 23, the same way: 25 of the 38 frames agree pixel for
  pixel. The other 13 differ by 4 to 19 pixels inside one object's box
  alone — an animation the Wald's handler starts (`0x03F8B`) that walks its
  frames at random on the game's generator, whose state at the moment of
  the capture is not known.
- **An answer's box in the Küche**, the caption centered on (160, 64) with
  its lines flush left, where the capture has it, pixel for pixel.
- **The title program's first logo** stands at full brightness for 107
  retraces in the capture, as it does here.
- **The pointer** with the choice menu up, and the shape it takes once a
  choice is made, as a recording shows them.
- **Captain Zins's title program**
  ([boot](engine/boot.md#captain-zinss-title-program)), from its start: the
  animation and the intro agree pixel for pixel, and so does the menu after
  a click has ended the credits. The credits roll 33⅓ lines a second, as
  the capture's do. The pictures come sooner than the capture's by the
  original's load times, which the runtime does not spend
  ([departures](departures.md)).
- **Captain Zins's first scenes**, `XP01` from a new game on: the
  sampled frames agree but for six pixels, the pointer's. Nine more
  states of a fresh DOSBox-X capture, the options left with Escape and
  the first conversation waited for, each held against the nearest of
  10 700 frames of the rebuild: six agree whole, the options among them
  with their heading in Captain Zins's own ink, the others differ in a
  figure's or a bubble's phase; none in the status band, which is set and
  held dark from row 130 as the program's painter does. The telephone
  call's pages agree in their text, and every text page of the capture —
  41 of them over 109 s — stands as long as the original's, within 0.1 s.
- **The way out of `XP01`** ([the way out](engine/ending.md)), reached by
  setting the location word as the main loop begins, under the DOSBox-X
  debugger. Dunkle Schatten's quit agrees frame for frame: every level of
  its four fades, the black between its two pictures, both waits. Its end:
  the credits agree pixel for pixel at every position the capture shows
  whole (2 170 frames); every row of the other 459 is the same row at a
  position at most two before or three after — the window caught while
  it was being copied, in eight of them in the middle of a row. They roll
  33⅓ lines a second; the campaign's picture is the same in all 865
  frames it stands. Captain Zins's parade: 909 frames agree whole, and
  every pixel of the others is one a neighboring picture of the ride
  has, the screen showing several of them at once because the ride waits
  for no retrace; it goes 100 columns a second, and each fade takes its
  129 retraces.

### The music

**The title program's song, write for write.** Module 0 of `PSMP2L.DAT`,
as `XP00` plays it from its menu, rendered from the game's own files
through its own `MUSADL.DRV` and held against an OPL capture of the
original: all **4 296** register writes the capture holds agree, in order.
`motionvm-parsec-tools registers` writes the stream that side of the
comparison is made from ([tools](tools.md)).

**The Zimmer's song, up to its exit fade.** Module 1, as `XP01` plays it
in the Zimmer of a new game, rendered from the engine's own music calls
through the same driver and held against an OPL capture of the original
that runs on from the Zimmer into the Küche: the first **1 705** of the
writes past the capture's snapshot agree, in order. The streams part
where the room's exit fade begins, which the original starts sooner than
the engine ([open questions](open-questions.md)); past that point the
two are not compared.

### The sound effects and the digital voices

**The mixer, block for block.** `DMABLAST.DRV`'s own routines — the block,
the channel routine, the clip, play, `StopAll`, entry 21 and the callbacks
that step a song's digital voices — run in an 8086 sandbox outside this
repository on the game's own samples and modules, and every 128-byte block
and every channel's state is held against `motionvm-psm`'s mixer after
every step. The suite's digests of every effect played alone and of every
song's digital voices over one pass of their stream are those of the
driver's blocks on the same plays: those 37 lines of the table carry
`anchored: the driver's own mixer, run on the same plays`.

**The Zimmer's opening, heard.** A DOSBox-X capture of the original with
the Sound Blaster's DAC and its OPL on separate tracks, `XP01` from a new
game into the Zimmer and on into the Küche, held against the engine's own
calls rendered through the same drivers — the songs' digital voices alone
(the rebuild without effects less its OPL-only output) and the effects
alone (the full output less the one without effects, held against the DAC
track less the aligned voices) — aligned once on the OPL track:

- the Zimmer's digital voices agree window for window: half a second at a
  time over 25 seconds, the waveforms correlate at 0.91 to 0.995, within
  half a DMA block (±3.1 ms) of the original's;
- in the Küche seven of its nine effects correlate at 0.96 to 0.98 and
  its song's voices at 0.77 to 0.999 in nine of twelve windows (about 0.5
  in the other three), the original's 28 to 35 ms later than the
  engine's;
- the Zimmer's ten effects are matched less well — 0.41 to 0.79 for nine
  of them, the original's 21 to 27 ms earlier than the engine's, and the
  first turn's not found (0.29).

Where the effects match, they match at the rebuild's level within 10 %,
and the voices within 5 %. The two offsets lie on either side of the
Zimmer's exit, whose timing is open ([open questions](open-questions.md)).

Over the music the DAC track against the OPL track has an RMS ratio of
1.26 in the capture and 1.93 here: the rebuild's OPL stands quieter
against its DAC than DOSBox-X's. Neither balance is the card's
([departures](departures.md)), and the rebuild is not set by the capture.

### The saved game

**The original's files are read, and written back as it wrote them.** A
`SAVEGAME.SAV` and a `SAVEGAME.CFG` the original wrote are read with its
own checks — the save's sum, xor and magic, the settings' check value —
and taken at the start: the save made on the map brings the main loop back
to location 1, the figure on its node. Saved again with the file's own
tick, every block the restore brings back comes out as the original wrote
it, the player's record whole, and the settings come out the same 31
bytes. A game played to the same place here and saved holds the same
player's record as the original's save, but for the three far pointers
into the original's heap ([departures](departures.md)). On the original
running under DOSBox-X the compatibility runs both ways: a save written
here loads there, and a save the original wrote there loads here.

### Timing

**The dissolve's length is the original's, measured.** The options'
dissolve draws from the game's generator until a second is over; on the
reference machine the generator's seed before and after it gives
**158 213** draws, and every dissolve here takes that many
([departures](departures.md)).

Those comparisons need recordings and traces of the original, and a
recording of a game is no more redistributable than the game. None ships
here, so the checks that consume them are not part of the test suite —
except the location logic, whose digests are what the original's traces
came to, and which the suite holds the engine to on every run. The saved
game is between the two: the suite reads, restores and writes back the
original's files only when `MOTIONVM_SAVES` names a directory holding
them, and skips those tests without one; that the original loads a save
written here is a check on the original, not a test.

## Held against itself

For every song the game ships and every case of the location logic the
suite keeps a **digest** — an FNV-1a 64 of the register stream in the order
the chip would have seen it, or of what the logic did — and holds each run
against the table. The sixteen songs are each played endlessly for 20 000
driver ticks through the game's own `MUSADL.DRV`; the 21 effects and the
sixteen songs' digital voices are mixed through its `DMABLAST.DRV`, and
their DMA bytes digested.

A digest says nothing about whether a stream is *right*. What it says is
that nothing moved, which a comparison made once by hand cannot say
twice. Sixteen hex digits reconstruct no song and are no fixture derived
from the game's data, which is why they may live in the repository where
the recordings may not. They rest on the engine being deterministic: no
clock is read, the game's generator is seeded by the host or starts at
zero, and the same input steps the same frame.

## Held against the original's files

Everything else. Every container is opened and every resource re-parsed:
the 60 slots of `DSROOM.DAT` with their six optional resources, every
room's playfield and hotspot table composed, every map cell naming a tile
in its layer's table, every `AN` block parsed, every dialogue bank read and
every text measured against the room the program copies it into, the
music library's sixteen songs, their module heads and the 21 effects, the
Ad Lib driver and the digital driver inside the driver file, the font inside the pointer file. The location
table is read out of the program and its 61 rows listed; the program's
data segment is found in each of the three programs. And the readers are
handed the shipped files damaged — cut short and flipped — with the
assertion that a reader answers rather than crashes.

## What has not been compared

Most of the games' rooms have never been held against a recording of the
original: in Dunkle Schatten the maze's row 30, the Wald's row 23, an
answer's box in the Küche, the title program's first logo and the pointer
with the choice menu up, in Captain Zins its title program and the first
scenes of a new game, and in both the way out of `XP01` are what pictures
cover. Captain Zins's rooms past those scenes are held to the logic traces
alone. Walking, the verb strip,
the options' pages and the dialogue menus are held to the logic traces and
to the program's code, not to a capture; so is every song but the title
program's and, up to its exit fade, the Zimmer's, which only the digests
hold still. The sound effects are held in the Zimmer and the Küche alone,
and never in a room without music.

That is not a gap being hidden. It is the edge of what a reimplementation
can claim without the original running beside it, and it is why the lists
above are kept apart.

## See also

- [Departures](departures.md) — every place motionvm knowingly does something else, and why
- [Open questions](open-questions.md) — what is unknown about the original
- [motionvm-parsec-tools](tools.md) — the inspector, and the register stream a music comparison is made from
- [PSM 2](../psm/README.md) — the drivers the sound runs through
