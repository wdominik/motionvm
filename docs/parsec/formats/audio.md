[← Documentation index](../README.md)

# Audio — `PSMP2L.DAT` and `PSMDRV.DAT`

*PARSEC — Dunkle Schatten (1994): the music library and the driver file, as `XP01.EXE` reads them. The PSM 2 songs, samples and drivers themselves are a component of their own, documented under [PSM 2](../../psm/README.md).*

The game's music is PSM 2: songs and samples in a library file, and a
resident driver per sound device in a driver file. Neither the song format
nor the driver belongs to the game, and both are on the
[PSM 2 pages](../../psm/README.md); what is the game's is how its two files
wrap them.

## `PSMDRV.DAT` — the drivers

One [RIB](containers.md) of the program's sound drivers, each resource
itself a RIB holding one file — a RIB inside a RIB. The program installs
**resource 5**, `MUSADL.DRV` (4480 bytes once unpacked), as the music
driver (`0x5B7B`), and **resource 1**, `DMABLAST.DRV` (9840 bytes), as the
digital driver beside it (`0x5BB3`, `0x5BEC`). Resource 0 is the hardware
detector, resources 2 to 4 the other cards' digital drivers.

## `PSMP2L.DAT` — the library

Read by `13ECh:0009h` (`0x13EC9`): the file opened (`0x13F00`), `34h` bytes
of header read (`0x13F2D`) and the first sixteen held against
`PSMLST PSM 2.00\0` (`0x13F3D`; the program's message for a mismatch is
*P2L: Ungültiges Signum!*), then the three tables read whole (`0x13F74`,
`0x13F9F`, `0x13FCA`).

| Offset | Field |
|---|---|
| `+00` | `"PSMLST PSM 2.00\0"` (`DS:15E8h`) |
| `+10h` | Table A: `u32` offset, bound, length — `SM8` samples, 12-byte rows |
| `+1Ch` | Table B: the same — `PLX` songs, 12-byte rows |
| `+28h` | Table C: the same — module heads, 16-byte rows |

**The middle word of a triple is a bound, not a length.** It is what a fetch
of one entry is checked against (`0x1418C`, `0x14364`; *P2L: Ungültiger
Index!*), and it is smaller than the table. Table A holds 60 rows and a
bound of 21 — the effects, the only samples fetched one by one; table B holds
16 rows and a bound of 0, because no song is fetched on its own: it is
reached through a module head. The rows are counted from the length.

A row of A or B is `u32 offset, u32 stored, u32 size`; a row of C is
`u32 assembled, u32 offset, u32 stored, u32 size`. An entry is read the way
`0x1442C` reads it: `stored` bytes at `offset`, and where that is not `size`,
unpacked into `size` by `1E05h:0000h` (`0x14492`), the
[RIB LZ codec](lz.md). All 16 songs are packed; all 60 samples and all 16
heads are stored.

## Module heads

A **module head** is a PSM 2 module whose sections are still to come:
`MTCVTS PSM 2.00\0`, at `+10h` its own length — the offset its section 0
will have — eight words at `+14h` naming rows of A (`FFFFFFFFh` for none),
and at `+34h` the row of B that is its song. A game asks for music by the
module's number, and the loader `13ECh:028Fh` (`0x1414F`) answers in one of
two ways, by whether the digital driver is installed (`[1EC6h]`,
`0x14162`):

- **With it** (`0x14186`–`0x1434C`), which is how the game plays, it
  assembles the module: the head, its song word cleared to 0 (`0x1425C`),
  the song unpacked behind it (`0x14288`), then each of the eight sample
  words — `FFFFFFFFh` made 0, any other replaced by the offset the sample
  is appended at. The head carries the song's `MDH` chunk at `+38h`, which
  with the sample words is what the digital driver plays as the song's
  digital voices.
- **Without it** (`0x1435A`) it holds the number to C's bound
  (`0x14364`), reads the head (`0x14398`), takes its `+34h` word as the row
  of B (`0x143A8`), and answers with that row read and unpacked
  (`0x14424`): the song's bare `PLX` section, which is all the FM driver
  plays. That row of B is indexed unchecked (`0x143E2`, `0x14414`).

The effects are rows 0 to 20 of table A, each fetched on its own by
`13ECh:0210h` (`0x140D0`): with the digital driver only, the row held to
A's bound (`0x140E6`) and read (`0x14147`) — an `SM8` block, 8-bit
unsigned mono with its own sample period.

Which module a room plays and which effect when are the
[music page](../engine/music.md)'s.

## See also

- [PSM 2](../../psm/README.md) — the `MTCVTS` and `PLX` format, and `MUSADL.DRV`
- [The RIB LZ codec](lz.md) — how a packed song unpacks
- [Resource containers](containers.md) — the RIB the drivers sit in
