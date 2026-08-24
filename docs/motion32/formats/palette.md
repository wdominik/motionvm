[← Documentation index](../../README.md)

# PALETTE — VGA Palettes

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit-enviro).*

A palette item is exactly **768 bytes**: 256 entries of R, G, B with **6 bits
per channel**, stored in the order they are written to the VGA DAC. The
standalone file `000.PAL` uses the same format.

`001.RSC` ships 60 palette items. Sprites additionally embed a private
palette of the same 768-byte form (see [GFX8 sprites](gfx8-sprites.md)).

## 6-bit to 8-bit widening

When converting the 6-bit DAC values to 8-bit color channels, the top bits
are replicated into the bottom:

```
v8 = (v6 << 2) | (v6 >> 4)
```

This maps full scale 63 to 255 rather than 252 (and 0 to 0). The mapping is
injective, so an 8-bit value produced this way can be converted back to its
6-bit original with a plain `>> 2`.

## Runtime use

- `SETPAL ( id -- )` loads a palette item into the DAC. The whole display
  reinterprets immediately: the frame buffer holds palette indices, not
  colors, so a palette change recolors everything already drawn.
- The first sprite drawn after startup establishes the initial palette from
  its embedded copy if no palette has been set yet.

## See also

- [GFX8 sprites](gfx8-sprites.md) — embedded per-sprite palettes
- [RSC containers](rsc-container.md)
- [Transitions](../engine/transitions.md) — palette changes during scene
  transitions
