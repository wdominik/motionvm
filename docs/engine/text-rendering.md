[← Documentation index](../README.md)

# Text Rendering

Text is drawn through type-4 [descriptors](descriptors.md). The string
comes from a [text table](../formats/text-tables.md); the glyphs from a
[font](../formats/fonts.md) via the shared
[reference table](../formats/font-reference-table.md). The pipeline has
two halves: the **setters measure** (`SDTXT`, `SDCOL`, `SDTDT` re-run
the layout after every change and cache it), and the **drawer**
(`0x6915b`, once per frame from the [game loop](game-loop.md)) renders
the cached layout. `DRAWSCR` draws nothing — it only marks.

## Selecting the text

- `SDTB ( table -- )` selects the text **table**, `SDTXT ( entry -- )`
  the entry — **1-based**: `SDTXT n` displays entry `n − 1`. (With
  table 6, `109 SDTXT`/`110 SDTXT` yield entries 108/109 — the
  production credit and the intro backstory; read 0-based, stray debug
  texts appear.)
- UI descriptors idle on `SDTXT 1` = entry 0, the empty string.
- `GDTXTLEN`/`GDTEXTLEN` return the entry's string length **including**
  newline characters (measured: 79 and 380 for entries 108/109).

## Selecting the font

`+FONT ( n -- handle )` registers a font; `SDFNT ( handle -- )` selects
it; without `SDFNT` text renders in the system font `000.FNT`. The game
registers fonts 5, 6, 8 (`_F1`–`_F3`); the info line uses `_F1`
(18 px), the templates carry font 6 (20 px) — the pairing behind the
outline effect below.

## Metrics

Line width (routine `0x25df6`): per glyph `total += width + gap`, minus
one trailing `gap`. Block (routine `0x2619c`): `height = lines ×
font height + (lines−1) × line gap`, width = maximum over the lines.
The two gaps are engine globals at `0xd66ee` (glyph) and `0xd66ec`
(line), both **1**, identical for all fonts — but the drawer *retargets
them per pass* from the text template (below). Lines split on `\n`.

**The stored size is the measured size plus 4.** `SDTDT` (and every
setter that re-measures) writes width+4 and height+4 into the
descriptor's stored-size fields — and all placement modes and geometry
getters resolve against this padded size, so `GDWIDTH` answers 43 for a
39-pixel line (measured: 43/91/191 for table 6 entries 1/2/3). Sprites
have no such padding. The glyph pass itself centers with the *fresh*
measured size.

## Text templates — DEFTDT fully decoded

`DEFTDT ( a b c d e f g font n -- )` fills template `n` (1–9) of an
18-byte-per-entry table at `0xF2594`. The game defines all nine
templates identically as `8 2 2 -1 -1 -1 -1 <font 6> n DEFTDT`, and
every value now has a meaning:

| Template field | From the game's tuple | Meaning |
|---|---|---|
| +0x00 | font handle (font 6) | The **outline typeface** |
| +0x04 / +0x06 | −1 / −1 | X/Y offset for the backing box (unset) |
| +0x08 / +0x0A | 2 / 2 | Extra width/height margins of the backing box |
| +0x0C | −1 | Glyph gap for the outline pass (negative: overlap) |
| +0x0E | −1 | Line gap for the outline pass |
| +0x10 | 8 | The **outline color** |

`SDTDT n` selects a template for the current descriptor and re-measures.

## The pen color, and what "no color" means

Two different fields feed the two passes, and neither substitutes for the
other:

- **glyph pass** — the descriptor's own color, `SDCOL`'s field +0x0C of
  the text record, masked to a byte (`and $0xFF` at `0x6a21c`/`0x6a289`).
- **outline pass** — the *template's* +0x10, zero-extended
  (`mov 0x10(%eax),%ax; xor %ah,%ah` at `0x6a12c`/`0x6a195`), which is the
  `8` every one of the game's nine templates carries.

The byte then goes straight into the picture: the 1-bit glyph blitter
(`0x17d49`, 8-bit path at `0x17f19`) stores it with no clamp, no range
check and **no transparency test** — index 0 is transparent for sprites,
not for text.

**A text nobody gave a color draws in index 0.** The color field is
zeroed when the text record is allocated and never written by the
constructor (see [Descriptors](descriptors.md)); the original has no
sentinel for "unset" and no default of its own. Index 0 is black in 54
of the 60 shipped palettes. This matters because whole parts of the game
run on it: `SHOW_DOC`'s twenty help pages and module 13's info book set
no color at all.

The outline pass is skipped entirely when the template index is 0
(`cmpl $0,8(%eax); jle` at `0x6a0c6`) — and those same descriptors never
call `SDTDT`, so the help pages have no outline either. Plain black
letters.

## The outline is a second typeface

The drawer renders every templated text **twice at the same position**:

1. First pass: the **template's font** (font 6, 20 px), with the
   template's glyph gap (−1 — the wider outline glyphs overlap to stay
   under the text) and the template's color (8).
2. Second pass: the descriptor's own font (18 px), gap restored to 1,
   color = the low byte of `SDCOL`.

Each pass centers with **its own font's height**, so the two-pixel
height difference lands one pixel above and one below — the outline
rings the text evenly. No glyph offsets, no color tricks: the outline
face is simply a heavier font.

## Composite colors — SDCOL ≥ 256 adds a translucent backing

The drawer tests the color against 256 (`cmpl $0x100` at `0x69f50`):

- **< 256**: a plain palette index; no backing.
- **≥ 256**: the low byte is the text color, and a **backing box** is
  drawn behind the text first — not filled, but **remapped**: every
  pixel under the box is replaced by `table[row][old]`, a palette-to-
  palette lookup (the mechanism translucency uses in a 256-color
  world). The value minus 256 selects the row; the drawer passes 0x100,
  i.e. row 0.
- Row 0 is built at startup (`0x14740`) by taking every palette entry,
  subtracting **20 from each 6-bit channel** (clamped at 0 — about a
  third of full scale), and finding the nearest palette entry: a
  uniform darkening. The game's values: `SETT1` passes 18+256, the
  dialogue speaker table 165+256. `GDCOL` returns the composite value
  undivided (421, not 165).

**The backing box geometry**: corner = drawn corner − 5 (and −1 from
the template offsets), size = stored size + 10 (+2 template margins) —
net: six pixels out, sixteen larger, centered on the placement point.

**An empty string draws no backing**: the layout stores the string
length, and the drawer skips the backing block when it is below 1 —
which is what keeps the idle UI descriptors (parked on the empty entry)
invisible instead of painting empty darkened boxes. Only the backing is
gated; glyphs and stored size are unaffected.

## Centering

The glyph output (`0x257cf`) centers by itself, controlled by flags the
layout derives from the placement modes: horizontally it centers **each
line** — unless the descriptor's byte +0x17 bit 7 is set (only writer:
**`SDBLK`**), which centers the **block as a whole** on its widest
line. `SDCEN`/`SDVCEN` set the center placement modes (see
[Descriptors](descriptors.md)).

Nothing in the shipped game calls `SDBLK` — every text this game draws centers
line by line. (motionvm therefore leaves it inert; see
[Departures](../departures.md).)

### Vertical centering: the gap is included

The instruction reading and the measurement disagree here, and the
measurement is what the engine does.

**From the disassembly:** the centering height is `font height × lines`,
**without** line gaps. `0x2588e` computes `schrift[+2] × zeilen` and the
three instructions that follow it — `0x25897`, `0x258a1`, `0x258a6` — carry
no gap term at all. On that reading the centering height is a different
number from the measured block height.

**From a running original:** it is not. A caption whose block the gapless
arithmetic puts at y 167 stands at 166, which is what the gap-inclusive
height `(font height + gap) × lines − gap` gives. So the engine centers on
the gap-inclusive height, whatever the instructions appear to say.

What the disassembly reading is missing has not been established — the
likeliest candidate is that `schrift[+2]` is not the bare glyph height but
already carries the leading. [Open questions](../open-questions.md) carries
it, and [Departures](../departures.md) records that the rebuild follows the
measurement rather than the instruction reading.

## Captions in practice

- `SETT1`-style captions center on x = 320 and then clamp the **left**
  edge to `screen origin + 20` (there is no right-edge clamp — text may
  legitimately overhang on the right).
- The script word `TSC` clamps the current descriptor into the box
  10…620 × 1…400 and re-centers (`GDCX SDCEN GDCY SDVCEN` — an exact
  round trip because getters and placement share the stored size).
- Display time: `TS` = `TSX SDWAIT` — a piecewise function of
  `GDTXTLEN` (<20 → 20; ≤50 → length; ≤100 → max(len/2, 35); else
  max(len/3, 60)), scaled by `_TSPEED`% (250 at runtime). At 25 fps a
  short line stands two seconds.
- Self-hiding runs through the descriptor's **callback**: the frame
  walk counts the wait down and then executes the word set by `SDWORD`
  (or `NEWSETDESC`'s sixth argument). The info line uses `FINISHTEXT`
  (= `FT1`) every frame: first switch the descriptor to the blank entry
  (table 2, entry 1), next time deactivate it; the `_TI1` caption uses
  `FOLLOWMAN` (plain `SDINACTIVE`). `?READYT1`/`?TEXTREADY` simply ask
  whether that self-deactivation has happened.

## Open questions

- The layout fields +0x24/+0x2C (an origin shift applied by the glyph
  passes) — not yet read.
- The exact glyph-advance mechanism inside the output routine (the
  resulting metric is established).

## See also

- [Descriptors](descriptors.md) — placement modes, stored size, waits
- [Fonts](../formats/fonts.md), [Text tables](../formats/text-tables.md)
- [Dialogue machine](dialogue-machine.md) — who shows most of this text
- [Blocks](../formats/block.md) — the translucency tables
- [Departures](../departures.md) — vertical centering and `SDBLK`
