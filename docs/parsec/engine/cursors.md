[← Documentation index](../README.md)

# The Mouse Pointer

*PARSEC — Dunkle Schatten (1994): `XP01.EXE`, as its unpacked load image of 150 096 bytes shows it. Addresses `0x…` are linear offsets into that image; `seg:off` names a routine by its far address; `[nnnh]` is a word of `DGROUP`.*

The pointer is a library of its own, segment `179Eh` (`0x179E0`), with its
state in `DGROUP`. It does more than follow the mouse: it animates, it
takes another shape over a region that asks for one, it shows a waiting
shape when nothing has moved for a while, and it runs on the timer
rather than on the frame loop.

## Descriptors

A pointer is a 32-byte descriptor:

| Offset | Field | Read at |
|---|---|---|
| `+0` | far pointer to the cel table — resource 1 of the pointer file, `[195Eh]` | `0x189DB` |
| `+4`, `+6` | signed offset from the mouse to the cel's corner | `0x188A3`, `0x188AF` |
| `+8` | flags: bits 0–1 the step mapper, 2–3 the duration function, `10h` one-shot, `20h` taken at once, `80h` no resume, `100h` static, `200h` hold the last cel | `0x189C1` |
| `+0Ah` | the first cel, an index into the table | `0x189D3` |
| `+0Ch` | a duration | `0x188DE` |
| `+0Eh` | the number of animation steps | `0x18826` |
| `+10h` | a second duration, a modulus, or one byte per step | `0x188E7`, `0x1890B`, `0x188F1` |

The loader reads the descriptors out of the data segment and centers four
of them on their cels, `−(w >> 1)`, `−(h >> 1)` (`0x05DFA`, `0x05E47`,
`0x05E6E`, `0x05E95`); then it copies `0234h` into the slot, starts the
library (`179Eh:0A00h`), hides the pointer (`0x05EFB`) and polls the input
once (`0x05F00`) (`0x05D45`–`0x05F0A`).

## Selectors, and the mode setter

A **selector** names the descriptor shown. Below `80h` it is `DS:0334h`
plus the selector (`0x189B5`): 0 is the slot the mode setter fills, `40h`
the one the timeout shows. From `80h` on it is a mouse region's shape,
`[3B4h]` plus the selector less `80h` (`0x1897F`).

`179Eh:0F78h` takes the selector asked for in `[326h]`. While the
timeout's `40h` is shown (`0x18963`), asking for it again does nothing, a
negative ask goes back to the one remembered in `[328h]` (`0x18993`), and
any other is noted there for later (`0x18997`). Otherwise a negative ask
withdraws (`0x18968`) and any other is shown (`0x1896C`). The descriptor's
flags go to `[31Eh]` (`0x189C5`), its first cel to `[32Ch]` (`0x189E3`),
and the animation starts from nothing (`0x189ED`).

The program changes the pointer by **mode**. `0590h:064Dh(mode)` notes the
mode in `[1962h]` (`0x05F51`, `0x05F5A`) and, for modes 1 to 4
(`0x05F60`), copies one of the four descriptors at `0234h`, `0274h`,
`02B4h`, `02D4h` — the table at `cs:06A1h` — over the slot `0334h`
(`179Eh:0960h`, `0x05F74`). A copy over the descriptor shown works the
shape out again at once (`0x18386`–`0x1838C`). The tasks ask for a mode
three draw passes out (`0CFDh:06AFh`); their default is mode 1 when the
figure file is loaded and 2 otherwise (`0CFDh:06CDh`, `0x0D69D`).

## Regions

A mouse region is a rectangle with a shape word. A region holds the mouse
when `x <= mouse x < x + w`, and the same for y (`0x1866A`, `0x18672`,
`0x1867A`); the same region as last time changes nothing (`0x1868A`). Its
shape word `+6` (`0x186A0`) gives the shape while hovering in its low byte
and, while a button is down (`[3D5h]` bits 0–1, `0x186AC`), in its high
byte; shape `n` is the descriptor `[3B4h] + (n − 1) · 20h` (`0x186BD`),
asked for as selector `80h + (n − 1) · 20h` (`0x186C8`) and taken at once
when the region's `+8` carries `20h` (`0x186CE`). Under no region a
region's shape goes back to selector 0 (`0x18716`).

The room's hotspots and the strip's widgets are regions whose shape word
is 0. The one region with a shape is the choice menu's (`179Eh:0210h`,
`0x0DC91`; out again with `179Eh:0730h`, `0x0D55F`), and its two bytes
are the same shape, `0101h`.

## The timer, and what a step does

The library registers a slot with the scheduler: a countdown, the calls
not yet made, and a reload, the interval `[314h]` in hundredths
(`0x05EE0`, `0x1A801`–`0x1A805`). Each interrupt counts the hundredths off
the countdown and, when it runs out, the callback `179Eh:0A90h` is called
with how many intervals passed — mouse units, two hundredths each
(`0x1A226`–`0x1A250`). The pointer moves and animates on that clock, in
step with the interrupt and not with the frames.

The callback adds the units to the backlog `[318h]` (`0x18477`); a frozen
pointer, `[316h]` negative (`0x18484`), only counts. Otherwise
`179Eh:0AB2h` takes the backlog whole as the step (`0x18492`) and runs the
movement (`179Eh:0AF9h`, `0x184A3`) and the shape (`179Eh:0C1Bh`,
`0x184A6`); a drawn pointer is drawn again only when something changed
(`0x184A9`), and after drawing what was pending is cleared
(`0x184BF`–`0x184C4`). The position is clamped to the limits at `[30Ah]`
— x min, y min, x max, y max — (`0x185B2`–`0x185C1`, `0x185F1`–`0x18602`),
and the cel is drawn at the mouse plus the descriptor's offset, `[3EEh]`.

## Animation

`0x187C6`–`0x18895` steps the shown descriptor by the units. A static one
(flags bit 8) only takes a new selector that is asked for (`0x187CD`).
Otherwise the time left `[320h]` loses the units (`0x1880C`) and, strictly
below 0, the step counter `[324h]` moves on (`0x18822`); past the last
step (`0x18826`) a selector asked for is taken (`0x1882C`), a one-shot
ends (`0x18837`), and any other goes round to 0 (`0x1883D`). Each step
adds its duration (`0x1884C`) until the time left is not below 0
(`0x18850`); then the counter is mapped once (`0x18855`) and added to the
first cel (`0x1885A`).

The duration function is chosen by flags bits 2–3 (`0x18802`): the
duration `+0Ch` (`0x188DE`); `+10h` for the first step, else `+0Ch`
(`0x188E3`); `+0Ch` for the first step, else the byte at `+10h + step − 1`
(`0x188EC`); or `+0Ch` plus a new random number modulo `+10h`
(`0x188FA`). The mapper is chosen by bits 0–1: the counter as it is
(`0x18950`); backwards, `count − 1 − step` (`0x18951`); up to half the
count and back down (`0x1891F`, `0x1892B`); or a random walk that keeps
the step when a new random number's low byte has even parity and steps two
back otherwise, wrapping below 0 to the last step (`0x18930`–`0x1894D`).

The library's generator is its own: `[1864h]`, multiplied by `3E5h`, with
`3F1h` and the product's high word added with the carry
(`0x188FD`–`0x18906`).

## The timeout

`[3C4h]` counts down while nothing happens, and `[3C2h]` is its reload.
Polling the input (`179Eh:00DBh`, inside `179Eh:01B0h`) reloads it
(`0x17ABE`–`0x17AC1`); when it had run out (`0x17AC5`), `179Eh:00C0h`
gives the timed-out pointer back: the shown selector is asked for again,
or, for the timeout's own `40h`, the ask is withdrawn and taken
(`0x17AA8`, `0x17AB8`). On the timeout's own pointer that routine also
clears `[3D4h]` (`0x17AAE`), the buttons word the input's event record is
built from (`0x17B24`). `179Eh:12C7h` does the same reload inline
(`0x18CA8`–`0x18CC9`) and clears the backlog and the step
(`0x18CD5`, `0x18CDB`).

## Hiding, showing, placing

| Routine | What it does |
|---|---|
| `179Eh:0882h` | off the screen and frozen: a drawn pointer gets its background back (`0x18293`), the drawn bit cleared and bit 15 set (`0x18299`, `0x1829C`) |
| `179Eh:091Ah` | frozen no more: a backlog not below the timeout's countdown is dropped (`0x1830D`, `0x18313`), any other stepped by (`0x1831D`) |
| `179Eh:0830h` | on the screen and not frozen: the backlog dropped when twice it reaches the timeout (`0x18224`), the pointer drawn (`0x18268`) |
| `179Eh:1017h` | the shape taken again — the shown selector asked for anew, the timeout's withdrawn (`0x18A04`–`0x18A18`) |
| `179Eh:0000h(x, y)` | the mouse put at a point, inside the limits (`0x179EF`), and drawn there when it is drawn (`0x17A5B`) |

The pointer is drawn over the finished picture with a save-under in video
memory (`0x17C50`, `0x17D5F`), so it never enters the room's own redraw.

## The keys

The library takes twelve keys for itself. When it starts (`179Eh:0A00h`),
each module in its table at `179Dh:0000h` is called with `AX = 0`
(`0x18431`–`0x18447`); the second, `179Eh:13B3h`, takes INT 9
(`0x18DC4`–`0x18DE8`) and puts its step first in the movement,
`[3FEh] = 14E6h` (`0x18E03`). The shutdown gives the interrupt back
(`0x18A4E`). In between nothing takes it away; one byte, `cs:13B2h`, says
whether it acts, the image holds it at 1, and only `179Eh:14B4h` changes
it — called by the developer's room list alone (`0x0F069`, `0x0F1A6`).

The handler (`179Eh:142Ah`) reads the scan code at port `60h`
(`0x18E14`), looks its low seven bits up in a table of 60h bytes at
`cs:1352h` (`0x18E25`) and keeps the level of the slot it finds, 1 to 12,
in `cs:1346h`. A key with a slot is acknowledged and **not passed on**
(`0x18E34`–`0x18E3A`): the BIOS never buffers it, and the program never
reads it as a keystroke. The program fills the table with
`179Eh:1483h(list)` right after setting the pointer up (`0x5F10`–`0x5F14`),
from `DS:0228h`:

| Slots | Keys | What they do |
|---|---|---|
| 1–8 | Home, Up, Page Up, Left, Right, End, Down, Page Down | the eight directions |
| 9, 10 | Space, Enter | the first button, the second |
| 11, 12 | keypad −, keypad + | every direction at its slowest, at its fastest |

The step (`179Eh:14E6h`, `0x18EC6`) runs first in every movement. With the
switch clear it clears the key buttons `[3CEh]` and stops (`0x18ED7`); with
no units elapsed it stops (`0x18ED6`). A held direction's speed, one of
eight words at `[1868h]`, grows by `[3CCh]` up to `[3CAh]`, a released
one's drops to `[3C8h]` — or all eight are `[3C8h]` while keypad − is
down and `[3CAh]` while keypad + is (`0x18EE0`–`0x18F32`). A held
direction moves by its speed times the units, at most ten, over 256
(`0x18F34`–`0x18F48`), into `[3E2h]` and `[3E4h]`, where the mouse
driver's mickeys go too (`0x18F51`–`0x18F8D`). Slots 9 and 10 become bits
0 and 1 of `[3CEh]` (`0x18F91`–`0x18F9E`), which the buttons take as their
level beside the mouse's (`0x187BE`).

The movement then converts what is carried, y before x
(`0x18500`–`0x1856F`): an acceleration factor of the value's size times
`[3D8h]` plus half the last factor, at most 1000h, which from 100h up
scales the value, rounded; the value over the mouse setting's divisor,
`[3DAh]` for x and `[3DCh]` for y, is the move, and the rest is carried
only when the move is zero (`0x18571`–`0x185E9`). Resetting the library
(`179Eh:12C7h`) runs the step with nothing elapsed and clears what is
carried and both factors (`0x18CDE`–`0x18CF1`).

`INFO.DOK`, the guide the game ships, says the same to the player: the
cursor keys move the pointer, Return or Space presses the button.

Held for eight BIOS ticks, 439 ms, in the options `XP01` opens with, under
DOSBox-X, Down takes the pointer from the last row into the strip; the
rebuild held for the same time comes to rest on the same picture, every
RGB pixel. The step and the conversion are held against the original's
poll `179Eh:0AF9h`, run under an 8086 emulator, for 264 polls
(`tests/ds1_keyboard_cases.txt`).

Captain Zins's `XP01.EXE` has the same library at `163Ch`, hands it the
same twelve keys (`0x1662`, `DS:01BEh`) and keeps the words at its own
addresses — the switch `cs:04C2h`, the speeds at `[138Ah]`, `[35Eh]` for
`[3C8h]`, `[36Eh]` for `[3D8h]`, `[370h]` for `[3DAh]`. Its handler passes
an `E0h` prefix on to the BIOS where Dunkle Schatten's swallows it
(`0x18E19`); the key after it is the one with the slot either way.

Both title programs, `XP00.EXE`, carry the same library at their own
addresses — Dunkle Schatten's at `08E6h`, Captain Zins's at `0970h` —
hand it the same twelve keys (`0x0CDF`, `DS:00BCh`) and register its
timer slot with the same two hundredths (`0x0CAF`, `[1A8h]`). Both read
the settings file at their start for the mouse setting alone and set the
divisors by `XP01`'s table (`0x69E`–`0x700`). Held for eight BIOS ticks in
Dunkle Schatten's title menu under DOSBox-X, Down takes the arrow from
`(160, 100)` onto *Neues Spiel beginnen*; the rebuild held for the same
time comes to rest on the same picture, every RGB pixel.

## See also

- [Verbs](verbs.md) — what a click under the pointer reaches
- [Tasks](tasks.md) — the choice menu's region, and the mode the tasks ask for
- [Graphics](../formats/graphics.md) — the pointer file's cels
