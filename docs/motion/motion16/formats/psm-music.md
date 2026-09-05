[← Documentation index](../../README.md)

# PSM 2 Music

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere, in `STERN.EXE` with Falsches Spiel mit Eddie M. and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

Ten blocks of the [DATA container](container.md) are music: PSM 2
modules, played on an OPL2 by `MUSADL.DRV`. The driver file is the whole
player — sequencer, fade engine and register back-end in 4480 bytes behind
a `MUS\0` header — and `ENVIRO.EXE` keeps only a loader, a call table and
a timer host around it. File offsets below are `MUSADL.DRV`'s unless
marked otherwise; they are also its code-segment offsets, because the
installer normalizes the loaded image to a paragraph boundary (`ENVIRO.EXE`
`198c:0193`).

## The module

Sixteen bytes of tag, `MTCVTS PSM 2.00\0`, then a table of up to ten
`u32` section offsets at +0x10, ascending, ending at the first zero. The
loader (`1696:017e`) converts them to far pointers in place and hands the
driver the first section — the `PLX\0` song, at offsets 767 to 1640 in
the shipped modules. Every later section is an `SM8\0` sample for the
digital-sample driver the game can install beside the Ad Lib one, and an
`MDH\0` block sits between table and sections at +0x38; the FM path reads
neither.

**The earliest game has no module at all.** All fourteen of Victor Loomes'
tunes are a bare `PLX` section stored as the block itself — no tag, no section
table, no samples, which fits a game that ships no digital drivers and has no
sample word in its kernel. Both forms reach the same reader, which sniffs the
tag: a block that opens on `MTCVTS` has its section 0 cut out first, a block
that opens on `PLX\0` is the section.

## The song — the `PLX` section

```
char tag[4]     ; "PLX\0" — the driver compares it (0xe3b, literal 0xe37)
u8   speed      ; ticks to a row
u16  tempo      ; the timer period, in PIT cycles (1 193 182 to a second)
u16  track[9]   ; one stream per OPL channel, PLX-relative; 0 = no stream
```

Across the ten shipped tunes the speed runs 10 to 34 and the tempo 0x15d6
to 0x1734 — 199 to 215 ticks a second, three to twenty rows a second.

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
| anything else left | — | The channel is killed (`0xa6f`); no shipped stream carries such a byte |

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

## The driver

`MUSADL.DRV` opens with `MUS\0`, version 0x0100, fifteen entry offsets at
+0x18, seven service-pointer cells the installer fills at +0x36, and an
`NS` trailer at the offset +0x0a names.

Two builds ship. The 1995/96 games all carry the same 4480-byte file with
fifteen entries; Victor Loomes carries a 3915-byte build with **fourteen**,
and its four data tables sit `0xd0` earlier — `0x4f4`, `0x5f4`, `0x5fd` and
`0x606` against `0x5c4`, `0x6c4`, `0x6cd` and `0x6d6`. The tables themselves
are byte-identical between the two, which is what makes reading them at the
wrong offset a silent wrong answer rather than a loud one: 256 plausible
defaults and a full note table come back either way, and nothing sounds wrong
until a note comes out flat. The entry count is what says where to read. The manager (`198c:0193`) verifies
exactly that, builds fifteen five-byte far-jump stubs, and calls entry 0 —
which flushes the install state to the chip: one default per register
(`0x5c4`), `0xFF` marking the untouched ones, written in ascending
register order.

Every write goes through that 256-byte shadow (`0x7df`/`0x7ec`): a value
the shadow already holds is not sent again, so the register stream carries
**changes only** — across tunes too, because the shadow lives as long as
the driver.

The game calls four entries. `STARTTUNE ( loop n -- )` (`ENVIRO.EXE` file
`0xbf89`) loads block `n` and runs SetSong (`0x2f9`) — tag check, pointer
stored — then Play (`0x470`): notes stopped if something still plays, the
loop count stored, the volume full, the streams primed, the host timer
registered with the period. The loop count is compared unsigned against a
pass counter that starts at 1, so the `-1` every one of the game's sixteen
call sites passes means forever. `ENDTUNE` (`1696:02fd`) is FadeOut
(`0x3e2`) with 2000 ms, a 500 ms wait, then Stop (`0x4c9`) — and the wait
is the script's: the stop routine resets the tick counter and spins until
the 200 Hz reading (`110a:042e`, the driver's millisecond count over five)
comes back 100, and only then stops the driver and returns. It does nothing
at all while its own flag (`ds:18f4`) says no song is playing, and Play
(`1696:02ce`) runs the whole stop routine first when one is — which is how
Victor Loomes changes its music, with no `ENDTUNE` between locations: the
new tune begins half a second after the old one's fade started. The other
three builds' routines are the same shape (`HPPLAY.EXE` `1639:02c0`,
`BMZ.EXE` `166d:02c2`, `LL.EXE` `0e87:01f2`). The flag is the host's, not
the driver's: SetSong's success sets it (`STERN.EXE` `1611:0020`) and only
the two stop routines clear it (`1614:0015`, `161a:0026`), so a tune that
ends on its own — a loop count of 0, as `SET_POINTS`'s jingle passes —
leaves it set, and the room's next `STARTTUNE`, `ENDTUNE` or `PLAYSAMPLE`
pays the half-second wait over silence. motionvm's flag lives the same way.

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
milliseconds, each live carrier follows `master × level × loudness` with
every product truncated to a byte, reaching zero closes all carriers
(`0x3F` + KSL), and reaching 0x100 ends the fade with a full pass.

**Stop** (`0xde2`): channel 8 down to 0 — key off where a key is on, then
the release rate of both operators opened to `0x0F`, so what still sounds
dies away; the timer is unregistered.

The two scale entries — tempo (`0x282`) and pitch (`0x2ba`) — stand at
their neutral 0x100 in the file and the game never calls them: the tempo
word **is** the period (clamped no lower than 0x200), and notes sound
exactly as the table says.

## The sample — an `SM8` block

Falsches Spiel mit Eddie M. ships thirteen blocks that are a sample on their
own — the sound effects `PLAYSAMPLE` names by block number — and the header is
the section header the modules carry:

```
char tag[4]     ; "SM8\0"
u16  version    ; 0x0100 in every shipped block
u16  length     ; bytes of PCM to follow — the block's length less ten
u16  period     ; one sample's length in PIT cycles: 56 (21.3 kHz) to 179 (6.7 kHz)
u8   pcm[]      ; unsigned, mid-point 0x80
```

The period is read the way `MUSADL.DRV` reads its tempo, in PIT cycles, and
the digital driver says so twice. Its direct path (`DMABLAST.DRV` `0x0611`)
clamps the word to 255 (`0x0653`) and indexes a 256-entry table at `0x60`
for the DSP's time constant (`0x065b`), and every entry of that table is
`256 − round(period · 1000 / 1193)`: the period in whole microseconds at
1.193 cycles each — `table[59] = 0xcf`, 49 µs; `table[149] = 0x83`,
125 µs — the constant being 1.193 and not the clock's 1.193182, which at
three periods (139, 207, 244) rounds the other way. The four `DMA*.DRV`
carry the same 256 bytes. A Sound Blaster DSP plays a byte every `256 −
constant` microseconds, so the DAC's clock is the DSP's rounding of the
header's: 8000 Hz where the PIT would say 8008, 20.4 kHz where it would say
20.2. The mixer (`0x196b`) reads the word at `+8` out of the header the other
way, as the step against the mixer's own period, itself set in PIT cycles
from a percentage (entry 24, `0x0ffd`: `256 − (n · 64 / 100 + 0x90)` cycles).

**Which path plays.** The driver's play entry (17, `0x1fcf`) allocates one of
up to eight mixer channels and gives it the header's second word as its
volume — `0x0100` meaning the default the host set with entry 22 — and a play
count of the mode argument plus one; with one channel installed it takes the
direct path instead (`0x2028`): the length word becomes a single-cycle DMA
count, the bytes go to the DAC as they are, and the interrupt at the end
writes the mid-point (`0x0a06`). `STERN.EXE` installs the driver with one
channel — the sixth install argument is what the sound manager's trampoline
leaves in that slot, the caller's `DI`, which is 1 at the call (`1058:01bd`) —
so every sample the game plays is the direct path: once, on the DSP's clock,
at full scale, no volume. The host's `PLAYSAMPLE` runs the driver's StopAll
(entry 18) before every play, so one sample sounds at a time whatever the
channel count. What motionvm makes of it is a
[departure](../../departures.md#the-16-bit-machine).

**The mixer, read and not taken.** With more channels the play entry's other
branch (`0x2045`) hands the block to a channel routine (`0x196b`) that the
driver's timer runs for every active channel into a 128-word accumulator
(`0x18e3`): each output sample takes the byte under the channel's clock, less
`0x80`, times the channel's volume over 256 — `0x0100` is unity — and adds
it. The clock is the sample's period against the mixer's own, in 16.16 fixed
point: a zero-order hold while the sample is slower than the mixer, and when
it is faster one or two bytes dropped per output sample and never more, so a
sample above twice the mixer's rate plays slow. The accumulator then becomes
the DMA buffer (`0x4b0`): each word plus `0x80`, clipped to `0x00` and
`0xff`. The mixer's rate is a time constant too, `0x90 + n · 64 / 100` for
the percentage entry 24 takes — 8.9 to 20.8 kHz — and it steps down one
constant whenever the mixer has fallen behind its buffer thirty-two times
(`0x530`). The other three drivers decide between the two paths the same way
(`DMASB16S.DRV` `0x24b6`) and carry the same table; only the stereo SB16
driver's mixer runs at a rate in hertz, 11 025 to 22 050, sent with the DSP's
set-rate command (`0x41`, `0x09be`), which its direct path never uses. Nothing
in shipped play reaches any of this, so none of it is rebuilt.

## How the rebuild is checked

Two ways, as with [the 32-bit FM driver](../../motion32/engine/fm-driver.md).
The tables — defaults, operator offsets, notes — are not transcribed but
read out of the shipped `MUSADL.DRV` at the addresses above, on every run;
that check works on any copy of the game. Above it, the whole register
stream has been held write for write against an OPL capture of the
original playing from game start into location 1 — tune 8, the scripted
room change's `ENDTUNE` with its fade and stop, then tune 1 — and the two
streams are identical to the capture's end, 2 738 writes. That needs a
recording of the original, so it is a result reported rather than
something a reader can re-run.

The same driver two years earlier is held against a recording of its own:
Falsches Spiel mit Eddie M.'s intro runs `-1 24 STARTTUNE` under a poll loop
that waits for a key, and a capture of the original standing there holds the
tune for 54.8 seconds — 3 640 writes, of which the 3 551 past the recording's
opening snapshot agree with the rebuilt stream write for write, the chip's
state at the first note agreeing register for register. That is the fifth
build's `MUSADL.DRV`, byte-identical to the 1995/96 games', playing a module
whose `PLX` section sits 1688 bytes in.

The sample path is held against a recording of the same original's rendered
audio — DOSBox-X with an Ad Lib and a Sound Blaster on, `PSMCFG4.DAT` naming
`DMABLAST.DRV` — through the opening scene's effect, block 17: 31 275 bytes at
a period of 149. Aligned at its onset, the recording peaks where the rebuild
does, the DAC at full scale; it lasts 3.88 seconds in both; the two waveforms
correlate at 0.98 over the whole effect; and where the effect sounds its RMS
is 2 to 10 % under the rebuild's, the emulator's resampler interpolating
across the DAC's steps that the rebuild holds. The clock came out of this
comparison: with the period read as PIT cycles the rebuild fell 2.3 ms behind
the recording over 2.3 seconds, and on the DSP's constant it stays within two
frames of it, first byte to last. The table itself needs no recording — the suite reads
the 256 bytes at `0x60` out of every `DMA*.DRV` the game ships and holds the
formula to them, on any copy of the game.

The older driver is held against a recording of its own, and against a
different question. Victor Loomes' intro runs `0 7 STARTTUNE ANIMPLAY
ENDTUNE` — block 7, a 552-byte jingle six seconds long, started with a loop
count of zero — and the two streams agree for 744 writes: the whole tune, first
note to last, and the start of the fade behind it. Since the two builds' tables
are byte-identical, that is the check which says the v14 profile reads the
right bytes rather than plausible ones. What the recording holds past the fade
is not covered ([open questions](../../open-questions.md#motion-16-bit)).

## Open questions

- The `SM8` sample sections are the digital renderer's voices — with the
  configuration set to digital only, the tunes play sampled
  ([other files](../../games/enviro/other-files.md#the-sound-stack)) —
  but the `DMA*.DRV` drivers' sequencer over them is unread, and motionvm
  plays the Ad Lib rendition only. What is read of those drivers is the
  sample path below.
- The driver's callback protocol (`0x27a`) and the Volume entry's negative
  selectors: present, but the game leaves the callback on its stub and
  never calls the entry.

## See also

- [Blocks](blocks.md) — where the modules live, and the other block families
- [The DATA container](container.md) — the BLK segment
- [Boot and frame loop](../engine/game-loop.md) — the clock the same driver file feeds
- [Other files](../../games/enviro/other-files.md) — `MUSADL.DRV` among the shipped drivers
- [The FM driver (MOTION 32-bit)](../../motion32/engine/fm-driver.md) — the other generation's music path
