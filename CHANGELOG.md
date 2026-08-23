# Changelog

All notable changes to this project are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/wdominik/motionvm/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/wdominik/motionvm/releases/tag/v0.2.0
[0.1.1]: https://github.com/wdominik/motionvm/releases/tag/v0.1.1
[0.1.0]: https://github.com/wdominik/motionvm/releases/tag/v0.1.0
