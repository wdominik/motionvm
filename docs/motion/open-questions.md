[← Documentation index](README.md)

# Open Questions

*Both generations of the engine, and every game motionvm plays — the one ledger of what is not yet known about the original. Each entry names the generation and, where it is one game's, the game. Deliberate divergences are a different record and live in [departures](departures.md).*

A consolidated list of everything that is documented as unknown, unverified,
or hypothetical, for both generations of the engine. Each topic page carries
its own short "Open questions" section; this page collects them with links.
The sections down to *The music's clock* concern the **32-bit engine** and
Dunkle Schatten 2; [MOTION 16-bit](#motion-16-bit) collects the 16-bit
engine's and its four games'. An entry that names a game is that game's.

## Remaining unmapped kernel words and handler regions

The audio words are fully mapped ([Audio](motion32/engine/audio.md)); the
interaction machine is mapped in skeleton and key paths
([Interaction machine](motion32/engine/interaction.md),
[Dialogue machine](motion32/engine/dialogue-machine.md)); `MOUSEINFO` (24 args),
`?XINSIDE`, `CCALCINV`, `ADDTOINV`/`SUBFROMINV`, `CTRL`, `DELAY`,
`QUITANIM`, `EXIST`, `FREEZESCR`, `GET` are handler-read. What remains:

| Area | Open |
|---|---|
| Interaction | What script code sets and clears the click gates `_ORDER+0x134`/`+0x1CC`. (**All eight `EXECORDER` verbs are read and built**, as are `TEXTTOPERSON` `0x7b053` and the keyword search of verbs 6 and 7. The verb menu too: the right click, modes 2–8, `GMSHOWMENU`/`GMREMOVEMENU`/`HIGHLIGHTORDERS`/`ANIMATEORDERS`/`CHOOSEORDERS`/`VERBOFSLOT`/`SPRRANGEOFVERB` and the epilogue `0x7eae2`) |
| Dialogue | Modes 15, 17, 18; the mid-conversation right-click menu. (Branch actions 5/6 and the name lookup `0x61596` are read and built; one recorded [departure](departures.md): the rebuild walks modules by number, the original's table at `0xEE6D0` by entry order. How `=>GET`/`=>ERASE` assign entries is read — first free slot, freed in place — and modeled, see [Savegames](motion32/engine/savegames.md)) |
| Walking | Command 1007's call path (no queue in the game writes one); whether the two line kinds mean more than which end anchors the scale ramp |
| Animation | The native layer: `ANIMPLAY` (meaning of the ten values the game pushes — the handler pops none), `ANIMSIM`. (`PUTANIM` `0x6dd87` and `GETANIM` `0x6e355` are read: the file layout, the tags `0x3E9`/`0x3EA`/`0x3EB`/`0x3EC`/`0x3EF` and the five globals are in [Savegames](motion32/engine/savegames.md). What three of them — `0xDB4AC`, `0xDB4B4`, `0xDB4B8` — and the 180-byte table at `0xF2594` mean is still open) |
| Text | The layout fields `+0x24`/`+0x2C` (glyph-origin shift); the third callback-address check `0x68367`. **Vertical centering**: `0x2588e` and the three instructions after it compute the height with no gap term, but a real run puts a caption at 166 where that arithmetic gives 167, so the code centers on the gap-inclusive height. What the instruction reading is missing — most likely that `schrift[+2]` already carries the leading — is unread. See [Text rendering](motion32/engine/text-rendering.md) |
| Module state | `INTERPRET$` — the developer console, its only caller. (`=>PUTAS`/`=>GETAS`/`PUT`/`GET`/`EXIST` are read and built; so is the descriptor-slot order `=>GET`/`=>ERASE` produce. See [Savegames](motion32/engine/savegames.md)) |
| Presenting | **The presenter is `0x1457D`**: it walks the 8×8 update map at `0xE7D84`, copies the marked tiles from the software surface (`0xE7D7C`, allocated at 0x14285) into video memory and clears the map. `0x18436` marks, `0x146D3` clears; twenty-odd callers include the fades, the mouse layer (0x25432/0x25622) and 0x270d6. See [Transitions](motion32/engine/transitions.md). Still open: why the inventory bar's real sprites stay invisible before the intro's first fade, and the drawer tail 0x69DB6–0x6A30A |
| Misc | `TXTSTAT` record layout, `REQUEST`/`SDIAL`. (`ADDMESSPIPE` is read and built: it appends a 0x22-byte record to the deferred-change queue at `_ORDER+0x1C0`, the same one the branch nodes write.) (`?INVINCL` is read and built.) Location 12 cannot be entered: its location-table entry is uninitialized in the shipped data, so the jump goes nowhere in the original too |

See [Kernel words](motion32/vm/kernel-words.md).

## Virtual machine

- **Module 0** — the kernel's base module, created at runtime, not a file.
  About 2600 call cells point into it and remain unresolved, and what its data
  area holds is not knowable from the outside, so reads of it answer 0 and are
  reported (see [Execution model](motion32/vm/execution-model.md)).
  ([Execution model](motion32/vm/execution-model.md))
- **Table 1's ordinal base** — its compiling words never appear as cells.
  ([Threaded code](motion32/vm/threaded-code.md))
- **`_LoopStart`'s ordinal** — arithmetic suggests 389; unconfirmed.
  ([Threaded code](motion32/vm/threaded-code.md))
- **`_ChElseDup`** — unmeasured; does not occur in the game's modules.
- **`/LOOP` termination rule** — possibly unsigned (`_ULoopEnd`); only the
  branch encoding is measured. ([Word semantics](motion32/vm/word-semantics.md))
- **`LEAVE` continuation point**; **negative-operand `/` and `MOD`**;
  **division by zero**; **`RANDOM`'s generator**.
  ([Word semantics](motion32/vm/word-semantics.md))

## Timing

- **The exact wall-clock frequency of the tick base** — 200 Hz fits
  every observed formula (frame divider, sample durations, curtain
  delays) but is an inference, not a measured clock.
  ([Game loop](motion32/engine/game-loop.md))
- **The master timer frequency** behind the audio timers (≈1 kHz
  inferred; consistent with a 200 Hz-derived chain).
  ([Audio](motion32/engine/audio.md))

## Engine behavior

- **`SCRVPOS`/`SCRPOS` interpretation** is a hypothesis (strongly supported
  by the pixel-exact title composition, unconfirmed in code).
  ([Screens](motion32/engine/screens.md))
- **Fade mode 2** — a branch exists, nothing invokes it.
  ([Transitions](motion32/engine/transitions.md))
- **`NEWSETDESC`'s fifth argument** — popped and discarded everywhere
  observed. ([Descriptors](motion32/engine/descriptors.md))
- **Whether a 32-bit descriptor stores its type.** `+0x02` is read as a type
  field — 1 group, 2 sprite, 3 image, 4 text — and no handler is cited for
  writing or reading it. The equivalent reading of the 16-bit engine turned
  out to be backwards: there the pair `+0x10`/`+0x12` says what a descriptor
  is, worked out on every ask rather than stored. Settling it means reading
  `SDSPR` (`0x71715`), `SDBL`, `SDTXT` (`0x71b4f`) and `SDTB` (`0x71d45`) for
  what they write at `+0x02`.
  ([Descriptors](motion32/engine/descriptors.md))
- **Descriptor fields still known by name only** — `SDBUF`,
  `SDSTARTLINE`, `SDALINES`, `SDTRANS`, `SDSHADE`, `SDINSERT`; modes
  `SDNORM`/`SDPOS`. ([Descriptors](motion32/engine/descriptors.md))
- **Descriptor flag `0x10`** — raised beside the dirty bit by `0x6ab6e` and
  cleared with it by the drawer, it picks between two blitters:
  `0x27765`/`0x29ae9` against `0x273e8`/`0x299e1`, whose destination is the
  display's software surface at `0xE7D7C` (`0x69331`, `0x697cd`). What the
  choice is for is unread. ([Screens](motion32/engine/screens.md))
- **Alt-F10** — the readme documents it as an immediate exit. What `?KEY`
  answers for it is settled — `0x2383d` maps the combination's scan code
  `0x71` back onto F10 and marks it, giving `0x944` — but no module tests
  that value, so where the engine acts on it is still unmapped.
  ([Other files](games/ds2/other-files.md))
- **The tune loop flag** — set by `STARTTUNE`, never observed being read;
  looping may be the driver default. ([Audio](motion32/engine/audio.md))
- **The sample-start words' second argument** — most plausibly a loop
  count. ([Audio](motion32/engine/audio.md))
- **Dialogue script-record fields** beyond id/line count/text table (the
  *engine-side* conversation layout is mapped — see
  [Dialogue machine](motion32/engine/dialogue-machine.md)).
  ([Objects and flags](games/ds2/library/objects-and-flags.md))

## Game data

- **Story-flag assignment** — flag numbers 1–110 are in use; which flag
  marks which plot event is unmapped.
  ([Game structure](games/ds2/game-structure.md))
- **The click-area record** beyond its `+16` route number. The route and
  extended-route layouts are mapped off `CROUTE`, as are the 64-byte item
  record and the dialogue records. ([Blocks](motion32/formats/blocks.md))
- **Who installs the translucency/shade blocks 50–54** — no script calls
  `SETTRANS`/`SETSHADE`. ([Blocks](motion32/formats/blocks.md))
- **Stale location-table entries** — location 12's entry is garbage though
  module 312 exists; module 330's scene macro is unreferenced; entry 30
  duplicates the title. ([Blocks](motion32/formats/blocks.md))
- **`TI1`–`TI50` story-event numbering** and the duplicated Karsten walk
  modules (9, 19, 212). ([Module map](games/ds2/module-map.md))

## File formats

- **GFX8 leading bytes** — they duplicate color 0's red and green
  channels (verified on all 1678 Dunkle Schatten 2 sprites); *why* is unknown.
  ([GFX8 sprites](motion32/formats/sprites.md))
- **The packed-length field** at sprite offset `+782` (correct only for
  single-block streams). ([GFX8 sprites](motion32/formats/sprites.md))
- **GFX16** — a format slot with zero instances; layout unknown.
  ([RSC containers](motion32/formats/container.md))
- **SCR fields** `0x14`/`0x18` (authoring addresses), `0x24`/`0x28`
  (high-water marks), and the word-header link field `+12`.
  ([Script modules](motion32/formats/script-modules.md))
- **RSC reserved bytes** at `0x18`.
  ([RSC containers](motion32/formats/container.md))
- **FNT fields** `+2` (duplicate size) and `+4` (packed size) — redundant;
  read by the engine? ([Fonts](motion32/formats/fonts.md))
- **HMI internals** — the four pad bytes in the tempo and time-signature
  maps, and whether `F0`'s length includes a trailing `F7` (no shipped song
  has a sysex event). The event encoding itself is read.
  ([HMI songs](motion32/formats/hmi.md))
- **RSC.INF** — header parameters partially read; the 22-byte record
  layout is unmapped. ([Other files](games/ds2/other-files.md))

## The FM driver

Three things `fmmidi3.com` does not answer for itself: **which register bank
is which side** (only the wiring can settle it, not a recording), **ordinal 1**
and why ordinals 6 and 8 share a body while the fuller reset at `0x0793` is
unreachable, and **`chan[8]`** in the engine's channel record, which is not the
channel number. Also one measured divergence in voice allocation under a
ten-note tick. All four are set out in
[The FM driver](motion32/engine/fm-driver.md#open-questions).

## The music's clock

The sequencer's timer rides the sound layer's 1500 Hz master rather than getting
one of its own, and a song asking for 120 Hz gets **thirteen** master ticks per
tick — 115.45 Hz. That thirteen is measured against the recording, not read:
`0x8CF23` computes a 16.16 ratio between the two divisors, and the timer service
that consumes it has not been followed. Twelve and a half is what the division
gives; what rounds it up, and whether the remainder is dropped once or per tick,
is unread. The measurement is unambiguous — over 150 notes, thirteen holds every
one within 41 ms where 120 Hz runs 313 ms ahead — so this is a question about
*why*, not about *what*.

The voice-allocation divergence measured in the same recording is with the
driver: see [The FM driver](motion32/engine/fm-driver.md#open-questions).

## MOTION 16-bit

The 16-bit engine is documented from its shipped files and from its
handlers as far as the intro, the locations and the conversations needed
them read. Almost all of that reading was done on `ENVIRO.EXE` with Die
Enviro-Kids greifen ein; `HPPLAY.EXE` and `BMZ.EXE` are the same player in
older builds and have been read only where they differ; `LL.EXE`, three
years older again, has been read only where the game it ships with reaches
something the others do not. What is open:

- **`?KEY`'s own translation.** The handler at `12c8:063c` is listed by
  name and address only; the extended-key marker, the modifier bits and any
  Alt table are unread, and the 32-bit engine's translation is applied in
  its place (the ledger records the departure). Reading it settles whether
  the two engines' keyboards really agree past the plain character byte.

- **Whether the driver clears the playing flag when a song plays out.**
  The sound module's flag (`ENVIRO.EXE` `ds:18f4`, `LL.EXE` `ds:13dc`) is
  set when the driver starts a song and cleared by the stop routine, and
  while it is clear `ENDTUNE` neither fades nor waits. Whether the driver's
  own end-of-song path clears it too is unread; it decides whether a stop
  asked for after a non-looping song has played out still holds the game
  half a second. Every stop the games ask for comes over a playing song,
  and Victor Loomes' jingle, played once, still gets its fade in the
  recording — so at least there the flag stood.
  ([PSM 2 music](motion16/formats/psm-music.md))
- **What *Motion 1.0* is, and who Michel "Babe" Stigler and EGO Software
  are.** Victor Loomes' credits close on *Erstellt unter · Motion 1.0 ·
  Michel "Babe" Stigler · EGO Software*. Two of the five games name the
  engine — Dunkle Schatten 2's credits carry *"Motion"-Präsentations-System
  von S. Hoffmann* — but only this one gives it a version, and `1.0` sits
  three years before the builds this documentation is written from. Whether
  the two names after the version belong to the engine or to the game is not
  something the flat credits list says: the same table gives `Graphik` two
  values. One thing is no longer open — a *Michael Stigler* is credited in
  Jeff Jet, the same studio's next game, for the bulk of its dialogue and for
  support, so the name is one Promotion Software worked with rather than an
  outside attribution of the tool. That does not say which of the two the
  Victor Loomes entry means.

- **Why the hole case was added.** `?XINSIDE` passes over an all-zero hot
  area in `ENVIRO.EXE` and `BMZ.EXE` and not in `HPPLAY.EXE` or `LL.EXE`, and
  the two that do are not the two later ones by date — Amajambere's build
  has it and Jeff Jet's does not. What the tables looked like that made it
  worth adding is not established.
  ([LL.EXE](motion16/engine/ll-exe.md))
- **What the fade after the intro's jingle runs into.** The rebuilt player
  matches the original's register stream for 744 writes — block 7 from first
  note to last, and the start of the fade `ENDTUNE` asks for — and the
  recording taken under DOSBox-X carries 33 seconds more. What plays there is
  not established; covering it needs the session modelled past `ENDTUNE`.
- **What `SETSHADE` was for.** It pops two and stores them at `ds:0x150` and
  `ds:0x152` (`0104:2824`), and no other instruction in `LL.EXE` or
  `ENVIRO.EXE` mentions either address. `RUN` calls it once.
- **The 32-byte `PSMCFG.DAT`** the older sound setup writes, which `LL.EXE`
  reads (the name is at file `0x14ee9`). The later games' 36-byte
  `PSMCFG4.DAT` layout is known and does not apply.

- **The interpreter loop** at file `0x174f7` and the nested-run sentinel
  `0xfffd` — from a first reading.
  ([Execution model](motion16/vm/execution-model.md))
- **The descriptor drawer** `016a:0aac` beyond its two paths — the
  save-under's save step, clipping and keying; the buffer priority byte;
  `SDBLK`.
  ([Off-screen buffers](motion16/engine/buffers.md))
- **The clock's speed class** (`DS:0x10ce`) and who fills the mouse
  record. ([Boot and frame loop](motion16/engine/game-loop.md))
- **The rest of the descriptor and screen structures**, `NEWSETDESC`'s
  fifth argument (always 15). `XGFXVFLIP`'s third argument is read
  (a count, `05f1:221b`); its mirror axis is still assumed left-right.
  ([Descriptors](motion16/engine/descriptors.md),
  [Screens and the draw chain](motion16/engine/screens.md),
  [Transitions](motion16/engine/transitions.md))
- **`_PutStringAdr`'s skip** — whether the handler follows the compiler's
  padding rule; `_ChElseDup`'s semantics.
  ([Threaded code](motion16/vm/threaded-code.md))
- **The `link` field** of a word header. ([Script modules](motion16/formats/script-modules.md))
- **Block fields** — the route links, the extended-route values, the click
  `kind`, the item record's `DR`/`DX`/`DY`/`EXIT`/`ORDER`, and the
  animation-catalog record layout. ([Blocks](motion16/formats/blocks.md))
- **The digital music renderer** — the `DMA*.DRV` drivers play the same
  tunes from the modules' `SM8` sample sections (measured: digital-only
  configuration still plays the music, sampled), picked by index and
  installed at IRQ 7/DMA 1 (`10d8:0110`, `10d8:01f5`); their internals
  are unread and motionvm plays the Ad Lib rendition only. `MUSADL.DRV`'s
  callback protocol and the Volume entry's negative selectors are present
  and unexercised by the game.
- **The original's save bytes** — `PUTANIM`/`GETANIM` (file `0xc46d`,
  `0xcb1e`) are unread and no save of the original is on hand; the rebuild
  writes a layout of its own, as for the 32-bit game.
  ([Boot and frame loop](motion16/engine/game-loop.md#saves))
- **The sprite blit's clipped odd widths** — the blit is read (index 0
  is the key, skipped per pixel), but its row-start arithmetic on a
  top-clipped sprite of non-eightfold width reads as a shear.
  ([Sprites](motion16/formats/sprites.md))
- **`SFT` with a non-zero argument** — the game only passes 0; the
  drawer's rules themselves are read
  ([Text rendering](motion16/engine/text-rendering.md)).
- **The spare `u32` entries** at the end of a container's offset table —
  their count is in the header at 20 and they are zero in every shipped
  container. ([The DATA container](motion16/formats/container.md))
- **Jeff Jet's two contradictory GFX slots** — 1319 and 1848 carry an
  occupancy word naming a volume whose offset table gives them a length of
  zero: the container saying a sprite is there and then not having one. One
  sits on each volume (1319 flagged for `DATA.-1-`, 1848 for `DATA.-2-`), both
  in the middle of a run of occupied slots, with `off[i] == off[i+1]` — the
  ordinary encoding of an empty slot, beside an occupancy word that says
  otherwise. Whether that is an authoring leftover, a sprite deleted without
  its flag being cleared, or a meaning nobody has read is unread; the loader
  is not affected, because what has bytes is what is there. Die Enviro-Kids
  greifen ein's container has none, and `motionvm-motion-tools info` reports the
  count. ([The DATA container](motion16/formats/container.md),
  [Resource inventory](games/jeffjet/inventory.md))
- **The volume-change path** — the player carries *"Bitte Diskette #d
  einlegen!"* and *"Datenblock <#s> nicht gefunden."* and what it does with
  a volume that is not in the drive has not been read; motionvm opens every
  volume at once and never reaches it
  ([Departures](departures.md#the-16-bit-machine)).
  ([The DATA container](motion16/formats/container.md))
- **The animation catalogs' first field** — `0xFFFF` in several of Jeff
  Jet's catalogs 125–182 where Die Enviro-Kids greifen ein's hold small
  integers. ([Blocks](motion16/formats/blocks.md))
- **`A.DAT`** — unreferenced by Die Enviro-Kids greifen ein's player,
  117 192 bytes of high entropy. ([Other files](games/enviro/other-files.md))

- **What `NEWSETDESC` does when a screen already holds a hundred descriptors**
  — `05f1:0ad4` jumps past every pop *and* the push, so six values stay on the
  data stack and no handle comes back. motionvm pops cleanly and answers a
  handle, which is a divergence from the hundred-and-first descriptor on;
  nothing in the shipped games gets near it.
  ([Descriptors](motion16/engine/descriptors.md))

- **Hilfe für Amajambere's location 7 has no item table** — block 307 is not
  in the container, and its occupancy word agrees, while `INCLLOC` loads block
  `300 + N` for every location it enters. The room is reachable in play:
  location 11 sets `7 NEXTLOC !` on one of its branches. The original does not
  recover — `GET` in `BMZ.EXE` (`12bb:0e91`) tests the resolved block for null
  and answers a missing one with error `0xE`, *Fehler diverser Natur (FDN)*,
  leaving `_LDITEM` unfilled, so the room would come up holding the previous
  location's items. Whether the room was cut late, or its table lost in
  mastering, is unread. ([Game structure](games/hfa/game-structure.md),
  [departures](departures.md#the-16-bit-machine))
- **Why Hilfe für Amajambere's occupancy words over-claim** — it flags whole
  segment ranges rather than single slots, so 1534 of them name a volume that
  gives them no bytes, against Jeff Jet's two and Die Enviro-Kids greifen ein's
  none. It looks like an authoring habit — reserve the segment, fill what there
  is — but nothing has been read that says so, and the loader is unaffected
  because what has bytes is what is there.
  ([The DATA container](motion16/formats/container.md),
  [Resource inventory](games/hfa/inventory.md))
- **What Hilfe für Amajambere's 70 animation catalogs hold** — blocks 125–212,
  unclassified, where the sibling games' have been mapped.
  ([Blocks](motion16/formats/blocks.md))

## See also

- [Departures](departures.md) — the deliberate differences, as opposed to the
  unknowns
- [Documentation index](README.md)
