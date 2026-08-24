[← Documentation index](../../README.md)

# The GFXCRUNCH LZW Codec

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit-enviro).*

Sprites and fonts share one compression scheme, called GFXCRUNCH inside the
engine. It is ordinary LZW over an 8-bit alphabet with one unusual property:
the encoder announces every code-width increase explicitly instead of letting
the decoder infer it.

## Parameters

| Parameter | Value |
|---|---|
| Alphabet | 8-bit bytes (codes 0–255) |
| Bit order | MSB first |
| Initial code width | 9 bits |
| Maximum code width | 11 or 12 bits, declared by the enclosing format |
| First free dictionary entry | 258 |

## Reserved codes

| Code | Name | Meaning |
|---|---|---|
| 256 | Clear | Reset the dictionary and return to 9-bit codes |
| 257 | Bump | Increase the code width by one bit, capped at the maximum |

The Bump code is the distinguishing feature. GIF and TIFF widen codes
implicitly as soon as the dictionary fills up, and use 257 as an end-of-input
marker. Here the **encoder signals each widening explicitly**, and there is
**no end marker at all**: the decoder runs until it has produced the output
length declared by the enclosing format.

## Decoding notes

- Dictionary entries are built as usual: after each decoded code, append
  (previous string + first byte of current string) as the next free entry.
- The KwKwK case (a code referencing the entry about to be defined) occurs
  and must be handled; any code beyond the next free entry is an error.
- The dictionary conceptually keeps counting entries past its capacity so the
  KwKwK boundary check stays in sync with the encoder, even though such codes
  can never be read back at the capped width.
- Because there is no end marker, the declared output length is the only
  termination condition. Reading must stop exactly there; the packed length
  fields in the enclosing formats are not reliable (see
  [GFX8 sprites](gfx8-sprites.md)).

## Where it is used

| Format | Maximum width | Where the parameters come from |
|---|---|---|
| [GFX8 sprites](gfx8-sprites.md) | 11 or 12 | Declared in the sprite header |
| [Fonts](fonts.md) | 11 | Declared explicitly in the font header (dictionary limit 2048, start width 9) |

All 1678 sprites in Dunkle Schatten 2's containers decode to exactly the output
length their headers declare.

## See also

- [GFX8 sprites](gfx8-sprites.md)
- [Fonts](fonts.md)
