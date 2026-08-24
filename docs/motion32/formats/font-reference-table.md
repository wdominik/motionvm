[← Documentation index](../../README.md)

# 000.FRT — Font Reference Table

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit-enviro).*

`000.FRT` maps character codes to glyph indices. The fonts themselves store
only a linear array of glyphs (see [Fonts](fonts.md)); this table says which
glyph renders which CP437 character code.

All multi-byte values are little-endian.

## Layout

| Offset | Type | Description |
|---|---|---|
| `+0` | `u16` | Number of character slots (256) |
| `+2` | `u16` | Number of glyphs (124) |
| `+4` | `u16[256]` | Glyph index per character code; `0xFFFF` where the font has no glyph |

## Properties

- `A`–`Z` map to a contiguous run starting at glyph 0.
- The table is shared: one `000.FRT` serves all nine fonts. There is no
  per-font character map.
- Character codes are CP437 bytes, so German umlauts resolve through the
  `0x80..0xFF` range.

At runtime the engine reads the table from a fixed location in its data
segment (`0xE8378`) when measuring and drawing text.

## See also

- [Fonts](fonts.md)
- [Text rendering](../engine/text-rendering.md)
- [Text tables](text-tables.md)
