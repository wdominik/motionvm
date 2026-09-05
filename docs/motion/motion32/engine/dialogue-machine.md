[← Documentation index](../../README.md)

# The Dialogue Machine

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Conversations run natively, inside the
[interaction machine](interaction.md): `EXECORDER`'s talk case fetches
the conversation data, `CALCDIALOG` (`0x7b49c`) walks it, and `DOORDER`'s
modes 12–18 drive display and input. The script side only supplies the
data and the per-location talk word.

## Starting a conversation (verb 5, `0x7c69e`)

The location's **talk word** (callback at `_ORDER+0x50`) receives the
clicked target and answers with **two addresses**: the conversation
**record** (stored at +0x17C) and its **fields** (+0x180). A −1 answer
means "nothing to say."

## The conversation data

The record:

| Offset | Description |
|---|---|
| +0x04 | Entry node |
| +0x08 | Number of answers |
| +0x0C | Number of lines |
| +0x20 | Alternate entry node (the "say nothing" continuation) |
| +0x24 | Name (ASCII) |
| +0x30 | Enable flags, 4 bytes per answer (bit 0 = available, bit 1 = one-time) |

The fields area holds three consecutive packed arrays:

| Array | Stride | Fields |
|---|---|---|
| Answers | **0x12** | +0 node id, +8 name |
| Lines | **0x10** | +0 text, +4 text table, +8 next node, +0xC speaker |
| Branches | **0x22** | +0 target-conversation name, +9 answer name, +0x12 action, +0x1E next node |

The strides 0x12 and 0x22 are **not divisible by four**: every second
entry starts mid-cell. The engine resolves only the *base* to a cell
boundary and then indexes byte-wise — any reimplementation must read
these records byte-exact, not through cell fetches (see
[Execution model](../vm/execution-model.md)).

## Node numbering

The current node lives at `_ORDER+0x194`, and its **magnitude encodes
its kind**:

| Node | Kind |
|---|---|
| −1 | Conversation over |
| < 1000 | A spoken line |
| 1000–1999 | A player answer (index − 1000) |
| 2000–2999 | A branch (index − 2000) |
| 4000 | Return to where the player last picked an answer |

A branch's successor is *renumbered* on the way out: 0–999 becomes a
branch (+2000), 1000–1999 a line (−1000), 2000–2999 an answer (−1000).
Two globals support this: `0xdbd64` holds a queued node offset consumed
in passing, `0xdbd60` the last answer node (what node 4000 returns to).

## Speakers

The speaker table hangs at `_ORDER+0x16C`: up to ten entries of **0x28
bytes** — +0 a script word, +4 color, +8 text template, +0xC/+0x10
position, +0x14 level, +0x18 the unhighlighted answer color; the
terminator is an entry whose +4 is −1. The speaker word is called with a
small integer telling it why:

| Argument | Occasion |
|---|---|
| 1 | The line has expired |
| 2 | While the line stands |
| 3 | Conversation end |
| 4 | Every frame, for everyone *not* speaking |

## Showing a line (modes 12/13)

`CALCDIALOG` mode 0 fills the line descriptor (+0x174) from the line and
its speaker entry, then clamps it into the picture natively — 20 pixels
of margin on the 640×400 view, re-centering through the computed
`GDCX`/`GDCY` after every move (the same algorithm the script word `TSC`
applies). The mode becomes 13 when the line names a speaker, 12
otherwise.

`DOORDER` then waits in mode 12/13 until the text descriptor has expired
through the standard wait/callback mechanism (see
[Descriptors](descriptors.md)); a click shortens the wait to zero. Then
the next node is fetched.

## The answer menu (`0x7b9fd`, mode 14)

Four descriptors, stacked bottom-up from +0x174: up to three answers
plus the standing "say nothing" line (from +0x198/+0x19C). Answers whose
enable bit 0 is clear are skipped; the chain runs answer-to-answer
through each entry's +4 field. Geometry: the "say nothing" line sits at
`GSCRY + 0x168`, the first answer 0x23 above it, each further answer its
own height plus 0x11 higher — placed by setting the bottom edge and
re-centering. The chosen indices stay at +0x1A4… so the click can map
back.

**Mode 14** hit-tests the four boxes on a fresh left click (first hit
wins, all four disappear together): the fourth box continues at the
record's +0x20 node; an answer whose enable bit 1 is set gets bit 0
cleared — **a one-time answer**. Every frame, the answer under the
pointer is recolored with the speaker table's +4 color and the others
with +0x18 — and `SDCOL` runs only on an actual change, because setting
a color re-measures the text.

## Ending (mode 16)

The location's talk word is called one final time with −1, the pointer
is restored, and the mode falls to 0 — which is why `?DIALON` (true for
modes 12–18) then reports the conversation over.

## The item menu (modes 17 and 18)

The inventory bar stays live under the answers. While mode 14 waits, a
fresh right press over one of the bar's eight slots — `_IMX` from 0x40, 64 a
slot — with something in it (0x7e52e) puts the item into the block's target
(+4) and slot (+0xF0), sets **mode 17** and calls `GMSHOWMENU` for the bar's
strip with the mask `0x60`: two icons, verbs 6 and 7, `INFO` and `GIVE`,
centered over the slot and 0x30 down. `GMSHOWMENU` reads the mode first
(0x7a303) and adds the look verb to every menu but this one's and the
answers' — which is why the mode goes up before the strip does.

**Mode 17** (0x7e918) is the strip's frame: `HIGHLIGHTORDERS` over the bar's
descriptors, then a fresh left press is `CHOOSEORDERS` over the same mask —
slot 0 is verb 6, slot 1 verb 7 (0x7aeae) — which sets the verb, takes the
strip down and leaves **mode 18**; a fresh right press with +0x134 clear
takes the strip down and goes back to the answers, mode 14 (0x7e997). The
epilogue animates the bar's strip in mode 17 as it does in mode 2 (0x7eaee).

**Mode 18** (0x7e9b1) waits for the figure — the person at +0x78, whose
command cell +0x1CC must be 0 or 999 — then runs the pick as a forced order:
mode **98**, `EXECORDER` on the verb, the item and the flag (0x7bf80), and
the picked word at +0x120. Mode 98 hands back to the answers when the order
is done; verbs 6 and 7 enter the conversation at the answer named `DINFO` or
`DGIVE` for the item (see
[Game library](../../games/ds2/library/game-library.md)).

## Mode 15, which nothing reaches

The branch at 0x7e6d0 shows a last line: while the line descriptor at +0x174
is up, speaker 0 hears 2 and the others 4; once it is gone every speaker
hears 3 and the mode falls to 16. The only store of 15 into the mode cell is
in the routine at `0x7b120`, which lays the quiet line (+0x198/+0x19C) into
+0x174, centered at `GSCRX + 0xA0`, `GSCRY + 0x50`, runs the word at +0x1A0
and sets the mode — and nothing in `ENGINE.EXE` calls, jumps to or holds the
address of that routine, in code or data. The build left it behind, and the
shipped game cannot take the branch. The rebuild refuses the mode and says
why.

## Branches and the deferred-change queue

A **branch node** changes state instead of speaking. Without a name it
applies to the current conversation immediately; with a name it is
copied onto a queue (head at +0x1C0, 0x22-byte entries: conversation
name +0, answer name +9, action +0x12) that is worked off when the named
conversation next starts — each entry firing once. The actions:

| Action | Effect |
|---|---|
| 1 | Enable an answer (set bit 0) |
| 2 | Disable an answer (clear bit 0) |
| 3 | Set the entry node |
| 4 | Set the alternate entry node (+0x20) |
| 5 | Call a script word **by name** (lookup at `0x61596`), only if found (0x7be58); a miss reaches the diagnostic channel and nothing runs |
| 6 | As 5, but **unchecked** (0x7bece — a miss calls whatever the previous hit left in the globals `0xEE668`/`0xEE66C`), then an unconditional pop becomes the queued node offset `0xDBD64` (0 on an empty stack, per `pop` 0x66c95) |

Only actions 5 and 6 drain the deferred-change queue afterwards
(0x7b258); actions 1–4 jump straight out. Every path then re-enters
`CALCDIALOG` with mode 1 (0x7bf6b), so the step happens in the same call.

The name lookup walks the module table at `0xEE6D0` (stride 0x30, count
`0xDB027`, module number at +0x10) in entry order; the hash over the
name's length and characters 0, 1, 2 and 5 only prunes the search.
Entry order is slot order: `=>GET` takes the first free slot (0x64999) and
`=>ERASE` frees one in place, so two modules that define one name are told
apart by which was loaded into the earlier slot.

**A trap that bit here:** every field of the packed tables must be read
byte-exact. The branch successor at +0x1E of a 0x22-stride entry never
sits on a cell boundary; a cell-addressed read turned a stored −1 into
−65536, which the signed compare at 0x7b780 sent down the spoken-line
path — the crash after the first dialogue choice.

## Open questions

None on this page. Modes 12 to 18 are read; 15 is not built because nothing
reaches it, and the menu routines the item menu shares with the verb menu
(`0x7a552`, `0x7a759`, `0x7ad26`) and `TEXTTOPERSON` (`0x7b053`) are read
under the [interaction machine](interaction.md).

## See also

- [Interaction machine](interaction.md)
- [Objects and flags](../../games/ds2/library/objects-and-flags.md) — the script-side
  dialogue records and blocks
- [Text rendering](text-rendering.md) — how the lines are drawn
- [Departures](../../departures.md) — the name lookup's walk order
