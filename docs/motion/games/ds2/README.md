[← Documentation index](../../README.md)

# Im Netzwerk gefangen – Dunkle Schatten 2

*Dunkle Schatten 2 — this page introduces the game and indexes its documentation. The engine it runs on is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

A point-and-click adventure about a teenager who finds the town's memorial
stone daubed with far-right graffiti and follows the trail — through a
bulletin board where the people who did it talk to each other — back to the
people he thought he had seen the last of. It is the only game in this corpus
on the 32-bit engine, and the newest of the six.

## At a glance

| | |
|---|---|
| Released | 1996 — `ENGINE.EXE` 1996-10-22, the containers 1996-10-13 to 1996-10-25 |
| Commissioned by | Bundesministerium des Innern |
| Produced by | Art Department WA GmbH, Bochum |
| Developed by | DigiTales GmbH, Hamburg |
| Engine | MOTION 32-bit — `ENGINE.EXE` V0.06.06/R109 |
| Display | 640×480 in 256 colors, 25 fps |
| Given away | Free of charge, as a campaign title |

## What it is about

The player is **Karsten Wegner**, and the game says who he is in its own
opening text (text table 6, entry 109): he is *"vielen schon aus DUNKLE
SCHATTEN1 bekannt"*, a little older now, working in a small litho shop and
attending vocational school. The summer when skinheads nearly stopped an old
factory from becoming a youth center — the first game — is behind him, *"aber
dann tauchen sie wieder auf, die dunklen Schatten aus der Vergangenheit."*

What starts it is a defaced memorial stone: *"Der ist ja total beschmiert, der
Gedenkstein"* (entry 110). The refusal that follows sets the game's tone, and
it is Karsten's answer to being told this is just an opinion — *"Du hast
Deine, ich habe meine. Aber Meinen und Rumschmieren sind zweierlei"* (entry
114).

The **network of the title is an in-game bulletin board**, a modem terminal
the player dials into and reads
([the BBS](library/bbs.md)). Alongside it Karsten carries an *info book*, the
game's eight-chapter reference on extremism, whose chapters unlock as the
conversations earn them ([dialogue](library/dialogue.md)). The story runs over
the rooms the 100-series modules cover — locations 1 to 22, with 23 given to
the title sequence — through eight verbs and a story-flag system that decides
what may be said next ([game structure](game-structure.md)).

## Who made it

The game credits itself in text table 6, and the entries are unusually full
for this corpus:

| Role | Named |
|---|---|
| Entwicklung und Gestaltung | DigiTales GmbH, Hamburg |
| Produktion, Projektleitung | Art Department WA GmbH, Bochum — Stephan Reisinger |
| Konzept | Ulf Hausmanns, Platzer Kommunikation GmbH |
| Text | Stefan Hoffmann, Ulf Hausmanns, Stephan Reisinger |
| Game-Design, Logik-Adaption | Stefan Hoffmann |
| Art Direction, Grafik | Thomas Andrae |
| Background-Artwork | Thomas Andrae, Frank Schlief |
| Animation | Animationsstudio Ludewig |
| Sonstige Grafik | Michael Hoffmann, Christian Krämer |
| Programmierung | Frank Fischell, Olaf Finger, Christian Krämer, Stefan Hoffmann |
| Musik | Jochen Heß, Dietmar Heß |
| Redaktion, Supervision | Ulf Hausmanns, Stephan Reisinger |

**This is one of the two games that name the engine.** The programming entry
adds *"Basierend auf: … 'Motion'-Präsentations-System von S. Hoffmann"*, and
the same table names DigiTales GmbH of Hamburg as the developer — which is
where this documentation's attribution of MOTION to DigiTales and Stefan
Hoffmann comes from. The other is [Victor Loomes](../vloomes/README.md),
three years earlier, which gives the engine a version and not an author.

The credit also names a second borrowed component, the *PXF-Library von
Norbert Schmidt und Jochen Heß* — the two who write the music for
[Jeff Jet](../jeffjet/README.md).

## Why it exists

The game states its own purpose (text table 6, entry 105). It belongs to an
*"Aufklärungskampagne der Innenminister von Bund und Ländern gegen
Extremismus und Fremdenfeindlichkeit"*, and *"Der Herausgeber des Spiels ist
das Bundesministerium des Innern."* It was given away rather than sold, and
carried a competition with it: `LIESMICH.DOK` is a printable form, and the
game ends by handing the player the answer to send in — *"Der Lösungssatz, auf
den Du so lange gewartet hast, lautet:"* (entry 107).

## What is documented

| Page | Covers |
|---|---|
| [Game structure](game-structure.md) | Startup, locations, story flags, saving |
| [Module map](module-map.md) | What each of the 86 script modules does |
| [Resource inventory](inventory.md) | What the containers hold, by the numbers |
| [Other shipped files](other-files.md) | Bootstrap, sound drivers, readmes |

Its script library is documented module by module, which no other game here
has: [globals](library/globals.md), [shell](library/shell.md),
[game library](library/game-library.md), [animation](library/animation.md),
[objects and flags](library/objects-and-flags.md),
[dialogue](library/dialogue.md),
[text and speech](library/text-and-speech.md) and [the BBS](library/bbs.md).

## Sources

Everything above is read out of the game's own files except the following,
which come from the published record and are not verifiable here:

- The campaign is generally known as *FAIRSTÄNDNIS*, and the game shipped
  bundled with its 1994 predecessor. ([MobyGames](https://www.mobygames.com/game/6464/dunkle-schatten-2-im-netzwerk-gefangen/), [Adventure-Treff](https://www.adventure-treff.de/spiele-datenbank/12309-dunkle-schatten-2-im-netzwerk-gefangen))

## See also

- [Documentation index](../../README.md) — the engine, both generations
- [MOTION 32-bit](../../README.md#motion-32-bit) — the engine this game runs on
- [Victor Loomes](../vloomes/README.md) — the other game that names the engine
- [Die Enviro-Kids greifen ein](../enviro/README.md) — the same agency's game of the same year
