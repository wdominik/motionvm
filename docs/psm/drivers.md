[← PSM 2](README.md)

# The PSM 2 drivers

*The PSM 2 component — `MUSADL.DRV`, the Ad Lib driver, in the 4480-byte and the 3915-byte build that are read, the sample path of the four `DMA*.DRV` digital drivers, and the mixer of `DMABLAST.DRV` with a song's digital voices, read out of the driver files themselves. File offsets are each driver's own; `MUSADL.DRV`'s are also its code-segment offsets, because its installer normalizes the loaded image to a paragraph boundary. How a host installs a driver and which entries it calls when is the hosting family's to document.*

## `MUSADL.DRV`

The driver file is the whole player — sequencer, fade engine and register
back-end in 4480 bytes behind a `MUS\0` header; a host keeps only a
loader, a call table and a timer service around it. It opens with `MUS\0`,
version 0x0100, fifteen entry offsets at +0x18, seven service-pointer cells
the installer fills at +0x36, and an `NS` trailer at the offset +0x0a
names.

**Two builds.** Besides the 4480-byte file with fifteen entries there is a
3915-byte build with **fourteen**, and its four data tables sit `0xd0`
earlier — `0x4f4`, `0x5f4`, `0x5fd` and `0x606` against `0x5c4`, `0x6c4`,
`0x6cd` and `0x6d6`. The tables themselves are byte-identical between the
two, which is what makes reading them at the wrong offset a silent wrong
answer rather than a loud one: 256 plausible defaults and a full note table
come back either way, and nothing sounds wrong until a note comes out flat.
The entry count says where to read, beside the offset of the `NS`
trailer: `0x1165` in the 4480-byte build, `0xf49` in the 3915-byte one.

**Builds that are not read.** Further 4480-byte builds with fifteen
entries exist, their trailers at `0x1166` and `0x1168`. Their four tables
are the same bytes at the same offsets; their code is not — the first
three entry offsets stand later by the same one or three bytes as the
trailer, and the code differs in places on both sides of the tables. What
any of it does is unread. The entry count
alone would read their tables right and play them with the read build's
code, so the rebuild tells a build by both words and refuses the others.

**Install.** Entry 0 flushes the install state to the chip: one default
per register (`0x5c4`), `0xFF` marking the untouched ones, written in
ascending register order (`0xf9e` → `0x1062`).

Every write goes through that 256-byte shadow (`0x7df`/`0x7ec`): a value
the shadow already holds is not sent again, so the register stream carries
**changes only** — across tunes too, because the shadow lives as long as
the driver. A copy loaded and installed afresh starts from the file's own
shadow, so its install sends every default again, whatever an earlier copy
left on the chip.

**The entries a song goes through.** SetSong (entry 5, `0x2f9`) stops a
song that still plays (`0x319`), checks the tag and stores the pointer.
Play (entry 10, `0x470`) stops the notes if something still plays, stores
the loop count, sets the volume full with its fraction byte cleared
(`0x4a5`, `0x4b6`), primes the streams and registers the host timer with
the period (`0xcb4`). The loop count is compared unsigned against a pass
counter that starts at 1, so `-1` means forever. PlayFade (entry 7,
`0x34a`) is Play with a fade-in: it starts a song only when none is
playing (`0x35f`), puts the level and its fraction byte at 0 (`0x378`,
`0x389`) and works out a rising step from a length in milliseconds
(`0x378`–`0x3d8`). FadeOut (entry 8, `0x3e2`) takes a length in
milliseconds the same way; at level 0 it does nothing (`0x3fe`), and
otherwise it clears the fraction byte and holds the level at full at most
(`0x405`–`0x412`) before the step is worked out. Stop (entry 11, `0x4c9`)
is the hard stop below.

**A fade's length in ticks** is worked out alike by both fade entries
(`0x398`–`0x3c1`, `0x419`–`0x444`): the milliseconds times `0x4a9`, 1193
PIT cycles to the millisecond, into a double word, divided by the tick's
period — after a test (`cmp cx, dx` / `jae`) that the high word stands
below the period, since the division would overflow otherwise. No length,
or one of two ticks or less, ends the fade at once: PlayFade starts at
full (`0x3ca`), FadeOut is silent (`0x44d`). A quotient past the word, or
past `0x8000` ticks, takes the smallest step there is, `±1` (`0x3c1`,
`0x444`). Any other count gives the step `±0x10000 / ticks`.

**The tick** (`0xa71`), on the host timer at the period: every `speed`th
tick is a row. Channels run 8 down to 0, each draining its due events; a
channel that dies under them no longer counts. When no channel is live, or
the sixteen-bit row counter wraps, the song is over — passes exhausted
stops it, otherwise the streams re-prime and the fresh row 0 plays **in
the same tick** (`0xb63`): the loop is gapless.

**Volume.** After every row (`0xc5e`), each live channel's carrier level
register is set from its loudness and its channel master (0x100 = full,
the file's value for all nine): full master passes the loudness through,
anything lower scales it byte-wise; the result is inverted back
(`^ 0x3F`) with the KSL bits on top. A fade (`0xb77`) instead runs **every
tick**, falling or rising alike: a Q8.8 level moves by the step the fade
entry worked out, each live carrier follows `master × level × loudness`
with every product truncated to a byte, level zero closes all carriers
(`0x3F` + KSL) and opens them again as a fade-in climbs, and reaching
0x100 ends the fade with a full pass (`0xb96`). A fade-out whose step
carries the level below zero ends it too, at 0: the level word is stored
as 0 (`0xb96` → `0xbc0`), the carriers close (`0xc26`), and since the
per-row pass runs only for a level above zero, they stay closed while the
song plays on.

**The callback cell** (`0x27a`) is how the driver drives a digital
driver's half of a song. It holds the file's stub (`0x57e`, a bare
`retf`) until a digital driver announces itself on the sound host's bus
with its table (`bh = 0x80, bl = 9`, taken at `0xf62`; `bl = 0x0a` puts the
stub back, `0xf54`). The driver calls it with `bx`:

| `bx` | Where | Arguments | What it says |
|---|---|---|---|
| 0 | the song is over (`0xb66`) or stopped (`0xe2f`) | — | the digital voices go |
| 2 | the end of every priming (`0xda3`): play, fade-in, loop | `al` the fast-forward flag, `cx` the level word `[280h]` | a song starts; the answer is the row count `[278h]` |
| 4 | the tick where the countdown `[5B1h]` reaches 0 (`0xa82`–`0xa9f`), when `[278h]`, counted down, reaches 0 | `al` the fast-forward flag | the next events; the answer is the next row count |
| 6 | every fade tick (`0xbae`, `0xc1e`, `0xc51`) | `ax = −1`, `cx` the level: `0x100` at full, 0 at silence, the level word between | every voice's scale |

So a step falls once per row, in the tick before the row (the countdown
stands at `speed − 1` after priming, `0xcc2`), and a song at one tick a row
never steps. A loop re-primes (`0xb48`, start) and counts down in the same
tick (`0xb4b`): its fresh row 0 and its first step share a tick. The
fast-forward flag is set only by entry 14 (`0x502`), which the rebuild does
not build; it is 0. The stub's `retf` hands back the `ax` it was called
with, so without a digital driver the row count is 0 and no step is ever
due. The fade's past-the-end branch decides its callback by itself: a level
past `0x100` ends the fade, at full where the level less `0x100` reads
non-negative (`0xba8`), and otherwise at 0 — `0xb96` clears the level word
before the sign of that comparison sends it to the store (`0xb9e` →
`0xbc0`), which closes the carriers (`0xc26`) and calls with 0 (`0xc49`).

**Stop** (`0xde2`): channel 8 down to 0 — key off where a key is on, then
the release rate of both operators opened to `0x0F`, so what still sounds
dies away; the timer is unregistered.

The two scale entries — tempo (`0x282`) and pitch (`0x2ba`) — stand at
their neutral 0x100 in the file: uncalled, the tempo word **is** the
period (clamped no lower than 0x200, `0xd29`), and notes sound exactly as
the table says.

## The sample path of the `DMA*` drivers

The period word of an `SM8` header is read in PIT cycles, and the digital
driver says so twice. Its direct path (`DMABLAST.DRV` `0x0611`) clamps the
word to 255 (`0x0653`) and indexes a 256-entry table at `0x60` for the
DSP's time constant (`0x065b`), and every entry of that table is
`256 − round(period · 1000 / 1193)`: the period in whole microseconds at
1.193 cycles each — `table[59] = 0xcf`, 49 µs; `table[149] = 0x83`,
125 µs — the constant being 1.193 and not the clock's 1.193182, which at
three periods (139, 207, 244) rounds the other way. The four `DMA*.DRV`
carry the same 256 bytes. A Sound Blaster DSP plays a byte every `256 −
constant` microseconds, so the DAC's clock is the DSP's rounding of the
header's: 8000 Hz where the PIT would say 8008, 20.4 kHz where it would say
20.2. The mixer (`0x196b`) reads the word at `+8` out of the header the
other way, as the step against the mixer's own period, itself set in PIT
cycles from a percentage (entry 24, `0x0ffd`: `256 − (n · 64 / 100 + 0x90)`
cycles).

**Which path plays.** The driver's play entry (17, `0x1fcf`) reads the
header's second word signed: at zero or below it takes the direct path
(`0x2012`). Otherwise it allocates one of up to eight mixer channels, with a
play count of the mode argument plus one; with one channel installed and
the mixer standing it takes the direct path too (`0x2020`). On the direct
path the length word becomes a single-cycle DMA count, the bytes go to the
DAC as they are, once, at full scale, no volume, and the interrupt at the
end writes the mid-point (`0x0a06`). `StopAll` is entry 18. Both paths are
rebuilt: the direct one as the one-channel voice, the mixer below. The
other three drivers decide between the two paths the same way
(`DMASB16S.DRV` `0x24b6`) and carry the same table; only the stereo SB16
driver's mixer runs at a rate in hertz, 11 025 to 22 050, sent with the
DSP's set-rate command (`0x41`, `0x09be`), which its direct path never
uses.

## The mixer of `DMABLAST.DRV`

The mixer is the play entry's other branch, and it is rebuilt from
`DMABLAST.DRV`, the 9840-byte build every install measured ships; the
other three drivers' mixers sit at other offsets and are not read. What
it needs from the file is read out of it: the time constant `[4A0h]`
(`0xd0`), the mixer's period `[49Eh]:[49Ch]` (`0x0039:0x4120`, 48 µs at
1.1928 PIT cycles each, what `0x0530` works out for `0xd0`), the default
volume `[1888h]`, the scale shift `[284h]` (0, written by no code), the
play cursor `[1180h]`, and the volume curve at `0x2212`.

**Channels.** Entry 0 takes the channel count clamped to 1–8 (`0x0e5f`):
channel 0 off at priority `0xff`, channels 1 to `n` switched on with
priorities rising by `0x1fe / 2n` to `0xff`, and behind them, from `n + 1`
on, a song's digital voices (`0x0e72`–`0x0e95`).

**Play** (`0x200c`). A free channel is the first switched-on one with the
idle routine, searched round from the cursor over the effect channels
only (`0x2045`–`0x206a`); with none free the cursor's own channel is taken
from whatever plays on it, channel 1 if the cursor's is switched off
(`0x206c`–`0x207c`); the cursor moves past the channel taken. The channel's
volume is the default where the header's second word says `0x0100`, and
**`0x0100` for any other word** — the entry loads `0x100` (`0x2090`) and
replaces it only with the default (`0x2093`–`0x2098`), never with the word
itself. Then one pass, nothing held, the fraction at `0xffff`, the length
and the first PCM byte (`0x20a2`–`0x20e0`). A mixer that runs goes on; one
that stands starts from rest (`0x1b8a`); one whose stop the interrupt has
not yet acknowledged is waited for, and starts from rest at that interrupt
(`0x20e6`–`0x20fa`).

**A block** (`0x18e3`) is 128 words cleared, then every switched-on
channel from the last down to 0 running its routine into them — the sample
routine `0x196b` or the idle `0x1965` — and the words turned into bytes
(`0x04b0`): each plus `0x80`, its low byte where the high byte is 0, `0xff`
above, `0x00` where the sign is set, the sum wrapping in sixteen bits.
A block with no channel playing and no song's stream running is the end:
its half gets the mid-point (`0x0500`) and the driver marks itself
stopped (`[1596h] = 0`).

**The sample routine** (`0x196b`) works on copies of the channel's volume,
length, read position, fraction and held contribution and writes them back
at the block's end (`0x19bc`); a sample that ends inside the block writes
nothing back, and its channel adds nothing more to that block. A byte adds
`(byte − 0x80) × volume`, signed, bits 8 to 23 of the product (`0x19e8`).
The sample's period `P` is stepped against the mixer's `M.m` in 16.16:

- **`M < P`, held** (`0x19a2`–`0x19fd`): the step is `M.m / P`; an output
  fetches a byte only when the step carries out of the fraction, and adds
  the held contribution otherwise. The fraction starts at `0xffff`, so byte
  0 is fetched for output 0, and the fetch that empties the length ends the
  sample: the last byte counts for one output only.
- **`M ≥ P`, fast** (`0x1a03`–`0x1a3b`): one byte per output, then on by
  one or two bytes — the step `((M mod P).m) / P` for a quotient of 1,
  `0xffff` for more (`0x1a5e`), its carry the second byte — so the routine
  skips zero or one byte per output and never more, and a sample above
  twice the mixer's rate plays slow. From the fraction's `0xffff` the first
  step carries: byte 1 is skipped. The length counts down by the move and
  ends the sample at zero or past it.

**The DMA buffer** is 256 bytes, two halves written in turn; the first
block after a start goes to the first half (`[188Bh]`, `0x04b7`).
Every block is one single-cycle transfer of 128 bytes (`0x06d8`: `0x14`,
length `0x7f`) at the file's time constant — a byte every 48 µs, 20 833⅓
a second, mono, unsigned. The interrupt at a block's end takes over the
mixer's state (`[1592h] = [1596h]`, `0x07e1`); running, it starts the next
half, already mixed, and then mixes the one after it (`0x1b10`): while one
block plays the next is ready, so what happens during a block is heard
from the block after the next, 128 to 256 bytes later and always from a
block's first byte. Stopped, it plays nothing further (`0x085e`) and
writes nothing to the DAC. A start from rest (`0x1bc7`–`0x1bd6`) mixes the
first half, starts it, and mixes the second: a play into a quiet mixer is
heard from the call.

**`StopAll`** (entry 18, `0x2121` → `0x2104`, `0x0794`) sets every mixed
channel idle — the song's voices too — and marks the mixer stopped: the
block playing ends, the one mixed after it is not played, and a song's
stream runs on with its voices silent until a play or a song start runs
the mixer again. The argument makes the caller wait for that interrupt
(`0x2139`–`0x2145`).

**Entry 22** (`0x1f37`) takes the default volume shifted by `[284h]` when
it lies in 1 to `0x2001`, and answers the old one.

**What is not taken.** Every play here passes mode 0, a single pass; the
repeat branch is not built — its restart on the held path (`0x1a83` →
`0x19e3`) fetches without counting the output it fetches for, so a channel
writes a 129th word past the accumulator, onto the cursor at `0x1180`. A
period of 0 sends the routine to `0x1a52` without the `pop bx` its frame
needs, and a length of 0 counts down past zero: such samples are refused.
The overload handling never acts in the rebuild: it needs a DSP
interrupt to arrive while a block is still being mixed (`0x07f2`,
`0x1b35`), which a model that mixes a block whole before time moves on
never produces. Read, it counts blocks with an overflow in a row and lowers
the time constant by one after thirty-two, then after every sixteen more
(`0x1b41`–`0x1b51`, `0x0530`), down to `0x90`, where `0x1b58` instead
searches the switched-off channels for the lowest priority.

## A song's digital voices

`DMABLAST.DRV` plays a song's `MDH` part ([the format](format.md#the-digital-part--the-mdh-chunk))
in the same mixer as the effects, and only through `MUSADL.DRV`'s callback
cell: the driver announces its table (`0x2318`, a jump through `0x231d`)
when it installs (`0x0f02`–`0x0f25`), and the music driver's calls are
what step the voices. Entry 21 (`0x25ea`) only hands over the `MDH` and
the song's sample table — checked for `MD`, `H\0` and a voice count of 1 to
8, any null pointer clearing both — and lets the last song's voices go.

- **`bx = 0`** (`0x2325` → `0x2444`): the voices cut, from the next block
  on, and the stream over.
- **`bx = 2`** (`0x2329` → `0x2506`): the voices placed on the channels
  behind the effects, each with its record's priority and four levels, the
  level word as its scale **as it is** (no curve), and its sample from the
  table; a voice whose slot is empty stays switched off. The stream starts
  and, if the mixer stands, so does the mixer (`0x25ce`–`0x25e4`). Answers
  1, or 0 without an `MDH`.
- **`bx = 4`** (`0x2342`): events up to the next pause, answering the
  pause in rows (0 at the stream's end). A voice event sets the voice's
  channel volume, level times scale, bits 8 to 23 of the unsigned product
  (`0x23b0`), and plays its sample from the start (`0x24c8`): nothing
  held, the fraction at `0xffff`, one pass — never the default volume, and
  never a start of the mixer. There is no note-off and no pitch: a sample
  runs to its end or is struck again, at its own period.
- **`bx = 6`** with `ax < 0` (`0x2405`, `0x242b`–`0x2440`): every voice's
  scale, a level below `0x100` through the curve at `0x2212` first; each
  voice with a sample takes it and its channel volume follows at once;
  above `0x100` the call only asks. The form for one voice, `ax ≥ 0`, which
  only the Ad Lib driver's volume entry sends, is not built.

## How the rebuild meets the two clocks

The music ticks on the PIT, the blocks on the DSP, and the two meet only
where a callback writes channel state that the next block reads. These are
the rebuild's choices, where the files say nothing:

- The DSP's clock starts at the frame a start from rest happens on, and
  both clocks run exact from there; at a frame where a tick and a block's
  end fall together, the block ends first.
- The blocks play back to back. Each is its own transfer, and the
  interrupt takes some time before the next begins; how long is the
  machine's.
- **The level of the DAC against the OPL** is the one-channel path's: each
  DMA byte, held at the device rate, adds `(byte − 0x80) << 8`, saturating,
  onto both channels of the frame the OPL filled. The card mixed the two in
  analog, and no file records the balance.
- After the last block the rebuild adds nothing; the driver writes nothing
  to the DAC there.

## Open questions

- The mixers of `DMASB16M.DRV`, `DMASB16S.DRV` and `DMASB2P.DRV`, and
  their halves of a song's digital voices, are unread; of those drivers
  what is read is the sample path.
- `MUSADL.DRV`'s volume entry (2, `0x10fc`) and the fast-forward entry
  (14, `0x502`) are not built, nor the one-voice volume callback they
  would send.
- Whether a digital driver already resident answers a freshly installed
  `MUSADL.DRV`'s question for its table (`bx = 0x8006`, `0x108e`) is
  unread; the rebuild links a reinstalled copy to a mixer it keeps as the
  first. A host whose programs take the digital driver down with them —
  Dunkle Schatten's — takes the mixer down too and installs it anew
  (`Player::uninstall_mixer`, `Player::install_mixer`).

## See also

- [The format](format.md) — what these drivers play
- [PSM 2](README.md) — the component's index
