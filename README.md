# motionvm

Play German MS-DOS point-and-click adventures of the mid-nineties natively on
today's Windows, macOS and Linux. No DOSBox, no emulator: motionvm is a
from-scratch Rust reimplementation of **MOTION**, the adventure engine those
games were built with. You bring the files of your own copy of a game;
motionvm finds them and plays it — picture, music, savegames and all.

The games it plays today are **Im Netzwerk gefangen – Dunkle Schatten 2**,
**Die Enviro-Kids greifen ein** and **Jeff Jet - Abenteuer InfoHighway**.
MOTION made more than those, and that list is a record of what has been
done, not a limit of the engine underneath it.

> *motionvm spielt DOS-Adventures der neunziger Jahre nativ auf heutigen
> Rechnern — ohne DOSBox, ohne Emulator. Benötigt werden nur die Dateien
> einer eigenen Spielkopie. Zurzeit laufen „Im Netzwerk gefangen – Dunkle
> Schatten 2“, „Die Enviro-Kids greifen ein“ und „Jeff Jet - Abenteuer
> InfoHighway“.*

MOTION was written by DigiTales (Stefan Hoffmann), and the games made with it
were German advergames and edutainment titles — commissioned work, given away
rather than sold. Of the three here, two were productions of the **Art
Department Werbeagentur GmbH**, each commissioned by a German public authority
— Dunkle Schatten 2 by the Bundesministerium des Innern (the Federal Ministry
of the Interior), Die Enviro-Kids greifen ein by the Ministerium für Umwelt,
Raumordnung und Landwirtschaft des Landes Nordrhein-Westfalen (North
Rhine-Westphalia's environment ministry); Jeff Jet was made by the
**Promotion Software GmbH** in Tübingen for the **Hewlett Packard GmbH**.

The engine is not a game: it is an authoring system — a Forth compiler and a
runtime, in its later form also an IDE and a debugger, in one binary. A game
made with it is *data*: compiled Forth script modules plus sprites, palettes,
fonts, texts and music inside resource containers. motionvm reads that data
and runs it — the same bytecode, the same frame cycle, the same picture, the
same music, with no emulator underneath.

## The games it plays

Three so far, across the two generations of the engine:

| Game | Engine | In motionvm |
|---|---|---|
| *Im Netzwerk gefangen – Dunkle Schatten 2* (**DS2**, 1996) | **MOTION 32-bit** — `ENGINE.EXE` V0.06.06/R109, dated 1996-10-22: a 32-bit protected-mode binary with the IDE, compiler and debugger still inside; `NNN.RSC` containers, 640×480×256, HMI music at 25 fps | **Runs**, end to end. Every reader, the VM's address model and the runtime in this tree are this engine's |
| *Die Enviro-Kids greifen ein* (**ENVIRO**, 1996) | **MOTION 16-bit** — `ENVIRO.EXE`, dated 1996-08-27: a 16-bit real-mode player with no compiler; one `DATA.-1-` container, 320×200×256, PSM 2 music | **Plays.** The container, the 16-bit machine and the engine's words carry `RUN` through the DigiTales logo and the briefing into the scrapyard and on through every location: the walk, the inventory bar, the verb menu, the hover caption and the conversations run as read from `ENVIRO.EXE`. Saves go through the game's own page into three files of motionvm's own layout, and the PSM 2 tunes play through a rebuild of the game's own Ad Lib driver, held register for register against an OPL capture of the original. `docs/motion16/` and `docs/games/enviro/` are its specification |
| *Jeff Jet - Abenteuer InfoHighway* (**JEFFJET**) | **MOTION 16-bit** — `HPPLAY.EXE`, an older build of the same player, five kernel words fewer; two `DATA.-n-` volumes with every item LZW-packed, 320×200×256, PSM 2 music | **Plays.** The same machine, the same words and the same save scheme as Die Enviro-Kids greifen ein — what is this game's own is the container: two volumes, and 2.5 MB of packed items unfolding to 8.4 MB. `RUN` carries it through the intro into Jeff's room and on through all thirteen locations. `docs/motion16/` and `docs/games/jeffjet/` are its specification |

The two generations share the Forth dialect, the compiler's output
conventions and most of the kernel's vocabulary; they do not share the
machine — cell width, kernel ordinals, address model, container, asset
encodings and music format all differ.
[`docs/README.md`](docs/README.md) lays the two side by side. Throughout this
repository a statement about one game or one generation says so; a statement
that names neither holds for all of them.

**The other MOTION games.** There are more of them, and the work here already
reaches past the three: the 16-bit container is documented from four games
rather than two — the pair above plus *Amajambere* and *Eddy M.* — and the
32-bit readers index *Checker 2000*, a game on an earlier build of the same
engine, and disassemble its modules through its own kernel table. Reading a
game's data and *playing* it are different distances, though. The engine, the
readers, the renderer and the audio are shared already; what a further game
needs is its own bootstrap, its script variables and its location scheme read,
which is the per-game work the
[documentation map](docs/README.md#documentation-map) keeps a section for.
Until that is done for a game, motionvm refuses its directory by name at
start-up rather than opening it under another game's title.

## What is here

| Crate | What it is |
|---|---|
| `motionvm-formats` | Readers for every shipped format of both generations: the 32-bit RSC containers, GFX8 sprites and the GFXCRUNCH LZW codec, HMI songs, Ad Lib banks, driver archives and LE binaries; the 16-bit `DATA.-n-` containers, raw sprites and fonts, the MZ binary; and for both, palettes, fonts, text tables, script modules and the kernel tables |
| `motionvm-forth` | The Forth virtual machines — threaded code, the two stacks, the kernel dispatch; the 32-bit machine with its packed address model, the 16-bit machine with its flat space and word-id table |
| `motionvm-render` | The indexed framebuffer and the screen compositing above it |
| `motionvm-audio` | The rebuilt FM driver, the HMI sequencer and OPL3 synthesis; the 16-bit games' PSM 2 sequencer and player, rebuilt from `MUSADL.DRV` |
| `motionvm-engine` | The runtime the VM calls into: screens, descriptors, text, walking, saving, the game loop |
| `motionvm-tools` | `motionvm-tools`, a CLI for inspecting and extracting a game's resources |
| `motionvm-app` | `motionvm`, the window |
| `motionvm-testutil` | Where the test suites find the games' files. Development only; nothing ships with it |

`motionvm-formats` and `motionvm-forth` carry both engine generations, as
`m32` and `m16`; the engine is generic over the machine it drives, with the
behaviors that differ named per generation, and the renderer, the audio and
the window serve every game here.

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

The other 22 files of the installation — the DOS extender, the launcher,
the sound-setup kit, the loose copies of things the containers already
hold, the German readmes — are never opened: copying them costs nothing
and leaving them out costs nothing. What each of them is, file by file,
is documented in [Other shipped files](docs/games/ds2/other-files.md).

So a minimal copy is five files, or eight with sound:

```sh
mkdir motion-min
cp 001.RSC 002.RSC 003.RSC ENGINE.EXE 000.FRT \
   HMIMDRV.386 MELODIC.BNK DRUM.BNK  motion-min/
```

### Die Enviro-Kids greifen ein

#### Required

Two files. Without either one motionvm stops at startup and says which.

| File | Size | What it holds |
|---|---|---|
| `DATA.-1-` | 7.6 MB | The whole game: 65 script modules, 1586 sprites, 96 text tables, 130 blocks, 23 palettes, 3 fonts, the font reference table |
| `ENVIRO.EXE` | 167 KB | Not run, read: the 233-word kernel table is lifted out of the MZ image; its handlers are what the engine words follow |

#### Required for sound

| File | Size | What it holds |
|---|---|---|
| `MUSADL.DRV` | 4 KB | The PSM 2 Ad Lib driver, whose tables the rebuilt player reads |

Missing, it is not fatal: motionvm prints `sound is off: …` and plays on
in silence.

#### Everything else is ignored

The other eleven files of the installation — `KIDS.BAT` and `SOUND.EXE`,
the five other `.DRV` files, `README.TXT`, and three files nothing
references — are never opened. What each of them is, file by file, is
documented in [Other shipped files](docs/games/enviro/other-files.md).

So a minimal copy is two files, or three with music:

```sh
mkdir enviro-min
cp DATA.-1- ENVIRO.EXE MUSADL.DRV  enviro-min/
```

### Jeff Jet - Abenteuer InfoHighway

#### Required

Three files. Without any one of them motionvm stops at startup and says which.

| File | Size | What it holds |
|---|---|---|
| `DATA.-1-` | 1.4 MB | Volume 1: 55 script modules, 65 text tables, 119 blocks and 947 sprites, all LZW-packed |
| `DATA.-2-` | 1.1 MB | Volume 2: 523 more sprites, and **every palette, both fonts and the font reference table** |
| `HPPLAY.EXE` | 166 KB | Not run, read: the 228-word kernel table is lifted out of the MZ image. It is an older build than `ENVIRO.EXE` and its ordinals differ, so this game's table has to come from this game's binary |

The second volume is not optional. Everything the game draws through lives on
it; a copy without it would find every script and no colour, and motionvm
refuses it by name rather than starting.

#### Required for sound

| File | Size | What it holds |
|---|---|---|
| `MUSADL.DRV` | 4 KB | The PSM 2 Ad Lib driver — byte-identical to Die Enviro-Kids greifen ein's |

Missing, it is not fatal: motionvm prints `sound is off: …` and plays on
in silence.

#### Everything else is ignored

The other nine files — `HP.BAT`, `SOUND.EXE`, the five other `.DRV` files and
the two splash pictures `HPLOGO.EXE` and `PROMSOFT.EXE`, which are not MOTION
programs at all — are never opened. What each of them is, file by file, is
documented in [Other shipped files](docs/games/jeffjet/other-files.md).

So a minimal copy is three files, or four with music:

```sh
mkdir jeffjet-min
cp DATA.-1- DATA.-2- HPPLAY.EXE MUSADL.DRV  jeffjet-min/
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
and asked for again — so the binary can be double-clicked. Savegames go under
the platform data directory (above), one directory per game: `saves/ds2/`,
`saves/enviro/` and `saves/jeffjet/` — all three name their slots alike and
each looks for them at start-up; the directory is created on startup if it is
not there and its path is printed, so a fresh clone needs no setup.
`--loc N` starts in a given location — instead of the intro for Dunkle
Schatten 2, right after it for the two 16-bit games, whose `RUN` enters a
first location itself — and `--no-sound` runs silent.

The window shows the picture the way the game's own monitor did, and only
ever scaled by whole numbers — one per axis. Dunkle Schatten 2's 640×480 is
square-pixel 4:3 and opens at twice its size; the 16-bit games'
320×200 filled a 4:3 screen with pixels 6/5 as tall as wide, so each of their
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
ordinary files — a 32-bit game's `NNN.RSC` banks or a 16-bit game's `DATA.-n-`
volumes, told apart by the files in the directory. It asks less of a directory
than the player does — a container and the engine binary beside it are enough,
and it reads a MOTION game whether or not motionvm plays it. Every command
takes the game directory as its first argument; `--help` prints the same
summary. The ids in the examples are Dunkle Schatten 2's. The release archives
carry only `motionvm` — the tool is run from a checkout:

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
module's symbol table) and as `.f` listings read through the game's own
engine binary's kernel table, with its own `kernel-usage.txt`, and the
sprites — which carry
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
name — through `ENGINE.EXE`'s table for a 32-bit module, and through
`ENVIRO.EXE`'s or `HPPLAY.EXE`'s, whichever the directory holds, for a 16-bit
one. Reading a compiled module is in scope for this project; writing
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

| Input | What it does |
|---|---|
| Left click | Walk, use, or pick the thing under the pointer |
| Right click | Open the verb menu on it |
| Escape | Dunkle Schatten 2's in-game menu — save, load, options, quit. In the two 16-bit games it skips the intro; their menu is the icon at the bar's right end |
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
just check-nodata   # the same, with no game data at all — what CI runs
```

or without [`just`](https://github.com/casey/just):

```sh
cargo test --workspace
```

which finds the games at `../games/DS2`, `../games/ENVIRO` and
`../games/JEFFJET` beside the checkout; `MOTIONVM_GAMEDATA_DS2`,
`MOTIONVM_GAMEDATA_ENVIRO` and `MOTIONVM_GAMEDATA_JEFFJET` point anywhere
else.

The tests hold the implementation against the originals' own files: the
decoders against every resource in all three games' containers, the FM driver
against the bytes of `HMIMDRV.386` and the PSM player against `MUSADL.DRV`'s
tables, the renderer against extracted artwork, the engine against the
behavior of the games' own script modules — Dunkle Schatten 2's scenes, Die
Enviro-Kids greifen ein's boot, locations, conversations, savegames and text
rendering, and Jeff Jet's boot, thirteen locations, modules, music and slots.
They need the game directories — `MOTIONVM_GAMEDATA_DS2` for Dunkle Schatten 2
(`001.RSC` and friends, looked for at `../games/DS2` when the variable is not
set), `MOTIONVM_GAMEDATA_ENVIRO` for Die Enviro-Kids greifen ein
(`ENVIRO.EXE`, `../games/ENVIRO`) and `MOTIONVM_GAMEDATA_JEFFJET` for Jeff Jet
(`HPPLAY.EXE`, `../games/JEFFJET`); without one, every test that needs that
game's data **skips itself** rather than failing, so a checkout tests cleanly
on a machine that has no copy of any of them, and a machine with one game runs
that game's tests. Every test says which game it drives.

The two 16-bit games are told apart by their engine binary, and so are their
variables: both ship a `DATA.-1-`, so a probe for the container would let
either variable accept the other game.

Setting one of the variables to a directory that does not hold its game — no
`001.RSC`, no `ENVIRO.EXE`, no `HPPLAY.EXE` — is the one case that is **not**
a skip: it panics
and says so. A mistyped path would otherwise read as "this machine has no game
data", and a run that skips everything looks exactly like a run that passes
everything.

`tests/ds2_scenes.rs` drives the engine to four standing pictures of Dunkle
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
Interaction, dialogue, walking, savegames, the verb menu and most locations of
any of the three games have never been differentially compared, and **nothing
of Jeff Jet has been**: it is held against its own files — every item unpacked
and re-parsed, every module disassembled with no unknown ordinal, all thirteen
locations entered and drawn, its nine tunes played — and not yet against a
recording of the original. That is not a gap being hidden; it is the honest
edge of what a reimplementation without the original running beside it can
claim.

## Questions that come up

**Is this an emulator?** No. Nothing emulates a CPU or DOS here: motionvm
is a native program that reads the game's own data files — bytecode,
sprites, music — and runs them itself, the way the original engine did.
DOSBox is not involved and not needed.

**Where do I get the games?** From your own copy — an original CD-ROM or
installation. They were given away free of charge at the time, as
commissioned promotional games, and copies circulate on the internet —
but free distribution then is not a license now: the copyright stands
with its holders, and this repository neither hosts nor links to any
game files. Without them, motionvm starts, says exactly what is missing,
and stops.

**Does it run on current Windows and macOS?** Yes — the
[release archives](#downloads) carry a Windows and a macOS build; on
Linux it builds from source with one `cargo build --release`.

**Are the games in English?** No, they are German-language throughout, and
motionvm plays them as they are: it changes nothing about the content.

**Will it play another MOTION game?** Not today. `motionvm-tools` reads one —
the readers are the engine's, not a game's — but playing one needs its
bootstrap and its script variables read first, and until they are, motionvm
says so at start-up rather than opening the directory under the name of a game
it already knows.

## Documentation

[`docs/README.md`](docs/README.md) is the hub for the full technical
documentation — every file format, the virtual machine and the engine's
subsystems for both generations of the engine, a page per script module of
Dunkle Schatten 2 and the structure of each game it plays — written to stand
on its own as a specification of MOTION.

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
