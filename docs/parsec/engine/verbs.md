[← Documentation index](../README.md)

# Verbs and the Mode Machine

*PARSEC — Dunkle Schatten (1994): `XP01.EXE`, as its unpacked load image of 150 096 bytes shows it. Addresses `0x…` are linear offsets into that image; `seg:off` names a routine by its far address.*

Everything a click does goes through one word, the interaction mode
`[288Fh]`, and the block of words around it. A click on the strip under
the playfield picks a verb or an item; a click in the picture picks an
object; the machine decides from the mode what that means, walks the
figure there, and asks a table of rules what the verb does to the
object. A rule's handler is code, and what it does in the end is name a
scene for the [tasks](tasks.md) to play.

## The block

The entry function clears `38h` bytes at `DS:288Dh` (`0x0EDA8`), so a
fresh location is mode 0 with nothing chosen.

| Word | What |
|---|---|
| `[288Dh]` | the verb |
| `[288Fh]` | the mode |
| `[2891h]`, `[2893h]` | the object chosen and its class |
| `[2895h]`, `[2897h]` | the second object and its class |
| `[2899h]` | the two-object flag: 1 nothing to do, 2 something to do |
| `[289Bh]` | the bits: 1 a walk is under way, 2 input belongs to a script (`0x1041A`), `10h` wait for the tasks, `80h` wait for the figure |
| `[289Dh]` | the verb as chosen, before a rule changed it |
| `[289Fh]` | the node the figure is walking to |
| `[28A3h]`, `[28A5h]` | what the pointer was over last frame |
| `[28BBh]`, `[28BDh]` | the gate: a far routine the machine asks before it goes on |
| `[28C3h]` | the status line's refresh age |

The modes: 0 nothing chosen; 1 a verb parked; 2 a verb taken; 3 a second
object wanted; 4, 5 and 6 acting, entered from 0, 2 and 3 once a rule
answers; 7 and 8 the road out of a room that a script asked for.

## The strip

The strip's widgets are one table of seventeen 18-byte records at
`2115:0000`, the same mouse-object shape a room's hotspot table carries
([hotspots](../formats/hotspots.md)): the whole strip (`114h`), six verb
buttons of 56×22 in two rows, the two arrows (`115h`, `116h`), and eight
inventory slots from `11Eh` (`0x0CAB6`, `0x0CAA1`). Reading the table is
the only way to know where a button is or which verb it picks: an id
above `100h` is a widget (`0x0FBF2`), and its low byte is the verb
(`0x0FBFB`) that goes into `[288Dh]` (`0x0FC1D`). The buttons, in reading
order, are `103h`, `104h`, `102h`, `106h`, `105h`, `101h`; `103h` is one
button for two verbs, *Nimm/Gib*. A slot holds the item `scroll + n` of
what is carried (`0x0CAC0`); the carried list is kept sorted, a new item
shifted in from the end (`0x0CD6F`) and one given away shifted out
(`0x0CDFA`).

A click on a verb button is a nine-arm switch through the table at
`CS:148Bh`, with five distinct arms. In modes 0, 1 and 3 the verb is
taken — mode 2 — and what was chosen before is forgotten
(`0x0FC12`–`0x0FC26`). In mode 2 the same verb again parks it, mode 1
(`0x0FC6D`). Modes 4, 5 and 6 ask the gate with 1 (`0x0FC88`), mode 8
with 2 (`0x0FC97`), and both end in mode 1 with the new verb
(`0x0FCA4`). Mode 7's arm does nothing (`0x0FFEF`). Parking is not
un-picking: the verb stays and the status line still names it, but the
picture's side answers only in modes 0, 2 and 3 (`0x0F5F4`), so in mode 1
a click on a hotspot does nothing. The sixth verb takes a road of its
own (`0x0FC3A`): its rule installs the options gate.

## A click in the picture

`0EB9h:0C58h` is a click on an object of the room or an item of the
strip. Modes 4 to 8 go through a table at `cs:0D7Bh` (`0x0F7FA`): the
same object again takes modes 4, 5, 6 back to 0, 2, 3 (`0x0F818`);
another asks the gate. In modes 0 to 3 the object is chosen
(`0EB9h:0A0Dh`, `0x0F80C`). Then, in modes 0 and 2 with something to do,
a two-object verb — 5 and 9 — wants its second (`0EB9h:0776h`, mode 3),
or else the machine asks whether a rule answers for the pair without
running it (`0EB9h:07C7h`), goes to the acting mode — 4 from 0, 5 from 2,
6 from 3 — and walks the figure to the object (`0x0F8D4`); without a walk
the arrival comes at once (`0x0F905`).

Choosing, `0EB9h:0A0Dh(id, hotspot)`, gives the object a class: an id
below `17h` is an item, class 0 (`0x0F5AC`); a hotspot takes its class
from bits 8 to 11 of its verb mask (`0x0F5EE`). The hover rules then run
— the search with flag 1 (`0x0F636`) — and their answer sets the
two-object flag (`0x0F648`, `0x0F663`); mode 2 runs the search with flag
2 (`0x0F6BA`), mode 3 with `10h` for the second object (`0x0F796`).

## The step, once a frame

Every frame of a visit the location's handler calls the dispatcher
`0EB9h:0FCEh` (`0x9C8`). Its head is the machine's step (`0x0FB69` to
`0x0FB91`): it asks what task 0 is doing, runs the step on that answer,
and ends the frame when an exit is asked for, `[1F48h]` not zero.

The step `0EB9h:0965h` goes through a table at `cs:0A03h` for modes 4 to
8 (`0x0F4FB`). In modes 4, 5 and 6 the figure standing with nothing busy
is an arrival (`0x0F519`): `0EB9h:087Fh` runs the rules with flag 4 for
modes 4 and 5 (`0x0F446`) and with `20h` for mode 6 (`0x0F4B6`); a figure
that stopped short of its node runs them with `1080h` and then `80h`
(`0x0F4D3`). Mode 7 becomes 8 (`0x0F51E`); in mode 8 the gate is asked
with 0 (`0x0F567`), and then the status line's age `[28C3h]` is set to
`0Ah` (`0x0F56D`), so the status line's gate composes the line afresh
once the scene or the talk that held the machine is over
([the status line](#the-status-line)). Captain Zins's step (`0A01h:09E0h`, `0x0AA62`) and Bi-Fi 2's
(`09B6h:098Dh`, `0x0A55F`) go straight on from the gate to the door that
was come through and leave the age as it stands.

The dispatcher's tail (`0x0FDE1` to `0x0FFEF`) is the pointer's own
effect, while nothing is busy. The same record under the pointer as last
frame does nothing; over a hotspot the hotspot's first text is the hover
text (`0CFDh:1810h`, `0x0FE8F`) and the hotspot is chosen (`0x0FEC6`);
over nothing the hover text stops and modes 0 and 1 reset (`0x0FED5`);
over the strip a verb button is taken in mode 1 (`0x0FF9F`) and an item
is chosen (`0x0FFC8`).

## The rules

What a verb does to an object is a table, not a switch. A rule is
fourteen bytes (`0x13E4F`): a verb, 0 for any; a flags word saying which
searches it takes part in — bit 0 while the pointer only names things,
bit 2 for acting on one object, bit 5 for acting with two; an object, by
the item row's number, negative for a class and −1 for anything; a second
object; and the far address of its handler. The location row's `+2Bh`
points at the location's own table in segment `1FFCh` (`0x0EDD0`), and
the program installs one global table beside it.

The search `13D0h:009Ch(flags)` takes the location's list first
(`0x13DA8`): rules for the object `[2891h]` (`0x13DB3`), for its class
complemented (`0x13DBC`), and for object 0 (`0x13DD5`); then the global
table the same way (`0x13DDC`); then both lists from their first rule for
any object (`0x13E04`, `0x13E13`). A rule takes part when its flags meet
the search's (`0x13E54`) and its verb is this one or any (`0x13E5E`). A
search for two objects — flags `10h` or `20h`, without `2000h` — also
holds the rule's second object against the one chosen (`0x13E6B`–`0x13E90`):
the object itself, its class complemented, 0 or `FFFFh` match, and a rule
whose second is any other is passed over, so a two-object rule answers
only for its own second; the search with flag 1 asks neither the verb nor
the second (`0x13E58`). Captain Zins's matcher (`0x0F2C7`) and Bi-Fi 2's
(`0x0E577`) hold it the same way. The handler is called with the flags
(`0x13EB3`). An answer of 0 keeps
looking (`0x13EBD`), −1 starts over (`0x13E32`), and nothing found
answers −1 (`0x13E3A`).

What a handler's answer does to the machine is `0EB9h:0836h`: 8 to 12
set a bit of `[289Bh]` (`0x0F3C9`) — 9 waits for the figure (`0x0F3E2`),
10 for the tasks (`0x0F3EA`) — and anything else resets the machine,
`0EB9h:003Fh`: mode 0, no verb, nothing hovered (`0x0EBCF`–`0x0EC41`).

The generic handlers in segment `1586h` answer what the location's own
rules leave open, each with a hotspot text of its own when the hotspot
carries one and a line of bank 999 otherwise:

| Handler | Verb | What it plays |
|---|---|---|
| `1586h:0125h` | *Betrachte* | the hotspot's fourth text (`0x15996`), else bank 2's place for an item, else bank 999's place 16 |
| `1586h:017Ch` | *Sprich mit* | the hotspot's fifth text, else place 4 for something carried and 3 for anything else |
| `1586h:01CCh` | *Nimm* | the hotspot's sixth text; else an item that is there and live goes out of the room — its live bit cleared (`0x15A8F`) and its own hotspot with it (`14DDh:074Bh`, `0x15A98`) — and into the player's hands (`0x15AB2`); else one of places 9, 10 and 11, rolled |
| `1586h:0273h` | *Benutze* | place 7 for something carried; else the hotspot's third text, else place 6 |
| `1586h:02C3h` | *Benutze … mit* | place 15 when both are class 2, 12 when the second is, 8 otherwise |
| `1586h:00D3h` | *Gib … an* | one of places 0 to 2 when the second object is carried; else, for a class-2 second object, its record in subject slot 9 (`0x1596C`; the setter stamps `3000h` when the room has no such object) and place 14; else 5 |
| `1586h:04A3h` | the third button | *Gib* for something carried, *Nimm* otherwise (`0x15D14`) |
| `1586h:02E9h` | walk | the figure to the object, and on through a door |

## The road out of a room

Two gates and the rule between them make it. A `03 78` control in a text
(`1586h:05E1h`) chooses the door: the hover is forgotten, the gate
`1586h:059Fh` installed (`0x15E4F`), input locked, and the machine put
into mode 7 (`0x15E78`). When the tasks fall idle the step asks the gate,
which makes the walk verb `0Ah` the chosen one, mode 4, and walks the
figure to the door's node (`0x15E29`). When the figure stands there the
arrival runs the rules with flag 4 and finds the walk verb's rule, which
installs the second gate, `1586h:0000h` (`0x15B52`), and walks the figure
on to the door's twin, its id plus `7D0h` (`0x1588B`). When it stands
there the step asks that gate, which finds the door among the eight exit
ids of the location's row (`0x158CE`) and its destination beside it
(`0x158E7`) — a destination below 61 and of a kind other than 0
(`0x158FC`, `0x1590B`) — and puts the exit into `[1F48h]` and where it
leads into `[1F4Ah]` (`0x15925`, `0x15929`). The visit is over.

The third gate, `0EB9h:0489h`, is installed by the sixth verb's rule
(`0x0F056`) and by Escape (`0x0FD3E`), and opens the game options
([options](options.md)).

## The status line

The line in the band under the playfield names what a click would do. The
templates are a row of sixteen far pointers at `DS:1084h`, one per verb,
each a format with `%c` ink controls — *`%cBetrachte %c%s`*,
*`%cBenutze %c%s%c mit %c%s`* — and the composer picks the inks by the
mode (`0x10056`–`0x10092`). `[28C3h]` is the age that says when it is
composed again: while the age is not 0 and the bits `[289Bh]` hold
neither 1 nor 4, the line's gate counts it up and composes the line once
it was above 3 (`0x101EB`) — above 2 in Captain Zins's gate (`0x0B63F`)
and in Bi-Fi 2's (`0x0B105`).

## See also

- [Hotspots](../formats/hotspots.md) — the records a click in the picture lands on, and the strip's widget table
- [Tasks](tasks.md) — what a rule's scene is played by
- [Walking](walking.md) — the walk a click starts
- [Text](text.md) — the `03 78` control and the others
