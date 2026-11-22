[← Documentation index](../README.md)

# Tasks and Scenes

*PARSEC — Dunkle Schatten (1994): `XP01.EXE`, as its unpacked load image of 150 096 bytes shows it. Addresses `0x…` are linear offsets into that image; `seg:off` names a routine by its far address.*

A room that has been entered and drawn is still a still life. What speaks
in it — a line under a hotspot, a dialogue between Karsten and his
mother, a choice of answers — is played by **tasks**, and everything a
task plays is aimed at a **subject slot**. Both live in segment `0CFDh`,
and the location's handler reaches them through a handful of calls: load
a scene, arm a task, start it.

## Subject slots

The program keeps twelve slots at `DS:1D56h`, six bytes each: a code
word and, in place of the two words after it, either a far pointer to a
record or a literal point (`0x0EA6F`). Four setters write them, and the
kind they put into the code's high nibble is the whole difference between
them:

| Routine | Kind | What it takes |
|---|---|---|
| `0CFDh:1A8Ch` | `1000h` | a record, far |
| `0CFDh:1AD0h` | `2000h` | a record, far |
| `0CFDh:1B14h` | raw | a code and two words, as given |
| `0CFDh:1B40h` | `3000h` | the code alone |

The two record setters fall back to `3000h` when the pointer they are
handed is null (`0x0EA6B`, `0x0EAAF`), so an empty slot and a slot whose
object was not found read the same way. The resolver `0x0D290` turns a
slot into a screen point, switching on `code & 0F000h` through the table
at `0x0D474`; kinds `1000h` and `2000h` share one arm.

The entry function fills slots 0 to 10 before anything else in the
location runs (`0x0EDF7`). Slot 0 takes the player when the figure file
is loaded — `[1E66h]`, which `0x0EBB2` sets exactly when the far pointer
at `[2865h]` is not null (`0x0EDE0`). Slots 1 to 9 take the location row's
nine objects ([objects](objects.md#a-locations-own-objects)). Slot 10
takes a literal point, the raw setter with code `53h` and (160, 64), the
middle of the playfield (`0x0EE58`). Slot 11 belongs to the hover text,
which puts it on the pointer (`0x0E826`). The low twelve bits of a slot's
code are the style word a caption is laid out by (`0x0D3D6`), so slot
10's `53h` centers the box on the middle of the playfield with its lines
flush left.

## The two tasks

The record array at `DS:1D9Eh` has two 74-byte records; every loop over
it counts from 2 (`0x0E213`, `0x0E6DB`, `0x0DF6F`). Task 0 is the
location's scene: a dialogue, a hotspot's own line, a choice menu, the
queue of scenes behind it. Task 1 is the second voice, the one a
location hands a range of dialogues to draw from at random.

| Offset | Field | Read at |
|---|---|---|
| `+00` | countdown, in ticks | `0x0E23D` |
| `+02` | kind: 0 idle, 1 finish, 2 pages of a hotspot text, 3 a text with its cue, 4 a dialogue, 5 a menu | `0x0E24B` |
| `+04` | the subject slot the speaker is | `0x0D990` |
| `+06`, `+08` | the point the box hangs on | `0x0E45F` |
| `+0Ah` | the kind asked for next | `0x0E420` |
| `+0Ch` | a fixed duration, when a script set one | `0x0D83E` |
| `+0Eh`, `+10h` | the point the resolver found | `0x0D45F` |
| `+12h` | the style word | `0x0D7C6` |
| `+14h` | a page was just turned | `0x0D778` |
| `+16h` | bit 0 running, bits 1 and 2 the menu's | `0x0D8E8` |
| `+18h` | the speakers the dialogue uses, one bit each | `0x0D0C7` |
| `+1Ah` | bit 15 armed, bit 8 hold for a click, bit 1 protected, bit 0 repeat | `0x0E21E` |
| `+1Ch`, `+1Eh` | the range task 1 draws from | `0x0E39A` |
| `+20h` | the scratch buffer the page is copied into | `0x0D92B` |
| `+24h` | the speaker's style | `0x0D96D` |
| `+28h` | the text cursor | `0x0D741` |
| `+2Ch` | the dialogue | `0x0D5EE` |
| `+30h` | the composed box | `0x0D512` |
| `+3Eh`, `+40h` | the box's `+0Eh`/`+10h`: where the page it was composed from starts | `0x0D50A` |
| `+42h` | the record being played | `0x0D9D9` |
| `+46h` | the record to play next | `0x0D9CC` |

`+3Eh` is a far pointer into the scratch, its segment at `+40h`. The
compositor `1269h:0082h` takes it as its first argument and writes it into
the box as it begins (`0x12743`, `0x12747`); starting an armed task whose
speaker has moved hands it back to compose the same page (`0x0D50A`), not
the cursor, which has already moved past it. `+26h`, the segment half of
the style's far pointer, is `2139h` at the writes `0x0D967`, `0x0E882` and
`0x0E8CC`, the segment the style rows are in.

## Where a scene comes from

`0CFDh:172Fh` loads scene `n` into task 0: it binds the scene
(`0CFDh:059Eh`, `0x0E703`), empties the queue `[1CF4h]` (`0x0E711`), asks
for kind 4 with the flags, the countdown and the fixed time cleared
(`0x0E728`–`0x0E73A`), and runs the tick once with nothing (`0x0E740`).
`0CFDh:19A4h(lo, hi, flags)` does the same for task 1 with a scene rolled
out of `lo..=hi` by the game's generator (`0x0E988`); bit 0 of the flags
rolls the next one when this one is over (`0x0E381`).

What a scene number names is the binder's to say. A number that is not
negative is one of the location's own scenes, a place in the bank the
room loader keeps at `[2855h]`, while it is below the count `[1EEAh]`
(`0x0D5EA`). A negative one is a place by its complement (`0x0D5D6`):
below `4000h` in bank 999, the generic lines; from `4000h` up in bank 2,
the inventory, less `4001h` (`0x0D5F0`). When nothing is found, place
`13h` of bank 999 stands in (`0x0D5FD`). That is how a generic rule says
*Ich kann nichts Besonderes erkennen.*: its handler hands over `~16`, and
bank 999's place 16 is that line ([dialogue](../formats/dialogue.md)).

Binding refuses a whole scene when the *other* task is speaking with an
overlapping speaker and cannot be interrupted (`0x0D0EA`).

## The tick

`0CFDh:11ECh` runs both tasks, task 1 first. An armed or held-off task —
`+1Ah` bits 15 or 10 — with time left is skipped (`0x0E21E`, `0x0E22B`);
otherwise the countdown goes down (`0x0E23D`) and on zero the kind decides
what follows through a jump table (`0x0E259`). Then a requested kind is
taken (`0x0E420`): it becomes the kind and is cleared (`0x0E44B`), the
point comes from the resolver (`0x0E468`), the cursor starts at the
scratch (`0x0E490`), and the kind picks its start (`0x0E4A9`) — 1 resets,
2 shows pages, 3 cues and shows pages, 4 plays. A finished dialogue is
followed by the next in line (`0x0E31F`): task 0 drains its queue, task 1
draws the next of its range.

What the tick answers about task 0 is what the [mode machine](verbs.md)
waits on (`0x0E4F8`): 0 idle, −1 for a hotspot text, 2 for a menu, 1
otherwise. So a hotspot's own line does not count as busy, and a walk
that waits for the tasks goes on under it.

`0CFDh:1A2Eh` arms a task — it waits for a start, `+1Ah` bit 15.
`0CFDh:1A3Fh` starts an armed one (`0x0EA0F`): the bit cleared, running
set (`0x0EA33`), the page composed again if the speaker moved, and the
hook `[1E32h]` told code 10 with the slot and the task (`0x0EA4A`).
`0CFDh:1A0Eh` stops one: countdown 0, kind 1 asked for. `0CFDh:06DFh`
resets one: no record, kind 0, and for task 0 the band under the
playfield shown again (`0x0D6E0`).

## Playing a record

`0CFDh:09E9h` plays the task's next record. The record to play next
becomes the one being played (`0x0D9CC`, `0x0D9D9`); a non-zero id is
kept in `[1685h]` and told to the hook with code 4 (`0x0DA41`). The low
nibble of its flags picks the arm (`0x0DAC4`): 1 is a line of speaker 0,
shown with cue 4; `n` above 1 a line of speaker `n − 1`, shown with cue 3
for task 0; 0 is the choice list (`0x0DAD6`). The reply list is
remembered in `[1E38h]` (`0x0DE60`) and its first entry is the next
record; with no reply the dialogue is over (`0x0DEB2`).

In the choice arm a spent record (bit 15) is skipped (`0x0DB24`), and a
conditional one (bit 12) whose game flag `[273Fh + n]` holds is spent and
skipped (`0x0DB43`). One survivor needs no menu (`0x0DB6A`); more put the
menu up. The menu is the record at `DS:04F8h` — five lines at most, at
(5, 132), 310 wide, 13 a line — with its own mouse region
(`179Eh:0210h`, `0x0DC91`); every frame the pointer's row inside it is the
highlighted choice (`0x0E061`) and a click takes it (`0x0E0D5`), as do
the keys (`0x0E17C`). It is painted over the panel's picture, one line of
glyphs at a time in ink `F8`, or `E8` for the highlighted choice
(`0x12AFC`).

## A page and how long it stays

`0CFDh:0754h` turns out the task's next page. A zero byte at the cursor
is the end (`0x0D741`); `0Dh` turns the page, holding a base's worth
`[167Dh]` (`0x0D783`). Otherwise the controls in the page are scanned
(`0CFDh:01BEh`, `0x0D79E`), the speaker's point resolved (`0x0D7B1`), the
page composed and kept on the playfield ([text](text.md#the-caption)),
and its characters counted, at most a thousand (`0x0D865`).

The time it stays up is `[167Fh]` per line plus `[1681h]` per character
shifted right by 5 (`0x0D871`, `0x0D87C`), scaled by the adjustment plus
8 over 8 when the adjustment is above −8 (`0x0D887`), and kept between
the floor `[167Dh]` and the ceiling `[1683h]` (`0x0D8AC`, `0x0D8A2`). The
adjustment is the page's own: the scanner counts `03 03` against `03 04`.
The scanner's other controls are `03 01`, hold for a click (`0x0D222`),
`03 02` and `03 05`, two flags of task 0 (`0x0D22E`, `0x0D243`); an
`03 n` above 5 goes to the hook as an event, code 7 (`0x0D24C`).

## A hotspot's own line, and the hover text

`0CFDh:18C2h` plays a hotspot's own line as kind 3 of task 0. The text is
substituted into the scratch (`0x0E8AC`); the speaker is slot 0, the
player (`0x0E8B6`), and the point is the player's x and, above the feet,
forty rows of the figure's scale (`0x0E8DE`–`0x0E8E7`).

`0CFDh:1810h` is the name a hotspot shows while the pointer is over it:
task 0 as kind 2, on slot 11, which is put on the pointer's own point
(`0x0E80E`), for as long as it takes — a fixed time of `7FFFh`
(`0x0E87C`). With a text or a dialogue up it does nothing (`0x0E7E3`).

## See also

- [Dialogue](../formats/dialogue.md) — the banks, the records and their flags
- [Text](text.md) — the control bytes, the substitutions and the caption box
- [Verbs](verbs.md) — the mode machine that waits on task 0
- [Objects](objects.md) — the object list behind slots 1 to 9
