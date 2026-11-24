[← Documentation index](README.md)

# Departures from the Original

*PARSEC, and the games motionvm plays on it, Dunkle Schatten and Captain Zins — the one ledger of where motionvm knowingly does something else, and why. What is not yet known about the original is a different record and lives in [open questions](open-questions.md).*

The rest of this documentation describes the original programs. This page
is the single ledger of the places where **motionvm knowingly does
something else**, and why — the counterpart to
[open questions](open-questions.md), which records what is not yet known
about the original.

Everything here is a deliberate choice. Faithfulness is the default and
the subsystem pages are written as if it were absolute; each entry below is
a signed exception to that, with the evidence it rests on and what a player
can tell from it.

## Starting and jumping

**`--loc` without `--entry` shows a room, not a visit.** The original only
ever enters a location through an entry id, and only the entry puts the
player anywhere ([game loop](engine/game-loop.md)). A jump the host asks
for by number alone loads the row's room as the visit's setup does — the
picture, its objects, the object map of a forest row, its animations and
its palette — and runs no handler: the player's record stays out of every
chain and is not drawn. With `--entry E` the row's handler is entered on
entry id `E` the way a door enters it; `7Fh` is the entry a fresh game starts on
(`0x4F37`, `0x4F3D`). A row of kind 0 is no place to show, so a jump to it
takes the program's own path onward with entry `7Fh` (`0x053B2`–`0x05402`):
`--loc 20` lands in the Schuppen, row 21, and `--loc 0` on the Karte, row
1. A number past the table is ignored, where the main loop would hand such
a word to the same onward walk (`0x04F8E`–`0x04FAB`); no exit of the game
writes one.

**A jump while the title program runs starts `XP01` as "continue" would.**
The batch file runs `XP01` only after `XP00` has left errorlevel 0, and
nothing passes between the two programs but that code and `SAVEGAME.SAV`
([boot](engine/boot.md)). A jump asked for during the title program takes
it down, stops its song as a chosen line stops it (XP00 `0x3B07`), installs
the driver afresh as `XP01` does (`0x5B7B`), and runs `XP01`'s start with
the settings and the saved game read, so the jump lands where it would
land from `XP01`'s own start.

**A jump takes the options down without their close.** `XP01` opens the
game options before its first location, unconditionally (`0x4F87`), and
the original has no road out of them but their own close. A jump that
arrives while they are up drops them: the verb strip's colors, the DAC and
the pointer's mode go back to what the options found, and the settings
stay as they stand. An entry whose effect is still playing is finished
first, as if the effect had played.

## Timing

**A frame is one interrupt and one frame pass, and an interrupt is one
tick.** The original's interrupt comes every `952h` PIT cycles, 500 a
second, or on a client's shorter period, and its tick group counts every
`26D6h` cycles, on the first interrupt at or after each boundary; its
visit loop runs as often as the machine lets it, and a frame pass with no
tick behind it finds the tick account `[1C76h]` empty and hands round a
delta of 0 ([game loop](engine/game-loop.md)). Here an interrupt is
`26D6h` cycles, every turn of the loop is one interrupt and one pass, and
the pass takes the one tick that interrupt counted. The rates of all the
counters are the original's; where a count lands within its period is not.
What depends on the delta — the tasks, the walkers, the animator, the
timed wait `[28BFh]` — sees the same ticks in the same order; what differs
is how many passes a tick is divided into, which on the original is the
machine's speed and not the program's. The animator's extra passes with a
delta of 0 are those the original runs and this engine does not.

**One keystroke a step.** The BIOS keeps fifteen keystrokes in its ring
and drops a sixteenth; the program takes them out one call at a time. Here
a press goes into such a ring and the game is handed one keystroke a step,
before the step runs. How many the original's frame pass takes when more
are waiting is not read ([open questions](open-questions.md)); keys typed
faster than the steps run land in later steps rather than being lost.

**The timer's countdowns start a full period away.** The countdowns the
interrupt handler keeps per group (`1950:E6/E8/EA`) are zero in the
shipped program, and where they stand when a location is entered depends
on everything that ran before. Here they start a full period away when
the game is opened. The arithmetic of each countdown is the original's, down to
its unsigned borrow when one lands on zero (`0x19F40`).

**The dissolve takes a fixed count of draws.** The options' dissolve draws
from the game's generator until the hundredths word `[1C82h]` reaches 100
(`0x1208C`–`0x120D3`), so how many draws it makes is how many passes the
machine fits into a second, and the program does not say. The count is
measured on the original: on DOSBox-X with `core = normal` and `cycles =
fixed 25000`, the generator's seed (`1EB5h:0006h`) stands at `0000:0000`
before the dissolve of the options `XP01` opens at its start and at
`7698h:E605h` after it in Dunkle Schatten, which is 158 213 steps, and at
`815Ch:96FFh` in Captain Zins (its seed at `1D77h:0004h`), 157 491 steps:
its loop costs more per pass.
Every dissolve here takes its title's count, the one at the start and the
ones in game. The pattern is the
reference machine's; what the generator rolls after it — every random
choice the game makes later — follows from it as it does there.

**The music's driver tick is on its own period.** The driver asks for its
tick every `period` PIT cycles, following the song's tempo events, and the
rebuild ticks it exactly there. In the original the driver is one client
of the timer module's interrupt, which runs on the shortest of its base
period, `952h` cycles, and its clients' periods, and counts the period
that just elapsed off the driver's countdown (`0x1A115`): a driver tick
lands on the first interrupt at or after it — on the base period, less
than 2 386 cycles (2 ms) later than here. The register writes and their
order are the same.

## Sound

**An effect is heard from the audio buffer after the step that asked for
it.** In the original the call reaches `DMABLAST.DRV` at once, and the
driver's double buffer makes it heard from the first byte of the block
after the next — or at once, into a standing mixer
([music](engine/music.md#when-an-effect-is-heard)). Here the game asks on
its own thread and the audio thread takes the request when it starts its
next buffer; from there the mixer's blocks decide as they do there. So an
effect, a `StopAll` and a song's start land up to one platform audio
buffer later than the step that made them, and on the block grid from
there. Nothing in the game waits on the sound, so nothing else moves.

**The digital side and the OPL are added at one fixed level.** The card
mixed its DAC and its OPL in analog, and no file records the balance; the
rebuild adds each DMA byte as `(byte − 80h) << 8` onto the frames the OPL
filled ([PSM 2 drivers](../psm/drivers.md)).

**An object without a record answers 1 to the seventh and eighth verbs'
rules.** `1586h:0364h` and `1586h:0438h` read the kind of the object's
record unchecked (`0x15BDB`, `0x15CAF`); for an object the lookup finds no
record of, that is a read through a null pointer, and what follows depends
on the machine's interrupt table. Here both rules answer 1 for such an
object and play no sample.

**A music library that will not read plays no sound.** The original ends
at its init when `PSMP2L.DAT` cannot be opened or does not carry its
signature (`0x13F00`, `0x13F3D`). Here the game opens without music and
without effects — `[20Ch]` and `[210h]` clear, no slot filled — and says
why, because everything but the sound is there to play.

**A song number the library does not hold is no song.** The loader
`1197h:000Dh(n)` answers null without a card (`0x1199F`); for a number past
the library's bound the original ends the program with *"P2L: Ungültiger
Index!"* (`0x14376`). Here it answers null too, which the logic tests and
passes on as it does without a card. No shipped call names such a number.

## Files

**A saved game that does not hold is a notice, not the end.** The start's
load reads `SAVEGAME.SAV` when it is exactly `8B1h` bytes and checks its
sum, xor and magic; one that does not hold ends the original with a
message (`0x149C7`). Here the message stays up and the game goes on as a
fresh game, as it does without a file.

**A file that cannot be written is a notice, not the end.** The original's
file routines (`1E69h:003Ch`, `00C0h`) end the program with their message
when a file cannot be created or written. Here the message stays up and
the game goes on; the file is the one that could not be written.

**The player's record keeps three far pointers it cannot have.** A saved
game holds the player's record whole, `30h` bytes (`0x14C73`), and three
of its words are far pointers into the original's heap: the `AN` entry
the figure was last bound to (`+18h`), the next record of its chain
(`+20h`) and the room's walk graph (`+24h`). This engine has no heap and
no addresses for them, so it carries what the program image or the last
loaded save put there and writes that back. Every other byte of the
record is the walker's: a game saved here and one the original saved at
the same place hold the same record but for those three pointers. The
items' cel pointers at `DS:1621h` (`0x0C29F`), which the restore does not
take back, are the same case: the original fills them from its heap.

## Rooms and objects

**The forest's animations start afresh in each row.** Between two forest
rows the original keeps the slot loaded, and with bit 4 of the object map
set the slot unload keeps the `AN` entries as they stand (`0x8592`) — their
working state with them — so an animation in one maze row runs on in the
next ([objects](engine/objects.md)). Here the animation runs are built
anew from the block on every arrival, the map's renaming applied
(`0x82E5`): an animation that was mid-stream when the player stepped on
starts again from its beginning. The picture, the objects' positions and
frames and the song are kept as the original keeps them.

**The live byte has no counterpart.** An object map's byte at `+6` goes
into the bound object's `+1` (`0x7EB9`), and only `0883h:000Fh` reads it,
to leave a standing single-cel visible record's rectangle off the list of
dirty ones (`0x884D`). This engine redraws the whole frame and keeps no
such list, so the byte is carried and read by nothing. The picture is the
same; the original's saving is redrawing less.

**A zero divisor in a leg's step steps by one.** A leg's step is the low
word of a quotient whose divisor is the edge's length times the walker's
speed, and a zero step is made 1 (`0x0AF22`, `0x0AF26`–`0x0AF2D`). A zero
divisor traps the original — its divide takes the short road
(`0x00219`) to a `div bx` (`0x00280`); here it reaches the same guard and
steps by 1. In play it cannot happen: the shortest edge in the game's
rooms is 4 and the fastest walk speed the options offer is 2, so the
divisor is at least 8.

## Text

**A substitution has no bound of its own.** The copy that resolves `02 n`
out of the substitution row (`13A7h:0023h`) stops where the room its caller
hands it runs out (`0x13AC3`, `0x13AEA`): `[1E4Ah]`, half of the scratch
`[1691h]`, a thousand bytes. The copy here takes the whole text. No text
the game ships comes near the bound: the longest is 510 bytes.

**The status line is always centered and never cut.** The setter
`0C3Fh:0B3Eh` takes characters off a line wider than `140h` until it fits
(`0x0CF79`–`0x0CFA7`) and places it by its second argument: 0 at the left
edge, 2 against the right, anything else centered (`0x0CFAD`–`0x0CFCB`).
Here every line is centered and kept whole. Four calls reach the setter
(`0x0ED60`, `0x0F0F6`, `0x101AB`, `0x10462`). In play the composer passes
1 (`0x10140`, `0x101A5`), and the widest line it can compose — a template
with each name the widest an item or a room's hotspot carries,
*Krankenschwester* — is 220 pixels with the shipped font, so no line
reaches the bound. The leaving passes 0 with the empty line `DS:546h`
(`0x0ED5A`–`0x0ED60`), and an empty line never reaches the placement:
the setter stores x 0 for it whatever it was handed
(`0x0CF5B`–`0x0CF5F`). The other placements come only from the room list *Raum %2d : %s*, which runs while bit 0 of
`[1EDCh]` is set (`0x0FD65`), and from the location's name, which
replaces the empty line while bit 5 is set (`0x10149`); `0x10462` hands
on its own caller's placement and has no caller in the image. Both bits
are developer's switches, `#` arguments looked for among the program's
command-line arguments (`0x5586`, `0590h:0F5Ah`). The batch files pass on
whatever follows their own name and add none of these, and this engine
has neither the room list nor the name line: in a game started without
those switches a player cannot tell. One who types the switches gets the
room list or the name line from the original and nothing here.

**A caption with both across bits set stands at x 0.** The composer
`1269h:0082h` places a box across by bits 2 and 3 of its style: 0 centers it on
the anchor, 4 puts its left edge there, 8 its right edge
(`0x12824`–`0x12853`). With both bits set none of the three runs, and the
box keeps the x its record held when the caller handed it over; here it
keeps the x of the freshly measured box, 0, before the clamp. Only a style
with both bits set can tell the two apart.

## The pointer

**The pointer is the host's mouse, drawn by the original's rules.** The
pointer library's descriptors, regions, animation and timeout are the
original's ([the pointer](engine/cursors.md)). What is not:

- **The mouse driver's mickeys and their acceleration**
  (`0x184D9`–`0x185FA`): the position is the host's, clamped to the
  library's limits, and a change sets the same bits 0 and 1 of `[2F6h]`.
- **The buttons' click timing and pressed shapes**
  (`0x18733`–`0x187C2`): the one shaped region the game registers, the
  menu's, names the same shape for hover and press (`0101h`).
- **Regions whose shape word is 0** — the room's hotspots, the strip's
  widgets — are not told apart from one another or from no region; the
  hit test treats them differently only when a selector of `80h` or more
  is shown away from its region, which the timeout's parking (`0x18997`)
  alone brings about.
- **The save-under in video memory** (`0x17C50`, `0x17D5F`): the cel is
  drawn over the finished frame.
- **The timeout does not clear the buttons word.** On the timeout's own
  pointer `179Eh:00C0h` clears `[3D4h]` (`0x17AAE`), the word the input's
  event record is built from (`0x17B24`); the buttons come from the host
  here, and no event record is built.

## The way out of `XP01`

What the program shows after its main loop is played as the original
plays it ([the way out](engine/ending.md)), with two differences of the
machine's, the first of which Captain Zins's title program shares:

- **No load time.** The original reads each picture out of
  `RESOURCE\EXTROS.DAT` as it goes on — a few retraces of black between
  Dunkle Schatten's banner and its picture, and before Captain Zins's
  water. Here the slots are read when the sequence begins. Captain Zins's
  title program ([boot](engine/boot.md#captain-zinss-title-program)) is
  played the same way: the original spends its own load times on the
  reference machine, before its first picture and before each part, and
  the runtime does not, so against a capture under DOSBox-X its pictures
  come 0.4 s sooner from the start, and 0.1–0.2 s more by the credits.
- **Captain Zins's ride is drawn once a retrace.** The original draws it
  as often as the machine lets it, waiting for no retrace, and the
  screen can show parts of several of its pictures at once; where the
  portrait stands is the hundredths since the pass began either way.
  The clock the pass sets to 0 is taken as starting on a hundredth.

## See also

- [Open questions](open-questions.md) — what is unknown about the original
- [Verification](verification.md) — what is held against the original's own output
- [Game loop](engine/game-loop.md), [Music](engine/music.md), [Options and the saved game](engine/options.md) — the original behavior these entries depart from
