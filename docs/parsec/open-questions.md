[← Documentation index](README.md)

# Open Questions

*PARSEC, and Dunkle Schatten, the game motionvm plays on it — the one ledger of what is not yet known about the original. Deliberate divergences are a different record and live in [departures](departures.md).*

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

## See also

- [Departures](departures.md) — every place motionvm knowingly does something else, and why
- [Verification](verification.md) — what is held against the original's own output
