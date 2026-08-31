[← Documentation index](../../README.md)

# Transitions

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

`FADEIN` and `FADEOUT` take three arguments and hold the interpreter inside
their own handler while the curtain runs — the one place in this engine, other
than the frame loop itself, where a word occupies more than one frame.

## Transitions

`FADEIN` (`05f1:29e4`) and `FADEOUT` (`05f1:2827`) take the 32-bit pair's
`( mode duration step -- )` but draw a **box**, not a band. `FADEOUT` in
mode 1 fills four strips a ring, the black frame growing in from the
view's edges toward its center, and a last fill takes what the rings
leave; `FADEIN` first sets the screen active and composes it whole
(`016a:0821`), then blits the same strips out of the surface, the box
growing from the center, and squares the rounding with a whole-view copy
— it blanks nothing, so it opens over whatever the display holds. A ring
is `(width / (2·step) + 7) & ~7` wide — 24 on the 320-wide view — and
`height / (2·step)` tall — 10 on the 160-tall one — with one ring taken
off where the rounded width would overrun the view, so the game's
`1 50 8` walks seven rings. Each ring waits `200 / duration` ticks of the
200 Hz clock (`05f1:2998`): 20 ms a ring, about 140 ms a fade, measured
identical against a frame-rate capture of the original. Mode 0 is the
same effect all at once; any other mode skips the drawing but keeps the
flag work. Both handlers hide the pointer for the duration, spin inside
themselves — the frame loop does not turn — and `FADEOUT` sets the screen
inactive where `FADEIN` set it active, the coupling `GSCRACT` reads.
Unlike the 32-bit handler, `FADEOUT` leaves the screen's surface
untouched: only the display goes black.

What erases, instead, is the frame step. `SCRACT` (`05f1:094a`) clears
the freeze bits (`0x4000`, `0x2000` of the screen word at `+0x1E`) and
sets **bit 0 of the flags at `+0x18` — the same rebuild request `SCRPOS`
makes**; `SCRINACT` (`05f1:09b9`) freezes (`+0x1E |= 0x4000`), sets the
same bit, and releases the screen's save-unders. The per-frame screen
step (`016a:0821`) reads them: a frozen screen is cleared once
(`016a:0880`); a running one takes the **full** path while any of the
low flag bits stands — surface cleared, save-unders released, every
active descriptor drawn (`016a:092c`–`016a:09ef`) — and the incremental
path otherwise, with the bit walking `0x1 → 0x2 → 0x4` so a rebuild
holds for three frames (`016a:0848`). `FADEIN` calls `SCRACT` and then
the step directly, which is why a scene can swap its pictures under a
`FADEOUT`/`FADEIN` pair without erasing anything itself — the intro and
the start-up page's teardown both lean on it.

## Open questions

- The `XGFXVFLIP` mirror axis: left-right — a flip about the vertical axis —
  is what a walk cycle and the card flip need, and it is unmeasured against a
  capture.

## See also

- [Screens and the draw chain](screens.md) — what a curtain is drawn over
- [Descriptors](descriptors.md) — the flags a transition reads
- [Boot and frame loop](game-loop.md) — the frames a transition takes
- [Transitions (MOTION 32-bit)](../../motion32/engine/transitions.md) — the other generation's
