[← Documentation index](../../README.md)

# Descriptors and Screens

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein and in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, which is an older build of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names the other game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit-ds2).*

The 16-bit kernel has the same display vocabulary as the 32-bit one —
screens with a size, a position and a viewport; descriptors that show a
sprite, a block or a text; `SD*` setters and `GD*` getters; three-argument
fades — and the scripts use it the same way. What this page records is the
**stack effects measured from ENVIRO's call sites**, the places where the
16-bit usage differs from the 32-bit game's, and what is still unread. The
C structures behind the handles are unread; the 32-bit engine's layouts
([descriptors](../../motion32/engine/descriptors.md),
[screens](../../motion32/engine/screens.md)) are the working hypothesis for
what a 16-bit descriptor holds, not a measurement.

## Building a descriptor

`NEWSETDESC` takes **six** arguments, as in the 32-bit engine, with a word id
where that engine takes an address:

```
x y level gfx arg5 callback   NEWSETDESC   ( -- handle )
```

The script helpers in module 600 fix the last two:

| Helper | Stack | Compiles to |
|---|---|---|
| `XYLSITEM.` | `( x y lev gfx-id -- handle )` | `gfx-id 15 -1 NEWSETDESC`, then `SDSPR` with the same id — a sprite descriptor |
| `XYLBITEM.` | `( x y lev gfx-id -- handle )` | `15 -1 NEWSETDESC` with the id already in place — a background |
| `NEWTEXT.` | `( x y lev -- handle )` | `0 15 -1 NEWSETDESC`, then `SDTB` — a text descriptor |

So `arg5` is always 15 in this game and the callback is −1 (none). Where a
callback *is* wanted the scripts set it afterwards with `SDWORD ( word-id
-- )`: the intro text gets `1082 SDWORD`. A callback is a **global word
id**, never a packed address ([execution model](../vm/execution-model.md)).

The frame loop's callback walk (`016a:05d6`–`06da`) runs a descriptor's
word only when `cb != -1` **and the active bit `+0x28` bit 7 is set**
and `wait != -1`; it makes the descriptor current first (the screen into
`DS:0x5de2`, the number into `DS:0x3058`). The active gate carries the
selection semantics of the whole intro: a skipped callback leaves the
controller's own `SMDESC` choice standing, and `ICTRL`'s motif phases
write `SDH%SHR`/`SDSPR`/`SDCX` frame after frame without re-selecting —
`_DINFO` holds `1082 SDWORD` from birth and stays `SDINACTIVE` while
they run.

`SMDESC` (module 603) is `_MS @ ACTSCR @ ACTDESC` — make the main screen
current, then the descriptor whose handle is on the stack; `SSACT` is
`@ ACTSCR`.

## Screens and graphics mode

| Word | Effect | Evidence |
|---|---|---|
| `TOGFX` / `GFXTO` | `( -- )` enter / leave 320×200×256 | first and last thing `RUN` does; no resolution word exists in this kernel |
| `SETPAL` | `( id -- )` | `0 SETPAL`, `22 SETPAL` |
| `NEWSCREEN` | `( -- handle )` | stored in `_MS` |
| `SCRSIZE` | `( w h -- )` | `960 544` |
| `SCRPOS` | `( x y -- )` | `0 0` |
| `SCRVSIZE`, `SCRFVSIZE` | `( w h -- )` — the viewport | `320 200` |
| `SCRVPOS` | `( x y -- )` | `0 0` |
| `SCRCTRL` | `( word-id -- )` — the per-frame handler | `400`, `1086` |
| `ACTSCR` | `( handle -- )` | `_MS @ ACTSCR` |
| `ERASESCR`, `REMSCR` | `( handle -- )` | intro teardown |
| `GSCRX`, `GSCRY` | `( -- x )`, `( -- y )` | mouse-to-world in `CTRL` |
| `GSCRACT` | `( -- flag )` | `CTRL` |
| `->SCRX`, `->SCRY` | scroll the viewport | one site each; arity by analogy with the 32-bit `SCRX`, unmeasured |
| `FADEIN`, `FADEOUT` | `( mode duration step -- )` | always `1 50 8`; read — see below |
| `FREEZESCR`, `UNFREEZESCR` | as in the 32-bit engine | used |
| `XGFXSTAT` | `( a b c -- )` | `420 2499 -1 XGFXSTAT` on leaving a location |
| `XGFXVFLIP` | `( src dst count -- )` | `2483 2464 1 XGFXVFLIP`; `05f1:221b` installs mirror aliases, no pixels move until the loader resolves them |

A 960×544 screen behind a 320×200 viewport means the picture scrolls; the
per-location tables carry world coordinates up to x = 627
([blocks](../formats/blocks.md)).

## Sprites, text, fonts

Same vocabulary as the 32-bit engine: `SDSPR ( sprite-id -- )` (358
sites), `SDX`/`SDY`, `SDLEV`, `SDACTIVE`/`SDINACTIVE`, `SDWAIT` (298 sites),
`SDWORD`, `KILLNDESC ( n -- )`, `ACTDESC`, `GDSPR`, `GDX`/`GDY`, `SDCX`/`SDCY`,
`SDOX`/`SDOY`, the scale words `SD%SHR`/`SDH%SHR`/`SDV%SHR`.

Text: `SDTB ( table -- )`, `SDTXT ( n -- )` 1-based, `SDFNT ( handle -- )`
with a handle from `+FONT`, `SDCOL ( color -- )` (18 in the intro), `SDCEN
( x -- )`, `SDVCEN ( y -- )`, `SDTDT ( template -- )` with templates defined
by `XDEFTDT` (module 603) over the kernel's `DEFTDT`.

The typical sprite idiom:

```
x y lev sprite-id  XYLSITEM.   _MOT1 !   SDINACTIVE   2 SDBUF
```

## Mouse and keys

`MOUSEX`, `MOUSEY`, `MOUSELK`, `MOUSERK`, `?KEY` (0 when no key), `ATMOUSE`,
`XATMOUSE`, `SHOWMOUSE`, `HIDEMOUSE`, `MOUSEINFO`, `?XINSIDE`. `FATMOUSE` and
`FXATMOUSE` are script words (module 603) that store the sprite id in
`_BMNR` and call `ATMOUSE`/`XATMOUSE`. `KEY` blocks for a key; the game
uses it on its debug path only.

## The structures, as far as read

Reading the handlers that drive the intro and the first locations settled
this much of what the handles stand for (addresses in `ENVIRO.EXE`):

- **Screens** are two blocks of 0x13ae bytes at `DS:0x305a`, the active one
  named by `DS:0x5de2`: `+0/+2` the window's origin in surface
  coordinates — `SCRPOS ( x y -- )` writes both (file `0x979b`), **`SCRX`
  is `SCRPOS` with the y kept** (file `0x9809`) and `GSCRX` reads the x back
  (file `0x97da`), which is how a wide location scrolls (`34 SCRX`,
  `152 SCRX` in the macros) and how the scripts turn the mouse into world
  coordinates (`GSCRX _MX @ +`). The native placements that mirror a
  `GSCRX`-relative script (the verb strip's clamp, `MOUSEINFO`, the
  conversation layout) answer the same sum the damage map bases on —
  the 32-bit origin plus this scroll; each generation leaves the other
  register at zero. Both words set bit 0 of the flags at `+0x18`,
  which the frame loop answers by clearing the surface and drawing every
  active descriptor again; `+8/+0xa` the view size; `+0x10/+0x12` the
  view's place on the display; `+0x14` the controller word id; `+0x16` the
  descriptor count; `+0x1a` the head of the draw list; `+0x20` the surface
  image; `+0x24` a hundred descriptors of 0x2e bytes; `+0x121c` fifty dirty
  rectangles. **`->SCRX ( screen x step -- )`** (file `0xc149`) scrolls a
  screen's origin to `x` in steps of `step`, one a tick, blitting each — a
  blocking scroll; `->SCRY` the same for y. The 32-bit `SCRX` writes a
  different pair, hanging off the screen.
- **Descriptors are numbered per screen.** `NEWDESC` and `NEWSETDESC`
  (file `0x9bb8`, `0x9bd8`) hand back the active screen's count and raise
  it (a hundred at most); `ACTDESC` (file `0x9b92`) stores the number and
  nothing else — the pair screen and number is resolved when a word
  touches the descriptor, so `ACTDESC` before `ACTSCR` means the same as
  after; `KILLNDESC n` (file `0x9d3a`) frees the active screen's
  descriptors from `n` up and sets its count back to `n`, which is how
  `INCLLOC`'s `?LPD 1 + KILLNDESC` clears a location's descriptors above
  the permanent ones. The same number names one descriptor on each screen.
- **Descriptors** are 46 bytes: `+0` x, `+2` y, `+8/+0xa` size, `+0x10`
  the picture — `0x8000 | id` for a sprite, the bare id for a block, which
  is how the drawer (`016a:0aac`) tells them apart: a sprite goes through
  the keyed blit at `14ee:0d1e`, a block through the plain copy at
  `14ee:0d47`, every pixel, index 0 included (the location backgrounds are
  80-pixel block strips, dark where they hold 0) — `+0x12` flags (`0x4000` marks a text), `+0x16` buffer number + 1,
  `+0x20` callback word id (`SDWORD`), `+0x22` wait countdown (`SDWAIT`),
  `+0x26` next in the draw list, `+0x28` flag byte with `0x80` active and
  `0x40` dirty — every `SD*` word sets it — and `+0x2b`/`+0x2c` the buffer
  and font indices `SDX` consults when it re-centers a text. The rest of
  the record is unread.

## Walking, inventory, orders, the pointer

The handlers behind these names are **read and the same code as the
32-bit engine's**, compiled for 2-byte cells — the same records with every
field at half the offset, the same command numbers, the same state
machine — with the differences listed:

| Word | Read at | Same as the 32-bit handler | Differs |
|---|---|---|---|
| `DOWALK ( person -- )`, `CROUTE` | file `0xf25f`, `0xe6d0` | the person record (`+0xc8` descriptor, `+0xd0` step index, `+0xd4` walking, `+0xe0` queue, `+0xe6` command …), the commands 1001–1003, 1006, 1007, 999, the turn table, the step buffer, the route search, the facing rule; a 12-byte step, an 18-byte route after a 2-byte count, a 12-byte extra record | arithmetic in 16 bits — a product before a division in the line test and the size ramp can wrap where the 32-bit code's cannot; the `1006` size is the shadow's as it stands (both binaries, in fact) |
| `?XINSIDE ( x y table stride count -- i \| -1 )` | file `0xf09b` | inclusive corners, the all-zero record a hole | — |
| `CCALCINV`, `ADDTOINV`, `SUBFROMINV`, `?INVINCL` | file `0x13a5d`, `0x13cb4`, `0x13d98`, `0x13d20` | the list (scroll offset at +0, slots from byte 4, 99 of them, zero-terminated), the 5-cell item record, eight slots in the bar, the same call `CALCINV` makes | `ADDTOINV` scrolls from the tenth slot on and by one less; `CCALCINV` clears a slot with `SDINACTIVE` where the 32-bit one gives it sprite 0 |
| `DOORDER ( _ORDER -- )`, `EXECORDER`, the menu routines | file `0x1255f`, `0d34:164e`, `0d34:0006`–`0d34:0b9b` | the 260-byte `_ORDER` block (every field at half the 32-bit offset, callbacks as word ids), modes 0–9 and 97–99, the click path, the verb strip and its pulse, verbs 1–4 and 8, `hit_area` biased by 1000, `flash_entry` | the bar: slots of 32 from x 0x20, the strip 0x14 down and 0x14 apart, the pointer at (8, 8), a held item at (0x10, 0xa); every wait is on the figure's command cell being 0 or 999 |
| `MOUSEINFO ( 24 values -- i \| -1 )` | file `0x10015` | the three cases — scene, bar, neither — the caption at the mouse, the pointer through `FXATMOUSE`, the "mode changed" global | the bar's slots of 32 from 0x20; the item's name centered 158 below the origin |
| `ADDMESSPIPE ( name1 name2 kind a b _ORDER -- )` | file `0x13e41` | two 9-byte names, then kind and two values | a 26-byte record (0x22 on the 32-bit engine); the queue's room is never checked — `_MESSPIPE` holds five records while the block says 30 |
| `GFXSTAT`/`XGFXSTAT`/`GFXSTAT+`/`XGFXSTAT+`, `TXTSTAT`/`XTXTSTAT` | file `0xb0e2`–`0xb2cf` | status tables for the kernel's own loader — a cell per sprite (`DS:0x765e`) and per text (`DS:0x0476`); the `+` forms load at once; `XGFXSTAT … -1` frees a range | inert where resources are loaded on demand |
| `SD%SHR ( p -- )` | file `0xb38b` | `SDH%SHR` and `SDV%SHR` with the same value — as the 32-bit handler writes both fields | — |

`DELAY`, `RANDOM` (188 sites) and `STEPMULTI` (the walk's step size,
`DS:0x0ff6`) are the same words as well. The script side — `XYWALK`,
`PSETWALK` in module 604; the verb table in 603 — calls them the same way
the 32-bit game does.

**The conversation machine**, read at `calc_dialog` (`0d34:0ed9`), the
change drain (`0d34:0d42`), the finish (`0d34:0d04`), the `DOORDER` modes
12–18 (`0d34:2d16`–`0d34:33c0`), verb 5 (`0d34:1b2a`) and verbs 6 and 7
(`0d34:1d13`, `0d34:1e9c`): the same design as the 32-bit engine's — a
record with a table of answers, a table of lines and a graph of branch
nodes, driven by the mode cell and a node number whose range says what it
is (under 1000 a line, to 2000 an answer, to 3000 a branch, 4000 "back to
the last answer", −1 over) — on an older layout and a smaller screen:

| | MOTION 16-bit (`ENVIRO.EXE`) | MOTION 32-bit |
|---|---|---|
| Record | `+2` entry node, `+4` answers, `+6` lines, `+0xa`/`+0xc` the two heads' sprites, `+0x10` quiet node, `+0x12` name, `+0x1c` a permission cell per answer | `+4`, `+8`, `+0xc`, —, `+0x20`, `+0x24`, `+0x30` four bytes each |
| Answer | 14 bytes: `+0` node, `+2` next answer, `+4` name | 18 bytes: `+0`, `+4`, `+8` |
| Line | 8 bytes: `+0` text, `+2` table, `+4` next, `+6` flags, bit 0 = the right speaker | 16 bytes, `+0xc` a speaker index |
| Branch | 26 bytes: `+0` name, `+9` answer name, `+0x12` kind, `+0x14` target, `+0x18` next; kinds 1–4 as the queue's, 5 runs a word by name and drains the queue; named, it is queued with no room check | 34 bytes, `+0x1e` next; kinds 5 and 6 |
| Speakers | two words in the block (`+0xdc`, `+0xde`), run with a phase — 0 start, 1 line over, 2 talking, 3 end, 4 listening — and two head descriptors (`+0xb6`, `+0xb8`) with the record's sprites at the view's left and right, bottom 165 | a table of ten 0x28-byte speaker entries |
| A line | centered at (160, 80) of the view, level 0x73, in the speaker's color and template (`+0xc2`/`+0xe8` left, `+0xc6`/`+0xea` right), no clamp; mode 12 left, 13 right | the speaker's place, clamped to the view |
| The answers | up to three from the chain, stacked downward from 20 below the view's top at x 70, each the next's height plus 7 lower; the quiet line at 130; all in the answers' color `+0xc4`; the answer under the pointer takes `+0xc2` | centered at x 320, stacked upward from 360 |
| Verb 5 | the talk word answers the record and its field table (or −1); the queued changes are applied; both speakers hear 0 | the same |
| Verbs 6 and 7 | two passes over the answer names — the word's keyword, then the field's | three, with a literal between |

([Dialogue machine (MOTION 32-bit)](../../motion32/engine/dialogue-machine.md)
has the 32-bit side in full.)

**Scripts that wait in a loop of their own.** `RUN`'s start-up page and
location 5's newspaper (module 615, `DOZEITUNG`) poll the pointer and the
key buffer in `BEGIN … UNTIL` loops inside one word, outside `ANIMPLAY`
or inside one of `CTRL`'s frames; the original's input words read live
hardware and the loop turns as the player moves. How the rebuild keeps
such a loop turning is a [departure](../../departures.md#the-16-bit-machine).

## Setters, and what they mark

Every `SD*` handler read so far writes its field and sets the dirty bit
(`+0x28 |= 0x40`) **whatever the value** — `SDX` (`05f1:0df2`), `SDFNT`
(`05f1:1038`), `SDLEV`, `SDNORM` — where the read 32-bit setters skip an
unchanged one. That is what the intro's `.DRAWNEW` (`SMDESC GDX SDX`, a
coordinate written over itself) is for: the mark alone. `SDX` also keeps
the two positions consistent: it stores the left edge at `+0` and, on a
centered descriptor, measures the text with its template's margins and
rewrites the center anchor at `+4` (`05f1:0ef1`; the boxed bit `0x800`
adds one). `KILLNDESC` erases no pixels — a killed descriptor's last
picture stands on the surface until a rebuild clears it, which is why
`INCLLOC` runs `0 0 SCRPOS` on the scene screen.

## The draw chain

The screen's descriptors hang on a doubly linked chain (head at screen
`+0x1A`, tail `+0x1C`, next/prev at `+0x26`/`+0x24` of the record) sorted
ascending by level, and the drawer walks it head first
(`016a:0a4f`–`016a:0aa0`), so the chain order **is** the paint order. The
one insert routine (`0362:2007`) walks to the first node whose level is
*greater* and splices in before it — the incoming descriptor lands
**after every equal** and draws on top of them. Its callers are the three
places the chain ever changes: `NEWDESC` (creation, `05f1:0c1b`), and
`SDLEV` with its alias (`05f1:1054`, `05f1:10c0`), each unlink
(`0362:2111`) and re-insert — **even when the level is unchanged**. A
walking figure asserts its level every step, which is what keeps it in
front of scenery it shares a level with; the figure's level itself comes
from the route it stands on ([blocks](../formats/blocks.md), the
`_XROUTE` z field).

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

- The rest of the 46-byte descriptor and of the screen block; the meaning
  of `arg5` (15); the `XDEFTDT` template numbers; the mirror axis of
  `XGFXVFLIP` (left-right — a flip about the vertical axis — is what a
  walk cycle and the card flip need; unmeasured against a capture).
- `SDBLK` (file `0xa625`) sets bit `0x2000` of the descriptor's flags —
  one of the three layout bits the drawer folds for its text call
  (`0x4000` centers on x, `0x1000` on y) — and the drawer reads it as
  justification: the newspaper's article texts are set in a block
  ([text rendering](text-rendering.md)).
- The other handlers with a 32-bit namesake, at the instruction level.

## See also

- [Boot and frame loop](boot-and-loop.md)
- [Off-screen buffers](buffers.md)
- [Fonts](../formats/fonts.md), [Text tables](../formats/text-tables.md)
- [Descriptors (MOTION 32-bit)](../../motion32/engine/descriptors.md) — the 32-bit structures
