[← Documentation index](../../README.md)

# LL.EXE — The Oldest Build of the MOTION 16-bit Player

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere, in `STERN.EXE` with Falsches Spiel mit Eddie M. and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

`LL.EXE` (123 222 bytes, 1993-05-20) is the same player as
[`ENVIRO.EXE`](enviro-exe.md), built three years earlier — the oldest MOTION
artifact there is. Everything the `ENVIRO.EXE` page says about the *shape* of
the binary holds here — Turbo C real mode, two kernel tables of far function
pointers, a data stack behind a far pointer in DGROUP — and this page is what
differs. More differs than for the other builds, and the first thing is
the one that makes every address on every other page useless here.

## The binary

| Field | `LL.EXE` | `ENVIRO.EXE` | `BMZ.EXE` | `HPPLAY.EXE` | `STERN.EXE` |
|---|---|---|---|---|---|
| Size | 123 222 | 167 430 | 166 806 | 165 702 | 167 334 |
| Header | **`0x1e00`**, load image 115 542 bytes | `0x3200`, 154 630 | `0x3200`, 154 006 | `0x3200`, 152 902 | `0x3200`, 154 534 |
| Relocations | 1853 | 3160 | 3162 | 3157 | 3123 |
| Entry | `CS:IP = 0000:0000`, `SS:SP = 1c27:00e6` | `SS:SP = 25b2:00e6` | `SS:SP = 258b:00e6` | `SS:SP = 2546:00e6` | `SS:SP = 25ac:00e6` |
| Data segment | `0x1271` | `0x1c24` | `0x1c03` | `0x1bce` | `0x1bcb` |
| Turbo C banner | file `0x14514` | file `0x1f444` | file `0x1f234` | file `0x1eee4` | file `0x1eeb4` |

**The load image starts at file `0x1e00`, not `0x3200`.** The other four
builds share a 800-paragraph header, and every `seg:off` this documentation
gives for them converts with the same constant. This one has 480 paragraphs,
so a far pointer here is `file = 0x1e00 + segment * 16 + offset`. Together
with a data segment 2483 paragraphs lower, that leaves nothing about an
address transferable: not the code, not the globals, not the arithmetic that
finds them.

Its prompts are the informal *Du* (`Lege bitte`, file `0x145dd`), as
`ENVIRO.EXE`'s are and unlike the two builds between them. It carries the
fifteen interpreter error messages (from file `0x15479`) that `ENVIRO.EXE`
strips and `BMZ.EXE` and `HPPLAY.EXE` keep.

## The kernel, and where the domain table starts

| Build | Domain words | Core words | Total | Domain base |
|---|---:|---:|---:|---:|
| `LL.EXE` | 124 | 80 | 204 | **102** |
| `STERN.EXE` | 146 | 80 | 226 | **102** |
| `HPPLAY.EXE` | 146 | 82 | 228 | 105 |
| `BMZ.EXE` | 150 | 82 | 232 | 105 |
| `ENVIRO.EXE` | 151 | 82 | 233 | 105 |

The word counts are the story the other build pages tell — a table grows by
appending, so fewer words means earlier — and this build is the shortest of
the five. Its domain table is `ENVIRO.EXE`'s less 27 words: the whole
inventory group, the walk and order words, the print group, the digital-sound
words and the `SETMOUSE*` four, none of which existed yet. Its core table is
`ENVIRO.EXE`'s less the two that build appends, `_PutStringAdr` and `$->`; the
first 80 are name for name and order for order the same.

**The base is the part that is not like the others.** A kernel cell is
`0x8000 | ordinal`, the core table is 1-based, and the domain table starts
wherever the player's own registration leaves off. The player hands ordinals
out in the order it registers words, and it registers three runs: the core
table (`0afe:000c`), then a run of `DUMMY#F0R3i` placeholders (`0afe:006b`,
bounded by `0afe:009a  cmp $0x15,%si` — 21 of them), then the domain table
(`013a:0011`). Each registration writes the running counter at `ds:8688` into
the word's header and steps it (`0af7:0167`, at `01ed` and `01f7`).

So the first domain word binds at `80 + 21 + 1 = 102` — here and in
`STERN.EXE`, whose core table is this one's eighty. The three later builds
run the same three loops with 82 core words and 22 placeholders —
`140e:002f` in `ENVIRO.EXE`, `140a:0039` in `BMZ.EXE`, `13d9:002f` in
`HPPLAY.EXE` — which is where their 105 comes from. The gap between the two
tables is not unused numbering; it is placeholder words.

This is the loudest case of the rule the other pages make quietly. A table
baked from any later build binds this game's bytecode without complaint and
names every word of it three places along: `SETSHADE` comes out as `NEWANIM`,
and `SDTXT` — which this game calls 149 times — is not reachable at all.
Nothing in the format announces it. The base is read out of the binary's own
registration loops, not assumed.

## What else this build does not do yet

Two behaviors the later builds have are absent, and both were found by a game
that reaches them.

**`?XINSIDE` has no hole case.** In `ENVIRO.EXE` (`0a40:1b37`) and `BMZ.EXE`
the four corner comparisons are followed by four more asking whether every
corner is zero, and an entry that is all zeroes is passed over — 101
instructions. Here, in `STERN.EXE` and in `HPPLAY.EXE`, the handler stops
after the comparisons: 71 instructions, no `cmpw $0` among them. So in this
game a hole in a hot-area table is a rectangle at the origin. The split
follows the order of the tables — the two later builds test, the three
earlier do not. The behavior is read out of the handler, not derived from the
game.

**`?INSIDE` exists and is called.** It is `?XINSIDE` over a single record
(`0104:2b0f`): three values popped, four inclusive comparisons against the
cells at `+0`/`+4` and `+2`/`+6`, one or zero pushed. Every build has the
word; only this game calls it.

## What lives where

| File offset | What |
|---|---|
| `0x146ca` | Kernel table 2 — 124 domain words, ordinals 102–225 ([kernel words](../vm/kernel-words.md)) |
| `0x14feb` | Kernel table 1 — 80 core words, ordinals 1–80, the later builds' first 80 name for name |
| `0x15457` | `DUMMY#F0R3i`, the placeholder name the 21-word run registers |
| `0x15479` | The fifteen interpreter error messages |
| `0x1462e` | `data.-1-` — the literal, where the later builds hold the `DATA.-#i-` name former and can open a second volume |
| `0x14e86` | `gfx.inf`, which this game ships and the later ones only name |
| `0x14ee9` | `psmcfg.dat` — 32 bytes here, against the later games' 36-byte `PSMCFG4.DAT`; the loader reads `0x20` bytes at `091e:0093` and branches on the words at `+0x1A`, `+0x16` and `+0x18` |

## What the game asks of it

Victor Loomes' bytecode uses 100 of the 124 domain words. Nine of them no
other game calls, and all nine are read from their handlers: `?INSIDE`,
`CROUTE` (`0104:4a45`, the five pointers on the stack where the later games
reach the same routine through `DOWALK` — and a routine of its own in two
places: it copies a zero shrink as it stands where `ENVIRO.EXE` takes it as
1000, and it closes with a pass over the finished headings, `0104:516d`,
that no other build has; both are read off the binary when the game opens,
see [the walk](interaction.md)), `GSCRPOS` (`0104:13bf`), `SETSHADE`
(`0104:2824` — two cells into two globals nothing in either binary reads
back), `SETCYCLE` (`0104:5319`, with the tick that turns the palette once a
frame at `0104:536d`), `SYSFC`/`SYSBC` (`0104:358e`/`0104:3597`, which only
the request box reads), `REQUEST` and `_POOR` (`0104:0002`, a constant
zero).
The core table adds one: `I'` (`0af7:095e`), which is `I` with the fetch at
`+2` instead of `+0`.

`REQUEST` is worth its own line too, because it is the one word in either
generation that stops the game to ask something. The handler (`0104:35a0`)
pops a box, a message and up to five captions and blocks in the drawer
(`0104:7332`) until a click, Enter or Escape answers it — the button's
**one-based** index, the default, or 0. The layout is arithmetic rather than
a picture: a button is `(w - 10 - (n - 1) * 4) / n` wide, the first starts 5
in and each next 4 past the last, and the row runs from `h - 18` to `h - 6`
with the hit test a pixel wider on each side. The strings are numbered from
one — the fetcher at `0104:80ac` admits an index the table's count is not
*less* than and reaches the first string at 1. This is the game's save and
load menu.

Its two texts carry no styling of their own. Both go to the ordinary run
drawer (`0d06:1107`), which takes a mode word and no spacing: the message with
mode **1** at `x = w/2`, `y = 5` (`0104:7407`) — bit 0, centered on x, `y` the
top — and each caption with mode **5** at the button's middle, `y = h - 12`
(`0104:74aa`) — bits 0 and 2, centered on both. The drawer spaces from the
globals rather than from its caller: it measures a line as the glyph widths
plus the **glyph gap** at `ds:0x13c4` each, less one trailing gap
(`0d06:12d1`, `0d06:12ec`), advances by the same word per glyph
(`0d06:11fb`), and puts the **line gap** at `ds:0x13c6` between lines
(`0d06:11d2`). Both words rest at 1 in this build's data segment, and nothing
on the request path writes either — no template, and so no second pass and no
outline behind these letters.

`SCRCTRL` is worth its own line, because reading it as the later games can be
read is wrong rather than incomplete. The handler is three instructions
(`0104:165d`): pop, current screen, store at `+0x14`. It resolves nothing and
checks nothing, so the id belongs to the screen and a negative one is the
absence of a controller. This game gives three screens three different
controllers and clears the third, which is what showed it.

## Open questions

Each kept in the [ledger](../../open-questions.md#motion-16-bit):

- **What `SETSHADE` was for** — it stores two values at `ds:0x150` and
  `ds:0x152`, nothing in this binary or `ENVIRO.EXE` reads them, and `RUN`
  calls it once.
- **The 32-byte `PSMCFG.DAT`** the older sound setup writes and this binary
  reads (its name is at file `0x14ee9`); the later games' 36-byte layout
  does not apply.
- **Whether the driver clears the playing flag** (`ds:13dc`) when a song
  plays out, which decides whether `ENDTUNE` after Victor Loomes' jingle
  still holds the game — the recording says it did, at least there.
- **What the fade after the intro's jingle runs into**: the rebuild matches
  the recording for 744 writes and the recording carries 33 seconds more.
- **What *Motion 1.0* is**, and who the two names after it in the credits
  are.
- **Whether the car's key stops for a key** — the talk kernel's assertion
  hook ends in `KEY`, which blocks, and motionvm answers at once
  ([departures](../../departures.md#the-16-bit-machine)).

## See also

- [ENVIRO.EXE](enviro-exe.md) — the latest build, and everything the five share
- [BMZ.EXE](bmz-exe.md) — the fourth build, whose ordinals do not shift
- [HPPLAY.EXE](hpplay-exe.md) — the build whose ordinals do
- [STERN.EXE](stern-exe.md) — the next build after this one, with this core table and a base of 102
- [Kernel words](../vm/kernel-words.md) — the two tables, entry by entry
- [The DATA container](../formats/container.md) — the earlier framing this game ships
- [Other files (Victor Loomes)](../../games/vloomes/other-files.md) — what else the installation holds
