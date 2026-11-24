[← Documentation index](../README.md)

# Game Loop

*PARSEC — Dunkle Schatten (1994): `XP01.EXE`, as its unpacked load image of 150 096 bytes shows it. Addresses are linear offsets into that image; `seg:off` pairs are far addresses of the load image, `[nnnnh]` words of `DGROUP`.*

The game is a loop over a table of locations. Each row of the table names
its own handler, and the handler is the location: it sets the room up
through the shared entry function, runs its own state machine one frame
at a time, and leaves by writing where the player goes next.

## The location table

A location is a row in a table of 52-byte records at `204Bh:0000`. The
main loop at `0x4FC0` reads the location word `[1F4Ah]`, multiplies it by
the stride and calls through the row's handler pointer — five
instructions that occur exactly once in the program. The table has 61
rows: before it dispatches, the main loop bounds the word against `3Dh`
(`0x04F8E`). Three words sit together: the entry id `[1F48h]`, the
location `[1F4Ah]`, and `[1F4Ch]`, the location the running handler
calls its own, which the main loop compares with `[1F4Ah]` after the
handler returns to see whether it moved.

| Field | What it is | Read at |
|---|---|---|
| `+07h` | the kind; zero is "not a place" | `0x04FA1` |
| `+09h`…`+10h` | eight exit ids | `0x05436` |
| `+11h`…`+18h` | the eight destinations beside them | `0x05471` |
| `+19h`… | nine object ids, the ones the entry function sets up | `0x0EE01`, bound `0x0EE53` |
| `+23h` | far pointer to the location's handler | `0x4FC0` |
| `+27h` | far pointer to the location's name | `0x1016B` |
| `+2Bh` | far pointer to the location's own interaction rules | `0x0EDD0` |

An exit is taken by looking its id up in the row's list and storing the
destination beside it in `[1F4Ah]`. The object list is positional:
position `i` names subject slot `i + 1` whether or not the positions
before it are filled. Thirty rows carry a name pointer; `0x1016B` takes it
off the row of `[1F4Ch]` and `0x10191` formats it as `[ %s (%02d) ]`.

### Kinds

| Kind | What the row is |
|---|---|
| 0 | not a place: rows 0 (named *Extro*) and 20 |
| 1, 2 | a place, shown from the `DSROOM.DAT` slot of its own number — nine rows of kind 1, the Karte among them, and fifteen of kind 2, the Zimmer among them |
| 3 | a forest row — the Wald, row 23, and the thirty maze rows 30 to 59 — shown from slot 20, the forest picture ([rooms](../formats/rooms.md)) |
| 4 | row 60, the restore ([options and saving](options.md#location-60)) |
| 5 | the F rows, which share the reveal between two of them as the forest rows do |

The kind decides which slot the room loader takes (`0x077ED`–`0x077FC`),
whether leaving fades the music and plays a transition, and which effect
an entry plays: between two places of kind 3, or two of kind 5, the entry
reveals the new picture; otherwise it slides it in.

### Rows that are not places

For a row of kind 0, and for a word past the table, the main loop does
not reach through the table at all: it calls `0x053B2` directly
(`0x04F8E`–`0x04FAB`), which is also the handler both rows name. The
routine loads no room and has no frame loop: it walks the table from the
row in `[1F4Ah]` past every row of kind 0 or 4, wrapping to row 0 when it
runs off the end, puts the first other row in `[1F4Ah]` with entry `7Fh`
in `[1F48h]` (`0x05402`) and returns. From row 20 that is the Schuppen,
row 21; from row 0, row 1.

## A visit

Every handler has the same shape: the visit's setup `0EB9h:0001h`, which
loads the room; the shared entry function `0EB9h:0207h(loc, entry)` with
the entry id it was reached through; the entry's finish `0EB9h:00B8h`;
then the handler's own switch on its state byte, run once a frame, until
a frame sets a new location. `0EB9h:017Ah` leaves:

1. The pointer off the screen (`0x0ED0A`), the interaction machine reset,
   both tasks stopped, the band held.
2. A last frame pass (`0x0ED2C`), which takes the stop requests before the
   destination replaces the task records.
3. Unless both places are forest rows: the music faded over 1 500 ms
   (`0x0ED39`), one of six transition tints rolled (`0x0ED42`), the
   transition played (`0x0ED4F`), and the music stopped (`0x0ED55`).
4. The status line set to the empty string and painted (`0x0ED5A`).

The screen effects of segment `10B8h` are synchronous: a call returns
when the effect is over. So whatever a handler does after it starts one
happens after the effect — the stop behind the transition lands a
transition later than the fade before it, and a handler that writes the
tick count `[1C76h]` right behind a blocking effect (`0xC5B`, `0x2F8F`)
writes it once the effect has played.

## The clock and the frame pass

The library hooks INT 8 (`1950h:1883h`) and registers a record at
`DS:1C74h` with the scheduler (`0x5B45`). Every interrupt, the handler at
`0x19F40` takes the PIT period that just elapsed and counts it into that
record in three groups, each by its own countdown: every `4A9h` cycles
`+0` counts up, a thousand a second; every `26D6h` cycles `+2`, `+4` and
`+6`, 120 a second; every `2E9Ah` cycles `+8` to `+0Eh`, a hundred a
second; and every tenth of those `+10h` to `+16h`, ten a second
(`0x19FC5`). `[1C82h]`, the `+0Eh` word, is the hundredths the handlers
and the effects wait on; the Zimmer waits on it (`0x9CD`).

The interrupt itself comes every period the library runs the PIT on: its
base period, `952h` cycles, 500 a second (`0x5B29`, `1950h:166Dh`,
which programs channel 0 at `0x1ABE5`), or a client's shorter one. On the
base period the `26D6h` group counts on every fourth or fifth interrupt,
the first at or after each boundary.

`[1C76h]`, the `+2` word, is the **tick account**. The
frame pass `0EB9h:01DEh` takes the account whole and clears it
(`0x0ED6E`), and hands the count round as the frame's delta
(`0EB9h:163Eh`): to the tasks (`0x101F4`), the walkers (`0x101FB`) and
the animator (`0x10202`), in that order. Then the interaction machine's
timed wait counts down: while bit 5 of `[289Bh]` is set, `[28BFh]` loses
the delta and stops at zero — a signed test, `sub [28BFh], si` then `jg`
(`0x10210`, `0x10214`). Last, the room redraws what changed and the tasks
paint their boxes.

A frame of a handler's loop is one frame pass, and takes the ticks
counted since the pass before it — none when no tick has come between.
The pass that leaving runs, right behind the loop's last, takes none
unless a tick has come between the two; the first one after a blocking
effect takes every tick the effect waited out. The entry's own animator pass runs
with a count of 0 (`0x0EC4E`), so an animation just started takes its
first step before the room is drawn.

## The screen

Mode 13h, 320×200 in 256 colors, which filled a 4:3 monitor: a pixel is
six units tall for five wide. The playfield is the top 320×128. Below it
the program reserves 72 rows at row 128 (`0x0D012`) and divides them into
a 12-row status band at row 128 (`0x0C875`) and a 60-row verb strip at
row 140 (`0x0C7E3`); `PANELS.DAT` resource 0 is the 320×72 picture that
fills the reservation. The strip's widgets — six verb buttons in two
rows, eight inventory slots and two arrows — are a table of seventeen
18-byte records at `2115:0000` ([verbs](verbs.md)).

Captain Zins's painter, `078Ah:0069h`, sets the line two rows lower and
holds the band dark from there: each glyph at row 130 (`0x07C7B`, `push
82h`) and the twelve rows flattened from 130 (`0x07D59`), where Dunkle
Schatten's `0C3Fh:005Ch` uses 128 for both (`0x0C7BE`, `0x0C89C`). The
runtime reads the row off the painter of the program it opens.

## Open questions

What is not yet known about the timing of the leaving is recorded in the
[open questions](../open-questions.md).

## See also

- [Overview](overview.md) — the subsystems and how a location comes up
- [Boot and the title program](boot.md) — how the main loop is reached
- [Music](music.md) — what the entry and the leaving do with the song
- [Rooms](../formats/rooms.md) — the slot a location shows
