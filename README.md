# motionvm

A from-scratch reimplementation of **MOTION**, the DOS adventure engine
DigiTales (Stefan Hoffmann) shipped in 1996, so that the games built with it run
natively on current systems.

MOTION is not a game. It is an authoring system: a Forth compiler and a
runtime — in its later form also an IDE and a debugger — in one binary. A
game made with it is *data* — compiled Forth script modules plus sprites,
palettes, fonts, texts and music inside resource containers. motionvm reads
that data and runs it: the same bytecode, the same frame cycle, the same
picture, the same music, on a modern machine with no emulator underneath.

## Two generations, two games

Two games were built with MOTION, two months apart, on two generations of the
engine:

| Game | Engine | In motionvm |
|---|---|---|
| *Im Netzwerk gefangen – Dunkle Schatten 2* (**DS2**, 1996) | **MOTION 32-bit** — `ENGINE.EXE` V0.06.06/R109, dated 1996-10-22: a 32-bit protected-mode binary with the IDE, compiler and debugger still inside; `NNN.RSC` containers, 640×480×256, HMI music at 25 fps | **Runs**, end to end. Every reader, the VM's address model and the runtime in this tree are this engine's |
| *Die Enviro-Kids greifen ein* (**ENVIRO**, 1996) | **MOTION 16-bit** — `ENVIRO.EXE`, dated 1996-08-27: a 16-bit real-mode player with no compiler; one `DATA.-1-` container, 320×200×256, PSM 2 music | **Plays.** The container, the 16-bit machine and the engine's words carry `RUN` through the DigiTales logo and the briefing into the scrapyard and on through every location: the walk, the inventory bar, the verb menu, the hover caption and the conversations run as read from `ENVIRO.EXE`. Saves go through the game's own page into three files of motionvm's own layout, and the PSM 2 tunes play through a rebuild of the game's own Ad Lib driver, held register for register against an OPL capture of the original. `docs/motion16/` and `docs/games/enviro/` are its specification |

The two share the Forth dialect, the compiler's output conventions and most
of the kernel's vocabulary; they do not share the machine — cell width,
kernel ordinals, address model, container, asset encodings and music format
all differ. [`docs/README.md`](docs/README.md) lays the two side by side.
Throughout this repository a statement about one game or one generation
says so; a statement that names neither holds for both.

## What is here

| Crate | What it is |
|---|---|
| `motionvm-formats` | Readers for every shipped format of both generations: the 32-bit RSC containers, GFX8 sprites and the GFXCRUNCH LZW codec, HMI songs, Ad Lib banks, driver archives and LE binaries; the 16-bit `DATA.-1-` container, raw sprites and fonts, the MZ binary; and for both, palettes, fonts, text tables, script modules and the kernel tables |
| `motionvm-forth` | The Forth virtual machines — threaded code, the two stacks, the kernel dispatch; the 32-bit machine with its packed address model, the 16-bit machine with its flat space and word-id table |
| `motionvm-render` | The indexed framebuffer and the screen compositing above it |
| `motionvm-audio` | The rebuilt FM driver, the HMI sequencer and OPL3 synthesis; the 16-bit game's PSM 2 sequencer and player, rebuilt from `MUSADL.DRV` |
| `motionvm-engine` | The runtime the VM calls into: screens, descriptors, text, walking, saving, the game loop |
| `motionvm-tools` | `motionvm-tools`, a CLI for inspecting and extracting a game's resources |
| `motionvm-app` | `motionvm`, the window |
| `motionvm-testutil` | Where the test suites find the game's files. Development only; nothing ships with it |

`motionvm-formats` and `motionvm-forth` carry both engine generations, as
`m32` and `m16`; the engine is generic over the machine it drives, with the
behaviors that differ named per generation, and the renderer, the audio and
the window serve both games.

**What is not here.** MOTION was two halves: the runtime that plays a game, and
the authoring side that makes one — an IDE, a Forth compiler and a debugger, all
described in [engine-exe.md](docs/motion32/engine/engine-exe.md). Only the runtime is
reimplemented. Reading a compiled module is in scope, and the disassembler in
`motionvm-tools` does it; writing one is not.

## Game data

**No game data is included, and none can be.** The resources are copyrighted
material that has to come from your own copy of the game. Point motionvm at the
directory the original was installed into and it will find what it needs.

### Dunkle Schatten 2

If you would rather know exactly what that is — to carry a minimal copy, or to
check an incomplete one — here is the whole answer for Dunkle Schatten 2. Every
line of it was established by taking the file away and seeing what happened,
not by reading the loader.

#### Required

Five files. Without any one of them motionvm stops at startup and says which.

| File | Size | What it holds |
|---|---|---|
| `001.RSC` | 3.8 MB | The 86 script modules — the game itself — plus 133 text tables, 250 data blocks (26 HMI songs, the location table, walking routes, click areas, item records, 121 dialogue definitions), 9 fonts, 60 palettes |
| `002.RSC` | 12.3 MB | 1619 sprites: nearly all the artwork |
| `003.RSC` | 2.1 MB | 57 more sprites |
| `ENGINE.EXE` | 845 KB | Not run, read: the 356-word kernel table is lifted out of the LE image, and the bytecode's ordinals mean nothing without it |
| `000.FRT` | 516 B | Character to glyph. Half a kilobyte, and without it the game draws every picture and not one word |

motionvm looks for `NNN.RSC` by pattern and merges whatever it finds, so a
MOTION game shipping a different number of containers works the same way.

#### Required for sound

| File | Size | What it holds |
|---|---|---|
| `HMIMDRV.386` | 118 KB | The original OPL3 driver image, whose tables the rebuilt FM driver reads |
| `MELODIC.BNK` | 5.4 KB | The melodic instrument patches |
| `DRUM.BNK` | 5.4 KB | The percussion patches |

Missing, these are not fatal: motionvm prints `sound is off: …` and plays on in
silence.

#### Everything else is ignored

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
[Other shipped files](docs/games/ds2/other-files.md).

So a minimal copy is five files, or eight with sound:

```sh
mkdir motion-min
cp 001.RSC 002.RSC 003.RSC ENGINE.EXE 000.FRT \
   HMIMDRV.386 MELODIC.BNK DRUM.BNK  motion-min/
```

### Die Enviro-Kids greifen ein

Two files, and motionvm stops at startup and says which is missing:

| File | Size | What it holds |
|---|---|---|
| `DATA.-1-` | 7.6 MB | The whole game: 65 script modules, 1586 sprites, 96 text tables, 130 blocks, 23 palettes, 3 fonts, the font reference table |
| `ENVIRO.EXE` | 167 KB | Not run, read: the 233-word kernel table is lifted out of the MZ image; its handlers are what the engine words follow |

`MUSADL.DRV` (5 KB) is the music: the PSM 2 Ad Lib driver, whose tables the
player reads out of the shipped file — without it the game runs silent.
The other eleven files of the installation — `KIDS.BAT` and `SOUND.EXE`,
the five other `.DRV` files, `README.TXT`, and three files nothing
references — are never opened; what they are is in
[Other shipped files](docs/games/enviro/other-files.md).

So a minimal copy is two files, or three with music:

```sh
mkdir enviro-min
cp DATA.-1- ENVIRO.EXE MUSADL.DRV  enviro-min/
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
happened to be started from. Both paths are printed as they are used.

## Downloads

Every release on the [Releases page](https://github.com/wdominik/motionvm/releases)
carries two zips, built from the tag by `.github/workflows/release.yml`:

| File | What it is |
|---|---|
| `motionvm-x.y.z-windows-x86_64.zip` | One `motionvm.exe`. The C runtime is linked in and everything else it uses ships with Windows 10 and later, so there is nothing to install beside it. SmartScreen says "Windows protected your PC" the first time — **More info → Run anyway** |
| `motionvm-x.y.z-macos-universal.zip` | `motionvm.app`, one binary for Apple Silicon and Intel. macOS refuses the first start of an app it has not seen before — **System Settings → Privacy & Security → Open Anyway** lets it through, once; `xattr -dr com.apple.quarantine motionvm.app` in a terminal does the same |

Unzip, start, point the folder dialog at your copy of the game. `LICENSE`,
`NOTICE` and this README are in each zip. Linux builds from source, below.

The release also carries the source archive of the tag the binaries were built
from, and that is not decoration: the OPL3 core in them is LGPL-licensed, and
`NOTICE` explains that whoever hands out a binary has to hand out the means to
relink it — the archive is that means. See [Licensing](#licensing).

## Building and running

```sh
cargo build --release
./target/release/motionvm /path/to/gamedata
```

`motionvm` takes the game directory as its first argument and tells the game
by the files in it. Without one it asks: the platform's own folder dialog
opens, and a directory that is not a MOTION game is reported in a message box
and asked for again — so the binary can be double-clicked. Savegames go into `saves/` under the platform data directory
(above) for Dunkle Schatten 2 and `saves/enviro/` for Die Enviro-Kids greifen
ein — the two games name their slots alike and each looks for them at
start-up; the directory is created on startup if it is not there and its path
is printed, so a fresh clone needs no setup. `--loc N` starts in a given
location — instead of the intro for Dunkle Schatten 2, right after it for
Die Enviro-Kids greifen ein, whose `RUN` enters location 1 itself — and
`--no-sound` runs silent.

The window shows the picture the way the game's own monitor did, and only
ever scaled by whole numbers — one per axis. Dunkle Schatten 2's 640×480 is
square-pixel 4:3 and opens at twice its size; Die Enviro-Kids greifen ein's
320×200 filled a 4:3 screen with pixels 6/5 as tall as wide, so each of its
pixels becomes an sx×sy block with sy/sx as close to 6/5 as whole numbers
come — exact at 5×6 and its multiples, and the window opens on the largest
exact step the screen has room for (1600×1200 where it fits, 960×800 at 3×4
below that). Drag the window bigger and the picture steps up to the next
whole pair that fits, with black around it. Alt+Enter (Option+Return on
macOS) takes the whole screen the same way — a borderless window at the
largest pair that fits, no change of display mode — and again to come back.

Requires a Rust toolchain of 1.97.0 or newer. That is not the oldest one that
would work — the workspace compiles on 1.95.0 — but it is recent, so a Rust
that came with your distribution may well be too old; `rustup` is the reliable
way to have one. Nothing else — no C compiler, no system libraries beyond what
`winit` and `cpal` need for a window and an audio device. On Linux that means
the X11/Wayland and ALSA development headers; `.github/workflows/ci.yml` names
the Debian packages. The folder dialog adds nothing to that list: on Linux it
goes through the XDG desktop portal, which costs nothing to build against and
at run time needs the `xdg-desktop-portal` service GNOME and KDE desktops
carry, or `zenity` as the fallback — and without either, the directory goes on
the command line.

**Where it has actually been played: macOS.** CI builds, lints and tests it on
Linux and Windows too, but without the game data — so those two are known to
compile and known to pass the data-free tests, and are otherwise untried. The
game directory is read case-insensitively, which is the one portability trap
that mattered: a copied install is often lower-cased, and on a case-sensitive
filesystem an exact-case lookup would find nothing.

## Looking inside the data

`motionvm-tools` reads a game's containers and writes what it finds as
ordinary files — the 32-bit `NNN.RSC` banks of Dunkle Schatten 2 or the 16-bit
`DATA.-1-` of Die Enviro-Kids greifen ein, told apart by the files in the
directory. Every command takes the game directory as its first argument;
`--help` prints the same summary. The ids in the examples are Dunkle
Schatten 2's. The release archives carry only `motionvm` — the tool is run
from a checkout:

```sh
cargo run --release -p motionvm-tools -- info /path/to/gamedata
```

The examples below abbreviate that invocation to `motionvm-tools`.

**`info <gamedata>`** — one line per resource bank: how many sprites, texts,
songs, fonts, palettes and script modules it holds, and how much of the file
is not accounted for. The quickest way to tell a complete installation from a
partial one.

```sh
motionvm-tools info /path/to/gamedata
```

**`extract <gamedata> [--out DIR] [--pal N]`** — writes everything out at
once, each resource both as the raw bytes it is stored as and as something you
can look at: sprites as indexed PNGs, palettes as `.pal` plus a swatch grid,
fonts as `.fnt` plus a contact sheet and a JSON of their glyph metrics, text
tables as JSON, and script modules as `.scr`. For a 32-bit game it also writes
the songs as `.hmi`, a `.f` disassembly of every module with kernel words
resolved by name, and `kernel-usage.txt`, which lists the kernel words the
game's own code reaches for and marks which of them this engine implements.
For a 16-bit game the blocks come out as `.blk` with an index that marks the
PSM 2 songs, the modules additionally as `scripts/modules.txt` (every
module's symbol table) and as `.f` listings read through `ENVIRO.EXE`'s
kernel table, with its own `kernel-usage.txt`, and the sprites — which carry
no palette of their own — through palette `--pal N`, palette 0 by default.
`--out` defaults to `out`.

```sh
motionvm-tools extract /path/to/gamedata --out out
```

**`sprite <gamedata> <id> [--out FILE] [--pal N]`** — one sprite as an
indexed PNG, palette index 0 written as transparent; `--pal` picks the palette
for a 16-bit sprite. `--out` defaults to `sprite-NNNN.png`.

```sh
motionvm-tools sprite /path/to/gamedata 1010
```

**`script <gamedata> <id>`** — one script module on stdout: its header, its
symbol table, and its threaded code disassembled with kernel words resolved by
name — through `ENGINE.EXE`'s table for a 32-bit module, `ENVIRO.EXE`'s for a
16-bit one. Reading a compiled module is in scope for this project; writing
one is not.

```sh
motionvm-tools script /path/to/gamedata 323 | less
```

## Controls

The game is played with the mouse; the keys are the original's, delivered into
`ICTRL`'s key variable exactly as the DOS engine's BIOS reads did — a character
where the key has one and a scan code where it has none, which is what lets
Dunkle Schatten 2's mailbox be worked with the cursor keys the way its manual
describes.

| | |
|---|---|
| Left click | Walk, use, or pick the thing under the pointer |
| Right click | Open the verb menu on it |
| Escape | Dunkle Schatten 2's in-game menu — save, load, options, quit. In Die Enviro-Kids greifen ein it skips the intro; that game's menu is the icon at the bar's right end |
| Cursor keys | Move through Dunkle Schatten 2's in-game mailbox; Return or Space takes what is highlighted |
| Return, Space, Backspace, letters | Passed through to the game, which uses them on its own pages |
| F12 | Freeze the picture **and** write it out as an indexed PNG — to `shot.png` in the data directory; the path is printed |
| Alt+Enter | Borderless fullscreen, on and off (Option+Return on macOS). The picture keeps whole-number scales — the largest pair that fits, in the game's own pixel shape — with black around it |
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
just test           # only the tests, with the games' files
```

or without [`just`](https://github.com/casey/just):

```sh
cargo test --workspace
```

which finds the games at `../games/DS2` and `../games/ENVIRO` beside the
checkout; `MOTIONVM_GAMEDATA_DS2` and `MOTIONVM_GAMEDATA_ENVIRO` point
anywhere else.

The tests hold the implementation against the originals' own files: the
decoders against every resource in the containers of both games, the FM driver
against the bytes of `HMIMDRV.386` and the PSM player against `MUSADL.DRV`'s
tables, the renderer against extracted artwork, the engine against the
behavior of both games' script modules — Dunkle Schatten 2's scenes, and Die
Enviro-Kids greifen ein's boot, locations, conversations, savegames and text
rendering.
They need the game directories — `MOTIONVM_GAMEDATA_DS2` for Dunkle Schatten 2
(`001.RSC` and friends, looked for at `../games/DS2` when the variable is not
set) and `MOTIONVM_GAMEDATA_ENVIRO` for Die Enviro-Kids greifen ein
(`DATA.-1-` and `ENVIRO.EXE`, `../games/ENVIRO`); without one, every test
that needs that game's data **skips itself** rather than failing, so a
checkout tests cleanly on a machine that has no copy of either game, and a
machine with one game runs that game's tests. Every test says which game it
drives.

Setting either variable to a directory that does not hold its game — no
`001.RSC`, no `DATA.-1-` — is the one case that is **not** a skip: it panics
and says so. A mistyped path would otherwise read as "this machine has no game
data", and a run that skips everything looks exactly like a run that passes
everything.

`tests/scenes.rs` drives the engine to four standing pictures of Dunkle
Schatten 2 — the title, a page of intro text, a conversation's answer menu, a
page of the help viewer — and asserts it gets there and draws something. Reaching them is most of the
check: the answer menu needs startup, the location loader, the task machine and
the dialogue apparatus all working at once.

It does **not** compare the picture. A checked-in reference frame would be a
rendering of the game's own artwork, which cannot sit in a public repository —
so the pixels stay out and the shape assertions stand in. Rendering is
deterministic (see CONTRIBUTING), so anyone with the game can still hold two
builds' frames against each other locally; the comparison just cannot ship
here.

The suite reaches wider than the game does. Beyond the files above it also
reads `TEST.HMI` and `TEST.MID` (a song decoded two ways and cross-checked),
`HMIDRV.386` and `HMIDET.386` (every driver archive is walked to its last
byte), and the loose `000.PAL` and `011.SCR` (the palette for its size and
range, the module for its number and its words). Checking those is the point:
they are evidence about the formats even where the game never touches them.
Run the tests against a complete installation.

One test wants a savegame, which no checkout carries. Point `MOTIONVM_SAVES` at
a directory holding one to include it; otherwise it skips. It is the only test
that skips on a machine that has the game's files.

### What is and is not verified against the original

The distinction matters and is easy to blur, so it is written down rather than
implied. **Verified against the original engine's own output**, for Dunkle
Schatten 2 on the 32-bit engine:

- **One rendered frame**, pixel for pixel — Dunkle Schatten 2's title screen,
  307 200 pixels over 206 palette indices, held against the original running
  under DOSBox-X.
  Every index maps to one color and every color back to one index, which is a
  stronger check than comparing RGB.
- **Fourteen VM primitives**, against modules the 1996 compiler produced.
- **A song's first 8 000 OPL register writes**, and one mixer envelope.

And for Die Enviro-Kids greifen ein on the 16-bit engine, against lossless
recordings of the original under DOSBox-X:

- **The intro's title scene**, 99.9 % of sampled pixels identical to a
  frame-rate video capture.
- **A page of the help viewer**, RGB-identical to the pixel — 0 of 51 200
  differ — reached through the game's own menu.
- **The music's whole register stream**: 2 738 OPL writes across two tunes
  and the room change between them, identical to the capture's end.
- **The box fades**, ring for ring — same widths on all four sides, same
  cadence — and the text drawer's outline against a played capture.

Those comparisons need recordings of the original engine — a screen capture, a
register dump, a mixer capture. Recordings of the game are no more
redistributable than the game, so none ships here and the checks that consume
them are not part of this suite. What ships is their result, stated above.

Everything else is verified against the original's *files* — its resources, its
bytecode, its driver binary — which is a different and weaker thing: it says the
readers agree with the data, not that the engine behaves as the engine did.
Interaction, dialogue, walking, savegames, the verb menu and most locations
of either game have never been differentially compared. That is not a gap being
hidden; it is the honest edge of what a reimplementation without the original
running beside it can claim.

## Documentation

[`docs/README.md`](docs/README.md) is the hub for the full technical
documentation — every file format, the virtual machine and the engine's
subsystems for both generations of the engine, a page per script module of
Dunkle Schatten 2 and the structure of both games — written to stand on its
own as a specification of MOTION.

## Licensing

**MIT** — see [LICENSE](LICENSE).

One dependency has a license of its own: **`nuked-opl3` is
`LGPL-2.1-or-later`**, as every derivative of Nuked-OPL3 is. Linked statically
into a binary, that means anyone handed the binary must be able to relink it —
the object files or the sources have to be on offer. Distributed as source, as
this is, that condition is already met; ship a pre-built binary and it becomes
yours to meet. [NOTICE](NOTICE) says so in full, and
[`docs/motion32/engine/audio.md`](docs/motion32/engine/audio.md) says why that core was chosen
anyway.

**The games' own files are not covered by any of this.** They are not part of
the project and are not redistributable; motionvm reads an installation you
already have.
