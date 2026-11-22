[← Documentation index](../README.md)

# Objects

*PARSEC — Dunkle Schatten (1994): `XP01.EXE`, as its unpacked load image of 150 096 bytes shows it. Addresses `0x…` are linear offsets into that image; `seg:off` names a routine by its far address.*

What a room is made of, besides its tiles, is a set of 32-byte object
records: the doors, the sleeper in his bed, the mother at the stove, the
things that can be taken. They sit inside the loaded room, in the layers
of its picture, and the program rewrites them in place — the animator
counts a frame into `+9` (`0x08971`), the entry pass writes a state into
`+12h` (`0x14E3A`), a hide clears bit 0 of `+2` (`0x157C1`). Every
routine that shows, hides or binds one lives in segment `14DDh`; the
lookup by id is `0BB6h:052Bh` (`0x0C08B`).

## Where the records sit

A room's picture is a list of 20-byte layer records
([rooms](../formats/rooms.md)), and each layer names three things: its
tiles, its own 32-byte objects (`+0Ch`, counted at `+2`), and a chain of
walking records (`+10h`). The redraw `0BB6h:027Ah` (`0x0BDDA`) takes the
layers in order and draws, in each, the tiles, then the objects, then the
chain. The lookup `0BB6h:052Bh` walks the same layers and answers the
first object whose id at `+13h` is the one asked for (`0x0C0E1`).

## What a record holds

| Offset | Field |
|---|---|
| `+0` | the kind: 4 a door, 8 a walking figure; 0, 1 and 5 the kinds a location remembers |
| `+1` | set by the loader when `+15h` is above 1, and by a binding; see [the live byte](#the-live-byte) |
| `+2` | flags: bit 0 live, bit 1 hidden by its animation (a frame of `FFh`), bit 6 drawn mirrored (`0x0BB79`) |
| `+3` | the anchor: 1 centers the cel on the point, 2 centers it and stands it on the point, anything else draws from the point |
| `+4`, `+6` | the point |
| `+8` | which loaded file the cel counts in: 0 the room's own table, 1 the player's cels (`0x10AA0`) |
| `+9` | the frame, a cel counted from `+0Ch` |
| `+0Ah` | the cel drawn, `+0Ch` plus the frame (`0x0956B`) |
| `+0Ch` | the cel the frames count from |
| `+0Eh`, `+10h` | the scale on each axis, `100h` and 0 both one |
| `+12h` | the state, 0 for a still object |
| `+13h` | the id |
| `+15h` | how many cels the object's block holds |
| `+16h` | the ticks until the animator looks at it again |
| `+18h` | the `AN` entry it is bound to — and, before anything binds it, the defaults an object map takes (`+18h`–`+1Fh`) |

The draw pass draws a record only when `(+2 & 3) == 1`: live and not
hidden (`0x0BFC2`). The object drawer takes `+8` four bytes at a time into
the far-pointer table at `DS:2879h` and indexes what that names with
`+0Ah` (`0x08868`); the scale multiplies the cel's size and shifts eight,
and is skipped at 0 and `100h`.

The state at `+12h` says who moves the object. A record bound to an `AN`
entry of its own is state 7; the head of a chain is 8 and its parts 9;
the figure, while an animation of its own runs, `0Ah` (`0x14F9A`); a
kind-8 record would take 3 (`0x14F3D`), and none stands in the game's
rooms. The [animator](animator.md) runs them.

## A location's own objects

Each row of the location table lists nine object ids from `+19h`. The
entry function hands each to the lookup and puts what it finds into the
next subject slot, so list entry `i` lands in slot `i + 1` and slot 0
stays the figure's (`0x0EE01`, `0x0EE53`). The list is short because it is
not the room's furniture: it is what the location's scenes speak about —
see [tasks](tasks.md).

## Arriving: what a visit remembers

On the way out of a location `14DDh:068Ah` writes one byte per object of
kinds 0, 1 and 5, in the layers' order, into the location's own 32-byte
record at `DS:1F56h + 20h × [1F4Ch]` (`0x9EF`): for kind 0 its live bit
plus one (`0x154B8`); for kinds 1 and 5 a 1 when it is not live
(`0x154CE`), a 2 when it is hidden (`0x154D5`), and its frame plus three
otherwise (`0x154EA`). The sixty-one records are zero in the image
(`0x23E46`), which is what leaves every object as the file has it on a
first visit.

The entry pass `14DDh:0293h` reads them back before the handler runs, in
two halves. The animation half (`0x150C6`–`0x151A5`) binds every entry of
the room's `AN` block that names a room object: in a chain, the first
node whose `+2` has a zero low byte while no head is bound yet becomes the
head, in state 8 (`0x15108`), and every other node is a part, in state 9
(`0x15142`); an entry without a chain binds the object itself, in state 7
(`0x15170`). The object half (`0x151E5`–`0x1530A`) puts back what the
location's record remembers — for kind 0 a remembered 1 or 2 is the flags
byte plus one (`0x15218`); for kinds 1 and 5 a 1 is not live, a 2 live
and hidden, and anything above live on the frame the byte names less
three (`0x15262`) — and settles the state. A zero byte leaves the object
as the file has it.

The entry function also opens the door the player came through: an entry
id between 100 and 109 whose object is of kind 4 is a door, remembered in
`[1EDEh]` and shown on frame 1 by `14DDh:0A06h` (`0x157F1`).

## Showing, hiding, and the hotspot that follows

| Routine | What it does |
|---|---|
| `14DDh:084Ah(id)` | the object shown — its live bit set, and for the head of a chain every part's, hidden until the animator places them (`0x15687`) |
| `14DDh:096Bh(id)` | the object hidden — its live bit cleared, and a chain's parts with it (`0x15791`, `0x157C1`) |
| `14DDh:0A06h(id, k)` | the object shown on frame `k` (`0x157F1`) |
| `0883h:03EBh(obj, k)` | frame `k` on an object, hidden for a `k` below zero (`0x08C28`); a state-7 object's entry takes the frame too (`0x08C4D`) |
| `14DDh:074Bh(obj)` | the hotspot whose verb mask ends in the object's id takes the object's live bit as its own (`0x155B6`) |

The last is what keeps a room's hotspots in step with its objects: the
key is the id with bit 15 set (`0x15529`), matched against a mask's id and
bit 15 (`0x15572`), and a live object makes its hotspot active, bit 0 of
`+4` (`0x15586`). Objects of kinds 1 to 4 and 9 have no hotspot to move
(`0x1553C`).

## The draw chain

A walking record — the figure, or one of the room's people — hangs in the
chain of one layer, headed at `+10h` of the layer record and linked
through `+20h` of the records. `0A9Fh:015Ah` puts a record at the **head**
of the chain its `+2Ah` names (`0x0AB8E`–`0x0AB9F`) and `0A9Fh:002Fh`
takes it out (`0x0AA1F`): a head is replaced with its successor
(`0x0AA78`), anything else is bypassed in its predecessor's next link
(`0x0AB1A`). The pass, the animator and the drawer all start at the head,
so the record linked last is drawn first, under the others. A record that
hangs in no chain is not drawn at all.

`0A9Fh:0F42h` (`0x0B932`) is how a location takes the figure out of the
picture without taking it out of the room: it stops the figure's
animation (`0x0B94C`), marks its old tiles (`0x0B95A`) and removes the
chain link, keeping node, position and walk. Insertion is the separate
routine at `0x0AB4A`, which the snap calls (`0x0B7E2`). The Zimmer uses
the pair while Karsten sleeps: the bed's animation stands for him, and
the standing figure is out of the chain until he wakes.

## The forest and the maze

The Wald, row 23, and the thirty maze rows 30 to 59 are of kind 3, and a
location of kind 3 does not show its own slot. The room loader
`0766h:010Ah` reads the row's kind (`0x077ED`), compares it with 3, and
takes slot 20, the forest picture, for it; any other row shows the slot
with its own number (`0x077FC`). The picture, the walk graph, the
animations and the dialogue bank all come from that slot. The row's own
slot is loaded beside it (`0x7B6D`) for one resource: its
[object map](../formats/object-map.md), which tells the loader what to
make of the picture's records for this row.

**Binding** (`0x7BFB`–`0x830C`). For a forest row the loader first writes
the row's exits into the map (`0x7CA7`–`0x7D7D`): entry `i` of the first
four takes the id of the row's exit `i + 1`; with no such exit it hides,
and entry `i + 8` takes the id 2000; with one, entry `i + 4` hides under
the id 0 and entry `i + 8` takes the exit's id plus 2000. So the
picture's exits 101 to 104, their nodes and the twins at 2101 to 2104
take the ids the row's own exits have, which is how one picture is thirty
places with different ways out.

Then every record of every layer — the layers in order, each layer's
records from last to first — binds the first entry not yet bound whose id
is the record's (`0x7DB6`–`0x7DF5`). An entry whose new id is `FFh` takes
the record's defaults from `+18h`–`+1Fh` first (`0x7DFC`–`0x7E6D`). The
record takes the new id (`0x7EA4`); a cel below zero hides it, `FFh` sets
its state to 3, any other is the frame it shows from its base
(`0x7EC0`–`0x7F12`); the offset moves it (`0x7F28`, `0x7F3D`). Whether or
not it bound, every record leaves with its defaults cleared and a state
of 7, 8 or 9 back at 0 (`0x7F5B`–`0x7F92`), for the entry pass to bind
afresh.

The map's flags word then renames the rest of the picture, each kind of
record binding entries of its own:

| Bit | What it renames |
|---|---|
| 0 | the hotspots (`0x7FB5`): the low byte of the mask at `+10h` cleared and the new id ored in (`0x8039`, `0x8049`), a cel of zero or more active and one below zero inactive (`0x8050`–`0x8064`), the rectangle moved by the offset (`0x8073`) |
| 5 | every node of every walk graph whose id is above zero (`0x811E`, `0x8174`), which takes the new id (`0x81BA`) |
| 4 | every `AN` entry without a chain (`0x826A`, `0x8296`), which takes the new id as the object it animates (`0x82E5`) |

A bit left clear takes that resource from the row's own slot instead
(`0x80A2`, `0x81E9`, `0x8310`), which holds none, and the picture's
stays as it is. Every map the game ships has the flags `3Fh`.

**From one forest row to the next** the picture stays loaded (`0x7806`).
The slot release `0766h:0D34h` keeps the forest for the next row
(`0x83A3`–`0x83FB`) and, instead of dropping the records, gives back what
the binding took (`0x86CE`–`0x87F6`): an object takes the entry as its
defaults (`0x8712`–`0x8751`), its own id back (`0x875D`) and its offset
off (`0x876B`); a hotspot its id back (`0x879B`) and its offset off
(`0x87AF`); a node and an `AN` entry their ids back (`0x87D2`,
`0x87E7`). The active bits, the objects' flags and frames stay as the
visit left them, and the next row's map binds onto the records as they
are. What a set map bit names is kept rather than released — the
hotspots (bit 0, `0x8433`), the dialogue bank (bit 1, `0x8547`), the
samples (bit 2, `0x85D4`), the song (bit 3, `0x85ED`), the animation
entries (bit 4, `0x8592`), the walk graph (bit 5, `0x84FC`). The song is
the one a player hears: with bit 3 set the release stops nothing, and a
map with bit 3 clear would put the row's own song in its place
(`0x8339`); every map has it set, so the forest's music plays on from
row to row ([music](music.md)).

The maze's own layout is the handler's, not the loader's. The maze keeps
a byte per location at `1FFC:0482h + n`, outside `DGROUP`; on the first
visit — row 30's byte still the image's `58h` — the handler `04B0:0223`
fills it with each maze row's distance from the three ways out, rows 57,
30 and 44, then overwrites it row by row with the exit that leads one
step closer (`04B0:013D`), and the maze's rule walks the player through
that exit.

## The live byte

The byte at `+6` of a map entry goes into the object's `+1` (`0x7EB9`),
beside the loader's own `+1` from `+15h`. One routine reads it,
`0883h:000Fh`: a visible record that stands still on a single cel, with
`+1` clear, has its rectangle left off the list of dirty rectangles the
screen update copies (`0x884D`). It is a saving in what is redrawn, not a
change in what is seen.

## See also

- [Rooms](../formats/rooms.md) — the layer records, the tiles and the objects in the file
- [The object map](../formats/object-map.md) — the `OMP` resource of the forest rows
- [The animator](animator.md) — what moves a bound object
- [Walking](walking.md) — the figure and the people in the chains
- [Tasks](tasks.md) — the subject slots the location's object list fills
