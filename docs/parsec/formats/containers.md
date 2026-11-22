[← Documentation index](../README.md)

# Resource Containers — FGROUP and RIB

*PARSEC — Dunkle Schatten (1994): the two container shapes every file in its `RESOURCE/` directory is built from, as `XP01.EXE` reads them.*

Every `.DAT` file in `RESOURCE/` is one of two shapes, and both carry the
program's own names, which the unpacked executable spells out: the outer
table is an **FGROUP**, its entries **FDIR**s; the inner bundle is a
**RIB**. Most files are a single RIB. Three are an FGROUP of RIBs:
`DSROOM.DAT`, the 60 room slots, and the title and end sequences
`INTROS.DAT` and `EXTROS.DAT`. The two are told apart by their first bytes
and the header arithmetic below, not by the file name. `PSMP2L.DAT` is
neither; it is the music library and has its [own layout](audio.md).

All multi-byte values are little-endian.

## The directory both share

The outer table and a RIB's inner one have the same shape:

```text
u32 off[n]
u32 0
u32 size[n]
# the entries follow
```

The first offset is `4*(2n+1)` — `n` offsets, the zero word and `n` sizes —
so the first entry starts right behind the table, and that equation is how
`n` is found. In each of Dunkle Schatten's three FGROUPs,
`off[i] + size[i] = off[i+1]`, and the last entry ends at the end of the
file.

## FGROUP

An FGROUP is that directory at the start of the file, and each entry is one
**slot**: a RIB, or the literal `NULLPTR\r\n` (nine bytes) that marks a slot
as deliberately empty. Slot 0 of `DSROOM.DAT` is that marker.

A slot that is neither a RIB nor the marker is raw bytes. The intro and end
sequences keep their palettes that way — DAC banks of 128, 192 and 224
colors in the `start, count` form of the [graphics](graphics.md) page — and
`INTROS.DAT` three 34-byte texts.

## RIB

```text
"RIB\0"          # 52 49 42 00
u32 bufSize      # the size once decoded
# the payload
```

The payload is copied to the start of a `bufSize` buffer and decoded in
place by the [RIB LZ codec](lz.md). A payload exactly `bufSize` long is
stored rather than packed, and the decoder returns at once; no RIB in Dunkle
Schatten's files takes that path.

Decoded, the buffer opens with the directory above, and the resources sit
at their offsets. The last resource runs to the end of the buffer with no
slack — `off[n-1] + size[n-1] = bufSize` — in all 65 of the game's RIBs
that carry a table; in every live `DSROOM.DAT` slot that last resource is
the `RDV1.00` room.

**A second flavor carries no table.** Seven slots of `INTROS.DAT` and
`EXTROS.DAT` decode to a single object, six of them 320×200 stills. The RIB
layer does not care: a buffer that does not open with the directory shape
is one object.

Always decode before parsing. The first bytes of a packed file look like
those of the decoded buffer only where the codec's tokens happen not to
overlap the table's tail.

## Where each file goes

| File | Shape | What it holds |
|---|---|---|
| `DSROOM.DAT` | FGROUP of 60 | The rooms, one slot per location number — see [rooms](rooms.md) |
| `INTROS.DAT`, `EXTROS.DAT` | FGROUP | The title program's logos, stills, palettes and texts, and the end sequence |
| `CMBMOU.DAT` | RIB | The pointer's palette bank and cels, and the font — see [graphics](graphics.md) |
| `PANELS.DAT` | RIB | The strip under the playfield and its palette bank |
| `PLAY00.DAT` | RIB | The player's palette bank and walk cels |
| `DIALOG.DAT` | RIB | The two global dialogue banks and the player's `AN` block — see [dialogue](dialogue.md) |
| `PSMDRV.DAT` | RIB | The sound drivers; resource 5 is `MUSADL.DRV` — see [audio](audio.md) |
| `PSMP2L.DAT` | P2L | The music library — see [audio](audio.md) |

## See also

- [The RIB LZ codec](lz.md) — how a RIB's payload decodes
- [Rooms](rooms.md) — what a `DSROOM.DAT` slot holds
- [Graphics](graphics.md) — pictures, cels and palette banks inside the RIBs
- [Executables](executables.md) — the program files beside `RESOURCE/`
