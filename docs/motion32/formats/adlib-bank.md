[← Documentation index](../../README.md)

# Ad Lib Instrument Banks — MELODIC.BNK and DRUM.BNK

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit-enviro).*

The FM patches the game's music is actually played with. Both files are 5404
bytes and both are the standard Ad Lib bank layout.

`ENGINE.EXE` opens them **by name** during sound initialization (`0x850DB`,
`0x8511C`) and uploads them to the MIDI driver (`0x8F7C4`) — and it does so
**only** for the FM device ids `0xA002` and `0xA009`. The drivers never open
them; the strings do not occur in any `HMI*.386`. A missing or short bank
silently disables music.

## Header

| Offset | Type | Description |
|---|---|---|
| `0x00` | `u8[2]` | Version, major then minor — `0.0` |
| `0x02` | `char[6]` | Signature `ADLIB-` |
| `0x08` | `u16` | Instruments used |
| `0x0a` | `u16` | Instruments named |
| `0x0c` | `u32` | Offset of the name table — 28 |
| `0x10` | `u32` | Offset of the instrument records — 1564 |
| `0x14` | `u8[8]` | Padding, zero |

**Both counts read 127 while there are 128 of each record.** They are a highest
index, not a count. The arithmetic only closes at 128:

```
28 + 128 × 12 + 128 × 30 = 5404
```

which is the file size to the byte. A reader that sizes its tables from the
header loses the last instrument — in the melodic bank that is `GUNSHOT`. Size
them from the two offsets instead.

## Name table — 12 bytes an entry

| Offset | Type | Description |
|---|---|---|
| `+0` | `u16` | Index of the instrument record this entry points at |
| `+2` | `u8` | See below — it means different things in the two files |
| `+3` | `char[9]` | Name, NUL-padded |

`MELODIC.BNK` is the General MIDI melodic set in program order — index 0
`PIANO1`, 48 `STRINGS`, 127 `GUNSHOT` — and the byte at `+2` is **1** in all
128 entries.

`DRUM.BNK` uses that byte for a **MIDI note number** instead: index 36 carries
note 35 and the name `Kick`. Sixty entries are named, at indices 28…87
(`clap`, `rimshot`, `Snare`, `Openhat`, `Crash`, `Ride`, `Cowbell`, `conga`,
`timbale`, `agogo`, `clave`, `taiko`, …); the rest are literally `Blank`, and
only **34** of the 128 records are distinct.

## Instrument record — 30 bytes

```
u8  percussive        non-zero for a percussion patch
u8  voice             which percussion voice, when percussive
    modulator, 13 bytes:
u8  key_scale_level        u8  frequency_multiplier   u8  feedback
u8  attack                 u8  sustain                u8  sustaining
u8  decay                  u8  release                u8  output_level
u8  amplitude_vibrato      u8  frequency_vibrato      u8  key_scale_rate
u8  connection
    carrier, the same 13 bytes
u8  wave_select_modulator  u8  wave_select_carrier
```

The bytes are **unpacked** — one field per byte, not yet folded into OPL
registers.

## What the driver makes of a record

`fmmidi3.com` rewrites the thirty bytes **in place** into OPL register values
(`0x0CD6`) and stamps `'H'` over byte 2 of the bank header so it cannot run
twice:

| Register | Built from | Ends up at byte |
|---|---|---|
| `0x20 + op` | `(am << 7)｜(vib << 6)｜(sustaining << 5)｜(ksr << 4)｜multiple` | 11 / 24 |
| `0x40 + op` | `(ksl << 6)｜level` | 2 / 15 |
| `0x60 + op` | `(attack << 4)｜decay` | 5 / 18 |
| `0x80 + op` | `(sustain << 4)｜release` | 6 / 19 |
| `0xC0 + ch` | `(feedback << 1)｜connection` — **from the modulator only** | 14 |
| `0xE0 + op` | the waveform bytes, unchanged | 28 / 29 |

The carrier's `feedback` and `connection` are discarded, and `percussive` and
`voice` are never read.

**Three records are never converted.** The pass bounds itself by the header's
count minus two, and that count is one short of the truth — so instruments 125,
126 and 127 are played from raw Ad Lib fields sitting at the register offsets.
See [The FM driver](../engine/fm-driver.md) for the rest.

## Verified

Both shipped banks — `MELODIC.BNK` and `DRUM.BNK` — are 5404 bytes and hold
**128 name entries and 128 instrument records**, not the 127 their headers
claim, and all 128 slots of each resolve through their name entry to a
record. The melodic bank is in program order; the drum bank's key bytes are
MIDI note numbers, and its 128 slots are filled by 34 distinct patches.

## See also

- [The FM driver](../engine/fm-driver.md) — what the driver does with a record
- [Audio](../engine/audio.md) — who loads the banks and when
- [Other files](../../games/ds2/other-files.md)
