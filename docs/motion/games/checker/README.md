[← Documentation index](../../README.md)

# Checker 2000

*Checker 2000 — this page introduces the game and indexes its documentation. The engine it runs on is documented under [MOTION 32-bit](../../README.md#motion-32-bit); the build it ships, `ENGINE.EXE` V0.04.15/R78, has [a page of its own](../../motion32/engine/engine-r78.md).*

A campaign title for school leavers, built for the AOK — Germany's regional
public health insurers — and given away to young people about to start an
apprenticeship. It is a **menu of five things** rather than one story: a
story game with twenty mini-games and a highscore, a photo story, thirty
information pages on applying for a job and on what the insurer does,
music, and the way out. It is the second game in this corpus on the 32-bit
engine, and the earlier of the two by half a year of engine development.

## At a glance

| | |
|---|---|
| Released | 1996 — the texts date their sample letters *06.08.96* and *13. April 1996*; `003.RSC` is stamped 1996-03-10, and every other file in the copy on hand was re-stamped 2010-10-30 |
| Commissioned by | The AOK — the texts speak for the regional insurers of the new federal states, naming AOK Brandenburg's branch offices and AOK Chemnitz's press service |
| Developed by | Promotion Software GmbH, Tübingen — the studio behind [Jeff Jet](../jeffjet/README.md) and [Victor Loomes](../vloomes/README.md), attributed from outside the files, which name no studio; the engine is DigiTales' MOTION, as [Dunkle Schatten 2](../ds2/README.md) names it |
| Engine | MOTION 32-bit — `ENGINE.EXE` V0.04.15/R78 |
| Display | 640×480 in 256 colors, 8 frames a second (`8 DELAY`) |
| Sound | Five HMI tunes on the OPL3; the story **spoken**, 73 WAV files under `WAVS/`, with 39 effects as blocks beside them, and subtitled only where no sound card answers ([speech](game-structure.md#speech)) |
| Given away | Free of charge, as a campaign title |

## What it is

The player first types a name and a postcode into the registration board,
and the **first three digits of the postcode** pick one of eight regions —
the information pages come in eight regional editions, and the board keeps
the choice in `_REGIONAL` ([game structure](game-structure.md#the-registration)).
Then the main menu, *CHECKER 2000*, offers:

| Button | The board says | What it is |
|---|---|---|
| **Play it!** | *Zum Futurespiel* | The story game: a second menu with *Play it!*, *Play it?* (the instructions), *Load it!* (a saved game) and *Stop it!* |
| **Watch it!** | *Zum Fotoroman* | A photo story *zum Schlaumachen für Berufsstarter und Azubis*, thirty topics deep, with *Print it!* for the printer |
| **Check it!** | *Zu den Infos* | The information pages: applying, the aptitude test, the CV, and what the insurer does for people starting work |
| **Dance it!** | *Zu den Superhits* | The music |
| **Stop it!** | *Nichts wie weg* | Quit |

The story game is where the engine is used as an adventure engine. Its
cast is a class of ninth-graders — **Jenny, Tom, Malte, Monika, Georg** —
and the people around them: **Vicky**, a teacher, a *Meister*, a reporter,
*Brink*, *Hilde*, a colleague. It opens on the English test (text table 2,
entries 0 to 11): *"Georg - Drei..... Monika - Sechs.... Malte - Vier
minus...... Tom - Fünf plus"*, and Tom's *"..miese Noten in der 9. Klasse
gleich miese Aussichten auf 'ne Lehrstelle. No future!"* is what the rest of
the game answers. The answer is the insurer's — *"Keine Panik! Wir geh'n zur
AOK. Die hilft weiter."* (entries 66, 70) — delivered through seventeen
story locations and twenty mini-games, `GAMEA` to `GAMET`, that keep a score
and a clock and feed a **highscore of five** with name, date and score
([game structure](game-structure.md)). Two of the games say what they are in
their own texts: *"Setze Vickys Motorrad zusammen!"*, a sliding-tile puzzle,
and *"Bring das Dreamteam zum Sieg. Du hast fünf Würfe"*, a throwing game
(text tables 2 and 3).

## What is documented

| Page | Covers |
|---|---|
| [Game structure](game-structure.md) | Startup, the shell and its boards, the registration, the task sequence, saving, speech |
| [Resource inventory](inventory.md) | What the five containers hold, by the numbers |
| [Module map](module-map.md) | What each of the 119 script modules does |
| [Other shipped files](other-files.md) | The two bootstraps, `ENGINE.RSC`, the sample path, sound setup, the readme |
| [ENGINE.EXE V0.04.15/R78](../../motion32/engine/engine-r78.md) | The build this game ships, and where it differs from Dunkle Schatten 2's |

Nothing here is a script library reference of the kind Dunkle Schatten 2
has: this game's shell was read as far as running it takes — the boards,
the registration, the task machine — and the rest of its 119 modules are
mapped by what they export, not by what every word does.

## Sources

Everything above is read out of the game's own files, with one exception:
the files name no studio, no author and no year, and the developer —
Promotion Software GmbH of Tübingen, which built the two other campaign
games on this engine — is attributed from outside them. The year is inferred
from the dates inside the texts and the one file the copy on hand did not
re-stamp.

## See also

- [Documentation index](../../README.md) — the engine, both generations
- [MOTION 32-bit](../../README.md#motion-32-bit) — the engine this game runs on
- [Dunkle Schatten 2](../ds2/README.md) — the other 32-bit game, on the later build
