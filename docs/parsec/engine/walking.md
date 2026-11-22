[← Documentation index](../README.md)

# Walking

*PARSEC — Dunkle Schatten (1994): `XP01.EXE`, as its unpacked load image of 150 096 bytes shows it. Addresses `0x…` are linear offsets into that image; `seg:off` names a routine by its far address.*

A figure walks on a graph, resource 0 of the room's slot
([rooms](../formats/rooms.md)): nodes with a point, a scale and a layer,
and edges between them with a length, a blocked bit and the layer the
walk is drawn in. The routines are segment `0A9Fh` — the flood, the leg
setup `0A9Fh:0404h`, the snap `0A9Fh:0C59h`, the walk call `0A9Fh:0617h`
and the mover pass `0A9Fh:0892h` — with the interpolation itself in
`0724h:0035h`. The player and the room's people walk through the same
routines.

## The walker is an object

A walker is not a thing beside the objects: it is a 30h-byte record whose
first 32 bytes read as an [object](objects.md) record. The program keeps
two in its data segment. The player's, at `DS:1693h`, is kind 3, live,
anchored 2 — centered on `x`, standing on `y` — at (100, 100), drawing
cel 0 out of bundle 1, the figure file, at scale `100h`, with id `0FFh`;
the second, at `DS:16C3h`, is the same with bundle 2 and id `0FEh`. The
rest of the record is the walk: the node under the figure (`+28h`,
`FFFFh` while moving), the layer it names (`+2Ah`), the direction block
(`+2Ch`) and the facing to take on arrival (`+2Eh`).

The last two bytes of the object part are the **gait**. `+14h` is the
interval, which does two jobs: the leg setup multiplies an edge's length
by it (`0x0AF0B`–`0x0AF10`), and the animator puts it back on the
countdown each time the walk cycle steps (`0x08AF7`) — so the gait is tied
to the speed, one cel every `+14h` ticks. `+15h` is how many walk cels a
direction block has (`0x0ABE2`); the block holds one more, the standing
one. The player's record ships with an interval of 13 and six cels.

A room's people are 30h-byte records in the room file itself. The loader
leaves them in no chain, so they are neither moved nor drawn until the
location binds one to a walk graph with `0A9Fh:0EA6h`; from then on the
mover pass, the animator and the drawer run them as they run the player.
A room with a person in it carries two graphs, and graph 1 is the
person's.

## The flood

A walk starts with a flood out of the destination (`0x0AD23`): every node
is reset to `FFFFh`, the destination set to zero, and the distance spread
along the edges. The cost of an edge from distance `d` is `(blocked |
length) + d + 10`, saturating (`0x0AD45`–`0x0AD58`); a node improves only
when its distance is strictly larger, remembers the edge it came by, and
floods on. The recursion follows the edges' own chain order, so equal
costs resolve the same way every time. Afterwards every node knows the
edge that leads one step closer.

## A leg

`0A9Fh:0404h` picks the edge out of the node the figure stands on and
loads the interpolation into the graph header: the from-node's x, y and
scale, twice the difference to the far node, and a parameter `t` that
runs from 0 to `7FFFh` — or backwards, when the edge is walked from its
second end. The step of `t` is `2FFFAh` divided by the edge's length times
the interval (`0x0AF15`, `0x0AF19`): the product is the divisor, so a long
edge takes more ticks than a short one and the walk is evenly paced. The
multiply is sixteen bits (`0x0AF10`), the step the low word of the
quotient (`0x0AF22`), and a step of zero is 1 (`0x0AF2D`). Over the 559
edges of the game's rooms the shortest is 4, the fastest walk the options
offer the player is 2 (`DS:14E6h`), the second record walks at 10
(`DS:16D7h`) and the people at 12, 15 or 17, so the product is never
small enough to matter.

The leg turns the figure. Its direction code, 3 to 6, comes from the
edge's axis word (`0x0AFCE`): a non-zero one turns it by the sign of the y
difference, a zero one by the x difference's. The word is the room's
choice, not the edge's slope. When the edge names another layer, the
figure moves to that layer's chain (`0x0AFA2`–`0x0AFBD`). The answer is 0
when the node has no way on (`0x0AE1D`), −1 when its edge is blocked
(`0x0AE34`), otherwise the direction code.

## The pass, and the animator a step behind

The mover pass `0A9Fh:0892h` runs once a frame with the ticks since the
last. In mode 2, walking, `0724h:0035h` moves `t` and writes the point it
names into the graph header's `+28h`, `+2Ah`, `+2Ch` (`0x072B3`) — not
into the record. At the end of a leg the record stands on the node
reached and the next leg starts; when there is none the mode becomes 1
(`0x0B4E9`), and a blocked edge makes the arrival face front
(`0x0B4DF`). In mode 1, on the next pass, the record takes the node's id
(`0x0B36E`), the chain of the node's layer when it differs, mode 0, and
stands — on `+0Ch` for kinds 6 and 10h, on the block's last cel for the
others (`0x0B3AC`–`0x0B3E6`) — with the kept facing (`0x0B3F3`). So
`+28h` stays `FFFFh` for one pass after the last leg ends.

The record itself follows the header only when the animator steps the
walk cycle: `0883h:0305h` takes a walking record's countdown down and,
for every interval it covers, `0A9Fh:0B4Ch` copies the header's point
into the record (`0x0B593`–`0x0B5C3`), the cycle goes round
(`0x0899A`–`0x089C4`) and the interval goes back on. The drawn figure
moves once a walk step, every `+14h` ticks, while `t` moves every tick.
The same copy plays the footsteps: `0EB9h:1949h(0)` at walk step 1 and
`(1)` at step 4 (`0x0B5DC`), for every kind but 8 (`0x0B5D6`).

Run as the original's own routines under an 8086 emulator, on the game's
records and graphs — the Laden's person walking from node 12Ch to 12Dh,
the player over two legs through a waypoint — these two halves leave the
record described here, tick by tick.

## Turning

What a direction code does to a record is its kind's (`0A9Fh:02CFh`,
through the table at `cs:0315h`, `0x0ACD9`):

| Kind | What a direction does |
|---|---|
| 3, the player | the block of the facing `code − 3`, the facing times `+15h + 1`; below 3, turned round — the facing read out of `+2Ch` and xored with 2 (`0x0ABCB`) |
| 6, 7 | no block: codes 3 and 4 mirror off, 5 and 6 on, any other toggles bit 40h |
| 10h, 11h | the tiles marked, nothing else |

For kind 3 a block that is already `+2Ch`'s changes nothing
(`0x0ABF1`); otherwise the cel and its base move by the difference, with
the mirror set when the new block is past the second and cleared when the
old one was (`0x0AC02`–`0x0AC30`), so facing 3 is block 1 mirrored.

## Putting a figure down

The snap `0A9Fh:0C59h` puts a record on the node with a given id: it
leaves its chain first (`0x0B722`), copies the node's point and scale
(`0x0B738`–`0x0B819`), and links into the chain of the node's layer
(`0x0B7E2`). A node the graph does not have leaves the record in no chain,
undrawn. The snap also sets the header's mode to 0 (`0x0B7D0`); until a
snap has, the file's `FFFFh` makes `0A9Fh:0617h` refuse to walk
(`0x0B0A6`–`0x0B0BB`).

Where an entry puts the player is tried in the order `0x0EF4B` tries it,
the first id that names a node winning: the entry plus `7D0h` — the twin a
room carries when it wants the player to walk in — then the entry itself,
then node `7Fh`, and last the first exit of the location's own row
(`0x0EFF3`). With the twin, the figure is snapped onto it (`0x0EF57`) and
walks to the node with the plain id (`0x0EF6F`). When none of the four
names a node the record stays out of every chain, which is a room without
the player in it, as the Karte is.

## Asking for a walk

`0A9Fh:0617h(rec, id, facing)` is what the location logic and the mode
machine call. Standing on the node already, the figure turns to facing
+ 3 and the answer is 2 (`0x0B03B`); an id no node has answers 0
(`0x0B08E`). Otherwise the facing is kept at `+2Eh` (`0x0B09E`) and the
mode decides (`0x0B0A6`): 0 or 1 is a fresh walk, with the flood and the
first leg; 2 is a retarget (`0x0B13D`).

A retarget does not abandon the leg the figure is on: it keeps it or
walks it backwards. When the new destination is one of the current
edge's ends, both ends' edges are cleared so that arriving there ends the
walk; otherwise the flood runs again from the new destination and the end
it favors decides the direction. Turning round negates the step of `t`
and turns the figure (`0x0B26F`); keeping the leg answers 1 (`0x0B25C`).

## See also

- [Rooms](../formats/rooms.md) — the walk graph in the slot
- [Objects](objects.md) — the record a walker is, and the chains
- [The animator](animator.md) — what times the walk cycle
- [Verbs](verbs.md) — the mode machine that walks the figure to what was clicked
