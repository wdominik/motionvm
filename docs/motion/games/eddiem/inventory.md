[← Documentation index](../../README.md)

# Resource Inventory

*Falsches Spiel mit Eddie M. — this page describes the game's own data. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

What the three `DATA.-n-` volumes hold, by the numbers.

## The container as a whole

Three volumes — the most of any game on hand — 2 677 683 bytes between them,
which unpack to 9 352 179 bytes of items: every one of the seven segments is
packed, as in Jeff Jet. Nothing is left over: each volume's items tile it from
its own table's end to its last byte.

| | `DATA.-1-` | `DATA.-2-` | `DATA.-3-` |
|---|---:|---:|---:|
| File | 216 697 | 1 265 145 | 1 195 841 |
| Tables end at | 25 872 | 17 224 | 17 224 |
| First item at | 25 872 | 17 248 | 17 248 |

The header reserves 4305 slots — `2500, 1000, 700, 25, 10, 10, 60` — the later
framing's geometry with the smallest text segment of any game, and declares
three volumes and one spare offset entry. The second and third volumes open on
a `u32` slot table of 4 × 4306 bytes and leave 24 bytes between its end and
their first item, which is why the container page documents that offset as
where items *may* begin and not where they must
([the DATA container](../../motion16/formats/container.md)).

| Segment | Occupied / slots | Bytes unpacked |
|---|---:|---:|
| GFX | 1772 / 2500 | 8 745 880 |
| BLK | 119 / 1000 | 309 864 |
| SCR | 62 / 700 | 160 448 |
| PAL | 25 / 25 | 19 200 |
| FNT | 2 / 10 | 5 158 |
| FRT | 1 / 10 | 516 |
| TXT | 51 / 60 | 111 113 |

**The occupancy word is a bitmask, and this is the game that shows it.** Each
slot's word names the volume that holds it by bit — 1, 2 or 4 — where a game on
two volumes reads the same with a count. Volume 1 holds everything the game
runs and says — all 62 modules and all 51 text tables — with the per-location
tables, the animation catalogs, the jingle and three sprites; volume 2 holds
995 sprites, every palette, both fonts, the font reference table, the thirteen
samples and the two songs; volume 3 holds 774 sprites, ids 401 to 1716, and
nothing else. Every occupancy word agrees with the offsets.

## Sprites (GFX)

1772 sprites, ids sparse across the 2500 slots, all packed and all unpacking to
`[u16 w][u16 h][u16 0]` followed by `w × h` bytes. 443 distinct sizes; the
widest is 320, a backdrop strip, and the tallest 155, the height of the world
viewport less the strip's overlap. Full-screen artwork is a block, not a sprite.

## Palettes

25 palettes, ids 0–24 — every slot the header reserves — each exactly 768 bytes
of six-bit DAC values. `RUN` installs 0 and then 1; the intro installs 17, 21
and 23; the rooms install their own.

## Fonts

Two, ids 0 and 2 with slot 1 the one gap, 103 glyphs each: heights 12 and 14.
`RUN` installs 2 as the shadow font.

The **font reference table** is 516 bytes — `u16 256, u16 120, u16[256]` —
with the same `'A'→0` mapping as every sibling and 154 codes mapped to
nothing. It was written for a font of 120 glyphs over fonts of 103: four
entries map to glyph 100 or above, and two of them — `0x8C` at 104 and `0xA0`
at 103, the same two bytes as Jeff Jet's and Hilfe für Amajambere's — point
past both fonts. Neither occurs in any of the 2443 shipped strings.

## Text tables

51 tables, ids 1 to 59, 2443 strings, CP437 German. Table 11 is the hotspot
labels — *Parkbank*, *Kiosk*, *Zur STERN-Dok*, *Gruner + Jahr*,
*Hitler-Tagebücher* — table 21 the default responses, and the rest the
conversations, one table per dialogue.

## Blocks

119 blocks in six families:

| Ids | Count | What |
|---|---:|---|
| 24, 25 | 2 | The PSM 2 modules — the intro's and the ending's song, and the map's — see [PSM 2 music](../../motion16/formats/psm-music.md) |
| 19 | 1 | A bare `PLX` section of 366 bytes: the jingle `SET_POINTS` plays |
| 1, 9, 10, 12–15, 17, 18, 20–23 | 13 | `SM8` samples on their own, 2305 to 31 960 bytes — the sound effects |
| 101–108, 125–159 | 43 | Animation catalogs, 16 to 3236 bytes |
| 200+N | 15 | Per-location item tables, 1120 bytes = `A_LDITEM × S_LDITEM` (35 × 32) |
| 400+N, 600+N, 800+N | 45 | Walk routes (452 bytes), extended routes (300), click areas (182) |

The samples' header is `SM8\0`, a version word `0x0100`, a length word — the
block's length less ten in every one — and a period in PIT cycles from 56 to
179: 21.3 kHz down to 6.7 kHz as the header names them, unsigned 8-bit, played
once through the digital driver's direct path on the DSP's clock — the period
in whole microseconds —
([PSM 2 music](../../motion16/formats/psm-music.md#the-sample--an-sm8-block)).

## Script modules

62 modules, 1543 words — 493 colon definitions, 784 variables, 266 constants.
Every module's declared number equals its slot, and every colon body
disassembles under the game's own kernel table with no unknown ordinal and ends
on `##`. The numbering is in the [module map](module-map.md).

## See also

- [Module map](module-map.md) — what each module does
- [Game structure](game-structure.md) — what they add up to
- [The DATA container](../../motion16/formats/container.md) — the format, across the six 16-bit games
- [Resource inventory (Jeff Jet - Abenteuer InfoHighway)](../jeffjet/inventory.md) — two volumes, packed
