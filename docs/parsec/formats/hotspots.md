[← Documentation index](../README.md)

# Hotspots — `MBX`

*PARSEC — Dunkle Schatten (1994): resource 4 of a `DSROOM.DAT` slot, the clickable rectangles of a room and their canned answers.*

The things a player can point at in a room are not its objects: they are
the slot's hotspot table, resource 4, the `MBX` of the six. **20 slots carry
one**, 132 records in all, every one of them live.

## Layout

```text
u32 8
u16 0
u16 count       # the 46-byte records
u32 0
# count records, then the string pool
```

A record, 46 bytes:

| Offset | Field |
|---|---|
| `+0` | `u32` active — 1 is live |
| `+4` | `u16` x, y, w, h — the rectangle in the 320×128 playfield |
| `+0Ch` | `u16` verb mask — bit 15 marks a record the hover takes; the low byte is the record's id |
| `+0Eh` | `u32` the name, an offset into the string pool |
| `+12h` | `u32 text[7]` — the canned answers, 0 for none |

The string pool follows the records: CP437, each string ended by a zero,
some wrapping with an embedded CR and LF. **The verbs' names are not here.**
`text[0]` is what the hover shows at the pointer, and the verb handlers read
`text[2]` Benutze, `text[3]` Betrachte, `text[4]` Sprich mit and `text[5]`
Nimm. The handlers name them at `+1Eh`, `+22h`, `+26h` and `+2Ah` of the
run-time record, which is the file's record shifted four bytes on from the
verb mask.

A point is inside a rectangle when `x ≤ px < x + w` and `y ≤ py < y + h`;
where two records overlap, the later one wins.

## Mouse objects

The header's `+4` word, 0 above, counts a second kind of record: 18-byte
**mouse objects** from `+8`, which `0x1543B` would register. No slot of the
game carries any. The same record is what the program keeps for the strip
under the playfield — seventeen of them at `2115:0000` (`0x0C40C`):

| Offset | Field |
|---|---|
| `+0` | far: the next |
| `+4` | `u16` flags — bit 0 live |
| `+6` | `u16` |
| `+8` | `u16` x, y, w, h |
| `+10h` | `u16` id — the verb buttons `101h` to `106h`, the strip `114h`, the arrows `115h` and `116h`, the inventory slots from `11Eh` |

## See also

- [Rooms](rooms.md) — the slot the table sits in
- [The object map](object-map.md) — how a forest row renames its hotspots
- [Verbs](../engine/verbs.md) — what a click on a record does
