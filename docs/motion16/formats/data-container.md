[← Documentation index](../../README.md)

# The DATA Container

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein; what is measured here is measured on that game's files. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit-ds2).*

The 16-bit engine keeps a whole game in one file. ENVIRO ships `DATA.-1-`,
7 609 296 bytes: a header that also names the boot word, an occupancy table,
one offset table, and then every resource back to back. Seven resource kinds
share a single slot space, segment after segment; the id a script uses is the
slot number minus the segment's base.

All multi-byte values are little-endian.

## Header (offset 0)

Eleven `u16`:

| Offset | Value in ENVIRO | Meaning |
|---:|---:|---|
| 0 | 100 | **Boot module** — the module the engine loads first |
| 2 | 401 | **Boot word id** — the word it runs (`RUN`) |
| 4 | 2500 | GFX slots |
| 6 | 1000 | BLK slots |
| 8 | 700 | SCR slots |
| 10 | 25 | PAL slots |
| 12 | 10 | FNT slots |
| 14 | 10 | FRT slots |
| 16 | 100 | TXT slots |
| 18 | 1 | Open — reads like the volume number of this file |
| 20 | 3 | Open — possibly the volume count or a format revision |

Sixteen zero bytes follow at 22–37.

The boot pair is the 16-bit counterpart of the 32-bit engine's `SYSTEM.RSC`:
there is no bootstrap file, the container itself says where to start. The
engine carries the strings `DATA.-#i-` and `Bitte Diskette #d`, so the
format is designed for several volumes; ENVIRO ships one, and every occupied
slot is in it.

## Occupancy table (offset 38)

One `u16` per slot — 4345 of them, the sum of the seven counts — holding 0 or
1. **1 means occupied.** Measured against the offset table below, the two
agree slot for slot: a flag is 1 exactly when the slot's offset differs from
the next slot's. With a single volume the table is redundant; with several it
is presumably what says which volume to ask for, which is why it is listed
here as a fact and the volume logic as open.

## Offset table (offset 8728)

One `u32` file offset per slot, 4345 entries, ending at 26108. Twelve zero
bytes follow — three `u32` zeros — and the first payload byte is at 26120.

- An **empty** slot has the same offset as the next slot.
- The **size** of an occupied slot `i` is `off[j] - off[i]`, where `j` is
  the next slot with a different offset, or the end of the file for the last
  one. The last occupied item starts at 7 608 232 and runs to the end of the
  file.

## Segment map

| Segment | Slots | What the script sees | Page |
|---|---|---|---|
| GFX | 0–2499 | sprite id = slot | [Sprites](sprites.md) |
| BLK | 2500–3499 | block id = slot − 2500 | [Blocks](blocks.md) |
| SCR | 3500–4199 | **module number** = slot − 3500 | [Script modules](script-modules.md) |
| PAL | 4200–4224 | palette id = slot − 4200 | below |
| FNT | 4225–4234 | font id = slot − 4225 | [Fonts](fonts.md) |
| FRT | 4235–4244 | only slot 0 is occupied | [Fonts](fonts.md) |
| TXT | 4245–4344 | text table id = slot − 4245 | [Text tables](text-tables.md) |

What ENVIRO puts in those slots — 1586 sprites, 130 blocks, 65 modules, 23
palettes, 3 fonts, 1 font reference table, 96 text tables — is counted in
the [resource inventory](../../games/enviro/inventory.md).

### Palettes

A PAL item is 768 bytes: 256 entries of `u8 r, g, b`, each 0–63 (6-bit VGA
DAC values; every byte of ENVIRO's 23 palettes is ≤ 63). That is the layout
of the 32-bit engine's palette item and of its loose `000.PAL`, byte for byte
— see [Palettes (MOTION 32-bit)](../../motion32/formats/palette.md) for the
6-to-8-bit expansion. `SETPAL ( id -- )` installs one; `RUN` installs 0 and
then 1, the intro installs 22.

## How the engine addresses the segments

- `=>GET ( module -- )` and `=>ERASE` take a **module number**, i.e. a SCR
  slot minus 3500. `=>GET` of an empty slot is an error in the original — it
  asks for a diskette, which is the multi-volume path.
- `GET ( nbytes dest-addr block-id -- )` takes a **local block id** (0–999),
  not the 4345-wide slot index.
- Palettes, fonts and text tables are selected by kernel words (`SETPAL`,
  `+FONT`, `SDTB`) with their local ids.
- Sprite ids are literals in the scripts (`2482`, `2050`, `399`). The GFX
  space also holds **mirror aliases**: `XGFXVFLIP ( src dst count -- )`
  (`05f1:221b`; `GFXVFLIP` `05f1:21f2` is the count-1 form) copies no
  pixels — per target id it sets the flag byte `ds:[0x2694+id] = 0x80`
  and writes the source id into the table behind `ds:[0x764E]`, and the
  loader mirrors on first use. Id 2464 is **not** in the container
  (`motionvm-tools sprite … 2464` answers *no such sprite*): the alias
  slots are ids no resource holds, which is how the intro's card flip
  gets 2483's mirror image. The third argument is a **count** — `1880
  1889 9` aliases a run of nine.

The engine's strings `#F0R3i.txt`, `#F0R4i.gfx` and the reference to a
`gfx.inf` that is not shipped are authoring-time file names; the shipped
game is addressed through the container only.

## Open questions

- The two header values at 18 and 20 (1 and 3).
- How the occupancy table and the volume prompt interact when more than one
  `DATA.-n-` exists; no second volume is on hand.

## See also

- [Sprites](sprites.md), [Fonts](fonts.md), [Text tables](text-tables.md),
  [Blocks](blocks.md), [Script modules](script-modules.md) — the item formats
- [Resource inventory](../../games/enviro/inventory.md) — what ENVIRO ships in each segment
- [RSC containers (MOTION 32-bit)](../../motion32/formats/rsc-container.md) — the 32-bit engine's counterpart
