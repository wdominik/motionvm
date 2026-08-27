[← Documentation index](../../README.md)

# Transitions — FADEOUT and FADEIN

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Scene transitions are a **curtain (wipe), not a palette fade**, despite the
names. Neither handler touches the DAC: the palette writer is `0x82078`, its
only caller is `0x820b8`, and no path from either fade reaches it. Both words
take

```
FADEOUT ( mode duration step -- )
FADEIN  ( mode duration step -- )
```

and the game calls them with `1 50 8` throughout.

## The arguments

The **first** is a mode (below). The second is the duration, used only for
the per-band delay (see [Timing](#timing)).

The **third is popped and never used.** `-4(%ebp)` appears exactly once in
each handler, in the store that saves it (`0x74a5a`, `0x74c91`). The band
height is a literal 8 in the code — `addl $0xFFFFFFF8` at `0x74b11`,
`addl $8` and `mov $8,%ecx` at `0x74d62`/`0x74d68`. The game passes 8
everywhere, so nothing depends on the distinction; it matters only as a
warning not to read the argument as a step size.

`FADEIN` also takes its band **width** from a different field than `FADEOUT`:
`+0x18` at `0x74b22`, which `SCRSIZE` writes, against `+0x1C` at `0x74d70`,
which `SCRVSIZE` writes. Every screen the game builds sets both to the same
value. Heights are `+0x1E` in both — the **view** height, which `SCRVSIZE`
writes; `SCRSIZE` keeps a separate height at `+0x1A` that the fades never
look at. Origins are `+0x2C`/`+0x2E`.

## The mode argument

Both handlers compare the mode against 1 and have a second branch for mode 2
that the game never invokes — all 180 call sites pass 1.

Mode 2 is **not** a black curtain. `FADEOUT` mode 2 advances in steps of two
and fills its bands through `0x28479` with color `0x102` (`0x74eef`), in up
to four phase-shifted offsets. `0x102` is not a color: the 8-bit fill path
at `0x18584` treats a value ≥ 256 as a row in the **darkening tables** the
second half of `SETPAL` builds (`0x147ff` and its siblings, one table per
subtracted amount). So mode 2 is a *translucent* fade — an interlaced
dimming, not a blanking. `FADEIN` mode 2 draws a white box and a black frame
at (25,122)–(452,317) before its band loop, which looks like authoring-tool
furniture rather than a game effect.

## Geometry

The effect operates on the screen currently selected with `ACTSCR` — not on
the display as a whole. Height and origin come from the screen object
(fields `+0x1E`, `+0x2C`, `+0x2E`). The title macro, for example, fades the
status bar while the picture above it stays put.

**FADEOUT** fills the buffer with color 0, then per pass copies **two**
bands of `step` rows while `i` runs from 0 to `height/2`:

```
top:     y = base + i
bottom:  y = base + height - step - i
```

Black grows from both edges toward the middle.

**FADEIN** first forces a redraw of the target screen (the finished image
is in the buffer *before* anything becomes visible), then runs `i` from
`height/2` down to 0, copying **one** band:

```
y = base + i,   band height = (height/2 - i) * 2
```

At `i = height/2` the band is zero rows tall; at `i = 0` it covers
everything — the picture opens from the middle outward.

Both directions reduce to the same shape: a symmetric band of visible rows
around the screen's vertical center that grows or shrinks. At 480 rows and
step 8 that is **31 passes per direction** — the number that multiplies the
per-band delay in [Timing](#timing).

## Interaction with the screen active flag

`FADEOUT` **clears** the screen's active bit (bit `0x80` of flag byte
`+0x13`); `FADEIN` **sets** it — the same bit `GSCRACT` reads. The scripts
depend on this coupling: `INCLLOC` (the location loader) ends with

```
GSCRACT NOT IF … FADEIN
```

— a location is revealed *because* the preceding fade-out left the screen
inactive. Note the compositing consequence: a screen mid-fade-in is by
definition still inactive, yet must already be drawn.

## Nothing is drawn to the visible screen — tiles are marked

This is the part that decides how a fade *looks*, and it is not in the band
arithmetic at all.

Everything the engine draws goes into one **software surface** (`0xE7D7C`).
Beside it sits an 8×8-tile **update map** (`0xE7D84`). `0x18436` clips a
rectangle and marks the tiles it covers — it copies no pixels. `0x146D3`
clears the whole map. The **presenter** `0x1457D` walks the map and copies
just the marked tiles into video memory, then clears it again. Video memory
is therefore persistent: **anything nobody marked keeps showing what it
showed before.**

Both handlers use that deliberately, and in opposite ways:

- **`FADEOUT`** fills the screen's rectangle of the surface with color 0
  (`0x74d44`), *then* clears the map (`0x74d49`), then marks two bands per
  pass. Every band it marks turns black, and black grows in from the edges.
- **`FADEIN`** draws the screen (`0x74af9`), *then* clears the map
  (`0x74afe`), then marks one growing band. **It blanks nothing.** Outside
  the band, video memory still holds the previous frame — so a `FADEIN` with
  no `FADEOUT` before it does not open out of black, it reveals the new
  picture *over* the old one.

The second point is not a curiosity. The game's menu is built on it: opening
the load page is a bare `FADEIN` (`0x01af4`), and a help-page turn is
`SHOW_DOC` followed by **two `FADEIN`s and no `FADEOUT` at all**
(`0x03bc0`/`0x03be8`). Treating a fade in as "black everything the band has
not reached" — which is exactly right for a fade out — makes every one of
those blink through black.

`FADEIN` also hands the drawer **the fading screen itself**, nothing else,
after setting `0xD0` on its flags (`0x74ae7`: the fade is what brings the
screen to life) and marking its content (`0x6B0FE`). The band loop then runs
synchronously in the handler, so no other screen gets a frame while a fade is
up; a change elsewhere waits for the next `ANIMPLAY` frame. `FADEOUT` never
calls the drawer at all.

Both handlers hide the pointer for the duration: `HIDEMOUSE` at `0x74ae2`
before the loop, `SHOWMOUSE` at `0x74b64` after it.

`ERASESCR` (`0x747bc`) is `FADEOUT`'s black fill without the curtain — it
fills and marks but never calls the presenter, so it shows at the next frame.

Both therefore operate on a **still frame fixed when the curtain
starts**. This matters because scripts flip descriptors immediately
before fading: location 1's task manager runs
`_BLACK SMDESC SDINACTIVE  1 50 8 FADEOUT` — the black overlay is
switched off and the fade starts, but since nothing draws in between,
what fades out is still the *old* picture with the overlay. The new
state becomes visible only when the next `FADEIN` draws it. (See
[Screens](screens.md) for the drawn-buffer model behind this.)

## Blocking

Both words run their curtain loop entirely **inside** the word, with the
mouse cursor hidden; the bytecode does not advance until the curtain is
done. One intro phase performs, in a single invocation:

```
FADEOUT   …swap background descriptors…   91 SETPAL   FADEIN
```

While the curtain closes, the background swap and the palette change have
genuinely not happened yet — the old image in the old palette is what gets
covered. See [Execution model](../vm/execution-model.md) for the
suspension mechanism this requires.

## Timing

The handler divides twice, and the two divisions do different jobs — which
is the thing to get straight, because the band count is **not** the loop
bound:

```
bands = view_height / 16      0x74cd0 / 0x74a99   only ever the divisor below
half  = view_height / 2       0x74ce7 / 0x74ab0
delay = duration / bands      0x74cf6 / 0x74abc   signed idiv, truncating
loop:   i = 0 … half, step 8  0x74d55             -> half/8 + 1 passes
```

The wait itself is at `0x74db6` (`FADEIN`: `0x74b3c`): poll `GIVETIMER`
(`0x236FE`) and spin while the elapsed count is **unsigned-below** `delay`,
then reset the timer to zero (`0x236A0`) and call the presenter. So each
band waits `delay` ticks measured from the previous band, there is **no
minimum** — a `delay` of 0 waits not at all — and `bands == 0` (a view
under 16 rows) would divide by zero. Neither is reachable with Dunkle Schatten 2's
screens.

(motionvm clamps both divisions rather than reproducing the division by
zero — a [departure](../../departures.md), invisible with Dunkle Schatten 2's screens.)

The spin quantizes, and the quantization is part of the arithmetic. A
unit-3 timer reads `(raw − stamp) × 10 / 51`, truncating (`0x23761`), so
"elapsed ≥ delay" first holds at `ceil(delay × 51/10)` raw ticks of the
1020 Hz master — see [Game loop](game-loop.md) for that rate. A delay of
1 therefore costs **6** raw ticks (5.88 ms), not the nominal 5.1; a
delay of 2 costs **11** (10.78 ms); a delay of 10 costs **51**, which is
50 ms exactly. The stretch lands precisely on the short delays — that
is, on the big screens.

**A larger screen still fades faster**, because the truncation of
`50/bands` bites harder than the quantization stretches:

| view height | `bands` | `delay = 50/bands` | passes | raw ticks/band | whole fade |
|---|---|---|---|---|---|
| 480 — the title screen | 30 | **1** (from 1.67) | 31 | **6** | **0.182 s** |
| 400 — the picture in play | 25 | **2** (exact) | 26 | **11** | **0.280 s** |
| 80 — the status bar | 5 | 10 | 6 | **51** | **0.300 s** |

The intro's fades are therefore the fastest in the game by a factor of
1.5, and that is the original's arithmetic, not an accident of any port.
The timing is independent of the frame rate: while a fade runs the frame
loop does not turn at all.

> **Under DOSBox-X the intro fades run ~2.4× slower than this table**,
> and that is the emulator, not the game. Measured on the running
> original (930 bands, timed by its own unit-3 timer): the master
> averages the design rate, but its ticks are delivered in bursts
> aligned to the emulated 70 Hz refresh, and the burst size moves with
> the `cycles` setting (15.2 s at `cycles=max`, 22.7 s at
> `cycles=10000`, sound on or off makes no difference). A 1-tick band
> then waits a whole burst (~14–16 ms) instead of 6 raw ticks, so the
> title wipe stretches from 0.18 s to ~0.44 s while the status bar's
> 51-tick bands barely notice. A side-by-side against DOSBox-X shows
> exactly that difference — the emulator's interrupt batching, not the
> engine's arithmetic.

**The picture is updated once per pass, after the wait** — mark the band
(`0x18436`), wait, reset, present (`0x1457D` at `0x74dcf` / `0x74b55`). So
a fade is 31, 26 or 6 *pictures*, at up to 200 a second. Anything that
drives the curtain off the 25 fps frame clock instead gets the duration
right and the movement wrong: eight bands arrive at once on the title
screen, and a wipe that shows four of its thirty-one steps reads as a cut.
The band is the step, not the frame.

One asymmetry at the start: `FADEIN`'s first pass marks a band of height
`(half - i) * 2 = 0`, which `0x18436` rejects outright (`jle` at
`0x18471`). It costs a full `delay` and shows nothing.

See [Game loop](game-loop.md) for the tick base — and for the reason it
cannot be closed from `ENGINE.EXE` alone.

## Several curtains at once

The interpreter is stopped while a curtain runs, so ordinarily one fade
finishes before the next word is reached. There is one exception, and it is
the whole game menu: `DO_INVSEL` arrives as a **descriptor callback**
(`CALLMENU` arms it with `SDWORD`/`SDWAIT`), and a callback runs
re-entrantly, blocking three times in a row. Its documents case starts three
fades in one call: `FADEIN` on the bar, `FADEOUT` on the picture, `FADEIN` on
the picture (`0x01c44`–`0x01ca8`).

The consequence for anything that reproduces this: the later `FADEIN` has
already drawn the new picture into the screen's buffer before the earlier
`FADEOUT` has shown a single band. A fade out that composed its frame from
the buffers would hide a picture nobody had seen yet. Keeping what is on
screen — as the original's video memory does by construction — is what makes
the order come out right.

## Open questions

- What `FADEIN` mode 2's white box and frame are for. The drawing is read;
  the intent is not.

## See also

- [Screens](screens.md)
- [Execution model](../vm/execution-model.md) — blocking words
- [Game loop](game-loop.md)
- [Departures](../../departures.md) — the clamps, the step clock, the timing base
