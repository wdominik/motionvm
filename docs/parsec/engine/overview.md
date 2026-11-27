[← Documentation index](../README.md)

# Engine Overview

*PARSEC — Dunkle Schatten (1994): the programs and what they read, as a map of the engine pages. Addresses are linear offsets into the unpacked `XP01.EXE` (150 096 bytes) unless a sentence names `XP00.EXE`.*

PARSEC is a **C program over data-driven rooms**. It is not a bytecode
machine and not an interpreter sitting on `RESOURCE/*.DAT`. Pictures,
walk graphs, hotspots, animations, dialogue trees and music come out of the
shipped files; what a verb does, what a location does when it is entered,
which item a puzzle gives and when a scene plays is compiled code in the
game's own executable. A location is set up by shared library code and
driven by its own handler.

That library is PARSEC's one generation here, **`parsec1994`**, as MOTION's
are `motion16` and `motion32`. The engine pages and the format pages
describe it: the routines every location calls and the files they read.
What one location's handler does, its puzzles and the game's own data
belong to the game, and are under [Dunkle Schatten](../games/ds1/README.md);
the other titles' pages — [Captain Zins](../games/capzins/README.md) and
[Bi-Fi 2](../games/bifi2/README.md) — say where their programs' builds of
the library differ.

## The programs

| Program | What it is | Page |
|---|---|---|
| `XP00.EXE` | The title program: logos, menu, backstory; it leaves an exit code and decides whether `SAVEGAME.SAV` survives | [Boot and the title program](boot.md) |
| `XP01.EXE` | The game: the location table of sixty-one rows, their handlers, the library they call, the options and the saved game | [Game loop](game-loop.md) |

Both are packed the same way, as is the third program the install ships,
`XP02.EXE`: a `PRSC` stub in front of LZEXE 0.91
([executables](../formats/executables.md)). Unpacked, an image is also the
game's data: the location table, the item names, the verb templates, the
caption styles, the option texts and the music table are rows and strings
inside it.

## The subsystems

| Subsystem | What it decides | Page |
|---|---|---|
| Containers and codecs | how a byte becomes a picture, a map or a text | [Containers](../formats/containers.md), [LZ](../formats/lz.md) |
| Rooms | which picture a location shows, its layers, its walk graph and its hotspots | [Rooms](../formats/rooms.md), [Object maps](../formats/object-map.md), [Hotspots](../formats/hotspots.md) |
| The location table and the frame | which handler runs, how ticks are counted and handed round | [Game loop](game-loop.md) |
| Objects | what stands in a room, where, at what scale, in which layer | [Objects](objects.md) |
| Walking | where the player may stand and how a leg is walked | [Walking](walking.md) |
| Verbs | what a click reaches, through the interaction mode word `[288Fh]` | [Verbs](verbs.md) |
| Tasks | who is speaking, what a scene is about, how long a page stays | [Tasks](tasks.md) |
| Animator | what a door, a sleeper or a mother does frame by frame | [Animator](animator.md), [Animations](../formats/animations.md) |
| Text | what the bytes below `20h` mean, and how a caption is set | [Text](text.md), [Dialogue](../formats/dialogue.md) |
| The pointer | which cursor shows, how it animates, how input is polled | [Cursors](cursors.md) |
| Palettes | which colors the DAC holds, and the flatten lookup under captions | [Palettes](palettes.md), [Graphics](../formats/graphics.md) |
| Options and saving | the game options, `SAVEGAME.SAV`, `SAVEGAME.CFG`, location 60 | [Options and saving](options.md) |
| Music | which piece plays where, when it fades and stops | [Music](music.md), [Audio](../formats/audio.md) |

## How a location comes up

1. The main loop (`0x4FC0`) reads where the player is, `[1F4Ah]`,
   multiplies it by the record stride and calls that row's own handler.
   A row of kind 0 is not dispatched: the main loop walks on to the next
   row that is a place ([game loop](game-loop.md#rows-that-are-not-places)).
2. The handler runs the visit's setup, `0EB9h:0001h`, which loads the
   room slot — the row's own, or slot 20 for a forest row — with its
   picture, walk graph, animations, object map, dialogue bank and song.
3. It calls the shared entry function `0EB9h:0207h(loc, entry)`. The entry
   clears the block the verb heads (`0x0EDA8`), installs the location's
   own interaction rules from record `+2Bh` (`0x0EDD8`), fills the subject
   slots (`0x0EDF7`), installs the location's hooks (`0x0EE6C`,
   `0x0EE96`) and puts the player down at the entry it was reached
   through.
4. `0EB9h:00B8h` finishes the entry: one animator pass with no ticks
   (`0x0EC4E`), the room drawn (`0BB6h:027Ah`), the entry effect, the
   game's gray ramp (`0x0EC84`), the room's song (`0x0ECB7`), the tasks
   enabled.
5. The handler's own state machine runs, one frame at a time, until a
   frame sets a new location; `0EB9h:017Ah` then leaves the location —
   both tasks stopped, a last frame pass, the music faded, a transition
   rolled and played — and the main loop goes on.

Steps 1 to 4 are the same routines for every location; step 5 is each
location's own.

## How a click travels

The input dispatcher is one routine, `0EB9h:0FCEh`, and it splits on what
is under the pointer: an id above `100h` is a widget on the strip below
the playfield (`0x0FBF2`), whose low byte is the verb; anything at or
below it is something in the room (`0x0FCB5`). Both halves go through the
interaction mode word, and a pair of verb and object is looked up in the
location's own rule table before the global one. [Verbs](verbs.md) has
the machine; [tasks](tasks.md) what a rule's answer starts.

## See also

- [Game loop](game-loop.md) — the location table, the main loop and the frame
- [Boot and the title program](boot.md) — `XP00` and how `XP01` begins
- [Departures](../departures.md) — where motionvm knowingly does something else
- [Open questions](../open-questions.md) — what is not yet known about the original
