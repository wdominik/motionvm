[← Documentation index](../../README.md)

# Resource Inventory

*Hilfe für Amajambere — this page describes the game's own data. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

What the two `DATA.-n-` volumes hold, by the numbers.

## The container as a whole

Two volumes, 5 333 609 bytes between them, of which 44 044 are the header and
the two offset tables and 5 289 565 are items. The items are stored **plainly**
— this is the game that shows two volumes and packed items to be independent
choices, where Jeff Jet has both and Die Enviro-Kids greifen ein neither.
Nothing is left over: each volume's items tile it exactly from its own table's
end to its last byte.

| | `DATA.-1-` | `DATA.-2-` |
|---|---:|---:|
| File | 506 066 | 4 827 543 |
| Tables | 26 436 | 17 608 |
| Items | 479 630 | 4 809 935 |
| Slots filled | 324 | 1077 |

The header reserves 4395 slots — `2500, 1000, 700, 25, 10, 10, 150` — which is
the sibling games' geometry except for the text segment, where they reserve 100
and this one 150. Seven spare offset entries follow the table, as in Jeff Jet.

| Segment | Occupied / slots | Bytes |
|---|---:|---:|
| GFX | 1045 / 2500 | 4 765 086 |
| BLK | 153 / 1000 | 170 322 |
| SCR | 76 / 700 | 127 788 |
| PAL | 24 / 25 | 18 432 |
| FNT | 7 / 10 | 25 901 |
| FRT | 1 / 10 | 516 |
| TXT | 95 / 150 | 181 520 |

**What is on which volume matters, and here it is split by kind rather than by
half.** Volume 2 holds everything the game draws through — every one of the
1045 sprites, all 24 palettes, all seven fonts and the font reference table —
and volume 1 holds everything it runs and says: the scripts, the texts and the
music. A reader that opened only the first volume would find every script and
nothing at all to show.

**The occupancy words over-claim.** This game flags whole segment ranges rather
than single slots: all 2500 GFX slots and all ten FNT slots carry a volume
number while 1045 and 7 of them carry bytes. `motionvm-tools info` reports
**1534** slots whose occupancy word and offsets disagree, against Jeff Jet's two
and none in Die Enviro-Kids greifen ein. Nothing downstream is affected — what has
bytes is what is
there — but it is why presence is read from the offsets and the flag word is
kept as a diagnostic. See [the DATA container](../../motion16/formats/container.md).

## Sprites (GFX)

1045 sprites, ids sparse across the 2500 slots, all on volume 2 and all stored
as `[u16 w][u16 h][u16 0]` followed by `w × h` bytes — `w·h + 6 == itemLen` in
every one. 276 distinct sizes; the largest is 304×162, which fits inside the
320×165 world viewport. Full-screen artwork is a block, not a sprite.

## Palettes

24 palettes, ids 0–23, each exactly 768 bytes of six-bit DAC values (every byte
≤ 63). `RUN` installs 0 and then 1; the rooms install their own.

## Fonts

Seven, more than either sibling ships — ids 0 and 2–7, with slot 1 the one gap,
691 glyphs between them:

| Id | Glyphs | Height |
|---:|---:|---:|
| 0 | 102 | 12 |
| 2 | 102 | 14 |
| 3 | 90 | 18 |
| 4 | 90 | 20 |
| 5 | 102 | 18 |
| 6 | 102 | 20 |
| 7 | 103 | 11 |

`RUN` installs 2 and then 7. The two 90-glyph faces are the headline sizes.

The **font reference table** is 516 bytes — `u16 256, u16 120, u16[256]` — with
the same `'A'→0` mapping as both sibling games and 154 codes mapped to nothing.
It was written for a font of 120 glyphs and the largest here holds 103, so two
entries point past every font: `0x8C` at 104 and `0xA0` at 103 — the same two
bytes as Jeff Jet's. Neither occurs in any of the 2709 shipped strings.

## Text tables

95 tables, ids 9 to 113, 2709 strings, CP437 German. The header reserves 150
slots for them, half again what the sibling games reserve.

## Blocks

153 blocks in four families:

| Ids | Count | What |
|---|---:|---|
| 1–4 | 4 | The PSM 2 songs — see [PSM 2 music](../../motion16/formats/psm-music.md) |
| 125–212 | 70 | Animation catalogs |
| 300+N | 19 | Per-location item tables, 1120 bytes = `A_LDITEM × S_LDITEM` (35 × 32) |
| 400+N | 20 | Walk routes, 452 bytes |
| 600+N | 20 | Extended routes, 300 bytes |
| 800+N | 20 | Click areas, 182 bytes |

The item table is at 300+N here where both sibling games put it at 200+N;
`INCLLOC` computes the number as `ACTLOC @ 300 + `. **Nineteen of twenty**:
block 307 was never written, so location 7 has no item table. See
[game structure](game-structure.md#open-questions).

## Script modules

76 modules, 1195 words — 469 colon definitions, 533 variables, 193 constants.
Every module's declared number equals its slot, and every colon body
disassembles under the game's own kernel table with no unknown ordinal and ends
on `##`. The numbering is in the [module map](module-map.md).

## See also

- [Module map](module-map.md) — what each module does
- [Game structure](game-structure.md) — what they add up to
- [The DATA container](../../motion16/formats/container.md) — the format, across the four games
- [Resource inventory (Jeff Jet - Abenteuer InfoHighway)](../jeffjet/inventory.md) — two volumes, packed
- [Resource inventory (Die Enviro-Kids greifen ein)](../enviro/inventory.md) — one volume, plain
