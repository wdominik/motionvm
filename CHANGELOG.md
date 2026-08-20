# Changelog

All notable changes to this project are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/wdominik/motionvm/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/wdominik/motionvm/releases/tag/v0.1.0
