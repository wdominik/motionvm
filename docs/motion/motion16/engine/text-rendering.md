[← Documentation index](../../README.md)

# Text Rendering

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere, in `STERN.EXE` with Falsches Spiel mit Eddie M. and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

The 16-bit text drawer is one branch of the descriptor drawer (`016a:0aac`)
feeding a run drawer in the blitter segment (`14ee:11cf`), with the
measuring next to it (`14ee:16fd` for a block, `14ee:143e` for a line,
`14ee:14b9` counting lines) and a one-glyph primitive under all of it
(`14ee:14df`, into the 1-bit blits at `17f2:0999` and `17f2:0a2d`). What
follows is read from those routines at the instruction level.

## What the descriptor carries

The text branch runs on two fields of the descriptor, and both are shared
with the picture branch. **+0x10** is what the descriptor shows — for a text,
the id of the **table** to read from, resolved by `0362:0023` (-1 means the
table is not there). **+0x12** is the text word: its **low byte the string
number** within that table (`GDTXT` masks it with `0xFF` at `05f1:0cec`, and
`GDTXTLEN` hands the same byte to `016a:1E71` and takes `strlen` of what comes
back), bit `0x8000` set by `SDTXT`, and the layout in between — `0x4000`
centers on x (`SDCEN`), `0x1000` on y (`SDVCEN`), `0x2000` justifies
(`SDBLK`), `0x0800` boxes. `SDNORM` clears the layout with `and 0x80FF`,
keeping the string number and bit 15.

What makes a descriptor a text is **`+0x12 != 0`**, tested first and in that
form at all three places that ask — the drawer `016a:0b09`, the resolver
`016a:1ef6` and the save-under check `0362:10d9`. Bit `0x8000` is set but
never read: its only job is to make that test true when the string number and
the layout bits are all zero. A descriptor that is not a text is a sprite when
`+0x10` has bit 15 and a block otherwise (`016a:0fe8`).

**+0x2A** is the color's low byte, **+0x2B** the face — an index into the
font-handle table at `DS:0x45C` — and **+0x2C** the template. After drawing,
the box goes to **+8**/**+0xA**.

## Measuring

A glyph is found through a character remap the pointer at `ds:0x18D8`
names — the [font reference table](../formats/fonts.md), which `SFT n`
installs: the handler (`05f1:1813`) spells `n` into the file template
`#F0R3i.frt`, fetches that item through the container hook (`0362:0bf9`),
and stores the pointer (`14ee:10ae`), freeing the table before it. The
games pass 0 and nothing else. A line
(`14ee:143e`) is the sum of glyph widths plus a **glyph gap** for each,
minus one trailing gap; a block (`14ee:16fd`) is the widest line by
`lines × height + (lines − 1) × line gap`. Both gaps are globals —
`ds:0x18DC` and `ds:0x18DE`, 1 and 1 at rest — and the drawer retargets
them per pass, exactly as the 32-bit engine does with its pair.

`GDWIDTH` and `GDHEIGHT` (`05f1:1705`, `05f1:177b`) measure live, with the
descriptor's own face and the resting gaps, and answer the bare size —
where the 32-bit engine stores the measured size **plus 4** and its
getters read that back. The size the 16-bit drawer stores at +8/+0xA is a
third number again: the measured block grown by the template's margins.

## The two passes

A template (`SDTDT`, table at `ds:0x1A0C`, ten bytes an entry) makes the
drawer lay the text down twice: first the shadow — the template's own
font, gaps, and color — then the face over it, with the gaps put back to
1. `DEFTDT` (`05f1:2ee7`) fills an entry from nine stack values, popped
as: template id; the shadow **font** handle (a word, at +0); **glyph
gap** (+6) and **line gap** (+7); **x** and **y offsets** (+2, +3);
**width** and **height margins** (+4, +5); the shadow **color** (+8) —
all bytes, signed. The game's `XDEFTDT` gives every template
`_SHFONT @ -1 -1 -1 -1 1 1` and a color per template id, so the shadow
font is [font 2](../formats/fonts.md), the silhouette face whose glyphs
measure exactly two wider and two taller than the text face's.

Two gates on that table:

- `DEFTDT` **overwrites** its entry — the table is a fixed array indexed
  by the id. The game leans on it: the intro loads its own shadow font
  and defines templates 6 and 2 over it (module 610), frees that font on
  its way out (`_SHFONT @ -FONT`), and `RUN` defines all nine templates
  afresh over a new handle right after (module 100). The later
  definition replaces the intro's, so no template ever names the freed
  font.
- `SDTDT` (`05f1:0c78`) takes only 1..=20: `cmp $1` / `jl` and
  `cmp $0x14` / `jg` skip the store, so an id outside the table —
  `0 SDTDT` included — leaves the descriptor's template standing.
  `SAYDAVID` runs on that: it hands `_SxTDT @` to `SDTDT`, and `_SxTDT`
  is 0 until the first `SETSAY`.

Where a pass starts depends on the axis (`016a:0dc4`–`016a:0e60`):

- **Centered** (`SDCEN`/`SDVCEN`): the run drawer re-centers per pass —
  and per line on x — with its own font and the gaps set for it
  (`14ee:1231`, `14ee:1262`). The silhouette line measures two wider at
  gap −1, so it lands one pixel up and left of the face: a one-pixel ring.
- **Not centered**: the shadow pass starts at the anchor **plus the
  template's x/y offsets** (`016a:0e04`, `016a:0e2b`) — −1, −1 in every
  shipped template — and the face at the anchor itself. The same ring, by
  addition instead of arithmetic. The save-under behind the text moves by
  the same offsets (`016a:0cba`).

The 32-bit drawer differs here: it adds no offsets — both of its passes
compute the same point (`ENGINE.EXE` `0x6a136`, `0x6a228`) — and its
per-pass centring alone makes the ring.

## Justification, and `#`

With the 0x2000 bit set the run drawer works in block mode: every line
starts at the block's left edge — the widest line's, per pass — and a
helper (`14ee:111c`) spreads the line's deficit against the block width
one pixel at a time over its inner spaces, round robin from the left.
Leading spaces and a space at the line's end carry nothing, and a line
that opens with `#` stays ragged. The newspaper (module 615) is the one
caller: its texts mark headings and paragraph ends with `#`.

`#` itself is eaten by the run drawer — no glyph, no advance
(`14ee:135e`) — though the measure counts it like any character.

## Open questions

- Bit 0x800 of the text word: the drawer grows the box by two and skips
  the glyph passes entirely when no template is set. The only handler that
  sets it (`05f1:16e3`) is bound to no entry of either build's kernel table,
  so no script can reach it — what it was for is unread.
- The stored box at +8/+0xA feeds the drawer's own save-under; whether
  anything else reads it is unchecked.

## See also

- [Fonts](../formats/fonts.md) — the three faces and the reference table
- [Descriptors and screens](descriptors.md) — the fields around the text
- [Boot and frame loop](game-loop.md) — `XDEFTDT` at start-up
- [Text rendering (MOTION 32-bit)](../../motion32/engine/text-rendering.md) — the successor's drawer, read the same way
