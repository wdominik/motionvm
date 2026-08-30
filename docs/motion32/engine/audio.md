[← Documentation index](../../README.md)

# Audio

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

The engine links the **HMI "Sound Operating System" (SOS)** middleware
statically — the same layer whose loadable drivers (`HMI*.386`) and
instrument banks ship in the game directory. Audio splits into two paths:
**MIDI music** ([HMI songs](../formats/hmi.md) stored as block resources)
and **digital samples** (WAV data, either from block resources or from
loose files). All engine addresses below refer to the relocated image.

## Configuration and driver startup

Sound initialization runs once (guarded by a global at `0xDBD68`):

1. **Read `HMISET.CFG`** — searched via the path spec `<SOSPATH<PATH`
   (the environment variables `SOSPATH`, then `PATH`). If the file is
   missing, the engine tries to *run* `HMISETUP.EXE` to create it, then
   looks again; if it still fails, sound stays off. The shipped game
   creates the file through its own `SNDSETUP.EXE` instead (see
   [Other files](../../games/ds2/other-files.md)). From the config it reads
   `[DIGITAL]` (`DeviceID`, `DevicePort`, `DeviceDMA`, `DeviceIRQ`;
   sample rate fixed at 11025 Hz) and `[MIDI]` (`DeviceID`,
   `DevicePort`).
2. **Load the drivers** from the `HMI*.386` archives (format below),
   selected by device id. Success is recorded in a status bitfield at
   `0xDC3FC`: bit 1 = MIDI driver up, bit 2 = digital driver up.
3. For the **FM-synth MIDI drivers only** (device ids `0xA002`, `0xA009`),
   load `MELODIC.BNK` and `DRUM.BNK` (`0x850DB`, `0x8511C`) and upload the
   patches (`0x8F7C4`). Other MIDI devices never touch the banks — and note
   that it is **`ENGINE.EXE` that opens the two files**; the strings do not
   occur in any `HMI*.386`. A missing or short bank silently disables music.

   The driver the game asks for is **OPL3**: `SNDSETUP.INI` maps "Sound
   Blaster 16" to device `0xA009`, which is `fmmidi3.com`, *"HMI MIDI Driver
   For OPL3"*. What it does with the banks and the messages is
   [its own page](fm-driver.md).
4. Register the sample garbage collector (`SCOLLECT`, under the internal
   name `SMPCOLLECT`) as an idle callback the main loop runs
   automatically.
5. Read the optional **`smppath`** file (plain text, max 80 bytes, `#`
   starts a comment): a directory prefix for speech WAV files. It is not
   present in the shipped game.

No software interrupts are involved: the SOS layer is called directly, and
the `.386` driver images are entered through an initial `jmp`.

### The `.386` driver archives

A 44-byte file header, then a flat chain of 48-byte record headers each
followed by its own image: `next = here + 0x30 + image_size`. **There is no
directory** — the only way to reach the last driver is to walk. The full layout,
the eight MIDI drivers and their device ids are on
[their own page](../formats/driver-archive.md).

Each image is a flat 32-bit `.COM`-style blob beginning with a near `jmp`; the
loader copies it into memory and enters it there. It has no object format at
all, so reading one means disassembling flat 32-bit code from the payload with
the base set to zero: the image's own absolute references count from the byte
after the header, which sits `0x30` into the archive entry.

**There is no directory at `0x2C`.** Read as one it lands on the first
*header* instead, and every entry derived from it makes the rest of the file
look like garbage — a reading that fails in a way easy to mistake for a
corrupt archive.

Only the two FM drivers can make sound from what is on the disc: MPU-401,
MT-32, AWE32 and Gravis all want external hardware or a patch set that is
not shipped. `SNDSETUP.INI` sends Sound Blaster and SB Pro to OPL2 (`0xA002`,
`fmmidi.com`), and SB16, Microsoft Sound System and PAS16 to OPL3 (`0xA009`,
`fmmidi3.com`).

## The nine audio words

All in kernel table 2:

| Word | Stack effect | Purpose |
|---|---|---|
| `STARTTUNE` | `( tune loop -- handle )` | Start a song |
| `ENDTUNE` | `( handle -- )` | Stop a song |
| `STARTSAMPLE` | `( block n -- handle )` | Play a sample from a block resource |
| `->STARTSAMPLE` | `( name$ n -- handle )` | Play a WAV file from disk (speech path) |
| `STOPSAMPLE` | `( handle -- )` | Stop a sample and free it |
| `?STIME` | `( handle -- t \| -1 )` | Sample playback position |
| `SCOLLECT` | `( -- )` | Sample garbage collector (runs automatically) |
| `?SOUND` | `( -- f )` | **1 or 0** (not −1) if the **digital** driver initialized |
| `MUSVOLUME` | `( f -- )` | Music duck: 0 = 44 % volume, else full |

If sound never initialized, every word is a silent no-op (the start words
push 0).

### STARTTUNE

**One value comes back.** `0x7FB24` is the handler's only exit and it does a
single `push()`; when sound never initialized the value is still 0, so the word
answers 0 without doing anything (`0x7F9A9`). Counting pushes statically gives
two, which is the upper bound a branchy handler always yields — the second
`push()` is on a path the first one has already left.

Formats the tune id as **`NNN.blk`** and opens it through the engine's
resource layer — block resources are exposed to the sound code as virtual
`.blk` files, which is why the songs live in the BLOCK segment. If a tune
node for that id already exists, the song is simply **restarted** rather
than reloaded. Otherwise the whole file is read, registered with the HMI
MIDI layer (requires the MIDI status bit), and started; the word returns
the sequence handle (0 on failure). The `loop` argument sets a flag bit in
the tune node; the game always passes −1 (loop). No code that *reads* the
flag back was found — looping may be the driver default (open question).

### ENDTUNE

Takes the **handle returned by `STARTTUNE`** (not the tune id), finds the
node, and issues a stop. The node itself stays allocated; only the
shutdown routine (`EXITMUSIC`) frees tune nodes.

**The stop silences what was sounding.** `0x858F4` reaches the sequencer's stop
at `0x8FE86`, which calls `0x99B11` — and that walks the song's channel list and
sends each channel **controller 108**, the sequencer's own all-notes-off. The
controller never leaves for the device; what it does is drop that channel's
entries from the pending-note queue, note off. Since every channel of the song
gets one, the whole queue goes. Without it a location change would leave the old
tune's notes hanging under the new one.

### STARTSAMPLE and ->STARTSAMPLE

Both play 11025 Hz PCM WAV data and return a handle:

- `STARTSAMPLE` reads block resource `NNN.blk` whole and plays it at
  **¼ volume** (0x1FFF of 0x7FFF).
- `->STARTSAMPLE` takes a **file name string**; the file is looked up as
  `<smppath>\<name>`. Small files (≤ 128 KB) load whole; larger ones
  **stream** through a 136 KB ring buffer refilled in 8 KB chunks by a
  driver callback. The engine parses exactly the canonical 44-byte
  RIFF/WAVE header (rate at `+24`, bits at `+34`, channels at `+22`) and
  streams the rest. Speech plays at **full volume** (0x7FFF). Only one
  stream can be active at a time.
- Playback runs on channel 0; the second argument is passed to the SOS
  start call (most plausibly a loop count — the game always passes 0 for
  speech; open question).
- A missing file is a quirk case: `STARTSAMPLE` returns 0 **silently**,
  and `->STARTSAMPLE` **retries in an endless loop** (a "please insert
  the CD" spin). Both paths try to print a diagnostic, but they pass a
  module id ≥ 1000 to the diagnostic routine, which ignores such ids —
  so no message ever appears.

Each sample gets a node carrying the SOS handle, a timer, the data size,
and a duration computed as `bytes × 200 / rate` — i.e. in **1/200 s
units** for 8-bit mono.

### ?STIME

Returns the elapsed playback time of a sample in 1/200 s units, or **−1**
when the handle is unknown or the sample has finished. "Finished" is
decided by comparing the elapsed timer against the duration **plus a 30 %
safety margin** (`duration × 1.3`); reaching it marks the node done.
Script code consistently divides the result by 2 to get 1/100 s.

The engine's timer objects come in three types scaling a ≈1 kHz master
tick by 1/20, 1/10, and ×10/51 — sample timers use the third, the 1/200 s
unit. (The exact master frequency is inferred, not measured.)

### SCOLLECT

Registered as an idle callback (internal name `SMPCOLLECT`) in a four-slot
callback table the main loop services — scripts never need to call it.
Per pass it tops up the active stream's ring buffer and frees every
finished sample (stops the voice, unlinks the node, frees data and
timer).

### ?SOUND and MUSVOLUME

`?SOUND` tests only the **digital** driver status bit — it says nothing
about music. `MUSVOLUME` is a boolean duck applied to all sequences:
argument 0 sets volume 0x3800 (≈ 44 %), anything else restores 0x7FFF.

## The path a note takes

[Sequencer](../formats/hmi.md) → [FM driver](fm-driver.md) → OPL3 → samples.
Three of those four are in the shipped files and are described on their pages.
The fourth is not, and cannot be: the OPL3 is hardware, and there is no code in
the game that says what a YMF262 does with a register — only code that writes to
one. Reproducing the chip therefore means going outside the game entirely, which
is a [departure](../../departures.md) and is recorded there.

**The clock is not the rate the songs ask for.** See
[HMI songs](../formats/hmi.md): the sequencer rides the sound layer's 1500 Hz
master timer, and a song asking for 120 Hz gets thirteen master ticks per tick,
which is 115.45 Hz. Four per cent slow, and audible.

## How the game uses it

- **Music is per location**: each scene macro ends with
  `NN -1 STARTTUNE _ACTMUSIC !`, where **location module 3NN plays tune NN**.
  The reachable set is exactly `{1–11, 13, 14, 15, 17–22, 25}` — 21 tunes,
  every argument a literal. Three of those pairings do not follow the rule and
  have to be read off the modules rather than inferred: **location 23 plays
  tune 25**, not 23; **locations 12, 16 and 30 start no music at all** and are
  deliberately silent, because `INCLLOC` has already stopped the previous tune;
  and **no script ever starts tune 0**. Songs 0, 12, 16, 23 and 24 ship and are
  never asked for. The only stop site is the location switch:
  `_ACTMUSIC @ ENDTUNE  0 _ACTMUSIC !`, with the **handle**.
- One dead branch in the location loader would start a **random tune
  60–63** — no such blocks exist, and the guarding location-range test
  can never be true; if it ever ran, `STARTTUNE` would fail silently.
- `?SOUND` is called once, in `STARTUP`, storing the result in `_SPEECH`
  — which is never read again.
- `STARTSAMPLE`, `STOPSAMPLE`, and `MUSVOLUME` are never called by the
  shipped scripts.

## The speech system — fully built, never enabled

The scripts contain a complete lip-sync speech layer that the shipped
game never activates:

- A **speaker table** of 10 slots × 12 bytes — the audio one, `SPEAKER`,
  which is *not* `_SPEAKTABLE` (stride 40, the text-dialogue table filled by
  `SETSPEAKER` from 197 live call sites in sixteen location modules). **The two
  are separate tables with confusable names**: only `SPEAKER` has anything to
  do with sound, and the only live code that touches it is `INCLLOC`, which
  resets its ten idle markers. Layout:
  `{ +0 start-callback, +4 stop-callback, +8 scheduled stop time }`.
  `SETSPEAKER` binds a figure to a slot; the callbacks toggle the talking
  animation.
- `->SPEECHSEQ ( name$ table -- )` starts a WAV via `->STARTSAMPLE` and
  stores a **cue table**: records of
  `{ +0 start time, +4 stop time, +8 speaker index }` in 1/100 s,
  terminated by speaker index 0.
- A per-frame pump (`SAMPLE_TIMING`) polls `?STIME`, fires each speaker's
  start callback at its cue time, schedules the stop, and advances
  through the cue table.

Nothing calls `->SPEECHSEQ` or the pump, and the address `SAMPLE_TIMING` would
have (`0x40060`) appears in no module — the only two literals in module 4's
address window are `DO_INVSEL` and `ICTRL`. `->SPEAKER`, which would install
the callbacks, is never called either, so even a running pump would execute
nothing. No `smppath` file and no speech WAVs ship, and a byte scan of all
three containers finds **no `RIFF`, `WAVE`, `MThd` or VOC data anywhere** —
there is nothing for the sample path to play. Infrastructure for a voiced
version that never shipped on this disc.

One latent bug in it, for the record: the "sample finished" arm reads
`?STIME 2 / -1 =`, and `/` truncates, so `-1 2 /` is 0 and that arm can never
fire. `SPEECHSEQ->` gets it right by testing before dividing.

## Test files

`TEST.WAV` is canonical RIFF/WAVE: PCM, 2 channels, 11025 Hz, 8-bit,
3.66 s — a **pan sweep from right to left**. Measured per eighth: left peak 16
against right 127 at the start, 91 against 11 at the end. (It is not one silent
channel, which is what a glance at the opening samples suggests.)
`TEST.RAW` is the same material as headerless 8-bit mono. `TEST.HMI` is a
regular HMI song. All three exist for `SNDSETUP.EXE`'s hardware tests.
The extracted songs end with their original DOS file names (e.g.
`susp0_03.mid`) — a leftover of the conversion tool.

## Open questions

- What bit `0x04`, set and cleared by controller 106, means in the channel
  record.
- The second argument of the sample-start words (loop count is the best
  reading).
- The exact master timer frequency (≈1 kHz inferred).
- The texts behind the sound module's diagnostic message ids.

## See also

- [HMI songs](../formats/hmi.md)
- [Blocks](../formats/blocks.md) — where songs live
- [Other files](../../games/ds2/other-files.md) — drivers, banks, SNDSETUP
- [Game loop](game-loop.md)
- [Departures](../../departures.md) — the OPL3 core and the playback clock
