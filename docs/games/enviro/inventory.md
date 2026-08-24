[← Documentation index](../../README.md)

# Resource Inventory

*Die Enviro-Kids greifen ein — this page describes the game's own data. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit-enviro).*

A survey of everything `DATA.-1-` holds. Counts, ids and sizes are measured
from the shipped file.

## The container as a whole

7 609 296 bytes: 26 120 bytes of header, occupancy table and offset table,
then 7 583 176 bytes of items with nothing left over — every byte after the
tables belongs to an occupied slot.

| Segment | Occupied / slots | Bytes |
|---|---:|---:|
| GFX | 1586 / 2500 | 6 888 132 |
| BLK | 130 / 1000 | 312 732 |
| SCR | 65 / 700 | 167 302 |
| PAL | 23 / 25 | 17 664 |
| FNT | 3 / 10 | 8 016 |
| FRT | 1 / 10 | 516 |
| TXT | 96 / 100 | 188 814 |

## Sprites (GFX)

1586 sprites in 262 distinct sizes, from 8 pixels wide and 1 high up to
320×155 and 320×177. Nothing is wider than the display; no single sprite is
a whole 320×200 screen.

| Group | Ids / count |
|---|---|
| Placeholders, 64×115 | ids 0–11 |
| Most common size | 32×20 — 160 sprites |
| Person frames | 120×109 (92), 64×54 (91), 80×155 (75), 96×112 (33), 32×56 (41), 32×58 (32) |
| The pointer | id 399, 16×15 |
| The intro's motifs | ids 2482–2486, 160×77 to 168×83; 2493 is 304×117 |
| Runtime scratch | the high range up to 2499 is what the scripts flip into and free (`XGFXVFLIP`, `420 2499 -1 XGFXSTAT`) |

## Palettes

23 palettes, ids 0–22; 23 and 24 are empty. `RUN` installs 0 and then 1,
the intro 22, the location macros their own.

## Fonts

Three fonts, ids 0, 2 and 7, 116 glyphs each, 12, 14 and 11 rows high; one
font reference table in FRT slot 0 that serves all three. Font 0 is the
text face, font 2 the solid shadow behind it (`_SHFONT`), font 7 the
outlined memo face (`_MEMO`).

## Text tables

96 tables, ids 1–99 except 0, 19, 83 and 84, holding 3508 strings. The
large ones are the dialogue tables: 20 (257 strings), 21 (243), 11 (235),
22 (160). Table 14 is the town briefing read in the intro; table 20 carries
the character switch lines *"Jetzt bin ich Eva."* / *"Jetzt bin ich
Maik."*

## Blocks

130 blocks in four families (see [Blocks](../../motion16/formats/blocks.md)):

| Ids | Count | What |
|---|---:|---|
| 1–5, 7–11 | 10 | PSM 2 music, 9 017 to 39 387 bytes |
| 116, 125–129, 131–180 | 57 | Animation catalogs, 110 to 1430 bytes |
| 201–215, 217 | 16 | Item tables, 1120 bytes each |
| 401–415, 417 | 16 | Walk routes, 452 bytes each |
| 601–615, 617 | 16 | Extended routes, 300 bytes each |
| 801–815, 817 | 16 | Click areas, 182 bytes each |

The four per-location ids are 200, 400, 600 and 800 plus the location
number, for locations 1–15 and 17.

## Script modules

65 modules, 167 302 bytes, defining 1774 words: 672 colon definitions, 792
variables and 310 constants. The biggest are 607 (13 688 bytes, 259 words —
the item constants and documents), 606 (11 256 bytes — speech), 107
(10 786 bytes, 125 words — the shopping centre), 111 (10 046) and 113
(9 956). Sixteen of the modules are one word each: the location macros
301–315 and 317. What each module holds is in the
[module map](module-map.md).

## See also

- [The DATA container](../../motion16/formats/data-container.md) — how the segments are laid out
- [Module map](module-map.md), [Game structure](game-structure.md)
