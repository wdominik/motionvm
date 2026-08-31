[← Documentation index](../../README.md)

# Interaction: Mouse, Keys, Walking and the Inventory

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

What the player does and what the scripts read: the pointer and the keyboard,
the figure's walk, the inventory bar and the order table. All of it measured
from the call sites in Die Enviro-Kids greifen ein.

## Mouse and keys

`MOUSEX`, `MOUSEY`, `MOUSELK`, `MOUSERK`, `?KEY` (0 when no key), `ATMOUSE`,
`XATMOUSE`, `SHOWMOUSE`, `HIDEMOUSE`, `MOUSEINFO`, `?XINSIDE`. `FATMOUSE` and
`FXATMOUSE` are script words (module 603) that store the sprite id in
`_BMNR` and call `ATMOUSE`/`XATMOUSE`. `KEY` blocks for a key; the game
uses it on its debug path only.

## Walking, inventory, orders, the pointer

The handlers behind these names are **read and the same code as the
32-bit engine's**, compiled for 2-byte cells — the same records with every
field at half the offset, the same command numbers, the same state
machine — with the differences listed:

| Word | Read at | Same as the 32-bit handler | Differs |
|---|---|---|---|
| `DOWALK ( person -- )`, `CROUTE` | file `0xf25f`, `0xe6d0` | the person record (`+0xc8` descriptor, `+0xd0` step index, `+0xd4` walking, `+0xe0` queue, `+0xe6` command …), the commands 1001–1003, 1006, 1007, 999, the turn table, the step buffer, the route search, the facing rule; a 12-byte step, an 18-byte route after a 2-byte count, a 12-byte extra record | arithmetic in 16 bits — a product before a division in the line test and the size ramp can wrap where the 32-bit code's cannot; the `1006` size is the shadow's as it stands (both binaries, in fact) |
| `?XINSIDE ( x y table stride count -- i \| -1 )` | file `0xf09b` | inclusive corners | the all-zero-record-is-a-hole test is the build's, not the generation's: `ENVIRO.EXE` and `BMZ.EXE` make it, `HPPLAY.EXE` and `LL.EXE` do not |
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

## Open questions

- The `XDEFTDT` template numbers.

## See also

- [Descriptors](descriptors.md) — what the pointer and the bar are made of
- [Screens and the draw chain](screens.md) — where they are drawn
- [Boot and frame loop](game-loop.md) — when input reaches the scripts
- [Interaction (MOTION 32-bit)](../../motion32/engine/interaction.md), [Walking (MOTION 32-bit)](../../motion32/engine/walking.md) — the other generation's
