[← Documentation index](README.md)

# Open Questions

*PARSEC, and the games motionvm plays on it, Dunkle Schatten and Captain Zins — the one ledger of what is not yet known about the original. Deliberate divergences are a different record and live in [departures](departures.md).*

What is documented as unknown, unverified or not yet read about the
original programs, in one place. A topic page that has something open
says so and points here.

## Timing

**Two ticks between a location's exit fade and its stop.** Leaving a
location fades its song over 1 500 ms (`0x0ED39`), plays the transition,
and stops the song once the transition is over (`0x0ED55`), because the
effect is synchronous ([music](engine/music.md)). Out of the Zimmer, on
the engine's 120 Hz ticks, the fade comes 2 972 ticks after the room's
song starts and the stop 117 ticks after the fade. A capture of the
original under DOSBox-X has 119 ticks between the fade and the stop, the
stop where the engine has it and the fade two ticks sooner, counted from
the song's start. Where the two ticks come from is not found. The fade
does not reach its end before the stop, so the driver's end of a fade-out
does not bear on it. A capture with the Sound Blaster has the Zimmer's
effects 21 to 27 ms sooner in the original than here, and the Küche's song
and effects 28 to 35 ms later ([verification](verification.md)).

## The forest and the maze

**The maze's renaming is read, not observed.** How a maze row's object
map binds the forest picture's objects, hotspots, walk-graph nodes and
chainless `AN` entries is read from the loader (`0x7BFB`–`0x830C`) and the
reset of the entry defaults and of states 7, 8 and 9 that follows it
(`0x7F5B`) ([objects](engine/objects.md)). The pictures it produces agree
with the original's: row 30 pixel for pixel in all 38 frames of a capture
([verification](verification.md)). What the original's memory holds after
the binding — every record's id, position and defaults — has not been held
against a dump of it.

## Input

**How many keystrokes a frame pass takes.** The program takes keystrokes
out of the BIOS's ring one call at a time; how many of them its frame pass
takes in one pass when more are waiting is not read
([departures](departures.md)).

## Captain Zins

**The title program's open readings.** Captain Zins's `XP00` is built
from its listing ([boot](engine/boot.md)); two things in it are not
settled:

- Whether a click during its animation, which polls nothing
  (`0323h:01FFh`), is still latched when the intro's first check polls:
  the buttons are latched in `[268h]` and cleared by the poll (`0x9799`),
  and whether `0970h:0AAAh`/`08DDh` (`0x36CB`, `0x36D0`) clear them first
  is not read. The runtime keeps a press latched until a poll takes it,
  and the title programs' hide and thaw, which stop and restart the
  mouse's sampling in `XP01`, are not mapped onto the runtime in either
  title program: there it samples throughout.
- What becomes of a key sent to it: in a capture, a space injected under
  DOSBox-X during the credits does not end them, where the
  poll (`0970h:00F0h`, `0x9810`) reads a key as an event — unless the
  mouse's word it tests first (`0x9803`–`0x980E`) stands at a value that
  passes the keys by, which is not read. A click does end them. The
  runtime ends them on either.

## The way out of `XP01`

The sequences after the main loop ([the way out](engine/ending.md)) are
held against recordings that set the location word as the main loop
begins, out of the start's options, and end each part with a click. Not
held against the original:

- The music: what each sequence asks of the driver is read off the
  program, not compared with a recording of the original's.
- The way in through the game: Dunkle Schatten's −1 from the JZ's last
  state or the F1's event, Captain Zins's from Money's third scene, and
  with it the last picture the parade fades out of.
- A key: the sequences end on any event the poll reports, a key among
  them as the program reads it; only clicks were recorded, and a key
  injected the same way did not end the credits of Captain Zins's title
  program.

## See also

- [Departures](departures.md) — every place motionvm knowingly does something else, and why
- [Verification](verification.md) — what is held against the original's own output
