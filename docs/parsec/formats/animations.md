[← Documentation index](../README.md)

# Animation Blocks — `AN`

*PARSEC — Dunkle Schatten (1994): the blocks a room's objects and the player's figure animate from, as `XP01.EXE` loads and relocates them.*

What moves in a room besides the player walking — a door swinging, a
sleeper turning over, a mother at the stove — is not in the location's own
code. It is an `AN` block: a live `DSROOM.DAT` slot carries one as its
resource 1, the `ANI` of the six (`[286Dh]`), and `DIALOG.DAT` carries one
more as its resource 2, the block the player's own figure animates from
(`0x0525F` loads it into `[1ED4h]`). The room loader relocates a block with
`0563h:0147h` (`0x7B4E`), the entry pass `14DDh:0293h` binds its entries to
the room's objects, and the animator in segment `0883h` plays them. What
the bytes of an animation *do* is the [animator's page](../engine/animator.md).

## The block

Two magic bytes `AN`, a word, and from `+4` a directory of `u32` offsets
ended by a zero — the walk `0563h:0147h` makes, turning each into a far
pointer. Each offset names an **entry**:

| Offset | Field |
|---|---|
| `+0` | `u16` the offset of a chain of part nodes, 0 for none |
| `+2` | `i16` the object animated; negative is the negated id of a figure (compared at `0x14E9D`) |
| `+4` | `u8` how many animations the entry holds |
| `+0Ch`, `+0Dh` | `u8` the bounds of a random wait |
| `+0Eh` | `u8` the wait a frame takes when its opcode names none |
| `+5`–`+73h` | Otherwise the animator's own working state |
| `+74h` | `"ANIS"`, checked by `14DDh:0293h` (`0x1509D`) and `0883h:1F82h` (`0x0A81D`) |
| `+78h` | `u16 off[count]` — each animation's byte stream, as an offset from the block's start |

**An entry is a program and its variables in one.** The animator keeps its
cursor, its stack of frames and its loop counters inside the entry it plays
(`0x08E1E` writes the cursor to `+68h`, `0x0A253` a frame to `+18h`), which
is what makes an entry `78h` bytes long. On disk every byte from `+5` to
`+73h` is zero in every shipped block, except `+0Ch` to `+0Fh`: the three
waits, and `+0Fh`, which nothing here reads. An animation whose slot is zero
or beyond the count is refused by `0883h:0582h` (`0x08DF4`).

## Chains

An entry whose `+0` is not zero animates a composite: a head object and
parts placed relative to it. A node:

| Offset | Field |
|---|---|
| `+0` | `u16` the next node, 0 at the end |
| `+2` | `u16` kind — low byte 0 the head, otherwise a part's placement; high byte 0 one cel per frame, `100h` eight bytes per frame |
| `+4` | `u16` the object |
| `+6` | far: the object, filled at run time |
| `+0Ah`, `+0Ch` | `i16` a part's offset from the head (high byte 0) |
| `+0Eh` | `u8 cel[frame]` — a part's cel per frame (high byte 0) |
| `+0Ah` | `{kind, cel, x, y, mirror, three unread}` per frame, eight bytes each (high byte 1) |

`0883h:1A82h` reads them that way (`0x0A3F1` the per-frame cel,
`0x0A404`–`0x0A43B` the eight-byte form). The Küche shows it: the mother is
object 200, a head with twelve cels, and object 201 below it, her body with
one.

Every live slot that carries a resource 1 parses this way, and so does
`DIALOG.DAT` resource 2.

## See also

- [The animator](../engine/animator.md) — what an animation's bytes do
- [Rooms](rooms.md) — the objects the entries animate
- [The object map](object-map.md) — how a forest row renames chainless entries
