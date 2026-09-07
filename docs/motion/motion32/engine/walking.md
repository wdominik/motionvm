[← Documentation index](../../README.md)

# Walking — `DOWALK`, `CROUTE` and the Route Graph

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2 and V0.04.15/R78 with Checker 2000; what is measured here is measured on those games' files, and an address is R109's unless the page says otherwise. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

A figure is an ordinary sprite [descriptor](descriptors.md). What moves it
is a **command queue** that script code writes and the kernel word
`DOWALK` consumes — but not one command per call. The handler opens on a
gate:

```text
0x78881  cmpl $0,0x1C8(%eax)
0x7888b  jne  0x78E14          ; already walking: step, fetch nothing
```

So the queue is only touched **between** walks. A `1002` hands the
destination to `CROUTE` (0x7780c), which fills a buffer with ready-made
steps and opens the gate; every call after that spends one step, and the
gate shuts again on arrival. Nothing interpolates while it draws — the
stepper only replays what the planner worked out.

```text
DOWALK ( person -- )
```

Static analysis of the handler suggests far higher pop/push counts;
those are artifacts of native words invoking other primitives through
the Forth stack (see [Kernel words](../vm/kernel-words.md)).

## The person record

In **module memory**, not engine memory — the scripts reach it with
ordinary `@`/`+@`. `DEF_KARSTEN` and `KARSTEN_BIG` (module 202) write
every field below in decimal.

| Offset | Description |
|---|---|
| `0x00`…`0x3C` | **Walk-cycle pair per heading**: 3 `(0,4)`, 4 `(8,0xC)`, 2 `(0x10,0x14)`, 1 `(0x18,0x1C)`, 8 `(0x20,0x24)`, 7 `(0x28,0x2C)`, 5 `(0x30,0x34)`, 6 `(0x38,0x3C)` |
| `0x40`…`0x7C` | **Turn frames per ring transition**, first and last of a run |
| `0x80`…`0x9C` | **Standing sprite per heading** (used by command 1006) |
| `0xA8` | Scale factor ×10 — a value of 10 means full size |
| `0xAC` | Personal Z bias, added to the route's Z |
| `0x190`, `0x194` | Descriptor and screen (`ACTDESC`, `ACTSCR`) |
| `0x198` | Current heading, 1–8; 0 is drawn as 3 |
| `0x19C` | Heading wanted: −1 none, 1–8 fresh, 101–108 / 201–208 while turning |
| `0x1A0` | Index into the step buffer; −1 means arrived |
| `0x1A4` | The step buffer (0x18-byte records) |
| `0x1A8` | Script-side "is walking" flag — what `?THERE` reads |
| `0x1AC` | Shadow record (below) |
| `0x1B4`, `0x1B8` | Route table (`_ROUTE`) and extended table (`_XROUTE`) |
| `0x1BC` | How many steps the buffer holds (50) |
| `0x1C0`, `0x1C4` | Command queue and its cursor, in cells |
| `0x1C8` | **The gate**: non-zero means a walk or turn is in progress |
| `0x1CC` | The command being worked on |
| `0x1D0` | Bit 0: walk-cycle representation (see below) |

### The shadow record, `person[0x1AC]`

| Offset | Description |
|---|---|
| `+0` | Descriptor handle |
| `+4` / `+8` | Current / destination route, **1-based** |
| `+0x18` / `+0x1C` | Destination x / y (a y of −1 means "take it off the route's line") |
| `+0x20` | Non-zero for an eight-heading figure; Karsten is a four |
| `+0x24` / `+0x28` | Where the figure is now |
| `+0x2C` | Current heading |
| `+0x30` / `+0x34` | Step speeds across and down (Karsten: 20 and 15) |
| `+0x38` | Current scale in thousandths |
| `+0x3C` | Current Z |

### Two ways to hold a walk cycle

`person[0x1D0]` bit 0 decides how the pairs at `0x00`…`0x3C` read:

- **clear** — the pair is the first and last sprite id of a run, counted
  up by `STEPMULTI` each step and restarted when it passes the end.
- **set** — the first is a handle to a zero-terminated array of sprite
  ids and the second the index into it. Karsten works this way:
  `KARSTEN_BIG` puts `_KVORN`, `_KHINTEN`, `_KLINKS`, `_KRECHTS` — the
  four ten-frame cycles — into the four cardinal slots.

(The park's slots hold pointers too, which reads as if its figures were
driven through the animation system instead. They are not: those pointers
are the cycle lists, and the park fills the standing table as well.)

## The commands

| Command | Cells | Effect |
|---|---|---|
| **999** | 1 | End of queue: clears `person[0x1A8]`, so `WALKKARSTEN` stops calling |
| **1001** | 6 | x, y, Z, scale, route → `SDCEN`/`SDOY`/`SDZ`/`SD%SHR` **and** the matching shadow fields. The only command that reads the *next* one in the same call (0x789d4) |
| **1002** | 4 | x, y, route: into the shadow, then `CROUTE`, step index 1, **gate open** |
| **1003** | 2 | A heading to reach — the gate opens and the stepper turns |
| **1006** | 2 | Face a heading without moving: standing sprite, position read back off the descriptor and written straight back |
| **1007** | 2 | Run a script word there and then (0x78dc8). No queue in the game carries one |

`1002` first asks whether the figure is already standing on that spot in
that route (0x78a01); if it is, nothing is planned and the gate stays
shut. `PSETWALK`, `DESTWALK` and `PINITFIG` lay the queues out to match:

```text
PSETWALK  [0]=1002 [1]=x [2]=y [3]=route+1  [4]=1003 [5]=dir  [6]=1006 [7]=dir  [8]=999
PINITFIG  [0]=1001 [1]=x [2]=y [3]=Z [4]=scale [5]=route      [6]=999
```

After a `1001` the cursor is left standing **on** the following command
rather than past it — the original's own off-by-one, invisible because
only a `999` ever follows and `999` reads no operands.

## The turn machinery (0x78e52)

A fresh heading is first turned into a direction of travel: the table at
`0xDBC38`, indexed by `[have][want]`, answers 1 for the ring
3-8-2-7-4-6-1-5 and 2 for the other way round, always the shorter arc.
The heading becomes `want + 100·way` and nothing is drawn that tick.

Each following tick plays one ring step through 0x786bd: the frame pair
for that transition is advanced by one, position, scale and Z are
re-asserted from the shadow record, and the heading is committed on the
tick that shows the last frame. With a pair whose two ends are the same
sprite — which is how Karsten's are written — every step commits at
once, so a turn costs one call per ring step.

## The route graph

Two blocks per location, and the sizes alone pin the formats: block
*100+N* is **904 bytes** = 4 + 25 × 36, block *200+N* is **600** = 25 ×
24.

**Route record (36 B, after a 4-byte count the original never reads):**

| Offset | Description |
|---|---|
| `+0`…`+0xC` | `x0, y0, x1, y1` — a rectangle's corners, or a line's ends |
| `+0x10`…`+0x20` | Up to five neighboring route indices, the list ending at the first −1 |

**Extended record (24 B, same index):**

| Offset | Description |
|---|---|
| `+0` | Z for the whole route (the stepper adds `person[0xAC]` and passes it to `SDZ`) |
| `+4` | Kind: 0 rectangle, 1 and 2 the two line orientations |
| `+8` / `+0xC` | Scale at each end, interpolated along the route |
| `+0x10` | Fixed step size, or −1 to use the shadow speeds |
| `+0x14` | Flags: bit 0 arrive on both axes together, bit 1 mirror the facing, bit 2 interpolate the scale along x |

Measured in the park: the first route is `(−80, 311)-(198, 323)`, kind 1,
neighbors 1 and 2 — the diagonal Karsten spawns on — with Z 48, step 12
and scale 900; the uphill route carries the ramp 900 → 800.

## `CROUTE` (0x7780c)

1. Seed step 0 from the shadow record.
2. If the destination route is not the current one, `CALCROUTE`
   (0x7829c) twice for the next two hops. That is a depth-first search
   over the neighbor lists (0x78347) scoring a path by the width plus
   height of the rooms it crosses, and answering with the **first** hop
   of the cheapest one.
3. Find the leg's waypoint (0x76dcc): inside the destination route that
   is the target itself, with a missing y taken off the route's line
   (0x76448); otherwise the point the two routes share, found by
   comparing coordinates for **equality** — so neighboring routes have
   to meet exactly.
4. Step until the buffer is full or the destination is reached. Each
   axis moves by the route's fixed step or by the shadow speed scaled
   with the figure, sticks to the line on a diagonal, and is retired the
   moment it reaches the waypoint. A heading is written only on the
   first step of a leg — the slope decides it against the thresholds
   0.2, 1.3 and 5.0 — and the rest carry −1, meaning "keep facing".
5. The end of the path is a step whose Z is −1.

When the stepper runs past the buffer it calls `CROUTE` again and
carries on from step 1.

## The script-side walking words

The game library (module 5) writes the queues; all operate on a person
record (usually `_ACTPERSON`).

| Word | Effect |
|---|---|
| `?WALK ( -- )` | walk to the mouse: `_MMX/_MMY` → `_DESTX/_DESTY`, then `DESTWALK` |
| `XYWALK ( x y -- )` | walk to a point |
| `DESTWALK ( -- )` | the planner for a click (below) |
| `PSETWALK ( x y route dir person -- )` | queue a walk and a facing — skipped entirely if the shadow says the figure is already there |
| `PSETSTOP ( dir person -- )` | stop and face only |
| `PINITFIG ( cen oy lev shr route person -- )` | place a figure: a `1001` and a `1006`, each pumped through `DOWALK` at once |
| `PSETSTEPS ( a b person -- )` | write the shadow record's step speeds |
| `?THERE ( person -- f )` | true when `person[0x1A8]` is clear |

`DESTWALK` plans a walk from a click: hit-test the 35 location-item
rectangles (`?XINSIDE`) and walk to the item's stored destination, else
hit-test the click areas (10 × 20-byte records, field `+16` the route
number) and walk to the click point clamped into that route's box.

Walk speed: the options menu applies `_GSMODE` (1–3) through `STEPMULTI`,
which multiplies both the planner's step sizes and the walk-cycle
advance.

## The protagonist's walk set (module 9)

Module 19 is a byte-identical copy; module 212 embeds a third.

- Four **walk cycles of ten sprites**: right 102–111, front 113–122,
  back 124–133, left 135–144.
- Four **standing frames**: 112, 101, 123, 134 — the ids missing between
  the cycles. `KARSTEN_BIG` writes them into `0x80`…`0x9C` and also into
  all eight turn slots, which is why his turns take one tick per step.
- `DEF_KARSTEN` builds the figure and hangs `WALKKARSTEN` on the
  descriptor as its `SDWORD` callback; that word calls `DOWALK` on
  every second frame while `person[0x1A8]` is set.

The mirrored left/right cycles are made at startup with `XGFXVFLIP`.

## Offset access helpers

`+@ ( addr offset -- value )` and `+! ( value addr offset -- )` are
literally `+ @` and `+ !` — not standard Forth, defined in the globals
module.

## Open questions

- Command 1007's call path (no queue in the game writes one).
- Whether the two line kinds mean anything beyond which end anchors the
  scale interpolation and the order of the overshoot clamps.
- The waypoint cases for route pairs that share no exact coordinate:
  some leave the previous waypoint standing, which may be unreachable by
  construction rather than intended.

## See also

- [Descriptors](descriptors.md)
- [Blocks](../formats/blocks.md) — the per-location route data
- [Module map](../../games/ds2/module-map.md)
