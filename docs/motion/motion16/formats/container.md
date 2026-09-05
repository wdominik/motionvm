[← Documentation index](../../README.md)

# Resource Containers — `DATA.-n-`

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere, in `STERN.EXE` with Falsches Spiel mit Eddie M. and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

The 16-bit engine keeps a whole game in `DATA.-n-`, one volume per floppy it
took: a header that also names the boot word, an occupancy table, an offset
table, and then every resource back to back. Seven resource kinds share a
single slot space, segment after segment; the id a script uses is the slot
number minus the segment's base.

**How many volumes there are and whether the items are packed are two
independent choices**, and the games make them differently.
Die Enviro-Kids greifen ein ships
one volume of 7 609 296 bytes with its items stored plainly. Jeff Jet ships
two, 1 404 960 and 1 098 474 bytes, with every item packed — 2 459 890 bytes
that unfold to 8 412 811, a bigger game than the first on a third of the disc.
Hilfe für Amajambere ships two, 506 066 and 4 827 543 bytes, and packs
neither: it is the
game that shows the two properties apart, and until it was read they had never
been seen except together. The header says which shape a given game has.

**There are two framings of that header**, sixteen bytes apart, and the layout
below is the later one. The earlier one — 1993 and 1994, Victor Loomes and
Compaq — has no packing field at `0x16`, so its occupancy table begins at 22
instead of at 38 and everything after it moves with it. See
[The earlier framing](#the-earlier-framing).

All multi-byte values are little-endian.

## Header (offset 0)

Nineteen `u16`, of which the last is spare. Victor Loomes' header stops after
the eleventh — the seven packing words and the spare are the later framing's,
and its column is `—` for them ([the earlier framing](#the-earlier-framing)):

| Offset | Die Enviro-Kids greifen ein | Jeff Jet | Hilfe für Amajambere | Victor Loomes | Description |
|---:|---:|---:|---:|---:|---|
| 0 | 100 | 100 | 100 | 100 | **Boot module** — the module the engine loads first |
| 2 | 401 | 401 | 401 | 449 | **Boot word id** — the word it runs (`RUN`) |
| 4 | 2500 | 2500 | 2500 | 1200 | GFX slots |
| 6 | 1000 | 1000 | 1000 | 500 | BLK slots |
| 8 | 700 | 700 | 700 | 700 | SCR slots |
| 10 | 25 | 25 | 25 | 21 | PAL slots |
| 12 | 10 | 10 | 10 | 10 | FNT slots |
| 14 | 10 | 10 | 10 | 10 | FRT slots |
| 16 | 100 | 100 | **150** | 30 | TXT slots |
| 18 | 1 | 2 | 2 | 2 | **Volumes** the game ships on — in the earlier framing it is not that: Victor Loomes says two and ships one, and its player has no name former for a second |
| 20 | 3 | 7 | 7 | 0 | **Spare** `u32` entries after the offset table |
| 22 | 0 | 1 | 0 | — | GFX items are packed |
| 24 | 0 | 1 | 0 | — | BLK items are packed |
| 26 | 0 | 1 | 0 | — | SCR items are packed |
| 28 | 0 | 1 | 0 | — | PAL items are packed |
| 30 | 0 | 1 | 0 | — | FNT items are packed |
| 32 | 0 | 1 | 0 | — | FRT items are packed |
| 34 | 0 | 1 | 0 | — | TXT items are packed |
| 36 | 0 | 0 | 0 | — | Spare |

The boot pair is the 16-bit counterpart of the 32-bit engine's `SYSTEM.RSC`:
there is no bootstrap file, the container itself says where to start.

The seven flags at 22–36 are read one at a time, by the segment being loaded.
The item loader (`HPPLAY.EXE` file `0x400c`, `ENVIRO.EXE` file `0x401f` — the
same routine in both builds it has been read from) branches on the authoring-time extension of the
segment, and each branch fetches its own word: `.gfx` takes `+0x16`, `.blk`
`+0x18`, `.fth` `+0x1a`, `.pal` `+0x1c`, `.fnt` `+0x1e`, `.frt` `+0x20`,
`.txt` `+0x22`. Zero reads the item straight into the caller's buffer;
anything else reads it aside and unpacks it. Measured over the four
multi-volume-capable games on hand — Die Enviro-Kids greifen ein, Jeff Jet,
Hilfe für Amajambere, Falsches Spiel mit Eddie M. —
the flag and the shape of the items agree in all 28 segments, with no
exception in either direction.

## Occupancy table (offset 38)

One `u16` per slot — 4345 in Die Enviro-Kids greifen ein and Jeff Jet, 4395 in
Hilfe für Amajambere, the sum
of the seven counts.
Zero means the slot is empty; anything else is a **volume bitmask**,
`1 << (volume − 1)`. The engine builds the same value from the volume it has
open and matches it (`00e0:0017` in the loader: the current volume from
`ds:0x1af2`, less one, shifted into a `1`). No slot names two volumes.

The words of Die Enviro-Kids greifen ein are all 0 or 1, and with one volume that
makes the table
redundant with the offsets. Jeff Jet's are 0, 1 and 2: 2615 empty, 1187 on
`DATA.-1-`, 543 on `DATA.-2-` — and among those 543 are every palette, both
fonts and the font reference table, so a reader that ignored the second
volume would not render the game worse, it would render nothing. Those of
Hilfe für Amajambere are 0, 1 and 2 as well, but flagged by whole segment
ranges rather than by
slot: 1460 empty, 400 on `DATA.-1-`, 2535 on `DATA.-2-` — all 2500 GFX slots
among them, of which 1045 carry bytes.

## Offset table

One `u32` per slot, and then `spare` more that are zero. Volume 1 keeps it
behind the occupancy table; **a second or third volume is nothing but this
table from byte 0, and then its items** — no header, no occupancy words of its
own, the same 4345 slots. `DATA.-2-`'s first entry is 17408, which is
`4 × (4345 + 7)`: the table's own length, saying where it ends. The table of
Hilfe für Amajambere says 17608, which is `4 × (4395 + 7)` for its larger
slot space.

The first byte a volume's items may occupy is therefore
`38 + 2·n + 4·(n + spare)` for volume 1 and `4·(n + spare)` for the rest —
26120 in Die Enviro-Kids greifen ein, 26136 and 17408 in Jeff Jet, 26436 and
17608 in Hilfe für Amajambere,
and the first item sits exactly there in all three. (Falsches Spiel mit Eddie M.'s second and third volumes leave 24 bytes between the
two, so this is where items may begin and not where they must.)

- An **empty** slot has the same offset as the next slot, in every volume.
- The **size** of an occupied slot `i` is `off[i+1] − off[i]` read in *that
  slot's own volume*, or to the end of that volume for the last slot. A
  volume's offsets run over all 4345 slots, so a slot that lives elsewhere
  simply repeats its neighbour's offset and the arithmetic still lands.
- Measured over all eight volumes of the four later-framing games: every
  volume's items tile it exactly, from its own table's end to its last byte,
  with nothing left over.
- A slot can be flagged and still have no bytes. Jeff Jet has two, sprites
  1319 and 1848, whose offsets are degenerate in the volume they name.
  Hilfe für Amajambere has **1534**, because it flags whole segment ranges
  and fills what
  it has — 1455 GFX slots, 45 text tables, 31 blocks and three fonts that are
  claimed and empty. What has bytes is what is there, in both games and for the
  same reason: presence is read from the offsets and the flag word is kept as a
  diagnostic (`motionvm-motion-tools info` counts it).

## Packed items

An item of a packed segment is an eight-byte header and then a GFXCRUNCH LZW
stream:

| Offset | Description |
|---:|---|
| 0 | `u16` unpacked length |
| 2 | `u16` packed length — the item's size less these eight bytes |
| 4 | `u16` 2048, the dictionary size |
| 6 | `u16` 9, the initial code width |

The stream is the codec the 32-bit engine packs its sprites and fonts with,
bit for bit — see [GFXCRUNCH LZW](../../formats/lzw.md) — with a
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

Die Enviro-Kids greifen ein and Jeff Jet use the same seven counts, so the bases
above are theirs. Hilfe für Amajambere reserves 150 text tables rather than
100, which leaves every base up
to TXT unchanged and makes its slot space 4395 — the counts are read from the
header, so a different geometry is data and not a case to handle.

What Die Enviro-Kids greifen ein puts in those slots — 1586 sprites, 130 blocks,
65 modules, 23
palettes, 3 fonts, 1 font reference table, 96 text tables — is counted in
[its resource inventory](../../games/enviro/inventory.md); Jeff Jet's 1470,
119, 55, 16, 2, 1 and 65 in [its own](../../games/jeffjet/inventory.md), and
1045, 153, 76, 24, 7, 1 and 95 in
[its own](../../games/hfa/inventory.md).

### Palettes

A PAL item is 768 bytes: 256 entries of `u8 r, g, b`, each 0–63 (6-bit VGA
DAC values; every byte of the 23, 16, 24 and 21 palettes the four games ship is
≤ 63).
That is the layout of the 32-bit engine's palette item and of its loose
`000.PAL`, byte for byte — see
[Palettes (MOTION 32-bit)](../../motion32/formats/palette.md) for the
6-to-8-bit expansion. `SETPAL ( id -- )` installs one; in
Die Enviro-Kids greifen ein `RUN`
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
  (`motionvm-motion-tools sprite … 2464` answers *no such sprite*): the alias
  slots are ids no resource holds, which is how the intro's card flip
  gets 2483's mirror image. The third argument is a **count** — `1880
  1889 9` aliases a run of nine.

The engine's strings `#F0R3i.txt`, `#F0R4i.gfx` and the reference to a
`gfx.inf` that is not shipped are authoring-time file names; the shipped
game is addressed through the container only.

## The earlier framing

Victor Loomes' container and Compaq's are the same format with one field
missing. Nothing in either file says which framing it is, so it is worked out
by reading the file as the earlier one and asking whether it adds up — two
identities that have to hold at once:

* the offset table's first entry is where the tables end, `22 + 2n + 4(n + spare)`;
* its last entry is the file's length.

Both hold for Victor Loomes (14 848 and 1 009 597) and for Compaq (14 848 and
445 972). No later container can satisfy the first, whose table begins sixteen
bytes further on. Both are needed, not either: Jeff Jet's last offset happens
to be its file length.

Three things differ once the framing is known.

**There is no packing field**, so which segments are packed is not written
down. It is the same in both games that use the framing, measured over their
items: sprites and fonts are packed, the font reference table is packed, and
blocks, modules, palettes and text tables are stored plainly — 721 of 721
sprites and 2 of 2 fonts in Victor Loomes, 311 and 6 in Compaq.

**A packed item's header is ten bytes rather than eight**: the unpacked length
is repeated in front of the later header, so the packed length and the two
GFXCRUNCH parameters sit two bytes further on. The two copies agree in all 723
packed items of Victor Loomes and all 318 of Compaq. The font reference table
is the exception in both games and carries the eight-byte header.

**The occupancy word is a plain flag**, not a volume bitmask, because there is
only one volume to be on. Both games hold 2 in the word at `0x12` that the
later framing uses for a volume count, and ship one file; `LL.EXE` has only
the literal `data.-1-` where the later builds hold the `DATA.-#i-` name former
(file `0x1462e`), so it could not open a second volume if one existed. What
that word means here is open.

The earlier-framing games also ship a `GFX.INF` beside the container, which the
later ones name and none of them ships: one `u16` width and height per GFX
slot, `0xFFFF, 0xFFFF` for an empty one. A player that unpacks an item on
demand cannot read a sprite's size out of a packed item without unpacking it
first, and this is where it reads it instead. See
[Other files (Victor Loomes)](../../games/vloomes/other-files.md).

## Open questions

- What the spare `u32` entries at the end of the offset table are for. Their
  count is in the header and they are zero in every shipped container.
- What the engine does when a volume it wants is not in the drive. It carries
  the prompts — *"Bitte Diskette #d einlegen!"*, *"Datenblock <#s> nicht
  gefunden."* — and the path they sit on has not been read.
- What Jeff Jet's two contradictory GFX slots mean. 1319 and 1848 are flagged
  for a volume — one for each — whose offset table gives them a length of
  zero, both in the middle of a run of occupied slots. Nothing depends on the
  answer: what has bytes is what is there.

## See also

- [Sprites](sprites.md), [Fonts](fonts.md), [Text tables](text-tables.md),
  [Blocks](blocks.md), [Script modules](script-modules.md) — the item formats
- [GFXCRUNCH LZW](../../formats/lzw.md) — the codec a packed item's stream is
- [Resource inventory](../../games/enviro/inventory.md) — what Die Enviro-Kids greifen ein ships in each segment
- [Resource inventory (Jeff Jet)](../../games/jeffjet/inventory.md) — and what Jeff Jet does
- [RSC containers (MOTION 32-bit)](../../motion32/formats/container.md) — the 32-bit engine's counterpart
