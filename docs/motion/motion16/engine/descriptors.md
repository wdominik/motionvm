[← Documentation index](../../README.md)

# Descriptors

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

The 16-bit kernel has the same descriptor vocabulary as the 32-bit one —
descriptors that show a sprite, a block or a text; `SD*` setters and `GD*`
getters — and the scripts use it the same way. What this page records is the
**stack effects measured from the call sites in Die Enviro-Kids greifen ein**,
the places where the 16-bit usage differs from the 32-bit game's, and what is
still unread. The C structures behind the handles are unread; the 32-bit
engine's layouts ([descriptors](../../motion32/engine/descriptors.md),
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
  it — a hundred at most: at a hundred, `NEWSETDESC` (`05f1:0ad4`; the same
  test in `HPPLAY.EXE` at file `0x9b2e` and `BMZ.EXE` at `0x9bd5`) jumps past
  every pop and the push, so its six arguments stay on the stack and no
  handle comes back, where `LL.EXE` (file `0x44ce`) raises the count without
  looking; `ACTDESC` (file `0x9b92`) stores the number and
  nothing else — the pair screen and number is resolved when a word
  touches the descriptor, so `ACTDESC` before `ACTSCR` means the same as
  after; `KILLNDESC n` (file `0x9d3a`) frees the active screen's
  descriptors from `n` up and sets its count back to `n`, which is how
  `INCLLOC`'s `?LPD 1 + KILLNDESC` clears a location's descriptors above
  the permanent ones. The same number names one descriptor on each screen.
- **Descriptors** are 46 bytes in `ENVIRO.EXE` and 45 in `BMZ.EXE`, whose
  record ends at `+0x2c` (`imul $0x2d`); the extra byte is padding nothing
  reads. `+0` x, `+2` y, `+4`/`+6` the centering anchors, `+8`/`+0xa` the
  last box drawn, `+0xc`/`+0xe` the scaled target size or −1, `+0x10`
  **what the descriptor shows**, `+0x12` **the text word**, `+0x16` buffer
  number + 1, `+0x18`–`+0x1e` a partial-redraw rectangle, `+0x20` callback
  word id (`SDWORD`), `+0x22` wait countdown (`SDWAIT`), `+0x24`/`+0x26`
  previous and next in the draw list, `+0x28` flag byte with `0x80` active
  and `0x40` dirty — every `SD*` word sets it — `+0x29` level, `+0x2a` the
  color's low byte, `+0x2b` the face font index and `+0x2c` the template.
- **`+0x10` and `+0x12` are one pair, and between them they say what a
  descriptor is.** `+0x10` holds `0x8000 | id` for a sprite and the bare id
  for a block — or, when the descriptor is a text, the id of its text table:
  one word over two id spaces, written by `SDSPR` (`05f1:12b6`), `SDBL`
  (`05f1:11ee`) and `SDTB` (`05f1:0d7b`) alike. `+0x12` is the text word,
  and **any non-zero value in it means text**. The drawer asks in that order
  (`016a:0aac`, and the same test again in the resolver `016a:1eea` and the
  save-under check `0362:10d9`): text first, then bit 15 for a sprite —
  which goes through the keyed blit at `14ee:0d1e` — else a block through
  the plain copy at `14ee:0d47`, every pixel, index 0 included (the location
  backgrounds are 80-pixel block strips, dark where they hold 0).
  `NEWSETDESC` writes its graphics argument straight into `+0x10`
  (`05f1:0b51`) and zeroes `+0x12`, so there is no "nothing set" state:
  `0 15 -1 NEWSETDESC`, which is how the scripts make a text descriptor, is
  a block on graphic 0 until `SDTXT` runs.

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

## Open questions

- `SDBLK` (file `0xa625`) takes no argument and sets bit `0x2000` of the
  **text word** `+0x12` — one of the three layout bits the drawer folds for
  its text call (`0x4000` centers on x, `0x1000` on y), and one of the bits
  `SDNORM` takes away again with `and 0x80FF`. The drawer reads it as
  justification: the newspaper's article texts are set in a block
  ([text rendering](text-rendering.md)).
- `NEWSETDESC` writes `arg5` (always 15) to `+0x14`, and **no instruction in
  either binary reads that field**; `SDTXT` sets bit `0x20` of the flag byte
  and nothing tests it either.
- The other handlers with a 32-bit namesake, at the instruction level.

## See also

- [Screens and the draw chain](screens.md) — where a descriptor is drawn
- [Transitions](transitions.md) — fading what they make
- [Interaction](interaction.md) — the pointer, the walk and the inventory bar
- [Boot and frame loop](game-loop.md)
- [Off-screen buffers](buffers.md)
- [Fonts](../formats/fonts.md), [Text tables](../formats/text-tables.md)
- [Descriptors (MOTION 32-bit)](../../motion32/engine/descriptors.md) — the 32-bit structures
