[← Documentation index](../../README.md)

# Descriptors

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

A descriptor is one visible element on a [screen](screens.md): a
background, a character, an item, a text box. Descriptors are the
engine's scene graph.

## The current-descriptor model

`NEWSETDESC` creates a descriptor, returns its handle, and makes it the
*current* descriptor. The many `SD…` words that follow operate on the
current descriptor — they take no descriptor argument.
`ACTDESC ( handle -- )` re-selects an existing descriptor; a handle
that resolves to nothing leaves the **previous selection in place**
(the handler skips both stores), so subsequent `SD…` words silently hit
the old descriptor rather than nothing. The current handle lives at
`0xDB4C0` (read identically by `GDNR` and `?ACTDESC`), the resolved
record separately at `0xF2AF0`.

## NEWSETDESC — six arguments, now fully read

```
x y level gfx ? callback   NEWSETDESC   ( -- handle )
```

- Arguments 1–3 (`x`, `y`, `level`) go to fields +4, +6, +8, and both
  placement modes (below) are set to 1 — corner.
- Argument 4 is a **graphics id**, stored into the same payload `SDBL`
  writes — a room background needs no `SDSPR` after it.
- Argument 5 is popped and **discarded** (meaning unknown).
- Argument 6 — the top of the stack — is a **completion callback**: the
  same field (+0x14) that `SDWORD` writes. 0 and −1 clear it; anything
  else is validated before storing (the module must be loaded and the
  offset inside it; a third header check exists at `0x68367`). The new
  descriptor's wait (+0x18) is initialized to **−1** ("never"), so the
  callback cannot fire before it is armed.

The game's caption descriptor `_TI1` is created as
`160 100 100 1 0 <FOLLOWMAN> NEWSETDESC` — the callback is module 5's
`FOLLOWMAN`, whose entire body is `SDINACTIVE EXIT`. That is the
mechanism by which a caption hides itself when its time expires.

## The descriptor structure

A descriptor is a **0x3E-byte structure** plus a payload. Known fields:

| Offset | Type | Description |
|---|---|---|
| +0x02 | `u16` | Descriptor type: 1 group, 2 sprite, 3 image, 4 text — **unverified**, see [Open questions](#open-questions) |
| +0x04 / +0x06 | `i16` | X / Y position (interpreted per placement mode) |
| +0x08 | `i16` | Level (draw order) |
| +0x0E | | Group: sibling-list head/tail; text: the layout record |
| +0x12 | `u16` | **Placement modes**: bits 0–1 horizontal, bits 2–3 vertical |
| +0x13 | `u8` | Flags: `0x80` active, `0x40` dirty, `0x20` buffer filled, `0x10` changed, `0x08` buffered (`SDAUTOBUF`); on a screen's byte, bit 2 = frozen |
| +0x1C | `u16` | The buffer number `SDAUTOBUF` hands out, or −1 for none |
| +0x14 | `u32` | **Completion callback** (set by `SDWORD` or `NEWSETDESC` arg 6) |
| +0x17 | `u8` | Bit 7 = center the text block as a whole (set only by `SDBLK`, which no shipped script calls — see [Text rendering](text-rendering.md#centering)) |
| +0x18 | `i32` | **Wait counter** (set by `SDWAIT`): −1 never, 0 due every frame, n counts down |
| +0x1E / +0x22 | | Sibling chain: next / previous |
| +0x26 | | Owning screen |
| +0x2E / +0x32 | `u16` | **Stored size** width / height (for text: measured + 4) |
| +0x36 / +0x3A | `u16` | Same values, written together with +0x2E/+0x32 |

## Placement modes

What the coordinate at +4/+6 *means* is stored beside it, per axis, in
the mode field at +0x12:

| Mode | Set by | Meaning of the stored value |
|---|---|---|
| 1 | `SDX`, `SDY` (and `NEWSETDESC`) | The **corner** |
| 2 | `SDCEN`/`SDCX`, `SDVCEN`/`SDCY` | The **center** |
| 3 | `SDOX`, `SDOY` | The **far edge** — `SDOY 315` puts a figure's feet on y = 315 |

`SDCEN` and `SDCX` are **the same word** (the `SDCEN` handler carries
the internal name `SDCX`), as are `SDVCEN` and `SDCY`. The drawn corner
is computed from mode and **stored** size:

```
mode 1: corner = value
mode 2: corner = value − size/2
mode 3: corner = value − size
```

All geometry getters answer from this computation: `GDX`/`GDY` return
the drawn corner, `GDCX`/`GDCY` corner + size/2, `GDOX`/`GDOY` the far
edge. `GDWIDTH` and `GDXLEN` are one handler under two names (as are
`GDHEIGHT`/`GDYLEN`), both returning the stored size — which for text
is the measured size **plus 4** (see
[Text rendering](text-rendering.md)). Because getters and placement use
the same stored size, `GDCX SDCEN` is an exact round trip.

## The text payload is created late, and zeroed

A descriptor made by `NEWSETDESC` has **no text record**. The 0x5C-byte
record hanging off +0x0E is allocated by whichever of `SDTXT` (`0x71b4f`)
or `SDTB` (`0x71d45`) runs first, and both allocate through `0x203F3`,
which hands the small-block case to `0x823E0` — a **zeroing** allocator
(`xor %al,%al; rep stos`).

The constructor then writes five fields by hand — +0x00 table, +0x04
entry, +0x08 template, +0x10 font, +0x18 — and leaves the rest to the
allocator. Among the rest is **+0x0C, the color** (`SDCOL`/`GDCOL`).

So there is **no "unset" color**: a text nobody gave one draws in index
0 and `GDCOL` answers 0. That is not an inference — it is why the help
pages are black, since `SHOW_DOC` contains no `SDCOL` and `XYLTITEM.`,
which builds its descriptors, contains none either (see
[Text rendering](text-rendering.md) and [Shell](../../games/ds2/library/shell.md)).

## The payload

A descriptor carries exactly one payload, set by the word that gives it
content:

| Type | Set by | Payload | Meaning |
|---|---|---|---|
| 2 | `SDSPR` | 8 bytes | Sprite with animation state |
| 3 | `SDBL` | 4 bytes | Full image without animation state |
| 4 | `SDTXT` / `SDTB` | 0x5C bytes | Text |

(Type 1 is a group — the screen itself heads the sibling list.)
**Types 2 and 3 index the same GFX8 pool**; the name "block" in `SDBL`
is unrelated to the BLOCK resource type. All ids passed to `SDBL` in
the game (43, 60, 64, 66, 1010) exist as GFX8 sprites.

## `SDAUTOBUF` — the only thing that erases

`SDAUTOBUF` (`0x72104`) raises a counter at `0xDB4B4`, writes the number it
comes to into +0x1C and sets flag `0x08`. Hanging off that number is a **record
of the picture under the descriptor**: the drawer copies the surface into it
before every blit (`0x696ed` → `0x6b936` → `0x2825d`), filling in the same
rectangle and level the descriptor has — width rounded up to a multiple of
eight (`0x6ba18`), and for a text grown by the template's margin (`0x6baa7`) —
and setting `0x20` to say the copy is there (`0x6bb89`, `0x6bd26`). When the
descriptor is then hidden or moved, `0x6ab6e` marks that rectangle too
(`0x6ac33`) and the copy goes back.

**Without one, nothing disappears.** `SDINACTIVE` marks only its own rectangle
on its own level, which repaints what is above it and nothing below (see
[Screens](screens.md#the-damage-map)), so the descriptor's pixels stay on the
surface.

The game is consistent about it: `INITANI` gives every animation the flag
(module 6, `0x007dc`), and so do the captions `_TI1`/`_TI2`, the descriptions
`_IINFO`/`_MINFO`/`_TINFO` (module 3) and the menu sprites. Across all
eighty-six modules only thirteen descriptors are made without it and hidden
later — and four of those are the mailbox's own, where staying put is the
point.

motionvm reproduces the effect rather than the record: when a descriptor
carrying the flag leaves a place, that place is built again out of the
descriptor list. The two agree wherever the picture belongs to descriptors,
which is everywhere the game uses the flag, and a rebuild cannot go out of date
the way a remembered copy can. It is a deliberate difference and is written down
as one — see [Departures](../../departures.md#display-and-timing).

## Visibility, order, and the frame walk

- The active bit is `0x80` in byte +0x13, set only by
  `SDACTIVE`/`SDINACTIVE` — and **a newly created descriptor is
  visible**, dirty and changed with it (`NEWSETDESC` writes `0xD000` at
  `0x70d07`).
- Descriptors hang in a **sibling chain per screen**, and insertion
  (`0x6a648`) advances past every entry of the same or lower level
  before linking: the chain is a **stable sort by level**. Draw order
  and the per-frame walk follow the chain; no separate sort exists.
- Being active is not enough to be drawn: the dirty bit `0x40` has to be up
  as well, and the damage map is what raises it. See
  [Screens](screens.md#the-damage-map).
- The per-frame walk (`0x68c64`, run from the
  [game loop](game-loop.md)) implements timed callbacks: a descriptor
  **with** a callback (+0x14) has its wait (+0x18) counted down; at
  zero the callback runs as a nested bytecode call. Without a callback
  nothing happens — `SDWAIT` alone is inert. Descriptors on a
  **frozen** screen (`FREEZESCR`, bit 2 of the screen's flag byte) are
  skipped entirely.
- **`SDWORD` sets the callback** — it does *not* control word
  wrapping, which is what the name long suggested. It validates the
  address exactly like `NEWSETDESC`'s sixth argument.

## Scaling

`SD%SHR` scales the current descriptor (`SDH%SHR`/`SDV%SHR` per axis)
in **thousandths**: 1000 is unity, 2000 doubles. The title draws sprite
86 (320×200) at 2000, centered on (320, 240) — corner (0, 40), filling
the 640×400 main screen. Scaling is nearest-neighbor: pixels are
palette indices, and a blend of two indices would be meaningless.

## Removing descriptors — KILLNDESC kills a tail

`KILLNDESC ( handle -- )` takes a **descriptor** (not a screen). It
finds the handle's index in its screen's descriptor list (count at
screen +0x418) and kills **that descriptor and every later one on the
same screen** — always at the same index, because `KILLDESC` shifts the
rest down. `INCLLOC` uses it as the location teardown: `_BG @
KILLNDESC` removes everything from the scene's background onward, while
the system descriptors created earlier (captions, info line, inventory
bar — and the title image, sprite 86, which legitimately survives at
level 1 under every background) stay. A handle that matches nothing
does nothing.

## Other descriptor words

Setters whose fields are identified but whose runtime effect is not yet
established: `SDBUF`, `SDSTARTLINE`, `SDALINES`, `SDINSERT`
(3 arguments). `SDTRANS`/`SDSHADE` select palette-lookup effects whose
tables are known (see [Blocks](../formats/blocks.md)) but whose
per-descriptor semantics are not. `SDNORM`/`SDPOS` switch animation
modes (unmapped). `GDCOL` returns the color **undivided** — a backed
text answers 421, not 165 (see [Text rendering](text-rendering.md)).

## Open questions

- The meaning of `NEWSETDESC`'s fifth argument.
- The third callback-address check (`0x68367`).
- **Whether a descriptor's type is stored at all.** The table above reads
  `+0x02` as a type field, and no handler on this page cites writing or
  reading it. The same reading of the 16-bit engine did not survive contact
  with its handlers: there the pair `+0x10`/`+0x12` says what a descriptor is,
  it is worked out on every ask rather than stored, and the earlier
  three-field model was an artifact of reading the two fields the other way
  round ([16-bit descriptors](../../motion16/engine/descriptors.md)). Whether
  the 32-bit engine keeps a discriminant that the 16-bit one does not is open
  until `SDSPR` (`0x71715`), `SDBL`, `SDTXT` (`0x71b4f`) and `SDTB`
  (`0x71d45`) are read for what they write there.
- The animation state inside the sprite payload.
- `SDBUF`, `SDSTARTLINE`, `SDALINES`, `SDTRANS`, `SDSHADE`, `SDINSERT`
  semantics.
- What flag `0x10` picks between — see [Screens](screens.md#open-questions).

## See also

- [Screens](screens.md), [Text rendering](text-rendering.md)
- [Interaction machine](interaction.md) — who clicks on all this
- [GFX8 sprites](../formats/sprites.md)
- [Kernel words](../vm/kernel-words.md)
