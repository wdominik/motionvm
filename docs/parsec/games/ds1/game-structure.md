[← Documentation index](../../README.md)

# Game Structure

*Dunkle Schatten — how the game is put together out of the location table of `XP01.EXE`: the rows, the opening, the map and the days, the forest and the maze, and saving. The machinery behind it is documented under [the engine](../../README.md#the-engine).*

The game is sixty-one rows of the program's location table, each naming a
handler of its own, a name, eight exits and up to nine objects
([game loop](../../engine/game-loop.md)). What a place does — who stands
in it, what a click on a thing answers, which scene plays and what it
changes — is that handler, compiled; the room it shows is the
`DSROOM.DAT` slot of the row's number ([rooms](../../formats/rooms.md)).

## The rows

The table names thirty of its rows, and the names are the game's own:

- **Row 0, *Extro*, and row 20** are not places. A main loop that arrives
  on one walks on to the next row that is one, with entry `7Fh`: from row
  20 to the Schuppen, row 21, and from row 0 to the Karte, row 1.
- **Row 1, the Karte**, is the town map, where a place is chosen. Its
  objects 200 to 223 stand for locations 0 to 23, one each, and a byte per
  place at `DS:26F6h` says whether it is shown on the map; a place that
  will not be gone to plays a scene instead (`DS:262Eh`).
- **Rows 2 to 19, 21 and 22** are the town's places, from Karsten's room,
  the *Zimmer*, the *Küche* and the *Wohnzimmer*, to the *Straße*, the *Schule*, the *Werkstatt*, the *Krankenhaus*, the
  *Kiosk*, the *Ruine* and *In der Ruine*, the *Schuppen* and the
  *Stammtisch* — and the *JZ*, row 8, the Jugendzentrum the game is
  about.
- **Rows 24 to 26, *JZ(1)* to *JZ(3)***, are the Jugendzentrum again: all
  four JZ rows name one handler, one hook and one animation hook, whose
  arms are chosen by one state byte, `[271Ch]`.
- **Row 23, the *Wald***, is the forest, and **rows 30 to 59** are the
  maze behind it: thirty rows without names, of kind 3, all drawn on the
  forest picture of slot 20 ([objects](../../engine/objects.md)).
- **Rows 27 to 29, *F1* to *F3***, are of kind 5; between two of them an
  entry reveals the new picture rather than sliding it in, as between two
  forest rows.
- **Row 60** is no place either: it is the way back into a loaded game
  ([options and the saved game](../../engine/options.md#location-60)).

## The opening

A fresh game opens at **location 2, the Zimmer, on entry `7Fh`**: the
program's start sets the two words when nothing it loaded left a location
(`0x4F2A`, `0x4F37`, `0x4F3D`). Before that, the game options come up
once, unconditionally ([boot](../../engine/boot.md)). Karsten lies in bed:
while he sleeps, the bed's animation is the figure (`0x985`), and the
standing figure is taken out of its draw chain (`0x0AA1F`) until he
wakes.

## The days

The day is the byte `[2727h]`. The Karte shows it, as the frame of its
marker object 202, and the handlers switch on it: the Zimmer of a later
day is entered in the state its wake-up leaves, the Küche has an arm for
the fourth day, and the Jugendzentrum opens on the map on the first. The
readme's tip says what closes a day: Karsten going to bed in his room.
A routine of plain story writes in the global rule handlers' segment,
`1586:06B1` (`0x15F11`), sets the state a chapter's turn leaves behind; the
JZ and the Schule call it, and so does the *Brand*.

## The forest and the maze

The Wald and the thirty maze rows share the forest picture: a row of kind
3 shows slot 20 and binds its own object map to it, which renames the
picture's objects, hotspots, walk-graph nodes and animation entries for
that row ([object maps](../../formats/object-map.md)). Between two forest
rows the picture stays loaded and the music plays on
([music](../../engine/music.md)).

The maze's thirty rows share one handler too, and it keeps a byte per
location in the segment `1FFCh`, outside `DGROUP` (`1FFC:0482h`). On the
first visit it fills each maze row's byte with the row's distance from the
three ways out — rows 57, 30 and 44 — and then overwrites it, row by row,
with the exit that leads one step closer; the maze's rule walks the player
through that exit.

## Saving

There is **one saved game and no slot**: `SAVEGAME.SAV`, written whole from
five blocks of the data segment, and `SAVEGAME.CFG` beside it for the
settings ([options and the saved game](../../engine/options.md)). The title
program's *Neues Spiel beginnen* deletes the saved game before `XP01`
starts; *Altes Spiel fortsetzen* leaves it, and `XP01` restores it at once
through location 60. Loading in game asks first, *Tatsächlich neu
einladen?*, and sends the main loop to location 60, whose handler copies
the blocks back and returns to the location the game was saved in.

## See also

- [The game](README.md) — what it is about and who made it
- [Game loop](../../engine/game-loop.md) — the location table and the main loop
- [Objects](../../engine/objects.md) — the forest picture and its object maps
- [Options and the saved game](../../engine/options.md) — the files and location 60
