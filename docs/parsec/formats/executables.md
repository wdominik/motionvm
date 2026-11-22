[← Documentation index](../README.md)

# The Program Files

*PARSEC — Dunkle Schatten (1994): `XP00.EXE`, `XP01.EXE` and `XP02.EXE`, how they are packed, and what in their images is data.*

Dunkle Schatten is three DOS programs: `XP00.EXE`, the title program with
its logos, menu and backstory; `XP01.EXE`, the game; and `XP02.EXE`. All
three are MZ executables compiled with Turbo C and packed the same way, and
everything the documentation cites by address is an offset into the
**unpacked** load image, never into the file.

## Packing

A packed program carries `PRSC` at the MZ header's extra bytes, offset
`1Ch`. In front of a stock **LZEXE 0.91** decompressor sits a stub of two
steps, at the packed `CS:IP` (`CS:01D0`):

1. The first decrypts, in place, a bilingual anti-debug notice at
   `CS:000C`.
2. The second decrypts five words at `CS:0000` with the key `1234h`. The
   second word is the paragraph of the LZEXE 0.91 info block, and the fifth
   decrypts to `534Eh` — `"NS"`.

The LZEXE information then reads as LZEXE's always does: the original
`CS:IP` (Turbo C's `0000:0000`), `SS:SP`, and the relocation table at
`CS_lz:0158`, in the 0.91 encoding.

**The bit stream starts at load offset 0** — file offset `e_cparhdr × 16`,
`20h` for a two-paragraph header — and not where the usual LZEXE unpacker's
formula, which uses the packed `CS`, puts it: the stub sits *behind* the
compressed image.

| Program | Unpacked image | `DGROUP` |
|---|---:|---:|
| `XP00.EXE` | 64 176 bytes | `0E3Ah` |
| `XP01.EXE` | 150 096 bytes, 2 441 relocations | `21EFh` |
| `XP02.EXE` | 28 816 bytes | `01E2h` |

## Addresses

The load image is laid out for base segment 0. A linear address such as
`0x4FC0` is an offset into it; a far address `segment:offset`, such as
`0BB6h:027Ah`, is the same place as `segment × 16 + offset`; a far pointer
stored in the image is already relocated for that base. A variable in
brackets — `[1F4Ah]` — and an address written `DS:xxxx` are offsets into the
program's data segment, `DGROUP`.

`DGROUP` is not a constant to look up: it is read out of the program.
Turbo C's entry code opens by loading it into `DX` as an immediate, the word
at image offset 1, and some twenty bytes later moves `DX` into `DS`. Between
the two it stores `DX` into a word of the code segment, asks DOS for its
version and reads the PSP's words at 2 and `2Ch`. The three programs'
prologues are byte for byte the same from offset 0 to `15h` except for the
two immediates — the segment, and the address it is stored at.

## What the images hold as data

A good deal of what the game shows lives in the programs rather than in
`RESOURCE/`, and is read out of the images as data:

- **`XP01.EXE`:** the location table — one 52-byte record per location at
  `204Bh:0000`, each carrying its kind, its exits and their destinations,
  the objects its entry sets up and far pointers to its handler, its name
  and its interaction rules; where a fresh game starts; the object names,
  examine paragraphs and the verbs' fail lines; the status-line templates,
  a sixteen-entry row at `DS:1084` with their `%c` ink controls; the panel's
  colors and ramps; the strip's seventeen widget records at `2115:0000`;
  the substitution row the text controls index; the starting inventory.
- **`XP00.EXE`:** its tables and strings — the menu's lines and the six pages of the backstory among them.

## See also

- [Resource containers](containers.md) — the files beside the programs
- [The RIB LZ codec](lz.md) — the other compression, inside the resources
- [Graphics](graphics.md) — the screen the programs draw
