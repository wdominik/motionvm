[← Documentation index](../../README.md)

# Screens

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

The display is composed from **screens**: independent layers, each with its
own pixel buffer, onto which [descriptors](descriptors.md) are drawn. The
game runs at 640×480 in 256 colors.

## Configuration words

`NEWSCREEN` creates a screen, returns its handle, and makes it the
*current* screen; the `SCR…` words then configure the current one:

```
NEWSCREEN            ( -- handle )    create, becomes current
640 400 SCRSIZE      ( w h -- )       logical size of the surface
640 400 SCRFVSIZE    ( w h -- )       full view size
640 400 SCRVSIZE     ( w h -- )       view window size
0 0 SCRVPOS          ( x y -- )       position of the view on the display
0 0 SCRPOS           ( x y -- )       scroll position within the surface
```

`ACTSCR ( handle -- )` selects a screen as current; `SCRACT`/`SCRINACT`
set and clear its active flag; `ERASESCR` clears its buffer; `REMSCR`
removes it. `DRAWSCR`/`FRESHSCREEN` request a redraw.

### Interpretation of the position words

The reading of `SCRVPOS` as "where the view sits on the display" and
`SCRPOS` as "scroll offset within the surface" is a **hypothesis** — it is
the interpretation under which the startup layout (below) tiles the display
exactly and under which the composed title screen matches the original
pixel for pixel. It has not been confirmed against the handler code.

### What the pixel-exact match does and does not settle

The strongest evidence for the reading above is a measurement rather than a
handler: the title macro's composed frame, held against a capture of the
original running under DOSBox-X, agrees on **307,200 of 307,200 pixels,
across 206 distinct palette indices**.

The comparison is on palette indices, not RGB. A screen capture has been
through the display's color profile, so its channel values are not what the
emulator put in the DAC — but the transform is one-to-one, so every index
the engine drew must come out as one and the same color everywhere it
appears, and every color must map back to one index. Checking both
directions is stronger than comparing RGB: it fails when the engine drew a
different index and passes only when it drew the same picture, and it
catches a render that collapsed two indices into one.

That the capture is pixel-doubled rather than resampled is not an
assumption either: the full 14.7-megapixel fullscreen image holds 206
distinct colors, which no interpolating scaler could produce, and the logo
measures exactly twice its size in the rendered frame.

What that settles: composition (`SCRPOS` against `SCRVPOS`), layer order,
scaling through `SD%SHR`, and the palette chain all agree with the original
engine for this frame. What it does not settle is the *meaning* of the two
position words — one frame in which both readings would produce the same
picture cannot distinguish them. It is strong evidence, not a reading of the
code.

## The startup layout

The game's `STARTUP` word creates three screens, in this order:

| Screen | Size | `SCRVPOS` | Role |
|---|---|---|---|
| 1st | 640×80 | (0, 400) | Status bar |
| 2nd | 640×400 | (0, 0) | Main picture |
| 3rd | 384×480 | (640, 0) | Dialogue surface |

The first two tile the 640×480 display without gap or overlap. The third
sits entirely **off-display** at x = 640 and is clipped away until the
engine repositions it — the dialogue surface exists but is parked outside
the visible area.

## Three layers, not one

A pixel passes through three places before anyone sees it:

| Layer | Where | Written by |
|---|---|---|
| Screen surface | one per screen | the drawer `0x6915b` |
| Software surface | `0xE7D7C`, one for the display | the drawer, the fills, the mouse layer |
| Video memory | the card | **only** the presenter `0x1457D` |

The presenter does not copy the surface. It walks an 8×8-tile **update map**
(`0xE7D84`) and copies just the tiles somebody marked with `0x18436`, then
clears the map. So video memory keeps whatever was last copied into it, and
a routine that marks only part of the display changes only that part. That
is exactly what a fade is (see [Transitions](transitions.md)) — and it is
why `FADEIN` needs no black background to open over.

## The drawn buffer

The engine renders into a **drawn buffer** at specific moments — the
drawer (`0x6915b`) runs once per frame from the game loop's frame pump,
and once inside `FADEIN` before the curtain opens. Nowhere else.

**The drawer is incremental and the surface persists.** It clears no buffer:
there is no fill anywhere in `0x6915b`. What it clears is the screen's
**damage map**, and then it repaints only the descriptors that map names.

### The damage map

Every screen carries one `u16` per 8×8 tile of its view at `screen+0x41A`,
`(view_w · view_h) >> 6` of them. Each holds **the lowest level that has to be
redrawn in that tile**, or `0x7FFF` for nothing.

| Step | Where | What |
|---|---|---|
| Mark | `0x6e701` | `map[tile] = min(map[tile], level)` over a rectangle, clipped to the view and taken relative to the screen origin at `+0x24`/`+0x26` |
| Raise | `0x6ab6e` | Every change to a descriptor marks **its own rectangle at its own level** and sets `0x50` on it |
| Spread | `0x6e8c8` | Before each pass, a descriptor is marked dirty when a tile it covers holds a level **at or below its own** (`0x6eb04`); descriptors neither active nor dirty are skipped (`0x6ea02`) |
| Empty | `0x69248` | The pass then refills the whole map with `0x7FFF` |
| Draw | `0x694ed`, `0x69680`, `0x69425` | A descriptor is drawn only with **both** `0x80` (active) and `0x40` (dirty); the bits go again at `0x69659`/`0x694c6` |

A fresh descriptor arrives ready: `NEWSETDESC` writes the flag word `0xD000` at
`0x70d07` — active, dirty, changed.

### What follows from it

- **Switching a descriptor off erases nothing.** The mark is on its own level,
  so the repaint reaches what is above it and never what is below, and the
  descriptor itself is no longer drawn. Its pixels stay on the surface until
  something paints over them. The only thing that takes a picture away is
  `SDAUTOBUF` — see [Descriptors](descriptors.md#sdautobuf--the-only-thing-that-erases). The in-game
  mailbox is built on that difference; see [the BBS](../../games/ds2/library/bbs.md).
- **A whole screen is repainted on demand**: `0x6a8f9` sets `0x50` on every
  descriptor of a screen and on its buffers. `FADEIN` reaches it through
  `0x6b0fe` (`0x74af1`) before its single draw, which is how a full picture
  comes back after a fade.
- **A surface is wiped by a fill, not by a draw.** `FADEOUT` fills the screen's
  rectangle with color 0 before its first band (`0x74d44` → `0x188fd`) and
  `ERASESCR` calls the same routine (`0x74801`).
- **Between draws, the buffer holds the last drawn frame.** `FADEOUT`
  never draws; it fades out whatever was last rendered, even if
  descriptors have changed since (see [Transitions](transitions.md)).
- **Before the first draw, the buffer is black.** State that exists but
  has never been drawn — the status bar during the very first frames,
  for instance — is simply not visible yet.

### How motionvm runs the pass

Same selection, different mechanics: it works out which descriptors have to be
drawn exactly as above, takes the union of their rectangles together with the
places `SDAUTOBUF` owes, paints the **whole** screen afresh, and then publishes
only that region onto the surface. Everything outside it stays as it was.

Painting in full and publishing in part rather than clipping each blit is a
deliberate choice: a clipped blit still has to know it was clipped — the text
passes place themselves from their own measurements — and a text backing is a
*darkening* of what is under it (`0x186d5`), which cannot be run twice over the
same pixels without showing. Publishing a region that was painted exactly once
gives every pixel one pass over it and no arithmetic to repeat.

## Compositing rules

- Descriptors draw in **level order, lowest level first** — the sibling
  chain is maintained as a stable sort by level (see
  [Descriptors](descriptors.md)).
- Drawing clips at buffer edges; nothing wraps.
- Screens are composed onto the display in creation order. A screen being
  faded in is inactive until the fade ends and yet has to show — the
  original gets there by blitting past the flag, straight to what is on
  screen, which is what a curtain does (see
  [Transitions](transitions.md)).
- A **frozen** screen (`FREEZESCR`, bit 2 of the flag byte) is skipped by
  the per-frame descriptor walk — its waits and callbacks stand still —
  until `UNFREEZESCR` clears the bit. The menu freezes the play screen
  this way.

## Screen object fields

Known fields of the engine's screen structure:

| Offset | Description |
|---|---|
| `+0x12` | Flag word; `NEWSCREEN` initializes it to `0xD000` (bit 7 of the flag byte set — a fresh screen is active) |
| `+0x13` | Flag byte; bit `0x80` = active (read by `GSCRACT`, cleared by `FADEOUT`, set by `FADEIN`); bit 2 = frozen |
| `+0x1C` / `+0x1E` | View size, read by `GSCRVSIZE` (height first, width on top) |
| `+0x24` / `+0x26` | Origin x / y, set by `SCRX`, read by `GSCRX` / `GSCRY` |
| `+0x2C`, `+0x2E` | Origin values used by the transition curtain |
| `+0x30` | Descriptor list (count at `+0x418`) |

## Open questions

- **The meaning of `SCRVPOS` and `SCRPOS`.** The reading above is a
  hypothesis, strongly supported by the startup layout and by the
  pixel-exact title composition, and unconfirmed against the handlers.
- **Descriptor flag `0x10`.** Set beside the dirty bit by `0x6ab6e` and
  cleared with it by the drawer, it picks between two blitters —
  `0x27765`/`0x29ae9` against `0x273e8`/`0x299e1`, whose destination is the
  display's software surface at `0xE7D7C` (`0x69331`, `0x697cd`). What the
  distinction is for is not established.
- **Why the inventory bar's real sprites stay invisible** before the intro's
  first fade.

## See also

- [Descriptors](descriptors.md) — what is drawn onto screens
- [Transitions](transitions.md) — the curtain effect and the active flag
- [Game structure](../../games/ds2/game-structure.md) — STARTUP in context
