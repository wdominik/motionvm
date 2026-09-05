[← Documentation index](../../README.md)

# Game Structure

*Falsches Spiel mit Eddie M. — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

How the game's 62 script modules organize into a running adventure: the
setting, the location scheme and its three module series, the opening scene,
the verbs, the two minigames, the menu, the sound, and saving. For what each
module contains, see the [module map](module-map.md); for the boot sequence and
the location loader word by word, [boot and frame loop](../../motion16/engine/game-loop.md).

## Setting

A German point-and-click comedy made for Gruner + Jahr to advertise the
magazine *Stern*. The player is **Eddie Mockelby**, a media planner at the
advertising agency **Fast & Faster**, dismissed on the morning the game opens
because the pitch he wrote turned up at the rival agency **MicroBrain** the
day before. Fifteen locations carry the investigation: his flat, the agency
and its offices, the rival agencies MicroBrain and **Makel & Loos**, the
publisher's house, the *Stern-Dokumentation*, a television station and its
studio, the harbour, a headhunter's villa, a bar. The named cast is broad and
comic — Irene Lindberg of the *Stern* editorial office, Holger Flöttenkamp,
Stanislaus Klöbenrick, Eleonore Schmitt, Tamara Collaris, a dwarf called
Kettensäge — and the dialogue is German in CP437. The engine's own prompts are
formal *Sie*, as in Jeff Jet and Hilfe für Amajambere.

What the game is about, who made it and why it exists are on
[the game's own page](README.md).

## Startup

`RUN` (module 100, word id 411 — the pair the container header names; the
three later games' `RUN` is id 401) loads the resident library — modules 600
to 607 and 609 — pushes the constants 1 to 10, enters graphics with `TOGFX`,
installs palette 0 and switches the off-screen buffers on with `BUFON`. Then,
unless `_FASTSTART` is set, it plays the intro from module 610 and erases it;
installs palette 1 and the pointer; defines the permanent characters and the
items out of modules 611 and 612 and drops them; installs seven text templates;
installs `CTRL` as the frame handler with `410 SCRCTRL`; fades out; sets
`1 INTROON !`; and enters `STARTLOC @ INCLLOC` — **location 3, the flat**,
the value module 601 declares, which `RUN` never stores itself — before it runs
`ANIMPLAY`.

**The ten constants are a check on the engine.** `RUN` pushes them before
`TOGFX` and never pops them, and `CTRL` opens every frame on `DUP 10 !=`: when
the top of the stack is not the tenth constant, it prints the top two cells
with `.` and `EMIT` and waits for a key. It is a leftover assertion from the
game's development, and it holds the engine to every kernel word's stack
effect — which is how a word that left a cell behind in every 16-bit game was
found ([departures](../../departures.md#the-16-bit-machine)). `CTRL` also
counts frames in `_TENMIN` and, every 9000 of them, in `_GESTENMIN`: a
play-time counter nothing else reads.

**There is no save-slot probe in `RUN`.** Die Enviro-Kids greifen ein and Jeff
Jet probe the five slots at start-up and offer a start-up page; this game, like
Hilfe für Amajambere, probes them from the menu — in `SHOW_FILES`, which here
is a word of the boot module itself.

## The opening scene

`INTROON` is a variable of module 607, 0 when declared and set to 1 by `RUN`.
While it is 1, `CTRL` takes its second branch: the location's own controller
word (`_LOCCTRL`) and the global one (`_MAINCTRL`) run, `NEXTLOC` is polled,
and the verb machinery is held back — the scene of the dismissal plays out in
the flat and moves Eddie on to the agency, location 4, by script. The scripts
of location 1 set `INTROON` to 2 when the scene is over, and `CTRL`'s first
branch — the player's — is guarded by `INTROON @ NOT`. The intro module 610
keeps its own state in `_INPIC`, `_FADE` and `_PHASE`, and `STARTEXTRO`, the
ending, is the same module's second word, run from location 9.

## Locations

Fifteen, numbered 1 to 15 with no gap, and each owns three modules and four
blocks:

| Kind | Id | Lifetime |
|---|---|---|
| Scene | module 100+N | resident while the location is active |
| Macro | module 300+N, word id 549, named `ZEIT` | loaded, run through `LOCINIT EXECUTE`, erased |
| Click | module 500+N, ids from 750 | resident while the location is active |
| Item table | block 200+N, 1120 bytes | loaded into `_LDITEM` |
| Walk routes | block 400+N, 452 bytes | loaded into `_ROUTE` |
| Extended routes | block 600+N, 300 bytes | loaded into `_XROUTE` |
| Click areas | block 800+N, 182 bytes | loaded into `_KLICKAREA` |

The numbers are the sibling games' — the item table at 200+N as in Die
Enviro-Kids greifen ein and Jeff Jet, where Hilfe für Amajambere puts it at
300+N — and `INCLLOC` (module 605) computes them as `ACTLOC @ 200 +` and so on,
with 1120 as `A_LDITEM S_LDITEM *`, 35 × 32, the constants module 601 declares.
All fifteen tables of every family are shipped.

Which room is which is read off the click modules' hotspot words and the
labels in text table 11:

| N | The room, as its hotspots name it |
|---:|---|
| 1 | The street outside the agency — *Parkbank*, *Kiosk*, *Haltestelle*, *Agentureingang*, the receptionist |
| 2 | An office — telephone, coffee pot, overhead projector, Tobias |
| 3 | **Eddie's flat**, where the game opens — balcony, bed, answering machine, television, kitchen, the *STERN-Puzzle* |
| 4 | The agency's offices — Fasti, Bianca, Kruse, Martha, the photocopier, the boss's office |
| 5 | **The city map** — *Eddies Wohnung*, *Fast & Faster*, *Gewächshaus*, *MicroBrain*, *Fernsehsender*, *Hafenstraße*, *Headhunter-Villa*, *Szenekneipe*, *Gruner + Jahr*, *Makel u. Loos* |
| 6 | A park bench with two exits |
| 7 | MicroBrain — Tamara, Flöttenkamp, fax, files, shredder, safe |
| 8 | A house entrance with bushes |
| 9 | The street at *Gruner + Jahr* — the sign, a van; the ending plays here |
| 10 | Makel & Loos — Klöbenrick, the dustbin, a ship model, a video recorder |
| 11 | The harbour — a boat, a car wreck, a shed |
| 12 | An office with a desk, a bookend and two chairs |
| 13 | **The *Stern-Dokumentation*** — Irene, Simone, a terminal, and the *Hitler-Tagebücher* with their fellows |
| 14 | The bar — a counter, a stage, and most of the cast |
| 15 | The television studio — the dwarf, Tommy, Jörg, monitors, camera, a vending machine, a kitchen |

Moving between them goes through one variable. `CTRL` runs
`NEXTLOC @ -1 != IF NEXTLOC @ INCLLOC -1 NEXTLOC ! THEN` every frame — in both
branches of its `INTROON` split — and the scripts store their exits there;
`INCLLOC` does the rest. That is also the only way in from outside: motionvm's
`--loc N` writes `NEXTLOC` and the game honors it on the next frame.

The room is drawn on a screen larger than the display and scrolled — 960×280
seen through a 320×165 window — with the verb and inventory strip on a second
screen of 320×35 beneath it. The two add up to the 320×200 the engine's `TOGFX`
mode gives, the geometry of the three later games.

## Hotspots, items, verbs

Eight verbs, as constants in module 603: `TAKE` 1, `EXAMINE` 2, `HANDLE` 3,
`USE` 4, `TALK` 5, `GIVE` 6, `INFO` 7, `LEAVE` 8. What a hotspot admits is a
bitmask of its own — `TAKEABLE` 3, `HANDLEABLE` 6, `USEABLE` 10, `TALKABLE` 18,
`LEAVEONLY` 128 — and the click modules 500+N name one word per hotspot from id
750 up, `LD_`*noun*. The inventory items are named words in module 607 —
`PRÄSENTATIO`, `AUDIOTAPE`, `FAX`, `KLEBER`, `PAPPE`, `DIKTIER`, `STERN`,
`GLAS`, `PUZZLE`, `SCHNIPSEL`, `FOTO`, `VISITEN` — and the item machinery
(`CALCINV`, `ADDITEM`, `SUBITEM`) is module 602's. The readme tells the
player how the verbs are reached: *"während der Name oder die Bezeichnung
erscheint, die rechte Maustaste drücken."*

## Talking and walking

Conversations run through module 614's `XCALCTALK` and the engine's own
dialogue path, with the per-character state in module 606. The player figure
keeps the name the authoring template gave it: the words are `_DAVE`,
`_DAVESTR`, `_DAVEINFO`, `_DAVEPIPE`, `DAVE_SMALL` — Dave being the hero of
Die Enviro-Kids greifen ein, whose template this game shares two years earlier
— and they move Eddie. Module 604 is the walk machinery — `WALKQUEUE`,
`_DESTX`, `_DESTY`, `SETWALK`, `SETSTEPS` — over the route tables in blocks
400+N and 600+N.

## The minigames, the map and the score

Three of the library modules are the game's own devices, switched by
`_PUZZON` in `CTRL`:

- **The `STERN-Puzzle`** (module 613, `_PUZZON` 2): a 4×4 slide puzzle over
  `_PFIELD`, sixteen cells the player clicks tiles into; `MIX_PUZZLE` shuffles
  it and `CALC_PUZZLE` checks it.
- **The photograph** (module 608, `_PUZZON` 3 and 4): pieces dragged into
  place, counted in `FOTOFOUND`.
- **The map** (module 615, reached from location 1's scene): the city map of
  location 5 with a `_MAPSTATE` that grows as places are learned of.
- **The score** (module 609): `SET_POINTS` adds to `_POINTS` and plays the
  jingle, block 19; `CALC_PPANEL` draws the panel; and the menu's
  `GEWINNCODE` (module 100) renders a code from the score, digit by digit out
  of sprites 2310 and up — the competition's prize code.

### The date and the issue number

Module 609 also holds `STNR`, and it is the one place any MOTION game reads
the clock. Location 1's scene asks whether today is *STERN-Tag* and lets the
player type a two-digit number; on Enter it runs `GIVEDATE STNR _STNR @
SAY_ELVIRA @ =`: the kernel word pushes the day, the month and the year, `STNR`
stores them, looks up the year's first Thursday in `_SKAL`, adds the months out
of `_MTABLE`, and divides by seven into an issue number, capped by `_JTABLE`.
The tables cover 1993 to 2000 — eight entries, ninety-six months — and a date
past them reads beyond the variables ([open questions](../../open-questions.md#motion-16-bit)).
motionvm answers `GIVEDATE` with the UTC date; a suite fixes it
([departures](../../departures.md#the-16-bit-machine)).

### Three keyboard codes

`CTRL` keeps the last five keys in `_KQ` and compares them against three
words: `wiona` sets `_MAPSTATE` to 8 and moves to the map, `brian` lays the
puzzle's sixteen fields out solved, and `robin` toggles a variable `ROBIN`.
They are the developers' shortcuts, shipped.

## The menu

The strip below the world is where the menu lives, and `_INVMODE` says which
page is up: 0 is play, 1 the inventory bar shown, 2 the menu with its icons
(`_INVOV1`, `_INVOV2`), 3 the load page and 4 the save page — five slots
drawn from `_LOADTABLE` at 42-pixel intervals — 5 the documents (`SHOW_DOC`
pages through `_DOC_TABLE`, the *Stern-Dok* material the player has
collected), 6 the quit confirmation (`UNFREEZESCR QUITANIM`), and 7 to 9 the
options: text speed `_TMODE` (`_TSPEED` 225, 150 or 75) and walking speed
`_DMODE` (1, 2 or 3 through `STEPMULTI`). The menu opens from the icon at the
right end of the strip, x ≥ 288, as in the three later games; the readme
calls it *"Menü"*.

## Sound

Three tunes and thirteen samples. Block 24, a PSM 2 module, plays looping
under the intro and again under the ending in location 9; block 25 plays on
the city map and nowhere else; block 19, a bare `PLX` section of 366 bytes,
is the jingle `SET_POINTS` plays once for a scored action — and leaves the
player's tune flag set when it ends, so the room's next effect, jingle or
`ENDTUNE` waits the stop routine's half second over silence first
([PSM 2 music](../../motion16/formats/psm-music.md#the-driver)). Every room but the
map is silent, and every `STARTTUNE` site sits in an `IF` on a variable
**`MAC`**: `MAC @ IF 24 2 PLAYSAMPLE ELSE -1 24 STARTTUNE THEN`. `MAC` is
declared 0 in module 601 and no module ever stores to it, so the branch that
would play the songs' digital rendition through `PLAYSAMPLE` is never taken
([open questions](../../open-questions.md#motion-16-bit)).

The samples are the game's sound effects — a door, a cupboard, a telephone —
called as `13 0 PLAYSAMPLE` from the scene modules, thirty-four sites over
nine blocks. Each cuts whatever tune plays — which in shipped play can only
be `SET_POINTS`'s jingle, since every effect sits in a silent room and the
intro's key runs `ENDTUNE` before the flat's first effect — holds the game
half a second, and then plays once on the DSP's clock through the digital
driver's direct path;
the header the thirteen blocks share, the driver's two paths and what motionvm
makes of them are on the [PSM 2 music](../../motion16/formats/psm-music.md#the-sample--an-sm8-block)
page and in the [departures](../../departures.md#the-16-bit-machine).

## Saving

The engine's three files per slot, as in every 16-bit game: `PUT` writes the
location into `701.blk`, `PUTANIM` the display into `701.anm`, `=>PUTAS` the
resident modules into `701.FRZ`, over the five slots 701 to 705. The pages
that drive them are module 100's — `SHOW_FILES` with the `706 701 DO I =>EXIST
… LOOP` probe, `REMOVE_FILE` — reached from the menu at `_INVMODE` 3 and 4;
loading runs `GET`, `INCLLOC`, `GETANIM` and `=>GETAS` on the slot and then
`UPLOAD_GAME`, which re-marks the sprites and texts the loaded state needs.

## Open questions

- **`MAC`.** What would have set it, and what the name stands for — the
  scripts are ready to play both songs digitally and never do
  ([ledger](../../open-questions.md#motion-16-bit)).
- **The issue-number tables end in 2000**, and what the puzzle does with a
  wrong number on a modern date is unread
  ([ledger](../../open-questions.md#motion-16-bit)).
- **What the 43 animation catalogs in blocks 101–108 and 125–159 hold** has
  not been classified the way Die Enviro-Kids greifen ein's have.
- **Location 12.** Its hotspots — a door, a window, a telephone, a desk, a
  bookend, two chairs — do not say whose office it is.

## See also

- [Module map](module-map.md) — every module
- [Resource inventory](inventory.md) — the counts
- [Other shipped files](other-files.md) — the sound stack, the boot-disk tooling, the readme
- [Boot and frame loop](../../motion16/engine/game-loop.md) — `RUN`, `CTRL`, `INCLLOC`
- [STERN.EXE](../../motion16/engine/stern-exe.md) — the build, and the two words only this game calls
- [Game structure (Die Enviro-Kids greifen ein)](../enviro/game-structure.md) — the same template, two years later
