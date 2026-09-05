[← Documentation index](../../README.md)

# Die Enviro-Kids greifen ein

*Die Enviro-Kids greifen ein — this page introduces the game and indexes its documentation. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

A point-and-click adventure in which four children run a newspaper supplement
and use it to find out where their town's rubbish actually goes. It is the
newest of the five games on the 16-bit engine, and the build it ships,
`ENVIRO.EXE`, is the one the whole 16-bit half of this documentation is
measured on.

## At a glance

| | |
|---|---|
| Released | 1996 — `ENVIRO.EXE` 1996-08-27, `DATA.-1-` 1996-08-26 |
| Commissioned by | Ministerium für Umwelt, Raumordnung und Landwirtschaft des Landes Nordrhein-Westfalen |
| Made by | Art Department Werbeagentur GmbH |
| Engine | MOTION 16-bit — `ENVIRO.EXE`, the latest of the five builds |
| Display | 320×200 in 256 colors |
| Given away | Free of charge |

## What it is about

The town is **Waldbach**, *"ein hübsches kleines Städtchen"* that lives on
tourism (the intro's text table 14), and it has a **VISALUX** factory and a
dump. The town is preparing its 500-year festival while its rubbish problem
gets worse.

The player is **Eva or Maik** and can change between them at will — *"Jetzt
bin ich Eva."* The two run the *Müll-Gazette*, a supplement to the local
paper, and the game's own articles (`_ART1` to `_ART3`, module 615) are what
the player writes with what they find out. A day clock runs alongside them.
One of the endings the game prints has the paper doing its work: the illegal
dumping comes out *"als unser Mitarbeiter Maik Waffenschmidt die fertig
verpackten Container entdeckte, umgehend das Beweismaterial fotografierte und
die Tat anschließend sofort zur Anzeige brachte."*

Sixteen rooms carry it — locations 1 to 15 and 17, with no 16 — over a cast of
sixteen named characters including Dave, Sarah, Sascha, Opi, Waffi, Fischer
and the Postbote, reached through the family's eight verbs
([game structure](game-structure.md)).

## Who made it

**The shipped files name no client and carry no credits table.** What they do
carry is a joke at the same address: the town administration's door signs, in
text table 21, name the team as municipal officials.

| Door sign | Reading |
|---|---|
| *Frank Ziemlinski — Stadtreferent für Projektleitung* | Project lead |
| *Stefan Hoffman — Stadtamt für Spieldesign* | Game design (spelled with one `n` here) |
| *Thomas Andrae — Städtischer Grafikleiter* | Graphics lead |
| *Christian Krämer — Ortsamt für Programmierung, zusätzliche Grafik und Animation* | Programming, extra graphics, animation |

Three of those four names appear in [Victor Loomes'](../vloomes/README.md)
credits table three years earlier, where they belong to Promotion Software —
so the two studios of this corpus share people. The fourth, Frank Ziemlinski,
appears only here. The one-`n` *Hoffman* is the spelling on the sign; both
[Dunkle Schatten 2](../ds2/README.md) and Victor Loomes spell the same name
with two.

The sound setup signs itself *"A PARSEC Production"*, which is the house the
[PSM 2 music format](../../motion16/formats/psm-music.md) comes from.

## Why it exists

An advergame for a state environment ministry, given away rather than sold,
with the subject matter carried by the newspaper rather than by the puzzles:
the game's own information pages explain how to produce less waste while
shopping, and its closing text has the festival succeed because *"viele Gäste
begriffen auch plötzlich, welche Möglichkeiten sie selber haben, sich
umweltbewußt zu verhalten."*

## What is documented

| Page | Covers |
|---|---|
| [Game structure](game-structure.md) | Setting, the location scheme and its three module series, verbs, saving |
| [Module map](module-map.md) | What each of the 65 script modules does |
| [Resource inventory](inventory.md) | What `DATA.-1-` holds, by the numbers |
| [Other shipped files](other-files.md) | Sound setup and drivers, the launcher, readme, leftovers |

## Sources

Everything above is read out of the game's own files except the following,
which comes from the published record and is not verifiable here:

- The commissioning ministry. The files name no client; the attribution is the
  published one. ([Adventure-Treff](https://www.adventure-treff.de/spiele-datenbank/14046-die-enviro-kids-greifen-ein-umweltministerium-nrw), [Adventure Corner](https://www.adventurecorner.de/game/2559/die-enviro-kids-greifen-ein))

## See also

- [Documentation index](../../README.md) — the engine, both generations
- [MOTION 16-bit](../../README.md#motion-16-bit) — the engine this game runs on
- [ENVIRO.EXE](../../motion16/engine/enviro-exe.md) — the build it ships
- [Victor Loomes](../vloomes/README.md) — where three of its four names come from
- [Hilfe für Amajambere](../hfa/README.md) — the same agency, a year earlier
