[← Documentation index](../../README.md)

# BLOCK — Mixed Binary Data

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

The BLOCK resource type is a catch-all for untyped binary data. Despite
often being described as the "music" segment, only a minority of blocks are
music: of the 250 blocks present, **26 carry the signature
`HMI-MIDISONG061595`** ([HMI songs](hmi.md)) and the remaining **224 are
plain data** used by the game logic.

Blocks have no header of their own; their meaning comes entirely from the
code that loads them. The kernel word

```
GET ( size addr id -- )
```

copies block `id` into module memory at `addr` (a packed address, see
[Execution model](../vm/execution-model.md)), checking against the
expected `size`.

## Id layout

The occupied ids mirror the module numbering (see
[Module map](../../games/ds2/module-map.md)):

| Ids | Count | Size | Contents |
|---|---|---|---|
| 0–25 | 26 | 4–100 KB | [HMI songs](hmi.md) |
| 50 | 1 | 512 KB | Translucency tables (see below) |
| 51 | 1 | 64 KB | 256-level shade table (see below) |
| 52–54 | 3 | 8 KB | 32-level shade tables (see below) |
| 99 | 1 | 200 B | Location table (see below) |
| 101–123, 130 | 24 | 904 B | Walking routes for location *id−100* |
| 199 | 1 | 516 B | A byte-identical copy of `000.FRT` (the [font reference table](font-reference-table.md)) |
| 201–223, 230 | 24 | 600 B | Extended routes for location *id−200* |
| 301–323, 330 | 24 | 200 B | Click areas for location *id−300* |
| 401–423, 430 | 24 | 2240 B | Item records for location *id−400* |
| 450–570 | 121 | 64 B–2.8 KB | Dialogue definitions |

## Translucency and shade tables (50–54)

These blocks feed the engine's palette-space blending, installed by the
kernel words `SETTRANS` and `SETSHADE`:

- **Block 50 — `SETTRANS` format**: `u32 count` (8 here) followed by
  `count` tables of 65,536 bytes. Each table is a full
  `result = T[a][b]` lookup blending two palette indices; the diagonal is
  the identity, the tables are not symmetric (directional mixes), and
  each table covers one opacity band of the 0–255 range.
- **Blocks 52–54 — `SETSHADE` format**: exactly 8192 bytes =
  **32 shade levels × 256 palette entries**, no header. Row 0 is
  near-identity; higher rows darken progressively. The three blocks are
  alternative ramps (54 darkens most gently).
- **Block 51** is a 256-level variant of the same shade idea: its first
  8192 bytes are byte-identical to block 52, continuing to 256 rows ×
  256 entries, plus 4 trailing bytes (padding; hypothesis).

Because pixels are palette indices, translucency and shading must be
table lookups — arithmetic on indices would be meaningless. The
descriptor words `SDTRANS` and `SDSHADE` presumably select these effects
per descriptor. No shipped script calls `SETTRANS`/`SETSHADE`; the tables
are installed by engine startup or the standalone script modules.

## Block 99 — the location table

Fifty packed 32-bit addresses (200 bytes) of the form
`(module << 16) | byte offset`, each pointing at the body of a location's
scene-macro word. The table is indexed with **1-based** location numbers:
the entry for location *N* sits at offset `4·(N−1)`, and the location
loader executes it as `(N−1)·4 + table @ EXECUTE`.

Valid entries exist for locations 1–11 and 13–23, each resolving to module
*300+N* at byte offset `0x30` (the module's first word). The remaining
entries contain uninitialized garbage — including the slot for location 12,
even though module 312 exists and defines a scene macro. Entry 30 points at
module 323 (the title) a second time; module 330's scene macro is not
referenced by any valid entry.

## The per-location data blocks

The location loader `INCLLOC` copies four blocks into module-2 arrays when
entering location *N* (sizes are `count × record size`, from the game's own
constants):

| Block | Destination | Layout |
|---|---|---|
| 100+N | `_ROUTE` | 4-byte record count + 25 route records × 36 bytes: `x0, y0, x1, y1` (a rectangle's corners or a line's ends) and then up to five neighboring route indices, the list ending at the first −1. All figures in a location share the one table |
| 200+N | `_XROUTE` | 25 extended-route records × 24 bytes, same index: Z, kind (0 rectangle, 1/2 line), the scale at each end, a fixed step size (−1 = use the figure's speeds), and flags |
| 300+N | `_KLICKAREA` | 10 click-area records × 20 bytes, with **no count prefix** — the 200-byte block is exactly `10 × 20`; field `+16` is the route number |
| 400+N | `_LDITEM` | 35 item records × 64 bytes (below) |

### The item record

The game library defines a paired accessor per field
(`->LDX1` writes, `LDX1->` reads), which fixes the 64-byte item record's
layout. Field meanings follow from the accessor names; entries are 32-bit
cells:

| Offset | Field | Reading of the name |
|---|---|---|
| `+0` | `X1` | Click rectangle, left |
| `+4` | `Y1` | top |
| `+8` | `X2` | right |
| `+12` | `Y2` | bottom |
| `+16` | `TEXT` | Name text |
| `+20` | `ITEXT` | Info text |
| `+24` | `DR` | Direction/route selector |
| `+28` | `DX` | Walk-destination x |
| `+32` | `DY` | Walk-destination y |
| `+36` | `EXIT` | Exit target |
| `+40` | `FITEM` | Linked figure/item |
| `+44` | `MX` | Marker/mouse x |
| `+48` | `MY` | Marker/mouse y |
| `+52` | `DIR` | Facing direction |
| `+56` | `ORDER` | Order/priority |
| `+60` | — | (unnamed spare cell) |

The route and extended-route layouts are mapped above, read off `CROUTE`
(0x7780c) and its helpers and confirmed by the block sizes: 904 = 4 + 25 x 36
and 600 = 25 x 24. The click-area record is still only known by its `+16`
route number.

## Dialogue blocks (450–570)

Loaded on demand by the dialogue system (the block id is computed at run
time; the size is taken from the text-table status). The records contain
32-bit values interleaved with NUL-padded ASCII action names such as
`DGIVE` and `DINFO`; the exact record layout has not been mapped.

## Open questions

- Field layout of the click-area record beyond `+16`.
- Why location 12's table entry is stale, and why module 330's scene macro
  is unreferenced.
- Which code installs the translucency/shade blocks (no script calls the
  installer words).

## See also

- [HMI songs](hmi.md)
- [RSC containers](rsc-container.md)
- [Game structure](../../games/ds2/game-structure.md) — the location loader
- [Module map](../../games/ds2/module-map.md)
