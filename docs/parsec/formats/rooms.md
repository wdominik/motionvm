[← Documentation index](../README.md)

# Rooms — `DSROOM.DAT` and `RDV1.00`

*PARSEC — Dunkle Schatten (1994): the room container, the six resources of a slot, and the tiled playfield, as `XP01.EXE`'s room loader and redraw read them.*

`DSROOM.DAT` is an [FGROUP](containers.md) of **60 slots**. Slot 0 is the
`NULLPTR` marker; 28 slots carry a playfield. A location number is a slot
number: every slot that holds a playfield has a row in the program's
location table, and the room loader `0766h:010Ah` takes the slot by the
row's number. The exception is the row's kind: a row of kind 3 — the Wald
and the thirty maze rows 30 to 59 — takes **slot 20**, the forest picture
(`0x077ED`–`0x077FC`), and loads its own slot beside it for its
[object map](object-map.md) (`0x7B6D`). What the loader does with the two is
on the [objects page](../engine/objects.md).

## A slot is six resources

A live slot is a RIB of six resources, in an order the program names in its
data segment — `NOD ANI OMP DLG MBX RDO` at `DS:054Bh`. Any of the six may
be the `NULLPTR` literal instead.

| Index | Name | What it is | Kept at |
|---:|---|---|---|
| 0 | `NOD` | The walk graphs | `[2865h]` |
| 1 | `ANI` | The room's [`AN` block](animations.md), relocated by `0563h:0147h` (`0x7B4E`) | `[286Dh]` |
| 2 | `OMP` | An [object map](object-map.md) — 31 slots carry one | |
| 3 | `DLG` | The room's [dialogue bank](dialogue.md), the fourth of the six far pointers (`0x7B0D`, `0x7B14`) | `[2855h]`, its count at `[1EEAh]` |
| 4 | `MBX` | The [hotspot table](hotspots.md) — 20 slots carry one | |
| 5 | `RDO` | The `RDV1.00` room below | `[2851h]` its header |

## The `RDV1.00` room

```text
"RDV1.00\0"
u8 start, u8 count   # 00 80 in every room: DAC 0, 128 colors
u8 rgb[count][3]     # six-bit
# tile cels, object records, layer directories
# trailer, 36 bytes from the end
```

The trailer opens with the viewport, 320×128; the tile size, 16×16; and the
map size, 20×8 — so a filled map is the whole playfield — followed by the
layer count, 1 to 6 in Dunkle Schatten's rooms. The loader takes the room
header from those last `24h` bytes (`0x789B`). The resource's last twelve
bytes are three `u32`s; the first of them, `pA`, points at the layer
directories, one 20-byte record per layer:

| Offset | Field |
|---|---|
| `+0` | `u16 kind` — 1 or 0 here; 0 draws no tiles |
| `+2` | `u16 count` — this layer's 32-byte objects |
| `+4` | The map: 160 cells of one-based tile indices, 0 empty |
| `+8` | The cel-offset table the map indexes |
| `+0Ch` | This layer's objects, 0 when it has none |
| `+10h` | The head of the layer's chain of walking records — 0 in every file; `0A9Fh:015Ah` puts a record in |

The offsets are file-relative on disk and far pointers once loaded.

**A tile is its table's entry `k − 1`, and nothing bounds it.** The redraw
`0BB6h:027Ah` switches on the record's kind (`0x0BE8E`–`0x0BE9B`: 1 a map of
bytes, 2 a map of words, anything else no tiles), takes the table pointer
less four (`0x0BE7A`) and adds four times the map value
(`0x0BEDB`–`0x0BEE9`). There is no end to the table and nothing is scanned
for. Every layer draws with index 0 transparent.

**Each layer draws three things, in this order.** The redraw (`0x0BDDA`)
takes the layers in directory order and in each draws its tiles
(`0x0BE2B`–`0x0BFA0`), then its objects (`0x0BFA2`–`0x0C008`), then its
chain, from the head at `+10h` along each record's `+20h`
(`0x0C00A`–`0x0C074`). An object or a chain record is drawn only when its
`(+2 & 3) == 1`. A walking figure is so drawn over its own layer's tiles
and objects and under every layer after it.

**The room's cel tables are one table.** In all 28 rooms the resource closes
with, in this order: one run of `u32` cel offsets from `pB`, the layers'
objects, the directories at `pA`, and the trailer. A layer's table pointer
points into that run, and the run ends where the first object record starts
— a measurement, not something the program reads. Entry 0 of the run is
offset 8, the palette.

### Objects

The layers' 32-byte object records are the room's furniture that is not a
tile: doors, sleepers, people's heads and bodies, everything the animator
moves. The object lookup `0BB6h:052Bh` (`0x0C08B`) walks the layer records
and the objects inside each, by id.

| Offset | Field |
|---|---|
| `+0` | `u16` kind — 4 a door, as the entry gate wants it |
| `+2` | `u8` flags — `80h` set on arrival; the low two bits the anchor state |
| `+3` | `u8` anchor — 1 centers the box on the position, 2 stands it on the point |
| `+4`, `+6` | `u16` x, y |
| `+8` | `u8` bundle — which loaded file `+0Ah` indexes: 0 the room's own cels, 1 the player's |
| `+9` | `u8` the frame |
| `+0Ah` | `u16` the cel shown — `+0Ch` plus the frame (`0x0956B`) |
| `+0Ch` | `u16` the cel the frames count from |
| `+0Eh`, `+10h` | `u16` x and y scale, `100h` being one; 0 and `100h` skip the multiply |
| `+13h` | `u8` the object's id, the lookup's key (`0x0C0E1`) |
| `+15h` | `u8` — the loader writes 1 into `+1` when it is above 1, else 0 |
| `+18h`–`+1Fh` | An object map's defaults: new id, live byte, cel, `dx`, `dy` (`0x7E04`–`0x7E6D`) |

The bundle byte indexes a far-pointer table at `DS:2879h` four bytes at a
time, and `+0Ah` indexes what that names (`0x08868`, `0x0888C`).

### People

A room in which someone besides the player walks carries a table of
`30h`-byte records (`imul ax, 30h` at `0x7988`), named by the trailer: its
`+0Eh` is the count and `+14h` the file offset (`0x7972`). Nine slots carry
one — the same nine whose walk-graph resource holds a second graph.

| Offset | Field |
|---|---|
| `+0` | `u8` kind — 3 the player; 6, 7, `10h`, `11h` people; 8 a special kind |
| `+2` | `u8` flags — bit 0 live, `40h` mirrored |
| `+3` | `u8` anchor |
| `+4`, `+6` | `u16` x, y |
| `+8`, `+9`, `+0Ah`, `+0Ch` | Bundle, frame, cel and base, as an object's |
| `+0Eh`, `+10h` | Scale |
| `+12h` | `u8` state — 0 standing, 6 walking |
| `+13h` | `u8` id |
| `+14h` | `u8` interval — ticks per walk step, and the cost of a leg |
| `+15h` | `u8` walk steps per direction |
| `+20h` | Next in the layer chain; from here on the walker's own run-time state |
| `+28h`, `+2Ah` | The node it stands on and its layer, before anything puts it on a graph |

## The walk graphs, resource 0

The resource opens with a word counting graph headers; `0724h:0000h`
relocates that many, `2Eh` bytes apart from `+4` (`0x7259`, `0x726A`). Most
rooms hold one graph, the player's; the nine rooms with a person in them
hold two, and graph 1 is the person's.

A header's `+0` is the node count, `+2` the edge count, `+4` and `+8` the
offsets of the two arrays of 16-byte records, and `+10h` a mode word: the
snap `0A9Fh:0C59h` sets it to 0 (`0x0B7D0`), and until then the walk
routines read the graph as "no walk" (`0x0B0A6`–`0x0B0BB`).

A **node**, as the snap copies it into a walker (`0x0B738`–`0x0B7B4`):

| Offset | Field |
|---|---|
| `+0`, `+2` | `u16` x, y of the feet |
| `+4` | `u16` the figure's scale there, `100h` one (`0x0B77C`) |
| `+6` | `u8` the layer the figure hangs in |
| `+8` | `u16` id — a door's or object's, negative for an anonymous waypoint, or an entry twin `id + 7D0h` |
| `+0Ah` | The first edge of the node's circular adjacency chain |

An **edge**:

| Offset | Field |
|---|---|
| `+0`, `+2` | The two endpoints, as node indices |
| `+4`, `+6` | The next edge in each endpoint's chain |
| `+8` | Joined to the length by `or` into the flood's edge cost (`0x0AD45`) |
| `+0Ah` | The length; a leg's step is worked out from it and the walker's interval (`0x0AF0B`) |
| `+0Ch` | The axis word: non-zero picks the walker's direction by the sign of its y travel, zero by its x travel (`0x0AFCE`) |
| `+0Eh` | The layer while on this edge |

The axis word is the room's choice, not the edge's slope: of the 559 edges
in the game's rooms, 193 are non-zero, 57 of them at least as wide as they
are tall, and 5 of the 366 zero ones are taller than they are wide.

## See also

- [The object map](object-map.md) — what a forest or maze row renames in slot 20's picture
- [Hotspots](hotspots.md), [Dialogue](dialogue.md), [Animations](animations.md) — resources 4, 3 and 1
- [Graphics](graphics.md) — cels and palette banks
- [Walking](../engine/walking.md) — how a figure moves through the graph
