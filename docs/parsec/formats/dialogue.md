[← Documentation index](../README.md)

# Dialogue — `DIALOG.DAT` and the Rooms' Banks

*PARSEC — Dunkle Schatten (1994): the `II` records every line the game speaks is kept in, and the banks that number them.*

Every line of speech and every choice list is an `II` record. The records
are kept in **banks**, and there are three a scene can reach: the two
global banks, which are the first two resources of `DIALOG.DAT` — bank 999,
resource 0, which holds among other things what a verb that leads nowhere
says, and bank 2, resource 1, which holds what the inventory says (both put
in place at `0x525F`) — and the room's own, resource 3 of its
`DSROOM.DAT` slot, kept at `[2855h]` with its count at `[1EEAh]`.
`DIALOG.DAT`'s resource 2 is the player's [`AN` block](animations.md).

## A bank

A bank is a second directory nested inside the resource — the
`u32 off[n]`, `u32 0`, `u32 size[n]` shape of a [RIB](containers.md)'s own
table — whose every entry is an `II` blob or the literal `NULLPTR\r\n`. An
entry is addressed by its number, so a bank is a list of numbered places,
some of them deliberately empty; the program says so when it meets one:
`NULLDLG: D%03d_%03d.DCS`, formatted with the bank's number and the
entry's.

How a scene's number picks a bank (`0CFDh:059Eh`): a negative number is a
place by its complement — below `4000h` in bank 999 (`0x0D5DC`), from
`4000h` up in bank 2, less `4001h` (`0x0D5F0`); a number that is not
negative indexes the room's bank while it is below `[1EEAh]` (`0x0D5EA`).
When nothing is found, place `13h` of bank 999 stands in (`0x0D5FD`).

## An `II` blob

```text
"II"
u16 count
# from +4: count records of eight bytes
# the string pool and next-lists, offsets from the blob's start
```

The player reads a record as `{flags, id, textOff, replyOff}`, eight bytes
starting at `+4` (`0x0D9FE`, `0x0DA1C`, `0x0DAAC`, `0x0DA70`) — so the
header's third and fourth words are record 0's flags and id, and record
`i`'s text offset lands at `8 + 8i`.

| Offset | Field |
|---|---|
| `+0` | `u16` flags — the low nibble the speaker plus one, or 0 for a choice list; bit 12 asks a story flag, bit 13 sets one, bits 4–11 name it; bit 15 marks the record spent |
| `+2` | `u16` an id the location's hook is told about (code 4, `0x0DA2C`) |
| `+4` | `u16` the text |
| `+6` | `u16` the reply: 0 for a leaf, else a string or a next-list |

A **next-list** is `u16 n` and `n` record indices, `n` from 1 to 32 and
every index below the count; a reply that does not read as one is a string.

## Where a text ends

The program never measures a record's text on its own. `0CFDh:0932h` hands
the record's pointer to `13A7h:0023h` (`0x0D93D`; the choice menu does the
same at `0x0DC14`), and that copy decides where it ends: a byte above `1Fh`
is copied (`0x13AB3`), a zero ends the text (`0x13AB7`), a byte above 3 is
copied (`0x13ABB`), a `01` or `03` is copied with the byte after it
(`0x13AC8`), and a `02` takes the byte after it as a substitution index,
whatever that byte is (`0x13ABF`, `0x13AD2`). So `01 00` and `02 00` are
inside the text: the argument of a control is never a terminator.

The copy also stops where the room its caller gives it runs out
(`0x13AC3`, `0x13AEA`): `[1E4Ah]`, half of the 2000-byte scratch `[1691h]`
(`0x0DF3D`–`0x0DF45`) — 1000 bytes. No text the game ships reaches it; the
longest is 510 bytes. What the control bytes mean when drawn is the
[text page](../engine/text.md).

## See also

- [The text controls](../engine/text.md) — inks, substitutions, the draw
- [Rooms](rooms.md) — the slot whose resource 3 is a room's bank
- [Resource containers](containers.md) — the directory shape a bank nests
