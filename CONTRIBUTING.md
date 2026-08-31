# Contributing to motionvm

motionvm is a from-scratch Rust reimplementation of MOTION, the DOS adventure
authoring system. MOTION made a series of German advergames; five of them are
in this tree: *Im Netzwerk gefangen – Dunkle Schatten 2* (on the 32-bit
engine `ENGINE.EXE` V0.06.06/R109, 1996-10-22),
*Die Enviro-Kids greifen ein* (on the 16-bit engine
`ENVIRO.EXE`, 1996-08-27), *Jeff Jet - Abenteuer InfoHighway* (on
`HPPLAY.EXE`), *Hilfe für Amajambere* (on `BMZ.EXE`, 1995-06-05) and
*Victor Loomes – Das Spiel* (on `LL.EXE`, 1993-05-20) — the last three on
older builds of that same 16-bit engine, the last of them three years older
than the first and in an earlier framing of the container. It runs the
originals' compiled Forth bytecode natively, and its single hard constraint
shapes every convention in this document: **the original binary a game ships
with is the authority on what that engine does.** Code here is not merely
correct or incorrect; it is faithful or unfaithful, and fidelity is
established by evidence, not by plausibility. The code runs all five today:
Dunkle Schatten 2 end to end on the 32-bit engine, and Die Enviro-Kids greifen
ein — intro, locations, walk, conversations, music, saves — with Jeff Jet,
Hilfe für Amajambere and Victor Loomes on the 16-bit one. It is written for the
engine rather than for those five: what
a further MOTION game would need is its own file under `titles/`, and the
naming rules below are what keeps that cost down.

This document describes how to set up a working tree, what "done" means, and
the conventions the codebase holds itself to. The conventions are not
aspirational — the tree passes all of them today, and the gate below keeps it
that way.

## Getting started

**Toolchain.** The workspace tracks stable Rust via `rust-toolchain.toml` and
uses the 2024 edition. The minimum supported version is `rust-version` in
`Cargo.toml` (currently 1.97.0). It is not a floor forced by a feature — the
whole workspace and all its targets compile on 1.95.0, and the oldest thing
that really stops it is let-chains, stable in edition 2024 since 1.88.

It is a **ceiling**, and the ceiling is CI. The runner images bring their own
stable and lag the channel by a week or two, and the workflow deliberately
builds on that rather than downloading a toolchain three times per push. A
`rust-version` above what the images carry fails every job before a crate is
compiled — a freshly released stable sits above them for days.

So: use current language features where they make code clearer, but raise the
MSRV deliberately, in its own change with a stated reason, and only to a stable
the runners already have — never as a side effect of reaching for a new API.
What CI would say is checkable without waiting for a push — install the
version `rust-version` names and run the workflow's four commands on it:

```sh
rustup toolchain install 1.97.0 -c rustfmt -c clippy
export RUSTUP_TOOLCHAIN=1.97.0 RUSTFLAGS="-D warnings"
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo doc --workspace --no-deps --document-private-items
```

**Game data.** The games' files are not in the repository and cannot be; they
are copyrighted. The `justfile` reads six locations and one switch:

- `GAMEDATA_DS2` — a copy of Dunkle Schatten 2's game directory (`001.RSC`
  and friends); `GAMEDATA_ENVIRO` — a copy of Die Enviro-Kids greifen ein's
  (`DATA.-1-`, `ENVIRO.EXE`); `GAMEDATA_JEFFJET` — a copy of Jeff Jet -
  Abenteuer InfoHighway's (`DATA.-1-`, `DATA.-2-`, `HPPLAY.EXE`);
  `GAMEDATA_HFA` — a copy of Hilfe für Amajambere's (`DATA.-1-`, `DATA.-2-`,
  `BMZ.EXE`); `GAMEDATA_VLOOMES` — a copy of Victor Loomes' (`DATA.-1-`,
  `LL.EXE`). Each
  defaults to a sibling directory of this checkout (`../games/DS2`,
  `../games/ENVIRO`, `../games/JEFFJET`, `../games/HFA`, `../games/VLOOMES`),
  and that default is only passed on
  when the data is really there — so `just check` is green on a machine with
  no copy of a game, with that game's data-dependent tests skipping. A path
  you name yourself is always passed on, so a typo panics instead of quietly
  skipping the suite. The four 16-bit games are recognized by their engine
  binary, not by their container: they all ship a `DATA.-1-`.
- `SAVES` — the directory the games' own save directories are under, for the
  savegame tests: the `saves/` the program writes, not `saves/ds2/`, because
  that is what the engine is pointed at and it puts the game's name on itself.
  Savegames cannot be reconstructed, only played to, so there is no default
  that could work; without one those tests skip.
- `MOTIONVM_NO_GAMEDATA=1` — the opposite: no data at all, whatever this
  machine has. `just check-nodata` sets it, and that is the only way to
  reproduce CI's floor here, because the sibling-directory defaults above are
  anchored at compile time and neither an empty variable nor a different
  working directory escapes them. Worth running before a push that touches a
  test: the data-free path is the only thing CI proves, and a test that
  quietly starts needing a file was previously invisible until CI said so.

Tests that need data they cannot find skip themselves and say so. A *set but
wrong* path panics instead: a run that skips everything is indistinguishable
from a run that passes everything, so a mistyped path would otherwise report
green without executing a line.

**Running.** `just run-ds2`, `just run-enviro`, `just run-jeffjet`,
`just run-hfa`, `just run-vloomes` — one per game, and no game is the one you get for saying `just run`. This builds in
release mode, and that is not
optional ceremony: the frontend is a software renderer that walks every
physical window pixel on the CPU, and at opt-level 0 the result is a
slideshow — which reads as broken hardware rather than as a missing flag.
`MOTIONVM_PERF=1` prints frame timings when you are working on performance.

**Self-containment.** The repository must check out, build, test and *read*
on its own. Nothing anywhere in this tree may reference a private working
directory, reverse-engineering scratch material, a tool that is not shipped
here, or any document that is not in this tree. That includes prose: a
measurement keeps its rig ("under DOSBox-X") without naming the harness that
took it.

**Code and `docs/` stand alone from each other.** A comment states its
load-bearing fact in full rather than pointing at a documentation page, and
`docs/` describes the original engine and its formats without naming the Rust
file that implements a reader. Either may name the shipped CLI as a product.
The two are independent records of the same knowledge, and a pointer in
either direction turns one of them into a stub.

## The quality gate

```
just check
```

That is `cargo fmt --check`, `cargo clippy` over the whole workspace, the full
test suite with the game's files, and `cargo doc` with broken intra-doc links
promoted to errors. It is the definition of "green". Run it before calling any
change finished. For multi-step restructurings, run it after every step, so
that the step that broke something is the step you just took.

`just check-nodata` runs the suite the way CI does, with no game data at all;
it is worth a run before pushing anything that touches a test.

`.github/workflows/ci.yml` — whose header comment names every
`MOTIONVM_GAMEDATA_*` variable and counts the suite, so a game touches it
too — runs the same four commands on Linux, macOS and
Windows — but **without the game data**, which cannot be in the repository. So
CI is the floor and this command is the gate: CI proves the workspace builds,
lints, documents and passes its data-free tests on three platforms, and only a
local run with your own copy of the game exercises the engine at all. A green
CI badge on a change that broke the renderer is entirely possible.

## Rust conventions

**Follow the defaults.** This project deliberately adds nothing on top of
official Rust style. `rustfmt.toml` configures stock formatting; the
[Rust API Guidelines] and standard `RFC 430` naming apply as written. When a
question of style comes up, the answer that requires no configuration and no
memory is the right one.

[Rust API Guidelines]: https://rust-lang.github.io/api-guidelines/

**Formatting.** `just fmt`. The only sanctioned escape is `#[rustfmt::skip]`
on a data table whose line breaks carry meaning — applied to the item, never
by loosening a width globally.

**Lints.** Declared once in `[workspace.lints]` and switched on per crate
with `[lints] workspace = true`:

- `unsafe_code = "forbid"` — this project reads files and pushes pixels
  around; there is no reason for unsafe code, and `forbid` means an exception
  cannot be smuggled in locally without editing the workspace manifest.
- `missing_docs = "deny"` — everywhere, including public struct fields and
  enum variants, which are the bulk of a format reader's surface.
- `clippy::correctness` and `clippy::suspicious = "deny"` — the two groups
  that describe code which is probably wrong rather than code which is merely
  unfashionable. `pedantic` is deliberately absent: this codebase makes
  deliberate choices it would argue with. What it would say is mostly five
  lints — run `cargo clippy --workspace -- -W clippy::pedantic` to see them —
  and about two thirds of what it reports are the cast four,
  `cast_sign_loss`, `cast_lossless`, `cast_possible_truncation` and
  `cast_possible_wrap`, with `must_use_candidate` most of the rest.
  **The casts are the semantics.** A 16-bit
  Forth machine truncating an `i32` to a `u16` is reproducing what the
  original did; a lint asking for `try_into` there is asking the
  reimplementation to stop being one. Named here so that the next person does
  not have to run the lint to find out what it would have said.

`#[allow]` is legal but must carry its reason at the attribute — no bare
`#[allow]` anywhere in the tree.

## Language

All identifiers, comments, documentation, and user-facing strings are written
in **US English** (color, center, behavior, initialize, gray, artifact).
German appears only inside quoted original game data, which is reproduced
byte-for-byte. Note that *dialogue* is the standard US spelling for a
conversation and is used throughout for the game's dialogue machine.

## Comments and documentation

This is the heart of the house style. Roughly a third of the tree is
commentary, and what makes that worth reading rather than noise is a single
rule:

**A comment explains why, never what.** If a fluent Rust reader can see what
a line does by reading it, saying so again is clutter. What the reader cannot
see — and what this project uniquely owes them — is where a fact *came from*:
the intent behind a structure, the reverse-engineering evidence behind a
constant, and the reading that looks right and is not.

**Write the present tense.** Every comment, doc page and manifest note in this
tree describes the state of the tree. "We do A because B would be wrong" is
the load-bearing form and must stay complete — it is the reverse-engineering
record. "A was changed to B", "this used to be C", dated events and progress
notes are not: they describe a tree that no longer exists, to a reader who
never saw it. A near-miss reading is worth keeping and belongs in the present
tense as a standing hazard — *"the event stream is at `+0x57`; `+0x0c` merely
holds 75 and reads like a plausible offset"* — not as an account of who read
it wrongly and when.

Concretely:

- **Every file opens with `//!` module documentation** — all of them, tests
  included. Long module docs use headings and bolded lead-ins to stay
  navigable.
- **Cite the original inline.** Facts read out of `ENGINE.EXE` cite the
  address in the relocated image as a backticked literal in the sentence:
  "the jump table at `0x98967`"; facts read out of `ENVIRO.EXE` cite the
  file offset ("the core table at file `0x2064e`"), with the load image's
  `segment:offset` where a far pointer is meant. Do **not** extract these
  addresses into named constants — a literal sitting next to its citation
  is the house style, because the address *is* the evidence and belongs
  where the claim is made.
- **Name the evidence class — and the game.** A claim is read from the
  disassembly, measured against the running original under DOSBox-X,
  measured over a game's shipped files, or a hypothesis — and the reader
  must be able to tell which. A measurement taken on one game's files or
  one game's run names that game ("all 86 modules of Dunkle Schatten 2",
  "Die Enviro-Kids greifen ein under DOSBox-X"); a sentence that names no game
  holds for all of them. Never let
  a guess wear the clothes of a fact.
- **Quote measurements with their rig.** "~978 Hz, measured on the original
  running under DOSBox-X, timed by its own unit-3 timer" is evidence;
  "~978 Hz" alone is folklore in the making.
- **Back format claims with corpus counts.** "All 1678 shipped sprites decode
  to exactly the length their headers declare" settles an argument that "the
  header declares the length" merely opens.
- **Guard the shapes that look improvable.** Where code deliberately stays in
  a shape a reader would want to simplify — duplicated twin functions that
  mirror two original handlers, say — write on the item why the simplification
  would be wrong, in the present tense. Not "this was tried and reverted": say
  what folding them would cost.
- **No TODO markers.** There are no `TODO`, `FIXME`, `XXX`, or `HACK`
  comments in this tree, and none may be added. Open state lives in a named
  document: unknowns about the original go to `docs/motion/open-questions.md`,
  where they are tracked rather than scattered.

**The rustdoc gate.** `just doc` treats broken intra-doc links as errors and
documents private items, because in this project the private comments carry
as much of the evidence as the public ones — the `ENGINE.EXE` citations sit
wherever the code does, and a link checker that covered half of them would
only read as if it covered them.

## Architecture

The workspace is two layers — a neutral one that knows no engine family, and
the MOTION family's crates, marked as such by their names — with the contract
crate between them; [`ARCHITECTURE.md`](ARCHITECTURE.md) lays the shape out.
Dependencies point strictly downward:

| Crate | Role |
|---|---|
| `motionvm-motion-formats` | Readers for the shipped file formats; std-only, zero dependencies |
| `motionvm-motion-forth` | The Forth VM: address model and kernel words |
| `motionvm-render` | Framebuffer, palettes and blits |
| `motionvm-playable` | The window-to-engine contract |
| `motionvm-motion-audio` | FM driver, HMI and PSM 2 sequencers, OPL3 |
| `motionvm-motion-engine` | The runtime that ties them into the game loop |
| `motionvm-motion` | The family's front door: engine and audio, paired behind the contract |
| `motionvm-workspace-tests` | The rig: tests over the tree itself |
| `motionvm-app` | The window: winit/softbuffer frontend |
| `motionvm-motion-tools` | CLI for reading and extracting the shipped formats |
| `motionvm-motion-testutil` | Test-data location, shared by every suite |

The same crates carry both generations; the rule for how they share is in the
next section, and what the whole shape is — the two layers, the four levels of variance, the
seams between them, and why some things are shared behind a `Rules` struct
where others are written twice — is in
[`ARCHITECTURE.md`](ARCHITECTURE.md). Principles the layout encodes:

- **Readers never panic on malformed input.** A damaged install must surface
  as an error value, not a crash report about this codebase.
- **Determinism is engineered, not hoped for.** The engine reads no wall
  clock, the RNG is a seeded LCG, no `HashMap` sits on any drawing path, and
  resource directories are walked sorted. A given input state renders a given
  frame, every time — which is what lets anyone with the game compare two
  renderings of a scene across a change and trust the difference.
- **Structure mirrors the original where that buys checkability.** One
  function per original handler, even when two handlers are near-twins;
  dispatch arms in the order of the original's dispatch, where position is
  load-bearing. The point is that any piece of this code can be held up
  against its counterpart in the binary.

## Two generations, and what belongs to a game

Above everything in this section sits the layer boundary: what is **the
window's** — winit, cpal, the scaling, the roster — lives in the neutral
crates, names no family and cites no family's evidence, and the boundary test
in `motionvm-playable` holds that line. Everything below is placement *within*
the family's crates.

Three kinds of things live in a family's crates, and each is named by a
different rule. What is **shared by the engine family** — the `Host` boundary, the
branch arithmetic, the descriptor scene graph, the framebuffer, the palette
and font-reference readers, the curtain, the clock, the CLI skeleton —
carries no tag: an unqualified name or sentence holds for both generations.
What exists in **both generations with a different shape** — the VM
binding, the kernel table and its ordinals, the container, the module
format, the sprite, font and text encodings, the display size, the music
format — is tagged by **engine generation**: in code, sibling modules
`m16` and `m32` inside a crate whose root keeps only what both share; in
`docs/`, the trees `motion16/` and `motion32/`; in prose, "MOTION 16-bit
(`ENVIRO.EXE`)" and "MOTION 32-bit (`ENGINE.EXE` V0.06.06/R109)". A format
that exists in one generation only (`rsc`, `hmi`, `le`; `dat`, `psm`, `mz`)
still lives under that generation's module, so that the unqualified level
stays honest. What belongs to **one game** — its bootstrap words and
module numbers, the names of its script variables, its module map and
location scheme, its title strings, tests that drive its data — is tagged
by **game**: code under `titles/ds2`, `titles/enviro`, `titles/hfa`, `titles/jeffjet` and
`titles/vloomes`,
documentation under `docs/motion/games/<game>/`, and a sentence that names the game.
Where the games of one generation share something — the 16-bit opener, the
frame handler, the location mechanism — it belongs to the generation and lives
under its name (`titles/motion16.rs`), not copied into each game's file.

Never mix the two generations' readers or machines in one function: a
reader reads one layout, and the caller chooses the reader. Where the same
kernel word name means something different in the two generations, the two
handlers are two functions, each citing its own binary.

The tree holds to this today. Every page under `motion32/` and `motion16/`
states its generation in the line under its title, every page under
`games/` its game, and the two ledgers `departures.md` and
`open-questions.md` keep a section per generation. `motionvm-motion-formats` and
`motionvm-motion-forth` carry both generations as `m32` and `m16` with only the
shared things at their roots; `motionvm-motion-tools` dispatches on the game's
files to an `m32` and an `m16` command set; `motionvm-motion-engine` hosts both
machines behind one `Game<M>` driver, keeps each game's bootstrap under
`titles/`, and keeps the one word group that exists for one machine only —
the 16-bit words in `words/m16.rs`, buffer words among them — in a file of
its own; the walk, the inventory, the savegame words and the dialogue queue
each serve both machines over `AddressSpace` and a `Rules` table. `motionvm-motion-audio` carries
the two music stacks the same way, `m32` for the HMI sequencer and the FM
driver and `m16` for PSM 2, with the OPL3 they both end at at its root.
Text drawing lives in the engine as the pair `text.rs` and `text16.rs`:
one glyph and one measure serve both generations, and the 16-bit run
drawer, which lays a line down by rules of its own, is the twin file —
while `motionvm-render` underneath knows only the surface and the blits.
The window serves every game through the contract and never
learns which generation it is looking at: the music, the keys and the pacing
are all the family's answers, made behind `motionvm-motion`.

## Adding a game

MOTION made more games than the ones this plays, and the shape above is what
keeps a further one cheap. Cheap is not free, though, and it is not one file:
what follows is every place a game is named, in the order it makes sense to
work through them. It is a checklist because the alternative is doing it from
memory, and from memory the code gets done and the prose does not: a sentence
that counts the games is wrong the moment there is another one, and nothing
fails when it is.

**The engine**

1. A variant of `Title`, and a line in each of its five total matches —
   `name`, `short`, `slug`, `needs`, `generation` — and in `ALL`. They are
   total on purpose: the start-up message that lists what each game needs is
   built from `needs`, the window's help from the roster's cards, and the
   family's music opener from `generation`, so none of them can fall behind
   the roster.
2. A module `titles/<slug>.rs`: `REQUIRED` — each file and what it is for,
   quoted verbatim in the error message — `LOCATION`, and `missing_data` and
   `open` forwarding to the generation. On the 16-bit side it also names
   `ENGINE`, the binary its kernel table comes out of; on the 32-bit side
   `SIGNATURE`, the words that tell its container from another game's. Nothing
   else belongs here: if the game needs behavior, that is a finding about the
   *engine* and belongs to the generation.
3. A line in its generation's `GAMES` table, and `pub mod` plus an `open` arm
   in `titles/mod.rs`.

**The test rig**

4. `motionvm-motion-testutil`: a `gamedata_<slug>()` beside the others, probing for
   the file that identifies the game, and its `MOTIONVM_GAMEDATA_<SLUG>` in
   that crate's module doc.
5. The `justfile`, in six places: the header comment that names the variables,
   the `DEFAULT_GAMEDATA_*`/`GAMEDATA_*` pair, the `_DATA_*` probe, the
   environment `test` passes, the same in `test-one`, and a `run-<slug>`
   recipe. No game is the default for `just run`.
6. Tests that drive the game's data, one file per subject, each named for the
   game and closing its `//!` with the line every such file carries: *The game
   this file drives is `<title>` (MOTION 16-bit).*

**What a person reads**

7. `motionvm-motion-tools`: the `USAGE` string and the crate's module doc, both of
   which list the games and the files that identify them.
8. `README.md`: the roster table, the "what a game needs" table with its four
   columns, the `--loc` paragraph where a game's opening needs a word, the
   per-game menu sentence under Controls, and the list of save directories.
9. `docs/motion/README.md`: the roster table, a block of rows in the documentation
   map, and a section under "The shipped files".
10. `docs/motion/games/<slug>/`: the five pages the other games have — the game, its
    structure, its module map, its resource inventory, its other files — with
    the `Sources` discipline that separates what the published record says from
    what was read out of the binaries.
11. `docs/motion/verification.md`: what has been held against the original's own
    output for this game, and what only against its files.
12. A line under `## [Unreleased]` in `CHANGELOG.md`.

**Then check the counts.** Every sentence that says how many games or builds
there are, and every "the other game" that has just stopped identifying one,
is now wrong. Two greps find them:

```sh
grep -rniE '\b(two|three|four|five) (16-bit |32-bit )?(games?|titles?|builds?)\b' .
grep -rniE 'the other( two| three)? (16-bit |32-bit )?games?' .
```

Neither is a formality. The hits run into the dozens, they are spread over
code comments, test headers, the `justfile`, both READMEs and half the
documentation tree, and not one of them is caught by anything else in the
gate.

**If it is the second game on the 32-bit engine**, three things that are
simplifications today stop being any: `motion32::detect` answers from the
container shape alone because its table has one entry (the compile-time guard
beside the table refuses a second entry until this moves); the signature
check that would tell two 32-bit games apart happens later, in `open`; and
`motion32`'s shell and boot functions reach the one manifest's `SHELL`
constant directly — a second game moves that constant into the opener's
plumbing, beside `LOCATION`. Move the check into `detect` and the shape
becomes what it already is on the 16-bit side — a narrowing, with the game's
own data settling it.

## Adding an engine build

A build of the same engine is not a game and mostly not code. The point of
probing rather than assuming is that a build usually costs nothing at all;
when it costs something, this is where the something is.

1. **Check what the scan makes of it first.** `mz::binding_of` reads the
   kernel tables out of the MZ image and derives the domain ordinal base by
   disassembling the registration loop. If that succeeds and the inline words
   are all found, the machine binds and the build costs a manifest.
2. **A missing inline word is a decision.** `Inline::by_name` requires every
   name but `_PutStringAdr` and `_ChElseDup`, which are optional because a
   build that lacks them exists. Making a third one optional is a claim about
   the engine family and needs its evidence.
3. **A behavior that differs is a capability, not a version test.** Read it out
   of the binary the way `?XINSIDE`'s hole-skipping is read, name it for what
   it does, and cite the address on the field. Never `if build == …`.
4. **Constants that differ go in a `Rules` table; readings that differ go in a
   second file.** The criterion is in [`ARCHITECTURE.md`](ARCHITECTURE.md).
5. **Nothing about the build belongs in a game's module.** What the games of a
   generation share — the opener, the frame handler, the location mechanism —
   is the generation's, in `titles/motion16.rs`, and a game's file holds
   constants only.

The documentation costs more than the code:

6. A page `docs/motion/motion16/engine/<binary>-exe.md` of its own — what the image
   is, where its tables are, what its ordinals do, what its game asks of it —
   and its row in `docs/motion/README.md`.
7. A *See also* bullet on each sibling build page, and a look at what those
   pages say about how many builds there are and which is oldest.
8. **The scope line under the title of every page in that generation's tree**
   names every build. That is one line on some thirty pages, and it is the
   thing most likely to be forgotten.
9. The build's name in `CONTRIBUTING`'s fidelity section, which lists the
   binaries behavior is matched against.

## Adding an engine family

A further family is a bigger thing than a game or a build: a different
original, its own formats, its own crates. The architecture makes room for it
without being edited — the shape is in [`ARCHITECTURE.md`](ARCHITECTURE.md)
under *Extending the tree* — and this is the order that keeps each step
compiling:

1. **A crate set of its own, named for the family.** `motionvm-<family>-*`
   for its parts — however many it needs; one is a fine start — and a front
   door `motionvm-<family>` holding its `Family` implementation. The family
   reaches the neutral layer only through `motionvm-playable` and
   `motionvm-render`; nothing in it names the window, and nothing in the
   window names it.
2. **Implement the contract.** `Playable` for its games, with the family's
   own error type behind the contract's boxed one — the `Display` output is
   what a player reads, so the family's tests should pin its messages.
   Translate the contract's `KeyPress` into whatever the family's scripts
   read, the way the MOTION engine's `keys.rs` does, and answer
   `Playable::open_music` with the audio thread's source, with `None` for a
   game that has nothing to play, or with an error that says why sound is
   off — how the music reaches the source stays the family's own, behind
   its front door.
3. **One roster line.** `roster.rs` in the window is the only platform edit:
   the usage text, the folder dialog and every message follow from `games()`
   and the openers. The roster's fallback rule — a directory nobody claims is
   answered by the first family's refusal — stops being exact at this point;
   give the unrecognized directory an answer built from every family before
   the line lands.
4. **The boundary test's list.** The family's name goes into `FAMILIES` in
   `motionvm-workspace-tests/tests/boundary.rs`, which is what makes the new
   crates family crates in the layer rule's eyes.
5. **Its games, its data, its rig.** Each game then follows *Adding a game*
   inside the new family: lookup functions, justfile plumbing, suites, and
   `docs/motion/games/` trees. A family whose originals need different rig rules
   brings its own testutil crate rather than growing MOTION's. **Slugs are a
   workspace-wide namespace**: `saves/<slug>/`, `MOTIONVM_GAMEDATA_<SLUG>`
   and `docs/motion/games/<slug>/` are shared with every family, so a new game's
   slug is checked against every roster, not just its own — a collision puts
   two games' savegames in one directory.
6. **Its own documentation trees.** A family's pages live under
   `docs/<family>/…` — MOTION's under `docs/motion/`, ledgers included —
   with an index of their own, and `docs/README.md` at the root is the map
   that carries a row per family. Every page's scope line names what it
   covers, the way the generation trees' pages do.
7. **The stale-count grep.** Sentences that count families are wrong the
   moment there is a second one:

   ```sh
   grep -rniE '\bone (engine )?\*?family\*?\b' .
   ```

   finds them, beside the two game-count greps under *Adding a game*. The
   emphasis marks are in the pattern because the sentence most likely to go
   stale writes the word as `*family*`; the pattern is line-based, so keep
   the phrase on one line wherever it is written.
8. **The changelog.** A new family is the loudest thing a release can carry;
   its entry belongs at the top of *Added*.

## Fidelity and accepted divergences

Behavior is matched to the original binary of the generation in question —
`ENGINE.EXE` V0.06.06/R109 for the 32-bit engine, `ENVIRO.EXE` and its older
builds `BMZ.EXE`, `HPPLAY.EXE` and `LL.EXE` for the 16-bit one. When motionvm and the original
disagree, motionvm is wrong —
unless the divergence is recorded, which is what stops anyone "fixing" a
deliberate decision. **The record is [`docs/motion/departures.md`](docs/motion/departures.md)
and only that** — every divergence is written there, where the code lives, in
the section of the generation it concerns. The ten below are the ones worth
knowing before reading any of this code; they are a summary of that page, not
a second list, and a divergence that is here and not there is a mistake. Every
one of them concerns the 32-bit engine as it runs Dunkle Schatten 2 unless it
names the 16-bit engine:

1. **Savegame formats are motionvm's own.** The original stores raw DOS4GW
   heap pointers; matching that is structurally impossible.
2. **Module 0 reads answer 0**, counted and reported. The kernel's base
   module is built at runtime and cannot be reconstructed.
3. **The OPL3 is `nuked-opl3`**, bit-identical to the reference emulator;
   nothing in the game defines what the chip does with a register.
4. **The digital-audio layer is not ported.** The shipped game never calls
   it, and no sampled-audio data exists in any container.
5. **The authoring half of MOTION is out of scope.** motionvm runs games; it
   does not author them.
6. **`Memory::lookup` walks modules by number**, not by the original's table
   order; all known duplicate word names live in modules never loaded
   together.
7. **`Vm::call_nested` does not block** where the original's re-entrant
   handlers do.
8. **`rsc.inf` is never rewritten.** motionvm treats the game directory as
   read-only — a guarantee the original does not make.
9. **The 16-bit engine's disk-change prompt is unreachable.** Every
   `DATA.-n-` volume is opened when the game is opened, and a missing one
   refuses the game rather than emptying the slots it holds.
10. **A font reference table entry past the end of the font draws nothing.**
    Jeff Jet's table was written for a larger font than it ships; no shipped
    string reaches one of the two entries concerned.

Adding a divergence is a deliberate act: document it where the code lives,
state why matching the original is impossible or undesirable, record it in
`docs/motion/departures.md`, and point at that entry from the page it concerns. This
summary grows only when the new one belongs among the first things a reader
should know.

## Testing

- **A test name is a sentence** stating the fact it proves:
  `the_title_screen_matches_the_original_engine`,
  `escape_opens_the_quit_page`. If the fact will not fit in a name, the
  test's doc comment carries it.
- **Every test file documents what it verifies and against which evidence** —
  the same evidence discipline as the code.
- **Fixtures come from your own game install**, located through
  `motionvm-motion-testutil`, never checked in. Remember the rule from setup:
  missing data skips loudly, a wrong path panics.
- **A test that needs a game's files says which game.** It asks
  `motionvm-motion-testutil` for that game — `gamedata_ds2()`, `gamedata_enviro()`,
  `gamedata_jeffjet()`, `gamedata_hfa()` or `gamedata_vloomes()` — and closes
   its `//!` with the one
  line every such file carries: *The game this file drives is `<title>`
  (MOTION 16-bit).* The title is the game's short form, the one prose uses.
  It is spelled out because the module numbers, word names and ids the file
  asserts are that game's and would be nonsense against another.
- **A test save directory carries its game's slug.** `saves_dir` wipes what it
  hands back and one directory under `target/` serves the whole suite, so two
  files asking for the same bare name would delete each other's slots mid-run.
  `ds2-`, `enviro-`, `jeffjet-`, `hfa-`, `vloomes-` — the prefix is what makes that
  impossible rather than merely unlikely.
- **There are no golden frames, and their absence is a real loss.** A
  pinned rendered scene is the one kind of check that catches a change
  nobody thought to assert — and a checked-in baseline is a rendering of
  the game's own artwork, which the rule above forbids. So there is none.
  `tests/ds2_scenes.rs` drives the engine to four scenes and asserts it
  arrives and draws something, which covers the navigation but not the
  picture. If you are changing the drawing path, render the scenes before
  and after on your own machine and compare them there — the determinism note
  above is what makes that trustworthy.
- **Measure before optimizing.** `MOTIONVM_PERF=1` and the frame timing
  `motionvm-app` prints under it exist so that performance claims are
  measurements.
  Plausible arguments about frame cost are wrong in both directions: the
  obvious candidate turns out to cost nothing measurable, and the real hot
  spot sits in a routine nobody suspected.

## Documentation pages

The reverse-engineering knowledge lives in `docs/` — format specifications,
the VM's execution model, the engine's subsystems, and a page per script
module. When code changes what is known, the docs move with it in the same
change. Conventions:

- Pages are laid out by family, then by engine generation and by game:
  `docs/motion/motion32/`
  and `docs/motion/motion16/` hold the format, VM and engine pages of the two
  generations, `docs/motion/games/<game>/` the pages about
  one game's own data and script library. Every generation page carries a
  one-line scope note under its title naming its engine and the game it
  was measured on; every game page names its game the same way.
- Every page opens with a breadcrumb back to the documentation index and ends
  with a **See also** link list. A page carries an **Open questions** section
  when it has open questions; empty boilerplate sections are noise, not
  honesty.
- The family's index (`docs/motion/README.md`) fixes the shared vocabulary
  once: values are
  little-endian, text is CP437, a cell is 32 bits under `motion32/` and 16
  under `motion16/`, stack effects use Forth's `( a b -- c )` notation, and
  engine addresses refer to `ENGINE.EXE`'s *relocated* image for the 32-bit
  engine and to file offsets in `ENVIRO.EXE` for the 16-bit one.
- **File sizes are decimal KB and MB**, everywhere a round figure is given —
  845 KB for a file of 845 467 bytes, 7.6 MB for one of 7 609 296. Exact byte
  counts are written out in full and grouped with thin spaces. Two
  conventions in one column is the kind of drift nothing catches — the
  README's file tables carried both for a release before anyone
  measured them.
- Unresolved questions about the original belong in
  `docs/motion/open-questions.md`, the single ledger of what is not yet known, in
  the section of the generation they concern.
- Deliberate differences from the original belong in `docs/motion/departures.md`,
  the single ledger of where the code knowingly does something else. The
  subsystem pages describe the original engine; a departure is recorded there
  and pointed at from the page it concerns.
- The documentation contains nothing derived from a game's copyrighted
  data beyond what a specification needs: layouts, counts, ids, names and
  short quoted strings — never a decoded listing of a script module, a
  dumped table or a rendered picture.
- User-visible changes get a line under `## [Unreleased]` in `CHANGELOG.md`, in
  the same change that makes them — the same rule as the docs above. The format
  is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), whose six
  category headings are the only ones it has; "user-visible" means a reader of
  the release notes would care, not that a file moved.

## Dependencies

The dependency list is short on purpose, and every entry in
`[workspace.dependencies]` carries a comment saying what it is for and what
license it carries. A new dependency needs that justification, a license
check, and — if the license imposes obligations, as `nuked-opl3`'s
LGPL-2.1-or-later does — the corresponding entry in `NOTICE`.

## Contribution workflow

- **Branch from the default branch**; keep changes small and single-purpose.
- **Never move code and change it in the same commit.** Moving an arm and
  moving a file at the same time makes neither reviewable.
- **Prove refactors neutral.** A restructuring that claims to change nothing
  should be able to demonstrate it — a whitespace-stripped comparison of
  before and after, an untouched test suite, a scene that renders the same
  before and after on your machine.
  Behavioral claims in a change description cite their evidence the same way
  code comments do.
- **Run `just check` before submitting.** Green is defined above; a change is
  not finished until it is green.
- For changes that touch fidelity — anything the original engine also does —
  state the evidence: the address read, the measurement taken, or the
  hypothesis being made, and update `docs/`, `docs/motion/open-questions.md` and
  `docs/motion/departures.md` accordingly.

## Releasing

A release is a tag `vX.Y.Z` on `main` plus the two zips
`.github/workflows/release.yml` builds from it — a self-contained
`motionvm.exe` and a universal `motionvm.app`. The workflow runs on the tag
and ends in a **draft** release; a person reads the draft and publishes it.
Nothing is released by a push alone.

1. **Bump the version.** `version` in `[workspace.package]` in `Cargo.toml`,
   following semver — while the major is 0, a change to the command line is a
   minor bump. `cargo build` once, so `Cargo.lock` carries the new number.
   Rename `## [Unreleased]` in `CHANGELOG.md` to `## [X.Y.Z] - YYYY-MM-DD`,
   put an empty `## [Unreleased]` above it, and move the link references at
   the foot of the file along: `[Unreleased]` compares against the new tag,
   and the new version gets a line of its own.
2. **Gate.** `just check`; commit, push, and wait for CI on all three
   platforms.
3. **Tag.** `git tag -a vX.Y.Z -m "vX.Y.Z"`, then `git push origin
   vX.Y.Z`. The workflow refuses a tag that does not match the Cargo version,
   so a tag cannot lie about its contents, and it takes the release notes from
   the CHANGELOG section of that version, so a missing section fails it too.
4. **Publish.** Open the draft on GitHub: two zips and the source archive, the
   notes as in the CHANGELOG. Fix what reads wrong, then publish.

**Dry run.** "Run workflow" on the Release workflow in the Actions tab — or
`gh workflow run release.yml` — builds the same two zips as workflow artifacts
and creates no release. That is how to try a change to the packaging without
a tag.

## License

motionvm is MIT-licensed; contributions are accepted under the same license.
`NOTICE` records third-party obligations. The game's data files are
copyrighted and must never enter the repository — that includes captures,
screenshots, and any fixture derived from them.
