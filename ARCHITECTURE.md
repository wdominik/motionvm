# The architecture of motionvm

motionvm virtualizes game engines. It ships one engine *family* today —
MOTION, in a 16-bit and a 32-bit generation, the 16-bit one in five distinct
builds and the 32-bit one in two, with the games on top — and holds the
family apart from the window
that plays it the same way it holds the generations apart from each other.
Family, generation, build and game are four different axes, and almost every
decision in this tree is about keeping them apart.

This page is what the shape is. [`CONTRIBUTING.md`](CONTRIBUTING.md) is how to
work in it — the placement law that follows from this, the quality gate, the
procedures for adding a game, a build, or an engine family. `docs/` is the
reverse-engineering record: the original engine, its formats and its behavior,
described without reference to any of the code here.

## Two layers, and where the name of a thing puts it

The tree is two layers inside one workspace:

- **The neutral layer** — `motionvm-render`, `motionvm-playable`,
  `motionvm-app` — knows no engine family. Pixels, palettes, the contract a
  game is driven through, the window and its devices: everything in it holds
  for whatever engine sits behind the contract, and none of it names one.
- **A family** is a set of crates behind one implementation of the contract's
  `Family` trait. MOTION's set is the six `motionvm-motion-*` crates plus its
  front door, `motionvm-motion`, which is the crate a roster line points at.

The layer is written in the name, not in a directory: a crate that exists for
one family carries the family's name — `motionvm-motion-engine` — and an
unprefixed `motionvm-*` name belongs to the neutral layer. Unprefixed names
that would suit genuinely shared code (`motionvm-formats`, say) stay free for
the day two families really share some; nothing is reserved by an empty
placeholder. The `crates/` directory itself stays flat.

The two layers meet only at `motionvm-playable`, and the one sanctioned
crossing above it is the window's roster: one manifest line and one file,
`motionvm-app/src/roster.rs`, naming each family's front door and nothing
deeper. That is not a convention: the rig's boundary test
(`motionvm-workspace-tests`) reads the workspace's manifests and sources and
fails on a dependency or a piece of family evidence on the wrong side of the
line. `motionvm-render` and
`motionvm-playable` compile with no family present, and the whole family
compiles with no window; only the roster ties the two together.

## The workspace

Eleven crates, dependencies pointing strictly downward.

The neutral layer:

| Crate | Role | Depends on |
|---|---|---|
| `motionvm-render` | Indexed framebuffer, palettes and blits | — |
| `motionvm-playable` | The contract between the window and whatever engine it drives | `render` |
| `motionvm-app` | The window: winit, softbuffer, cpal, and the roster of families | `playable`, `render`, `motion` |

The MOTION family:

| Crate | Role | Depends on |
|---|---|---|
| `motionvm-motion-formats` | Readers for the shipped file formats; std-only, zero dependencies | — |
| `motionvm-motion-forth` | The two Forth machines behind one trait set | `formats` |
| `motionvm-motion-audio` | The two music stacks and the OPL3 they share | `formats` |
| `motionvm-motion-engine` | The runtime: descriptors, drawing, text, screens, the game loop | `formats`, `forth`, `render`, `playable` |
| `motionvm-motion` | The family's front door: the engine and its music, paired behind the contract | `playable`, `engine`, `audio`, `formats` |
| `motionvm-motion-tools` | The CLI that reads and extracts the shipped formats | `formats`, `forth`, `render` |
| `motionvm-motion-testutil` | Test-data location, shared by every suite; not published | — |

And outside both layers:

| Crate | Role | Depends on |
|---|---|---|
| `motionvm-workspace-tests` | The rig: tests over the tree itself, the layer law above all; not published | — |

Three absences in those tables are load-bearing:

- **`motionvm-motion-engine` does not depend on `motionvm-motion-audio`.** The
  seam is the engine's own `MusicSink`, two methods wide: the engine hands
  whoever is listening the song's bytes exactly as the original hands the
  file to its MIDI layer, and never learns the codec. Only `motionvm-motion`
  knows both worlds: its front door opens the music and installs the sink,
  which is that crate's whole job.
- **`motionvm-app` does not depend on `motionvm-motion-engine`.** The window
  drives a `Box<dyn Playable>` and holds a list of families; the only family
  crate it names is a front door, and only in `roster.rs`.
- **What a build decides is a different type from what a frame changes.** The
  engine holds a `Profile` — every capability its opener read off the shipped
  binary, the savegame layout, the display mode — which is `Copy`, has no
  method that changes it, and arrives whole through `Engine::new`. Everything
  else is state, and it is grouped by subject rather than lying flat: `Scene`
  (the descriptors and what a picture is drawn from), `Input`, `Transitions`,
  `Cursor`, `Sound`, `Persistence`, `Dialogue`. So `self.profile.opaque_blocks`
  is visibly a fact about the engine build and `self.input.polls` visibly is
  not; on one flat struct neither would be visibly anything.
- **There are no Cargo features anywhere in the workspace.** Family,
  generation, build and game variance is expressed in crates, modules, types
  and values read from the game's own files — so every configuration is always
  compiled, and CI's data-free floor type-checks all of it.

Policy is central: `unsafe_code = "forbid"` and `missing_docs = "deny"` over
the whole workspace, with private items in the documentation build, because
here the private comments carry as much of the evidence as the public ones.

## Four levels, four mechanisms

### Family is a crate set and a contract

A family is not a value the program switches on — there is no family enum
anywhere. It is a set of crates behind one `Family` implementation: `name` for
prose, `games` for the roster a window shows, `detect` to claim a directory by
its file names, `open` for the game. Music is the opened game's own answer —
`Playable::open_music` takes the device's rate and hands back the source the
audio thread renders, or nothing for a game with nothing to play; how a song
reaches that source never crosses. What crosses the contract is
deliberately platform-shaped: a `Framebuffer` and a `Palette`, a `KeyPress` as
a keyboard reports one, samples for a device — never a format, a word, or a
code an engine's scripts read. The window's usage text and its complaints
about a wrong directory are built from the roster, so what the program plays
is what its help names; the folder dialog itself asks for the game directory
in one sentence.

### Generation is a type

There **is** one `Generation`, in `motionvm-motion-formats`, and what it
selects is *formats*: which container the resources are in, how a script module
is laid out, which savegame magic is written, which music stack plays. It never
selects behavior. What the engine does differently is a capability, named for
the behavior and carrying the address it was measured at — see below, and note
that three of those already vary *within* a generation, which is the whole case
for the split. One type and not one per crate: a roster's, a savegame
layout's and a tool's own enum for the same question would agree by convention
only. The one place two arms still say "which generation" is the engine's
resource holder, and they stay because they hold two different container
types — a place to put them rather than a question to ask.

There is no `EngineVersion` enum and no `if version >= n` anywhere. The two
machines are two concrete types — `motionvm_motion_forth::m16::Vm` and `m32::Vm` —
and they are **two interpreters, not one parameterized by a width**, because
the two originals are: the 32-bit engine keeps a loop's limit in a frame of its
own, the 16-bit one keeps limit and index on the return stack, their `EXECUTE`s
take different things, and each was read out of its own binary. What they do
share is their own bookkeeping — the step budget, the trace, the counters, the
generator, the nesting depth — and that lives in one `Core` both hold. They
meet the rest of the workspace behind three traits at that crate's root:

- **`Machine`** — start, resume and park an execution, look a word up, read a
  variable, and say how many cells and host words it has run. What the generic
  driver drives.
- **`Host<M>`** — how a kernel word the machine does not own reaches the
  engine, *by ordinal*. The engine resolves each of its kernel's ordinals into
  one of its own words when the game opens, and indexes that table thereafter;
  a name is materialized only to report one that is not implemented.
- **`AddressSpace`** — machine-neutral memory access. `cell_size()` answers 2
  or 4, and the shared word groups take `&mut dyn AddressSpace`, so one
  implementation serves both cell widths.

The same split runs through the format readers (`m16`/`m32`), the music stacks,
the CLI, and the documentation trees — where the code module `mNN` and the
directory `motionNN/` name the same generation, module-short and
directory-readable. In the engine's own files the rule is total: an
unqualified name — `text.rs`, a bare word group — holds for both generations,
and anything that holds for one carries its number, `dialogue32.rs` beside
`dialogue16.rs`, `plain_word32` beside `plain_word16`, `m32::Vm` beside
`m16::Vm` at every signature.

### Build is probed from the binary

This is the most distinctive part of the design, and the part that pays for
itself when an unknown build turns up. The five 16-bit builds differ in kernel
size, in ordinal base and in a few behaviors, the two 32-bit builds in kernel
size and in two behaviors, and **none of that is keyed on a game name**:

- The kernel binding is scanned out of the shipped executable, and the domain
  ordinal base is derived by disassembling the registration loop in the MZ
  image — the `cmp %si, imm8` bound gives 105 for three builds and 102 for
  `LL.EXE`.
- Whether this build's `?XINSIDE` skips an all-zero hot area is read out of its
  code as well, and wired into the engine as the one capability that genuinely
  varies *within* a generation.
- Which inline ordinals the branch decoder uses come from the same scan, on
  both machines.
- The PSM driver's table location is chosen by counting the driver file's
  entries, not by asking which game is running.
- The 16-bit container's two framings are told apart by a structural identity
  test on the offset tables.
- The 32-bit kernel binding is derived from the binary's own init rather
  than tabled, so both shipped builds — and a third — bind by one reading.
- What a 32-bit build's `SDINSERT` pops — a slot and a value, or a kind
  beside them — is counted off the handler's pop calls, and the text layout
  converts the five insert slots by that reading.
- Whether a 32-bit build's curtains wait between their bands is read off
  `FADEIN` and `FADEOUT`: the one division of the duration by the band count
  is there in R109's handlers and absent from R78's.

### Game is an enum and a manifest

`Title` has one variant per game, and every accessor is a total match, so a
game cannot be added without a line in each — including the one the
"unrecognized directory" message is built from, which is what stops that
message falling behind the roster, and `generation()`, which is how the
family's own front door picks a music stack.

`slug()` is the universal key: it names the environment variable, the saves
subdirectory, the documentation tree and the module. A game's module under
`titles/` is a manifest — the files it ships and what each is for, the binary
its kernel comes out of, where it keeps its location, and for a 32-bit game
the words that tell its container from another's. It holds constants and no
behavior.

**No engine code branches on which game is running.** Not one arm of one match
in `motionvm-motion-engine` names a title.

## The seams

| Seam | Where | Separates |
|---|---|---|
| `Family` | `motionvm-playable` | the window from the very idea of a particular engine: a roster line is all a family is, from the outside |
| `Playable` | `motionvm-playable` | the window from the engine — and the engine stays out of reach, so a window cannot move the game's state and produce a picture the original could not |
| `KeyPress` | `motionvm-playable` | a key transition as a keyboard reports it — presses and releases both, the typed stream and the wheel beside them — from whatever an engine's scripts read out of one; the translation is the engine's |
| `AudioSource` | `motionvm-playable` | the platform's device from the family's synthesis; samples come back over `Playable::open_music`, nothing else crosses |
| `Driven` | `titles/mod.rs` | the front door from the engine: the family's own side of the contract, with `set_music` where the contract has `open_music` |
| `MusicSink` | `motionvm-motion-engine`'s root | the engine from the audio stack; bytes go over it, never a parsed song — family-internal, paired in the front door |
| `Machine` / `Host<M>` / `AddressSpace` | `motionvm-motion-forth`'s root | the engine from either machine |
| `Generation` | `titles/mod.rs` | one family-internal answer — which container, savegame layout and music stack — off the roster's total match |
| `Resources` | `resources.rs` | the only place engine code knows which container format is open |
| `Profile` | `profile.rs` | what a build decides when a game opens from what a frame changes; the two generations' readings are its two constructors |
| `Player` | `motionvm-motion-audio`'s root | a song from the driver that sounds it |
| `Picture` | `motionvm-render`'s root | a decoded picture from the container it was decoded out of |
| `LocationScheme` | `game.rs` | one location mechanism from five sets of variable names |
| `Rules` | `order.rs`, `words/` | one algorithm from the generation-different constants it runs on |

## Behavior differences are named capabilities

Where the two generations really do behave differently, the engine carries
sixteen named booleans, two named enums — what `TOGFX` does with the pointer,
what a text record makes of `SDINSERT` — a display size and a savegame layout
— `opaque_blocks`, `text_runs`, `per_screen_descriptors`, `skips_holes` and
the rest — each documented with the disassembly address it was measured at.
They live on the `Profile`, which is built whole before the engine exists:
`Profile::motion32` and `Profile::motion16` are the two readings. Six of the
fields are probed out of the shipped binary by the opener, which puts what it
read into the profile rather than into a built engine: `skips_holes`,
`walk_defaults_shrink`, `walk_smooths_headings` and `screen_holds_a_hundred`
on the 16-bit side, `inserts` and `curtains_wait` on the 32-bit.

Eleven of the booleans are, today, two-valued functions of "is this the
16-bit engine", and folding them into the `Generation` above would lose
nothing that is currently true. It is deliberately not done. Each was
*measured separately*, each names a behavior rather than a version, and the
six probed ones already vary within a generation — which is the whole case
for capabilities over version tests in a family whose next build is unknown.

## The rule for sharing, and the rule for not

The tree applies two strategies to the same-looking problem, and which one is
right follows from what differs:

- **When the constants differ, share the algorithm behind a `Rules` struct.**
  The order machine and the inventory bar read the same way on both machines
  over different offsets and coordinates, so there is one implementation and
  two tables of measurements, each field citing where it was read.
- **When the *reading* differs, write twin files.** `dialogue32.rs` and
  `dialogue16.rs` are the same conversation machine read twice, about 680 lines
  each, and they stay twins: the 16-bit engine has two speaker words instead of
  a speaker table, two talking-head descriptors, the answers stacked downward,
  colors and templates out of the block. A `Rules`-parameterized version would
  have to pretend those are the same shape with different numbers, and they are
  not.

The test is not how similar the code looks. It is whether one description of
the original is true of both engines. If it is, parameterize; if telling the
truth about one would require an exception in every other sentence, write the
second file. Mixing the two generations' readers or machines inside one
function is never right either way. Between the layers the same discipline
holds a different way: what an engine measured is said on the engine's side,
and the neutral layer states only its own contract.

## Rejected alternatives

The shapes a reader would want to reach for, gathered from the guard
comments beside the code so that what was weighed is visible without a
search. Each names what it would buy and what it would cost, in the present
tense, because the cost is still there.

- **A loop over function pointers for the kernel-word groups.** `word32` and
  `word16` in `words/mod.rs` ask sixteen or seventeen groups in turn, written
  out. A table of function pointers would fold that to five lines and put a
  layer between the reader and the list of groups — and the list, in the
  original's own section order, is the reader's map of the kernel.
- **Kernel words dispatched by name.** A `match name` per group, the first to
  recognize a string winning, makes the order of the groups load-bearing with
  nothing checking it: sixteen names mean a different handler on the two
  machines, and a call order would be the only thing telling them apart. A
  word is one value of an enum instead, resolved once per kernel when the
  game opens, and a duplicate name is a compile error (`words/word.rs`).
- **One primitive set generic over a cell width.** About twenty-six of the
  two machines' arms are the same one-liners; the rest are two engines read
  separately — the 16-bit one keeps a loop's limit and index on the return
  stack, the 32-bit one in a frame of its own, and `EXECUTE`, `LEAVE`,
  `WHILE` and both string skips differ with it. A `Cell` trait would write
  the twenty-six once and turn the other twenty-one into trait methods, an
  indirection between every measured address and the code it explains. What
  is genuinely one thing — the step budget, the trace, the counters, the
  generator, the nesting depth — is one `Core` (`motionvm-motion-forth`).
- **A `Rules`-parameterized dialogue machine.** The sharing rule above says
  it: the two conversation machines are the same machine *read twice*, and
  a parameterized one would have to pretend that two speaker words and a
  speaker table are the same shape with different numbers.
- **One type for a picture and a framebuffer.** Same three fields; a source
  and a target. Nothing converts one into the other and `Picture` has no
  methods, so folding them would remove three declarations and one compiler
  check — the one that stops a surface being blitted as a sprite
  (`motionvm-render`).
- **Accessors over `Game.vm` and `Game.engine`.** The two are public because
  the test suites are the inspection point. The guarantee accessors would
  hold — that nothing outside replaces an engine — is held by the crate
  graph already: the only crate depending on the engine is the front door,
  and what it hands a window is a `Box<dyn Playable>` (`game.rs`).
- **A `Cell` in place of the stray-read `RefCell`.** The 32-bit memory counts
  reads into modules that are not loaded behind a `&self` fetch. A `Cell`
  total with a fixed array of the first addresses would be one borrow that
  cannot fail and a few allocations saved on a cold path, at the cost of the
  tail of the report; and it is not what keeps the machine `Send` and not
  `Sync` — the music sink is `Send` alone (`m32/mod.rs`).
- **Handing a finished song back to the game thread.** The old song is
  dropped on the audio thread when a new one arrives. A second channel would
  avoid even that free, for a location change that happens every few
  minutes; more machinery than the problem is worth (`motionvm-motion/src/music.rs`).
- **A generation enum that selects behavior.** Ten of the thirteen
  capabilities are two-valued functions of "is this the 16-bit engine" today
  and could be folded into `Generation`. Each was measured separately, three
  already vary within a generation, and the next build is unknown; the
  section above says the rest.
- **The inspection CLI depending on the engine.** It could then say which
  kernel words the engine implements. It reads more games than the player
  plays, and the narrowing would be silent — both crates are the family's,
  so the boundary test would not object. The coverage question is answered
  by the engine's own suite instead (`motionvm-motion-tools/src/main.rs`).
- **`pedantic` as a group.** Two thirds of what it would say is a
  mechanical `#[must_use]` pass and an errors section the `Result` type
  already documents; the manifest carries the numbers, and the three of its
  lints the tree does hold to are on one at a time. The casts, which were
  most of the group, are named instead: every `as` is inside a function
  whose `#[expect]` says which rule it applies, and `as_conversions` is
  denied (`motionvm-motion-forth/src/cell.rs`).

## Extending the tree

The step-by-step procedures live in `CONTRIBUTING.md`, and what the contract
asks of a family, method by method, in `docs/writing-a-family.md`; this is
the shape they follow.

**A further game of a family the tree already plays** is a `Title` variant —
whose total matches then refuse to compile until every answer about the game
is given — plus a manifest module under `titles/`, a lookup function in the
family's testutil crate with its justfile and CI plumbing, its suites, and its
documentation tree under `docs/motion/games/<slug>/`. No engine code changes, which
is the point of the manifests.

**A further engine family** is a crate set of its own, named
`motionvm-<family>-*`, behind one `Family` implementation in its front door
`motionvm-<family>`. It implements `Playable` for its games, translates the
contract's `KeyPress` into whatever its scripts read, and answers `open_music`
or plays silently. The platform changes by one roster line and the two
manifest lines that let it compile (`[workspace.dependencies]` and the
window's own) — usage text, complaints and the window's behavior follow from
the trait — plus the
family's line in the boundary test's list, its own test rig and data plumbing,
and its own trees under `docs/`. The roster's fallback rule — a directory no family claims
is answered by the first family's refusal — is exact while one family is on
the list, and is the line to revisit with a second.

## Determinism is engineered

The engine reads no wall clock, the RNG is a seeded LCG, no `HashMap` sits on
any drawing path, and resource directories are walked sorted. A given input
state renders a given frame, every time. That is not a nicety: it is what lets
anyone with a copy of a game render a scene before and after a change and trust
the difference, which is this project's main verification method for the
drawing code.

## See also

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — the placement law, the quality gate,
  and how to add a game, an engine build, or an engine family
- [`docs/README.md`](docs/README.md) — the map of the original engines'
  documentation, one tree per family, written as a specification
- [`docs/motion/departures.md`](docs/motion/departures.md) — every place this code knowingly
  does something else, and why
