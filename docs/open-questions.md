[← Documentation index](README.md)

# Open Questions

A consolidated list of everything that is documented as unknown, unverified,
or hypothetical. Each topic page carries its own short "Open questions"
section; this page collects them with links.

## Remaining unmapped kernel words and handler regions

The audio words are fully mapped ([Audio](engine/audio.md)); the
interaction machine is mapped in skeleton and key paths
([Interaction machine](engine/interaction.md),
[Dialogue machine](engine/dialogue-machine.md)); `MOUSEINFO` (24 args),
`?XINSIDE`, `CCALCINV`, `ADDTOINV`/`SUBFROMINV`, `CTRL`, `DELAY`,
`QUITANIM`, `EXIST`, `FREEZESCR`, `GET` are handler-read. What remains:

| Area | Open |
|---|---|
| Interaction | What script code sets and clears the click gates `_ORDER+0x134`/`+0x1CC`. (**All eight `EXECORDER` verbs are read and built**, as are `TEXTTOPERSON` `0x7b053` and the keyword search of verbs 6 and 7. The verb menu too: the right click, modes 2–8, `GMSHOWMENU`/`GMREMOVEMENU`/`HIGHLIGHTORDERS`/`ANIMATEORDERS`/`CHOOSEORDERS`/`VERBOFSLOT`/`SPRRANGEOFVERB` and the epilogue `0x7eae2`) |
| Dialogue | Modes 15, 17, 18; the mid-conversation right-click menu. (Branch actions 5/6 and the name lookup `0x61596` are read and built; one recorded [departure](departures.md): the rebuild walks modules by number, the original's table at `0xEE6D0` by entry order. How `=>GET`/`=>ERASE` assign entries is read — first free slot, freed in place — and modeled, see [Savegames](engine/savegames.md)) |
| Walking | Command 1007's call path (no queue in the game writes one); whether the two line kinds mean more than which end anchors the scale ramp |
| Animation | The native layer: `ANIMPLAY` (meaning of the ten values the game pushes — the handler pops none), `ANIMSIM`. (`PUTANIM` `0x6dd87` and `GETANIM` `0x6e355` are read: the file layout, the tags `0x3E9`/`0x3EA`/`0x3EB`/`0x3EC`/`0x3EF` and the five globals are in [Savegames](engine/savegames.md). What three of them — `0xDB4AC`, `0xDB4B4`, `0xDB4B8` — and the 180-byte table at `0xF2594` mean is still open) |
| Text | The layout fields `+0x24`/`+0x2C` (glyph-origin shift); the third callback-address check `0x68367`. **Vertical centering**: `0x2588e` and the three instructions after it compute the height with no gap term, but a real run puts a caption at 166 where that arithmetic gives 167, so the code centers on the gap-inclusive height. What the instruction reading is missing — most likely that `schrift[+2]` already carries the leading — is unread. See [Text rendering](engine/text-rendering.md) |
| Module state | `INTERPRET$` — the developer console, its only caller. (`=>PUTAS`/`=>GETAS`/`PUT`/`GET`/`EXIST` are read and built; so is the descriptor-slot order `=>GET`/`=>ERASE` produce. See [Savegames](engine/savegames.md)) |
| Presenting | **The presenter is `0x1457D`**: it walks the 8×8 update map at `0xE7D84`, copies the marked tiles from the software surface (`0xE7D7C`, allocated at 0x14285) into video memory and clears the map. `0x18436` marks, `0x146D3` clears; twenty-odd callers include the fades, the mouse layer (0x25432/0x25622) and 0x270d6. See [Transitions](engine/transitions.md). Still open: why the inventory bar's real sprites stay invisible before the intro's first fade, and the drawer tail 0x69DB6–0x6A30A |
| Misc | `TXTSTAT` record layout, `REQUEST`/`SDIAL`. (`ADDMESSPIPE` is read and built: it appends a 0x22-byte record to the deferred-change queue at `_ORDER+0x1C0`, the same one the branch nodes write.) (`?INVINCL` is read and built.) Location 12 cannot be entered: its location-table entry is uninitialized in the shipped data, so the jump goes nowhere in the original too |

See [Kernel words](vm/kernel-words.md).

## Virtual machine

- **Module 0** — the kernel's base module, created at runtime, not a file.
  About 2600 call cells point into it and remain unresolved, and what its data
  area holds is not knowable from the outside, so reads of it answer 0 and are
  reported (see [Execution model](vm/execution-model.md)).
  ([Execution model](vm/execution-model.md))
- **Table 1's ordinal base** — its compiling words never appear as cells.
  ([Threaded code](vm/threaded-code.md))
- **`_LoopStart`'s ordinal** — arithmetic suggests 389; unconfirmed.
  ([Threaded code](vm/threaded-code.md))
- **`_ChElseDup`** — unmeasured; does not occur in the game's modules.
- **`/LOOP` termination rule** — possibly unsigned (`_ULoopEnd`); only the
  branch encoding is measured. ([Word semantics](vm/word-semantics.md))
- **`LEAVE` continuation point**; **negative-operand `/` and `MOD`**;
  **division by zero**; **`RANDOM`'s generator**.
  ([Word semantics](vm/word-semantics.md))

## Timing

- **The exact wall-clock frequency of the tick base** — 200 Hz fits
  every observed formula (frame divider, sample durations, curtain
  delays) but is an inference, not a measured clock.
  ([Game loop](engine/game-loop.md))
- **The master timer frequency** behind the audio timers (≈1 kHz
  inferred; consistent with a 200 Hz-derived chain).
  ([Audio](engine/audio.md))

## Engine behavior

- **`SCRVPOS`/`SCRPOS` interpretation** is a hypothesis (strongly supported
  by the pixel-exact title composition, unconfirmed in code).
  ([Screens](engine/screens.md))
- **Fade mode 2** — a branch exists, nothing invokes it.
  ([Transitions](engine/transitions.md))
- **`NEWSETDESC`'s fifth argument** — popped and discarded everywhere
  observed. ([Descriptors](engine/descriptors.md))
- **Descriptor fields still known by name only** — `SDBUF`,
  `SDSTARTLINE`, `SDALINES`, `SDTRANS`, `SDSHADE`, `SDINSERT`; modes
  `SDNORM`/`SDPOS`. ([Descriptors](engine/descriptors.md))
- **Descriptor flag `0x10`** — raised beside the dirty bit by `0x6ab6e` and
  cleared with it by the drawer, it picks between two blitters:
  `0x27765`/`0x29ae9` against `0x273e8`/`0x299e1`, whose destination is the
  display's software surface at `0xE7D7C` (`0x69331`, `0x697cd`). What the
  choice is for is unread. ([Screens](engine/screens.md))
- **Alt-F10** — the readme documents it as an immediate exit. What `?KEY`
  answers for it is settled — `0x2383d` maps the combination's scan code
  `0x71` back onto F10 and marks it, giving `0x944` — but no module tests
  that value, so where the engine acts on it is still unmapped.
  ([Other files](formats/other-files.md))
- **The tune loop flag** — set by `STARTTUNE`, never observed being read;
  looping may be the driver default. ([Audio](engine/audio.md))
- **The sample-start words' second argument** — most plausibly a loop
  count. ([Audio](engine/audio.md))
- **Dialogue script-record fields** beyond id/line count/text table (the
  *engine-side* conversation layout is mapped — see
  [Dialogue machine](engine/dialogue-machine.md)).
  ([Objects and flags](library/objects-and-flags.md))

## Game data

- **Story-flag assignment** — flag numbers 1–110 are in use; which flag
  marks which plot event is unmapped.
  ([Game structure](engine/game-structure.md))
- **The click-area record** beyond its `+16` route number. The route and
  extended-route layouts are mapped off `CROUTE`, as are the 64-byte item
  record and the dialogue records. ([Blocks](formats/block.md))
- **Who installs the translucency/shade blocks 50–54** — no script calls
  `SETTRANS`/`SETSHADE`. ([Blocks](formats/block.md))
- **Stale location-table entries** — location 12's entry is garbage though
  module 312 exists; module 330's scene macro is unreferenced; entry 30
  duplicates the title. ([Blocks](formats/block.md))
- **`TI1`–`TI50` story-event numbering** and the duplicated Karsten walk
  modules (9, 19, 212). ([Module map](engine/module-map.md))

## File formats

- **GFX8 leading bytes** — they duplicate color 0's red and green
  channels (verified on all 1678 sprites); *why* is unknown.
  ([GFX8 sprites](formats/gfx8-sprites.md))
- **The packed-length field** at sprite offset `+782` (correct only for
  single-block streams). ([GFX8 sprites](formats/gfx8-sprites.md))
- **GFX16** — a format slot with zero instances; layout unknown.
  ([RSC containers](formats/rsc-container.md))
- **SCR fields** `0x14`/`0x18` (authoring addresses), `0x24`/`0x28`
  (high-water marks), and the word-header link field `+12`.
  ([Script modules](formats/script-modules.md))
- **RSC reserved bytes** at `0x18`.
  ([RSC containers](formats/rsc-container.md))
- **FNT fields** `+2` (duplicate size) and `+4` (packed size) — redundant;
  read by the engine? ([Fonts](formats/fonts.md))
- **HMI internals** — the four pad bytes in the tempo and time-signature
  maps, and whether `F0`'s length includes a trailing `F7` (no shipped song
  has a sysex event). The event encoding itself is read.
  ([HMI songs](formats/hmi.md))
- **RSC.INF** — header parameters partially read; the 22-byte record
  layout is unmapped. ([Other files](formats/other-files.md))

## The FM driver

Three things `fmmidi3.com` does not answer for itself: **which register bank
is which side** (only the wiring can settle it, not a recording), **ordinal 1**
and why ordinals 6 and 8 share a body while the fuller reset at `0x0793` is
unreachable, and **`chan[8]`** in the engine's channel record, which is not the
channel number. Also one measured divergence in voice allocation under a
ten-note tick. All four are set out in
[The FM driver](engine/fm-driver.md#open-questions).

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
driver: see [The FM driver](engine/fm-driver.md#open-questions).

## See also

- [Departures](departures.md) — the deliberate differences, as opposed to the
  unknowns
- [Documentation index](README.md)
