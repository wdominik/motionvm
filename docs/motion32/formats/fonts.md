[← Documentation index](../../README.md)

# Fonts

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Fonts are proportional 1-bit bitmap fonts, compressed with the same
[GFXCRUNCH LZW codec](lzw.md) as sprites — but unlike the sprite header, the
font header states the codec parameters explicitly instead of assuming them.

Dunkle Schatten 2 ships nine fonts (`001.RSC` font slots), 968 glyphs in total, with
heights from 11 to 40 pixels. `000.FNT` on disk is the **system font** in
the same format; it is the fallback whenever a text descriptor has no font
selected (see [Text rendering](../engine/text-rendering.md)).

All multi-byte values are little-endian.

## Container layout

| Offset | Type | Description |
|---|---|---|
| `+0` | `u16` | Unpacked size |
| `+2` | `u16` | The same size again |
| `+4` | `u16` | Packed size (file length minus this 10-byte header) |
| `+6` | `u16` | Dictionary limit, 2048 — codes grow to 11 bits |
| `+8` | `u16` | Initial code width, 9 |
| `+10` | … | LZW stream |

## Decompressed layout

| Offset | Type | Description |
|---|---|---|
| `+0` | `u16` | Glyph count `n` |
| `+2` | `u16` | Height, shared by every glyph in the font |
| `+4` | `n × { u16 offset, u16 width }` | Glyph table |
| … | | Bitmaps: `ceil(width/8)` bytes per row, `height` rows, 1 bit per pixel |

The glyph table ends exactly where the first bitmap begins — a useful
consistency check.

## Bit order: least significant bit first

Within each bitmap byte, pixels are stored **LSB first** (bit 0 is the
leftmost pixel of the byte). This is the one property the structure itself
does not reveal: with the opposite order the output resembles glyph-like
shapes that are not letters. Rendering the first glyphs of `008.FNT`
settles it — they spell A B C D E F G H, matching the
[font reference table](font-reference-table.md), which maps `'A'` to
glyph 0.

## Glyph rendering

- A set bit is a pixel of the text color; an unset bit is transparent.
  Glyphs are drawn as masks, never as filled rectangles.
- Glyph advance: one pixel of spacing follows every glyph but the last, so
  the width of a run of glyphs is `Σ(width + 1) − 1`. The spacing value is
  an engine global, not part of the font format — see
  [Text rendering](../engine/text-rendering.md).

## Open questions

- The duplicated size field at `+2` and the packed size at `+4` are
  redundant with observable quantities; whether the engine ever reads them
  is unknown.

## See also

- [The GFXCRUNCH LZW codec](lzw.md)
- [Font reference table (000.FRT)](font-reference-table.md)
- [Text rendering](../engine/text-rendering.md)
