[← Documentation index](../../README.md)

# Screens and the Draw Chain

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere, in `STERN.EXE` with Falsches Spiel mit Eddie M. and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

A screen is a surface with a size, a position and a viewport, and the display
is every screen composited in level order. The 16-bit kernel's screen
vocabulary is the 32-bit one's, and the scripts use it the same way; what this
page records is the stack effects measured from the call sites in Die
Enviro-Kids greifen ein, and how the drawer gets from a descriptor to pixels.

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

**Scaling is stored in pixels, not per mille.** `SDH%SHR` (`05f1:23f2`)
and `SDV%SHR` (`05f1:246f`) take the per-mille argument the scripts
write, look up the current sprite's width or height in the size tables
at `DS:0x764a`/`DS:0x7662` by the id in `+0x10`, and store
`natural · v / 1000` — a signed long division, truncating — into
`+0xC`/`+0xE` of the record; `1000` and `−1` are both stored as `−1`,
natural size. `SD%SHR` (`05f1:227b`) is the two of them with one value
and has no field of its own. `SDSPR` (`05f1:129a`) writes the sprite and
refreshes the natural pair `+8`/`+0xA` from the same tables, and leaves
`+0xC`/`+0xE` alone — so a sprite swapped under a stored size keeps the
size in pixels, and the intro's card flip, which writes `SDH%SHR` before
the frame's `SDSPR`, resolves each width against the motif of the frame
before. The drawer (`016a:100d`) draws a descriptor with both fields `−1`
at natural size; otherwise it takes each field where it is not `−1` and
the natural size where it is, draws nothing at all when either comes to
0 (`016a:119b`), and scales through a scratch image the size of the
target: the scaler (`18f3:0699`) builds a column table from
`x · natural / target` and picks each source row by `y · natural /
target`, truncating, and the scratch is then blitted keyed. motionvm
stores and draws exactly this, behind the engine profile's `scale_in_pixels`; the 32-bit
engine keeps per-mille factors and multiplies at paint time, and stays
modelled that way.

## Open questions

- The rest of the screen block, past the fields the setters are known to
  reach.

## See also

- [Descriptors](descriptors.md) — what is drawn onto a screen
- [Transitions](transitions.md) — fading a screen in and out
- [Off-screen buffers](buffers.md) — `BUFON`, `SETBUF`, `SDBUF`, `KILLNBUF`
- [Boot and frame loop](game-loop.md) — when a frame is drawn
- [Screens (MOTION 32-bit)](../../motion32/engine/screens.md) — the other generation's, in more detail
