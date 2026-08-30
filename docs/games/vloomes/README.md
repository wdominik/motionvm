[← Documentation index](../../README.md)

# Victor Loomes – Das Spiel

*Victor Loomes – Das Spiel — this page introduces the game and indexes its documentation. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

A point-and-click detective adventure that travels through three years. The
player is a private investigator who has to get a woman out of a gangster's
hands, and the case runs from prohibition-era Chicago to a Frankfurt seven
years in the game's own future. It is **the oldest MOTION artifact there is**
— three years older than any other build in this corpus, and the only game
here in the container's earlier framing.

## At a glance

| | |
|---|---|
| Released | 1993 — `LL.EXE` 1993-05-20, `DATA.-1-` and `GFX.INF` 1993-05-21 |
| Commissioned by | LBS — the Landesbausparkasse, a building society |
| Made by | Promotion Software GmbH, Ferdinand-Lassalle-Straße 57, 7410 (72770) Reutlingen |
| Engine | MOTION 16-bit — `LL.EXE`, the oldest of the four builds, credited as *Motion 1.0* |
| Display | 320×200 in 256 colors |
| Given away | Free of charge, with a nationwide competition |

## What it is about

The player is the detective **Victor Loomes**, and the plot moves between
three times the game's own strings name: *Chicago 1927*, *Frankfurt 1993* and
*Frankfurt 2000*. A machine is what one changes *"zwischen den drei
Zeitzonen"* with, and it is location 13. **Rachel** is who the case is about
and **Wild Bill** is who has her; fourteen speakers carry the dialogue, and
seven verbs — one fewer than every later game, which has no `LEAVE` — carry
the play over thirteen locations, two of which are transit lines with twenty
stops each ([game structure](game-structure.md)).

The client is inside the fiction as well as around it. The object table names
two counter clerks, `LBS93LADY` and `LBS00LADY`, and locations 9 and 11 are
the two rooms that place them and give them their own speech; a screen in the
game shows the client's slogan outright — *"Wir geben Ihrer Zukunft ein
Zuhause - LBS"* — and one of the items the player can hold is an
*LBS-Bausparurkunde*, which *"Rachel wird begeistert sein!"*

## Who made it

The game credits itself in text table 8, 25 entries:

| Role | Named |
|---|---|
| Spielkonzeption und Entwicklung | Promotion Software GmbH |
| Idee und Projektleitung | Ralph Stock |
| Programmierung PC | Stefan Hoffmann |
| Programmierung Amiga | Christian Krämer |
| Graphik | Thomas Andrae, Michael Hoffmann |
| Musik PC | Palladix |
| Musik Amiga | Pleiß & Weichselbaum |
| Support, Testing | Markus Ohnesorg |
| Erstellt unter | Motion 1.0 · Michel "Babe" Stigler · EGO Software |
| Very special thanx to | U4IA |

Two things in that list have no counterpart in any later game. **An Amiga
version existed** — *Programmierung Amiga* and *Musik Amiga* are credited
separately, to different people than the PC entries. And **the tool is
credited by version**: `Motion 1.0`, three years before the builds the rest of
this documentation is measured on. What the two names after the version belong
to is not settled ([open questions](../../open-questions.md#motion-16-bit));
a *Michael* Stigler is credited in [Jeff Jet](../jeffjet/README.md), the same
studio's next game, for dialogue and support.

Three of these names turn up again in
[Die Enviro-Kids greifen ein](../enviro/README.md) — an Art Department
production, not this studio's — where they are hidden as the door signs of a
town administration.

## Why it exists

An advergame for a building society, given away with a nationwide competition
the game itself runs: text table 6, the one the engine's message box draws
from, offers *"einen von 15 nagelneuen 486er DX33 PC mit 210 MB Festplatte und
VGA Grafikkarte"* for answering a riddle on the game's own initials — *VL*,
which is also *vermögenswirksame Leistungen*, the savings product the client
sells. The closing date is 1993-11-30, and the same table carries an
LBS hotline with opening hours.

The advertisement is the frame rather than the dialogue: the competition and
the two counter clerks carry it, and the detective story runs on its own.

## What is documented

| Page | Covers |
|---|---|
| [Game structure](game-structure.md) | Setting, the thirteen locations, the two module series, verbs, saving |
| [Module map](module-map.md) | What each of the 36 script modules does |
| [Resource inventory](inventory.md) | What the one `DATA.-1-` holds, by the numbers |
| [Other shipped files](other-files.md) | The four-program launcher chain, `GFX.INF`, the older sound setup |

## Sources

Everything above is read out of the game's own files except the following,
which come from the published record and are not verifiable here:

- The studio's own account: Ralph Stock founded Promotion Software in 1993,
  and it describes this game as an advergame for the LBS with the player as a
  private investigator in 1920s Chicago — where the game's own strings say
  1927. ([Sixteen Tons Entertainment](https://www.sixteen-tons.de/en/about-ralph-stock/))
- An Amiga release, which the credits imply and the published record
  confirms. ([uvlist](https://www.uvlist.net/game-102546-Victor+Loomes), [Werbespiel Archiv](https://werbespiel.blogspot.com/2010/09/victor-loomes-das-spiel.html))

## See also

- [Documentation index](../../README.md) — the engine, both generations
- [MOTION 16-bit](../../README.md#motion-16-bit) — the engine this game runs on
- [LL.EXE](../../motion16/engine/ll-exe.md) — the build it ships, and what differs in it
- [Jeff Jet](../jeffjet/README.md) — the same studio, two years later
- [Dunkle Schatten 2](../ds2/README.md) — the other game that names the engine
