# Contributing to motionvm

motionvm is a from-scratch Rust reimplementation of MOTION, the DOS adventure
engine that ships with *Im Netzwerk gefangen — Dunkle Schatten 2*. It runs the
original's compiled Forth bytecode natively, and its single hard constraint
shapes every convention in this document: **the original `ENGINE.EXE`
(V0.06.06/R109, 1996-10-22) is the authority on what the engine does.** Code
here is not merely correct or incorrect; it is faithful or unfaithful, and
fidelity is established by evidence, not by plausibility.

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
compiled — which is what a 1.98.0 four days after 1.98.0's release did.

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

**Game data.** The game's files are not in the repository and cannot be; they
are copyrighted. The `justfile` reads two locations:

- `GAMEDATA` — a copy of the shipped game directory (`001.RSC` and friends).
  Defaults to a sibling directory of this checkout, and that default is only
  passed on when it is really there — so `just check` is green on a machine
  with no copy of the game, with everything data-dependent skipping. A path
  you name yourself is always passed on, so a typo panics instead of quietly
  skipping the suite.
- `SAVES` — a directory holding a savegame, for the savegame tests. Savegames
  cannot be reconstructed, only played to, so there is no default that could
  work; without one those tests skip.

Tests that need data they cannot find skip themselves and say so. A *set but
wrong* path panics instead: a run that skips everything is indistinguishable
from a run that passes everything, so a mistyped path would otherwise report
green without executing a line.

**Running.** `just run`. This builds in release mode, and that is not
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

`.github/workflows/ci.yml` runs the same four commands on Linux, macOS and
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
  deliberate choices it would argue with.

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
  "the jump table at `0x98967`". Do **not** extract these addresses into
  named constants — a literal sitting next to its citation is the house
  style, because the address *is* the evidence and belongs where the claim
  is made.
- **Name the evidence class.** A claim is read from the disassembly, measured
  against the running original under DOSBox-X, or a hypothesis — and the
  reader must be able to tell which. Never let a guess wear the clothes of a
  fact.
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
  document: unknowns about the original go to `docs/open-questions.md`,
  where they are tracked rather than scattered.

**The rustdoc gate.** `just doc` treats broken intra-doc links as errors and
documents private items, because in this project the private comments carry
as much of the evidence as the public ones — the `ENGINE.EXE` citations sit
wherever the code does, and a link checker that covered half of them would
only read as if it covered them.

## Architecture

The workspace is layered, and dependencies point strictly downward:

| Crate | Role |
|---|---|
| `motionvm-formats` | Readers for the shipped file formats; std-only, zero dependencies |
| `motionvm-forth` | The Forth VM: address model and kernel words |
| `motionvm-render` | Framebuffer and compositing |
| `motionvm-audio` | FM driver, HMI sequencer, OPL3 |
| `motionvm-engine` | The runtime that ties them into the game loop |
| `motionvm-app` | The window: winit/softbuffer frontend |
| `motionvm-tools` | CLI for reading and extracting the shipped formats |
| `motionvm-testutil` | Test-data location, shared by every suite |

Principles the layout encodes:

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

## Fidelity and accepted divergences

Behavior is matched to `ENGINE.EXE` V0.06.06/R109. When motionvm and the
original disagree, motionvm is wrong — unless the divergence is on the list
below, which exists precisely so that nobody "fixes" a deliberate decision:

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

Adding to this list is a deliberate act: document the divergence where the
code lives, state why matching the original is impossible or undesirable, and
record it here.

## Testing

- **A test name is a sentence** stating the fact it proves:
  `the_title_screen_matches_the_original_engine`,
  `escape_opens_the_quit_page`. If the fact will not fit in a name, the
  test's doc comment carries it.
- **Every test file documents what it verifies and against which evidence** —
  the same evidence discipline as the code.
- **Fixtures come from your own game install**, located through
  `motionvm-testutil`, never checked in. Remember the rule from setup:
  missing data skips loudly, a wrong path panics.
- **There are no golden frames, and their absence is a real loss.** This
  project used to pin four rendered scenes against checked-in indexed PNGs —
  the one kind of check that catches a change nobody thought to assert. Those
  baselines were renderings of the game's own artwork, which the rule above
  forbids, so they were removed rather than shipped. What is left is
  `tests/scenes.rs`: it still drives the engine to those four scenes and
  asserts it arrives and draws something, which covers the navigation but not
  the picture. If you are changing the drawing path, render the scenes before
  and after on your own machine and compare them there — the determinism note
  above is what makes that trustworthy.
- **Measure before optimizing.** `MOTIONVM_PERF=1` and the `timing` rig in
  `motionvm-engine` exist so that performance claims are measurements.
  Plausible arguments about frame cost are wrong in both directions: the
  obvious candidate turns out to cost nothing measurable, and the real hot
  spot sits in a routine nobody suspected.

## Documentation pages

The reverse-engineering knowledge lives in `docs/` — format specifications,
the VM's execution model, the engine's subsystems, and a page per script
module. When code changes what is known, the docs move with it in the same
change. Conventions:

- Every page opens with a breadcrumb back to the documentation index and ends
  with a **See also** link list. A page carries an **Open questions** section
  when it has open questions; empty boilerplate sections are noise, not
  honesty.
- The index (`docs/README.md`) fixes the shared vocabulary once: values are
  little-endian, text is CP437, a cell is 32 bits, stack effects use Forth's
  `( a b -- c )` notation, and engine addresses refer to `ENGINE.EXE`'s
  *relocated* image.
- Unresolved questions about the original belong in
  `docs/open-questions.md`, the single ledger of what is not yet known.
- Deliberate differences from the original belong in `docs/departures.md`,
  the single ledger of where the code knowingly does something else. The
  subsystem pages describe the original engine; a departure is recorded there
  and pointed at from the page it concerns.
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
  hypothesis being made, and update `docs/`, `docs/open-questions.md` and
  `docs/departures.md` accordingly.

## License

motionvm is MIT-licensed; contributions are accepted under the same license.
`NOTICE` records third-party obligations. The game's data files are
copyrighted and must never enter the repository — that includes captures,
screenshots, and any fixture derived from them.
