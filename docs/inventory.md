[← Documentation index](README.md)

# Resource Inventory

A survey of everything the three containers hold. Counts and ids are
measured from the shipped files.

## Sprites (GFX8)

1678 sprites in total (2 in `001.RSC`, 1619 in `002.RSC`, 57 in
`003.RSC`). Dimensions range from 8×1 up to 640×480.

| Group | Ids / count |
|---|---|
| Full-screen backgrounds, 640×400 | 63, 65, 66, 1010, 2220 |
| Full 640×480 images | 40 sprites |
| Title logo, 320×200 (drawn at 200 %) | 86 |
| Most common size | 64×48 — 150 sprites (inventory icons and similar UI tiles) |

Character-animation frames dominate the count: long runs of ~100×220
frames (walk cycles in several directions per figure).

## Palettes

60 palettes. Ids 0–36 (with gaps) are the working set; the higher ids
group by scene: 40, 70, 80, 90–96 (96 = title logo, 91 = title artwork),
100, 110, 130, 140, 150, 180, 190–197.

## Fonts

Nine fonts, 968 glyphs in total:

| Id | Height | Glyphs |
|---|---|---|
| 0 | 12 | 116 |
| 2 | 14 | 116 |
| 3 | 18 | 90 |
| 4 | 20 | 90 |
| 5 | 18 | 120 |
| 6 | 20 | 120 |
| 7 | 11 | 116 |
| 8 | 40 | 84 |
| 10 | 11 | 116 |

Font 0 is also shipped as the standalone system font `000.FNT`. The game
registers fonts 5, 6, and 8 at startup; the nine text templates all use
font 6. Font 8 (40 px) is the large headline font.

## Text tables

133 tables, 6785 strings. Known roles:

| Table | Role |
|---|---|
| 2 | UI messages (entry 0 empty — the "no text" placeholder) |
| 4 | Info texts |
| 6 | Intro and title texts (entry 108 production credit, 109 backstory) |
| 8 | Diagnostic/info descriptor texts |

## Blocks

250 blocks, ids 0–570. The id ranges mirror the module numbering (see
[Module map](engine/module-map.md)):

| Ids | Count | Size | Contents |
|---|---|---|---|
| 0–25 | 26 | 4–100 KB | [HMI songs](formats/hmi.md) |
| 50 | 1 | 512 KB | Translucency blend tables (8 × 64 KB) |
| 51–54 | 4 | 8–64 KB | Palette shade tables |
| 99 | 1 | 200 B | The location table |
| 101–123, 130 | 24 | 904 B each | Per-location walking routes |
| 199 | 1 | 516 B | Copy of `000.FRT` |
| 201–223, 230 | 24 | 600 B each | Per-location extended routes |
| 301–323, 330 | 24 | 200 B each | Per-location click areas |
| 401–423, 430 | 24 | 2240 B each | Per-location item records |
| 450–570 | 121 | 64 B–2.8 KB | Dialogue definitions |

See [Blocks](formats/block.md) for the record layouts.

## Script modules

86 modules. See the [module map](engine/module-map.md) for what each one
does. Three define no words (10 carries only data; 123 and 130 are
entirely empty).

## See also

- [RSC containers](formats/rsc-container.md)
- [Module map](engine/module-map.md)
- [Blocks](formats/block.md)
