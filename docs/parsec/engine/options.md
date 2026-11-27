[← Documentation index](../README.md)

# Options and Saving

*PARSEC — Dunkle Schatten (1994): the game options, their confirmations, `SAVEGAME.SAV`, `SAVEGAME.CFG` and location 60, as `XP01.EXE` writes and reads them. Addresses are linear offsets into the unpacked image of 150 096 bytes; `[nnnnh]` and `DS:nnnnh` are words of `DGROUP`.*

There is one saved game and no slot. The game options are a menu over the
playfield; saving writes the file at once, loading in game reads no file
at all, and the settings file is written when the program ends.

## The game options

The options are `11A0h:0451h` (`0x11E51`): eight rows over the playfield —
music, mouse, text speed, walk speed, save, load, quit, back to the game.
They open in two ways: once at the program's start, before the main loop
(`0x4F87`), and in game through the gate `0EB9h:0489h` (`0x0F02E`), which
Escape puts up (`0EB9h:1190h`, `0x0FD20`).

The rows are records in `DGROUP` (`DS:1454h`), reached through a
null-terminated far-pointer row at `DS:14A4h`: ink, ground, row, the
current index, and a row of far pointers to the texts. The index lives in
the record, so a setting shown is what the record says and stays so
between openings. Every string comes out of the image.

The menu runs as a loop of its own inside the call. Opening it takes the
pointer off, writes the gray ramp again, holds the status band dark and
freezes the strip (`0x11EAE`–`0x11EBB`), flattens the playfield and draws
the rows into the back buffer. It then **dissolves** in: the loop copies
byte `n` and its mirror `9FFFh − n` from the back buffer to the screen,
`n` drawn from the game's generator as `rnd(0, 5000h)` (`0x1208C`), until
`[1C82h]` has counted a hundred hundredths (`0x120CE`), and then the whole
picture. How many draws that is depends on how fast the machine runs the
loop and is not in the program: on DOSBox-X with `core = normal` and
`cycles = fixed 25000`, the game started through its batch file, it is
158 215, measured on the generator's seed before and after the loop —
158 282 in Captain Zins and 158 808 in Bi-Fi 2, measured the same way
([departures](../departures.md)). Keys
typed during the dissolve are flushed (`0x12106`).

Each pass of the loop then polls the input once, follows the pointer and
takes at most one event (`0x1210B`–`0x125D4`): the row left is redrawn in
its own ink, the row entered in `FEh`. A click on a setting's row steps it
to its next text (`0x12482`–`0x1258F`):

| Row | What it sets |
|---|---|
| Music | the switch `[212h]`, toggled (`0x12304`); the drivers are told at once, the kept song stopped with its digital voices or started again; off, it silences the sound effects too (`0x621F`) |
| Mouse | `[16F3h]`; `147Ah:05DCh` (`0x14D7C`) sets the two mickey divisors `[3DCh]` and `[3DAh]` from it, through a table at `cs:062Dh`; a setting past 4 becomes 1 |
| Text speed | three rows of four words, slow to fast, copied to `[167Dh]` on the close — the base, per-line, per-character and ceiling weights a caption page is timed by |
| Walk speed | four bytes, slow to `****`, copied to `[16A7h]`; the walk row skips `****` unless the command line's `#SpeedHigh` set bit 4 of `[1EDCh]` (`0x1236F`) |

The close (`0x125D7`–`0x12677`) takes the text speed and the walk speed
from their rows — the settings marked changed if the walk speed moved —
takes the boxes away, writes the gray ramp and the strip back and puts
the pointer's mode back.

### The confirmations

Save, load and quit ask first, through the box `11A0h:00D2h`
(`0x11AD2`–`0x11CC7`): the question and its two answers, text width plus
16 wide and 60 rows tall, centered on the playfield. A click answers with
the box hovered; Tab moves to the other answer. What an answer does
(`0x1239D`–`0x12417`):

- **Save**, yes: the game saved, the menu closed.
- **Load**, yes — *Tatsächlich neu einladen?* (`DS:1430h`) — the menu
  closed and the main loop sent to location 60.
- **Quit**, yes: a second box asks whether to save first; either answer
  closes the menu with `[1F4Ah]` at −2 (`0x12403`), which ends the main
  loop. No to quitting steps the quit row, which has one text and stays
  (`0x123D5`).

## `SAVEGAME.SAV`

The save, `147Ah:044Eh` (`0x14BEE`), writes nothing before the first
location has been entered (`0x14BF2`, `[1F4Ch] < 0`). Otherwise it copies
the location's objects into their own row (`0x14C07`), pushes the location
onto the location stack (`0x14C0F`), copies five blocks of the data
segment into a far buffer of `8B1h` bytes, pops the location again
(`0x14CA3`), and writes the buffer whole as `SAVEGAME.SAV`
(`147Ah:0241h`, `0x149E1`). The name carries a leading `*`, which means
the name as it stands, in the current directory (`0x66D5`).

| Offset | Size | What |
|---|---:|---|
| `+00h` | 8 | `DSchSAV\0` (`0x14A66`–`0x14A77`) |
| `+08h` | 2 | the word `[1718h]` (`0x14A7D`) |
| `+0Ah` | 4 | the BIOS tick of the moment, `0:046Ch` low and `0:046Eh` high (`0x14A11`, `0x14A15`) |
| `+0Eh` | 4 | the sum |
| `+12h` | 2 | the xor |
| `+14h` | `45h` | the story bytes, from `DS:26F6h` (`0x14C2A`) |
| `+59h` | `7A0h` | 61 object rows of 32 bytes, from `DS:1F56h` (`0x14C42`) |
| `+7F9h` | 8 | the location stack, from `DS:1F4Eh`: a count word, then the locations (`0x14C5B`) |
| `+801h` | `30h` | the player's record, from `DS:1693h` (`0x14C73`) |
| `+831h` | `7Fh` | from `DS:15FEh` (`0x14C8B`): the carried items, and past them the far pointers to the items' cels at `DS:1621h` (`0x0C29F`) |
| `+8B0h` | 1 | cleared (`0x14A60`) |

The player's record holds three far pointers into the program's heap — the
`AN` entry at `+18h`, a pointer at `+20h`, the walk graph at `+24h` — and
the save writes them as they stand in memory; so do the items' cel
pointers in the last block. They are addresses of the run that wrote the
file.

**The check.** The tick at `+0Ah` seeds the game's own generator
(`1EB5h:000Ah`, `0x14A2D`); its first draw `rnd(1, FFFEh)` starts the
sum, sign-extended (`0x14A39`), and its second starts the xor
(`0x14A51`). Then `453h` words from `+14h` are each added into the sum
sign-extended, with the carry into its high word, and xored into the xor
(`0x14A8A`, `0x14A93`, `0x14A9E`–`0x14AAC`). `453h` words are `8A6h`
bytes, nine past the file's end, into the segment's padding, which is
zero. Saving therefore reseeds the generator the game rolls with.

**Reading.** The file is read once, at the program's start
(`147Ah:0072h`), into the same buffer, and only when it is exactly `8B1h`
bytes (`0x148ED`). The check is computed again — reseeding the generator
whether it holds or not — and the sum, the xor and the magic compared
(`0x149A3`–`0x149C5`); a file that fails ends the program with a message
(`0x149C7`). One that holds marks a game held, `[1744h]`, and is restored
at once through location 60 (`0x149D3`).

## Location 60

Loading in game, `147Ah:0518h` (`0x14CB8`), reads no file: with a game
in the buffer (`[1744h]`) it sends the main loop to location 60, entry 1.
Row 60 is of kind 4 and has no name, no rules and no hook; its handler,
`147Ah:052Ch` (`0x14CCC`–`0x14D7B`), has no frame loop. It copies the
location stack and the player's record back, sets `[1F4Ch]` to −1, pops
the location the game was saved in into `[1F4Ah]` with entry −1, and
copies the story bytes and the object rows back — and `23h` bytes of the
last block, though the save copied `7Fh` (`0x14D72`). The heap pointers
past them are not taken back. Then it returns, and the main loop goes on
in the saved location.

## `SAVEGAME.CFG`

The settings file is 31 bytes, written after the main loop (`0x4FF0`,
`147Ah:0373h`, `0x14B13`) when a setting changed, `[1ED2h]`:

| Offset | Size | What |
|---|---:|---|
| `+00h` | 8 | `DSchCFG\0` (`0x14B35`–`0x14B44`) |
| `+08h` | 2 | the word `[1718h]` |
| `+0Ah` | 4 | the BIOS tick |
| `+0Eh` | 4 | the check value |
| `+12h` | 2 | the mouse setting `[16F3h]` |
| `+14h` | 8 | the four text-speed words from `[167Dh]` |
| `+1Ch` | 2 | the walk speed `[16A7h]`, its byte widened (`0x14B68`) |
| `+1Eh` | 1 | 0 |

The check value (`147Ah:0007h`, `0x147A7`) starts at the tick, which also
reseeds the generator; each of the six words from `+12h` is added,
unsigned (`0x147E4`), and the value multiplied by 997 after each
(`0x147EE`). At the start the file is taken only when it is exactly `1Fh`
bytes (`0x1485B`), its magic holds and then its check value
(`0x148A6`); a file that fails is passed over. Taken, it sets the mouse
setting and its divisors, the text speed and the walk speed
(`0x148B6`–`0x148D7`).

## See also

- [Boot and the title program](boot.md) — when the start's load runs
- [Game loop](game-loop.md) — the location stack's words and the main loop
- [Tasks](tasks.md) — the caption timing the text speed sets
- [Walking](walking.md) — what the walk speed scales
