[← Documentation index](../../README.md)

# The DATA Container

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein and in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, which is an older build of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names the other game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit-ds2).*

The 16-bit engine keeps a whole game in `DATA.-n-`, one volume per floppy it
took: a header that also names the boot word, an occupancy table, an offset
table, and then every resource back to back. Seven resource kinds share a
single slot space, segment after segment; the id a script uses is the slot
number minus the segment's base.

ENVIRO ships one volume of 7 609 296 bytes with its items stored plainly.
Jeff Jet ships two, 1 404 960 and 1 098 474 bytes, with every item packed —
2 459 890 bytes that unfold to 8 412 811, a bigger game than ENVIRO's on a
third of the disc. Both are the same format; the header says which shape a
given game has.

All multi-byte values are little-endian.

## Header (offset 0)

Nineteen `u16`, of which the last is spare:

| Offset | ENVIRO | Jeff Jet | Meaning |
|---:|---:|---:|---|
| 0 | 100 | 100 | **Boot module** — the module the engine loads first |
| 2 | 401 | 401 | **Boot word id** — the word it runs (`RUN`) |
| 4 | 2500 | 2500 | GFX slots |
| 6 | 1000 | 1000 | BLK slots |
| 8 | 700 | 700 | SCR slots |
| 10 | 25 | 25 | PAL slots |
| 12 | 10 | 10 | FNT slots |
| 14 | 10 | 10 | FRT slots |
| 16 | 100 | 100 | TXT slots |
| 18 | 1 | 2 | **Volumes** the game ships on |
| 20 | 3 | 7 | **Spare** `u32` entries after the offset table |
| 22 | 0 | 1 | GFX items are packed |
| 24 | 0 | 1 | BLK items are packed |
| 26 | 0 | 1 | SCR items are packed |
| 28 | 0 | 1 | PAL items are packed |
| 30 | 0 | 1 | FNT items are packed |
| 32 | 0 | 1 | FRT items are packed |
| 34 | 0 | 1 | TXT items are packed |
| 36 | 0 | 0 | Spare |

The boot pair is the 16-bit counterpart of the 32-bit engine's `SYSTEM.RSC`:
there is no bootstrap file, the container itself says where to start.

The seven flags at 22–36 are read one at a time, by the segment being loaded.
The item loader (`HPPLAY.EXE` file `0x400c`, `ENVIRO.EXE` file `0x401f` — the
same routine in both builds) branches on the authoring-time extension of the
segment, and each branch fetches its own word: `.gfx` takes `+0x16`, `.blk`
`+0x18`, `.fth` `+0x1a`, `.pal` `+0x1c`, `.fnt` `+0x1e`, `.frt` `+0x20`,
`.txt` `+0x22`. Zero reads the item straight into the caller's buffer;
anything else reads it aside and unpacks it. Measured over the four
multi-volume-capable games on hand — ENVIRO, Jeff Jet, Amajambere, Eddy M. —
the flag and the shape of the items agree in all 28 segments, with no
exception in either direction.

## Occupancy table (offset 38)

One `u16` per slot — 4345 of them in both games, the sum of the seven counts.
Zero means the slot is empty; anything else is a **volume bitmask**,
`1 << (volume − 1)`. The engine builds the same value from the volume it has
open and matches it (`00e0:0017` in the loader: the current volume from
`ds:0x1af2`, less one, shifted into a `1`). No slot names two volumes.

ENVIRO's words are all 0 or 1, and with one volume that makes the table
redundant with the offsets. Jeff Jet's are 0, 1 and 2: 2615 empty, 1187 on
`DATA.-1-`, 543 on `DATA.-2-` — and among those 543 are every palette, both
fonts and the font reference table, so a reader that ignored the second
volume would not render the game worse, it would render nothing.

## Offset table

One `u32` per slot, and then `spare` more that are zero. Volume 1 keeps it
behind the occupancy table; **a second or third volume is nothing but this
table from byte 0, and then its items** — no header, no occupancy words of its
own, the same 4345 slots. `DATA.-2-`'s first entry is 17408, which is
`4 × (4345 + 7)`: the table's own length, saying where it ends.

The first byte a volume's items may occupy is therefore
`38 + 2·n + 4·(n + spare)` for volume 1 and `4·(n + spare)` for the rest —
26120 in ENVIRO, 26136 and 17408 in Jeff Jet, and the first item sits exactly
there in both. (Eddy M.'s second and third volumes leave 24 bytes between the
two, so this is where items may begin and not where they must.)

- An **empty** slot has the same offset as the next slot, in every volume.
- The **size** of an occupied slot `i` is `off[i+1] − off[i]` read in *that
  slot's own volume*, or to the end of that volume for the last slot. A
  volume's offsets run over all 4345 slots, so a slot that lives elsewhere
  simply repeats its neighbour's offset and the arithmetic still lands.
- Measured over all nine volumes of the four games: every volume's items tile
  it exactly, from its own table's end to its last byte, with nothing left
  over.
- A slot can be flagged and still have no bytes. Jeff Jet has two, sprites
  1319 and 1848, whose offsets are degenerate in the volume they name. What
  has bytes is what is there.

## Packed items

An item of a packed segment is an eight-byte header and then a GFXCRUNCH LZW
stream:

| Offset | Meaning |
|---:|---|
| 0 | `u16` unpacked length |
| 2 | `u16` packed length — the item's size less these eight bytes |
| 4 | `u16` 2048, the dictionary size |
| 6 | `u16` 9, the initial code width |

The stream is the codec the 32-bit engine packs its sprites and fonts with,
bit for bit — see [GFXCRUNCH LZW](../../motion32/formats/lzw.md) — with a
2048-entry dictionary, which is an 11-bit ceiling.

The decoder (`1614:000d` in `HPPLAY.EXE`, file `0x1934d`) reads the packed
length at `+2`, steps the source on by eight, and starts at width 9 with 258
as the first free code. It never reads the 2048 and the 9 back: the header
writes down what the code already assumes. All 1728 of Jeff Jet's items carry
that header and decode to exactly the length it declares.

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

Both games use the same seven counts, so the bases are the same for both.
What ENVIRO puts in those slots — 1586 sprites, 130 blocks, 65 modules, 23
palettes, 3 fonts, 1 font reference table, 96 text tables — is counted in
[its resource inventory](../../games/enviro/inventory.md); Jeff Jet's 1470,
119, 55, 16, 2, 1 and 65 in [its own](../../games/jeffjet/inventory.md).

### Palettes

A PAL item is 768 bytes: 256 entries of `u8 r, g, b`, each 0–63 (6-bit VGA
DAC values; every byte of ENVIRO's 23 and of Jeff Jet's 16 palettes is ≤ 63).
That is the layout of the 32-bit engine's palette item and of its loose
`000.PAL`, byte for byte — see
[Palettes (MOTION 32-bit)](../../motion32/formats/palette.md) for the
6-to-8-bit expansion. `SETPAL ( id -- )` installs one; in ENVIRO `RUN`
installs 0 and then 1 and the intro installs 22, and Jeff Jet's `RUN` opens
the same way.

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

- What the spare `u32` entries at the end of the offset table are for. Their
  count is in the header and they are zero in every shipped container.
- What the engine does when a volume it wants is not in the drive. It carries
  the prompts — *"Bitte Diskette #d einlegen!"*, *"Datenblock <#s> nicht
  gefunden."* — and the path they sit on has not been read.

## See also

- [Sprites](sprites.md), [Fonts](fonts.md), [Text tables](text-tables.md),
  [Blocks](blocks.md), [Script modules](script-modules.md) — the item formats
- [GFXCRUNCH LZW](../../motion32/formats/lzw.md) — the codec a packed item's stream is
- [Resource inventory](../../games/enviro/inventory.md) — what ENVIRO ships in each segment
- [Resource inventory (Jeff Jet)](../../games/jeffjet/inventory.md) — and what Jeff Jet does
- [RSC containers (MOTION 32-bit)](../../motion32/formats/rsc-container.md) — the 32-bit engine's counterpart
