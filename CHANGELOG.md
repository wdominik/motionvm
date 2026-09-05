# Changelog

All notable changes to this project are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0] - 2026-09-05

**Savegames written by earlier releases do not load.** The layout is this
release's, version 1; a slot from before it is refused by name and left on
disk.

### Added

- **The comparison against the original is a command.**
  `motionvm-motion-tools compare-frame` holds the frame F12 writes against a
  capture of the original on the colors the DAC held, six bits a channel,
  each picture through its own palette — a renumbered palette is the same
  picture, and a screenshot of a pixel-doubled window comes back through
  `--crop` and `--scale` — and names the first pixel that differs.
  `registers` renders a song's OPL register stream offline out of the game's
  own files, through the sequencer and driver the player runs; `compare-dro`
  holds it against a DRO recording of the original, the recording's snapshot
  set apart, the streams lined up on the write it resumes with, the chip's
  state at that note checked register for register, and the first write that
  differs named. Both exit 1 on a difference.
  `docs/motion/verification-method.md` says how a capture is made and what a
  match does and does not mean.

- **Every scene and every tune the suite reaches has a reference digest.**
  A rendered frame of a game's own artwork may not enter the repository, so
  the baselines are FNV-1a digests of the composed frame or the register
  stream, in `crates/*/tests/digests/<slug>.txt`, read on every run:
  seventy-four across the five games — each intro, each starting room, four of
  Dunkle Schatten 2's densest scenes and its opening played through, every
  tune the 16-bit games ship and the register stream of every 32-bit song. A
  change that moves a pixel or a register write is a failing test; `just
  digests` rewrites the tables from a run when the change was meant.

- **Every kernel word the shipped games reach for is built, and a suite says
  so.** `kernel_coverage.rs` disassembles every module a game ships and holds
  each word it calls to be one of the interpreter's or the engine's. The four
  16-bit games reach for nothing that is missing; Dunkle Schatten 2 names
  seven words the engine does not have, each carried with the reason none is
  reachable — the sampled-speech pump nothing calls, a location whose table
  entry is uninitialized, an unreferenced scene macro, a sprite inspector
  shipped by accident, a debug layer the shipped game cannot switch on.

- **Damaged copies of the shipped files are refused, never crashed on.**
  `malformed.rs` hands the readers inputs broken in ways somebody thought of;
  `mutation.rs` walks the real files and breaks them in ways nobody did —
  every container, every engine binary and a sample of every kind of item,
  cut short at every length through the header and flipped byte by byte
  where a seeded generator says — and asserts a reader answers rather than
  unwinds. The seed is the file's name, so a failure names a byte any other
  machine with the game reproduces.

- **A run says what it walked past.** `Playable::diagnostics` answers what
  the run has to report about itself — the words it reached that deliberately
  do nothing, the reads past a module that is not loaded, a tune that would
  not decode — and the window prints it on the way out. Nothing in the
  family's libraries prints any more.

- **A Windows player is told what happened.** The release build is a windowed
  program, so every fatal line also goes to a dialog, on every platform, and
  `MOTIONVM_LOG` switches on a log file for what a dialog cannot hold.

- **`RANDOM` is seeded by whoever runs the game.** `Playable::seed`, called
  before `start`; the window takes a seed from the clock, and a caller that
  never seeds keeps the fixed one, which is what lets the suite render a
  scene twice and compare the two.

- **`just bench`, and machine counters to feed it**: how fast each game's
  machine runs — cells and host words per second, the step and the draw
  timed apart — beside the rig that says where a frame's time goes. Neither
  asserts anything; a wall-clock number is a property of the machine it ran
  on.

- **Tests that reach past the corpus.** Detection is one table with a row
  per game, held against the roster; the three rosters — `Title::ALL`, the
  table `detect` walks, the manifests — are held to each other; the LZW
  decoder is round-tripped through an encoder of the tests' own over inputs
  the game never made; palette widening is checked over all sixty-four
  values a channel can hold; `scale_pair` against a brute force over a grid
  of window sizes. And twenty files' copies of the same test plumbing are
  one set of shared drivers.

- **Pages the documentation was short of.** `docs/writing-a-family.md`, the
  contract read from the engine's side for whoever brings a further engine;
  `docs/motion/debugging.md`, the switches, keys, reports and rigs; a
  glossary; a page for the three MOTION games on hand that the readers open
  and the player does not — Checker 2000, Compaq and Eddy M.; an *Open
  questions* section on each of the four binary pages that had none;
  `ARCHITECTURE.md`'s account of the alternatives the tree rejects and why;
  and `CONTRIBUTING.md`'s procedure for adding or correcting a kernel word.

### Changed

- **A savegame survives the things that break one.** Each of a slot's files
  is written to a temporary name, flushed and renamed, so a crash partway
  through a save costs the new slot and not the old one; the head carries the
  body's length and a CRC-32 of it, so a file damaged afterwards is refused
  as damaged; the body opens with the game's own slug, so a slot carried into
  another game's directory is refused as that game's; and the body is
  sectioned — `MODS` in a `.FRZ`, `HEAD`, `SCRN`, `FLIP`, `DESC` and the
  16-bit engine's `BUFS` in an `.anm` — so a later build can add a section
  without moving the version. An incomplete slot is reported as one, naming
  the file it lacks.

- **The display follows `SETRES` and `TOGFX`.** The composed picture takes
  the selected mode's size when `TOGFX` runs, as the original's does, where
  it took the generation's size at open. What is refused is refused at
  `TOGFX`, named: a 32K-color mode, which this renderer does not draw; a
  `TOGFX` with no mode selected; and a second entry at another size, which a
  window cannot follow. `HICOLOR` answers for the selected mode, and a
  pixel's shape follows the picture's size — square at 640×480, 5:6 at
  320×200.

- **The 16-bit handlers the machine had taken on trust are read.** Nine
  kernel words carried the 32-bit engine's reading as a hypothesis; each is
  read in `ENVIRO.EXE`. Most confirm it. Two differ, and the machine follows
  the reading: `/` and `MOD` divide the low words unsigned, `/` by zero
  answering 0 where `MOD` by zero faults; and `SFT n` installs font
  reference table `n` out of the container. The hundred-and-first
  `NEWSETDESC` on a screen of the three later builds answers no handle, as
  their handler does — `LL.EXE`'s does not look, and the opener reads which
  off the binary. `KEY` blocks in every build; the engine answers the waiting
  key or 0, a departure the ledger carries with its reading.

- **`RANDOM`'s generator is read on both engines**, and it is not a
  generator with a seed but a hash of the clock: a counter stepped by 331 a
  call, combined with the engine's raw tick count as
  `((t + s) / s) xor ((t − s) mod s)` and reduced modulo the count. The
  rebuild keeps a seeded LCG, since the original's input is wall-clock time
  and the rebuild reads none; the departure says what it departs from.

- **The frame loop is the game's, in tests as in play.** The second path
  `Game::step` had — a hand-built loop for entry points that skipped
  `4:START` — is gone with the entry points; the suites reach a scene the way
  the game does, through `START` and the game's own `_NEXTLOC`. The 32-bit
  input reaches the game by one route, the game's own: `ICTRL` reads the
  kernel words and stores the shell variables itself, and the driver no
  longer writes them a second way.

- **Kernel words are dispatched by a resolved value, not by name.** Every
  word is one value of an enum, resolved once per kernel when the game opens
  and indexed by ordinal; the eleven names that mean a different handler on
  the two machines — `FADEIN`, `SETBUF`, `SCRX`, `STARTTUNE` and their kin —
  are two values apiece, and a duplicate name is a compile error. A word
  reached that does nothing is recorded as that value, with the case that is
  inert beside it.

- **The engine's state is grouped by subject, and what a build decides is
  its own type.** `Profile`, `Copy`, arriving whole through `Engine::new` and
  carrying the capabilities the 16-bit opener probes out of the shipped
  binary; the rest in seven subsystems, thirty-one fields on `Engine` from
  seventy. The two interpreters share one `Core` of bookkeeping — the step
  budget, the trace, the counters, the generator — and keep their primitives
  apart, because the 16-bit rules have been read out of the 16-bit binaries
  and differ. Five over-long argument lists carry the record they stood for,
  each field with the offset it was measured at. The display's size is the
  contract's `Size` from the profile to the game; the music opener takes the
  opened game's generation instead of detecting the directory twice; the
  savegame's checksum is the render crate's CRC-32, one copy.

- **A frame is borrowed, not allocated.** `Playable::frame` hands over the
  picture and its palette at once, borrowed from a buffer the engine keeps,
  so a window that presents a hundred times a second allocates nothing — 3.7
  µs against 7.3 µs on the frame-cost rig, the copy being the whole of it.
  `key(press, down)` is `key_down` and `key_up`, because a release is not a
  press, and `Family::games` answers a static slice.

- **Errors keep their type and their address.** A stack underflow is stamped
  with where the machine was rather than module 0 cell 0; a missing resource
  and a `SD…` word with nothing selected have cases of their own instead of
  a sentence inside a word's name; the savegame reader answers a typed error;
  `Unread` says which binary its address is in. The family's libraries
  answer typed errors throughout, and the one box left is the contract's.

- **The readers hold their rule by construction.** `indexing_slicing` and
  `arithmetic_side_effects` are denied across `motionvm-motion-formats` and
  on the savegame reader, carried by a `Cursor` that checks a read and moves
  past it in one step, `records::<N>` over a table, and `Record`, whose
  field offsets the compiler holds inside the record; the song reader, the
  LE loader's fixup walk, the disassemblers and the driver archive read
  through the cursor, the container, bank, font and text tables through the
  records. Seven exemptions remain, each a bound the types carry and the
  lint cannot see. Two refusals came with it: an LE image spanning more than
  64 MiB of address space, refused before anything is allocated for it, and
  a fixup site not wholly inside the image, skipped rather than written past
  it.

- **Every cast is a named conversion, and `as_conversions` is denied.** Every
  `as` left in the tree is inside a function named for the rule it applies —
  the `cell` module for what a machine does with a cell's bits, the formats
  root for what a reader does with a file's offsets, the audio crate's `num`
  for the drivers' byte arithmetic — under an `#[expect]` with its reason;
  the count of those attributes is the count of conversions the tree
  performs on purpose. Everything else is `From`, a `to_le_bytes` round trip,
  or a `try_from` that refuses: a coordinate past `i32::MAX`, a frame time
  past `u64::MAX` nanoseconds, and a PNG whose pixels would not fit one
  chunk saturate or refuse instead of wrapping.

- **The lints are on, and the ones that are off say what they would cost.**
  Denied across the workspace: the idiom lints, `allow_attributes` with
  `allow_attributes_without_reason`, `wildcard_imports`, `doc_markdown`,
  `as_conversions`, and the restriction and pedantic lints the tree already
  held — `dbg_macro`, `todo`, `unimplemented`, `missing_assert_message`,
  `iter_over_hash_type` for the determinism rule, `needless_pass_by_ref_mut`,
  `precedence_bits`, `or_fun_call`, `redundant_clone`, `manual_let_else`,
  `semicolon_if_nothing_returned`, `string_slice` and a few more. Every
  exemption is an `#[expect]` with a reason. For what stays off the manifest
  carries measured counts with the command that produced them; `unwrap_used`,
  `expect_used` and `panic` stay off because the sources have none and the
  tests have all of them, which clippy's test exemptions do not reach.

- **The five longest files are split along the subjects inside them** —
  walking, the verb menu, the descriptor, the savegame module and the window
  program — moved, not rewritten, with every reference digest unchanged.

- **The PNG writer and reader are the workspace's own, and the dependency
  list is checked.** `motionvm-render` writes the indexed PNG F12 and the
  tools produce, and reads PNGs back — indexed, RGB or RGBA, any filter,
  any compression — with an inflate of its own, held against streams and
  pictures Python's `zlib` wrote; it depends on nothing. Five external
  crates in all, each with its reason in the manifest, and `deny.toml` with
  a CI job holds advisories, licenses, sources and duplicate versions.

- **`rust-version` is the floor it claims to be, and it stands where the
  code does**: 1.88.0, the 2024 edition's let-chains being the newest thing
  the tree uses, compiled on exactly that toolchain by a CI job, with
  `clippy.toml` at the same number. The rule is written down: the floor
  follows the code and rises the day a change needs a newer toolchain, with
  the feature as the reason, and no feature is avoided for the number's
  sake. Every crate is `publish = false`, decided rather than defaulted.

- **The boundary test asks Cargo instead of parsing manifests**, and a
  further test holds it to what the workspace is known to contain, so a rule
  enforced over an empty list cannot pass for the wrong reason.

- **The departures ledger is complete.** Presents are batched to what a
  display can show; every game plays its FM rendition whatever its sound
  setup said; the promise that a read past a module is counted and reported
  is kept; and the departure that walked script words by module number is
  gone — the lookup walks the module table in slot order, as `0x61596` does.

### Fixed

- **A right click on the inventory bar during a conversation opens the item
  menu.** The answers stand, the bar is live under them, and a right click on
  a slot with something in it offers `INFO` and `GIVE` for that item — the
  engine's modes 17 and 18, read and built on the verb strip the other menus
  use; the run had stopped on them as unread. Every speaker hears the idle
  call under the answers as it does under a line. Mode 15, the
  conversation's last line, is entered only by a routine nothing in
  `ENGINE.EXE` reaches, and stays unbuilt with that reason.

- **Two tracks of the opening tune that the original never plays are
  silent.** A track of an HMI song names the devices it is for, and the song
  open gives it to the first installed device one of the names matches; the
  sequencer had played every track. Dunkle Schatten 2's opening tune carries
  a bass and a second guitar for other devices, which took voices the
  original never gave them. The register stream now agrees with a recording
  of the original over the whole of it, all 17,047 writes.

- **A savegame taken with `SDBLK` set could not be loaded.** The descriptor
  fields are an enum now, one list that cannot drift from the words that
  write them, and a test drives every field through a save and a load.

- **An `ENGINE.EXE` whose header declares no objects is refused** rather than
  crashing the reader; the damaged-input suite is what would have found it.

## [0.7.2] - 2026-09-02

### Changed

- **`=>GET` loads a module and `=>ERASE` gives it back, as the original
  does.** `=>GET` (0x64999) loads a module out of the resource file into the
  first free descriptor slot every time it is asked, and `=>ERASE` frees the
  slot and the memory with it; a location's three modules come and go with
  the location, and their variables start over on every entry. The 32-bit
  machine had loaded all eighty-six modules when the game opened and kept
  them, so a location's state survived leaving and coming back, a call into
  an erased module succeeded where the original faults, and a read from one
  answered the stale image. Now the machine holds what the game has loaded:
  `SYSTEM.RSC`'s `4 =>GET` is the first load, `START` and `INCLLOC` do the
  rest, a module the container does not hold is refused by name, and a read
  into a module that has been given back answers zero and is counted, like
  every read into a module that is not loaded. Every reference digest is
  unchanged.

- **`ENDTUNE` waits the half second the original waits.** The 16-bit
  stop routine (`ENVIRO.EXE` `1696:02fd`; the other three builds' are the
  same shape) starts the driver's fade-out and spins until the 200 Hz tick
  has counted 100 before it stops the driver and returns to the script.
  The word had returned at once, so every room change with music began
  half a second early. The wait now stands where a wipe would, painting
  nothing; and `STARTTUNE` over a still-playing tune — the Play routine
  runs the stop first (`1696:02ce`), which is how Victor Loomes changes
  its music with no `ENDTUNE` between locations — fades, waits and starts
  the new tune when the wait is over. The wait comes only while the
  driver's own flag says a song is playing, as it does there.

- **`SETCYCLE` turns the palette.** Victor Loomes' time machine asks for
  entries 32 through 127 to cycle, and the request had been recorded and
  left standing. The tick is now the handler's (`LL.EXE` `0104:536d`,
  once a frame after the blit): the working palette is taken from the
  master, every entry of the range moves up by the turn's amount, wrapping
  inside the range, and the amount grows by one a turn until it would
  exceed the range's span.

- **The walk builder's two build variants are read off the binary.**
  Only `ENVIRO.EXE` (`0a40:1176`), as the 32-bit routine (`0x778ca`), takes
  a shadow's zero shrink as 1000 for the walk's first step — `HPPLAY.EXE`,
  `BMZ.EXE` and `LL.EXE` copy the field as it stands — and only `LL.EXE`
  closes `CROUTE` with a pass over the finished headings (`0104:516d`) that
  folds a one- or two-step flip between longer runs into the heading
  around it. The default had been applied to all four builds and the pass
  to none; both are now capabilities probed from `CROUTE`'s body, the way
  the hot-area hole test is.

## [0.7.1] - 2026-09-01

### Changed

- **The engine names its generation everywhere.** A crate-root import made
  bare `Vm` and `Memory` mean the 32-bit machine, so the same impl header
  meant different machines in different files. Every signature now says
  `m32` or `m16`, `dialogue32.rs` stands beside `dialogue16.rs` and
  `plain_word32` beside `plain_word16`, and the rule is total: an
  unqualified name holds for both generations, anything that holds for one
  carries its number. The one version-named capability went the same way —
  `text16` had pooled four separately measured behaviors under "is this the
  16-bit engine" and is now the four flags it was hiding, each carrying its
  own address.

- **Dunkle Schatten 2's module holds names, not behavior.** Its bootstrap
  words come from the authoring template — another MOTION 32-bit game
  exports `STARTUP`, `START` and `INCLLOC` from the same modules — so the
  functions that run them, and the one `Hooks` impl the machine can have at
  all, live with the generation in `titles/motion32.rs`, beside their
  16-bit twins. The game's file keeps what is genuinely its own: the
  module-2 variable names its compiler chose, as a `Shell` constant beside
  `LOCATION`.

- **The record says which machine a shared reading came from.** The `?KEY`
  translation is measured out of `ENGINE.EXE` and serves all five games;
  the 16-bit handler is unread, and the departures ledger and the open
  questions now carry that instead of nobody saying it. The `?XINSIDE`
  hole test is marked as the build split it is on both pages it touches,
  and the GFXCRUNCH LZW page sits at the documentation tree's shared level,
  where the codec's code has lived since the crate root took it.

## [0.7.0] - 2026-08-31

### Added

- **`ARCHITECTURE.md`**, and three procedures in `CONTRIBUTING.md` — adding a
  game, adding a build of the engine, adding an engine family. For anyone
  reading or changing this code rather than playing with it: how the tree
  splits into a neutral layer and the family behind the contract, how family,
  generation, build and game are kept apart, where every seam between them
  is, and every file a further game — or a further family — is named in.

- **The savegame files motionvm writes are documented**, in
  `docs/motion/savegames.md`: the header both of them carry, the layout of
  each, and
  what the version number in them means for slots already on disk — a slot
  from a build that wrote an older version is refused by name rather than
  half-read, and nothing deletes it. The README's savegame section says the
  same in two sentences. It was the one format in the project without a page,
  and the one a player's own data depends on.

### Changed

- **The window and the engine family are two layers with a contract between
  them.** The window drives any game through `motionvm-playable` and knows
  nothing else. `Playable` answers a name instead of a roster variant,
  `start` returns with the game parked in its own frame loop, the pointer's
  position and every button and key transition cross as the platform saw
  them — `key(press, down)`, with the scroll wheel, the typed stream and the
  modifiers' level state beside it for whatever reads them — and the
  picture's pixel shape is a named width and height, impossible to swap.
  What a press *means* is the family's translation: `?KEY`'s codes and the
  fifteen-slot BIOS buffer live with the engine that measured them, one
  keystroke per frame. Music is the opened game's own answer —
  `open_music(rate)` hands back the audio thread's source, or nothing for a
  game with nothing to play — with `AudioSource` joined to the engine's
  `MusicSink` inside `motionvm-motion`, the one crate that knows the engine
  and the audio stacks both. The window holds a roster of `Family` values
  and names no family crate but that front door — a rule a data-free
  boundary test reads out of the manifests — so adding a game, or a whole
  family, never edits a call site. The path is `Send`, machines included,
  so a window may open a game — the slowest thing it does — on a worker
  thread. A click during the F12 freeze now lands on the next frame instead
  of vanishing; everything else a player sees is unchanged.

- **Every crate that is MOTION's alone says so in its name.** The family's
  crates were named as if they were the whole product — `motionvm-formats`
  read MOTION's containers and nothing else, and five siblings alike. They are
  `motionvm-motion-formats`, `-forth`, `-audio`, `-engine`, `-tools` and
  `-testutil` now; a name without the family's is reserved for what does not
  belong to one family. The inspection CLI accordingly builds as
  `motionvm-motion-tools`.

- **A held mouse button reaches the scripts the way the hardware did.** The
  original's `MOUSELK` answers the live level, the shell's `_MPRESSED`
  debounces it in script, and the title's task handler reads it raw on
  purpose — holding a button skips the title a card per fade. The engine
  now delivers exactly that level, so held-button behavior is the game's
  own again; a press and release both falling between two frames is kept
  visible for the one frame the original's once-per-frame poll would have
  caught.

- **Both music stacks answer to one `Player` trait.** `rate`, `start`, `stop`,
  `playing` and `fill` were the same five methods twice, with the window
  improvising the abstraction over them; the trait is in the audio crate now,
  with the song type as its one difference. The rounding rule the two sample
  clocks shared word for word is one function, `clock::frames_to_tick` — the
  two PIT constants stay apart, because 1 193 180 and 1 193 182 are what the
  two drivers were each timed against.

- **`motionvm-motion-audio` carries its two stacks the way the rest of the tree
  carries two generations.** Its root re-exported the 32-bit game's HMI
  sequencer, FM driver and player unqualified and left the 16-bit games' whole
  PSM 2 stack in one module its own crate documentation never mentioned. Both
  are now under `m16` and `m32`, and the root keeps what genuinely belongs to
  both: the OPL3 and the register write. The 32-bit `Player` struct is
  `m32::Player` now and `psm::Sequencer` is `m16::Sequencer`; the root name
  `motionvm_motion_audio::Player` belongs to the trait.

- **The renderer takes a picture, not one generation's sprite.** Its blits took
  the 32-bit `Sprite`, so a 16-bit sprite had to be turned into one — with a
  768-byte all-zero palette invented per decode to fill a field the renderer
  never read — and every signature in a crate that serves both generations
  carried an `m32` name. `motionvm_render::Picture` is the three things a blit
  needs, both readers decode into it, and the palette a 32-bit sprite really
  does carry travels beside it instead of inside it.

- **A game's saves go in a directory of its own, and the engine is what puts
  them there.** Every game names its slots alike and the savegame magic is the
  generation's, so two games pointed at one directory would find each other's
  saves and the 16-bit ones would load them. `saves/ds2/`, `saves/enviro/` and
  the rest were the window's arrangement; now `Game::set_saves` takes the
  directory the games live under and adds the game's own name itself, so
  anything embedding the engine gets the same guarantee. Existing saves are
  where they always were. `Playable::saves` answers where they went.
  `MOTIONVM_SAVES` accordingly names the directory above a game's, not the
  game's own.

- **The container's two framings are called framings.** The reader's
  `m16::Generation` named the 1993 and 1995 header layouts, a distinction
  *inside* the 16-bit generation, while everywhere else in the project a
  generation is the 16-bit engine or the 32-bit one. It is `m16::Framing` now,
  with `Earlier` and `Later` for what were `One` and `Two`, and
  `Container::generation` is `Container::framing`. One concept owns the word.

- **The technical documentation is laid out by family.** MOTION's trees —
  `motion16/`, `motion32/`, `games/` — and its five ledgers live under
  `docs/motion/`, with the family's index in front of them; `docs/README.md`
  at the root is the map, one row per family. Links inside the record are
  unchanged — everything moved together.

- **The help text names the games off the roster.** The `GAMEDIR` paragraph
  listed the five games by hand and the folder dialog named two of their
  files; both are built from the roster now, so what the program plays is
  what its help names, and the dialog asks for the game directory in one
  sentence. A directory holding no recognizable game is refused with the
  same roster — every game and what it needs — where the closing line now
  says "a game this program does not play" rather than naming the engine.

### Fixed

- **The 32-bit machine decodes branches from the kernel it was bound to.** It
  read `IF`, `ELSE`, `UNTIL`, `WHILE`, `REPEAT` and the three loop ends off the
  ordinals `ENGINE.EXE` V0.06.06/R109 hands out, baked in at compile time,
  while the dispatch table beside it was already built from the scan of
  whatever binary the game ships. A build that numbered those words
  differently would therefore have bound correctly and then decoded an `IF` as
  something else — a run that goes wrong later and elsewhere. Both readings
  now come from the same scan, as the 16-bit machine's already did, and the
  shipped kernel agreeing with the measurement is a test.

## [0.6.0] - 2026-08-30

### Added

- **Victor Loomes – Das Spiel plays.** A fifth game, and the first of the
  16-bit engine's earlier generation: `LL.EXE` is dated 1993, three years
  before every other build in the tree. `RUN` carries it through the
  competition slide and the intro into the first room and on through all
  thirteen locations; its music plays through the rebuilt sequencer, and it
  saves and loads through its own drop-down panel.

  The container is the earlier framing, and nothing in a file says which
  framing it is — so the reader works it out by reading the file as the
  earlier one and asking whether it adds up: the offset table's first entry
  must be where the tables end and its last must be the file's length. Both
  hold for this game and for Compaq, and no later container can satisfy the
  first. What the framing changes: there is no per-segment packing field, so
  occupancy starts sixteen bytes sooner and which segments are packed is the
  generation's rather than the header's; packed sprites and fonts carry a
  ten-byte header where the later games' carry eight; and the word the later
  games use for a volume count means something else here, because this player
  has no name former for a second volume.

  The kernel is read the same way. Ordinals are handed out in registration
  order, and the player registers its core table, then a run of placeholder
  words, then its domain table — so where the domain table starts is however
  many placeholders that build registers. It is 21 here and 22 in the three
  later builds, which is why this one binds at 102 where they bind at 105.
  That count is now read out of the binary instead of assumed, which is what
  makes a fifth build cost nothing. `Inline::put_string_adr` became optional
  along the way: this kernel is two core words short of the later ones.

  Its music is fourteen bare `PLX` sections with no module around them, and
  its `MUSADL.DRV` is an older build with one entry fewer whose tables sit
  `0xd0` earlier — byte-identical tables at a different offset, which is the
  kind of difference that goes unheard rather than caught.

  Its location scheme is its own too: no module 601 and no `NEXTLOC`, but
  `NAO` and `AO` in module 605, and two modules per location — `N+100` and
  `N+20` — where the later games have three. Location 3 has neither and
  shares location 2's pair.

  Eight kernel words no other game calls are implemented from their handlers:
  `?INSIDE`, `CROUTE`, `GSCRPOS`, `SETSHADE`, `SETCYCLE`, `SYSFC`, `SYSBC` and
  `_POOR`, along with `REQUEST` and the `I'` primitive. Two of them say something worth
  writing down: `SETSHADE` stores its two arguments where nothing in either
  binary ever reads them, and `_POOR` pushes a constant zero. One is taken
  only as far as its arguments, noted in `docs/departures.md`: `SETCYCLE`'s
  palette rotation is recorded and not turned.

  `REQUEST` is the game's own message box, and it is the one word in either
  generation that stops the game to ask something. The original blocks inside
  its handler until a click; nothing here can block, so the word is asked
  again every frame until it has an answer — the machine stands on the cell
  while the frames that draw the box and read the pointer go by. That is what
  the menu's Save and Load pages are: five buttons, one per slot, laid out to
  the drawer's own arithmetic. The strings it shows are numbered from one,
  which the fetcher says outright (`0104:80e2` admits an index the count is
  not less than). A slot's three files sit under two numbers — `(700+n).blk`
  beside `n.anm` and `n.FRZ`, which is what `CTRL`'s own
  `DUP 700 + 2 AO ROT PUT`, `DUP PUTANIM`, `DUP =>PUTAS` asks for — and a
  slot written from the save box reloads in a later session to the location
  it was written in, with the same resident modules.

  The rebuilt music is held against a recording of the original for the first
  time on this generation of the driver: 744 register writes match — the
  intro's whole jingle and the start of the fade behind it. The two builds' tables being byte-identical is
  what makes that worth doing, because a reader at the wrong offsets comes
  back with plausible values rather than none.

  The whole intro is held against a recording of the original as well: all
  eight pictures it holds — the client's logo, the title card, *featuring
  Victor Loomes* over the office vignette and five more — are RGB-identical
  to a lossless capture, 0 of 64 000 pixels differing in any of them, and in
  the same order. The original needs no input to get there, so both sides
  are driven the same way and reduced by the same rule — a picture counts
  once it stands unchanged for forty steps. That is the earlier container
  framing's first evidence beyond its own files, and why
  `docs/verification.md` no longer says only this game's music has been
  compared.

- **Every game has a page that says what it is.** `docs/games/<game>/README.md`
  opens each game's documentation with what it is about, who commissioned it,
  who made it and why it exists, and indexes the four technical pages behind
  it. Each attribution names its evidence — a credits text table, a license
  file, a launcher's sign-off — and where the shipped files say nothing, as
  with Die Enviro-Kids greifen ein's client, the published record is named as
  such under its own heading rather than mixed in. Two credits tables that had
  been identified and never read are read: Dunkle Schatten 2's and Jeff
  Jet's. Dunkle Schatten 2's names the engine itself — *Basierend auf: …
  "Motion"-Präsentations-System von S. Hoffmann* — and is where the
  attribution of MOTION to DigiTales and Stefan Hoffmann now comes from, the
  game's own text table rather than an outside source; Victor Loomes remains
  the only game that gives the engine a version.

### Fixed

- **A screen's frame word belongs to the screen, and a frame runs them
  all.** `SCRCTRL` stores the id the screen runs into the screen's own
  record — the handler writes it through the current-screen accessor and
  never looks at it — and `ANIMPLAY` walks the screen slots once per frame,
  makes each screen current and runs the word it names, one after another
  inside one step; the activity test earlier in the same loop skips only the
  descriptor work, so a screen that is switched off still gets its
  controller, and a negative id is what it is in the original — no
  controller — rather than a word that failed to bind. motionvm had kept a
  single id for the whole engine and run one word per frame, which reads the
  same for the four games that give exactly one screen a controller and
  falls apart on Victor Loomes: its menu lives on a screen of its own,
  switched off, and the word that watches for the pointer reaching the top
  of the display — and switches the screen back on — runs *on that hidden
  screen*, so the menu could not appear; and of the three words its three
  screens name, the last call won under the old reading, so the game ended
  on the frame it started.

- **A `DO … LOOP` lives on the return stack, as the 16-bit engine keeps
  it.** `_LoopStart` pushes the index over the limit onto the return stack
  (`LL.EXE` `0af7:05d5`, the same code in `ENVIRO.EXE` at `12c8:01b8`),
  `LOOP` steps the index in place and leaves once limit ≤ index, popping
  both, and `LEAVE` copies the index over the limit. motionvm had kept the
  limit in a frame of its own beside the machine, which reads the same until
  bytecode reaches a loop's cells through the return stack — and Victor
  Loomes does: `STOPLOOP` in its module 605 ends an inventory scan early
  with `R> R> DROP R> DUP >R >R >R`, which under the split model overwrote
  the index with a return address and turned the early exit into a loop that
  never ends. Using the car key on the car stopped the game on its step
  limit; the four other games reach none of the difference. Along the way,
  three branch rules carried from the 32-bit engine as hypotheses are read
  out of the 16-bit handlers and hold: `WHILE` leaves on true, `UNTIL`
  branches back on zero, `=IF` on inequality.

- **Whether an empty hot area is a hole is the build's answer, not ours.**
  `?XINSIDE` passes over a hot area whose four corners are all zero in
  `ENVIRO.EXE` and `BMZ.EXE`, which follow the four corner comparisons with
  four more; `HPPLAY.EXE` and `LL.EXE` stop after the comparisons — 71
  instructions against 101 — and take such an entry as a rectangle at the
  origin. It had been the later pair's behavior for every game. It is now
  read out of the handler the game was opened with, which puts Jeff Jet on
  its own build's answer as well.

### Changed

- **`docs/` is laid out the same way throughout.** Where the two engine
  generations document the same concept the page now has the same filename and
  the same title in both — containers, sprites, blocks and the frame loop —
  and the format pages that had four names for the same table column
  (*Meaning*, *Content*, *Reading of the name*, *Description*) use one. Open
  state is `## Open questions` everywhere, and a page's own evidence section
  `## How this is checked`. Dunkle Schatten 2's four game pages, which shared
  almost no structure with the four 16-bit games', now follow the same
  skeleton: a file table with sizes and dates, a whole-container section, and
  the same heading names.

- **The README is written for someone who wants to play a game.** Which games,
  how to get a build, which files a copy needs, the controls, where savegames
  go and what to do when something does not start — in that order, with the
  developer and reverse-engineering material it had grown to carry moved to
  `docs/tools.md` and `docs/verification.md`.

- `motionvm-tools`' `is_location` accepts the modules 21 to 40. A location
  has two modules in the earlier generation, `100+N` and `20+N`, against the
  later one's three, and a location module counted as library teaches its ids
  to every other listing.

## [0.5.0] - 2026-08-29

### Added

- **Hilfe für Amajambere plays.** A fourth game, and the third on the 16-bit
  engine: the Art Department's edutainment adventure for the
  Bundesministerium für wirtschaftliche Zusammenarbeit und Entwicklung — whose
  acronym names the player, `BMZ.EXE` — given away as freeware in 1995. It is
  the cheapest title yet, because the work its container would have needed was
  done for Jeff Jet: two volumes, read since 0.4.0, and this game was already
  part of the corpus that format was measured over. What it adds to what is
  known is that **two volumes and packed items are independent choices**. Jeff
  Jet has both and Die Enviro-Kids greifen ein neither, so until this game they
  had never been seen apart; it ships two volumes with every item stored
  plainly, and splits them by kind rather than by half — volume 2 holds every
  sprite, palette and font, volume 1 everything the game runs and says.

  Its player is the build between the other two. `BMZ.EXE`'s kernel table is
  `ENVIRO.EXE`'s less exactly one word — `?SAMPLE`, the last — and
  `HPPLAY.EXE`'s plus the four `SETMOUSE*` that sit inside the table, which is
  what puts it in the middle: a word is appended to a live ordinal space, not
  inserted into it. So where Jeff Jet's build shifts 127 ordinals and would
  mis-name a table taken from elsewhere, this one shifts none and would be
  named correctly right up to the ordinal it does not have. Both are reasons to
  do what the engine already does and scan the table out of the game's own
  binary. The bytecode needs **zero new kernel words**: of the 143 it uses, the
  two neither sibling calls — `&` and `GFXVFLIP` — were implemented already.

  The game opens on its own front page rather than in a room: `RUN` writes
  `20 STARTLOC !`, enters location 20, and puts the menu up with
  `2 _INVMODE !`, which `CTRL`'s location switch is guarded against — so
  `--loc N` is honoured one click later. Twenty locations, more than either
  sibling; the per-location item table is block `300 + N` where the others put
  it at `200 + N`; and the slot probe lives in the menu's `SHOW_FILES` rather
  than in `RUN`. Savegames go to `saves/hfa/`.

### Changed

- **Savegames written by an earlier version no longer load.** A descriptor
  keeps what it shows in one field now instead of three (below), so its record
  in a `.anm` file is shorter. The format carries no version of its own, so an
  older file is refused for its length rather than by name. Nothing else about
  a save directory changes: the slots are still 701 to 705 and still live in
  `saves/<game>/`.

- **A game is named in three registers, and each has one job.** `Title::name`
  is the full title, the one a window shows — Dunkle Schatten 2 now gets its
  own, where it had been carrying the short form alone. `Title::short` is the
  title without its subtitle and is what prose, error messages and test output
  use. `Title::slug` is the key the game's files are kept under, and it never
  appears in a sentence.

  That last rule is what most of this change is: the directory name of Die
  Enviro-Kids greifen ein had become the game's name in some seventy doc
  comments and documentation lines, where it read as the engine binary beside
  it. The `Title` variants are the short form in PascalCase, so `EnviroKids`
  is now `DieEnviroKidsGreifenEin`; the slugs, the game directories, the
  environment variables and the savegame directories are untouched.

  `Title::needs` and `Title::ALL` came with it, so that the "not a game
  motionvm can open" message is built from the enum rather than kept in step
  with it by hand.

### Fixed

- **What a descriptor shows is one field now, as it is in the engine.** The
  16-bit machine keeps it in `+0x10` — `0x8000 | id` for a sprite, the bare id
  for a block, and the id of a text table when the descriptor is a text — and
  `SDSPR`, `SDBL` and `SDTB` are three words writing that one word
  (`ENVIRO.EXE` `05f1:12b6`, `05f1:11ee`, `05f1:0d7b`; the same code in
  `BMZ.EXE`). Three separate fields allowed states the engine cannot reach,
  and read `SDBL` as a picture where the original reads it as a text's table.

  Three corrections came with it. `GDSPR` and `GDBL` now answer -1 for the
  kind of picture they are not asked about (`05f1:16bd`, `05f1:168e`); `SDTB`
  no longer makes a descriptor a text on the 16-bit machine, where only
  `SDTXT` does that, while the 32-bit engine allocates its text record in
  `SDTB` and so still marks it; and the stored `kind` is gone, because the
  engine has none — it works out what a descriptor is from `+0x10` and `+0x12`
  each time it asks, and now so does this.

  The documentation had described the two fields the other way round — `+0x10`
  as the text, `+0x12`'s low byte as a length limit — which is where the
  three-field model came from. `text-rendering.md` and `descriptors.md` now
  say what the handlers do.
- **`Playable::start_location` no longer assumes every 16-bit game starts at
  location 1.** It answered a hardcoded 1 when no `NEXTLOC` was pending, which
  was true only of Die Enviro-Kids greifen ein: Jeff Jet's `RUN` enters
  location 13 and Hilfe für Amajambere's location 20. It now reads `STARTLOC`
  out of module 601 — where `RUN` leaves the answer for all three — and says
  `None` while no location has been entered yet, rather than naming one the
  game has not chosen.
- **`motionvm-tools` counted a location module as library above location 17.**
  The window that tells a location's three modules from the resident ones ran
  to 117/317/517, which was Die Enviro-Kids greifen ein's highest; Hilfe für
  Amajambere numbers to 120/320/520. Too low a bound is silent rather than
  loud — the extra modules taught their word ids to every other listing, and
  the names came out wrong with nothing said. It now runs to 20 and has a test.

## [0.4.1] - 2026-08-28

### Changed

- **`motionvm-engine` has an error type of its own.** `Game::open`,
  `titles::open` and every `Playable` method answered with
  `Box<dyn std::error::Error>`, so a caller could not tell "this directory
  holds no game" from "this sprite is truncated" without matching on message
  text. `motionvm_engine::Error` is a fourteen-variant enum with the shape the
  three crates below it already have, and `motionvm_engine::Result<T>` beside
  it. Every message it prints is the one that was printed before, character
  for character. **A breaking change for anything that named the old return
  type**; nothing in this repository did but the crate itself.

### Fixed

- **A damaged container no longer aborts `motionvm-tools extract`.** Twelve
  assertions sat on the result of looking up an item the container's index
  claimed to hold, where the game's own loader skips such a slot and plays on.
  `extract` now writes what is really there and says how many items the index
  over-claimed, instead of panicking with a message naming nothing the user
  can act on.
- **Errors from a damaged game directory name the file they are about.** An
  unreadable `ENGINE.EXE` reported "read of 4 bytes at 0x3c past end of
  17-byte buffer", which is true of every file in the directory; a broken
  container reported "RSC container: file is only 0 bytes". Both now open with
  the path.
- **Three file sizes and one word count in the documentation.** `HPPLAY.EXE`
  is 166 KB and was given as 162, `MUSADL.DRV` is 4 KB and was given as 5 in
  both of its tables, and the documentation index labelled the 16-bit
  generation with `ENVIRO.EXE`'s 233 kernel words where the older
  `HPPLAY.EXE` has 228.

## [0.4.0] - 2026-08-28

### Added

- **Jeff Jet - Abenteuer InfoHighway plays.** A third game, and the second on
  the 16-bit engine: Promotion Software's advergame for Hewlett-Packard, whose
  player `HPPLAY.EXE` is an older build of the one Die Enviro-Kids greifen ein
  ships. It asks nothing new of the machine — the same words, the same
  authoring template, the same save scheme — and everything of it that is its
  own is in the container. It comes on **two volumes**, and every item of it is
  **LZW-packed**: the occupancy word turns out to be a volume bitmask,
  `1 << (volume - 1)`, a secondary volume is nothing but an offset table over
  the same slots and then its items, and seven words of the header say per
  segment whether that segment's items carry a GFXCRUNCH header. The item
  loader reads exactly those words and branches on them (`HPPLAY.EXE` file
  `0x400c`, and the same routine at `0x401f` in `ENVIRO.EXE`, whose seven are
  all zero). Both volumes are opened when the game is opened, and every packed
  item is unfolded there — 2 459 890 bytes into 8 412 811 — so nothing else in
  the tree can tell a packed game from a plain one. A missing second volume is
  refused by name rather than silently emptying every palette, both fonts and
  the font reference table, which is what that volume holds.

  The kernel table has to come from the game's own binary, and now visibly so:
  this build has five words fewer, and because four of them were later inserted
  into the middle of a live ordinal space, 127 of its 146 domain words sit four
  ordinals below their namesakes in the other build. A table taken from one
  game and used on the other would bind, and would then call the wrong words.

- **The two header words the DATA container never explained are explained.**
  They are the volume count and the number of spare offset-table entries, and
  the sixteen bytes behind them that were documented as zero are seven
  per-segment packed flags and a spare. Measured over Die Enviro-Kids greifen ein,
  Jeff Jet, Hilfe für Amajambere and Eddy M.: the flags and the shape of the
  items agree in all 28
  segments.

### Changed

- **No game is the default any more.** With three games the shorthands that
  meant Dunkle Schatten 2 had stopped being shorthands and become a claim:
  `just run` is now `just run-ds2` beside `run-enviro` and `run-jeffjet`,
  `Game` is `Game<m32::Vm>` or `Game<m16::Vm>` with no default machine,
  `sound::open` is `sound::open_motion32` beside `open_motion16`, and every
  test that needs a game's files is named for it — `ds2_scenes.rs`,
  `enviro_psm.rs`, `gamedata_jeffjet.rs`. Nothing behaves differently; the
  names simply stop saying that one game is the one you get when you do not
  say which.

- **The GFXCRUNCH LZW codec moved to the crate root** of `motionvm-formats`,
  from `m32`. Both generations pack their containers with it, bit for bit, and
  the crate root is where what the two share lives.

- **Dunkle Schatten 2 keeps its savegames in `saves/ds2/`.** Each game
  has a directory of its own under the platform data directory, named for
  the game, rather than Die Enviro-Kids greifen ein sitting in a
  subdirectory of Dunkle Schatten 2's. The two still cannot share one —
  they name their slots alike, `701` through `705`, and each asks at
  start-up whether a slot exists — and none is now the special case; Jeff Jet
  keeps its own in `saves/jeffjet/`. The path in use is printed at start-up,
  as ever.

  **Upgrading from 0.3.1 or earlier:** this game's slots were in `saves/`
  itself. Move `saves/701.*` through `saves/705.*` into `saves/ds2/` to keep
  them — the load page lists what is in the directory it is given, so slots
  left behind stop appearing.

### Fixed

- **A MOTION 32-bit game that is not Dunkle Schatten 2 is refused by name.**
  `NNN.RSC` beside an `ENGINE.EXE` says which generation of the engine made a
  game, not which game it is, and the words the bootstrap names do not say it
  either: `START`, `STARTUP` and `INCLLOC` come from the authoring template, so
  another MOTION game exports them from the same modules. Checker 2000 — on the
  earlier build `V0.04.15/R78` — has all three. What it does not have is module
  2's `_STARTLOC` and `_NEXTLOC`, the script variables this game's own compiler
  named and this runtime reads, and those are what the opener asks for now.
  Such a directory used to pass detection, name the window after Dunkle
  Schatten 2 and then fail inside the VM with a `StackUnderflow`; it now stops
  before that and says which game's script the directory does not hold. The
  same message covers an incomplete copy of this game, which reaches the same
  state and cannot be told from the other case by the data.

- **The "not a game" messages say what they mean.** MOTION made more games than
  the three here, so a directory motionvm cannot open is not thereby "not a
  MOTION game directory" — one holding any of the others reached exactly that
  sentence. It now reads *"is not a game motionvm can open"* and lists what each
  of the three would need; a directory that is one of them but incomplete is
  named for the game it is missing files of.

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
behaves as the engine did. See `docs/verification.md`.

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

[Unreleased]: https://github.com/wdominik/motionvm/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/wdominik/motionvm/releases/tag/v0.8.0
[0.7.2]: https://github.com/wdominik/motionvm/releases/tag/v0.7.2
[0.7.1]: https://github.com/wdominik/motionvm/releases/tag/v0.7.1
[0.7.0]: https://github.com/wdominik/motionvm/releases/tag/v0.7.0
[0.6.0]: https://github.com/wdominik/motionvm/releases/tag/v0.6.0
[0.5.0]: https://github.com/wdominik/motionvm/releases/tag/v0.5.0
[0.4.1]: https://github.com/wdominik/motionvm/releases/tag/v0.4.1
[0.4.0]: https://github.com/wdominik/motionvm/releases/tag/v0.4.0
[0.3.1]: https://github.com/wdominik/motionvm/releases/tag/v0.3.1
[0.3.0]: https://github.com/wdominik/motionvm/releases/tag/v0.3.0
[0.2.0]: https://github.com/wdominik/motionvm/releases/tag/v0.2.0
[0.1.1]: https://github.com/wdominik/motionvm/releases/tag/v0.1.1
[0.1.0]: https://github.com/wdominik/motionvm/releases/tag/v0.1.0
