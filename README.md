# motionvm

A from-scratch reimplementation of **MOTION**, the DOS adventure engine
DigiTales (Stefan Hoffmann) shipped in 1996, so that the games built with it run
natively on current systems.

MOTION is not a game. It is an authoring system: a Forth compiler, an IDE, a
debugger and a runtime in one 16-bit-hosted, 32-bit-protected-mode binary. A
game made with it is *data* — compiled Forth script modules plus sprites,
palettes, fonts, texts and music inside a handful of resource containers.
motionvm reads that data and runs it: the same bytecode, the same 25 fps frame
cycle, the same 640×480×256 picture, the same OPL3 music, on a modern machine
with no emulator underneath.

The engine this was built from is version V0.06.06/R109, dated 1996-10-22, as
shipped with *"Im Netzwerk gefangen – Dunkle Schatten 2"*.

## What is here

| Crate | What it is |
|---|---|
| `motionvm-formats` | Readers for every shipped format: RSC containers, GFX8 sprites and the GFXCRUNCH LZW codec, palettes, fonts, text tables, script modules, HMI songs, Ad Lib banks, driver archives, LE binaries |
| `motionvm-forth` | The Forth virtual machine — threaded code, the two stacks, the address model, the 356-word kernel |
| `motionvm-render` | The indexed framebuffer and the screen compositing above it |
| `motionvm-audio` | The rebuilt FM driver, the HMI sequencer and OPL3 synthesis |
| `motionvm-engine` | The runtime the VM calls into: screens, descriptors, text, walking, saving, the game loop |
| `motionvm-tools` | `motionvm-tools`, a CLI for inspecting and extracting a game's resources |
| `motionvm-app` | `motionvm`, the window |
| `motionvm-testutil` | Where the test suites find the game's files. Development only; nothing ships with it |

**What is not here.** MOTION was two halves: the runtime that plays a game, and
the authoring side that makes one — an IDE, a Forth compiler and a debugger, all
described in [engine-exe.md](docs/engine/engine-exe.md). Only the runtime is
reimplemented. Reading a compiled module is in scope, and the disassembler in
`motionvm-tools` does it; writing one is not.

## Game data

**No game data is included, and none can be.** The resources are copyrighted
material that has to come from your own copy of the game. Point motionvm at the
directory the original was installed into and it will find what it needs.

If you would rather know exactly what that is — to carry a minimal copy, or to
check an incomplete one — here is the whole answer. Every line of it was
established by taking the file away and seeing what happened, not by reading the
loader.

### Required

Five files. Without any one of them motionvm stops at startup and says which.

| File | Size | What it holds |
|---|---|---|
| `001.RSC` | 3.8 MB | The 86 script modules — the game itself — plus 133 text tables, 250 blocks of music, 9 fonts, 60 palettes |
| `002.RSC` | 12.3 MB | 1619 sprites: nearly all the artwork |
| `003.RSC` | 2.1 MB | 57 more sprites |
| `ENGINE.EXE` | 845 KB | Not run, read: the 356-word kernel table is lifted out of the LE image, and the bytecode's ordinals mean nothing without it |
| `000.FRT` | 516 B | Character to glyph. Half a kilobyte, and without it the game draws every picture and not one word |

motionvm looks for `NNN.RSC` by pattern and merges whatever it finds, so a
MOTION game shipping a different number of containers works the same way.

### Required for sound

| File | Size | What it holds |
|---|---|---|
| `HMIMDRV.386` | 118 KB | The original OPL3 driver image, whose tables the rebuilt FM driver reads |
| `MELODIC.BNK` | 5.4 KB | The melodic instrument patches |
| `DRUM.BNK` | 5.4 KB | The percussion patches |

Missing, these are not fatal: motionvm prints `sound is off: …` and plays on in
silence.

### Everything else is ignored

The other 22 files of the installation are never opened. Copying them costs
nothing and leaving them out costs nothing:

| File(s) | Why it is not needed |
|---|---|
| `DOS4GW.EXE`, `_RUNVM.VMC`, `DS2.BAT`, `DS2.ICO` | The DOS extender, its memory configuration, the launcher and the icon. There is no DOS here |
| `SYSTEM.RSC` | The two-line Forth bootstrap (`4 =>GET` / `START`); motionvm carries the same sequence in code |
| `002.SCR`, `011.SCR` | Loose copies of modules 2 and 11, which also live inside `001.RSC` — that is where they are loaded from |
| `000.PAL` | A loose copy of a palette; palettes come out of the containers |
| `000.FNT` | The system font. It *is* read when present, but only as the fallback for text naming no font of its own — and every string this game draws names one, so the rendered picture is identical without it |
| `HMIDRV.386`, `HMIDET.386` | The digital-audio drivers and the card-detection stubs. The game's digital sound layer is never called |
| `SNDSETUP.EXE`, `SNDSETUP.INI`, `LOADPATS.EXE`, `PATCHES.INI` | The DOS sound-card setup program and its data |
| `TEST.HMI`, `TEST.MID`, `TEST.RAW`, `TEST.WAV` | Sound-card test samples that shipped with the driver kit |
| `RSC.INF` | Resource metadata, not analyzed — and not consulted |
| `LIESMICH.DOK`, `LIESMICH.TXT` | The German readme files. Worth reading, not by a program |

What those files actually *are* is documented in
[Other shipped files](docs/formats/other-files.md).

So a minimal copy is five files, or eight with sound:

```sh
mkdir motion-min
cp 001.RSC 002.RSC 003.RSC ENGINE.EXE 000.FRT \
   HMIMDRV.386 MELODIC.BNK DRUM.BNK  motion-min/
```

### The game directory is only ever read

motionvm never writes into it, and cannot be made to: the one place a writable
path enters the engine *refuses* any directory inside the game data — after
resolving both sides, so a relative path or a `..` cannot walk around it. A
read-only copy, a mounted image or a directory on a CD-ROM all work.

It does not write into the *source* tree either. Savegames and screenshots go
under the platform's data directory — `~/Library/Application Support/motionvm`
on macOS, `%APPDATA%\motionvm` on Windows, `$XDG_DATA_HOME/motionvm` or
`~/.local/share/motionvm` elsewhere — not into whatever directory the program
happened to be started from. Both paths are printed as they are used, and
`--saves` and `--shot` override them.

## Building and running

```sh
cargo build --release
./target/release/motionvm /path/to/gamedata
```

`motionvm` takes the game directory as its first argument and defaults to
`../gamedata`. Savegames go into `saves/` under the platform data directory
(above) unless `--saves DIR` says otherwise — either way the directory is
created on startup if it is not there and its path is printed, so a fresh clone
needs no setup. `--loc N` starts in a given location, `--shot PATH` moves the
screenshot, and `--no-sound` runs silent.

Requires a Rust toolchain of 1.97.0 or newer. That is not the oldest one that
would work — the workspace compiles on 1.95.0 — but it is recent, so a Rust
that came with your distribution may well be too old; `rustup` is the reliable
way to have one. Nothing else — no C compiler, no system libraries beyond what
`winit` and `cpal` need for a window and an audio device. On Linux that means
the X11/Wayland and ALSA development headers; `.github/workflows/ci.yml` names
the Debian packages.

**Where it has actually been played: macOS.** CI builds, lints and tests it on
Linux and Windows too, but without the game data — so those two are known to
compile and known to pass the data-free tests, and are otherwise untried. The
game directory is read case-insensitively, which is the one portability trap
that mattered: a copied install is often lower-cased, and on a case-sensitive
filesystem an exact-case lookup would find nothing.

## Looking inside the data

`motionvm-tools` reads the game's containers and writes what it finds as
ordinary files. Every command takes the game directory as its first argument;
`--help` prints the same summary.

**`info <gamedata>`** — one line per resource bank: how many sprites, texts,
songs, fonts, palettes and script modules it holds, and how much of the file
is not accounted for. The quickest way to tell a complete installation from a
partial one.

```sh
motionvm-tools info /path/to/gamedata
```

**`extract <gamedata> [--out DIR]`** — writes everything out at once, each
resource both as the raw bytes it is stored as and as something you can look
at: sprites as indexed PNGs, palettes as `.pal` plus a swatch grid, fonts as
`.fnt` plus a contact sheet and a JSON of their glyph metrics, text tables as
JSON, songs as `.hmi`, and script modules as `.scr` plus a `.f` disassembly
with kernel words resolved by name. It also writes `kernel-usage.txt`, which
lists the kernel words the game's own code reaches for and marks which of them
this engine implements. `--out` defaults to `out`.

```sh
motionvm-tools extract /path/to/gamedata --out out
```

**`sprite <gamedata> <id> [--out FILE]`** — one sprite as an indexed PNG,
palette index 0 written as transparent. `--out` defaults to
`sprite-NNNN.png`.

```sh
motionvm-tools sprite /path/to/gamedata 1010
```

**`script <gamedata> <id>`** — one script module on stdout: its header, its
symbol table, and its threaded code disassembled with kernel words resolved by
name. Reading a compiled module is in scope for this project; writing one is
not.

```sh
motionvm-tools script /path/to/gamedata 323 | less
```

## Controls

The game is played with the mouse; the keys are the original's, delivered into
`ICTRL`'s key variable exactly as the DOS engine's BIOS reads did — a character
where the key has one and a scan code where it has none, which is what lets the
mailbox be worked with the cursor keys the way its manual describes.

| | |
|---|---|
| Left click | Walk, use, or pick the thing under the pointer |
| Right click | Open the verb menu on it |
| Escape | The in-game menu — save, load, options, quit |
| Cursor keys | Move through the in-game mailbox's menus and lists; Return or Space takes what is highlighted |
| Return, Space, Backspace, letters | Passed through to the game, which uses them on its own pages |
| F12 | Freeze the picture **and** write it out as an indexed PNG — to `shot.png` in the data directory, or wherever `--shot` says; the path is printed |
| Close the window | Quit |

Escape opens the menu rather than quitting, because that is what the original
does. There is deliberately no quit key beyond the window: the game has its own
quit, and adding a second one would put a way out of the game that the game does
not know about.

F12 does two things on one key, which is a debugging convenience rather than a
design: a still picture is the one you can compare against a reference
screenshot, so freezing and capturing belong together.

## Tests

```sh
just check          # format, lints, tests and documentation
just test           # only the tests, with the game's files
```

or without [`just`](https://github.com/casey/just):

```sh
MOTIONVM_GAMEDATA=../gamedata cargo test --workspace
```

The tests hold the implementation against the original's own files: the decoders
against every resource in the containers, the FM driver against the bytes of
`HMIMDRV.386` itself, the renderer against extracted artwork, the engine against
the behavior of the shipped script modules. They need a game directory, which
they look for at `../gamedata` or wherever `MOTIONVM_GAMEDATA` points; without
one, every test that needs data **skips itself** rather than failing, so a
checkout tests cleanly on a machine that has no copy of the game.

Setting `MOTIONVM_GAMEDATA` to a directory that does not hold `001.RSC` is the
one case that is **not** a skip: it panics and says so. A mistyped path would
otherwise read as "this machine has no game data", and a run that skips
everything looks exactly like a run that passes everything.

`tests/scenes.rs` drives the engine to four standing pictures — the title, a
page of intro text, a conversation's answer menu, a page of the help viewer —
and asserts it gets there and draws something. Reaching them is most of the
check: the answer menu needs startup, the location loader, the task machine and
the dialogue apparatus all working at once.

It does **not** compare the picture. It used to, against checked-in copies, and
that was the one test that caught a change nobody had thought to assert. Those
copies were renderings of the game's own artwork and could not stay in a public
repository, so the check went rather than the rule. Rendering is deterministic
(see CONTRIBUTING), so anyone with the game can still do the comparison locally
across a change — it just cannot ship here.

The suite reaches wider than the game does. Beyond the eight files above it also
reads `TEST.HMI` and `TEST.MID` (a song decoded two ways and cross-checked),
`HMIDRV.386` and `HMIDET.386` (every driver archive is walked to its last byte),
and `000.PAL`, `002.SCR`, `011.SCR` (the loose copies are compared against their
twins inside the containers) — sixteen files in all. Checking those is the
point: they are evidence about the formats even where the game never touches
them. Run the tests against a complete installation.

One test wants a savegame, which no checkout carries. Point `MOTIONVM_SAVES` at
a directory holding one to include it; otherwise it skips. It is the only test
that skips on a machine that has the game's files.

### What is and is not verified against the original

The distinction matters and is easy to blur, so it is written down rather than
implied. **Verified against the original engine's own output:**

- **One rendered frame**, pixel for pixel — the title screen, 307 200 pixels
  over 206 palette indices, held against the original running under DOSBox-X.
  Every index maps to one color and every color back to one index, which is a
  stronger check than comparing RGB.
- **Fourteen VM primitives**, against modules the 1996 compiler produced.
- **A song's first 8 000 OPL register writes**, and one mixer envelope.

Those comparisons need recordings of the original engine — a screen capture, a
register dump, a mixer capture. Recordings of the game are no more
redistributable than the game, so none ships here and the checks that consume
them are not part of this suite. What ships is their result, stated above.

Everything else is verified against the original's *files* — its resources, its
bytecode, its driver binary — which is a different and weaker thing: it says the
readers agree with the data, not that the engine behaves as the engine did.
Interaction, dialogue, walking, savegames, the verb menu and every location but
the title have never been differentially compared. That is not a gap being
hidden; it is the honest edge of what a reimplementation without the original
running beside it can claim.

## Documentation

[`docs/README.md`](docs/README.md) is the hub for the full technical
documentation — 42 pages covering every file format, the virtual machine, the
engine's subsystems and the script library, written to stand on its own as a
specification of MOTION.

## Licensing

**MIT** — see [LICENSE](LICENSE).

One dependency has a license of its own: **`nuked-opl3` is
`LGPL-2.1-or-later`**, as every derivative of Nuked-OPL3 is. Linked statically
into a binary, that means anyone handed the binary must be able to relink it —
the object files or the sources have to be on offer. Distributed as source, as
this is, that condition is already met; ship a pre-built binary and it becomes
yours to meet. [NOTICE](NOTICE) says so in full, and
[`docs/engine/audio.md`](docs/engine/audio.md) says why that core was chosen
anyway.

**The game's own files are not covered by any of this.** They are not part of
the project and are not redistributable; motionvm reads an installation you
already have.
