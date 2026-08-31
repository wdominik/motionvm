[← Documentation index](../../README.md)

# Sprites

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

A GFX item is a raw, uncompressed 8-bit picture. There is no magic, no
compression and no palette of its own — three things the 32-bit engine's
[GFX8 format](../../motion32/formats/sprites.md) adds.

```
u16 width
u16 height
u16 0                        ; zero in every occupied slot
u8  pixels[width * height]   ; palette indices, row-major, top row first
```

Measured over all 1586 occupied GFX slots of Die Enviro-Kids greifen ein: the
third field is 0 in
every one, and every item's length is exactly `6 + width × height`.

## What the game ships

- 262 distinct sizes. Widths run from 8 to 320, heights from 1 to 177; the
  largest picture is 320×155 (49 600 pixels). Nothing is wider than the
  320-pixel display, and no single sprite is a full 320×200 screen — rooms
  are assembled from parts.
- Frequent sizes: 32×20 (160 sprites), 120×109 (92), 64×54 (91), 80×155
  (75), 16×9 (47), 32×56 (41).
- Slots 0–11 are all 64×115 — the first dozen ids read as placeholders. The
  real art is sparse across the 2500-wide id space; the scripts name ids like
  399 (the pointer, 16×15), 2050 (80×155), 2482–2486 (the intro's 160×77 to
  168×83 motifs) and 2493 (304×117).

## `GFX.INF`, and who needs it

The two earlier-framing games ship a `GFX.INF` beside the container: one `u16`
width and height per GFX slot, `0xFFFF, 0xFFFF` where the slot is empty, and
nothing else — 4800 bytes for the 1200 slots both declare. It exists because
of how they store their sprites. Theirs are packed, so a player that wants a
sprite's size before it draws cannot read one out of the item without
unpacking it first; the later games store sprites plainly, where the width is
the item's first word, and ship no such file although all four binaries still
name one.

Over Victor Loomes' 721 filled slots the file and the container agree entry
for entry, and every unpacked length is `width * height + 6`. motionvm does
not read it at run time — see [departures](../../departures.md) — because its
container unpacks as it opens, so the sizes are in the sprites by the time
anything asks.

## Colors

The pixel byte is an index into whatever palette `SETPAL` last installed
(see [the DATA container](container.md#palettes)). How index 0 is
drawn depends on what the picture is to the descriptor: the drawer
(`ENVIRO.EXE` `016a:0aac`) sends a **sprite** (`SDSPR`) through one blit
and a **block** (`SDBL`) through a plain copy that paints every index, 0
included — the location backgrounds are 80-pixel block strips, dark where
they hold 0. A sprite's index-0 surround stays unpainted by the picture's
evidence; the sprite blit itself (`14ee:0d1e`) is unread
([descriptors](../engine/descriptors.md)).

## The sprite blit, read

`14ee:0d1e` wraps a clipped copy (`14ee:0ba2`): the rectangle is cut
against both surfaces' headers, the width split into an 8-aligned run
and a remainder, and the two pixel loops (`17f2:084b`, `17f2:0911`)
test every byte — **a pixel of index 0 is skipped**, everything else
copied. That is the sprite key; the block path (`14ee:0d47`) copies
plainly. The row-start arithmetic multiplies the y offset by the width
rounded **down** to eight (`17f2:0868`), which is exact for the
8-aligned screen surfaces; what it does to a sprite of odd width whose
top is clipped is left open below.

## Open questions

- The blit's row-start arithmetic on a top-clipped sprite whose width is
  not a multiple of eight (`17f2:0868`: `y × (width & ~7)`) — a shear by
  the reading, unobserved in play.
- The ids above 2400 that the game writes into at run time (`XGFXVFLIP`)
  and frees on leaving a location (`420 2499 -1 XGFXSTAT`): the exact
  meaning of that status call is unread.

## See also

- [The DATA container](container.md) — where the GFX segment sits
- [Descriptors](../engine/descriptors.md) — how a sprite reaches the screen (`SDSPR`)
- [GFX8 sprites (MOTION 32-bit)](../../motion32/formats/sprites.md) — the compressed, palette-carrying successor
