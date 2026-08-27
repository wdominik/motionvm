[← Documentation index](../../README.md)

# Blocks

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein and in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, which is an older build of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names the other game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit-ds2).*

A BLK item is untyped binary data: the engine hands it to the script with
`GET` or plays it with `STARTTUNE`, and what it means is decided by who asks
for it. ENVIRO's 130 occupied blocks fall into four families, told apart by
id range and by what the scripts do with them.

## Music — ids 1–5 and 7–11

Ten blocks, 9 017 to 39 387 bytes, each beginning with the ASCII tag
`MTCVTS PSM 2.00\0`. These are **PSM 2** modules — the Parsec sound
system, played through `MUSADL.DRV` — not HMI songs. Id 6 is empty.

`STARTTUNE ( loop n -- )` plays block `n`; its handler (`ENVIRO.EXE` file
`0xbf89`) formats the id through the same `#F0R3i.blk` template the saves
use — `008.blk` for block 8 — loads that item into a fixed buffer and
hands it to the resident driver — `MUSADL.DRV`, loaded whole at start-up
when the sound configuration names an Ad Lib port. The module's sections,
the driver's sequencer and its OPL back-end are read and rebuilt: see
[PSM 2 music](psm-music.md).

## Animation catalogs — ids 116, 125–129, 131–180

Fifty-seven blocks of 110 to 1430 bytes, read and written with `GETANIM` /
`PUTANIM`. Each opens with a `u16` (2, 3 or 4) and then names animation
sequences: at a stride of 14 bytes the records read as `u16, char name[10],
u16` with names `D1_1`, `D1_2`, `D1_3`, `D2_1`, `E0_1`, `E1_1`, `G_MISC`,
`I_MISC`, `G_AUSW`, `G_PRALIN` — Dave and Evi walk cycles and the general and
inventory sets. Block 116 is exactly `2 + 24 × 14` bytes; most others leave
2 to 12 bytes over, so the 14-byte stride is the shape of the *first* records
and not proven for the whole block. **The record layout is open.**

## Per-location tables — ids 200+N, 400+N, 600+N, 800+N

For every location N (1–15 and 17) four blocks exist, all loaded by
`INCLLOC` with `GET ( nbytes dest-addr block-id -- )` into arrays that live
in module 601:

| Block | Destination | Size | Layout |
|---|---|---:|---|
| 200+N | `_LDITEM` — the hotspot/item table | 1120 | `A_LDITEM` = 35 records of `S_LDITEM` = 32 bytes. The script's accessor words name the fields: `X1` +0, `Y1` +2, `X2` +4, `Y2` +6 (a rectangle in world coordinates), `TEXT` +8, `ITEXT` +10, `DR` +12, `DX` +14, `DY` +16, `EXIT` +18, `FITEM` +20, `MX` +22, `MY` +24, `DIR` +26, `ORDER` +28, two bytes spare |
| 400+N | `_ROUTE` — walk routes | 452 | `u16 count` (1–14 across the sixteen locations), then `A_ROUTES` = 25 records of `S_ROUTES` = 18 bytes: a segment `x1 y1 x2 y2` and five `u16` link fields padded with `0xFFFF`. Entries past the count are mostly zero; six blocks keep one to four stale records there |
| 600+N | `_XROUTE` — extended routes | 300 | 25 records of `S_XROUTES` = 12 bytes, six `u16` each; the third and fourth are large (320–1120) and read as scaled values |
| 800+N | `_KLICKAREA` — click areas | 182 | `u16 count` (1–10), then `A_KLICKAREA` = 18 records of `S_KLICKAREA` = 10 bytes: `x1 y1 x2 y2 kind`. Every rectangle of every block is ordered (`x1 ≤ x2`, `y1 ≤ y2`) and lies within 0–627 horizontally and 0–155 vertically |

The record sizes and counts are the game's own: `S_LDITEM`, `A_LDITEM`,
`S_ROUTES`, `A_ROUTES`, `S_XROUTES`, `S_KLICKAREA` and `A_KLICKAREA` are
constants in module 601, and each block is exactly `count × size` (plus the
two-byte count where there is one). The item field names come from module
603's accessors `->LDX1` … `->LDORDER`, which store at `.LDITEM. + offset`.
A second, in-memory table `_FITEM` — `A_FITEM` = 60 records of `S_FITEM` =
10 bytes with fields `NAME` +0, `ORDER` +2, `GFX` +4, `INFO` +6, `DIR` +8 —
is the inventory, not a block.

Coordinates go past 320 horizontally because rooms are wider than the
320-pixel viewport — the screens scroll (see
[Boot and frame loop](../engine/boot-and-loop.md)). The vertical bound of
155 matches the world/inventory split the control handler applies at
y = 165.

What the route links, the extended-route values and the click `kind` mean
is **open**: these tables are read by the game's Forth (`FSCANITEM`,
`XYWALK`, `PSETWALK`, the click handlers), not by the kernel, so their
meaning is in the scripts and not in `ENVIRO.EXE`.

## Other blocks

Block 1 sits in the music range; there is no block 0 and nothing between 12
and 115, 181 and 200, or above 817 beyond the per-location ranges. Every
occupied block belongs to one of the four families above.

## Open questions

- The animation-catalog record layout and what `GETANIM`/`PUTANIM` keep of
  it across a save. Its first field is `0xFFFF` in several of Jeff Jet's
  catalogs 125–182 where Die Enviro-Kids greifen ein's hold small integers.
- The route links, the extended-route fields and the click `kind`; what `DR`, `DX`/`DY`, `EXIT` and `ORDER` of an item record select.

## See also

- [The DATA container](data-container.md) — the BLK segment (ids 0–999)
- [PSM 2 music](psm-music.md) — the music blocks' format and their driver
- [Boot and frame loop](../engine/boot-and-loop.md) — `INCLLOC`, which loads the per-location tables
- [Game structure](../../games/enviro/game-structure.md) — the id recipes per location
- [Blocks (MOTION 32-bit)](../../motion32/formats/block.md) — the 32-bit engine's counterpart, where the records are mapped
