[← Documentation index](../README.md)

# Graphics

*PARSEC — Dunkle Schatten (1994): palettes, pictures, cels and the font, in the files `XP00.EXE` and `XP01.EXE` load them from.*

The screen is VGA mode 13h, 320×200 in 256 colors. The game's pictures fill
it in three bands: the 320×128 playfield a room is drawn in, and under it the
72 rows that the status line and the strip share. The title program's
stills fill the whole screen.

## Palettes are banks

Nothing loads one 256-color dump. Each file writes its own **bank** of DAC
registers, and the banks divide the DAC between them:

| Registers | Written by |
|---|---|
| 0–127 | The room picture, from the palette at the head of its `RDV1.00` resource |
| 128–159 | `PLAY00.DAT` resource 0, the player |
| 160–191 | `PANELS.DAT` resource 1, the strip |
| 224–255 | `CMBMOU.DAT` resource 0, the pointer and the text inks |

A bank is `u8 start, u8 count` followed by exactly `count` RGB triples —
`2 + 3 × count` bytes. `XP01.EXE`'s loader `0590h:0A1Fh` (`0x0631F`) reads
those two bytes and nothing else, and takes a count of 0 for 256
(`0x0633E`–`0x06342`); `XP00.EXE` reads the count at `+1` and the triples
from `+2` the same way (`0x33D9`–`0x33F2`). Neither has a form without the
header. Channels are six-bit DAC values, 0 to 63.

## Pictures and cels

Two header shapes sit in the RIB resources:

- **A picture** is `u16 width`, `u16 height` and `width × height` pixels,
  top row first. The title program's stills are 320×200; the strip under
  the playfield is 320×72 (`PANELS.DAT` resource 0).
- **A cel** is `u16 width`, `u16 height`, `u16 flags`, `u16 0`, then its
  pixels. The player's walk cels (25×46, flags 2), the verb icons (24×24,
  flags 1), the pointer cels (flags 2) and the room tiles (16×16, flags 0
  or 1) all share it.

Flags 0 and 2 are `width × height` raw pixels; flags 1 is an opcode stream.
Index 0 is see-through wherever a sprite is drawn over a scene.

Which of the two a whole resource is follows from its size: `4 + w × h`
equal to its length for a picture, `8 + w × h` for a cel, with the width
between 1 and 320 and the height between 1 and 200. A cel *inside* a room
resource is the eight-byte form at the front of the bytes an offset names,
and the next record follows it directly.

### The flags-1 stream

The stream is a run of words, each with its opcode in the high byte and a
parameter in the low byte; every opcode ends by fetching the next word. It
fills a `width × height` cel row by row:

| Opcode | What it does |
|---|---|
| `00` | Ends the cel |
| `02` | Skips a whole row |
| `04` | Copies a whole row |
| `06` | Next word: copies its high byte's count of pixels, skips its low byte's, row ends |
| `08` | Next word: skips its low byte's count, copies its high byte's, row ends |
| `0A` | As `06`, then one byte more: that many pixels copied |
| `0C` | One byte skipped-count first, then as `06` |
| `0E` | Next word: skips its low byte's count, then its high byte's count of (copy, skip) word pairs |

Pixels the stream skips are not drawn.

## The pointer file, `CMBMOU.DAT`

One RIB: resource 0 is the bank at 224, resource 1
starts the pointer's cels — fourteen, the ones `XP01.EXE`'s loader visits
(`0x05D45`–`0x05D86`) into the table `[195Eh]` the pointer's descriptors
index — and resource 15 is the font.

## The font

The one font the game draws all its text with is resource 15 of
`CMBMOU.DAT`. `XP00.EXE` sets its font record up from it the way
`XP01.EXE` does (`0x0A3D`–`0x0AF6`).

| Offset | Field |
|---|---|
| `+0` | `2C 2E`; not read |
| `+2` | The first character code, `20h` |
| `+4` | The glyph count, `C2h` = 194 — `20h` to `E1h`, up to `ß` |
| `+6` | 7 |
| `+8` | The line height, 12 |
| `+0Ah` | The glyph records, eight bytes each |

A glyph record sits at `block + 0Ah + (code − first) × 8`:

| Offset | Field |
|---|---|
| `+0` | Zero when the font has no glyph for this code |
| `+2` | The advance in pixels |
| `+6` | The glyph's cel, as an offset from `block + 0Ah` |

122 of the 194 records carry a glyph. The advance stands whether or not
the record has a bitmap.

**The letter spacing is not in the file.** It is byte `+5` of the
program's text record `[196Ch]`, written as `0FFh` by the loader at
`0x5D13` and read signed by the status line's painter (`0x0C73F`): every
advance has −1 added, so glyphs overlap by one column and their outlines
meet. A line of `n` glyphs is `n` advances plus `n − 1` spacings.

**Ink is a ramp, not a color.** A glyph cel is an ordinary flags-1 cel whose
literals are 0 and 1, and it is painted as the ink plus the literal: ink
`k` paints `k` and `k+1`. Here the ink names the outline and the next
palette entry is the body — `0FEh` is white on black, `0F4h` yellow, `0F0h`
gray — which is why the pointer file's bank at 224 is built in pairs.

## The player, `PLAY00.DAT`

Resource 0 is the bank at 128; the resources from 1 on are the player's
25×46 walk cels. The program stores the whole file as the player's bundle
and steps the slot past the palette by one far pointer (`0x10AD0`), so the
player's cel 0 is resource 1.

## The strip, `PANELS.DAT`

Resource 0 is the 320×72 picture under the playfield, resource 1 its bank
at 160. Where on the strip each button and inventory slot is comes from the
program's own widget table, not from this file ([executables](executables.md)).

## See also

- [Rooms](rooms.md) — the room picture, its palette, tiles and objects
- [Resource containers](containers.md) — the RIBs these resources sit in
- [Animations](animations.md) — what moves a cel
