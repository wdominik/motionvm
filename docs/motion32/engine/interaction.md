[← Documentation index](../../README.md)

# The Interaction Machine

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Pointing, clicking, the verb menu, and verb execution form one native
subsystem of roughly 4,800 instructions — the largest coherent piece of
the engine. Its entry point is **`DOORDER`** (handler at `0x7cd61`, 8,072
bytes), called once per frame by the control handler with the `_ORDER`
block as its argument; verb execution goes through **`EXECORDER`**
(`0x7bfbb`). Conversations have [their own page](dialogue-machine.md).

## The `_ORDER` block

The 520-byte `_ORDER` record is the shared state between script and
engine. The script side installs callbacks into it (see
[Game library](../../games/ds2/library/game-library.md)); the control handler copies
the input snapshot in before every `DOORDER` call; `DOORDER` reads and
advances it. The engine-side field map:

| Offset | Content |
|---|---|
| +0x00 | Verb (0 = none; 1–8, e.g. 4 = "use item") |
| +0x04 | The affected item / target (a "leave" order stores the destination here) |
| +0x08 | Flag argument |
| +0x0C | **Mode** (see below) |
| +0x10 / +0x14 | First verb-menu descriptor on the scene screen / on the inventory bar (five each) |
| +0x18 | **Verb table**: 8 entries × 12 bytes — +0 the icon's rest sprite, +4 its end sprite, +8 the verb's script-handler address. The verb a bit stands for is **bit + 1** |
| +0x50 | The location's talk word (callback) |
| +0x7C / +0x80 | Screen for dialogue / for inventory drawing |
| +0x84 | Area table pointer |
| +0x88 | Item table pointer (20 B/entry: +0 name text, +4 flags, +8 sprite) |
| +0x8C…+0x98 | `_MMX`, `_MMY`, `_IMX`, `_IMY` |
| +0x9C / +0xA0 / +0xA4 | `_MLK`, `_MRK`, `_MPRESSED` |
| +0xAC | Inventory list (head = scroll offset, then item ids, zero-terminated) |
| +0xB0 / +0xB8 | `CALCINV` callback / cursor-setting callback (`FXATMOUSE`) |
| +0xF0 / +0xF4 | Clicked inventory slot / area count |
| +0xFC | Area under the pointer, **stored biased by 1000** |
| +0xC8…+0xD8 | Five pulse states, one per menu slot: 0 idle, 1 growing, 2 shrinking |
| +0x100 | Caption descriptor, placed 0x14 above the strip |
| +0x128 / +0x12C | Picked-up item / its caption |
| +0x134, +0x1CC | Pending-state fields; either nonzero suppresses the click path |
| +0x138 | Pickup origin: −1 from the inventory bar, +1 from the scene |
| +0x140, +0x1E0, +0x1E4 | Talk-line descriptor, its `SDCOL`, its `SDTDT` |
| +0x16C…+0x1C8 | The conversation state (see [Dialogue machine](dialogue-machine.md)) |

### Modes

The mode at +0x0C is the interaction state machine:

| Mode | Meaning |
|---|---|
| 0 | Free play (idle path: nothing pressed → nothing happens) |
| 2 / 4 | The verb menu stands open, over the inventory bar / over the scene |
| 3 | An item is "in hand", waiting for what to use it on |
| 5 | Run the chosen order now |
| 6 / 7 | Waiting for the figure to walk up, and what follows the walk |
| 8 | Execute the verb the menu picked |
| 9 | Exit chosen — runs the leave verb, but on the **next** frame |
| 0x11 | The yes/no menu (slots 0 and 1 are verbs 6 and 7) |
| 12 / 13 | A spoken line stands on screen (13 = with a named speaker) |
| 14 | Answer menu awaiting a pick |
| 15, 17, 18 | Dialogue sub-states (unmapped) |
| 16 | Conversation ending |
| 97 → 99, 98 | Forced orders (see below) |

`?DIALON`'s range 12–18 is exactly the dialogue modes.

### Fresh presses only

Every click branch requires `_MPRESSED = 0` — the "freshly pressed"
condition; a held button triggers nothing. The condition chains are
short-jump sequences whose targets are almost always the *skip* paths;
read naively, every branch inverts.

## Left click (`0x7ce73`)

- **On the inventory bar** (x in 0x40…0x240 — eight slots of 64 pixels,
  exactly where the inventory drawer puts them): look up the slot's item;
  if occupied, pick it up.
- **In the scene**: the area-search helper finds the area under the
  pointer; if the area carries an **item** (+0x28) whose flag bit 0
  ("takeable") is set, pick it up; else if it carries an **exit**
  (+0x24), store the destination and switch to mode 9.

Picking up an item sets verb 4 and mode 3, records the origin at +0x138
(−1 bar / +1 scene), and turns the pointer into the item: the cursor
sprite is the item's sprite **plus one** (the highlighted variant of the
pair), hung at hotspot (0x20, 0x18). A scene pickup also runs
`SETFLASHENTRY` (below); a bar pickup runs the +0x114 callback instead.

## Right click — the verb menu

The flags of the clicked target (item flags, or the area's own verb mask
at +0x38 when no item) are OR-ed with bit 1 and the set bits counted;
**flag bit 7 suppresses the menu entirely** (`LEAVEONLY` exits show no
menu). `GMSHOWMENU` (`0x7a2e2`) then builds the strip:

- Up to **five** descriptors, starting at +0x10 over the scene and +0x14
  over the inventory bar.
- One icon per set flag bit, its rest sprite from the verb table
  (+0x18 + 12·bit), laid left-to-right at 0x30-pixel steps, the whole
  strip centered on the anchor (half a step left per icon).
- The anchor comes from the area's fields +0x2C/+0x30, or the rectangle
  center when both are zero; it is clamped into the visible window and
  the caption descriptor (+0x100) is placed 0x14 above it.
- Unused menu descriptors are deactivated; the cursor switches to the
  menu pointer. Mode becomes **4** over the scene, **2** over the bar.
- Nothing under the pointer means no menu and no mode change at all —
  which is why a right click on an empty screen does nothing.

While the menu stands, two routines run every frame:
`HIGHLIGHTORDERS` (`0x7a759`) marks the slot the pointer is on in the
five pulse states at +0xC8, and `ANIMATEORDERS` (`0x7a900`, from the
epilogue) walks that icon one sprite a frame between its rest and end
pictures, turning round at each end and pushing every other icon back to
its rest picture.

**A pick** — `CHOOSEORDERS` (`0x7ad26`), on a fresh left click — box-tests
the five slots, maps the slot to a verb through `VERBOFSLOT` (`0x7a144`:
the *k*-th set bit, verb = bit + 1), stores it at +0x00, takes the menu
away and leaves **mode 8**, where the verb is executed. Two verbs go
elsewhere: verb 4 puts the thing on the cursor (mode 3), and verb 8 on an
area is a walk — mode 9, and the target becomes the area's exit.

**A cancel** — a fresh right click with +0x134 clear, or +0x1CC set —
calls `GMREMOVEMENU` (`0x7a552`): both strips off, the `CALCINV` callback,
the plain arrow back through +0xBC, the +0x13C callback, mode 0.

## The area records

The per-location click areas (64 bytes each, table at +0x84, count at
+0xF4):

| Offset | Content |
|---|---|
| +0x00…+0x0C | Rectangle x1 y1 x2 y2, **inclusive** corners |
| +0x10 | Label text (shown by the pointer-info word) |
| +0x24 | Exit (destination location) |
| +0x28 | Item id |
| +0x18 / +0x1C / +0x20 / +0x34 | The four approach parameters, copied to +0x10C / +0x104 / +0x108 / +0x118 |
| +0x2C / +0x30 | Menu anchor (0/0 = use the rectangle center) |
| +0x38 | Verb mask when the area itself is the target |

An area is switched **off** by pushing its x1 past its x2 — the shipped
data adds 10000 — so the rectangle test can never hit it.

The area search stores its hit as `1000 + index`, which is why the
handler then reaches area fields through large negative displacements
(`area·64 − 0xf9d8` folds back to `index·64 + 0x28`). A record of four
zeros is a hole, not a rectangle at the origin — the same rule the
generic rectangle test `?XINSIDE ( x y table stride count -- i )`
applies.

## SETFLASHENTRY — item into inventory (`0x7ac36`)

Adds the picked item to the inventory list, re-finds it, and **scrolls
the bar** when the item sits past the eighth slot (head = index − 7),
then redraws through the `CALCINV` callback. The inventory list itself
is `{scroll offset, 99 item slots, zero-terminated}`; adding appends,
removing closes the gap.

## EXECORDER — verb dispatch (`0x7bf80` head, `0x7bfbb` body)

Dispatches `verb − 1` through a jump table at `0x7bf9b` (a verb outside
1–8 returns immediately):

| Verb | 1 TAKE | 2 EXAMINE | 3 HANDLE | 4 USE | 5 TALK | 6 INFO | 7 GIVE | 8 LEAVE |
|---|---|---|---|---|---|---|---|---|
| Case at | 0x7c00c | 0x7c111 | 0x7c3aa | 0x7c543 | 0x7c69e | 0x7c7e8 | 0x7ca7f | 0x7cd43 |
| Handler slot | +0x20 | +0x2C | +0x38 | +0x44 | +0x50 | +0x5C | +0x68 | +0x74 |
| Script word (module 5) | `CALCTAKE` | `CALCEXAMINE` | `CALCHANDLE` | `CALCUSE` | `CALCTALK` | `CALCINFO` | `CALCGIVE` | `CALCLEAVE` |

The script words come from module 5, installed into those slots by
`STARTUP` in module 3. Note verbs 6 and 7: the slot at +0x5C gets
`CALCINFO` and the slot at +0x68 gets `CALCGIVE`, which is the **reverse**
of module 5's own constants (`GIVE=6`, `INFO=7`) — an original
inconsistency, harmless only because no menu can offer either verb. See
[Game library](../../games/ds2/library/game-library.md).

Each case sets a few descriptors up, runs the verb's **script word** from
the verb table (+8 of the entry) and reads back **one** value:

| Code | Effect | Meaning |
|---|---|---|
| 0 | activate the description descriptor; clear bit 0 of +0x11C | "nothing special — show the text" |
| 1 | **set** bit 0 of +0x11C | "not finished, call me again" (what `CALLDIR` answers while it turns the figure) |
| 2 | reset the text to the spare/refusal line; clear the bit | "done, and put the text back" |
| 3 | nothing | "done" |

Each script word asks the **location's own hook** first — the module-2
globals `_LC_TAKE`…`_LC_LEAVE`, written by every location macro and
cleared by `INCLLOC` — and falls back on a global in module 13. The verb
table itself is filled by `STARTUP` in **module 3**.

The description a verb shows comes from the thing itself: an item keeps
it at +0x0C (`FINFO`), an area at +0x14, and a thing with neither falls
back on +0x14C. It is shown in block +0x144 through the descriptor at
+0x140 (`_IINFO`, level 254). Verbs 2, 3 and 4 finish by calling
**`TEXTTOPERSON`** (`0x7b053`, 205 bytes), which centers that descriptor
over the actor's own descriptor (`person[0x190]`), 0x14 above it, and
then runs the hook at +0x168.

Verb 1 is the exchange: on code 0 it runs `SUBFROMINV(+0x128)`,
`ADDTOINV(+0x04)`, bumps the bar's scroll cell and redraws through
`CALCINV`; it is the only case that publishes its state, at +0x1DC, which
the menu's modes 6 and 7 read back. Verb 8 is two lines — run the word,
clear the bit — and is the whole of the player-driven scene change.

Verbs 6 (INFO) and 7 (GIVE) show nothing themselves: they ask their
script word for a **name** — the address `_PutStringAdr` pushes — and
enter the running conversation at the answer carrying it. Three passes:
the keyword (the word's answer, else the field at +0x160 / +0x15C, which
in the shipped game points nowhere); then the literal `"DINFO"` /
`"DGIVE"`; then a complaint, `node = 1000`, and one more look with the
field's own keyword. The node is therefore never below 1000, which makes
the handler's own check for that dead code. On success it writes
`o[0x194]`, deactivates the four answer descriptors at +0x174, hides the
pointer and calls `CALCDIALOG(o, 1)`. Neither touches +0x11C.

Only an exact match counts: `CompareString` also answers −2 and −3 for a
prefix and a suffix, and both cases test for −1 alone.

No flag constant in the game sets bit 5 or 6, so no menu can offer these
two; they are reached from a forced order.

## Forced orders — modes 97–99

`FORCE_ORDER ( verb obj1 obj2 -- )` (module 13) fills `_ORDER`, sets
mode **97**, and calls `DOORDER` — passing `_ORDER @` (the block's first
cell, which it just set to the verb) instead of the block's address. The
call is **harmless**: the work is in the four stores, and the control
handler finds mode 97 with the real block on the next frame. Mode 97
switches itself to 99 and calls `EXECORDER` with the stored verb/target;
modes 98/99 then either re-run `EXECORDER` or, once field +0x11C bit 0
is clear, run the +0x124 callback and fall back to mode 14 or 0. An
unknown mode falls through to the epilog — the argument is never
validated.

## Native helpers and reentrancy

The interaction helpers are internal functions with **register argument
passing** (Watcom convention: `eax`, `edx`, `ebx`, `ecx`, rest on the
stack) — which is why static pop-counting sees zero arguments for them.

**Native handlers re-enter the bytecode interpreter**: `MOUSEINFO` calls
script words eight times (always as the last act of a branch — a tail
call), `DOORDER` twenty-two times, including mid-function. The nested
runs use their own return stack while **sharing the data stack**, and
they must not be deferred: the called words read `_ORDER` fields the
handler rewrites between calls. A nested run also never treats a
blocking word as a suspension point (see
[Execution model](../vm/execution-model.md)).

## Pointer info — `MOUSEINFO`

The scene hit-test word takes **24 arguments** (the handler pops exactly
that many): the split mouse coordinates, the item and area tables with
their geometry, the info/mouse-over descriptors, the three cursor
sprites (390 normal, 391, 392), the cursor callback, and the interaction
state. Three cases — over the scene, over the bar, over neither; in the
menu modes (2/4/5) it does nothing. A last-mode global (`0xdbd5c`)
forces a redraw when the case changes, and every branch ends by
tail-calling the cursor callback.

## The pointer draws itself

The cursor is the mouse layer's own drawing, not a descriptor.
`SHOWMOUSE` (0x2541d) and `HIDEMOUSE` (0x2560d) wrap a nesting counter
(`0xD672C`); the draw helper 0x2543e composes into an 8-aligned scratch
block — save-under from the video surface `[0xE7D7C]`, then the masked
blit 0x26594 at the `& 7` x-remainder — and copies the block back, so
the net position is exactly pointer minus hotspot
(`0xD6724`/`0xD6728`), color 0 transparent. `XATMOUSE` (0x73525)
resolves the sprite, hides, defines the shape (0x24637) and shows
again. The callers of hide/show, found by searching the relocated image for
calls to them, are the driver layer
(the pointer follows movement in the interrupt) and engine routines
bracketing their own blits — fades included — so the pointer sits on
top of every shown frame, curtain and all — above the curtain, not
behind it, and above every descriptor regardless of level.

## Open questions

- The verb-menu interaction chain (modes 2–8) beyond the initial build.
- The case bodies of verbs 1–4 and 6–8 (only skeletons and callback
  slots are mapped).
- The meaning of the pending fields +0x134/+0x1CC.
- Helper routines `0x7ad26`, `0x7a759`, `0x7a552`, `0x7b053` (unread).

## See also

- [Dialogue machine](dialogue-machine.md)
- [Game loop](game-loop.md) — where DOORDER runs
- [Game library](../../games/ds2/library/game-library.md) — the script side of `_ORDER`
- [Descriptors](descriptors.md)
