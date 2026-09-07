[← Documentation index](../../README.md)

# Resource Inventory

*Checker 2000 — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

A survey of everything the five containers hold. Counts and ids are
measured from the shipped files.

## The container as a whole

32 383 276 bytes over five files, indexed by type and id rather than by one
slot space — the 32-bit generation merges the banks it finds, so which file a
resource lives in is not something a script can see
([resource containers](../../motion32/formats/container.md)). `ENGINE.RSC`
is the fifth, a container of the engine's own that no other game ships
([other shipped files](other-files.md#enginersc)).

| Type | Count | ENGINE.RSC | 001.RSC | 002.RSC | 003.RSC | 004.RSC |
|---|---:|---:|---:|---:|---:|---:|
| GFX8 | 1442 | — | 33 | 1321 | 88 | — |
| TEXT | 264 | — | 264 | — | — | — |
| BLOCK | 123 | — | 80 | — | — | 43 |
| FONT | 4 | 1 | — | 3 | — | — |
| SCRIPT | 119 | — | 119 | — | — | — |
| PALETTE | 154 | 1 | 65 | — | 88 | — |

Every one of the five carries data no index entry reaches: 2 020 bytes at
the end of `ENGINE.RSC`, `003.RSC` and `004.RSC`, 106 216 in `002.RSC` and
461 392 in `001.RSC`.

## Sprites (GFX8)

1442 sprites, from 8×7 up to 1240×480, every one with its own palette:

| Where | Ids | What |
|---|---|---|
| `001.RSC` | 604, 607, 647, 684, 700–708, 710–711, 716–719, 1111–1119, 1518–1520, 4138, 4800 | Thirty-three sprites, nineteen of them ids that `002.RSC` or `003.RSC` fills as well and that resolve here, the lowest slot ([containers](../../motion32/formats/container.md#multi-file-overlay)); the fourteen of its own include the pointer sprites 716–719 — `XATMOUSE` puts up 716, 718 and 719 in ten modules |
| `002.RSC` | 101–4745, in runs | The artwork: the scenes' backgrounds, the figures' frames, the games' pieces and their pre-rendered lines of text |
| `003.RSC` | 601–688 | The photo story: 88 full 640×480 photographs, one palette each (palettes 101–188) |

The sizes group by use:

| Size | Count | What |
|---|---:|---|
| 640×480 | 165 | Backgrounds and full-screen pictures — `101–106`, `158–167`, `462–470`, `700–715`, the location starts at `2000`, `2050`, `2100` … `3100`, and the 88 photographs |
| 776×480 and 1240×480 | 2 (104, 164) | The AOK office's and the beach's backgrounds, wider than the screen: what the scroll slide of `->SCRX` moves over ([module map](module-map.md)) |
| 640×380, 616×428, 640×180, 640×450 | 13 (471–482, 712) | Partial-screen panels |
| 176×410, 280×407, 216×424, 192×299, 168×239, 136×218 | runs of 18–63 | The figures' frames, one run a pose |
| 32×57 | 128 (4600–4745) | A small figure's frames, the games of locations 29 and 31 |
| 208×45 and 264×34 | 86 and 44 | Lines of text drawn as pictures, in a handwriting face: the questions and answers of the games in locations 27 and 31 |

## Palettes

154 palettes in three containers. `ENGINE.RSC` holds palette 0, the system
palette `TOGFX` installs ([screens](../../motion32/engine/screens.md)).
`001.RSC` holds 1–48, 50–65 and 147, the working set the boards and scenes
switch between with `SETPAL`. `003.RSC` holds 101–188, one for each of its
88 photographs, id for id.

## Fonts

Four fonts, 410 glyphs in total:

| Id | Where | Height | Glyphs |
|---|---|---|---|
| 0 | `ENGINE.RSC` | 12 | 116 |
| 5 | `002.RSC` | 18 | 105 |
| 6 | `002.RSC` | 20 | 105 |
| 8 | `002.RSC` | 40 | 84 |

Font 0 is the system font the engine's own messages use — the error boxes
`GET` puts up on a fresh copy ([engine R78](../../motion32/engine/engine-r78.md))
— and is the only one in `ENGINE.RSC` beside its palette. The three in
`002.RSC` are the game's: the same ids, heights and glyph counts as Dunkle
Schatten 2's fonts 5, 6 and 8 for the headline font, and fifteen glyphs
fewer in the two text fonts ([Dunkle Schatten 2's inventory](../ds2/inventory.md#fonts)).

## Text tables

264 tables, 1073 strings, ids 2–4 and 11–271:

| Tables | Entries | What |
|---|---:|---|
| 2 | 456 | The story's lines — what the scenes speak, and caption where no sound card answers ([speech](game-structure.md#speech)) |
| 3 | 8 | The games' instructions |
| 4 | 30 | The shell's templates — `#s` and `#i` formats for the registration and the highscore ([text tables](../../motion32/formats/text-tables.md)) |
| 11–256 | 1 each, 254–256 two | The information book: one table a page, in the six *Checks* the book is divided into — *Schul-*, *Start-*, *Azubi-*, *Body-*, *Future-* and *AOK-Check* |
| 257–264 | 25–61 | The eight regional pages, one insurer each — Berlin, Brandenburg, Chemnitz, Dresden, Halle, Leipzig, Magdeburg, Mecklenburg-Vorpommern — which `_REGIONAL` picks by the postcode |
| 265 | 1 | No regional page for the postcode entered |
| 266–271 | 1–11 | The program's help, the CD's other titles, and three more pages of the book |

## Blocks

123 blocks, ids 1–239, in `001.RSC` and `004.RSC`:

| Ids | Count | Where | Size | Contents |
|---|---:|---|---|---|
| 1 | 1 | `001.RSC` | 13 109 B | An [HMI song](../../motion32/formats/hmi.md) — the same bytes as `TEST.HMI`, the sound setup's test song; no script starts it |
| 2 | 1 | `001.RSC` | 40 320 B | Headerless 8-bit audio at the unsigned centre — the sound setup's raw test sample |
| 3 | 1 | `001.RSC` | 80 684 B | A WAV, 11 025 Hz 8-bit stereo, 3.66 s — the same bytes as `TEST.WAV`; no script starts it |
| 4 | 1 | `001.RSC` | 15 920 B | A WAV, 22 050 Hz 8-bit mono, 0.72 s |
| 11–49 | 39 | `004.RSC` | 1 572 B – 228 566 B | The sound effects: WAV, 22 050 Hz 16-bit mono, 0.03 s to 5.18 s, which `STARTSAMPLE` plays at a quarter of the layer's volume ([speech](game-structure.md#speech)) |
| 60–63 | 4 | `004.RSC` | 17 626 B – 26 838 B | HMI songs: 60 for the shell's board, 62 under location 3, and the schoolyard picks one of 60–63 at random |
| 64 | 1 | `001.RSC` | 14 726 B | An HMI song, the registration's and the highscore's |
| 99 | 1 | `001.RSC` | 200 B | The location table: fifty packed addresses `(module << 16) \| 0x30`, location *N*'s pointing at module `300 + N`'s scene macro — the same table as [Dunkle Schatten 2's](../../motion32/formats/blocks.md#block-99--the-location-table), which `_LOCTABLE` reads |
| 101–139 | 37 | `001.RSC` | 904 B – 933 B | The walking routes, one block a location, which `INCLLOC` (module 5) copies into `_ROUTE` as it enters one — 904 bytes where a location has routes, and blank blocks of 900 + id bytes where it has none ([blocks](../../motion32/formats/blocks.md#the-per-location-data-blocks)) |
| 201–239 | 37 | `001.RSC` | 600 B | The extended routes, copied into `_XROUTE` the same way, the same ids filled and blank |

Ids 116, 119, 216 and 219 are the gaps in both runs, as 116 and 119 are in
the script ids. Nothing here is a click-area, item or dialogue record:
`INCLLOC` loads two blocks a location where Dunkle Schatten 2's loads four,
and the scenes' clickable areas are literal ranges in their task managers
([game structure](game-structure.md#the-boards)).

## Script modules

119 modules: 1–7 and 99; 101–115, 117–118 and 120–139; 201–215, 217–218
and 220–239; 301–315, 317–318 and 320–339 — a location `n` has a macro at
`n`, a task manager at `200 + n` and a scene at `300 + n`, with 100 + `n`
beside them. See the [module map](module-map.md) for what each does.

## See also

- [RSC containers](../../motion32/formats/container.md)
- [Module map](module-map.md)
- [Other shipped files](other-files.md)
- [Blocks](../../motion32/formats/blocks.md)
