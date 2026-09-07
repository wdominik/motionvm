[← Documentation index](README.md)

# Departures from the Original

*Both generations of the engine, and every game motionvm plays — the one ledger of where motionvm knowingly does something else, and why. Each entry names the generation and, where it is one game's, the game. What is not yet known about the original is a different record and lives in [open questions](open-questions.md).*

The rest of this documentation describes the original engine, in both of its
generations. This page is the
single ledger of the places where **motionvm knowingly does something else**,
and why — the counterpart to [Open questions](open-questions.md), which records
what is not yet known about the original.

Everything here is a deliberate choice. Faithfulness is the default and the
subsystem pages are written as if it were absolute; each entry below is a
signed exception to that.

The sections down to *The FM driver* concern the **32-bit engine** as it
runs Dunkle Schatten 2; [The 16-bit machine](#the-16-bit-machine) holds the
entries about the 16-bit engine as it runs Die Enviro-Kids greifen ein,
Jeff Jet - Abenteuer InfoHighway, Hilfe für Amajambere and
Victor Loomes – Das Spiel. An entry that names one of them is that game's.

## The virtual machine

**A read through a stray pointer answers 0.** `@` validates nothing
(`0x626fc`), and the game leans on it: location 5's macro reads an item number
as if it were a flag, location 6 reads through a shadow record that does not
exist yet, and both land in module 0 — which is created at run time and has no
file. Refusing to answer stopped five of the game's sixteen locations. motionvm
therefore answers **0** for a module that is not loaded, counts every such
access, and names the total and the addresses when the run ends — the window
prints them on the way out, and any caller can ask for them at any time
through the contract's `diagnostics`. Zero is the only value that can be
justified: module 0's data area is allocated at run time (`0x5f39f`) and
filled by the kernel, so what it really holds is not knowable from outside. A
read from a module `=>ERASE` has given back gets the same answer and the same
count, where the original reads freed memory; no shipped script does it.

The **instruction fetch** and the inline-operand read stay strict. Jumping into
a module that is not loaded is a different matter from reading through a stray
pointer — the game does the second, only a mistake produces the first.
([Execution model](motion32/vm/execution-model.md))

**`RANDOM` draws from a linear congruential generator, not the original's.**
The original's is read, on both engines, and it is not a generator with a
seed but a hash of the clock: a counter stepped by 331 a call, combined with
the engine's raw tick count as `((t + s) / s) xor ((t − s) mod s)` and
reduced modulo the count on the stack
([word semantics](motion32/vm/word-semantics.md#random),
[16-bit kernel words](motion16/vm/kernel-words.md#random)). Its input is
wall-clock time — the interrupt count at the instant of each call — and the
rebuild reads no wall clock anywhere: a given input state renders a given
frame, every time, which is what its drawing is verified by. Feeding the
formula the frame clock instead would give a sequence the original never
produces either, so the rebuild keeps an LCG of its own — `x = 1664525x +
1013904223`, the answer taken modulo the count, and zero for a count that is
not positive, where the original would fault on zero and leave a negative
count's answer unreduced. What the game leans on holds on both: the answer
is in `0..n`, and no shipped call asks for anything else — 237 sites in
Dunkle Schatten 2, all in the location task managers 201–222, and 188 in Die
Enviro-Kids greifen ein, every one a choice among alternatives rather than a
value that has to come out a certain way.

The **seed** is the platform's, handed in before the game starts; the
original has none. A run that is never seeded stays on the constant the
machine is built with, which is what lets the test suite render a scene twice
and compare the two — see [verification](verification.md).
([Word semantics](motion32/vm/word-semantics.md))

**`_PutStringAdr` is followed twice, differently.** The handler advances by
`(strlen + 3) >> 2` cells, one less than the compiler laid down whenever the
length divides by four. The runtime follows the engine; the disassembler, which
has to walk a body rather than run it, keeps the compiler's
`(len + 1 + 3) & ~3`. In the whole game exactly one string is affected,
`"GANRUFBA"`, at four sites, where both routes reach the same return with the
same stack. ([Word semantics](motion32/vm/word-semantics.md))

**`GET` of a block no container holds copies nothing, and says nothing.**
Both 32-bit builds copy from linear address 0 — the interrupt vector table —
into the module, as their fetch has no miss path; motionvm leaves the memory
as it was and counts the miss. Checker 2000's first run makes it once, for
the highscore block 98 it has not written yet — and there the original does
say something first: its resource layer puts up two of the engine's own error
boxes, one after the other, each a grey *Fehler* panel with an *OK* button
at 40,40 over a black screen, and the game waits at each for the click.
The texts are messages 24 and 37 of the engine's message table (R78
`0xb5ffc`): *RSC-Item mit Namen 098.BLK nicht verfügbar!* and *Freies
Blk-Item auf 'frei' gesetzt: BlkNr 98*. After the second *OK* the
registration board comes up as it does here. motionvm shows neither box —
the engine's native error box is not rebuilt — and counts the miss instead.
Read on a DOSBox-X recording of the original.
([Game structure](games/checker/game-structure.md#saving))

## Display and timing

**An unwaiting curtain takes no time at all.** R78's `FADEIN` and `FADEOUT`
wait between bands for nothing but the presenter, and `FADEIN`'s mode 2
waits on neither build; a curtain of theirs is 31 passes of marking and
copying, about 40 ms under DOSBox-X. motionvm moves every band of such a
curtain in one step that costs the master clock nothing and shows no
partial picture, where the original shows one to three at 70 frames a
second. The presenter's own milliseconds are not modeled anywhere.
([Transitions](motion32/engine/transitions.md#timing))

**A box painted onto the surface is kept by when it was painted.** The
original's drawer paints onto a surface that persists, so `WHITEBOX`'s box
— and `FADEIN` mode 2's — goes over what was drawn before it and under what
is drawn after, and the copy `SDAUTOBUF` pastes back was taken with the
box in it. motionvm's drawer builds a rectangle it has to draw again out
of the descriptor list, and keeps each paint on its screen with the pass
it came after and each descriptor with the pass that last drew it, so the
rebuilt rectangle draws the older descriptors, then the paint, then the
newer — the order the surface would hold. The difference could show only
if a descriptor below the paint were redrawn without the ones above it,
which the original's map never does. ([Screens](motion32/engine/screens.md#the-drawn-buffer))

**The fade clamps its two divisions; the original does not.** `bands` and
`delay` are each taken to be at least 1. With the three screens Dunkle Schatten 2 has —
480, 400 and 80 rows, giving 30, 25 and 5 bands — and a duration of 50 at every
one of the 28 fade sites in modules 2, 4 and 5, the clamps never engage. They
exist so that a screen the original would have divided by zero on stops the fade
instead of the program. If a MOTION game ever turns up with a view under 16
rows, this is the line that makes it differ.
([Transitions](motion32/engine/transitions.md#timing))

**The draw pass paints whole and publishes in part.** The original clips each
blit to the region it has decided to repaint. motionvm makes the same
selection — the descriptors the damage map names, together with the places
`SDAUTOBUF` owes — takes the union of their rectangles, paints the **whole**
screen afresh, and then publishes only that region onto the surface; everything
outside it stays as it was. A clipped blit still has to know it was clipped —
the text passes place themselves from their own measurements — and a text
backing is a *darkening* of what is under it (`0x186d5`), which cannot be run
twice over the same pixels without showing. Publishing a region that was
painted exactly once gives every pixel one pass over it and no arithmetic to
repeat. The picture is the same; what differs is how many times a pixel is
touched to arrive at it.
([Screens](motion32/engine/screens.md#the-drawn-buffer))

**The timing follows the engine's arithmetic, not an emulator's.** `GIVETIMER`'s
unit-3 conversion is exact only at a 1020 Hz master, where unit 3 comes out at
the 200 Hz that `DELAY` assumes; that is the unit motionvm renders into
wall-clock time. Under DOSBox-X the same fades run about 2.4× slower, because
its ticks arrive in bursts aligned to the emulated 70 Hz refresh and a 1-tick
band waits a whole burst. That is the emulator's interrupt batching, not the
game's design, and it is not reproduced.
([Game loop](motion32/engine/game-loop.md), [Transitions](motion32/engine/transitions.md#timing))

**Presents are batched to something a display can show.** A step is not always
a frame: while a curtain runs, the game's clock advances one band at a time,
down to 5 ms on the title screen, and the original presented each of them —
that is what a fade *is* there. Presenting each separately here meant two
hundred pictures a second, which no display shows and which backed the event
loop up behind the presents, so steps are gathered until they are worth at
least one display frame and shown together. The wall-clock pace is unchanged:
the sum of the batched steps is what the loop then waits, so a fade takes
exactly as long as it did. What differs is how many intermediate pictures reach
the glass — the original offered every band to a CRT that could show it, and
this offers the same bands to a compositor that could not.
([Game loop](motion32/engine/game-loop.md))

**Nested callbacks do not pause.** A descriptor callback runs re-entrantly, and
the game menu depends on it: `DO_INVSEL`'s documents case starts three fades in
one call. The original simply blocks three times in a row inside the word; the
rebuild runs the nested call to completion without suspending the interpreter.
The observable order is the same, because what makes it come out right is
keeping the finished picture on screen rather than recomposing it from the
buffers. ([Transitions](motion32/engine/transitions.md))

**A `SETPAL` behind a queued fade queues with it.** The completion of the
departure above. In the original, `SETPAL` programs the DAC on the spot
(`05f1:01ff` → `116a:00ef`; the 32-bit handler alike) and the fades spin
inside their words, so a script's palette switch always lands on the view its
`FADEOUT` has already blacked. Here a fade started from a nested call only
queues — and Die Enviro-Kids greifen ein walks that path through every door:
the LEAVE verb's `CALCLEAVE` (module 606) runs `INCLLOC` inside the order
machine's callback, so the next room's `XSETPAL` would recolor the old room's
still-standing picture for the whole closing wipe. The palette therefore
attaches to the most recently queued fade and the display takes it when that
fade finishes; the script's own view of the palette — `RGB->COL`, a save's
record, the drawer's backing table — moves immediately, as the original's
tables do inside `SETPAL` itself.

**Text centers on the gap-inclusive height.** The disassembly says the centering
height is `font height × lines`, with no line-gap term. A running original says
otherwise: a caption the gapless arithmetic puts at y 167 stands at 166.
motionvm centers on `(font height + gap) × lines − gap`, and its text lands
where the original's does. This is the one place in the project where the code
follows a measurement against the original rather than the instructions; what
the instruction reading is missing is an
[open question](open-questions.md). ([Text rendering](motion32/engine/text-rendering.md))

**`SDAUTOBUF` rebuilds where the original remembers.** A descriptor carrying
the flag is the only thing in this engine that disappears cleanly, and the
original manages it with a copy: the drawer saves the surface under it before
every blit (`0x696ed` → `0x6b936` → `0x2825d`) and `0x6ab6e` pastes the copy
back when the descriptor is hidden or moves (`0x6ac33`). motionvm builds the
vacated place again out of the descriptor list instead. The two agree wherever
the picture under the descriptor came from descriptors, which is everywhere the
game sets the flag — `INITANI` sets it on every animation, and so do the
captions and the menu sprites — and a rebuild cannot go out of date the way a
copy can: a copy holds the surface as it was when it was taken, and every
repaint underneath it since is news the copy does not have. Where they part is
pixels no *active* descriptor owns. The mailbox has them on purpose: a terminal
row it has switched off keeps standing until a bar covers it, and when that bar
goes the original's copy would put the old row back where a rebuild leaves the
place empty.
([Descriptors](motion32/engine/descriptors.md#sdautobuf--the-only-thing-that-erases),
[Screens](motion32/engine/screens.md#the-drawn-buffer))

**`SDBLK` is read but not built.** The word sets bit 7 of descriptor byte
+0x17, which centers a text block as a whole on its widest line instead of
centering each line. Nothing in Dunkle Schatten 2 calls it — every text that game
draws centers line by line — so motionvm treats it as inert and counts it. The
entry sits on the inert list with that reason beside it, so the day a path does
call it, the run says so instead of drawing the wrong thing quietly.
([Text rendering](motion32/engine/text-rendering.md), [Descriptors](motion32/engine/descriptors.md))

**Only the 256-color modes are drawn.** `TOGFX` enters whichever of the
engine's seven modes `SETRES` selected (`0x13fc0`), three of them in 32K
colors; motionvm sizes its picture to the selected mode and refuses the
32K ones there, because it composes indexed pixels and a 32K mode would show
a wrong picture rather than the game's. It also refuses a `TOGFX` with no
mode selected — the original would enter VGA 320×200 with a display size of
nought — and a second `TOGFX` at another size, since a window is one size
for a game's lifetime. Dunkle Schatten 2 asks for `640x480x256` once, so none
of the three is reached.
([Screens](motion32/engine/screens.md#the-video-mode))

**A text's `#i` of an address prints the address.** The 32-bit layout hands
the `#`-formatter every set insert slot as a pointer into its own memory,
and a text that says `#i` where a script inserted an address prints where
DOS/4GW happened to put the module; the cell address stands in, and the
case is counted. No shipped text does it. `#s` of a number prints nothing
where the original reads its memory at that number, and `TEXT->PRINTER`
prints nothing at all: there is no printer.
([Text rendering](motion32/engine/text-rendering.md#the-layout-the-line-window-and-the-inserts))

## Savegames

**They are not interchangeable, in either direction.** `NEWSCREEN` and `NEWDESC`
return raw heap pointers in the original and small handles counted from one
here. Both are opaque numbers to the bytecode — it only ever passes them back to
`ACTSCR` and `ACTDESC` — but they are numbers the scripts *store*, in the module
memory a savegame is mostly made of. A module image from the original is full of
addresses that mean nothing here, and one of motionvm's is full of handles that
would mean nothing there. No amount of matching the file layout changes that, so
the layout of `.FRZ` and `.anm` is motionvm's own; only `.blk`, which is four
raw bytes, happens to coincide. What *is* reproduced is the mechanism: the same
three artifacts, the same ids, the same order, the same semantics, driven by the
game's own menu rather than by a second path beside it. The 16-bit game is the
same case with a different cause: its `=>PUTAS` (`ENVIRO.EXE` file `0x16fe9`)
writes one run of its arena, addresses and all, and the rebuild's arena is
laid out differently ([the 16-bit machine](#the-16-bit-machine)) — so its
`.FRZ` and `.anm` are motionvm's own as well, under magics of their own
(`ENVFRZ`, `ENVANM`), and its `.blk` is two raw bytes. What those layouts are
is written down: [motionvm's savegames](savegames.md) — including the three
things motionvm's files do that no original's did: they are written whole or
not at all, they carry a checksum, and they name the game they belong to.

**The magic is the generation's, and the directory is the game's.** All four
16-bit games write `ENVFRZ` and `ENVANM`, because that is a fact about the
engine's format and not about any one game; the alternative — a magic per game
— would be a file layout no original ever wrote. Every game also names its
slots alike, `701` through `705`, and asks at start-up whether one exists, so
two of them sharing a directory would find each other's saves and the 16-bit
ones would load them. What keeps them apart is therefore the path, and it is
the engine that puts the game's name on it rather than whoever embeds the
engine: `saves/ds2/`, `saves/enviro/` and so on under whatever directory it is
pointed at.
Victor Loomes numbers its own from one and puts only the block at `700 + n`,
which its `CTRL` does openly (`DUP 700 + … PUT`, the bare slot for the other
two). For the 16-bit games that is not merely tidiness: the magics are the
generation's and
not the game's, so a slot of one would be *opened* by the other rather than
refused, and what came back would be another game's module image
([boot and frame loop](motion16/engine/game-loop.md#saves)).

Handing one of motionvm's to the original is not merely useless, it is loud:
`GETANIM` reads the first two bytes of the `DS2ANM` magic as an item number and
stops with *„AM-Error: Item ID 21316 entspricht keinem CompItem"* — `21316` is
`0x5344`, the letters `DS`. Worth knowing, because the message looks like a data
fault and is not one.

**They go somewhere else.** The original writes beside its data files, with no
path component at all. motionvm treats the game directory as read-only, so a
save directory is explicit, and one that lies inside the game data is refused —
the check comes before the directory is created, and both paths are resolved
first so a `..` or a symlink cannot walk around it. The default is the
platform's own data directory, not the working directory, so nothing lands in a
source tree either.

**They are checked.** `=>GETAS` validates nothing whatsoever and works only
because the loaded set is identical on both sides. motionvm parses the file
whole, matches every record to a loaded module of the same size, and only then
writes anything back — a damaged savegame stops the load instead of
half-applying itself. Records are keyed by module number rather than by
position, which is strictly more robust and costs nothing.

**`rsc.inf` is not rewritten.** `PUT` and `=>PUT` both rewrite it in the
original — a memory image of the resource manager, pointers and all — so saving
the game mutates it. It is a DOS-era cache, and nothing in motionvm reads one.

### What the display snapshot keeps

The rule is: whatever `4:START` or `INCLLOC` creates again is left out.

*Kept* — the descriptors and which is current, the active screen, the screens'
geometry, the palette, the pointer's visibility, the two dialogue globals, and
the *recipe* for every sprite `GFXVFLIP` mirrored (the ids, not the pixels: the
mirrored copy sits under an id no resource file contains, so nothing else could
bring it back).

*Left out* — framebuffers and cached sprites; the frame timing and step
multiplier, which the load path re-establishes itself with `_GSMODE @
STEPMULTI`; fonts, text templates and screens, which only `3:STARTUP` and
`4:START` ever create (guarded by a test); the fade log, which is a record and
not a state; and the interpreter's own position, because a load moves the game's
state and not its instruction pointer.

Also left out, and the riskiest of these: whether a screen is **frozen**. When a
player saves, the location's screen is frozen — `FREEZESCR` runs at `0x02e40` as
the menu opens over it — so carrying the flag would be the faithful-looking
choice and would load a game that stands still. The load path's own
`UNFREEZESCR` and `1 _INVMODE !` re-establish it.

The palette is the opposite case, and the reason it is kept: `SETPAL` appears in
every location macro **except 312 and 330**, so a savegame made in location 12
or 30 would otherwise come back wearing the menu's colors.
([Savegames](motion32/engine/savegames.md))

## Audio

**The OPL3 itself comes from outside.** The chain is
[sequencer](motion32/formats/hmi.md) → [FM driver](motion32/engine/fm-driver.md) → OPL3 →
samples, and three of those four are read out of the original. The fourth is not
and cannot be: the OPL3 is hardware, and there is no code in the game that says
what a YMF262 does with a register — only code that writes to one. The chip is
therefore a third-party core, reached through one small interface — write a
register, take a sample pair — with nothing above it knowing what is behind it,
so a different core can be substituted at that seam. Which core, and the license
obligation it carries, are recorded in `NOTICE`.

**Every game plays its Ad Lib rendition, whatever its sound setup said.** The
five 16-bit games shipped a `SOUND.EXE` that let a player pick a digital
renderer instead — the `DMA*.DRV` drivers, which play the same tunes out of the
modules' `SM8` sample sections rather than synthesizing them, and a
digital-only configuration still has music. Their internals are unread, so
motionvm plays the FM rendition for every game and every setting. The one
word that reaches those drivers for a *sample* rather than a tune,
`PLAYSAMPLE`, is another matter: its path through the driver is read and
rebuilt ([the 16-bit machine](#the-16-bit-machine)). The 32-bit engine's
digital layer plays its samples — Checker 2000's speech and effects — as
its mixer does, read: unchanged at full volume, `2 × ⌊frame × volume /
65 536⌋` below it, at the DSP's 22 050 Hz. What the original never sums
digitally is the music: the OPL3 and the DSP meet in the card's analog
mixer, and here the voice is added to the OPL's frame with saturation
([audio](motion32/engine/audio.md#the-digital-mixer)). A player who
remembers the sampled rendition of a 16-bit tune will hear the
synthesized one. Both are
[open questions](open-questions.md) before they are choices; they are here
because the choice is what a player meets.
([Audio](motion32/engine/audio.md))

**The three parts run with a clock between them, in an audio callback.** After
the first few notes nothing on that path reaches the allocator; songs are parsed
on the game thread and sent across. With no output device, no supported format
or no driver, the game says so once and plays on in silence.
([Audio](motion32/engine/audio.md))

## The FM driver

**How the rebuild is checked.** Not against a reading of the driver's page, but
against `fmmidi3.com` itself: the tables quoted there are read out of the
shipped image at the addresses named, and the rebuilt driver is required to
produce the same values from the same inputs. That check runs on any copy of the
game.

Above it sits a stronger one that does not: the whole register stream has been
held write for write against a recording of the original playing the game's
opening music, and the two agree over all of it — 17,047 writes, 32.8 seconds,
the switch-on sequence, every note, every steal. That needs a recording of the
original, so it is a result reported rather than something a reader can re-run.

## The 16-bit machine

**A palette cycle stays inside the palette.** `SETCYCLE`'s tick (`0104:536d`
in `LL.EXE`) indexes its two 768-byte tables with whatever range the script
named and checks nothing; motionvm leaves an entry outside 0 through 255
alone. Victor Loomes, the one game that cycles, asks for 32 through 127.

**An earlier-framing game's `GFX.INF` is not read.** It says how big every
sprite is without unpacking it, which is what a player that unpacks on demand
needs; the container here unpacks everything as it opens, so the sizes are in
the sprites by the time anything asks. The file is read in the tests instead
and checked against them — over Victor Loomes' 721 sprites the two agree
entry for entry.

**A `DATA.-n-` volume is never asked for; they are all open.** The player
carries the prompts for a disk change — *"Bitte Diskette #d einlegen!"*,
*"Disketten-Fehler. Falsche Disk im Laufwerk?"*, *"Datenblock <#s> nicht
gefunden."* — because the format was made for floppies and both Jeff Jet and
Hilfe für Amajambere came on two. motionvm opens every volume the header declares when the game is opened,
and refuses to open the game at all when one of them is missing rather than
running on with the slots that volume holds reported empty. There is no
sequence of play that reaches the prompt, so nothing draws it; what the
original does on that path has not been read
([open questions](open-questions.md#motion-16-bit)).

**An index the font reference table has no glyph for draws nothing.** Jeff
Jet's table was written for a font of 120 glyphs and its fonts have 102, so
two CP437 bytes — `0x8C` and `0xA0` — map to glyphs 104 and 103 that do not
exist. Hilfe für Amajambere's table has the same two entries and its largest
font holds 103, so both point past every font there too. The renderer leaves
such a character out of the line. Neither byte occurs in either game's shipped
strings — 5709 in one, 2709 in the other — and what the original's drawer does
with one has not been measured
([resource inventory](games/jeffjet/inventory.md),
[and its own](games/hfa/inventory.md)).

**A descriptor keeps its measured size nowhere, and its type not at all.** The
original stores the last box it drew at `+8`/`+0xa` and works out what a
descriptor is — text, sprite or block — from `+0x10` and `+0x12` every time it
is asked. motionvm measures on demand instead, and derives the type the same
way the original does, from the same two fields in the same order. What it does not reproduce is the *reachable-but-unused* corner
of that pair: in the original `SDCEN`, `SDVCEN` or `SDBLK` on a picture
descriptor put a bit in `+0x12` and thereby turn it into a text whose string
number is 0, so it draws nothing at all. No shipped script does that, and
reproducing it could only ever turn a working picture blank, so here those
words leave a picture a picture ([descriptors](motion16/engine/descriptors.md)).

**A location whose block the game does not ship is refused, where the original
carried on.** Hilfe für Amajambere's location 7 has no item table — block 307
is absent, and its occupancy word agrees — while `INCLLOC` loads block
`300 + N` for every location it enters. The original's `GET` (`BMZ.EXE`
`12bb:0e91`) tests the resolved block for null, calls its own error reporter
with code `0xE` — *Fehler diverser Natur (FDN)* — and returns with the
destination unfilled, so the room comes up holding whatever the previous
location left in `_LDITEM`. motionvm refuses instead, naming the block: a
missing resource is not a no-op here, because something downstream reads the
memory that should have been filled, and a room furnished from another room's
table is a wrong picture rather than a stopped one. The room is reachable in
play, from location 11
([open questions](open-questions.md#motion-16-bit)).

**Modules are placed first-fit from address `0x100` upward; the original
stacks them.** `=>GET` (`ENVIRO.EXE` file `0x176ac`) appends a module at the
top of one arena and binds its ids to absolute cells there; `=>ERASE` (file
`0x1690a`) takes it out and moves everything above it down, fixing the word
table and the interpreter's pointer and nothing else
([execution model](motion16/vm/execution-model.md)). The rebuild keeps one
64 KiB space, hands out the lowest free range that fits, and frees it on
`=>ERASE` without moving anything. Every address a variable pushes is
therefore the rebuild's, not the original's — which changes nothing a module
can observe as long as modules go in the order they came, and that is what
the game does: the library stays resident, a location's modules are erased
before the next location's are loaded, and the transient modules 610–612,
650 and 651 are erased at once. A module erased from under a resident one
would shift that one's addresses in the original; the game never does it. A
savegame that kept raw addresses would differ; that is the
[savegame departure](#savegames) over again, on this machine.

**An id held by two resident modules belongs to the one loaded last, and
erasing that one leaves the id unbound.** The original's `=>GET` writes
every id it binds without looking, so the module loaded last owns an id
there too; its `=>ERASE` clears every id of the range it erases, so erasing
the one loaded *first* unbinds the id as well, where the rebuild keeps it
bound to the other. Die Enviro-Kids greifen ein never has two — its sixteen
modules that define id 549 are loaded one at a time and erased before the
next — so the rule is never exercised by the game.
([Script modules](motion16/formats/script-modules.md))

**The data stack holds 16-bit cells sign-extended in 32 bits.** The
original's stack is 16 bits wide. The rebuild pushes every result wrapped
to 16 bits and sign-extended, so that the engine's words see one stack type
on both machines; a module sees the same values either way.

**An `SDBUF` buffer is a save-under; the rebuild recomposes instead of
restoring.** Read from the frame loop (`ENVIRO.EXE` `016a:179e`,
`0362:0e5d`, `016a:0a2d`): each frame the kernel pastes back what a dirty
buffered descriptor had saved under itself, redraws the dirty descriptors in
draw-list order and saves under them again; an unbuffered descriptor is
simply drawn where it now is, and the place it left keeps its old picture.
The rebuild treats a descriptor with a buffer the way the 32-bit engine
treats one with `SDAUTOBUF`: the place it leaves is rebuilt from the scene
graph — and it rebuilds the place an *unbuffered* descriptor leaves as well.
The picture is the same wherever the game buffers what moves, which is what
it does (the intro's motifs, every person); a location that relied on the
smear an unbuffered descriptor leaves behind in the original would differ.
`KILLNBUF ( from to -- )` frees from `from` to `to`, -1 meaning the last
buffer, 79 (file `0xafab`); `SETBUF` allocates an empty image and copies
nothing (file `0xac33`). ([Off-screen buffers](motion16/engine/buffers.md))

**`->SCRX` and `->SCRY` scroll as a transition.** The handlers (file
`0xc149`, `0xc7ed`) take `( screen x step -- )`, move the screen's window
`step` pixels a tick and blit each step themselves, one per `DELAY` tick,
inside the word — the interpreter does not run while they slide. The
rebuild queues the slide and holds the interpreter the way a fade holds
it: one step a frame, presented each time, the word's successor running
once the window has arrived. Same pace, same pictures; the one difference
is that the frame clock is the engine's rather than the word's own loop.
([Transitions](motion16/engine/transitions.md))

**A press too short to span a frame is stretched to one frame.** The
original's `MOUSELK` reads the live button — held is held, and every
debounce is the scripts' own: the 32-bit shell maintains `_MPRESSED` from
the raw level (module 4, `0x04a80`), and the title's task handler reads
`_MLK` unfiltered on purpose (module 223, `0x01a38`), which is what makes a
held button skip the title a card per fade. motionvm delivers exactly that
level. The one difference: the original *polls* once a frame and catches a
real click because a click outlasts a frame, while an event-fed window can
see a press and its release both land between two steps — such a press is
kept visible for the one frame the original's poll would have given it,
and only that one. ([Interaction](motion32/engine/interaction.md))

**A script loop that polls for input turns once a frame.** The original's
`MOUSEX`, `MOUSELK`, `?KEY` and their kin read live hardware, so a script
may wait in a loop of its own — Die Enviro-Kids greifen ein's `RUN` does on
its start-up page, its location 5 does in the newspaper — and the loop
turns as the player moves. Here a frame's input is fixed for the frame.
So after 256 polls in one frame the word is taken to be waiting, and from
then on every poll yields the frame to the window: the loop turns once a
frame, with that frame's input, and the screen is presented in between,
which the original's loop does not do either way. A frame of `CTRL` polls
a handful of times and never comes near the budget. The rule is the
engine's and holds for every game; only the 16-bit ones have a loop that
exercises it. ([Boot and frame loop](motion16/engine/game-loop.md))

**Three kernel words of the 16-bit engine answer by reading, not by
measurement.** `NEWANIM` resets an animation system that starts out reset;
`.` and `EMIT` take their argument and print nothing, because there is no
text console behind a 320×200 game. Each is the reading the call sites
admit; none is the handler's.
([Descriptors](motion16/engine/descriptors.md))

**`KEY` answers the key that is waiting, or 0, where the original blocks.**
The handler (`12c8:061c`) loops on the keyboard translator until it answers
something other than 0 — `LL.EXE`'s (file `0xd6f8`) also gives up once
Ctrl-Break has been pressed. The one place a shipped script reaches it is
Victor Loomes' `PRINT` (`DUP . 32 EMIT KEY DROP`, module 605), the hook its
assertions call — an inventory past its thirty-nine slots, a busy count
below zero, the talk kernel finding its reply descriptor active — and this
engine reaches that last one when the key is used on the car. Holding the
game there, with `.` and `EMIT` printing nothing, would be a freeze with no
prompt; so the word answers at once, and whether the original stops there
too is in the [open questions](open-questions.md#motion-16-bit).
([Interaction](motion16/engine/interaction.md))

**The 16-bit scale fields hold pixels; this engine keeps per-mille.**
`SDH%SHR` (`05f1:23f2`) and `SDV%SHR` (`05f1:246f`) store `1000` and
`−1` as −1 (natural) and any other value as `v · natural / 1000` **in
pixels**, resolved against the sprite current at the call — `SDSPR`
(`05f1:129a`) refreshes only the natural pair `+8`/`+0xA`, and the
drawer (`016a:100d`) takes `+0xC`/`+0xE` where they are not −1. This
engine keeps the per-mille value and multiplies at paint time. The two
models agree wherever a script writes scale and sprite in the same
breath — the walk does, and the intro's card flip swaps between
equal-width mirror images — and they part on `0 SDH%SHR`: the original
draws zero width, this engine treats 0 as unset and draws full size.
To be remodelled when a scene shows the difference.
([Screens and the draw chain](motion16/engine/screens.md))

**The callback walk runs after the controller.** The 16-bit frame loop
(`016a:0516`–`06da`) runs descriptor callbacks first and the screen
controller second; this engine keeps the 32-bit order (`0x68c64`,
controller first). With the active gate modelled, nothing read so far
tells the two apart — a callback that fires sees one frame's controller
state either way. ([Screens and the draw chain](motion16/engine/screens.md))

**`LL.EXE`'s heading pass stops at the buffer's end.** The pass that build's
`CROUTE` closes with (`0104:516d`) walks the step buffer as runs of equal
heading and stops at the end marker — but a run itself tests the marker only
together with an index at or past the buffer's room, so a buffer the walk
filled to the last step without writing a marker sends the original reading
on into whatever memory follows it. motionvm ends a run at the room. A walk
that long is one Victor Loomes has not been seen to make; the pass itself,
and the stale headings a shorter walk leaves behind the marker, are
reproduced as read ([the walk](motion16/engine/interaction.md)).

**`PLAYSAMPLE` cuts the tune, then plays the sample at the DAC's full scale.**
The handler (`STERN.EXE` `15e5:0349`, the one build whose game calls it) is two
halves, and motionvm runs both — it is a machine with an Ad Lib card *and* a
Sound Blaster, the configuration `SOUND.EXE` writes when both are on. First,
with a tune playing, `ENDTUNE`'s stop routine without the fade that routine
starts first — the tick counter reset, a spin until it reads 100, the flag
cleared, the driver's Stop — so the tune sounds on at full volume for half a
second and is cut. Then the block, whole, to the digital driver's play entry,
where `STERN.EXE` has installed the driver with **one channel** (the sixth
install argument is the caller's `DI`, 1 at `1058:01bd`), so the driver takes
its direct path (`DMABLAST.DRV` `0x0611`): the length word is the DMA count,
the period word becomes the DSP's time constant through the table at `0x60`
— the period in whole microseconds, so the DAC runs on the DSP's rounding of
the header's clock, 8000 Hz where the PIT would say 8008 — and the bytes
reach the DAC as they are — unsigned 8-bit, once, at full scale, with silence
(`0x80`) written when the transfer ends. What the rebuild decides
that the files do not: the level of those eight bits against the OPL. The card
mixed the two in analog and no shipped byte says how loud; the rebuild gives
the DAC full scale over the OPL's own output, which is the level DOSBox-X
gives it — a recording of the original's opening scene under it peaks where
the rebuild does — and a player who remembers the card's mix may remember
another balance.
And the driver's *other* path — a software mixer of up to eight channels with
per-channel volume (`0x196b`), which `PLAYSAMPLE` never reaches with one
channel installed — is read and not rebuilt: nothing in shipped play would
sound through it. `ENVIRO.EXE` carries
the first half alone (`1696:0357`), and no game of that build calls the word.
([PSM 2 music](motion16/formats/psm-music.md),
[open questions](open-questions.md#motion-16-bit))

**`GIVEDATE` answers the UTC date.** The handler (`STERN.EXE` `0cd3:37d9`)
is DOS function 2Ah, the machine's local date; `std` has the seconds since
the epoch and no time zone, and a dependency for the zone would buy a day's
difference in the hours either side of midnight and nothing else. The one
caller is Falsches Spiel mit Eddie M.'s `STNR`, which turns the date into the
week's issue number of the magazine; a suite fixes the date so a scene
composes the same on any day ([kernel words](motion16/vm/kernel-words.md)).

## See also

- [Open questions](open-questions.md) — what is not yet known about the original
- [Documentation index](README.md)
