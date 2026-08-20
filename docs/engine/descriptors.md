[← Documentation index](../README.md)

# Descriptors

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

A descriptor is a **0x3E-byte structure** plus a type-dependent payload.
Known fields:

| Offset | Type | Meaning |
|---|---|---|
| +0x02 | `u16` | Descriptor type: 1 group, 2 sprite, 3 image, 4 text |
| +0x04 / +0x06 | `i16` | X / Y position (interpreted per placement mode) |
| +0x08 | `i16` | Level (draw order) |
| +0x0E | | Group: sibling-list head/tail; text: the layout record |
| +0x12 | `u16` | **Placement modes**: bits 0–1 horizontal, bits 2–3 vertical |
| +0x13 | `u8` | Flags: bit 7 active/visible; bit 3 grouped; screen byte: bit 2 = frozen |
| +0x14 | `u32` | **Completion callback** (set by `SDWORD` or `NEWSETDESC` arg 6) |
| +0x17 | `u8` | Bit 7 = center the text block as a whole (set only by `SDBLK`, which no shipped script calls — see [Text rendering](text-rendering.md#centering)) |
| +0x18 | `i32` | **Wait counter** (set by `SDWAIT`): −1 never, 0 due every frame, n counts down |
| +0x1C | | Parent group |
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
[Text rendering](text-rendering.md) and [Shell](../library/shell.md)).

## The three payload types

A descriptor is exactly one type, set by the word that gives it content:

| Type | Set by | Payload | Meaning |
|---|---|---|---|
| 2 | `SDSPR` | 8 bytes | Sprite with animation state |
| 3 | `SDBL` | 4 bytes | Full image without animation state |
| 4 | `SDTXT` / `SDTB` | 0x5C bytes | Text |

(Type 1 is a group — the screen itself heads the sibling list.)
**Types 2 and 3 index the same GFX8 pool**; the name "block" in `SDBL`
is unrelated to the BLOCK resource type. All ids passed to `SDBL` in
the game (43, 60, 64, 66, 1010) exist as GFX8 sprites.

## Visibility, order, and the frame walk

- The active bit is `0x80` in byte +0x13, set only by
  `SDACTIVE`/`SDINACTIVE` — and **a newly created descriptor is
  visible**.
- Descriptors hang in a **sibling chain per screen**, and insertion
  (`0x6a648`) advances past every entry of the same or lower level
  before linking: the chain is a **stable sort by level**. Draw order
  and the per-frame walk follow the chain; no separate sort exists.
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
tables are known (see [Blocks](../formats/block.md)) but whose
per-descriptor semantics are not. `SDNORM`/`SDPOS` switch animation
modes (unmapped). `GDCOL` returns the color **undivided** — a backed
text answers 421, not 165 (see [Text rendering](text-rendering.md)).

## Open questions

- The meaning of `NEWSETDESC`'s fifth argument.
- The third callback-address check (`0x68367`).
- The animation state inside the type-2 payload.
- `SDBUF`, `SDSTARTLINE`, `SDALINES`, `SDTRANS`, `SDSHADE`, `SDINSERT`
  semantics.

## See also

- [Screens](screens.md), [Text rendering](text-rendering.md)
- [Interaction machine](interaction.md) — who clicks on all this
- [GFX8 sprites](../formats/gfx8-sprites.md)
- [Kernel words](../vm/kernel-words.md)
