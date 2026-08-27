[← Documentation index](../../README.md)

# The FM driver — `fmmidi3.com`

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

The music the game actually makes is made by this: 14,416 bytes of 32-bit code
inside [`HMIMDRV.386`](../formats/driver-archive.md), device id `0xA009`, which
the game's `HMISET.CFG` selects as "Sound Blaster 16". It takes MIDI messages
from the [HMI sequencer](../formats/hmi.md) linked into `ENGINE.EXE` and writes
OPL3 registers.

**Everything on this page was read out of that image**, and the whole of it was
then checked against a register-level recording of the original engine playing
block 25 from its opening screen — every value it sent to the OPL3's ports. How
that check is run, and the one place the reading and the recording part, are in
[Departures](../../departures.md).

Addresses are load addresses in the image: file offset minus `0x3218`.

## The way in

`AX` indexes a jump table at `0x00BB`:

| `AX` | Entry | |
|---|---|---|
| 0 | `0x067F` | the driver's name |
| 2 | `0x0685` | **a MIDI message** |
| 4 | `0x068B` | **initialize**, with the port number |
| 6, 8 | `0x054A` | shut down — both ordinals reach the same body |
| 10 | `0x05A9` | **load an instrument bank** |

Ordinal 10 has to be called twice: `MELODIC.BNK` first, then `DRUM.BNK`; a flag
at `0x352C` says which of the two far pointers the bank is stored in.
`ENGINE.EXE` does exactly that (`0x850DB`, `0x8511C`, `0x8F7C4`) and **only for
the FM device ids** `0xA002` and `0xA009` — every other MIDI device gets no bank
and the two files go unread.

Two loose ends, stated as such: what ordinal 1 would be is not established, and
the fuller reset that is plainly meant for ordinal 8 sits at `0x0793` with
nothing reaching it.

## The switch-on sequence

`0x1D1A` and the silencing pass it calls at `0x1D8E`, in this order:

```
0x105 = 0x01      leave OPL2 compatibility            (second bank)
0x104 = 0x00      no four-operator channels           (second bank)
0xB0+v = 0        for v = 0…8, on both banks
0xBD  = 0xC0      deep vibrato, deep tremolo          (first bank only)
```

`0x104` staying zero is why a recording of the game identifies itself as "dual
OPL2" rather than OPL3: the driver never uses a four-operator channel. And the
rhythm bits of `0xBD` are never set — percussion is ordinary voices.

## Nine voices, two banks, one sound each

`program_voice` (`0x2162`) writes **every** register twice, once to the base
port and once to base + 2. The nine channels of the first bank and the nine of
the second are therefore not eighteen voices but nine, each sounding on both
sides at once. What differs between the two copies is the `0xC0` byte: the first
bank gets `| 0x20`, the second `| 0x10`.

Those are the chip's two output enables, and that is this driver's entire stereo
mechanism: pan is made by writing a *quieter* total level to one of the two
copies (`0x276D`), never by touching the enables.

Eleven registers, in this order, modulator first:

```
0x20+mod  0x40+mod  0x60+mod  0x80+mod  0xC0+voice  0xE0+mod
0x20+car            0x60+car  0x80+car              0xE0+car
```

**The carrier's `0x40` is missing on purpose.** `0x23F7` stores it into the
shadow at `0x3548` and stops there, because the velocity calculation is about to
write that register anyway. The modulator's is written *and* shadowed, so a
chained patch keeps the level its instrument asks for.

## Turning a bank into registers

`0x0CD6` folds the thirty bytes of an [Ad Lib record](../formats/adlib-bank.md)
into OPL register values **in place**, and stamps `'H'` over byte 2 of the bank
header so it cannot run twice.

| Register | Built from | Lands at byte |
|---|---|---|
| `0x20 + op` | `(am << 7)｜(vib << 6) ｜(sustaining << 5)｜(ksr << 4)｜multiple` | 11 / 24 |
| `0x40 + op` | `(ksl << 6)｜level` | 2 / 15 |
| `0x60 + op` | `(attack << 4)｜decay` | 5 / 18 |
| `0x80 + op` | `(sustain << 4)｜release` | 6 / 19 |
| `0xC0 + ch` | `(feedback << 1)｜connection` — **from the modulator only** | 14 |
| `0xE0 + op` | the waveform bytes, unchanged | 28 / 29 |

The carrier's `feedback` and `connection` fields are discarded, and the record's
`percussive` and `voice` bytes are never read at all.

**The pass stops two records early.** `0x0D17` loops while `i < used - 2`, and
`used` is the header's count, which reads 127 in both shipped banks while 128
records are present. Instruments 125, 126 and 127 are played from *unconverted*
bytes — the driver reads the same eleven positions and finds raw Ad Lib fields
in them. In `MELODIC.BNK` that is `HELICOPT`, `APPLAUSE` and `GUNSHOT`. No song
in the game uses them.

## The four tables

Read from the image, not written down, because two of them are hand-made and
neither is quite what the arithmetic gives.

| Address | Length | |
|---|---|---|
| `0x3614` | 103 × `u32` | `(block << 10)｜fnum` for notes 12…114 — every note the chip can express. Indexed `note - 12`; the compiler folded the offset into the address, which is why the image also refers to it as `0x35E4` |
| `0x35B9` | 18 bytes | operator offsets, modulator then carrier, for the nine channels |
| `0x35D0` | 64 bytes | the velocity curve, indexed `velocity >> 1`, falling from 63 to 0 |
| `0x37B0` | 12 × `u32` | the same notes one block higher — the f-number halved — indexed **backwards** by `11 - (note - 12) % 12` |

The halved table's seventh entry is 248 where halving gives 243. It is HMI's
slip and it stays: a corrected table would be a different driver.

## Velocity

```
c     = curve[velocity >> 1]
t     = (0x40 - c) * 2
level = (0x2000 - (0x40 - patch_level) * t) >> 7
```

with the patch's key-scale bits kept and the six level bits replaced. The
velocity that goes in is not the one on the wire: note-on first scales it by the
channel's volume (`0x1081`), `velocity * ((volume << 7) / 0x7F) >> 7`. The
carrier always gets the result; the modulator only when the patch's connection
bit is set.

One worked example, because it pins the whole chain down at once. Block 25's
first note on channel 5: velocity 100, controller 7 at 90.
`(90 << 7) / 127 = 90`, `90 * 100 >> 7 = 70`, `curve[35] = 11`, `t = 106`,
`level = (8192 - 64 × 106) >> 7 = 11`. The recording's byte for register `0x44`
is `0x0B`.

## Panning

`0x276D`, called on controller 10 and again at every note-on.

```
weight = (pan >= 0x40 ? 0x7F - pan : pan) * 2      126 at the center, 0 at either end
for each voice of this channel:
    x     = (volume * velocity) >> 7
    x     = (weight * x) >> 7
    level = the velocity law, applied to x
    write it to ONE bank: the second when pan < 0x40, the first otherwise
```

The other bank keeps the unattenuated value the note-on wrote. Same worked
example: pan 74, so weight 106; `(90 × 100) >> 7 = 70`, `(106 × 70) >> 7 = 57`,
`curve[28] = 15`, `level = 15`. The recording writes `0x0F` to register `0x44`
on the first bank — and leaves `0x144` at `0x0B`.

**Which side that is remains open.** The chip's own assignment is `0xC0` bit 4
for channel A and bit 5 for channel B, and in the ordinary wiring A is left and
B is right — which makes the first bank, with its `| 0x20`, the right side. The
driver quietens the first bank when controller 10 is at or above 64, i.e. it
quietens the right as the pan moves right. Read literally that is backwards, and
no recording can settle it: a recording shows which register was written, not
which speaker it came out of.

## Notes

**Note-on**, `0x0EF9` for the melodic channels and `0x12F4` for channel 9:

1. Allocate a voice, and key it off (`0x20D8`).
2. Write the old voice's release rate as `shadow | 0x0F`, **five times over**
   (`0x0F5F`) — carrier then modulator, both banks, so the envelope is down
   before the new patch lands. Only the first of the five changes anything.
3. Program the patch.
4. Velocity to total level, as above.
5. Pan.
6. Frequency, then key off and on again — the pulse at `0x2A80` is what makes a
   repeated note restart its envelope instead of continuing.
7. If the channel has ever bent, recompute the frequency and write it again.

**Note-off**, `0x1695`: with the sustain pedal down, the note goes into a
per-channel queue of 32 and the thirty-third is dropped. Otherwise every voice
playing that note **for that channel** is keyed off — and that is all that
happens. `0xB0` with bit 5 cleared, on both banks; no level write, no envelope
change. The patch and the frequency stay behind, which is what lets the next
note on that voice reuse them.

**Percussion**, channel 9: the patch comes from `DRUM.BNK` indexed by the MIDI
**note number**, and the pitch from byte `+2` of that record's name-table entry
(index 44 is `clsdht47`, played at note 47). No rhythm mode, no bend.

## Voice allocation

`0x2033`, in three steps and no others — no round-robin, no priority, no
oldest-first:

1. The first voice whose note is zero.
2. Otherwise, the first MIDI channel `0…15` that has **never sent a pitch
   bend** and owns a voice; that voice is taken.
3. Otherwise the requesting channel's own number, folded by nine if it is nine
   or more.

Step 2 has a consequence the code does not spell out. The pitch-bend handler
turns channel 9 away at the door (`0x1ABB`) — percussion does not bend — so
`bent[9]` never becomes true. In a piece where every melodic part bends at least
once, and every one of the game's does, **channel 9 is the only channel voice
stealing will ever take from**: when the ninth voice runs out, it is the drums
that get cut off.

## Pitch bend

`0x1B5F`. The bend's high byte alone is used; the low byte is discarded. Around
the center at `0x40` the driver interpolates in thousandths between the note and
the note a bend range away — controller 102 sets that range in semitones, and it
resets to 2.

When the two notes fall in different blocks the difference of the packed values
is meaningless, so the driver expresses the lower one an octave up out of the
`0x37B0` table and takes the difference of the f-numbers instead.

## Faults reproduced on purpose

Three, all audible, all left in.

1. **The `0x80` shadow overruns into the `0xA0` shadow.** `0x3568` is sixteen
   bytes — `0x3578` starts the per-voice `0xA0` shadow — but it is indexed by
   operator offset, and those run to `0x15`. Operators `0x10`…`0x15`, which
   belong to voices 6, 7 and 8, therefore read and write the f-number shadows of
   voices 0…5. The recording shows it: at the first note ever played on voice 6,
   with nothing having touched its registers, the release write goes out as
   `0xBF` — and `0xB0` is the low byte of the f-number voice 0 was playing.
2. **A downward bend indexes the bend range by the OPL voice** (`0x1BCD`) where
   every other site indexes it by the MIDI channel (`0x1BFA`, `0x1C58`).
3. **The percussion path tests the melodic patch's connection bit** (`0x1487`):
   it reads `program[9]` out of `MELODIC.BNK` to decide whether the drum patch's
   modulator gets the velocity, instead of looking at the drum record it is
   playing. `0x2919` does the same in the pan path.

## Controllers

Five reach the chip. Everything else, and the whole `0xA0`, `0xD0` and `0xF0`
classes, is dropped.

| | |
|---|---|
| 7 | volume — folded into velocity, never sent as a level of its own |
| 10 | pan |
| 64 | sustain pedal, with the note queue |
| 102 | pitch bend range, in semitones |
| 121, 123 | reset: volume 127, no sustain, bend centered, range 2 |

## The engine's own half

Two things happen before a message ever reaches the driver, both in the HMI
sequencer inside `ENGINE.EXE`:

- **Controller 7 is cached** as `value × song × master × channel / 127³`
  (`0x98C05`…`0x98C4D`). Nothing in the game moves those three factors, and the
  recording proves they are at maximum — any other value would change the drum
  levels, and they match to the byte.
- **On channel 9, and only there, that cache is folded into note velocity**
  (`0x98A89`, `velocity × chan[0x1C] / 127`). The controller still reaches the
  driver, so the drums are attenuated twice over. Tune 25's hi-hat: velocity 80
  at controller 7 = 105 becomes 66 here, and the driver's `(105 × 66) >> 7 = 54`
  gives attenuation `0x10` — which is the recording's byte.

  The code at `0x98C56` withholds the controller from the device when a field of
  the channel record reads 9. What that field is has not been established, and
  in the recording the controller plainly does reach the driver on channel 9.

## Open questions

Three things `fmmidi3.com` does not answer for itself:

- **Which bank is which side.** The driver mirrors every voice onto both
  register banks and pans by quietening one of them. Bank 0 takes `0xC0`
  bit 5 and bank 1 bit 4, which on the chip are channel B and channel A —
  right and left in the ordinary wiring. The driver quietens bank 0 as
  controller 10 moves *right*, which read literally is backwards. A register
  recording cannot settle it; only the wiring can.
- **Ordinal 1**, and why ordinals 6 and 8 reach the same body while the
  fuller reset at `0x0793` is unreachable.
- **`chan[8]`** in the engine's channel record. `0x98C56` withholds
  controller 7 from the device when it reads 9, and a recording shows the
  controller reaching the driver on channel 9 all the same — so the field is
  not the channel number.

A measured difference between the original and the reading of it — one voice
allocation out of 17,047 register writes — is recorded under
[Departures](../../departures.md).

## See also

- [Departures](../../departures.md) — how the reading is checked, and where it parts
- [Audio](audio.md) — what the engine asks for and when
- [The `.386` driver archives](../formats/driver-archive.md)
- [Ad Lib instrument banks](../formats/adlib-bank.md)
- [HMI song format](../formats/hmi.md)
