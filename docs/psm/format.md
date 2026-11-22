[← PSM 2](README.md)

# The PSM 2 format

*The PSM 2 component — the module, the `PLX` song, the `MDH` digital part and the `SM8` sample, as `MUSADL.DRV` and `DMABLAST.DRV` read them. File offsets are `MUSADL.DRV`'s unless marked otherwise. Which blocks of which game are music, and how many, is the hosting family's to say.*

## The module

Sixteen bytes of tag, `MTCVTS PSM 2.00\0`, then a table of up to ten `u32`
section offsets at +0x10, ascending, ending at the first zero. A host's
loader converts them to far pointers in place and hands the driver the
first section — the `PLX\0` song. Every later section is an `SM8\0` sample
for the digital-sample driver a host can install beside the Ad Lib one, and
an `MDH\0` block — the song's digital voices — sits between table and
sections at +0x38; the FM path reads neither.

A song may also be stored without a module: a bare `PLX` section with no
tag, no section table and no samples. Both forms reach the same reader,
which sniffs the tag: a block that opens on `MTCVTS` has its section 0 cut
out first, a block that opens on `PLX\0` is the section.

## The song — the `PLX` section

```
char tag[4]     ; "PLX\0" — the driver compares it (0xe3b, literal 0xe37)
u8   speed      ; ticks to a row
u16  tempo      ; the timer period, in PIT cycles (1 193 182 to a second)
u16  track[9]   ; one stream per OPL channel, PLX-relative; 0 = no stream
```

A track is a run of events. Each is one flags byte, its operands in bit
order, and a closing delay byte — rows until the channel's next event, 0
chaining several events into one row (the tick drains a channel until its
next event lies ahead, `0xadf`):

| Flags | Operand | What happens (`0x890`) |
|---|---|---|
| `0x00` | — | The stream ends: the channel falls idle and its key goes off. No delay byte follows |
| `0x80` alone | — | Nothing but the delay byte |
| bit 0 | `u16` | Instrument: the operand is a PLX-relative record offset (below) |
| bit 1 | `u8` | Volume: a raw carrier-level byte, kept as loudness, not written |
| bit 2 | — | Key off — the `B0` write happens only where the key is on |
| bit 3 | `u8` | Note: twice the semitone, a byte offset into the driver's note table |
| bit 4 | `u16` | A raw `A0`/`B0` register pair (only read when bit 3 is clear) |
| bit 5 | — | Key on: OR 0x20 into the pair — alone, it retriggers the held pair |
| `0x40` as the rest | `u16` | Tempo: a new period, re-clamped and re-registered |
| anything else left | — | The channel is killed (`0xa6f`) |

With bits 3, 4 or 5 the driver builds one sixteen-bit pair — `A0` low,
`B0` high — stores it, and writes both registers. The key bit is the
flag's **or the register shadow's** (`0x9a5`): a note without bit 5 keeps
a key that is already down sounding, and the stored pair carries the key
bit in its high byte.

An instrument record is twelve bytes at its offset. The driver skips the
first and writes ten of the remaining eleven straight to the chip — `C0+ch`;
`0x20/0x40/0x60/0x80/0xE0` + modulator; `0x20` + carrier; `0x60/0x80/0xE0`
+ carrier. The eighth is not written: it becomes the channel's loudness
word, `((b & 0x3F) ^ 0x3F) | (b & 0xC0) << 8` — level inverted into
loudness, the KSL bits kept beside it — and a volume event's byte goes
through the same transform onto the same word (`0x960`).

The note table (`0x6d6`) is 96 words of `block << 10 | fnum`, C-0 to B-7:
twelve F-numbers 343 363 385 408 432 458 485 514 544 577 611 647, the
block the octave.

## The digital part — the `MDH` chunk

What `DMABLAST.DRV` plays of a song beside the Ad Lib part: up to eight
voices, each striking one sample at one of four levels. Entry 21 (`0x25ea`)
checks the chunk and the start callback (`0x2506`) reads it, offsets from
the tag:

```
char tag[4]     ; "MDH\0" — compared as "MD" and "H\0" (0x2606, 0x260c)
u16  unread
u16  events     ; offset of the event stream (0x25bb)
u16  voices     ; 1 to 8 (0x2612)
struct {        ; one per voice, from +0x0a
  u8  unread
  u8  priority  ; the channel's, read only by the mixer's overload handling
  u16 sample    ; index into the song's sample table (0x2579)
  u8  level[4]  ; the four levels an event picks from (0x255d, 0x2565)
} voice[voices]
```

The sample table is the host's: a module's sample sections, as far
pointers. The stream is read a byte at a time (`0x2342`–`0x2402`):

| Byte | What it is |
|---|---|
| `000vvvll` | voice `v`'s volume from level `l` times the song's scale, and its sample struck from the start |
| `20 00` | the end of the stream |
| `01xxxxxx` | a pause of one row, then the event `xxxxxx` (`0x2365`, the byte read again) |
| `10dddddd` | a pause of `2·d` rows (`0x2378`) |
| `11dddddd e` | a pause of `(d << 8) + e` rows (`0x237d`) |

There is no note-off and no pitch: a sample runs to its end or until its
voice strikes it again, at the period its header names. Any other byte
where an event is due — `20` before anything but `00`, `21` to `3f` — runs
the end (`0x2402` → `0x2500`) from a stack it does not return through, and
the reader refuses a stream that reaches one, or that runs out before its
`20 00`.

## The sample — an `SM8` block

A sample is a section of a module or a block on its own, and the header is
the same either way:

```
char tag[4]     ; "SM8\0"
u16  volume     ; 0x0100 in every block measured — read signed by the play entry
u16  length     ; bytes of PCM to follow — the block's length less ten
u16  period     ; one sample's length in PIT cycles
u8   pcm[]      ; unsigned, mid-point 0x80
```

The period is read the way `MUSADL.DRV` reads its tempo, in PIT cycles,
and the digital driver says so twice — see
[the drivers](drivers.md#the-sample-path-of-the-dma-drivers) for the table
that turns it into the DSP's clock, and for the mixer that steps through
it. The second word is the volume, and the play entry reads it signed: at
zero or below it takes the direct path, which plays at full scale; on the
mixer path `0x0100` stands for the default volume a host sets with entry
22, and every other value gives the channel `0x0100` — the entry loads
`0x100` (`0x2090`) and replaces it only with the default (`0x2093`–`0x2098`),
never with the word ([the mixer](drivers.md#the-mixer-of-dmablastdrv)).

## See also

- [The drivers](drivers.md) — what plays these
- [PSM 2](README.md) — the component's index
