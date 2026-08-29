[← Documentation index](../../README.md)

# Game Structure

*Hilfe für Amajambere — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

How the game's 76 script modules organize into a running adventure: the
setting, the location scheme and its three module series, the verbs, the
characters, and saving. For what each module contains, see the
[module map](module-map.md); for the boot sequence and the location loader
word by word, [boot and frame loop](../../motion16/engine/boot-and-loop.md).

## Setting

A German point-and-click adventure made for the Bundesministerium für
wirtschaftliche Zusammenarbeit und Entwicklung — the federal development-aid
ministry, whose acronym is also the name of the player binary. The player is
**Paul Lerche**, an agricultural expert sent to the fictional region
*Amajambere* in the equally fictional country *Ubuntaka*, where a soil-erosion
project is to be got under way against a famine. The twenty locations are the
project's places: the arrival at **Barabarabengi**, the villages
**Mihanatoke**, **Kijijimbegu** and **Gombeshamba**, the regional capital
**Majikitoyo**. The people are named and recur — **Chief Kalala**, **Josefu**,
**Antony**, the counterpart **John**, the regional planner **Bomba** — and the
player is addressed throughout as *Mudagi*.

The subject matter is the game: the text corpus argues about soil conservation,
about who decides what a village needs, and about an *Irish Crossing* — a
concrete ford instead of a bridge — as the cheaper thing that lasts.

The dialogue is German in CP437. The engine's own prompts are formal *Sie*, as
in Jeff Jet and unlike Die Enviro-Kids greifen ein, whose build was edited to
*Du*.

## Startup

`RUN` (module 100, word id 401 — the pair the container header names) loads the
resident library, reads the walk-direction table out of module 651 and drops it
again, enters graphics with `TOGFX`, installs palette 0, and then, unless
`_FASTSTART` is set, plays the intro from module 610 and erases it. It defines
the characters and items out of modules 611 and 612, installs sixteen text
templates, installs `CTRL` as the frame handler with `400 SCRCTRL`, fades out,
runs `UPLOAD_GAME`, and then writes `20 STARTLOC !` and enters that location.

**Twenty is where this game begins** — the end of its numbering, where Die
Enviro-Kids greifen ein begins at 1 and Jeff Jet at 13. Location 20 is the
game's own front page rather than a room to walk in, and `RUN` treats it as
such: because `STARTLOC` is 20 it also fetches module 650, freezes the screen,
puts the menu up with `2 _INVMODE !`, and only then runs `ANIMPLAY`. So the
game opens on its menu, and `CTRL` — whose location switch is guarded by
`_INVMODE @ 2 <` — will not move anywhere until the menu is clicked away.

The save slots are probed from the menu rather than from `RUN`. `SHOW_FILES`
(module 650, word id 421) runs `706 701 DO I =>EXIST … LOOP` and fills
`_LOADTABLE` with which of the five answered; the sibling games run the same
probe inside `RUN` itself.

## Locations

Twenty, numbered 1 to 20 with no gap, and each owns three modules and up to
four blocks:

| Kind | Id | Lifetime |
|---|---|---|
| Scene | module 100+N | resident while the location is active |
| Macro | module 300+N, word id 549, named `ZEIT` | loaded, run through `LOCINIT EXECUTE`, erased |
| Click | module 500+N, ids from 750 | resident while the location is active |
| Item table | **block 300+N**, 1120 bytes | loaded into `_LDITEM` |
| Walk routes | block 400+N, 452 bytes | loaded into `_ROUTE` |
| Extended routes | block 600+N, 300 bytes | loaded into `_XROUTE` |
| Click areas | block 800+N, 182 bytes | loaded into `_KLICKAREA` |

The item table is the one number that differs from the sibling games, which put
it at block 200+N. `INCLLOC` (module 605) computes it as
`A_LDITEM S_LDITEM * _LDITEM ACTLOC @ 300 + GET`, and 1120 is `35 × 32`, the
two constants module 601 declares.

Moving between them goes through one variable. `CTRL` runs
`NEXTLOC @ -1 != IF NEXTLOC @ INCLLOC -1 NEXTLOC ! THEN` every frame, and the
scripts store their exits there; `INCLLOC` does the rest. That is also the only
way in from outside: motionvm's `--loc N` writes `NEXTLOC` and the game honours
it on the next frame the menu is not up for.

The room is drawn on a screen larger than the display and scrolled — 960×544
seen through a 320×165 window — with the verb and inventory strip on a second
screen of 320×35 beneath it. The two add up to the 320×200 the engine's `TOGFX`
mode gives.

## Hotspots, items, verbs

Eight verbs, as constants in module 603: `TAKE` 1, `EXAMINE` 2, `HANDLE` 3,
`USE` 4, `TALK` 5, `GIVE` 6, `INFO` 7, `LEAVE` 8. What a hotspot admits is a
bitmask of its own — `TAKEABLE` 3, `HANDLEABLE` 6, `USEABLE` 10, `TALKABLE` 18,
`LEAVEONLY` 128 — and the click modules 500+N name one word per hotspot from id
750 up.

The inventory items are named words in module 607 — `MAP`, `SOJA`, `ZETTEL`,
`GLAS`, `KOHLE`, `BLEI`, `STIEFEL`, `REAGENZ` and the rest — and the item
machinery (`CALCINV`, `ADDITEM`, `SUBITEM`) is module 602's.

`INFO` is this game's own verb in spirit: the subject matter is the point, and
module 607 carries a `_DOC_TABLE` of documents the player can call up and read.

## Talking

Conversations run through module 614's `XCALCTALK` and the engine's own
dialogue path, with the per-character state in module 606 (`_DAVESTR`,
`_DAVEINFO`, `_DAVEPIPE` and their siblings) and the speech words in each
location's scene module. Module 609 holds the standing-position words the
characters are placed by.

## Walking

Module 604 is the walk machinery — `WALKQUEUE`, `_DESTX`, `_DESTY` — over the
route tables in blocks 400+N and 600+N, and the direction table module 651
hands over at startup and is erased again.

## Saving

The engine's three files per slot, as in both sibling games: `PUT` writes the
location into `701.blk`, `PUTANIM` the display into `701.anm`, `=>PUTAS` the
resident modules into `701.FRZ`, over the five slots 701 to 705. The pages that
drive them are module 650's, reached from the menu at `_INVMODE` 3 (load) and 4
(save).

## Open questions

- **Location 7 ships no item table.** Block 307 is absent — the occupancy word
  and the offsets agree it was never written — while `INCLLOC` loads block
  300+N unconditionally. The room is reachable in play: location 11 sets
  `7 NEXTLOC !` on one of its branches. The original engine does not recover
  either — `GET` (`BMZ.EXE` `12bb:0e91`) tests the resolved block for null and,
  finding none, calls its error reporter with code `0xE`, *Fehler diverser
  Natur (FDN)*, and returns with `_LDITEM` unfilled, so the room would come up
  with the previous location's items. motionvm refuses the location by name
  instead. Whether the room was cut late or the table lost in mastering is
  unread. ([departures](../../departures.md#the-16-bit-machine))
- **What the 70 animation catalogs in blocks 125–212 hold** has not been
  classified the way the sibling games' have.

## See also

- [Module map](module-map.md) — every module
- [Resource inventory](inventory.md) — the counts
- [Other shipped files](other-files.md) — the launcher, the sound stack, the integrity chain
- [Boot and frame loop](../../motion16/engine/boot-and-loop.md) — `RUN`, `CTRL`, `INCLLOC`
- [Game structure (Jeff Jet - Abenteuer InfoHighway)](../jeffjet/game-structure.md) — the same template, one game later
- [Game structure (Die Enviro-Kids greifen ein)](../enviro/game-structure.md) — and the build this one's kernel is one word short of
