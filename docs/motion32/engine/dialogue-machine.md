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
The rebuild walks the same table in module-number order instead — a
recorded [departure](../../departures.md); all known duplicate names live in
location modules never loaded together.

**A trap that bit here:** every field of the packed tables must be read
byte-exact. The branch successor at +0x1E of a 0x22-stride entry never
sits on a cell boundary; a cell-addressed read turned a stored −1 into
−65536, which the signed compare at 0x7b780 sent down the spoken-line
path — the crash after the first dialogue choice.

## Open questions

- Dialogue modes 15, 17, 18 (and the mid-conversation right-click menu).
- The remaining `CALCDIALOG` helpers (`0x7ad26`, `0x7a759`, `0x7a552`,
  `0x7b053`).

## See also

- [Interaction machine](interaction.md)
- [Objects and flags](../../games/ds2/library/objects-and-flags.md) — the script-side
  dialogue records and blocks
- [Text rendering](text-rendering.md) — how the lines are drawn
- [Departures](../../departures.md) — the name lookup's walk order
