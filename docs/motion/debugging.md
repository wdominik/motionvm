[← Documentation index](README.md)

# Debugging and Diagnostics

*motionvm's own — the switches, keys, reports and rigs a person working on the engine reaches for, across both generations and every game. Nothing here describes the original engine; what it does is under [MOTION 32-bit](README.md#motion-32-bit) and [MOTION 16-bit](README.md#motion-16-bit).*

Everything below exists in the code and is described in its rustdoc; this
page is the map, so that an affordance can be found without reading the
crate that holds it.

## Switches and keys

| What | Where | Does |
|---|---|---|
| `--loc N` | the command line | Starts in location `N` through the game's own request cell, once startup is over — the intro is skipped, not faked |
| `--no-sound` | the command line | Opens no audio device; the music sink is never installed and the game plays silent |
| `MOTIONVM_PERF=1` | the environment | Once a second: presents per second, the engine's compose time, the window's blit-and-present time and its worst case, and the window's scale pair; also what the audio device settled on |
| `MOTIONVM_LOG=1` or `=<path>` | the environment | Appends every line the window prints to `motionvm.log` beside `saves/`, or to the path — the lines a windowed build otherwise loses |
| F12 | the window | Freezes the frame clock and writes the composed picture as an indexed PNG to `shot.png` beside `saves/`; the path and the frame number are printed. Press again to run on |
| Alt+Enter | the window | Borderless fullscreen, on and off |

A fatal stop — a word the engine does not implement, a savegame that will
not load, a window that would not open — goes to stderr, to the log if one
was asked for, and to a message box on every platform, so a player who
started the program from a launcher sees it.

## What a run reports about itself

On the way out the window prints the game's *diagnostics*: what the run
noticed and could not act on. Three things travel that channel today —

- **words reached that do nothing** — the inert words the scripts called,
  with counts. A word is inert only when the engine's word table says so and
  says why; anything else that is missing stops the game instead.
- **stray reads** — on the 32-bit machine, every read into a module that is
  not loaded, with the total and the first addresses. The
  [departures ledger](departures.md#the-virtual-machine) says why such a read
  answers 0 rather than stopping.
- **music** — a tune the sink could not decode, by number.

The channel is the contract's `diagnostics`, pulled rather than pushed: the
family collects, the window decides when to ask. A test can ask too, which
is how the suites pin which locations stray.

## Reading a module

`motionvm-motion-tools script <gamedata> <id>` prints one module's header,
symbol table and threaded code with every kernel word resolved by name
through the game's own binary; `extract` writes every module out that way
and a `kernel-usage.txt` beside them counting what the game reaches for.
The tool marks which of those the interpreter itself owns; whether the
*engine* implements the rest is the engine's own suite's to say, and it does
(below). The [tools page](tools.md) has the rest.

## Inside the machines

Both interpreters keep a **trace** — every executed word, with where it was
— switched on from a test, and a **step limit** that stops a program looping
forever. The 32-bit machine also keeps a **watch**: name a cell and every
store to it is reported with the word that made it. None of these is on in a
normal run; a run without one behaves identically. The interpreter crate's
rustdoc names the calls (`Vm` in `motionvm-motion-forth`).

An error out of the machine names a place in the game's own bytecode —
`module:offset` — and, for a word whose handler has not been read, the
binary and the address a reader would have to go to. Those `Unread` stops
are the project's alternative to a stub: the run stops and says what to read
rather than drawing something plausible.

## The suites that hold the engine still

- **Reference digests.** Every scene the suites compose and every tune they
  play has a line in `crates/*/tests/digests/<slug>.txt`; a change that moves
  a pixel or a register write is a red test. `just digests` rewrites the
  tables from a run when the change was meant, and the diff is the review.
  [Verification](verification.md#held-against-itself) says what a digest can
  and cannot say.
- **Kernel coverage.** `crates/motionvm-motion-engine/tests/kernel_coverage.rs`
  disassembles every module a game ships and asserts that every kernel word
  it reaches for is either one of the interpreter's primitives or a word the
  engine implements — the whole of the four 16-bit games, and all but a named
  handful of Dunkle Schatten 2's, each with the reason it is left unbuilt.
- **Damaged input.** `crates/motionvm-motion-formats/tests/malformed.rs`
  hands the readers inputs broken in ways somebody thought of;
  `tests/mutation.rs` walks the real files and breaks them in ways nobody did
  — cut at every length through the header, bytes flipped where a seeded
  generator says — and asserts that a reader answers rather than crashes.
- **The rigs.** `just bench` runs two `#[ignore]`d release-mode measurements:
  where a frame's time goes in Dunkle Schatten 2, and how fast each of the
  five games' machines runs — cells per second, host words per second, and
  the step and the draw timed apart. They assert nothing; they exist so a
  number quoted about the cost of anything here can be reproduced.

## See also

- [Verification](verification.md) — what has been held against the original, and what only against itself
- [Departures](departures.md) — every place motionvm knowingly does something else
- [Open questions](open-questions.md) — what is not yet read out of the original
- [motionvm-motion-tools](tools.md) — the inspection CLI
