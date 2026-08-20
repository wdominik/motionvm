[← Documentation index](README.md)

# Departures from the Original

The rest of this documentation describes the 1996 engine. This page is the
single ledger of the places where **motionvm knowingly does something else**,
and why — the counterpart to [Open questions](open-questions.md), which records
what is not yet known about the original.

Everything here is a deliberate choice. Faithfulness is the default and the
subsystem pages are written as if it were absolute; each entry below is a
signed exception to that.

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
([Execution model](vm/execution-model.md))

**`_PutStringAdr` is followed twice, differently.** The handler advances by
`(strlen + 3) >> 2` cells, one less than the compiler laid down whenever the
length divides by four. The runtime follows the engine; the disassembler, which
has to walk a body rather than run it, keeps the compiler's
`(len + 1 + 3) & ~3`. In the whole game exactly one string is affected,
`"GANRUFBA"`, at four sites, where both routes reach the same return with the
same stack. ([Word semantics](vm/word-semantics.md))

**Script words are looked up in module-number order.** The original walks the
module table at `0xEE6D0` in entry order. The rebuild walks it by module number
instead. All known duplicate names live in location modules that are never
loaded together, so no lookup in the shipped game can tell the two apart.
([Dialogue machine](engine/dialogue-machine.md))

## Display and timing

**The fade clamps its two divisions; the original does not.** `bands` and
`delay` are each taken to be at least 1. With the three screens this game has —
480, 400 and 80 rows, giving 30, 25 and 5 bands — and a duration of 50 at every
one of the 28 fade sites in modules 2, 4 and 5, the clamps never engage. They
exist so that a screen the original would have divided by zero on stops the fade
instead of the program. If a MOTION game ever turns up with a view under 16
rows, this is the line that makes it differ.
([Transitions](engine/transitions.md#timing))

**The timing follows the engine's arithmetic, not an emulator's.** `GIVETIMER`'s
unit-3 conversion is exact only at a 1020 Hz master, where unit 3 comes out at
the 200 Hz that `DELAY` assumes; that is the unit motionvm renders into
wall-clock time. Under DOSBox-X the same fades run about 2.4× slower, because
its ticks arrive in bursts aligned to the emulated 70 Hz refresh and a 1-tick
band waits a whole burst. That is the emulator's interrupt batching, not the
game's design, and it is not reproduced.
([Game loop](engine/game-loop.md), [Transitions](engine/transitions.md#timing))

**Nested callbacks do not pause.** A descriptor callback runs re-entrantly, and
the game menu depends on it: `DO_INVSEL`'s documents case starts three fades in
one call. The original simply blocks three times in a row inside the word; the
rebuild runs the nested call to completion without suspending the interpreter.
The observable order is the same, because what makes it come out right is
keeping the finished picture on screen rather than recomposing it from the
buffers. ([Transitions](engine/transitions.md))

**Text centers on the gap-inclusive height.** The disassembly says the centering
height is `font height × lines`, with no line-gap term. A running original says
otherwise: a caption the gapless arithmetic puts at y 167 stands at 166.
motionvm centers on `(font height + gap) × lines − gap`, and its text lands
where the original's does. This is the one place in the project where the code
follows a measurement against the original rather than the instructions; what
the instruction reading is missing is an
[open question](open-questions.md). ([Text rendering](engine/text-rendering.md))

**`SDBLK` is read but not built.** The word sets bit 7 of descriptor byte
+0x17, which centers a text block as a whole on its widest line instead of
centering each line. Nothing in the shipped game calls it — every text this game
draws centers line by line — so motionvm treats it as inert and counts it. The
entry sits on the inert list with that reason beside it, so the day a path does
call it, the run says so instead of drawing the wrong thing quietly.
([Text rendering](engine/text-rendering.md), [Descriptors](engine/descriptors.md))

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
game's own menu rather than by a second path beside it.

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
([Savegames](engine/savegames.md))

## Audio

**The OPL3 itself comes from outside.** The chain is
[sequencer](formats/hmi.md) → [FM driver](engine/fm-driver.md) → OPL3 →
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
([Audio](engine/audio.md))

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
([The FM driver](engine/fm-driver.md))

## See also

- [Open questions](open-questions.md) — what is not yet known about the original
- [Documentation index](README.md)
