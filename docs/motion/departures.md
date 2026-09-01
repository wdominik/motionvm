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
access, and names the total at the end of a run. Zero is the only value that can
be justified: module 0's data area is allocated at run time (`0x5f39f`) and
filled by the kernel, so what it really holds is not knowable from outside.

The **instruction fetch** and the inline-operand read stay strict. Jumping into
a module that is not loaded is a different matter from reading through a stray
pointer — the game does the second, only a mistake produces the first.
([Execution model](motion32/vm/execution-model.md))

**`_PutStringAdr` is followed twice, differently.** The handler advances by
`(strlen + 3) >> 2` cells, one less than the compiler laid down whenever the
length divides by four. The runtime follows the engine; the disassembler, which
has to walk a body rather than run it, keeps the compiler's
`(len + 1 + 3) & ~3`. In the whole game exactly one string is affected,
`"GANRUFBA"`, at four sites, where both routes reach the same return with the
same stack. ([Word semantics](motion32/vm/word-semantics.md))

**Script words are looked up in module-number order.** The original walks the
module table at `0xEE6D0` in entry order. The rebuild walks it by module number
instead. All known duplicate names live in location modules that are never
loaded together, so no lookup in the shipped game can tell the two apart.
([Dialogue machine](motion32/engine/dialogue-machine.md))

## Display and timing

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
is written down: [motionvm's savegames](savegames.md).

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
opening music. That needs a recording of the original, so it is a result
reported rather than something a reader can re-run.

**Where the rebuild and the original part.** At write 8043 of 17,047. A dense
passage hands ten note-ons to nine voices inside one tick, and the original puts
one of them on a different voice than the rebuild does — motionvm plays
`FINGBASS` on voice 5, the original `DISTGT`. The three allocation steps the
driver documents are not enough to explain it, so something in the step-2
search, or in the order the sequencer delivers a tickful of events, is still not
right. Everything before that point — nine and a half seconds — is identical.
This one is a known defect rather than a choice; it is listed here because it is
a measured difference from the original.
([The FM driver](motion32/engine/fm-driver.md))

## The 16-bit machine

**`?KEY` translates through the 32-bit engine's tables.** The 16-bit
handler (`12c8:063c`) has not been read; what its games are fed is the
translation measured out of `ENGINE.EXE` — the `0x100` scan-code marker,
the modifier bits, the Alt table with its `Z`-that-is-`O` mistake. No
16-bit module is known to test a scan code, so what reaches those games is
in practice the plain character byte, which both engines agree on; the day
one of them turns out to dispatch on a cursor code, the 16-bit handler has
to be read first. The open questions carry it.

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

**Four kernel words of the 16-bit engine answer by reading, not by
measurement.** `SFT` with 0 resets a font stack that starts out reset and is
the only argument the game passes; `NEWANIM` resets an animation system that
starts out reset; `.` and `EMIT` take their argument and print nothing,
because there is no text console behind a 320×200 game; `KEY` answers the
key that is waiting, or 0, rather than blocking. Each is the reading the
call sites admit; none is the handler's.
([Descriptors](motion16/engine/descriptors.md))

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

## See also

- [Open questions](open-questions.md) — what is not yet known about the original
- [Documentation index](README.md)
