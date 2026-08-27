[← Documentation index](../../README.md)

# TEXT — String Tables

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

A text item is a table of NUL-terminated strings. Dunkle Schatten 2 ships 133 tables
holding 6785 strings in total: dialogue, item descriptions, UI messages.

All multi-byte values are little-endian.

## Layout

| Offset | Type | Description |
|---|---|---|
| `+0` | `u32` | String count `n` |
| `+4` | `u16[n-1]` | Start offsets of strings 1..n−1, relative to the start of the string area |
| … | | NUL-terminated CP437 strings; string 0 starts at offset 0 |

The offset table has **one entry fewer than there are strings**, because
string 0 always sits at the very start of the string area (which begins at
`4 + 2*(n-1)`).

## Content rules

- Strings are encoded in **code page 437**. German umlauts and `ß` live in
  the `0x80..0xFF` range; decoding as Latin-1 or UTF-8 produces mojibake.
- Strings may contain `\n` (0x0A) as a line separator; the text renderer
  splits on it.
- **Index 0 is usually the empty string** and serves as a "no text"
  placeholder. UI descriptors that should show nothing point at it.

## Quirks

- **Text slot 32 does not hold a table.** It contains just two zero bytes.
  A robust reader treats any item shorter than 4 bytes (and any table with
  `n = 0`) as an empty table rather than an error.

## Runtime addressing

Scripts select a string with two descriptor words: `SDTB` picks the table
(the resource id), `SDTXT` picks the entry — and `SDTXT` is **1-based**:
`SDTXT n` refers to entry `n − 1` of the table. See
[Text rendering](../engine/text-rendering.md) for details and consequences.

## See also

- [Text rendering](../engine/text-rendering.md)
- [RSC containers](rsc-container.md)
