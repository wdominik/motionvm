[← Documentation index](../../README.md)

# Dunkle Schatten

*Dunkle Schatten — this page introduces the game and indexes its documentation. The engine it runs on is documented under [the engine](../../README.md#the-engine).*

A point-and-click adventure about a boy's school holidays, in which a group
of friends builds a youth center for their town and others try to wreck
it. Every address in the PARSEC documentation is an address in one of its
programs.

## At a glance

| | |
|---|---|
| Released | 1994 — every file `FILES.DOK` lists, 1994-10-24 |
| Commissioned by | Bundesministerium des Innern, Bonn — *Herausgeber* |
| Concept | Platzer Kommunikation GmbH, München — *Konzeption und Beratung* |
| Made by | The Art Department WA GmbH, Bochum — *Realisierung* |
| Engine | PARSEC — `XP00.EXE` and `XP01.EXE`, *A PARSEC Production* |
| Display | 320×200 in 256 colors |
| Asked of the machine | An AT with an 80286, DOS 3.30, VGA or MCGA, about 550 KB of free memory and 1.7 MB on the disk; an Ad Lib card for the music |

## What it is about

The title program's backstory says it in its own words (`XP00.EXE`): the
player is **Karsten Wegener**, through his school holidays. He helps, of his
own free will, to build a youth center for himself and his many friends in
the town — *"Aber Du und deine Freunde seid nicht allein in der Stadt. Und
es gibt welche, denen paßt nicht, was ihr machen wollt."* Whether the
Jugendzentrum opens on time is the question the game asks:
*"Fünf Tage hast Du Zeit, um Deinen Freunden so gut wie möglich zu helfen.
Probleme gibt es genug - wie im echten Leben!"*

The days are the game's frame. Karsten wakes in his room, the Zimmer, and
the town map, the Karte, marks the day; the readme's tip is the rule the
game runs on — *"Wenn Karsten sich schlafen legt, hast Du nämlich alle
Aufgaben für einen Tag geschafft!"* Between the places of the town lies a
forest with a maze behind it, and the readme's other tip is about getting
out of it again: *"Meistens kommst Du aus dem Wald, wenn Du drei- oder
viermal durch das Gestrüpp gegangen bist."* How the program puts those
pieces together is on [game structure](game-structure.md).

Two years later the same ministry and the same Bochum producer brought out
a sequel on another engine, *Im Netzwerk gefangen – Dunkle Schatten 2*,
whose opening text introduces its hero as *"vielen schon aus DUNKLE
SCHATTEN1 bekannt"* ([its page](../../../motion/games/ds2/README.md)).

## Who made it

The game credits itself in a table both programs carry, the same in
`XP00.EXE` and in `XP01.EXE`:

| Role | Named |
|---|---|
| Herausgeber | Bundesministerium des Innern, Graurheindorfer Straße 198, 53117 Bonn |
| Konzeption und Beratung | Platzer Kommunikation GmbH, München |
| Realisierung | The Art Department WA GmbH, Bochum |
| Redaktion | Ulf Hausmanns, Thomas Scholl |
| Gesamtleitung | Markus Scheer |
| Programmierung | Norbert Schmidt |
| Tool-Programmierung | Ekkehard Heß |
| Grafik | Michael Hoffmann |
| Animation | Thomas Andre |
| Musik | Jochen Heß |
| Verpackungsdesign | Michael Bohne |
| Grafikbearbeitung | Hakan Akbiyik, Christian Krämer |

The table closes on the music system and the campaign: *PSM 2.00
Soundsystem · Copyright (C) 1988-1994 · Norbert Schmidt & Jochen Heß*, and
*"Dunkle Schatten" · Im Rahmen der FAIRSTÄNDNIS-Kampagne*. The readme gives
a hotline for the game under a Bochum number.

## Why it exists

It was made for a campaign: the credits place the game *im Rahmen der
FAIRSTÄNDNIS-Kampagne*, with the Federal Ministry of the Interior as its
publisher. Its tips say the rest of what it is for — *"Sprich immer mit
allen Personen, die Du triffst"*, and *"Immer lässig bleiben, wenn Du blöde
angemacht wirst! Gewalt ist ist niemals eine Lösung."*

## What is documented

| Page | Covers |
|---|---|
| [Game structure](game-structure.md) | The sixty-one rows, the opening, the map and the days, the forest and the maze, saving |
| [Other files](other-files.md) | The batch files, the `.DOK` viewer, the end sequence's file and the rest |

The programs and their files are documented in the family's tree:
[the engine pages](../../README.md#the-engine) and
[the format pages](../../README.md#formats).

## Sources

- `XP00.EXE` and `XP01.EXE`, unpacked: the backstory, the credits table,
  the location table and every handler.
- `INFO.DOK`, the short guide the install ships: the machine it asks for,
  the controls, the tips.
- `FILES.DOK`: the files an installation has to hold.

## See also

- [Game structure](game-structure.md)
- [Verification](../../verification.md) — what of this game has been held against the original
- [Departures](../../departures.md) — where motionvm plays it otherwise, and why
