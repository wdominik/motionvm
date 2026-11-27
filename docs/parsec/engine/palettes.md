[← Documentation index](../README.md)

# Palettes

*PARSEC — Dunkle Schatten (1994): which colors the DAC holds, how the panel's variants and the ramps replace them, and the flatten lookup the captions paint through. Addresses are linear offsets into the unpacked `XP01.EXE` (150 096 bytes); `seg:off` pairs are far addresses of the load image.*

The program never loads one 256-color palette. It writes **banks** —
`start`, `count`, then that many 6-bit triples
([graphics](../formats/graphics.md)) — into the DAC, each into the range
that is its own, and the screen's colors are the sum of whoever wrote
last:

| Range | Whose |
|---|---|
| 0–127 | the room's bank, out of its slot |
| 128– | the player's bank, `PLAY00.DAT` resource 0 |
| 160– | the strip's bank, `PANELS.DAT` resource 1, in one of three variants |
| 192–207 | the ramp: the game's gray ramp, or an exit transition's tinted one |
| 224– | the pointer's bank, `CMBMOU.DAT` resource 0 |

A displayed DAC color and the index a caption's flattening maps a pixel to
are separate pieces of state, kept by separate writers.

## Two writers

The resource loader `0590:0A1F` (`0x0631F`) reads a bank's start and count
(a count of zero is 256), then calls the raw writer `0590:09E2`, which
puts the triples into the DAC and the program's shadow of it, and
`0724h:01E1h` (`0x07421`), which updates the flatten lookup for that range.
Calling the raw writer alone does not update the lookup — and the panel's
variants (`0x0C3E8`), the gray ramp and the transition ramp all call it
alone.

A room's banks are loaded by the entry pass `14DDh:0293h`, one
`0590:0A1F` each (`0x15313`–`0x1535F`). A bank that starts at register 0
has its first color blacked before it is loaded, `cmp byte es:[bx], 0 ;
jnz` and three byte stores of 0 (`0x1532F`), so color 0 is black whatever
the room's file holds there — the same test stands in Captain Zins's pass
(`0x10746`) and Bi-Fi 2's (`0x0F980`). The visit's setup that runs that
pass comes after the leaving has played its exit effect synchronously
(`0x0ED4F`), so the exit's picture is colored by the old room's bank, and
the new room's reaches the DAC only once the exit is over.

## The flatten lookup

Under a caption box the painter does not dim the picture arithmetically:
it maps every pixel of the rectangle through a 256-byte table in its own
code segment. `0724h:01A6h` builds it, filling every entry with one color
and mapping one run to itself (`0x073EE`–`0x07405`); Dunkle Schatten calls
it with `(0C0h, 10h)`, so everything becomes color 192 except 192–207,
which stay. From then on every resource load rewrites the entries of the
colors it loaded (`0x07421`): a triple's weight is `5·r + 9·g + 2·b`, and
its entry is 192 plus that weight scaled into the run's sixteen steps —
the gray of the ramp nearest the color's brightness. The painter reads
the table (`0x07461`); it computes nothing while it paints.

## Channel curves

`0724:0360` (`0x075A0`–`0x075EE`) builds a 64-entry channel table from a
start value and a divisor. It is not a multiplier: an 8-bit countdown
between entries decides where the value steps up, so outputs repeat
unevenly. Four rows exist. Rows 1 to 3 are built at the start with
immediates (`0x059A3`, `0x059B1`, `0x059BF`); row 0's divisor is a word of
`DGROUP` the start pushes (`0x05993`), and row 0 is not the identity.

## The panel's variants

The strip's bank is kept three times, behind three consecutive far
pointers at `DS:1CC4h`: a tinted variant and a dimmed one, each built from
the panel resource by picking a channel row for red, green and blue
(`0x0C32E`–`0x0C36A`, the rows pushed at `0x0C332` and `0x0C354`), and the
original. `0C1C:0202` (`0x0C3C2`) writes a variant through the raw writer,
only when it differs from the one at `DS:04E1h`, and replaces only that
bank's registers.

Which variant shows is a matter of owners, not of one flag:

| Word | What it holds |
|---|---|
| `DS:04ECh` | owners asking for the tinted panel and the flattened band |
| `DS:04EEh` | releases pending, applied when the refresh countdown runs out |
| `DS:04F0h` | owners suppressing a panel redraw |
| `DS:04F2h` | pending releases of that suppression |
| `DS:04F4h` | owners freezing refresh, as a choice menu does |
| `DS:04F6h` | panel refresh calls remaining — calls, not ticks |

`0x0C971` tints at once and cancels the owner's pending release;
`0x0C9CA` selects the dimmed variant at once and freezes refresh;
`0x0CA49` asks for a release three refresh calls later. The refresh
(`0x0C451`–`0x0C4A1`) counts down, stops even the countdown while a menu
holds it frozen, and on expiry applies the releases: the tinted variant
while an owner still asks for it, the original otherwise. The visit
acquires owner 4 on entry (`0x0EEF0`) and releases it as the entry
finishes (`0x0ECF4`); the leaving takes it again.

## The ramps

**The game's gray ramp**, `0590:18EF` (`0x071EF`–`0x07236`), fills 48
bytes backwards from 63 in steps of four, passes all three channels
through row 0 and writes the sixteen colors at DAC 192 through the raw
writer. The entry's finish calls it after the entry effect (`0x0EC84`),
and the options call it as they open (`0x11E5C`) and close.

**An exit transition's ramp**, `10B8:0210`, is sixteen
entries 0, 4, …, 60 in the channels its argument enables — bit 0 red,
bit 1 green, bit 2 blue, and no bit at all is gray (`0x10DB7`) — written
to 192–207 (`0x10E2C`). The leaving hands it a tint rolled from 1 to 6
(`0x0ED42`); the transition flattens the old picture onto that ramp and
darkens it as it goes. The gray ramp comes back with the next entry.

## See also

- [Graphics](../formats/graphics.md) — the bank format, pictures and cels
- [Text](text.md) — the caption painter that reads the flatten lookup
- [Game loop](game-loop.md) — the entry, the leaving and the synchronous effects
