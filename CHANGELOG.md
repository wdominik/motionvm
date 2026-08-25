# Changelog

All notable changes to this project are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1] - 2026-08-25

### Fixed

- **Die Enviro-Kids greifen ein: walking through a door flashed the old
  room in the new room's colors.** The LEAVE verb runs the location
  change inside the order machine's callback (`CALCLEAVE`, module 606:
  `_ORDER 2 +@ INCLLOC`), where the interpreter cannot pause — so the
  queued `FADEOUT` had not run yet when the next room's `XSETPAL`
  arrived, and the palette recolored the standing picture for the whole
  closing wipe. A `SETPAL` behind a queued fade now queues with it and
  takes effect when that fade finishes, the order the original gets by
  running its fades inside the word (`05f1:2827`, `05f1:01ff`).
- **Die Enviro-Kids greifen ein: some spoken lines lost their outline.**
  Every text on templates 2 and 6 — Eva's dialogue lines among them —
  drew without its silhouette ring. `DEFTDT` writes a fixed table
  indexed by the template id (`ds:0x1A0C`), so `RUN`'s nine definitions
  replace the two the intro made over its own, later freed, shadow font;
  appending instead kept the intro's dead entries first in line. `SDTDT`
  also now ignores an id outside 1..=20 on the 16-bit machine, as the
  handler at `05f1:0c78` does — `0 SDTDT` leaves a template standing.

## [0.3.0] - 2026-08-24

### Added

- **Die Enviro-Kids greifen ein plays, start to finish.** The second MOTION
  game — the 16-bit generation, `DATA.-1-` beside `ENVIRO.EXE` — is told
  apart by its files and played natively: `RUN` boots through the DigiTales
  logo and the briefing into the scrapyard, and every location, the walk,
  the inventory bar, the verb menu, the hover captions, the conversations
  and the day tasks run as read from `ENVIRO.EXE`'s handlers at the
  instruction level — the same machine as the 32-bit engine's at half the
  offsets, with every difference named per generation in the docs. That
  reading goes deep where the two engines part: descriptors are numbered
  per screen and drawn in the 16-bit level chain's order, background
  blocks are copied whole where sprites leave index 0 unpainted, the
  walking figure wears each route's per-mille size and level, scene
  changes erase through `FADEIN`'s full compose, the fades are the 16-bit
  engine's box wipes rather than the 32-bit band curtain, the pointer
  stays off the intro as its show counter asks, script loops that poll
  for input turn once a frame, and the text drawer follows its own read
  rules — the outline's silhouette pass, justified blocks for the
  newspaper, `#` eaten as the paragraph mark, bare `GDWIDTH` sizes.
  Saving and loading go through the game's own page and slot row into
  three files of motionvm's own layout (`ENVFRZ`/`ENVANM`), kept in
  `saves/enviro/` so the two games' identically named slots stay apart.
  The window shows the game as its monitor did: 320×200 filled a 4:3
  screen, so each pixel is drawn 6/5 as tall as wide, in whole-number
  factor pairs — exact at 5×6 and its multiples, the opening size the
  largest exact step the screen has room for. Where recordings of the
  original exist, the rebuild is held against them: the intro's title
  scene and the help viewer's pages to the pixel, the fade to the ring,
  the text to a played capture.

- **Die Enviro-Kids greifen ein plays its music.** The PSM 2 tunes go
  through a rebuild of the game's own Ad Lib driver: the data tables are
  read out of the shipped `MUSADL.DRV` at start-up and the sequencer
  around them follows that driver's code exactly, down to the register
  shadow that makes the chip see changes only. `STARTTUNE` plays endless,
  as every call site asks; `ENDTUNE` is the original's fade-and-stop
  pair. The register stream is identical, write for write, to an OPL
  capture of the original across two tunes and the stop between them.
  Without `MUSADL.DRV` the game runs silent. (The original can also play
  the same tunes sampled, through its digital drivers; motionvm plays
  the Ad Lib rendition.)

- **`motionvm-tools` reads Die Enviro-Kids greifen ein.** `info`,
  `extract`, `sprite` and `script` read the `DATA.-1-` container: sprites
  as indexed PNGs through a palette of the caller's choice (`--pal N`,
  palette 0 by default — a 16-bit sprite carries none of its own),
  palettes, fonts, text tables, blocks with an index that marks the PSM 2
  songs, and script modules with symbol tables and `.f` disassemblies
  read through `ENVIRO.EXE`'s kernel table, plus `kernel-usage.txt`. The
  32-bit commands are unchanged.

### Changed

- **motionvm describes itself as the reimplementation of the MOTION
  engine for the games built with it** — *Im Netzwerk gefangen – Dunkle
  Schatten 2* (MOTION 32-bit, `ENGINE.EXE`) and *Die Enviro-Kids greifen
  ein* (MOTION 16-bit, `ENVIRO.EXE`). README, CONTRIBUTING and the
  documentation say which generation and which game every statement is
  about, and the documentation tree is laid out by generation
  (`docs/motion32/`, `docs/motion16/`) and by game (`docs/games/ds2/`,
  `docs/games/enviro/`).

- **The test suite reads two game directories.** `MOTIONVM_GAMEDATA_DS2`
  points at Dunkle Schatten 2's and `MOTIONVM_GAMEDATA_ENVIRO` at Die
  Enviro-Kids greifen ein's; `MOTIONVM_GAMEDATA` is no longer read. The
  fallbacks are `../games/DS2` and `../games/ENVIRO` beside the checkout,
  so a plain `just check` runs everything with nothing passed. As before,
  a missing directory skips that game's tests and a wrong path panics.

### Fixed

- **`SD%SHR` sets both shrink fields and the walk's `1006` command takes
  the shadow's size as it stands** — both as `ENGINE.EXE` has them
  (0x721b8, 0x78d7f), confirmed against `ENVIRO.EXE`. The figure wears
  the route's scale on both axes wherever a scene's script has set one
  of them alone.

## [0.2.0] - 2026-08-23

### Added

- **A folder dialog asks for the game directory** when the command line names
  none, so the binary can be double-clicked. A directory that is not a MOTION
  game is reported in a message box — the same message the command line gets —
  and the dialog asks again; Cancel quits. The dialog is the platform's own:
  native on Windows and macOS, the XDG desktop portal on Linux.
- **Alt+Enter toggles borderless fullscreen** (Option+Return on macOS): the
  whole screen at the largest whole-number scale that fits, black around the
  picture, no change of display mode. Alt+Enter again brings the window back at
  its previous size.
- **Binary releases.** A self-contained `motionvm.exe` for Windows and a
  universal `motionvm.app` for macOS, built from the tag by a GitHub Actions
  workflow and published from a draft release; Linux builds from source. NOTICE
  and the README say how the source archive beside them meets the LGPL's relink
  condition.

### Changed

- **The window opens at twice the game's size**, not three times: 1280×960 fits
  under the title bar of a 1080p screen and 1920×1440 does not, and the window
  can always be dragged bigger — or sent fullscreen.
- **There is no `../gamedata` default any more.** No directory on the command
  line means the dialog; a script that relied on the relative default has to
  name the path.
- **No console window on Windows.** The release build is a windowed program;
  what goes to stderr — `savegames in …`, `sound is off`, `--help` — is not
  shown there. Debug builds keep the console.

### Removed

- **`--saves DIR` and `--shot PATH`.** Savegames and the F12 screenshot always
  go under the platform data directory — `saves/` and `shot.png` in
  `~/Library/Application Support/motionvm`, `%APPDATA%\motionvm` or
  `$XDG_DATA_HOME/motionvm` — and both paths are printed as they are used. A
  flag that moves them is mostly a way to point a save directory at something
  that is not one, and the default was already the documented place.

## [0.1.1] - 2026-08-22

### Changed

- **The minimum supported Rust version is 1.97.0**, down from 1.98.0. The old
  number was a policy — track current stable, carry nothing older — and it
  broke CI: the GitHub runner images bring their own stable and lag the channel
  by a week or two, so four days after 1.98.0 was released every job on all
  three runners failed before compiling a crate. 1.98.0 was never a floor
  either; the workspace and all its targets compile on 1.95.0, and the oldest
  thing that genuinely stops it is let-chains, stable in edition 2024 since
  1.88. `rust-version` is now kept at or below what the runners carry, and
  `rust-toolchain.toml`, `CONTRIBUTING.md` and the workflow say so.

### Fixed

- **The drawer is incremental, and the mailbox erases its screen again.** The
  original's drawer (`0x6915b`) never clears a surface. It empties the screen's
  damage map (`screen+0x41A`, one `u16` per 8×8 tile, refilled at `0x69248`),
  works out from that map which descriptors have to be redrawn (`0x6e8c8`,
  `0x6eb04`) and repaints only those — a descriptor needs the active bit `0x80`
  *and* the dirty bit `0x40` to be visited at all (`0x694ed`, `0x69680`) and
  loses the dirty bit once drawn (`0x69659`). motionvm cleared every screen
  buffer and redrew every active descriptor instead, which is the same picture
  in most scenes and the wrong one in the in-game mailbox: there, `HIDSCR`
  switches the terminal's sixteen row descriptors off and `CLSCR` then covers
  the rows with black bars, one every three frames, to fake a modem redraw. With
  a rebuilt frame the rows vanished the moment they were switched off, and the
  wipe — a second of black bars over an already black screen, the cursor
  stepping down the empty rows — was invisible. `SDINACTIVE` (`0x72033` →
  `0x6ab6e`) marks a rectangle on the descriptor's *own* level, so it repaints
  what is above and never what is below: hiding erases nothing.
- **`SDAUTOBUF` is what erases.** `0x72104` hands a descriptor a buffer number
  at +0x1C and sets flag `0x08`; the drawer copies the surface under it before
  every blit (`0x696ed` → `0x6b936` → `0x2825d`) and `0x6ab6e` puts the copy
  back when the descriptor is hidden or moves (`0x6ac33`). motionvm builds the
  vacated place again out of the descriptor list rather than remembering a copy
  — the same picture wherever the picture belongs to descriptors, which is
  everywhere the game uses the flag, and one that cannot go stale. The mailbox
  pairs the two deliberately: its rows carry no buffer (module 316, `0x00f60`)
  and its bars do (`0x01020`), and bar sprite 4148 is drawn in index 9, the very
  index the monitor's screen area carries in background sprite 4009 — over the
  background it is invisible, so it can only erase text.
- **`FADEOUT` wipes the surface it fades.** The handler fills the screen's
  rectangle with colour 0 before its first band (`0x74d44` → `0x188fd`) and
  `FADEIN` marks every descriptor of the screen (`0x74af1` → `0x6b0fe` →
  `0x6a8f9`) before its single draw. Neither mattered while every frame was
  rebuilt; with a surface that persists, both do.
- **The cursor keys reach the game.** `?KEY` does not answer a character: the
  translator both it and `KEY` call (`ENGINE.EXE` `0x2379b`) reads INT 16h and
  returns `flags | code`, where `0x100` says the low byte is a scan code and
  `0x200`/`0x400`/`0x800` are Shift, Ctrl and Alt. The frontend answered
  characters only and dropped every key that has none, so the in-game mailbox —
  which dispatches on 328, 336, 331 and 333 at module 216 `0x0c21c`, and which
  the game's own manual says is worked with the cursor keys — could not be
  navigated at all, and the debug layer's F1, F2 and F9 were dead. The whole
  translation is ported, function keys and modifier folding included.
- **Keystrokes are queued rather than overwritten.** The original reads from the
  BIOS buffer, which holds fifteen, and `?KEY` takes one per frame. The frontend
  held a single key and let the second of two presses inside one frame replace
  the first; it now keeps the same fifteen and hands out one per `ICTRL` round.

## [0.1.0] - 2026-08-20

The first public release: a from-scratch reimplementation of **MOTION**, the DOS
adventure engine DigiTales (Stefan Hoffmann) shipped in 1996, running the games
built with it natively — the same compiled Forth bytecode, the same 25 fps frame
cycle, the same 640×480 picture in 256 colors, the same OPL3 music, with no
emulator underneath. Built against engine version V0.06.06/R109 of 1996-10-22,
as shipped with *"Im Netzwerk gefangen – Dunkle Schatten 2"*.

**No game data is included and none can be.** The resources are copyrighted and
have to come from your own copy of the game; motionvm reads an existing
installation and never writes into it.

Played and tested on macOS. Linux and Windows are built, linted and tested by
CI, but without the game data — so they are known to compile and to pass the
data-free tests, and are otherwise untried.

Three things are verified against the original engine's own output: **one
rendered frame**, pixel for pixel (the title screen, 307,200 pixels over 206
palette indices, every index mapping to one color and back); **fourteen VM
primitives**, against modules the 1996 compiler produced; and **a song's first
8,000 OPL register writes**, with one mixer envelope. Everything else is
verified against the original's *files* — its resources, its bytecode, its
driver binary — which says the readers agree with the data, not that the engine
behaves as the engine did. See "What is and is not verified" in the README.

### Added

- **The game runs.** `motionvm` plays a MOTION title end to end: startup,
  locations, walking, the verb menu, conversations, the inventory, the in-game
  menu, saving and loading, and the ending.
- **Format readers** (`motionvm-formats`) for every shipped format — RSC
  containers, GFX8 sprites and the GFXCRUNCH LZW codec, palettes, bitmap fonts
  and the font reference table, text tables, binary blocks, compiled script
  modules, HMI songs, Ad Lib instrument banks, `.386` driver archives, and LE
  executables. Readers report malformed input as errors rather than panicking.
- **A Forth virtual machine** (`motionvm-forth`): threaded-code interpreter,
  data and return stacks, the packed `(module << 16) | offset` address model,
  the 356-word kernel, and the blocking words the engine suspends inside.
- **The runtime the bytecode calls into** (`motionvm-engine`,
  `motionvm-render`): screens and the descriptor scene graph, indexed
  compositing, text rendering with the original's measurement and outline
  passes, the `FADEOUT`/`FADEIN` curtain, the walking system and route graph,
  the interaction and dialogue machines, the location task system, and
  savegames.
- **Audio** (`motionvm-audio`): the rebuilt `fmmidi3.com` FM driver, the HMI
  sequencer, and OPL3 synthesis through `nuked-opl3`.
- **`motionvm`**, the windowed frontend: integer scaling with the picture
  centered, mouse input, and the original's keys delivered into `ICTRL` as the
  DOS engine's BIOS reads did. `--saves DIR`, `--shot PATH`, `--loc N` and
  `--no-sound`; F12 freezes the picture and writes it out as an indexed PNG.
  Savegames and screenshots go under the platform's data directory, never into
  the working directory.
- **`motionvm-tools`**, a CLI for inspecting a game's resources, with four
  subcommands: `info` (every resource bank with counts and sizes), `extract`
  (sprites, palettes, fonts, texts, music, scripts and the kernel usage table),
  `sprite` (one sprite as an indexed PNG) and `script` (one module's header,
  symbol table and disassembly).
- **Case-insensitive game-file lookup**, so an installation whose files were
  lower-cased in transit works on a case-sensitive filesystem.
- **43 pages of documentation** under `docs/` covering every file format, the
  virtual machine, the engine's subsystems and the script library — including
  `open-questions.md`, the ledger of what is not yet known about the original,
  and `departures.md`, the ledger of where this implementation knowingly
  differs from it.
- **A test suite that skips loudly.** Tests needing the game data say so and
  skip when it is absent; a *wrong* `MOTIONVM_GAMEDATA` panics rather than
  skipping, so a run that checked nothing cannot be mistaken for a run that
  passed. CI runs formatting, lints, tests and documentation on Linux, macOS
  and Windows.

[Unreleased]: https://github.com/wdominik/motionvm/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/wdominik/motionvm/releases/tag/v0.3.1
[0.3.0]: https://github.com/wdominik/motionvm/releases/tag/v0.3.0
[0.2.0]: https://github.com/wdominik/motionvm/releases/tag/v0.2.0
[0.1.1]: https://github.com/wdominik/motionvm/releases/tag/v0.1.1
[0.1.0]: https://github.com/wdominik/motionvm/releases/tag/v0.1.0
