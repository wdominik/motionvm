[← Documentation index](../../README.md)

# The `.386` driver archives

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Three of them ship with the game: `HMIMDRV.386` holds the MIDI drivers,
`HMIDRV.386` the digital ones, `HMIDET.386` the detection stubs. Each is a
bundle of small 32-bit flat images, and which one is used is decided by the
**device id** written into `HMISET.CFG` by `SNDSETUP`.

## Layout

A 44-byte file header, then one record per driver: a 48-byte header followed by
the image.

| Offset | Type | File header |
|---|---|---|
| `0x00` | `char[32]` | `3`, NUL-padded — the archive's own name |
| `0x20` | `u32` | Number of records |
| `0x24` | `u32` | Size of this header, 44 in all three |
| `0x28` | `u32` | The file size, **truncated to sixteen bits and sign-extended** |

| Offset | Type | Record header |
|---|---|---|
| `+0x00` | `char[32]` | File name, e.g. `fmmidi3.com` |
| `+0x20` | `u32` | Bytes to reserve for the loaded image |
| `+0x24` | `u32` | Bytes of image following this header |
| `+0x28` | `u32` | **Device id** |
| `+0x2c` | `u32` | Flags — `0x4000` and `0x8000` mark the two halves of a driver pair, `0xc000` both. Zero for every MIDI driver |

**There is no directory.** The next record starts at `here + 0x30 + image`, and
the only way to reach the last driver is to walk the chain. It closes exactly on
the file size in all three files, which is what makes the walk checkable.

The `u32` at `0x28` is not usable as a length: all three archives are larger
than 64 KB, so the value has lost its top bits. `0xFFFFCC5C` for the 117,852
bytes of `HMIMDRV.386` (`0x1CC5C`), `0x473E` for the 83,774 of `HMIDET.386`
(`0x1473E`), `0xFFFFD785` for the 317,317 of `HMIDRV.386` (`0x4D785`).

`mem` exceeds `image` by the driver's uninitialized data — 92 bytes in most of
them, 1732 in `fmmidi3.com`.

## `HMIMDRV.386` — eight MIDI drivers

| Device | Name | Bytes | |
|---|---|---|---|
| `0xA000` | `smii.com` | 503 | Sound Master II |
| `0xA001` | `mpu401.com` | 677 | MPU-401 |
| `0xA002` | `fmmidi.com` | 12,684 | **OPL2** — a Sound Blaster or SB Pro |
| `0xA004` | `mt32.com` | 664 | Roland MT-32 |
| `0xA006` | `intspkr.com` | 772 | PC speaker |
| `0xA008` | `awemidi.com` | 53,940 | AWE32 |
| `0xA009` | `fmmidi3.com` | 14,416 | **OPL3** — [the one the game uses](../engine/fm-driver.md) |
| `0xA00A` | `gusmidi.com` | 33,768 | Gravis Ultrasound |

Only the two FM drivers can make a sound from what Dunkle Schatten 2 ships: `ENGINE.EXE`
uploads `MELODIC.BNK` and `DRUM.BNK` for those two ids and no others. Everything
else expects an instrument set the hardware brings along.

`HMIDET.386` holds 80 detection stubs and `HMIDRV.386` 100 digital drivers, both
in the same format — neither is reachable from the game, because the digital
side of the sound layer is never called. See [Audio](../engine/audio.md).

## Verified

All three shipped archives walk to their last byte, with the driver chain
ending exactly at the end of the file and every driver reserving at least as
much memory as its image is long: **`HMIMDRV.386` holds 8 drivers,
`HMIDET.386` 80 and `HMIDRV.386` 100**, and all three carry the archive
name `3`.

## See also

- [The FM driver](../engine/fm-driver.md) — what `fmmidi3.com` does
- [Audio](../engine/audio.md)
- [Other files](../../games/ds2/other-files.md)
