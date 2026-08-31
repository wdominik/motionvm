[← Documentation index](../../README.md)

# Fonts

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

Two item kinds carry text rendering: FNT items hold the glyph bitmaps, and
the single FRT item maps a character code to a glyph number. Both have the
32-bit engine's layout — FNT *after* that engine's LZW decompression, FRT
byte for byte.

## FNT — bitmap fonts

A FNT item is stored raw; there is no compression header:

```
u16 n                      ; glyph count
u16 height                 ; rows per glyph, the same for every glyph
n × { u16 offset, u16 width }
; glyph bitmaps, each at `offset` from the start of the item:
;   height rows of ceil(width / 8) bytes, 1 bit per pixel,
;   the leftmost pixel in the least significant bit
```

Die Enviro-Kids greifen ein ships three fonts, and they agree on everything
but their height:

| Id | Glyphs | Height | Widths | Item size |
|---:|---:|---:|---|---:|
| 0 | 116 | 12 | 2–10 | 2556 |
| 2 | 116 | 14 | 4–12 | 3252 |
| 7 | 116 | 11 | 1–11 | 2208 |

The first glyph starts at `4 + 4 × 116` = 468 in all three, the bitmaps are
back to back, and the last one ends inside the item. No glyph has width 0.

**The bit order is least-significant-bit-first**, measured by rendering:
read that way, glyph 0 of font 0 is an `A`, glyph 1 a `B`, glyph 26 an `Ä`,
27 an `Ö`, 52 an `x`, and font 7 shows the same letters in outline; read
most-significant-bit-first the same bytes are scrambled. Font 2 holds the
solid silhouettes of those letter shapes — it is the shadow the scripts put
behind text (the boot sequence loads it into a variable named `_SHFONT`),
font 7 the outlined memo face (`_MEMO`).

## FRT — the font reference table

516 bytes in FRT slot 0, the same layout as the 32-bit engine's loose
`000.FRT` ([font reference table (MOTION 32-bit)](../../motion32/formats/font-reference-table.md)):

```
u16 256           ; entries
u16 120           ; glyph count the table was made for
u16 map[256]      ; CP437 byte → glyph number; 0xFFFF = no glyph
```

`A`–`Z` map to 0–25, `a` to 29, `0` to 68, space to 83; 143 of the 256
entries are `0xFFFF`, and the largest mapped glyph number is 115 — which is
the last glyph of a 116-glyph font, so one table serves all three.

## How the scripts use fonts

- `+FONT ( id -- handle )` loads a font and leaves a handle; `-FONT
  ( handle -- )` releases it. The boot sequence does `0 SFT`, `2 +FONT
  _SHFONT !`, `7 +FONT _MEMO !`; the intro loads font 2 as well and
  releases it on the way out.
- `SFT ( n -- )` with 0 precedes the first `+FONT` and reads as a reset of
  the font stack — a reading of the call site, not of the handler.
- `SDFNT` on a descriptor takes the **handle** `+FONT` left, not the font
  id. `RESETFONT` is in the kernel and unused by this game.
- Text is placed with `SDTB`/`SDTXT`, colored with `SDCOL`, centered with
  `SDCEN`/`SDVCEN`, and styled through `SDTDT`/`DEFTDT` templates — see
  [Descriptors](../engine/descriptors.md).

## Open questions

- What `SFT` does with a non-zero argument; the game only passes 0. Its
  stack feeds the character remap the drawer reads through `ds:0x18D8`
  ([text rendering](../engine/text-rendering.md)).

## See also

- [The DATA container](container.md) — the FNT and FRT segments
- [Fonts (MOTION 32-bit)](../../motion32/formats/fonts.md) — the compressed successor with the same decoded layout
