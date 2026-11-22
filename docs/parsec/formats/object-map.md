[← Documentation index](../README.md)

# The Object Map — `OMP`

*PARSEC — Dunkle Schatten (1994): resource 2 of a `DSROOM.DAT` slot, the map that turns the one forest picture into each forest and maze row.*

The Wald and the thirty rows of the maze share one picture, slot 20 of
`DSROOM.DAT`. What makes each row a place of its own is a map in the row's
own slot: which of the picture's records show and on which cel, where they
stand, and what they are called. The room loader loads the row's slot beside
the picture's for it (`0x7B6D`) and binds the map's entries to the
picture's records (`0x7BFB`–`0x830C`). The format is here; what the binding
does to the records is on the [objects page](../engine/objects.md).

**31 slots carry one** — the Wald's (23) and the maze's (30 to 59) — each
760 bytes, 26 entries, flags `3Fh`, and every byte of every entry past
`+0Ch` zero on disk.

## Layout

| Offset | Field |
|---|---|
| `+0` | `"OMP\0"` |
| `+4` | `u16` entry count |
| `+6` | `u16` flags — what else the map renames, below |
| `+20h` | The entries, `1Ch` bytes each |

An entry:

| Offset | Field |
|---|---|
| `+0` | `u16` id — the record it binds: an object's `+13h`, a hotspot's id, a node's id, an `AN` entry's object |
| `+2` | `u16` new id — the id the record takes; `FFh` takes the object's own defaults at `+18h`–`+1Fh` of its record first (`0x7DFC`–`0x7E6D`) |
| `+4` | `i16` cel — below 0 hides the record, `FFh` sets its state to 3, any other is the frame it shows from its base (`0x7EC0`, `0x7EC7`) |
| `+6` | `u16` the **live byte**, copied into the object's `+1` (`0x7EB9`) |
| `+8`, `+0Ah` | `i16` `dx`, `dy` — added to the record's position |
| `+0Ch` | far: the object bound |
| `+10h` | far: the hotspot bound |
| `+14h` | far: the walk-graph node bound |
| `+18h` | far: the `AN` entry bound |

The four far pointers are zero on disk and filled at load time (`0x7DF1`,
`0x802E`, `0x81AB`, `0x82D6`). A record binds the first entry, in file order,
that is still free and carries its id.

**The live byte has one reader.** Of everything in the program, only
`0883h:000Fh` reads an object's `+1`, and only to leave a standing
single-cel visible record's rectangle off the list of rectangles to redraw
(`0x884D`). It changes what is copied to the screen, never what the picture
holds.

## The flags

The word at `+6` says which further kinds of record the map renames, and
which of the row's resources the slot release keeps between two forest rows
instead of giving them back:

| Bit | Renamed at load | Kept between two forest rows |
|---|---|---|
| 0 | The hotspots (`0x7FB5`) | The hotspots (`0x8433`) |
| 1 | — | The dialogue bank (`0x8547`) |
| 2 | — the row brings no samples of its own (`0x8355`) | The samples (`0x85D4`) |
| 3 | — the row brings no song of its own (`0x8339`) | The song (`0x85ED`) |
| 4 | The `AN` entries without a chain (`0x826A`) | The animation entries (`0x8592`) |
| 5 | The walk graph's nodes (`0x811E`) | The walk graph (`0x84FC`) |

All 31 maps carry `3Fh`, every bit set: a forest or maze row renames
hotspots, nodes and animation entries, brings no song of its own, and the
music playing when it is entered keeps playing.

## See also

- [Rooms](rooms.md) — the slot, its objects and their default bytes `+18h`–`+1Fh`
- [Hotspots](hotspots.md) — the records bit 0 renames
- [Animations](animations.md) — the entries bit 4 renames
- [Objects](../engine/objects.md) — how the loader binds a map and releases it
