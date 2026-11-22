[← PSM 2](README.md)

# The PSM 2 drivers

*The PSM 2 component — `MUSADL.DRV`, the Ad Lib driver, in the 4480-byte and the 3915-byte build that are read, and the sample path of the four `DMA*.DRV` digital drivers, read out of the driver files themselves. File offsets are each driver's own; `MUSADL.DRV`'s are also its code-segment offsets, because its installer normalizes the loaded image to a paragraph boundary. How a host installs a driver and which entries it calls when is the hosting family's to document.*

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
the driver.

**The entries a song goes through.** SetSong (`0x2f9`) checks the tag and
stores the pointer. Play (`0x470`) stops the notes if something still
plays, stores the loop count, sets the volume full, primes the streams and
registers the host timer with the period (`0xcb4`). The loop count is
compared unsigned against a pass counter that starts at 1, so `-1` means
forever. FadeOut (`0x3e2`) takes a length in milliseconds; Stop (`0x4c9`)
is the hard stop below.

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
tick**: a Q8.8 level moves by `±0x10000 / ticks` for the asked-for
milliseconds — `0x4a9`, 1193 PIT cycles, to the millisecond — each live
carrier follows `master × level × loudness` with every product truncated
to a byte, reaching zero closes all carriers (`0x3F` + KSL), and reaching
0x100 ends the fade with a full pass.

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

**Which path plays.** The driver's play entry (17, `0x1fcf`) allocates one
of up to eight mixer channels and gives it the header's second word as its
volume — `0x0100` meaning the default the host set with entry 22 — and a
play count of the mode argument plus one; with one channel installed it
takes the direct path instead (`0x2028`): the length word becomes a
single-cycle DMA count, the bytes go to the DAC as they are, once, at full
scale, no volume, and the interrupt at the end writes the mid-point
(`0x0a06`). StopAll is entry 18. The direct path is the one rebuilt.

**The mixer, read and not taken.** With more channels the play entry's
other branch (`0x2045`) hands the block to a channel routine (`0x196b`)
that the driver's timer runs for every active channel into a 128-word
accumulator (`0x18e3`): each output sample takes the byte under the
channel's clock, less `0x80`, times the channel's volume over 256 —
`0x0100` is unity — and adds it. The clock is the sample's period against
the mixer's own, in 16.16 fixed point: a zero-order hold while the sample
is slower than the mixer, and when it is faster one or two bytes dropped
per output sample and never more, so a sample above twice the mixer's rate
plays slow. The accumulator then becomes the DMA buffer (`0x4b0`): each
word plus `0x80`, clipped to `0x00` and `0xff`. The mixer's rate is a time
constant too, `0x90 + n · 64 / 100` for the percentage entry 24 takes —
8.9 to 20.8 kHz — and it steps down one constant whenever the mixer has
fallen behind its buffer thirty-two times (`0x530`). The other three
drivers decide between the two paths the same way (`DMASB16S.DRV`
`0x24b6`) and carry the same table; only the stereo SB16 driver's mixer
runs at a rate in hertz, 11 025 to 22 050, sent with the DSP's set-rate
command (`0x41`, `0x09be`), which its direct path never uses.

## Open questions

- The `DMA*.DRV` drivers' sequencer over the `SM8` sections — the digital
  rendition of a tune — is unread, and the rebuild plays the Ad Lib
  rendition only. What is read of those drivers is the sample path above.
- `MUSADL.DRV`'s callback protocol (`0x27a`) and the Volume entry's
  negative selectors: present, and not read through.

## See also

- [The format](format.md) — what these drivers play
- [PSM 2](README.md) — the component's index
