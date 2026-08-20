[← Documentation index](../README.md)

# Screens

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
and once inside `FADEIN` before the curtain opens. Nowhere else. Three
consequences:

- **Each draw is stateless**: the screen buffers are cleared and rebuilt
  from the descriptor chain every time — deactivating a descriptor makes
  it vanish on the next draw without any `ERASESCR`.
- **Between draws, the buffer holds the last drawn frame.** `FADEOUT`
  never draws; it fades out whatever was last rendered, even if
  descriptors have changed since (see [Transitions](transitions.md)).
- **Before the first draw, the buffer is black.** State that exists but
  has never been drawn — the status bar during the very first frames,
  for instance — is simply not visible yet.

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

| Offset | Meaning |
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
- **Why the inventory bar's real sprites stay invisible** before the intro's
  first fade.

## See also

- [Descriptors](descriptors.md) — what is drawn onto screens
- [Transitions](transitions.md) — the curtain effect and the active flag
- [Game structure](game-structure.md) — STARTUP in context
