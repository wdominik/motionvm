[← Documentation index](../../README.md)

# Resource Inventory

*Jeff Jet - Abenteuer InfoHighway — this page describes the game's own data. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

A survey of everything the two `DATA.-n-` volumes hold. Counts, ids and sizes
are measured from the shipped files, after unpacking.

## The container as a whole

Two volumes, 2 503 434 bytes between them, of which 43 544 are the header and
the two offset tables and 2 459 890 are items. Unpacked those items are
8 412 811 bytes — 3.4 times what they take on the disc, and half again the
whole of Die Enviro-Kids greifen ein's uncompressed container. Nothing is left
over: each volume's items tile it exactly from its own table's end to its last
byte.

| | `DATA.-1-` | `DATA.-2-` |
|---|---:|---:|
| File | 1 404 960 | 1 098 474 |
| Tables | 26 136 | 17 408 |
| Items, as stored | 1 378 824 | 1 081 066 |
| Slots | 1187 | 543 |

Every segment is flagged packed, so every item is an eight-byte GFXCRUNCH
header and a stream — see [the DATA container](../../motion16/formats/data-container.md).

| Segment | Occupied / slots | Bytes, unpacked |
|---|---:|---:|
| GFX | 1470 / 2500 | 7 555 308 |
| BLK | 119 / 1000 | 410 190 |
| SCR | 55 / 700 | 146 004 |
| PAL | 16 / 25 | 12 288 |
| FNT | 2 / 10 | 5 108 |
| FRT | 1 / 10 | 516 |
| TXT | 65 / 100 | 283 397 |

Two further GFX slots, 1319 and 1848, are flagged for a volume — 1319 for
`DATA.-1-`, 1848 for `DATA.-2-` — and have no bytes in it; both sit in the
middle of a run of occupied slots. What that means is
[an open question](../../open-questions.md#motion-16-bit).

**What is on which volume matters.** Volume 2 holds 523 of the sprites and
**all sixteen palettes, both fonts and the font reference table**; volume 1
holds the scripts, the texts, the music and the other 947 sprites. A reader
that opened only the first volume would find every script and no colour.

## Sprites (GFX)

1470 sprites in 433 distinct sizes, ids 0 to 2417, from 8×1 up to 288 wide and
156 high. Nothing is a whole 320×200 screen: the room backdrops are blocks, and
155 or 156 rows is the world viewport this engine draws through.

| Group | Size / count |
|---|---|
| Most common size | 32×20 — 89 sprites |
| Walk cycles | 80×155 (77), 160×155 (42), 40×117 (25), 48×117 (20) |
| Strips and tiles | 160×35 (23), 40×31 (23), 8×1 (20) |

## Palettes

Sixteen palettes, ids 0–15, all on volume 2; 16 to 24 are empty. `RUN` installs
0 and then 1, and the location macros install their own.

## Fonts

Two fonts, ids 0 and 2, 102 glyphs each, 12 and 14 rows high, both on volume 2;
one font reference table in FRT slot 0 serves both. Font 0 is the text face and
font 2 the solid shadow behind it. There is no third, outlined face here — the
other 16-bit game's font 7 — and `-FONT` is never called.

The reference table was written for a font of 120 glyphs. Two of its entries
point past 102: CP437 `0x8C` to glyph 104 and `0xA0` to glyph 103. Neither byte
occurs in any of the game's 5709 strings.

## Text tables

65 tables, ids 9 to 82 with gaps, holding 5709 strings, CP437 German with
`0x0A` line breaks. Table 11 is the noun list the hover caption reads from
(132 entries, *Fotoapparat*, *Tür*, …), 12 and 13 the intro dialogue, 14 the
credits, 20 and 21 the comments Jeff makes about what he looks at.

## Blocks

119 blocks in three families (see [Blocks](../../motion16/formats/blocks.md)):

| Ids | Count | What |
|---|---:|---|
| 1–9 | 9 | PSM 2 music, 22 975 to 39 130 bytes |
| 125–182 | 58 | Animation catalogs, 162 to 3140 bytes |
| 201–213 | 13 | Item tables, 1120 bytes each |
| 401–413 | 13 | Walk routes, 452 bytes each |
| 601–613 | 13 | Extended routes, 300 bytes each |
| 801–813 | 13 | Click areas, 182 bytes each |

The four per-location ids are 200, 400, 600 and 800 plus the location number,
for locations 1–13 with no gap.

## Script modules

55 modules, 146 004 bytes, defining 1398 words: 481 colon definitions, 650
variables and 267 constants. The biggest are 607 (17 096 bytes, 195 words —
the item constants and the documents), 606 (10 294 — speech), 109 (9816) and
110 (9748, 100 words). The thirteen location macros are one word each, all
named `ZEIT` and all under id 549. What each module holds is in the
[module map](module-map.md).

## See also

- [The DATA container](../../motion16/formats/data-container.md) — how the segments and volumes are laid out
- [Module map](module-map.md), [Game structure](game-structure.md)
- [Resource inventory (Die Enviro-Kids greifen ein)](../enviro/inventory.md) — the other 16-bit game's, for comparison
