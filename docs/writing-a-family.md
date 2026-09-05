[← Documentation index](README.md)

# Writing an engine family

*For whoever brings a second engine to this program: what the window asks of a game, in the order it asks, and what each answer has to hold — the contract read from the engine's side of it. The procedure that gets a family into the tree (crates, the roster line, the boundary test's list) is in [`CONTRIBUTING.md`](../CONTRIBUTING.md) under *Adding an engine family*, and the shape it follows is in [`ARCHITECTURE.md`](../ARCHITECTURE.md). The MOTION family under [`motion/`](motion/README.md) is the worked example every section below points at.*

## The shape of the deal

Everything about a game crosses through `motionvm-playable` and nothing
else does. A family is a set of crates behind one `Family` implementation;
the window holds a roster of families and never names one of them, and a
family never names the window. What crosses is platform-shaped on purpose:
a picture and the palette its indices mean, a key press as a keyboard
reports one, a button transition, samples for a device, a sentence for a
player to read. Never a format, a word, a code a script compares against —
that translation happens on the engine's side, behind the call.

The engine stays out of reach. The window can start a game, step it, feed
it and take its picture; it cannot move the game's state, and that is what
makes the picture it shows one the original could have shown. A family that
finds itself wanting a method the window could use to reach in has found a
thing that belongs behind `step`.

## The order the window calls in

Once, before the first picture:

1. **`Family::detect(dir)`** on every family in the roster, from file names
   alone — cheap, so all of them can be asked before anything opens, and
   answered with a `GameCard` so the window can say what it recognized
   before the slow open runs. MOTION answers from the container and the
   engine binary beside it.
2. **`Family::open(dir)`**, which reads what a game needs and answers a
   `Box<dyn Playable>`. For a directory holding none of the family's games
   the error lists what every one of them would need; the window prints it
   or shows it in the folder dialog, so the words are the player's.
3. **`seed`**, once, from the clock. A test never calls it and gets the
   same run every time — see *Determinism* below.
4. **`open_music(rate)`**, before `start`, because a game's first song can
   fire during its own startup. The rate is the device's; the answer is a
   source for the audio thread, `None` for a game with nothing to play, or
   an error that becomes the line "sound is off: …". Silence by design is
   not a defect and gets no line.
5. **`set_saves(dir)`** with the platform's data directory. Take a
   subdirectory of your own under it, named by the game, so two games
   pointed at the same directory keep their slots apart — and know that
   slugs are a workspace-wide namespace, shared with every other family.
6. **`request_location(n)`**, only behind the window's `--loc`; a game with
   no such numbering ignores it.
7. **`start`**, which runs the original's own startup and returns once the
   game is parked in its frame loop. Everything after this is `step`'s.

Then the loop, until `finished` answers true or the window closes:
`frame_duration`, `step`, `frame`, present, wait — with input arriving
between the calls, never during one. And once, on the way out,
`diagnostics`.

## What one `step` must do

One round of the original's native loop: what that program did once per
frame — poll its input, run the script's controller, advance its timers by
the ticks a frame is worth, draw. Not a millisecond, and not as much as it
can: a step is the unit the original's own `DELAY` or its equivalent paced,
and the window's whole pacing rests on a step meaning that. The MOTION
engine's `Game::step` says what its frame is — the controller word, the
descriptor walk, the drawer — and `!LTWAIT`-style waits in the scripts count
these steps, not time.

A step owns its time. The game's clock advances by what the step covers,
and by nothing else: no call reads the wall clock anywhere on the engine's
side, which is the rule the next section and *Determinism* both rest on.

A step reads its input out of the latches the window filled since the last
one — see *Input* — and takes what it takes: one keystroke off the buffer,
the buttons' level, the pointer where it stands.

A step that cannot go on stops the game. `Err` from `step` becomes "the
game stopped: …" and the window exits; a path the engine has not read or
built should say so by name rather than limp on, because a silently skipped
word is indistinguishable from a working one until something far downstream
goes wrong. The MOTION engine's `Unread` error carries the binary and the
address to go and read.

## What `frame_duration` answers, and why the window batches

Asked *before* each step: how long the frame that step is about to run
should stay on screen. Afterwards the clock already belongs to the next one.

During a transition a step may be smaller than a frame. The MOTION engine
runs a curtain a band at a time — one step per band, down to 5 ms on the
title screen — and answers that band's duration, not the frame's. Answering
the frame's would be wrong twice over: the wipe would run at the wrong pace,
and its thirty-one bands would arrive as four pictures with the edge jumping
sixty-four rows, which reads as a cut. The engine's `clock.rs` says which
number it answers when.

Presenting every band separately is the window's problem, not yours. No
display shows two hundred pictures a second, and each present here walks the
whole physical window, so the window gathers steps until their durations add
up to at least a hundred-and-twentieth of a second, presents once, and waits
the sum — the fade's wall-clock pace is untouched, it just arrives in
pictures a screen can show. Two guards stop that loop early, a step count
and a wall-clock budget, so a step that takes long — a location loading, a
savegame written — still lets the window draw and read events. Do not batch
on the engine's side; answer each step's own duration and let the window add
them up.

`None` means "no wait at all", and the window then paces the loop at
twenty-five pictures a second rather than free-running. Every game this tree
ships asks for a real duration from its first frame on, because its startup
sets the pace before a window ever asks; a family should do the same.

## Input: when the pointer, the buttons and the keys arrive

All of it on the window's event thread, between steps, never during one — a
game is driven from one thread at a time, which is why `Playable` promises
`Send` and not `Sync`.

**`pointer(x, y)`** arrives as the hand moves, in the game's own
coordinates, clamped to the picture. The window asks for a repaint at once,
so the pointer a game draws itself — composed in `frame`, over whatever the
drawer left — follows the hand without waiting for the next step. Write the
position through immediately; the MOTION engine puts it straight into the
mouse record its `MOUSEX` reads.

**`button(which, down)`** arrives on every transition, releases included,
so nothing about a button dies before the seam. What a press *means* — a
one-frame click, a held drag — is the engine's reading, made at the game's
pace. One thing that reading has to get right: a press and its release that
both arrive between two steps still count. The MOTION engine keeps the
level and, beside it, a one-frame stretch that a press arms, so the step
that follows sees the button down once — the way the original's loop, which
polled the mouse once a frame, saw a quick click.

**`key_down(press)`** carries a `KeyPress`: the physical key by position,
the character the layout in force types for it, the modifiers that were
down. Translate it into whatever the scripts read and queue it the way the
original's keyboard did — the MOTION engine's `keys.rs` turns a press into
the code `?KEY` answers and drops it into a buffer the size of the BIOS
type-ahead, drained one keystroke per step. Nothing is answered: a press the
game has no code for, or one on a full buffer, is dropped as silently as the
hardware dropped it. Pin the translation with tests that build a `KeyPress`
literally; a translator with no test is how cursor keys end up on the floor.

`key_up`, `wheel`, `text` and `modifiers` have default bodies that do
nothing. Take the ones the engine reads — a game that reads held keys needs
the releases, one with a text field needs the typed stream — and leave the
rest alone.

## `frame`: the picture and its palette

One call answers both, borrowed: the indexed picture and the palette its
indices mean. Compose the pointer last. Stay indexed to the end — the window
expands through the palette when it presents, and a comparison against the
original is made on indices, which is what F12's picture is. `display_size`
and `pixel_aspect` are constant for the game's lifetime; the window caches
them, so a family whose original switches modes mid-run renders into one
size and says so — the MOTION engine refuses a second size at `TOGFX`
rather than change it.

## Threads, and what `Send` promises

Two threads touch a family's code. The **event thread** calls everything on
`Playable`, opening included today; `Send` on the trait is the promise that
opening — the slowest thing a window does — may move to a worker the day a
window wants a splash screen, and move the game back. The **audio thread**
calls `AudioSource::fill` from the device's callback: the source is moved
there, hence `Send` on it too, and everything slow — file reading, parsing,
a song being started — belongs on the far side of whatever feeds it. The
MOTION family hands the engine a sink and the audio thread a source, pairs
the two in its front door, and sends a song across as bytes; the engine
never learns a codec and the audio crate never learns an engine.

The error type is boxed and `Send + Sync`, and its `Display` output is what
a player reads — so a family keeps a rich error of its own behind it, and
its tests pin the messages.

## `diagnostics`: what the run noticed

Not errors. Nothing here stopped anything: these are the things a
reimplementation knows about itself that nobody else can — a word reached
that does nothing, a read into a module that is not loaded, a song that did
not decode — and that would be lost the moment the run ends. A claim about
another program needs a way to say where it fell short of one.

Collect them; do not print them. The window asks once, on the way out, and
prints what it gets, one line each: `subject` is a literal a reader can
group by and the window uses as a prefix, `detail` is one sentence in the
family's own words, with the count and the place. Totals rather than
events, because a run touches one address ten thousand times. The answer
is the report as it stands, not a queue that empties — asking twice answers
twice.

## Determinism

The engine reads no wall clock, its random numbers come from a seed, no
hash map sits on a drawing path, and directories are walked sorted. A given
input state renders a given frame, every time. That is what lets anyone with
a copy of a game render a scene before and after a change and trust the
difference, and it is the main way this tree's drawing is verified. `seed`
is how the platform gives a player a different game each launch without the
engine reaching for a clock; a run that is never seeded stays on the fixed
seed, and that run is the test suite's baseline.

## Where a family's evidence goes

A family's pages live under `docs/<family>/` with an index of their own and
their own ledgers — departures, open questions, verification — and every
page's scope line says what it covers. The neutral layer's sources and pages
name no family and cite no family's evidence; the boundary test holds that.
What a family measured from its original — a format, a handler, a timing —
is written on the family's side of the line, once, where the code that rests
on it lives.

## See also

- [`ARCHITECTURE.md`](../ARCHITECTURE.md) — the two layers, the seams, and *Extending the tree*
- [`CONTRIBUTING.md`](../CONTRIBUTING.md) — *Adding an engine family*, step by step
- [MOTION](motion/README.md) — the worked example, both generations of it
- [Debugging and diagnostics](motion/debugging.md) — what the window does with a family's report
