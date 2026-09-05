[← Documentation index](../../README.md)

# Falsches Spiel mit Eddie M.

*Falsches Spiel mit Eddie M. — this page introduces the game and indexes its documentation. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

A point-and-click comedy about an advertising man who is framed, made to
advertise the weekly magazine *Stern*. The player is Eddie Mockelby, a media
planner at the agency Fast & Faster, fired the morning a rival agency shows the
client the very pitch he wrote; the game is clearing his name, and the magazine
is in it as a place — its editorial office, its documentation department, its
publisher's house — and as a running gag about the day it comes out.

## At a glance

| | |
|---|---|
| Released | 1994 — `STERN.EXE` 1994-10-06, the three volumes 1994-10-10 and 1994-10-13, the boot-disk tooling and the readme November 1994 |
| Commissioned by | Gruner + Jahr, for *Stern*: the publisher and the magazine are hotspots of the game, the editorial office a character's, and Thursday *"STERN-Tag"* |
| Made by | via productions ag, by the published record; the files name no studio |
| Engine | MOTION 16-bit — `STERN.EXE`, the second of the five builds; the magazine names the binary |
| Display | 320×200 in 256 colors |
| Given away | On two floppies — the readme's *"rote Diskette 1/2"* — as a promotion with a prize: the game keeps a score and shows a *Gewinncode* to send in |

## What it is about

The pitch Fast & Faster made turned up, slide for slide, at the rival agency
MicroBrain the day before — *"MicroBrain hatte gestern bei Makel & Loos exakt
dieselbe Präsentation wie wir"* — and Eddie is the one blamed: dismissed by his
boss, cut by his colleagues (*"Ich bin kein Verräter." — "Wer's glaubt, wird
selig."*), and left by his girlfriend Tamara Collaris. He sets out to find who
leaked it. The trail runs through the agency and its people — Holger
Flöttenkamp of MicroBrain, Stanislaus Klöbenrick of Makel & Loos, a
photograph in a dustbin, a woman called Zemecki, a man called Eggebrecht — and
it runs through the magazine: Irene Lindberg of the *Stern* editorial office
has an anonymous letter *"randvoll mit bösen Worten über MicroBrain"*, and the
*Stern-Dokumentation*, the archive, is a room of the game with a terminal
whose entries are the *Hitler-Tagebücher* and their fellows — the magazine's
own 1983 affair, played for a laugh.

Fifteen locations carry it, and the game opens in the third, Eddie's flat,
with the morning of the dismissal playing as a scripted scene before the
player gets the mouse ([game structure](game-structure.md)). A city map
connects the rest: the agency and its offices, MicroBrain, Makel & Loos, the
publisher's house on the map as *Gruner + Jahr*, a television station whose
studio hosts a talk show, the harbour, a headhunter's villa, a scene bar. The
cast beyond the case is broad comedy — a dwarf called Kettensäge, a talk-show
guest who converses with dolphins, a former beer-advertisement face called
Starkwort — and the prompts address the player as *Sie* while the dialogue
uses *Du*.

Two small games sit inside it: a slide puzzle the scripts call the
*STERN-Puzzle*, and a photograph to reassemble. And one puzzle reads the
clock: asked whether today is *STERN-Tag*, the player types the current issue
number, and the game works out from the machine's date what the number is
([game structure](game-structure.md#the-date-and-the-issue-number)).

## Who made it

The game ships no credits table and no license file, and none of its 2443
strings names a studio. The attribution is the published record's:
Adventure-Treff lists via productions ag as the developer and G+J
Informationssysteme GmbH as the publisher, and the archive item the copy comes
from names Gruner + Jahr AG & Co KG. The engine's author is not the game's:
MOTION was licensed to several studios — Promotion Software, PASS VISION, The
Art Department — and nothing in these files puts DigiTales beyond the kernel.

## Why it exists

A magazine promotion, on floppy, with a competition. The scripts keep a score
(`_POINTS`, raised by `SET_POINTS` with a jingle) and the menu's first page
shows a *Gewinncode* computed from it, digit by digit out of sprites — a code
to send in, which is what the *Stern*'s reader got for finishing the game. The
Thursday gag is the same promotion from the inside: *"DONNERSTAG IST
STERN-TAG"* is the magazine's publication day, and the puzzle that asks for the
issue number is a puzzle about buying the magazine.

## What has been checked against the original

The intro's tune, block 24, held against an OPL register recording of the
original under DOSBox-X: 3 551 writes over 54.8 seconds, identical
([verification](../../verification.md)). Nothing of a played room has been
compared yet, and no sound effect has been heard against the original; the
readers and the engine are held to the game's files as for every game
([verification](../../verification.md#held-against-the-originals-files)).

## What is documented

| Page | Covers |
|---|---|
| [Game structure](game-structure.md) | Setting, the fifteen locations, the three module series, the opening scene, verbs, the minigames, the menu, sound, saving |
| [Module map](module-map.md) | What each of the 62 script modules does |
| [Resource inventory](inventory.md) | What the three `DATA.-n-` volumes hold, by the numbers |
| [Other shipped files](other-files.md) | The sound stack, the boot-disk tooling, the readme |

## Sources

Everything above is read out of the game's own files except the following,
which comes from the published record and is not verifiable here:

- The developer and the publisher: *via productions ag* and *G+J
  Informationssysteme GmbH*.
  ([Adventure-Treff](https://www.adventure-treff.de/spiele-datenbank/1937-falsches-spiel-mit-eddie-m6))
- The publisher as the archive names it, *Gruner + Jahr AG & Co KG*, and the
  catalogs' spelling *Eddy M.*, which the shipped files do not use: the
  readme, the boot-disk tool's banner and the dialogue all write *Eddie*.
  ([Internet Archive](https://archive.org/details/eddym))

## See also

- [Documentation index](../../README.md) — the engine, both generations
- [MOTION 16-bit](../../README.md#motion-16-bit) — the engine this game runs on
- [STERN.EXE](../../motion16/engine/stern-exe.md) — the build it ships, and the two words only this game calls
- [Victor Loomes – Das Spiel](../vloomes/README.md) — the game before it, on the build before it
