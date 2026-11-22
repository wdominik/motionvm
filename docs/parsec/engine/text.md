[← Documentation index](../README.md)

# Text and Captions

*PARSEC — Dunkle Schatten (1994): `XP01.EXE`, as its unpacked load image of 150 096 bytes shows it. Addresses `0x…` are linear offsets into that image; `seg:off` names a routine by its far address.*

Every line the game shows — a caption over a speaker, a choice in the
menu, the status line — is CP437 text with a few control bytes below
`20h` in it. Segment `13A7h` measures and copies the texts a task plays —
a caption, a choice of the menu — and segment `1269h` composes a page into
a box and paints it; the status line has a setter and a measure of its
own. This page is what the control bytes mean, what is substituted into a
text, where its box goes and how the status line is placed.

## The control bytes

| Bytes | What |
|---|---|
| `00` | the end of the text |
| `01 n` | the ink: byte `n` of the speaker's style row (`0x129B0`); `01 00` is the first ink again |
| `02 n` | substitution `n`, resolved when the text is copied |
| `03 n` | a signal to the task that plays the text: `03 01` hold for a click, `03 02` and `03 05` flags of task 0, `03 03` longer and `03 04` shorter, `03 78` leave through a door, anything else an event for the location's hook ([tasks](tasks.md#a-page-and-how-long-it-stays)) |
| `0Ah` | a line break |
| `0Dh` | a page break |

The byte after `01`, `02` or `03` is an argument and is never tested for
zero, so `01 00` and `02 00` are inside a text, and a text ends at the
first zero that is not such an argument.

**Measuring.** `13A7h:0196h` walks a line: a byte below `20h` stops it,
except that `01` and `03` take the byte after them along and add nothing
to the width (`0x13C13`). So a line ends at `0Ah`, `0Dh` or the end alike,
and a page is lines while the byte that stopped the last one is `0Ah`
(`0CFDh:071Dh`, `0x0D719`). Every advance adds the font record's `+5` to
a glyph's own width, read signed (`0x0C73F`); the loader writes that byte
as `0FFh` (`0x5D13`), so glyphs overlap by one column and their outlines
meet ([graphics](../formats/graphics.md)).

**The status line** is set by `0C3Fh:0B3Eh` (`0x0CF2E`), not through
`13A7h`. It copies the text into `2115h:0132h` (`1EBDh:00AEh`,
`0x0CF43`) and measures it with `1601h:1465h` (`0x0CF6D`). That walk ends
only at the zero (`0x1749A`) and steps over every byte outside the font's
range — below its first character or past its last — without taking the
byte after it along (`0x1749C`–`0x174A2`). A character counts when its
glyph record's `+0` is not zero (`0x174AB`) and adds the record's `+2`;
the spacing is added once fewer times than characters were counted
(`0x174B9`–`0x174C3`). A line wider than `140h` loses characters from its
end, each from `20h` up taking its advance (`1601h:14BAh`) off the width,
until it fits (`0x0CF79`–`0x0CFA7`). The second argument, after the
text, places it (`0x0CFAD`–`0x0CFCB`): 0 at the left edge, 2 against the
right, anything else centered. Four calls reach the setter. The composer
passes 1 for the verb's line (`0x10140`) and for the empty one
(`0x101A5`), and 2 for the location's name and number, which it sets in
place of the empty line while bit 5 of the developer's switches `[1EDCh]`
is set and `[1F4Ch]` equals `[1F4Ah]` (`0x10149`–`0x10158`, `0x101A1`).
The room list that formats *Raum %2d : %s* passes 0 (`0x0F0EF`), and so
does the leaving, with the empty line (`0x0ED5A`). The fourth call
(`0x10462`) hands on the placement its own caller gave, and no call to
that routine, `0x1044E`, is found in the image.

## Substitution

`13A7h:0023h` copies a text into a task's scratch and resolves `02 n` as
it goes: a byte above `1Fh` is copied (`0x13AB3`), 0 ends the copy
(`0x13AB7`), one above 3 is copied too (`0x13ABB`), a 1 or a 3 is copied
with the byte after it (`0x13AC8`), and a 2 takes the byte after it as an
index into a row of far pointers its caller hands over (`0x13ABF`). The
copy also stops where the room its caller hands it runs out (`0x13AC3`,
`0x13AEA`): the dialogue player and the choice menu (`0x0D93D`,
`0x0DC14`) hand it `[1E4Ah]`, half of the 2000-byte scratch `[1691h]`
(`0x0DF3D`–`0x0DF45`), so 1000 bytes. No text the game ships reaches that:
the longest is 510.

The row the player hands over is `DS:1689h` (`0x0D93A`), and it has two
entries. Entry 1 is a word the program keeps there from the start. Entry
0 is drawn: the Wohnzimmer's handler (`0100:0073`), on the way to a
telephone scene, counts the row of names at `DS:1060h` (`0x117D`), draws
an index into it with the game's generator (`0x1189`) and writes the name
into entry 0. Scene 12 of that location reads it — *Nein, hier
ist bestimmt nicht «0»* — so every wrong number asks for someone else,
and no dialogue carries a name. Nothing at the start draws into the row,
so a fresh game's entry 0 is what the image ships.

## The generator

`1EB5h:0028h` (`0x1EB78`) is a 32-bit linear congruential generator with
its seed in its own code segment, `cs:[6]` and `cs:[8]`: a 16×16→32
multiply by `660Dh` and `19h` (`0x1EB8E`, `0x1EB91`), `3619_1149h` added
with the carry (`0x1EBAB`–`0x1EBAF`), and the answer
`low + (hi mod (high − low + 1))` (`0x1EBC1`–`0x1EBC6`). A `low` that is
not below `high` answers `low` without moving the seed (`0x1EB80`). The
image ships the seed at zero. The pointer library has a generator of its
own ([cursors](cursors.md)).

## The caption

A task's page becomes a box in two steps. `1269h:0082h` measures it,
frames it and places it against its anchor — the point the speaker's
subject slot resolves to — by the style word; `1269h:0008h` keeps it on
the playfield.

The measure starts with the room for three lines (`0x12729`); each line
adds its characters and one for the line to the count (`0x1277C`,
`0x127A5`) and takes a line height off the room while one is left
(`0x127B1`). The widest line and the heights added up are the extent, and
the frame adds 8 to the width and 6 to the height (`0x127C3`, `0x127C8`).
The count less one is the answer (`0x12861`), and a page of one empty
line answers 0, which the caller takes for no page at all (`0x0D800`).

Bits 0–1 of the style word place the box up and down, through the jump
table at `0x127DF`:

| Bits 0–1 | Placement |
|---|---|
| 0 | ten rows above the anchor when the box and what is left of the room fit above it, ten below otherwise (`0x127E4`–`0x1280C`) |
| 1 | ten above (`0x12800`) |
| 2 | ten below (`0x1280E`) |
| 3 | centered on the anchor (`0x12810`) |

Bits 2–3 then place it across (`0x12824`): 0 centered on `x`
(`0x12837`), 4 its left edge on `x` (`0x1284B`), 8 its right edge on `x`
(`0x12853`). The clamp holds the box inside (0, 0, 320, 128), which is
what the caller hands it (`0x0D524`), one axis after the other: an edge
before the near bound is put on it (`0x126C9`, `0x126EA`), and only
otherwise a far edge past the far bound is pulled back to it (`0x126DA`,
`0x126FE`) — so a box larger than the playfield ends with its near edge on
the near bound when it began before it, and with its far edge on the far
bound otherwise.

The style word is the low twelve bits of the speaker's slot
([tasks](tasks.md#subject-slots)), and a hotspot's own line has style
word 1: ten rows above its anchor.

**Painting**, `1269h:01E0h`, puts one of two backgrounds under the box,
never both. Bits 6 and 7 of the style word at `40h` choose a flat panel in
the style's own color (`1AFAh:0092h`, `0x128D5`); every other value
translates every pixel under the box through the flattening table at
`[1CB0h]` (`0724h:0221h`, `0x128F5`).
Then the text, inset four across and three down. The ink starts as the
style's first byte and stays across lines; `01 n` switches it, and `03 n`
is stepped over (`0x1290F`, `0x129C6`). A glyph's pixels are offsets into
a two-entry ramp: the ink is the outline and the next entry the body.

## See also

- [Tasks](tasks.md) — the pages, their timing and the controls the scanner acts on
- [Dialogue](../formats/dialogue.md) — the banks the texts come from
- [Graphics](../formats/graphics.md) — the font
- [Verbs](verbs.md) — the status line
