[← Documentation index](../../README.md)

# Resource Containers — `NNN.RSC`

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2 and V0.04.15/R78 with Checker 2000; what is measured here is measured on those games' files, and an address is R109's unless the page says otherwise. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

The `.RSC` files (`001.RSC`, `002.RSC`, `003.RSC`) are the game's resource
containers. Each one is a flat archive of typed, numbered items: sprites,
string tables, binary blocks, fonts, compiled script modules, and palettes.
The engine merges all containers into a single id space at startup, so a
resource is always addressed by *(type, id)* alone, never by file.

All multi-byte values are little-endian.

## File layout

| Offset | Type | Description |
|---|---|---|
| `0x00` | `u32[6]` | Slot counts, in the order: gfx, text, block, font, script, palette |
| `0x18` | `u8[24]` | Reserved; zero in all shipped files |
| `0x30` | `u32[n]` | Offset table; bit 31 = "item present" |

The number of table entries is

```
n = 2*gfx + text + block + font + script + palette
```

The gfx count appears **twice** because the graphics slots are backed by two
parallel tables: 8-bit sprites (GFX8) and 16-bit hicolor sprites (GFX16). The
GFX16 table exists in the format but holds no items in Dunkle Schatten 2; its item
layout is unknown.

In the shipped files the slot counts are 5000, 300, 900, 20, 900, 200, giving
`n = 12320` table entries per container.

## Segment order

The offset table is laid out in seven consecutive segments:

| # | Type | Slots |
|---|---|---|
| 1 | GFX8 | 5000 |
| 2 | GFX16 | 5000 |
| 3 | TEXT | 300 |
| 4 | BLOCK | 900 |
| 5 | FONT | 20 |
| 6 | SCRIPT | 900 |
| 7 | PALETTE | 200 |

This order differs from the order of the `Scanning for ...` messages the
engine prints at startup (GFX, Palettes, Blocks, Fonts, Scriptor-Files,
Text). The table order above is the one that matches the data.

## Offset table semantics

- Each entry is a file offset with bit 31 used as a presence flag; mask with
  `0x7fffffff` before use.
- Offsets are strictly monotonically increasing.
- An item's **length is the distance to the next entry's offset**. Two equal
  neighboring offsets mean an empty slot.
- The last entry is an end sentinel; it marks the end of the last item and can
  never hold data itself.
- Invariant: the first offset points exactly at the end of the offset table
  (`0x30 + 4*n`). This is the strongest single check that a reader has
  parsed the segment layout correctly.

## Multi-file overlay

The resource manager registers `ENGINE.RSC`, where the game has one, and
then every container matching the pattern `%03d.rsc` (a file name whose stem
is exactly three decimal digits), and overlays them into **one shared id
space** — six container slots, `ENGINE.RSC` slot 0 and the numbered files
after it. `->RSCPATH ( path$ slot -- )` gives one slot a directory of its
own; Checker 2000's bootstrap uses it to keep `003.RSC` on the hard disk
([its other files](../../games/checker/other-files.md#systemrsc)).

An id filled by more than one container resolves to the **lowest slot**.
This is read off the catalog `RSC.INF` carries: its record for each kind
holds a byte per id with one bit per slot that has the item, and the lookup
(`0x44710`, at `0x4485c`) walks the slots from 0 upward and takes the first
whose bit is set. Both shipped games' catalogs were held against their
containers for every kind and every id — Dunkle Schatten 2 fills sprite 1324
twice, Checker 2000 nineteen sprites and palette 147, sixteen of the sprites
with different pictures — and the catalog names the lowest slot each time,
and no slot without the item.

Contents of the shipped containers:

| Game | File | Contents |
|---|---|---|
| Dunkle Schatten 2 | `001.RSC` | 2 GFX8 items, 133 text tables, 250 blocks, 9 fonts, 86 script modules, 60 palettes |
| | `002.RSC` | 1619 GFX8 items |
| | `003.RSC` | 57 GFX8 items |
| Checker 2000 | `ENGINE.RSC` | The system font and palette, both id 0 |
| | `001.RSC` | 33 GFX8 items, 264 text tables, 80 blocks, 119 script modules, 65 palettes |
| | `002.RSC` | 1321 GFX8 items, 3 fonts |
| | `003.RSC` | 88 GFX8 items, 88 palettes |
| | `004.RSC` | 43 blocks — the speech as WAV, and four songs |

Across Dunkle Schatten 2's containers there are 1678 GFX8 sprites, across
Checker 2000's 1442, and no GFX16 items in either.

## Trailing unreferenced data

`002.RSC` and `003.RSC` carry 782,780 and 1,240,300 bytes respectively
*after* the last indexed item. No table entry reaches this data. It appears
to be leftovers of sprites that were replaced during development and never
compacted away.

## Open questions

- Meaning of the 24 reserved bytes at `0x18` (always zero here).
- The GFX16 item layout — no instance exists in Dunkle Schatten 2.
- Whether the trailing unreferenced data has any recoverable structure.

## See also

- [GFX8 sprites](sprites.md) — the graphics item format
- [Text tables](text-tables.md), [Blocks](blocks.md), [Fonts](fonts.md),
  [Script modules](script-modules.md), [Palettes](palette.md) — the other
  item formats
