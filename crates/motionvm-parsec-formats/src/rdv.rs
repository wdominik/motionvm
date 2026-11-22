//! `RDV1.00` tiled rooms.
//!
//! Resource 5 of every live `DSROOM.DAT` slot is this format; the slot's
//! other five resources are listed at [`RoomFile::from_1994_slot`].
//! Layout:
//!
//! ```text
//! "RDV1.00\0"
//! u8 start, u8 count # `00 80` in every Dunkle Schatten room: DAC 0, 128 colors
//! u8 rgb[count][3]   # 6-bit
//! # … tile cels and other records …
//! # trailer, 36 bytes from EOF, starts:
//! u16 320, 128       # viewport
//! u16 16, 16         # tile size
//! u16 20, 8          # map size
//! ```
//!
//! The last 12 bytes of the resource are three little-endian `u32`s. The
//! first of them (`pA`, 12 bytes from EOF) points at one 20-byte directory
//! per map layer (the layer count is the `u16` after the map size in the
//! trailer: 1 to 6 in Dunkle Schatten's rooms):
//!
//! ```text
//! u16 kind, u16 count
//! u32 map_off        # 160 bytes of 1-based tile indices, 0 = empty
//! u32 table_off      # u32 offsets to cels
//! u32 objects        # this layer's 32-byte objects, 0 when it has none
//! u32 0
//! ```
//!
//! Each cel is the 8-byte picture header (`u16 w, h, flags, 0`) plus pixels.
//! Background tiles are 16×16. The map is 20×8, so a filled room is 320×128 —
//! the adventure playfield; the 72-pixel panel is a separate file.
//!
//! **A tile is its table's entry `k − 1`, and nothing bounds it.** The
//! redraw `0BB6h:027Ah` switches on the record's kind (`0x0BE8E`–`0x0BE9B`:
//! 1 a map of bytes, 2 a map of words, anything else no tiles), takes the
//! table pointer less four (`0x0BE7A`) and adds four times the map value
//! (`0x0BEDB`–`0x0BEE9`). There is no end to the table and nothing is
//! scanned for. Every layer draws with index 0 transparent.
//!
//! **Each layer draws three things, in this order.** The redraw
//! `0BB6h:027Ah` (`0x0BDDA`) takes the layers in directory order and, in
//! each, draws its tiles (`0x0BE2B`–`0x0BFA0`), then its objects, the
//! records `+0Ch` points at (`0x0BFA2`–`0x0C008`), then its chain, from
//! the head at `+10h` along each record's `+20h` (`0x0C00A`–`0x0C074`).
//! An object or a chain record is drawn only when its `(+2 & 3) == 1`.
//!
//! **The room's cel tables are one table.** In all 28 rooms of Dunkle
//! Schatten the resource closes with, in this order, one run of `u32` cel
//! offsets from `pB`, the layers' objects, the directories at `pA` and the
//! trailer: a layer's table pointer points into that run, and the run ends
//! where the first object record starts. That end is a measurement, not something the program
//! reads — it indexes without a bound — and it is how far this reader
//! decodes cels ahead of time.
//!
//! **A directory record is a layer, not just tiles.** Each 20-byte record
//! names its tiles, its own 32-byte objects, and the chain of walking
//! records that hang in that layer; the object lookup `0BB6h:052Bh`
//! (`0x0C08B`) walks the same records and the objects inside each. Read in
//! memory, the record is:
//!
//! ```text
//! +0    u16 kind      # 1 or 0 in Dunkle Schatten's rooms; 0 draws no tiles
//! +2    u16 count     # 32-byte objects in this layer
//! +4    far map
//! +8    far cel-offset table
//! +0Ch  far objects   # the objects sit right behind the cel table
//! +10h  far chain     # walking records, linked through their `+20h`;
//!                     # 0 in every file, `0A9Fh:015Ah` puts a record in
//! ```
//!
//! Non-16×16 sprites are placed by the layers' 32-byte object records
//! ([`Overlay`]), which sit behind the cel table, immediately before `pA`.
//! An object's cel indexes the offset table that starts at `pB` (entry 0 is
//! offset 8, the palette, not a sprite).
//!
//! **The picture is in the room file.** Every live `DSROOM.DAT` slot is a
//! 6-resource RIB whose resource 5 is an `RDV1.00` room.

use crate::error::{Error, Result};
use crate::hotspot::HotspotTable;
use crate::picture::Picture;
use crate::rib::Rib;
use crate::{Record, bytes, records, slice, tail, u16le};

/// Magic at the start of every RDV resource, including the trailing NUL.
pub const MAGIC: &[u8; 8] = b"RDV1.00\0";

/// Playfield width in pixels, from the trailer.
pub const VIEW_W: u16 = 320;
/// Playfield height in pixels, from the trailer.
pub const VIEW_H: u16 = 128;
/// Tile width in pixels, from the trailer.
pub const TILE: u16 = 16;
/// Map width in tiles, from the trailer.
pub const MAP_W: u16 = 20;
/// Map height in tiles, from the trailer.
pub const MAP_H: u16 = 8;
/// Cells in the map, [`MAP_W`] × [`MAP_H`] = 160.
pub const MAP_CELLS: usize = 160;

/// [`TILE`], [`MAP_W`] and [`VIEW_W`] as the sizes the composer cuts the
/// playfield by.
const TILE_PX: usize = 16;
/// See [`TILE_PX`].
const MAP_ROW: usize = 20;
/// See [`TILE_PX`].
const VIEW_ROW: usize = 320;
/// One row of tiles across the playfield, in pixels: `VIEW_ROW × TILE_PX`.
const TILE_BAND: usize = 320 * 16;
/// Pixels in the playfield, [`VIEW_W`] × [`VIEW_H`].
const VIEW_PIXELS: usize = 320 * 128;

/// One layer directory record.
#[derive(Debug, Clone)]
pub struct RoomLayer {
    /// Record `+0`. 1 or 0 in Dunkle Schatten's rooms; a record of kind 0
    /// has no map and draws no tiles.
    pub kind: u16,
    /// 160 map cells of this layer, 1-based into [`Self::cels`].
    pub cells: [u8; MAP_CELLS],
    /// This layer's cels, in table order.
    pub cels: Vec<Picture>,
    /// This layer's tiles alone, index 0 left transparent. Empty when the
    /// record draws no tiles.
    pub pixels: Vec<u8>,
}

/// One `RDV1.00` room: palette, composed playfield, raw tiles, map, trailer.
#[derive(Debug, Clone)]
pub struct Room {
    /// The header's DAC bank, `palette_count` 6-bit triples written from
    /// register `palette_start` (Dunkle Schatten: 128 colors at index 0).
    pub palette_rgb: Vec<u8>,
    /// First DAC register declared by the palette resource at offset 8.
    pub palette_start: u8,
    /// Number of RGB triples declared at offset 9, excluding untouched registers.
    pub palette_count: usize,
    /// Viewport as the trailer declared it. Always 320×128; [`Room::parse`]
    /// refuses anything else.
    pub view: (u16, u16),
    /// Tile size as the trailer declared it. Always 16×16.
    pub tile: (u16, u16),
    /// Map size in tiles. Always 20×8.
    pub map: (u16, u16),
    /// 160 map cells, row-major. Values are 1-based indices into [`Self::cels`];
    /// 0 is empty (index 0 of the playfield, i.e. whatever the palette's 0 is).
    pub cells: [u8; MAP_CELLS],
    /// Cels addressed by the offset table, in table order. Index 0 of this
    /// vector is map value 1.
    pub cels: Vec<Picture>,
    /// The playfield composed from [`Self::cells`] and [`Self::cels`].
    ///
    /// Every layer flattened, which is what a still of the room looks like
    /// with nothing standing in it. A drawer that has to put a figure
    /// *between* two layers wants [`Self::layers`] instead.
    pub pixels: Vec<u8>,
    /// One entry per 20-byte directory record, in file order.
    pub layers: Vec<RoomLayer>,
    /// 32-byte object records from each layer directory (`+0Ch`, count
    /// `+2`, stride `20h`). `0BB6h:052Bh` looks these up by
    /// [`Overlay::id`] (`0x0C08B`).
    pub overlays: Vec<Overlay>,
    /// The table an object's `+0Ah` indexes when its `+8` names bundle 0:
    /// the room's own cels, in the order of the offset table at `pB`. An
    /// entry the table names but no picture parses at is `None`. The
    /// animator moves an object through this table by changing `+0Ah`, so
    /// a drawer needs the whole of it and not only the cel each object
    /// starts on.
    pub object_cels: Vec<Option<Picture>>,
    /// The records that walk besides the player, in table order — see
    /// [`MoverRecord`].
    pub movers: Vec<MoverRecord>,
}

/// One 32-byte RDV object, as `0883h:000Fh` (`0x0883F`) and the lookup
/// `0BB6h:052Bh` (`0x0C08B`) read it.
///
/// ```text
/// +0   u16 kind        # 4: a door, as the entry gate wants it
/// +2   u8  flags       # 80h set on arrival; the low two an anchor state
/// +4   u16 x, y
/// +8   u8  bundle      # which loaded file the cel index names
/// +A   u16 cel index
/// +E   u16 x scale, +10h y scale   # 100h is one; 0 and 100h skip
/// +13h u8  object id   # the lookup's key
/// +15h u8  loader      # `>1` writes 1 else 0 into byte +1
/// +18h u8  map id, +19h u8 map live, +1Ah u16 map cel   # an object map's
/// +1Ch i16 map dx, +1Eh i16 map dy                       # defaults
/// ```
#[derive(Debug, Clone)]
pub struct Overlay {
    /// Word at +0.
    pub kind: u16,
    /// Byte at +2.
    pub flags: u8,
    /// Left edge, playfield pixels (+4).
    pub x: u16,
    /// Top edge, playfield pixels (+6).
    pub y: u16,
    /// Byte at +8: which loaded resource bundle the cel at `+0Ah` is an
    /// index into.
    ///
    /// The parsec1994 generation's object drawer reads it and uses it to
    /// index the far-pointer table at `DS:2879h` four bytes at a time, then
    /// indexes what that names with `+0Ah` (Dunkle Schatten `0x08868`,
    /// `0x0888C`). Slot 1 is the file the player's cels come from
    /// (`0x10AA0`), so an object that draws from the player is one whose
    /// `+8` is 1 rather than 0.
    ///
    /// A zero here means the room's own cel table, which is what
    /// [`Overlay::cel`] resolves against — so a non-zero one is a cel this
    /// reader cannot resolve and leaves as `None`.
    pub bundle: u8,
    /// Byte at +9. The animator reads it as the object's **frame**:
    /// `0883h:012Ah` counts it up through a walk cycle (`0x08971`) and the
    /// interpreter writes an animation's frame into it (`0x09539`), with
    /// the cel index at `+0Ah` kept as `+0Ch` plus this byte (`0x0956B`).
    pub slot: u8,
    /// Word at +0Ch: the cel the object's frames count from. The animator
    /// makes `+0Ah` out of it and `+9` (`0x0956B`, `0x08C64`).
    pub cel_base: u16,
    /// Object id at +13h, the byte `0BB6h:052Bh` compares (`0x0C0E1`).
    pub id: u8,
    /// Byte at +15h. The loader copies `>1` into byte +1.
    pub init: u8,
    /// Byte at +3, the anchor mode: 1 centers the box on the position, 2
    /// stands it on the point.
    pub align: u8,
    /// Words at +0Eh and +10h: the cel's scale on each axis, `100h` being
    /// one. The object drawer multiplies the cel's size by them and
    /// shifts eight (`0x0888C` onwards in Dunkle Schatten), and skips the
    /// multiply when the value is 0 or `100h`.
    pub scale: (u16, u16),
    /// Which layer directory listed this object, by index.
    pub layer: u8,
    /// The cel index at `+0Ah`, into whichever bundle [`Self::bundle`]
    /// names. Kept whether or not this reader could resolve it, because a
    /// caller that has the other bundle can.
    pub cel_index: u16,
    /// Cel named at +0Ah, when `+8` names the room's own table.
    /// Flags-1 payloads are the opcode stream [`Picture`] decodes.
    pub cel: Option<Picture>,
    /// `+18h` to `+1Fh`: what an object map entry whose new id is `FFh`
    /// takes in place of its own (`0x7E04`–`0x7E6D`).
    pub map: MapDefaults,
}

/// An object's defaults for an object map, `+18h` to `+1Fh` of its record:
/// the new id, the live byte, the cel and the offset, in the order the
/// loader copies them into a [`crate::MapEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MapDefaults {
    /// `+18h`, the entry's new id.
    pub id: u8,
    /// `+19h`, the entry's live byte.
    pub live: u8,
    /// `+1Ah`, the entry's cel.
    pub cel: i16,
    /// `+1Ch`, the entry's `dx`.
    pub dx: i16,
    /// `+1Eh`, the entry's `dy`.
    pub dy: i16,
}

/// One 30h-byte record of a room that walks besides the player: a person.
///
/// The table is named by the trailer — the room header the parsec1994 loader
/// takes from the last 24h bytes of the resource (`0x789B`): its `+0Eh` is
/// the count and its `+14h` the file offset (`0x7972`, relocated at
/// `0x78BF`). Dunkle Schatten carries one in nine slots, the same nine
/// whose walk-graph resource holds a second graph.
///
/// ```text
/// +0    u8  kind      # 3 the player; 6, 7, 10h, 11h people; 8 a special kind
/// +1    u8            # the loader's (+15h > 1) (0x7986)
/// +2    u8  flags     # bit 0 live, bit 40h mirrored
/// +3    u8  align     # 1 centered, 2 standing on the point
/// +4    u16 x, y
/// +8    u8  bundle    # 0: the room's own cels
/// +9    u8  phase
/// +0Ah  u16 cel
/// +0Ch  u16 base      # the block the walk cels count from
/// +0Eh  u16 scale x, +10h scale y
/// +12h  u8  state     # 0 standing, 6 walking
/// +13h  u8  id
/// +14h  u8  interval  # the gait: ticks per phase, and the leg's cost
/// +15h  u8  phases
/// +20h  far next      # the layer chain; the rest is the walker's runtime
/// +24h  far graph header, +28h u16 node id, +2Ah u16 layer, +2Eh i16 facing
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoverRecord {
    /// Byte `+0`. The direction routine `0A9Fh:02CFh` switches on it
    /// (Dunkle Schatten `0x0ACBF`, table `cs:0315h`).
    pub kind: u8,
    /// Byte `+2`.
    pub flags: u8,
    /// Byte `+3`.
    pub align: u8,
    /// Word `+4`.
    pub x: u16,
    /// Word `+6`.
    pub y: u16,
    /// Byte `+8`: the bundle `+0Ah` is an index into.
    pub bundle: u8,
    /// Byte `+9`.
    pub phase: u8,
    /// Word `+0Ah`.
    pub cel: u16,
    /// Word `+0Ch`.
    pub base: u16,
    /// Words `+0Eh` and `+10h`.
    pub scale: (u16, u16),
    /// Byte `+12h`.
    pub state: u8,
    /// Byte `+13h`.
    pub id: u8,
    /// Byte `+14h`.
    pub interval: u8,
    /// Byte `+15h`.
    pub phases: u8,
    /// Word `+28h`, what the record says it stands on before anything
    /// has put it on a graph.
    pub node: u16,
    /// Word `+2Ah`.
    pub layer: u16,
}

/// The size of one [`MoverRecord`] (`imul ax, 30h` at `0x7988`).
pub const MOVER_RECORD: usize = 0x30;

impl MoverRecord {
    fn read(b: &[u8; MOVER_RECORD]) -> Self {
        let r = Record(b);
        MoverRecord {
            kind: r.u8::<0>(),
            flags: r.u8::<2>(),
            align: r.u8::<3>(),
            x: r.u16::<4>(),
            y: r.u16::<6>(),
            bundle: r.u8::<8>(),
            phase: r.u8::<9>(),
            cel: r.u16::<0x0A>(),
            base: r.u16::<0x0C>(),
            scale: (r.u16::<0x0E>(), r.u16::<0x10>()),
            state: r.u8::<0x12>(),
            id: r.u8::<0x13>(),
            interval: r.u8::<0x14>(),
            phases: r.u8::<0x15>(),
            node: r.u16::<0x28>(),
            layer: r.u16::<0x2A>(),
        }
    }
}

/// The trailer's `+0Eh`/`+14h`: the room's 30h-byte records, as many of
/// them as the resource holds whole.
fn mover_records(data: &[u8], trailer: Trailer<'_>) -> Vec<MoverRecord> {
    let n = trailer.u16::<0x0E>();
    let at = trailer.u32at::<0x14>();
    if n == 0 || at == 0 {
        return Vec::new();
    }
    data.get(at..)
        .unwrap_or_default()
        .as_chunks::<MOVER_RECORD>()
        .0
        .iter()
        .take(usize::from(n))
        .map(MoverRecord::read)
        .collect()
}

/// One 16-byte walk node, as the snap copies it into a walker (Dunkle
/// Schatten `0x0B738`–`0x0B7B4`).
///
/// ```text
/// +0  u16 x, y     # copied to walker +4 / +6 (feet)
/// +4  u16          # copied to walker +10h / +0Eh
/// +6  u8           # walker +2Ah
/// +8  u16 id
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkNode {
    /// Playfield x, feet (→ walker +4).
    pub x: u16,
    /// Playfield y, feet (→ walker +6).
    pub y: u16,
    /// Figure scale at +4, base `100h`: the snap writes it into both of
    /// the walker's scale words (`0x0B77C`).
    pub scale: u16,
    /// Layer index at +6 (walker `+2Ah`, the draw-order chain).
    pub layer: u8,
    /// Lookup key at +8. Door / overlay id, negative for anonymous
    /// waypoints, or an entry twin (`id + 7D0h` where the room has one).
    pub id: u16,
    /// First edge of this node's circular adjacency chain, at +0Ah.
    pub first_edge: u16,
}

/// One 16-byte edge record (header +8 array).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkEdge {
    /// Endpoints, node indices (+0 / +2).
    pub a: u16,
    /// Second endpoint.
    pub b: u16,
    /// Next edge in `a`'s circular chain (+4).
    pub next_a: u16,
    /// Next edge in `b`'s circular chain (+6).
    pub next_b: u16,
    /// +8: joined to the length by `or` into the flood's edge cost
    /// (`0x0AD45`).
    pub blocked: u16,
    /// +0Ah: length; the leg's t-step is computed from it and the
    /// walker's interval (`0x0AF0B`).
    pub len: u16,
    /// +0Ch: the axis word. The walk tests it at `0x0AFCE` and picks the
    /// walker's direction by it: non-zero, by the sign of the walker's y
    /// delta (`+22h`, `0x0AED3`); zero, by that of its x delta (`+1Eh`,
    /// `0x0AEB5`). The word is the room's choice, not the edge's slope: of
    /// the 559 edges in Dunkle Schatten's rooms, 193 are non-zero, 57 of
    /// them at least as wide as they are tall, and 5 of the 366 zero ones
    /// are taller than they are wide.
    pub axis: u16,
    /// +0Eh: layer while walking this edge (walker `+2Ah`).
    pub layer: u16,
}

/// The stride of the walk-graph headers (`add bx, 2Eh` at Dunkle
/// Schatten's `0x726A`, `imul ax, 2Eh` at `0x0B8F1`).
pub const GRAPH_HEADER: usize = 0x2E;

/// A room slot's walk-graph resource, resource 0.
///
/// The header at +4 holds `+0` the node count, `+2` the edge count and at
/// `+4` / `+8` the offsets of the two 16-byte arrays (file-relative before
/// reloc). `NULLPTR` slots are not a graph.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WalkGraph {
    /// 16-byte records at header +4, count at header +0.
    pub nodes: Vec<WalkNode>,
    /// 16-byte records at header +8, count at header +2.
    pub edges: Vec<WalkEdge>,
}

impl WalkGraph {
    /// Parses graph 0. `None` when the blob is `NULLPTR`, too short, or
    /// the header does not name a 16-byte array that fits.
    pub fn parse(data: &[u8]) -> Option<Self> {
        Self::parse_graph(data, 0)
    }

    /// Every graph the resource holds, in header order.
    ///
    /// The word at `+0` counts the headers, and `0724h:0000h` relocates
    /// that many, `2Eh` bytes apart from `+4` (Dunkle Schatten `0x7259`,
    /// `0x726A`). Most Dunkle Schatten rooms hold one; the nine with a
    /// person in them hold two, and graph 1 is the person's.
    pub fn parse_all(data: &[u8]) -> Vec<Self> {
        let Some(count) = Self::count(data) else {
            return Vec::new();
        };
        (0..count)
            .map_while(|k| Self::parse_graph(data, k))
            .collect()
    }

    /// The header count at `+0`, when the blob is a graph resource at all.
    fn count(data: &[u8]) -> Option<usize> {
        if data.len() < 4 + 0x28 || data.starts_with(crate::NULLPTR) {
            return None;
        }
        let count = usize::from(u16le(data, 0).ok()?);
        (1..=8).contains(&count).then_some(count)
    }

    /// Where graph `k`'s header sits: `4 + k·2Eh`.
    fn header(k: usize) -> Option<usize> {
        k.checked_mul(GRAPH_HEADER)?.checked_add(4)
    }

    /// The mode word of graph `k`'s header (`+10h`), as the file has it.
    ///
    /// The snap `0A9Fh:0C59h` is what sets it to 0 (`0x0B7D0`); until then
    /// the walk routines read it as "no walk" (`0x0B0A6`–`0x0B0BB`).
    pub fn file_mode(data: &[u8], k: usize) -> Option<u16> {
        let h = Self::header(k)?;
        (k < Self::count(data)?).then(|| u16le(data, h.checked_add(0x10)?).ok())?
    }

    /// Graph `k`: the header at `4 + k·2Eh`, laid out like graph 0's.
    pub fn parse_graph(data: &[u8], k: usize) -> Option<Self> {
        if k >= Self::count(data)? {
            return None;
        }
        let head = Record(bytes::<10>(data, Self::header(k)?).ok()?);
        let n = usize::from(head.u16::<0>());
        if n == 0 || n > 256 {
            return None;
        }
        let e = usize::from(head.u16::<2>());
        if e > 256 {
            return None;
        }
        let nodes = records::<16>(data, usize::from(head.u16::<4>()), n).ok()?;
        let edges = records::<16>(data, usize::from(head.u16::<8>()), e).ok()?;
        let nodes = nodes
            .iter()
            .map(|r| {
                let r = Record(r);
                WalkNode {
                    x: r.u16::<0>(),
                    y: r.u16::<2>(),
                    scale: r.u16::<4>(),
                    layer: r.u8::<6>(),
                    id: r.u16::<8>(),
                    first_edge: r.u16::<0x0A>(),
                }
            })
            .collect();
        let edges = edges
            .iter()
            .map(|r| {
                let r = Record(r);
                WalkEdge {
                    a: r.u16::<0>(),
                    b: r.u16::<2>(),
                    next_a: r.u16::<4>(),
                    next_b: r.u16::<6>(),
                    blocked: r.u16::<8>(),
                    len: r.u16::<0x0A>(),
                    axis: r.u16::<0x0C>(),
                    layer: r.u16::<0x0E>(),
                }
            })
            .collect();
        Some(WalkGraph { nodes, edges })
    }

    /// The last node whose +8 equals `id`, walking the table backwards.
    pub fn lookup(&self, id: u16) -> Option<&WalkNode> {
        self.nodes.iter().rev().find(|n| n.id == id)
    }
}

/// The last 36 bytes of a room resource: viewport, tile and map size, the
/// layer count and the tables the loader takes from it.
type Trailer<'a> = Record<'a, 36>;

impl Room {
    /// Parses an `RDV1.00` resource. `data` is the RIB resource, not the DAT
    /// file.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let head = Record(bytes::<{ 8 + 5 + 36 }>(data, 0)?);
        let magic = head.bytes::<0, 8>();
        if magic != *MAGIC {
            return Err(Error::Corrupt {
                what: "RDV",
                detail: format!("expected RDV1.00, found {magic:02x?}"),
            });
        }
        // Same `start, count` DAC write as PANELS.DAT / CMBMOU.DAT. Every
        // Dunkle Schatten room is `00 80` — 128 colors at register 0.
        let pal_start = head.u8::<8>();
        let pal_count = usize::from(head.u8::<9>());
        let palette_rgb = match slice(data, 10, pal_count.saturating_mul(3)) {
            Ok(rgb)
                if pal_count != 0 && usize::from(pal_start).saturating_add(pal_count) <= 256 =>
            {
                rgb.to_vec()
            }
            _ => {
                return Err(Error::Corrupt {
                    what: "RDV",
                    detail: format!("palette start {pal_start} count {pal_count}"),
                });
            }
        };

        // Trailer: last 36 bytes start with the viewport / tile / map words.
        let t = data.len().saturating_sub(36);
        let trailer: Trailer<'_> = Record(bytes::<36>(data, t)?);
        let view = (trailer.u16::<0>(), trailer.u16::<2>());
        let tile = (trailer.u16::<4>(), trailer.u16::<6>());
        let map = (trailer.u16::<8>(), trailer.u16::<10>());
        if view != (VIEW_W, VIEW_H) || tile != (TILE, TILE) || map != (MAP_W, MAP_H) {
            return Err(Error::Corrupt {
                what: "RDV",
                detail: format!("trailer view {view:?} tile {tile:?} map {map:?}"),
            });
        }
        // Layer count sits right after the map size, 1 to 6 in Dunkle
        // Schatten's rooms; its directories are 20 bytes each from `pA`,
        // 12 bytes from EOF.
        let n_layers = usize::from(trailer.u16::<12>());
        if n_layers > usize::from(u8::MAX) {
            return Err(Error::OutOfRange {
                field: "RDV layer count",
                value: crate::long(n_layers),
                allowed: "at most 255",
            });
        }
        let p_a = trailer.u32at::<24>();
        let p_b = trailer.u32at::<28>();
        let dirs = records::<20>(data, p_a, n_layers)?;
        let table_end = dirs
            .iter()
            .map(|d| Record(d).u32at::<0x0C>())
            .filter(|&objects| objects != 0)
            .min()
            .unwrap_or(p_a);
        let layers = dirs
            .iter()
            .map(|d| read_layer(data, Record(d), table_end))
            .collect::<Result<Vec<_>>>()?;
        let (cells, cels) = layers
            .iter()
            .find(|l| l.kind == TILE_LAYER)
            .map_or(([0u8; MAP_CELLS], Vec::new()), |l| {
                (l.cells, l.cels.clone())
            });
        let mut pixels = vec![0u8; VIEW_PIXELS];
        for layer in &layers {
            for (dst, &src) in pixels.iter_mut().zip(layer.pixels.iter()) {
                if src != 0 {
                    *dst = src;
                }
            }
        }
        let object_cels = cel_table(data, p_b, table_end);
        let overlays = overlay_objects(data, dirs, &object_cels);
        Ok(Room {
            palette_rgb,
            palette_start: pal_start,
            palette_count: pal_count,
            view,
            tile,
            map,
            cells,
            cels,
            pixels,
            layers,
            overlays,
            object_cels,
            movers: mover_records(data, trailer),
        })
    }

    /// `true` when `data` starts with `RDV1.00\0`.
    pub fn looks_like(data: &[u8]) -> bool {
        data.starts_with(MAGIC)
    }
}

/// The kind of a directory record whose map is one byte a cell; a record
/// of any other kind but 2 draws no tiles (`0x0BE8E`).
const TILE_LAYER: u16 = 1;

/// One directory record as a layer: its map and the cels it names, from
/// its table pointer to `table_end`, the end of the room's cel table.
///
/// An entry that does not parse keeps its slot, so a later map value
/// still names the entry it names in the file; an empty picture is not
/// 16×16, and compose leaves such a cell blank.
fn read_layer(data: &[u8], dir: Record<'_, 20>, table_end: usize) -> Result<RoomLayer> {
    let kind = dir.u16::<0>();
    match kind {
        TILE_LAYER => {}
        2 => {
            return Err(Error::OutOfRange {
                field: "RDV layer kind",
                value: u64::from(kind),
                allowed: "0 or 1 (2, a map of words, is not read)",
            });
        }
        _ => {
            return Ok(RoomLayer {
                kind,
                cells: [0u8; MAP_CELLS],
                cels: Vec::new(),
                pixels: Vec::new(),
            });
        }
    }
    let map_off = dir.u32at::<4>();
    let cells = match bytes::<MAP_CELLS>(data, map_off) {
        Ok(cells) if map_off != 0 => *cells,
        _ => {
            return Err(Error::Corrupt {
                what: "RDV",
                detail: format!("directory map {map_off:#x}"),
            });
        }
    };
    let table = dir.u32at::<8>();
    let n = table_end.saturating_sub(table).checked_div(4).unwrap_or(0);
    let cels: Vec<Picture> = records::<4>(data, table, n)?
        .iter()
        .map(|off| {
            cel_at(data, crate::wide(u32::from_le_bytes(*off))).unwrap_or(Picture {
                width: 0,
                height: 0,
                pixels: Vec::new(),
            })
        })
        .collect();
    let pixels = compose(&cells, &cels);
    Ok(RoomLayer {
        kind,
        cells,
        cels,
        pixels,
    })
}

/// The cel at `off`, when one parses there.
fn cel_at(data: &[u8], off: usize) -> Option<Picture> {
    Picture::parse_prefix(tail(data, off).ok()?).ok()
}

/// Bytes in one object-table record, an [`Overlay`].
const OBJ_RECORD: usize = 32;

fn overlay_objects(data: &[u8], dirs: &[[u8; 20]], table: &[Option<Picture>]) -> Vec<Overlay> {
    let mut out = Vec::new();
    for (dir, layer) in dirs.iter().zip(0u8..) {
        let dir = Record(dir);
        let n = usize::from(dir.u16::<2>());
        if n == 0 || n > 64 {
            continue;
        }
        let objects = data
            .get(dir.u32at::<12>()..)
            .unwrap_or_default()
            .as_chunks::<OBJ_RECORD>()
            .0;
        for o in objects.iter().take(n) {
            let o = Record(o);
            let cel_index = o.u16::<0x0A>();
            // `+0Ah` is an index into whichever bundle `+8` names, and
            // this reader has only the room's own. An object drawing from
            // another file keeps its index and gets no picture here.
            let bundle = o.u8::<8>();
            let cel = (bundle == 0)
                .then(|| {
                    table
                        .get(usize::from(cel_index))
                        .cloned()
                        .flatten()
                        .filter(|c| c.width >= 2 && c.height >= 2)
                })
                .flatten();
            out.push(Overlay {
                kind: o.u16::<0>(),
                flags: o.u8::<2>(),
                x: o.u16::<4>(),
                y: o.u16::<6>(),
                bundle,
                slot: o.u8::<9>(),
                cel_base: o.u16::<0x0C>(),
                id: o.u8::<0x13>(),
                init: o.u8::<0x15>(),
                align: o.u8::<3>(),
                scale: (o.u16::<0x0E>(), o.u16::<0x10>()),
                layer,
                cel_index,
                cel,
                map: MapDefaults {
                    id: o.u8::<0x18>(),
                    live: o.u8::<0x19>(),
                    cel: o.i16::<0x1A>(),
                    dx: o.i16::<0x1C>(),
                    dy: o.i16::<0x1E>(),
                },
            });
        }
    }
    out
}

/// The run of cel offsets from `pB` to `table_end`, each decoded where a
/// cel parses.
fn cel_table(data: &[u8], p_b: usize, table_end: usize) -> Vec<Option<Picture>> {
    let n = table_end.saturating_sub(p_b).checked_div(4).unwrap_or(0);
    records::<4>(data, p_b, n)
        .unwrap_or_default()
        .iter()
        .map(|off| cel_at(data, crate::wide(u32::from_le_bytes(*off))))
        .collect()
}

/// One layer's tiles laid out on the playfield, index 0 left transparent.
fn compose(cells: &[u8; MAP_CELLS], cels: &[Picture]) -> Vec<u8> {
    let mut pixels = vec![0u8; VIEW_PIXELS];
    let tile = |idx: u8| {
        usize::from(idx)
            .checked_sub(1)
            .and_then(|k| cels.get(k))
            .filter(|c| c.width == TILE && c.height == TILE)
    };
    for (band, row) in pixels.chunks_mut(TILE_BAND).zip(cells.chunks(MAP_ROW)) {
        for (py, line) in band.chunks_mut(VIEW_ROW).enumerate() {
            for (dst, &idx) in line.chunks_mut(TILE_PX).zip(row) {
                let Some(src) = tile(idx).and_then(|c| c.pixels.chunks(TILE_PX).nth(py)) else {
                    continue;
                };
                for (d, &p) in dst.iter_mut().zip(src) {
                    if p != 0 {
                        *d = p;
                    }
                }
            }
        }
    }
    pixels
}

/// One room of `DSROOM.DAT`: the RDV picture, its hotspots, walk graphs and
/// animations, taken from the slot's six resources.
#[derive(Debug, Clone)]
pub struct RoomFile {
    /// The tiled playfield, resource 5.
    pub room: Room,
    /// Clickable rectangles, resource 4. Empty when that resource is the
    /// empty-slot literal.
    pub hotspots: HotspotTable,
    /// Every graph of resource 0, in header order: graph 0 is the
    /// player's ([`Self::walk`]), and a second one, where there is one, is
    /// the graph a person of the room walks on. Empty when resource 0 is
    /// the empty-slot literal.
    pub walks: Vec<WalkGraph>,
    /// Resource 1, the `AN` block the slot's objects animate from — the
    /// `ANI` of the six the loader names, kept at `[286Dh]` (`0x7B4E`).
    /// `None` when resource 1 is `NULLPTR`.
    pub animations: Option<crate::ani::Animations>,
}

impl RoomFile {
    /// Graph 0, the one the player walks on.
    pub fn walk(&self) -> Option<&WalkGraph> {
        self.walks.first()
    }

    /// Parses one slot of `DSROOM.DAT`.
    ///
    /// A room is one slot of an FGROUP, a RIB of six whose every resource
    /// may be the empty-slot literal instead:
    ///
    /// | Index | What |
    /// |---:|---|
    /// | 0 | the walk graph, 16-byte node and edge records |
    /// | 1 | an `AN…` animation |
    /// | 2 | an `OMP\0` block — 31 slots carry one |
    /// | 3 | dialogue |
    /// | 4 | the hotspot table, 46-byte records |
    /// | 5 | the `RDV1.00` room |
    ///
    /// Counted: all 28 rooms of `DSROOM.DAT` compose a 320×128 playfield
    /// through this, and every walk graph either parses or is the empty
    /// literal.
    ///
    /// `None` when the slot carries no room, which most of `DSROOM.DAT`'s
    /// slots do not.
    pub fn from_1994_slot(rib: &Rib) -> Option<Result<Self>> {
        let rdv = rib.get(5).filter(|b| !b.starts_with(crate::NULLPTR))?;
        Some((|| {
            let room = Room::parse(rdv)?;
            let hotspots = match rib.get(4).filter(|b| !b.starts_with(crate::NULLPTR)) {
                Some(h) => HotspotTable::parse(h)?,
                None => HotspotTable {
                    spots: Vec::new(),
                    widgets: Vec::new(),
                },
            };
            let walks = rib.get(0).map(WalkGraph::parse_all).unwrap_or_default();
            let animations = match rib.get(1).filter(|b| !b.starts_with(crate::NULLPTR)) {
                Some(a) => Some(crate::ani::Animations::parse(a)?),
                None => None,
            };
            Ok(RoomFile {
                room,
                hotspots,
                walks,
                animations,
            })
        })())
    }
}

#[cfg(test)]
mod walk_graph_tests {
    use super::*;

    #[test]
    fn a_one_node_resource_two_parses() {
        let mut blob = vec![0u8; 0x2C + 16];
        blob[0] = 1; // record count
        blob[4] = 1; // node count
        blob[8] = 0x2C; // nodes at +2Ch
        blob[0x2C] = 10; // x
        blob[0x2E] = 20; // y
        blob[0x2C + 8] = 0x85; // id
        let g = WalkGraph::parse(&blob).expect("graph");
        assert_eq!(g.nodes.len(), 1);
        let n = g.lookup(0x85).expect("id");
        assert_eq!((n.x, n.y), (10, 20));
    }

    #[test]
    fn nullptr_is_not_a_walk_graph() {
        assert!(WalkGraph::parse(b"NULLPTR").is_none());
        assert!(WalkGraph::parse(&[0, 0, 0, 0]).is_none());
    }
}
