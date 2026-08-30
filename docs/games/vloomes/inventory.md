[← Documentation index](../../README.md)

# Resource Inventory

*Victor Loomes – Das Spiel — this page describes the game's own data. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

What the one `DATA.-1-` holds, by the numbers. Counts, ids and sizes are
measured from the shipped file, after unpacking.

## The container as a whole

One volume, 1 009 597 bytes, of which 14 848 are the header and the two tables
and 994 749 are items. Unpacked those items are 2 580 566 bytes — 2.6 times
what they take on the disc, all of it in the three packed segments. Nothing is
left over: the items tile the file exactly from the tables' end to its last
byte.

This is the **earlier framing** of the container, which two games use and which
nothing in the file announces: no per-segment packing field, so the occupancy
table starts at 22 rather than 38 and everything after it moves sixteen bytes
with it. The reader works the framing out by reading the file as the earlier
one and asking whether it adds up — the offset table's first entry has to be
where the tables end (14 848 here) and its last has to be the file's length
(1 009 597). Both hold. See
[the DATA container](../../motion16/formats/container.md#the-earlier-framing).

The header reserves 2471 slots — `1200, 500, 700, 21, 10, 10, 30` — where the
three later games reserve 2500 sprites and 1000 blocks alone. There are
no spare offset entries, and the word at `0x12` that the later framing reads as
a volume count says **two** while the game ships one file. It cannot mean what
it means later: `LL.EXE` holds the literal `data.-1-` where the later builds
hold a `DATA.-#i-` name former, so this player could not open a second volume
if one existed.

| Segment | Slots | Occupied | As stored | Unpacked |
|---|---|---:|---:|---:|
| GFX | 0–1199 | 721 | 558 295 | 2 141 374 |
| BLK | 1200–1699 | 226 | 241 541 | — |
| SCR | 1700–2399 | 36 | 125 120 | — |
| PAL | 2400–2420 | 21 | 16 128 | — |
| FNT | 2421–2430 | 2 | 2 042 | 4 508 |
| FRT | 2431–2440 | 1 | 244 | 516 |
| TXT | 2441–2470 | 25 | 51 379 | — |

**Which segments are packed is the generation's, not the header's**, and it is
the same in both games that use this framing: sprites, fonts and the font
reference table are packed, and blocks, modules, palettes and text tables are
stored plainly. A packed sprite or font carries a **ten-byte** header — the
unpacked length written twice, then the packed length and the GFXCRUNCH
parameters — and the two copies agree in all 723 of them. The font reference
table is the exception and carries the later eight-byte header.

**The occupancy word is a plain flag here**, 0 or 1 rather than a volume
bitmask, and it agrees with the offsets in every one of the 2471 slots.
`motionvm-tools info` reports no mismatch at all, against Jeff Jet's two and
Hilfe für Amajambere's 1534. The 1439 empty slots repeat their successor's
offset instead of holding zero, which is what makes the offset table monotone
but not strictly so: 1438 equal-neighbor pairs.

## Sprites (GFX)

721 sprites, ids 0 to 1066 with gaps, all stored packed and all decoding to
`[u16 w][u16 h][u16 0]` followed by `w × h` bytes — `w·h + 6 == unpackedLen`
in every one. 213 distinct sizes; the most common is 32×20, with 71 of them,
and 296 of the 721 are 32 wide.

The largest is 216 wide and 195 tall, and **only two sprites are taller than
the room's 140-row window** — both are the intro's, which draws through a
window of the full 320×200. Full-screen artwork is a block, not a sprite.

The GFX space also holds **mirror aliases**, ids the container has no bytes
for: `699 700 GFXVFLIP` in the location-13 macro makes sprite 700 the mirror
of 699, and slot 700 is empty in the container and marked absent in `GFX.INF`.

`GFX.INF` ships beside the container and says every sprite's width and height
without unpacking it. Its 721 present entries and the container's 721 occupied
slots agree in both directions with no exception, and every entry matches the
decoded sprite's header. motionvm validates the file rather than reading it
([other files](other-files.md#the-sprite-dimension-table), [departures](../../departures.md#the-16-bit-machine)).

## Palettes

21 palettes, ids 0–20, each exactly 768 bytes of six-bit DAC values (every byte
≤ 63). This is the only segment with no free slot in it: 21 reserved, 21 used.
`RUN` installs 0, the intro installs 16, and each location macro installs its
own with `XSETPAL`.

## Fonts

Two, ids 0 and 2, **90 glyphs each** where the later games' two hold 102:

| Id | Glyphs | Height | Unpacked |
|---:|---:|---:|---:|
| 0 | 90 | 12 | 1984 |
| 2 | 90 | 14 | 2524 |

`RUN` installs 0 with `0 SFT` as the text face and 2 with `2 +FONT _SHFONT !`
as the shadow behind it — the same pairing Jeff Jet has, one face short of Die
Enviro-Kids greifen ein and five short of Hilfe für Amajambere.

The **font reference table** is 516 bytes — `u16 256, u16 120, u16[256]` —
with the same `'A' → 0` mapping as all three sibling games. 88 CP437 codes are
mapped and 168 are not, and the highest index it names is 87, so **nothing
points past the 90 glyphs the fonts hold**. Jeff Jet's table and Hilfe für
Amajambere's both carry two entries that do; this one, written for the same
120-glyph face, does not reach that far.

## Text tables

25 tables, ids 0–2, 4 and 6–26, holding 1336 strings, CP437 German. Table 1 is
the noun list the hover caption reads from — 120 entries, *Telefon*,
*Aschenbecher*, *Tür* — table 6 the system texts the message box draws
(*Spielstand sichern*, the five slot labels, the quit prompt, the competition
slide), table 7 the in-game help, and table 8 the 25-entry credits.

## Blocks

226 blocks in five families (see [Blocks](../../motion16/formats/blocks.md)):

| Ids | Count | Bytes each | What |
|---|---:|---:|---|
| 0–13 | 14 | 552 – 13 698 | The PSM 2 songs, stored as bare `PLX` sections — see [PSM 2 music](../../motion16/formats/psm-music.md) |
| 21–33, 41–80 | 53 | 202 | Click areas → `_KLICKAREA`, `A_KLICKAREA × S_KLICKAREA` (20 × 10) and two |
| 201–213, 221–260 | 53 | 1120 | Item tables → `_PKITEM`, `A_PITEM × SIZE_PKITEM` (40 × 28) |
| 301–313, 321–360 | 53 | 542 | Walk routes → `_ROUTE`, `A_ROUTES` (30) × 18 and two |
| 401–413, 421–460 | 53 | 360 | Extended routes → `_XROUTE`, `A_ROUTES × S_XROUTES` (30 × 12) |

**Fifty-three of each, not thirteen.** Thirteen belong to the locations —
`?AO` plus 200, 300, 400 and 20 — and forty to the two transit lines, twenty
stops each, at `AKTBAHN` plus 220, 320, 420 and 40 and `AKTFBAHN` plus 240,
340, 440 and 60. `INCLORT` chooses between the three arithmetics on the room
number; see [game structure](game-structure.md#the-two-transit-lines).

The four sizes are the four constants module 600 declares, so the block
families and the arrays they load into check against each other exactly.

## Script modules

36 modules, 125 120 bytes, defining 1299 words: 695 colon definitions, 472
variables and 132 constants. The biggest are 102 (15 510 bytes, 88 words), 104
(8930), 100 (8692 — the boot module, the frame handler and the location
switch) and 609 (8684 — the walker). Every module's declared number equals its slot, and every
colon body disassembles under the game's own kernel table with no unknown
ordinal and ends on `##`.

The bytecode calls **159 of the 204 kernel words** — 100 of the 124 domain
words and 59 of the 80 core ones. That count only comes out right at this
build's ordinal base of 102; read at the later builds' 105 the same cells name
words three places along ([`LL.EXE`](../../motion16/engine/ll-exe.md)). The
numbering is in the [module map](module-map.md).

## See also

- [Module map](module-map.md) — what each module does
- [Game structure](game-structure.md) — what they add up to
- [The DATA container](../../motion16/formats/container.md) — the format, and the earlier framing this game uses
- [Other shipped files](other-files.md) — `GFX.INF` and the rest of the installation
- [Resource inventory (Hilfe für Amajambere)](../hfa/inventory.md) — two volumes, plain, and the later framing
