[← Documentation index](../../README.md)

# HMI Song Format

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

The music blocks (see [Blocks](blocks.md)) and the standalone `TEST.HMI` use
the song format of Human Machine Interfaces (HMI), the middleware whose
drivers ship with the game. The game contains 26 songs, block ids 0–25.

Everything below was read out of the **HMI sequencer, which is statically
linked into `ENGINE.EXE`** — the parser at `0x8F880`, the per-tick service at
`0x8FC82`, the event dispatcher at `0x98983`, the variable-length reader at
`0x994B2`. Only the FM synthesis lives in the external `.386` drivers.

All multi-byte values are little-endian.

## Song header

| Offset | Type | Description |
|---|---|---|
| `0x00` | `char[18]` | Magic `HMI-MIDISONG061595` |
| `0xd2` | `i16` | **Division** — ticks per quarter note. Musical bookkeeping only |
| `0xd4` | `i16` | **Tick rate in Hz** — the unit every delta is counted in. 120 in all 26 songs and in `TEST.HMI` |
| `0xd6` | `u32` | Tempo map: `u8 count; u8 pad[4]; { u32 bpm; u32 length }…` |
| `0xda` | `u32` | Time-signature map, same shape (`0x40000` = 4/4) |
| `0xe2` | `u16` | Greatest number of notes that may sound at once |
| `0xe4` | `u32` | Track count (2–16) |
| `0xe8` | `u32` | File offset of the track offset table (`0x172` in every file) |

**The delta unit is `1/[0xd4]` of a second, not a PPQN tick.** The sequencer
installs its timer at that rate (`0x8FE43`) and counts each track's pending
delta down once per timer tick (`0x8FD53`). Division and tempo map exist, but
the sequencer never reads them — their only consumer is `0x9CA6D`, which turns
a tick count into bars and beats for display. Taking them for the clock puts
the music at the wrong speed.

### And it does not get the rate it asks for

`+0xd4` is what the sequencer **asks** for. What it gets is 115.45 Hz, four per
cent slower, and everything in the game's music is that much slower with it.

The sequencer does not receive a timer of its own. The sound layer has already
installed a master at 1500 Hz (`0x8F80D`, and the divisor `1193180/1500 = 795`
truncates, so it really runs at 1500.86 Hz). A second timer asking for a slower
rate rides on that one by counting master ticks rather than reprogramming the
chip — `0x8CF23` divides the two divisors against each other in 16.16 fixed
point. And `1500.86 / 120` is **12.507** master ticks, which is not a whole
number. The original takes thirteen: `1500.86 / 13 = 115.45 Hz`.

The two constants are read from the code. That the remainder is dropped rather
than carried is measured against the recording: over the first 150 notes of
block 25, thirteen master ticks puts every one of them within 41 ms of where the
original played it, where the header's 120 Hz has run 313 ms ahead. The
fixed-point arithmetic inside the timer service that produces the thirteen has
not been read — see [Open questions](../../open-questions.md).

The maps carry one entry each in every shipped song: 126 BPM for `TEST.HMI`,
90–110 for the game's, always 4/4. **There is no tempo event in any stream**,
so tempo cannot change during playback.

## Track structure

| Offset | Type | Description |
|---|---|---|
| `+0x00` | `char[13]` | Magic `HMI-MIDITRACK`, NUL-padded |
| `+0x0c` | `u32` | 75 in every track of every song; nothing in the image reads it |
| `+0x4b` | `u32` | → a patch-name blob, `<u8> <u8 len> <len chars> 00` |
| `+0x57` | `u32` | → **the event stream** (relocated in place at `0x986DE`) |
| `+0x63` | `u32` | → the branch-point table, or 0 |
| `+0x77` | `u16` | Voice priority (also settable by controller 107) |
| `+0x7b` | `u16` | **The channel every message of this track goes out on** |
| `+0x99` | `u16[≤8]` | **The devices the track is for**, ids of the `0xA000` series, zero-terminated. The song open (`0x9c8a6`) gives the track to the first installed device one of them names — `0xA000` naming `0xA001` and `0xA008` as well, `0xA002` naming `0xA009`, the OPL3 — and a track none of them names is not played. Every track of every shipped song names `0xA000`, and the ones that also name `0xA002` are the ones the OPL3 plays: song 17 has three tracks that do not, and the opening tune (25) two — its bass (channel 2) and second guitar (channel 4) |

Note that the event stream is at **`+0x57`**. `+0x0c` holds 75 in every track
of every song, which reads like a plausible offset and is not one — nothing in
the image ever loads it.

The branch-point table is `<u8 count> { i16 id; u32 offset from track start }…`.

## The event stream

`VLQ delta`, event, `VLQ delta`, event, … up to `FF 2F`. The delta is the
standard MIDI variable-length quantity — seven bits a byte, ending at the first
byte **without** the top bit (`0x994B2`).

| Status | Payload |
|---|---|
| `8n` note off | note, velocity — **never appears**; the engine's handler returns without advancing and stalls the track |
| `9n` note on | note, velocity, **`VLQ` duration** |
| `An` poly pressure | note, value |
| `Bn` control change | controller, value |
| `Cn` program | program |
| `Dn` channel pressure | value |
| `En` pitch bend | lsb, msb |
| `F0` sysex | `u32` length (**not** a VLQ), then that many bytes |
| `F1`–`FD` | nothing — the handler returns without advancing |
| `FE` | an extended event, below |
| `FF 2F` | end of track. No length byte; the `00` that follows in every file is padding |

Four traps, each of them something the code does and a reasonable guess would
not:

1. **A note-on carries its own length.** There are no note-offs in the format:
   40,879 note events across the 27 shipped files and not one `8n`. The
   sequencer queues the end and ages the queue at the top of each tick
   (`0x8FCD6`), firing when the counter it *reads* is already zero — so the end
   lands `duration + 1` ticks after the start.
2. **Running status is one byte, and `FE` and `FF` clobber it.** `0x8FD6C`
   stores *any* byte ≥ 0x80 into `track+0x42`; it is never reset at a delta.
3. **The channel nibble of the status byte is meaningless.** Output goes to
   `track+0x7B`. `TEST.HMI`'s first track is channel 4 and its first status
   byte is `B0`.
4. **A note-on with velocity 0 is not a note-off.** The message is sent either
   way (`0x98A89`).

### Controllers the sequencer keeps for itself

103, 104, 106, 107 are its own bookkeeping; **105** turns a track's voice on
and off (`b0 69 01` opens every track); 108 is its internal all-notes-off; 119
is a callback the game never installs. Everything else is passed through and
cached per channel. Census over all 27 files: CC 1 (16,028), 7 (505), 10 (279),
64 (186), 91 (197), 93 (193), 105 (398), 123 (48), 2 (2), and nothing else.

Volume is not a plain pass-through. Controller 7 is cached per channel as
`value × song × master × channel / 127³` (`0x98C05`…`0x98C4D`), and on **channel
9, and only there**, that cache is folded into note velocity as well
(`0x98A89`, `velocity × chan[0x1C] / 127`). The controller still goes out, so
the drums are attenuated twice over — once here and once by the driver's own
volume. A recording of the original pins it down: block 25's hi-hat, velocity 80
at controller 7 = 105, reaches the chip as attenuation `0x10`, and only
`80 × 105 / 127 = 66` followed by the driver's `(105 × 66) >> 7 = 54` gives
that.

`0x98C56` withholds the controller from the device when a field of the channel
record reads 9. What that field is has not been established, and in the
recording the controller plainly does reach the driver on channel 9. The three
volume factors are all at their maximum in the game; any other value would move
the drum levels, and they match to the byte.

### The `FE` family — where the loops live

Sub-type in the byte after `FE`; sizes from the jump table at `0x98967`.

| Event | Size | Meaning |
|---|---|---|
| `FE 10 <u16 id> <u8 len> <len bytes> <u32 tick>` | `9 + len` | **Branch point.** The bytes are a controller snapshot, the `u32` the tick the track resets to |
| `FE 11 <2> <u32 target>` | 8 | Conditional local branch, gated on a callback the game never installs |
| `FE 12` / `FE 14 <live> <reset>` | 4 | **Loop counter.** The engine copies `reset` over `live` *in the stream* (`0x991B8`) |
| `FE 13 <2> <u32 target> <u32 counter>` | 12 | Counted local branch |
| `FE 15 <u16 id> <u32 counter offset>` | 8 | **The loop the game uses** |
| `FE 16 <u16 id>` | 4 | Conditional global branch |

**A loop is collective.** `FE 15` names a branch id, and `0x90714` sends *every*
track of the song to its own `FE 10` marker with that id — not just the track
that hit the `FE 15`. And a branch is more than a seek: the engine emits
controller 108 (killing that track's sounding notes), replays the marker's
snapshot, reloads the track's tick from the marker's `u32`, and reads the next
delta.

In every shipped song the id is 384 and the counter is `0xFF`, which the engine
reads as "forever" (`0x9929D`). **That is why the tunes loop**: the game never
restarts them, and `STARTTUNE`'s loop argument is inert in `ENGINE.EXE` — it
sets a bit at `+0xD` of the tune node that nothing reads.

The snapshot's keys are the engine's cache slots rather than plain controller
numbers: 104 is a program change, 105 a pitch bend (`0x99A1B`, `0x99A38`),
anything else the controller of that number. Every shipped snapshot is
`68 <program> 69 40` — a program and a centered pitch bend.

## How this is checked

All 26 shipped songs decode: 186 tracks, every one ending exactly on its
`FF 2F`, no unknown event, no note-off.

`TEST.MID` is the standard-MIDI source `TEST.HMI` was converted from, and the
two agree: **1961 note-ons in both**, the same 13 channels, and channel by
channel the same sequence of pitch and velocity. Times line up through 5/42 —
480 ticks a quarter at 126 BPM against 120 Hz — to within the one tick the
conversion's rounding costs.

## Playback

The sound layer opens a song by formatting its block id as a virtual file name
`NNN.blk` and reading it through the resource layer — see
[Audio](../engine/audio.md) for the word set and the driver stack. Each
extracted song ends with its original DOS file name (e.g. `susp0_03.mid`), a
leftover of the conversion tool that built the blocks. Several songs are
duplicates: 26 blocks hold about 19 distinct pieces.

## Open questions

- The four pad bytes after the count in the tempo and time-signature maps —
  skipped by the reader, zero in every file.
- Whether `F0`'s `u32` length includes a trailing `F7`. No shipped song
  contains a sysex event.
- `FE 11`, `FE 13` and `FE 16` are code-derived only; no instance exists in the
  data.

## See also

- [Audio](../engine/audio.md) — playback words and drivers
- [Blocks](blocks.md) — where the songs are stored
- [Other files](../../games/ds2/other-files.md) — the HMI drivers and instrument banks
