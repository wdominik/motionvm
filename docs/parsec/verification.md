[← Documentation index](README.md)

# Verification

*What motionvm has been held against for Dunkle Schatten, the game it plays on PARSEC. Deliberate divergences are a different record and live in [departures](departures.md).*

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
rule handlers, are held against the original routines themselves:
**1 298 cases over 108 entry points** — 476 runs of a handler, 645 of a
hook, 149 of a rule handler and 28 of an animation hook. They cover every
handler the location table names but the onward walk of rows 0 and 20:
the three later JZ rows through the JZ's own, the maze's thirty rows
through the one they share, and location 60's restore.

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

What this settles is the logic: which engine routine a location calls, in
which order, with what, and what it leaves in memory. It does not settle
what the engine routines themselves draw or play; that is what the next
checks are for.

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
driver ticks through the game's own `MUSADL.DRV`.

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
music library's sixteen songs and their module heads, the Ad Lib driver
inside the driver file, the font inside the pointer file. The location
table is read out of the program and its 61 rows listed; the program's
data segment is found in each of the three programs. And the readers are
handed the shipped files damaged — cut short and flipped — with the
assertion that a reader answers rather than crashes.

## What has not been compared

Most of the game's rooms have never been held against a recording of the
original: the maze's row 30, the Wald's row 23, an answer's box in the
Küche, the title program's first logo and the pointer with the choice menu
up are what pictures cover. Walking, the verb strip,
the options' pages and the dialogue menus are held to the logic traces and
to the program's code, not to a capture; so is every song but the title
program's and, up to its exit fade, the Zimmer's, which only the digests
hold still. The sound effects are not
heard at all: the engine plays the game as the sound setup's Ad Lib answer,
with no digital channel ([departures](departures.md)).

That is not a gap being hidden. It is the edge of what a reimplementation
can claim without the original running beside it, and it is why the lists
above are kept apart.

## See also

- [Departures](departures.md) — every place motionvm knowingly does something else, and why
- [Open questions](open-questions.md) — what is unknown about the original
- [motionvm-parsec-tools](tools.md) — the inspector, and the register stream a music comparison is made from
- [PSM 2](../psm/README.md) — the driver the music runs through
