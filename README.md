# motionvm

Play German MS-DOS point-and-click adventures of the mid-nineties natively on
today's Windows, macOS and Linux. No DOSBox, no emulator: motionvm is a
from-scratch Rust reimplementation of **MOTION**, the adventure engine those
games were built with. You bring the files of your own copy of a game;
motionvm finds them and plays it — picture, music, savegames and all. The aim
is the original, not a remaster: motionvm follows the engine's own code
closely enough that a game looks, sounds and plays the way it did then — only
the machine under it is new.

> *motionvm spielt deutsche DOS-Adventures der neunziger Jahre nativ auf
> heutigen Rechnern — ohne DOSBox, ohne Emulator, und so originalgetreu wie
> möglich: Die Spiele sollen aussehen, klingen und sich spielen wie damals,
> nur eben auf einem heutigen System. Benötigt werden nur die Dateien einer
> eigenen Spielkopie; die Spiele selbst sind hier nicht enthalten.*

## The games it plays

Seven so far, across the two generations of the engine:

| Game | Year | Commissioned by | Made by | Engine |
|---|---|---|---|---|
| *Im Netzwerk gefangen – Dunkle Schatten 2* | 1996 | Bundesministerium des Innern | DigiTales GmbH, Hamburg, produced by Art Department WA GmbH, Bochum | **32-bit** — `ENGINE.EXE` V0.06.06/R109 |
| *Checker 2000* | 1996 | The AOK — the regional health insurers of the new federal states | Promotion Software GmbH, Tübingen (attributed; the files name no studio) | **32-bit** — `ENGINE.EXE` V0.04.15/R78 |
| *Die Enviro-Kids greifen ein* | 1996 | Ministerium für Umwelt, Raumordnung und Landwirtschaft NRW | Art Department Werbeagentur GmbH | **16-bit** — `ENVIRO.EXE` |
| *Jeff Jet - Abenteuer InfoHighway* | 1995 | Hewlett Packard GmbH | Promotion Software GmbH, Tübingen | **16-bit** — `HPPLAY.EXE` |
| *Hilfe für Amajambere* | 1995 | Bundesministerium für wirtschaftliche Zusammenarbeit und Entwicklung | ART DEPARTMENT WA GmbH, Bochum | **16-bit** — `BMZ.EXE` |
| *Victor Loomes – Das Spiel* | 1993 | LBS (Landesbausparkasse) | Promotion Software GmbH, Reutlingen | **16-bit** — `LL.EXE` |
| *Falsches Spiel mit Eddie M.* | 1994 | Gruner + Jahr, for the magazine *Stern* | via productions ag, by the published record | **16-bit** — `STERN.EXE` |

All seven are commissioned work — advergames and edutainment, given away rather
than sold — which is why each has a client as well as a studio; each entry
names them as the game's own files do, or, where the files name nobody, as
the published record does and says so. They are German-language throughout, and
motionvm plays them as they are: it changes nothing about the content.

Every one of them boots, enters each location it has and draws it, plays its
music, and saves and loads through the game's own pages; what is particular
to each one is on its own pages in the
[documentation](docs/motion/README.md#documentation-map).

MOTION was written by DigiTales (Stefan Hoffmann) — Dunkle Schatten 2's own
credits say so, naming the *"Motion"-Präsentations-System von S. Hoffmann* and
*DigiTales GmbH, Hamburg* — and Victor Loomes, three years earlier, is the one
that gives it a version: *Erstellt unter · Motion 1.0*.
The engine made more games than these seven, and
the list is a record of what has been done, not a limit of what is underneath
it.

## Getting started

### A ready-made build

Every release on the [Releases page](https://github.com/wdominik/motionvm/releases)
carries two zips:

| File | What it is |
|---|---|
| `motionvm-x.y.z-windows-x86_64.zip` | One `motionvm.exe`, for Windows 10 and later. The C runtime is linked in and everything else it uses ships with Windows, so there is nothing to install beside it. SmartScreen says "Windows protected your PC" the first time — **More info → Run anyway** |
| `motionvm-x.y.z-macos-universal.zip` | `motionvm.app`, one binary for Apple Silicon and Intel. macOS refuses the first start of an app it has not seen before — **System Settings → Privacy & Security → Open Anyway** lets it through, once; `xattr -dr com.apple.quarantine motionvm.app` does the same from a terminal |

Unzip and start it. The platform's own folder dialog asks for your copy of the
game, and a directory that is not a MOTION game is reported in a message box
and asked for again — so the binary can simply be double-clicked. `LICENSE`,
`NOTICE` and this README are in each zip.

### From source

This is also how Linux gets a build; there is no Linux zip.

```sh
cargo build --release
./target/release/motionvm /path/to/gamedata
```

Requires a Rust toolchain of 1.88.0 or newer — the floor `rust-version` in
`Cargo.toml` names, and one CI compiles the workspace on. That is recent
enough that a Rust which came with your distribution may well be too old;
`rustup` is the reliable way to have one. Nothing else is needed: no C compiler, and no system libraries
beyond what a window and an audio device take. On Linux that means the ALSA,
udev, xkbcommon and Wayland development headers, which
`.github/workflows/ci.yml` names by Debian package. The folder dialog adds nothing to build against: it
goes through the XDG desktop portal, which at run time wants the
`xdg-desktop-portal` service that GNOME and KDE carry, or `zenity` as the
fallback — and without either, the directory goes on the command line.

### Options

```
motionvm [GAMEDIR] [options]

  --loc N       start in location N instead of where the game would begin
  --no-sound    do not open an audio device
  -h, --help    the full text, with the game list built from the roster
```

`--loc N` skips the intro for Dunkle Schatten 2 and lands right after it for
the 16-bit games, whose boot word enters a first location itself. Two of them
take it one click later: Hilfe für Amajambere opens on its menu and Victor
Loomes on a full-screen competition slide, and both have to be clicked away
first. Checker 2000 refuses it and says why: its story is a task list that
enters each location when the step naming it comes round, so there is no
location to ask for.

## What a game needs

**No game data is included, and none can be.** The resources are copyrighted
material that has to come from your own copy of the game. Point motionvm at the
directory the original was installed into and it finds what it needs.

Nothing else in an installation is ever opened, so a copy can be this small:

| Game | Files it needs | For sound | Together |
|---|---|---|---:|
| Dunkle Schatten 2 | `001.RSC`, `002.RSC`, `003.RSC`, `ENGINE.EXE`, `000.FRT` | `HMIMDRV.386`, `MELODIC.BNK`, `DRUM.BNK` | 19 MB |
| Checker 2000 | `001.RSC`–`004.RSC`, `ENGINE.RSC`, `ENGINE.EXE`, `000.FRT` | `HMIMDRV.386`, `MELODIC.BNK`, `DRUM.BNK`; `SMPPATH` and `WAVS/` for the speech | 33 MB |
| Die Enviro-Kids greifen ein | `DATA.-1-`, `ENVIRO.EXE` | `MUSADL.DRV` | 7.8 MB |
| Jeff Jet | `DATA.-1-`, `DATA.-2-`, `HPPLAY.EXE` | `MUSADL.DRV` | 2.7 MB |
| Hilfe für Amajambere | `DATA.-1-`, `DATA.-2-`, `BMZ.EXE` | `MUSADL.DRV` | 5.5 MB |
| Victor Loomes | `DATA.-1-`, `LL.EXE` | `MUSADL.DRV` | 1.1 MB |
| Falsches Spiel mit Eddie M. | `DATA.-1-`, `DATA.-2-`, `DATA.-3-`, `STERN.EXE` | `MUSADL.DRV` | 2.8 MB |

Every line of that was established by taking the file away and seeing what
happened, not by reading the loader. Leave out a file from the middle column
and motionvm stops at start-up and says which one; leave out one from the third
and it prints `sound is off: …` and plays on in silence. The one exception
is Checker 2000's `ENGINE.RSC`: the game opens without it, because every
text it draws names a font of its own, but the system palette the engine's
arrow pointer takes its two colors from is in it, and without that both
colors resolve to index 0 and the arrow is not seen.

```sh
mkdir vloomes-min
cp DATA.-1- LL.EXE MUSADL.DRV  vloomes-min/
```

Two of those columns are worth a sentence. **The engine binary is never run —
it is read:** the game's kernel table is lifted out of the image, and the
game's own bytecode means nothing without the table from its own build, which
is why each game needs the binary that shipped with it. **A second `DATA.-2-`
is not optional** where one ships, nor a third: it is where the artwork lives —
in Hilfe für Amajambere the second volume holds every sprite, palette and font
the game ever draws, and Falsches Spiel mit Eddie M. spreads its sprites over
two more volumes.
motionvm looks for `NNN.RSC` and `DATA.-n-` by pattern and merges what it
finds, so the count is the game's to decide.

What each of the other files in an installation is — the launchers, the sound
setup, the readmes, the loose copies of things the containers already hold — is
written down file by file on that game's **Other shipped files** page, linked
from the [documentation index](docs/motion/README.md#documentation-map).

### The game directory is only ever read

motionvm never writes into it, and cannot be made to: the one place a writable
path enters the engine *refuses* any directory inside the game data — after
resolving both sides, so a relative path or a `..` cannot walk around it. A
read-only copy, a mounted image or a directory on a CD-ROM all work.

## Controls

The game is played with the mouse; the keys are the original's, delivered into
the engine's key variable exactly as the DOS engine's BIOS reads did — a
character where the key has one and a scan code where it has none, which is
what lets Dunkle Schatten 2's mailbox be worked with the cursor keys the way
its manual describes.

| Input | What it does |
|---|---|
| Left click | Walk, use, or pick the thing under the pointer |
| Right click | Open the verb menu on it |
| Escape | Dunkle Schatten 2's in-game menu — save, load, options, quit. In Checker 2000 it leaves a scene for the board. In the 16-bit games it skips the intro |
| Letters, digits, Return, Backspace | Checker 2000's registration board: the name and the postcode |
| Cursor keys | Move through Dunkle Schatten 2's in-game mailbox; Return or Space takes what is highlighted |
| Return, Space, Backspace, letters | Passed through to the game, which uses them on its own pages |
| F12 | Freeze the picture **and** write it out as an indexed PNG, to `shot.png` in the data directory beside `saves/`; the path is printed |
| Alt+Enter | Borderless fullscreen, on and off (Option+Return on macOS) |
| Close the window | Quit |

Each game's own menu is reached the way that game reaches it. Dunkle Schatten 2
puts it on Escape. Die Enviro-Kids greifen ein, Jeff Jet, Hilfe für
Amajambere and Falsches Spiel mit Eddie M. put it on the icon at the right end
of the bar. Victor Loomes hides
a panel along the top edge of the screen and fades it in when the pointer
reaches the top: its right end saves above the middle and loads below, and its
left end opens the game's own menu.

Escape opens the menu rather than quitting, because that is what the original
does. There is deliberately no quit key beyond the window: the game has its own
quit, and adding a second one would put a way out of the game that the game does
not know about.

F12 does two things on one key, which is a debugging convenience rather than a
design: a still picture is the one you can compare against a reference
screenshot, so freezing and capturing belong together.

## Savegames, and where things are written

Savegames and screenshots go under the platform's own data directory — never
into the game's directory, and never into whatever directory the program was
started from:

| Platform | Directory |
|---|---|
| macOS | `~/Library/Application Support/motionvm` |
| Windows | `%APPDATA%\motionvm` |
| anything else | `$XDG_DATA_HOME/motionvm`, or `~/.local/share/motionvm` |

Underneath it, one directory per game: `saves/ds2/`, `saves/checker/`,
`saves/enviro/`, `saves/jeffjet/`, `saves/hfa/`, `saves/vloomes/` and
`saves/eddiem/`. All seven games name their
slots alike and each looks for its own at start-up; the directory is created
then if it is not there and its path is printed, so a fresh install needs no
setup and a savegame is an ordinary file to copy or back up.

Each file is written whole or not at all — to a temporary name, flushed, then
renamed — so a crash partway through a save costs the new slot and not the old
one, and each carries a checksum, so a file damaged afterwards is refused as
damaged rather than half-read. A slot that has ended up in the wrong game's
directory is refused as that other game's, by name.

A savegame is a snapshot of the engine's own state, so it carries a version
number, and a release that changes what is in it raises that number. A slot of
any other version is refused by name — *`701.FRZ`: savegame version 0, this
build writes 1* — rather than half-read, and nothing deletes it. Until the
first stable release no converter ships: a release that raises the number says
so in its notes, and the slots from before it stay on disk and stop loading.
Within a version, saves carry across updates and between machines. What is in
the files is [documented](docs/motion/savegames.md).

The window shows the picture the way the game's own monitor did, and only ever
scaled by whole numbers — one per axis. Dunkle Schatten 2's 640×480 is
square-pixel 4:3 and opens at twice its size. The 16-bit games' 320×200 filled
a 4:3 screen with pixels 6/5 as tall as wide, so each of their pixels becomes a
block as close to that shape as whole numbers come — exact at 5×6 and its
multiples — and the window opens on the largest exact step the screen has room
for. Drag it bigger and the picture steps up to the next whole pair that fits,
with black around it. Alt+Enter takes the whole screen the same way, as a
borderless window rather than a change of display mode.

## If something does not work

| What happens | What it is |
|---|---|
| `… is not a game motionvm can open`, then what each game needs | The directory holds no game, or one that is not among the five. Started without a path, motionvm says so in a message box and asks again |
| A named file is missing at start-up | The copy is incomplete — the table above says which files that game needs |
| `sound is off: …` | A sound file is missing, or no audio device could be opened. The game plays on in silence |
| `the game stopped: …`, in a message box | The engine reached something it does not implement or a savegame it cannot load; the message names it. Every such line also goes to stderr, and `MOTIONVM_LOG=1` appends it, with everything else the run reports, to `motionvm.log` beside `saves/` — which is where a report from a Windows build, whose release binary has no console, is found |
| No folder dialog appears on Linux | Neither `xdg-desktop-portal` nor `zenity` is there. Pass the game directory on the command line instead |
| Windows or macOS refuses to start it | The build is not code-signed — see [A ready-made build](#a-ready-made-build) |

**Where it has actually been played: macOS.** CI builds, lints and tests
motionvm on Linux and Windows too, but without the game data — so those two are
known to compile and known to pass the data-free tests, and are otherwise
untried. The game directory is read case-insensitively, which is the one
portability trap that mattered: a copied install is often lower-cased, and on a
case-sensitive filesystem an exact-case lookup would find nothing.

## Questions that come up

**Is this an emulator?** No. Nothing emulates a CPU or DOS here: motionvm is a
native program that reads the game's own data files — bytecode, sprites, music
— and runs them itself, the way the original engine did. DOSBox is not involved
and not needed.

**Where do I get the games?** From your own copy — an original CD-ROM or
installation. They were given away free of charge at the time, as commissioned
promotional games, and copies circulate on the internet — but free distribution
then is not a license now: the copyright stands with its holders, and this
repository neither hosts nor links to any game files. Without them, motionvm
starts, says exactly what is missing, and stops.

**Will it play another MOTION game?** Not today. The readers are the engine's
rather than a game's, so `motionvm-motion-tools` already reads games motionvm does not
play; playing one needs its bootstrap, its script variables and its location
scheme read first. Until they are, motionvm refuses the directory by name at
start-up rather than opening it under the name of a game it already knows.

**How faithful is it?** Deliberately close, and honestly measured: single
frames of Dunkle Schatten 2 and Die Enviro-Kids greifen ein and the whole
eight-picture intro of Victor Loomes match recordings of the original pixel
for pixel, and three of the games' music has been held against register
captures of the original's sound hardware. Those recordings cannot ship, so
what the test suite keeps instead is a digest of every scene and every tune
it reaches — a change that moves a pixel or a register write is a failing
test on any machine that has the game.
[`docs/motion/verification.md`](docs/motion/verification.md) says what that covers and what
it does not; [`docs/motion/departures.md`](docs/motion/departures.md) lists every place
motionvm knowingly does something else.

One of those a player may notice: every game plays its Ad Lib rendition of
its music, whatever its sound setup said. The 16-bit games shipped a
`SOUND.EXE` that could pick a sampled renderer instead, and that renderer is
not ported — so a player who remembers the Sound Blaster mix of a 16-bit tune
hears the FM one. The 32-bit engine's digital layer is another matter: it
plays Checker 2000's speech and effects over the music as the original's
mixer does, read out of it; Dunkle Schatten 2 ships no WAV file, so its
scripts' speech path is never reached.

## How it works

A game made with MOTION is *data*: compiled Forth script modules plus sprites,
palettes, fonts, texts and music inside resource containers. motionvm reads
that data and runs it — the same bytecode, through the same frame cycle, onto
the same picture, with the same music, and with no emulator underneath.

It is one Rust workspace, deliberately small at the edges: five dependencies
to run at all — a window, a presenter, an audio device, a folder dialog and an
OPL3 core — and no graphics API anywhere. The readers, the two
virtual machines, the renderer, the audio and the runtime are split across
eleven crates — two layers and a test rig over both — a neutral one holding the window and the contract
it drives any game through, and the MOTION engine family behind that contract
— which [`ARCHITECTURE.md`](ARCHITECTURE.md) lays out, along with how the
family, its two engine generations, the five builds of the older one, the
two of the newer and the games on top of them are kept apart. The
two engine generations share the Forth dialect, the compiler's output
conventions and most of the kernel's vocabulary; they do not share the machine
— cell width, kernel ordinals, address model, container, asset encodings and
music format all differ.

MOTION itself was two halves: the runtime that plays a game, and the authoring
side that makes one — an IDE, a Forth compiler and a debugger, in the same
binary. **Only the runtime is reimplemented here.** Reading a compiled module
is in scope, and the disassembler in `motionvm-motion-tools` does it; writing one is
not.

## Further reading

| Where | What is in it |
|---|---|
| [`docs/README.md`](docs/README.md) | The map of the technical documentation — one tree per engine family; MOTION's covers every file format, both virtual machines, the engine's subsystems, and a page per game and per script module |
| [`docs/motion/tools.md`](docs/motion/tools.md) | `motionvm-motion-tools`, which reads a game's containers and writes out what it finds as ordinary files |
| [`docs/motion/verification.md`](docs/motion/verification.md) | What is checked against the original engine's own output, and what is only checked against the games' files |
| [`docs/motion/verification-method.md`](docs/motion/verification-method.md) | How such a check is made: the emulator's settings, and the two commands that compare a frame and a register stream against a capture |
| [`docs/motion/departures.md`](docs/motion/departures.md) | Every place motionvm knowingly does something else, and why |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | How the code is laid out: the two layers, the four levels of variance, and the seams between them |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | How to work in it: the conventions, the quality gate, adding a game, an engine build, or an engine family |
| [`docs/writing-a-family.md`](docs/writing-a-family.md) | The contract read from the engine's side: what the window asks of a game, in the order it asks, and what each answer has to hold |

## Licensing

**MIT** — see [LICENSE](LICENSE).

One dependency has a license of its own: **`nuked-opl3` is
`LGPL-2.1-or-later`**, as every derivative of Nuked-OPL3 is. Linked statically
into a binary, that means anyone handed the binary must be able to relink it —
the object files or the sources have to be on offer. Distributed as source, as
this is, that condition is already met; ship a pre-built binary and it becomes
yours to meet. Every release therefore carries the source archive of its tag
alongside the two zips, which is that means. [NOTICE](NOTICE) says so in full,
and [`docs/motion/motion32/engine/audio.md`](docs/motion/motion32/engine/audio.md) says why
that core was chosen anyway.

**The games' own files are not covered by any of this.** They are not part of
the project and are not redistributable; motionvm reads an installation you
already have.
