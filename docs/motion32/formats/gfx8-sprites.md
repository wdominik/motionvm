[← Documentation index](../../README.md)

# GFX8 — 8-Bit Sprites

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit-enviro).*

GFX8 items are the game's only graphics format: everything from mouse cursors
to full 640×400 room backgrounds. Each sprite is a paletted 8-bit image,
LZW-compressed, and carries its **own complete 256-color palette**.

All multi-byte values are little-endian.

## Item layout

| Offset | Type | Description |
|---|---|---|
| `+0` | `u8[2]` | Header; the two bytes duplicate the red and green channels of palette color 0 (see below) |
| `+2` | `u8[768]` | VGA palette: 256 × RGB, 6 bits per channel |
| `+770` | `char[8]` | Magic `32BITGFX` |
| `+778` | `u32` | Unpacked length (`6 + width*height`) |
| `+782` | `u32` | Packed length of the first block — unreliable, see below |
| `+786` | `u32` | Maximum LZW code width (11 or 12) |
| `+790` | … | LZW stream ([GFXCRUNCH codec](lzw.md)) |

## The decompressed stream

The unpacked data begins with its own 6-byte header, followed by the pixels:

| Offset | Type | Description |
|---|---|---|
| `+0` | `u16` | Width |
| `+2` | `u16` | Height |
| `+4` | `u16` | Color count (always 256) |
| `+6` | … | One byte per pixel, top row first |

The declared unpacked length always equals `6 + width*height`.

## Palette and transparency

- Every sprite carries a full 256-entry palette in 6-bit VGA DAC form; see
  [Palettes](palette.md) for the channel widening rule.
- **Palette index 0 is the transparency color.** When a sprite is drawn onto
  a screen, pixels with index 0 are skipped.

## The unreliable packed-length field

The packed length at `+782` is only correct for streams that fit into a
single block (1256 of the 1678 sprites). For larger sprites it undercounts.
The only trustworthy size is the **unpacked length at `+778`** — decoding
must be driven by it, not by the packed length. The GFXCRUNCH stream has no
end marker, so the unpacked length is also the decoder's only termination
condition.

## The two header bytes

Across all 1678 sprites Dunkle Schatten 2 ships, byte 0 equals the **red** channel of
palette color 0 and byte 1 the **green** channel — without exception. The
first five bytes of an item therefore read R, G, R, G, B: the pair at the
front duplicates the start of the embedded palette. Why the duplication
exists (a truncated leading palette copy, a writer artifact, or a field
that merely coincides with those channels) is unknown; readers can ignore
the two bytes.

## Open questions

- The purpose of the duplicated color-0 bytes at offset `+0`.
- The exact block structure implied by the packed-length field (why it
  matches only single-block streams).

## See also

- [The GFXCRUNCH LZW codec](lzw.md)
- [Palettes](palette.md)
- [RSC containers](rsc-container.md) — where sprites are stored
- [Descriptors](../engine/descriptors.md) — how sprites are placed on screen
