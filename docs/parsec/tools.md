[← Documentation index](README.md)

# motionvm-parsec-tools

*The command-line inspector, built from a checkout. It reads the `RESOURCE/` files of the PARSEC games motionvm plays and writes out what it finds; what those files are is documented under [formats](README.md#formats).*

`motionvm-parsec-tools` reads the containers of a Dunkle Schatten or
Captain Zins installation and writes what it finds as ordinary files or
lines. It asks less of a directory than the player does: the `RESOURCE/`
directory with the game's room container in it — `DSROOM.DAT` or
`CZROOM.DAT` — is what tells it which game is there, and each command
reads only the files it names. A directory without the room container is
refused with a sentence, rather than read until some file is missing.

Every command takes the game directory as its first argument — the one
holding `XP01.EXE` and `RESOURCE/`; `--help` prints the same summary this
page does. The release archives carry only `motionvm`, so the tool is run
from a checkout:

```sh
cargo run --release -p motionvm-parsec-tools -- info /path/to/gamedata
```

or `just tools-parsec info /path/to/gamedata`. The examples below
abbreviate that invocation to `motionvm-parsec-tools`.

## `info <gamedata>`

The game's name and slug, then one line per `DAT` file in `RESOURCE/`: an
FGROUP with its slot count and how many slots are RIBs and how many
`NULLPTR`, or a RIB with its resource count, its buffer size and its
stored and unpacked lengths. The quickest way to tell a complete
installation from a partial one.

## `extract <gamedata> [--out DIR]`

Every room playfield of the room container as an indexed PNG, 320×128 in the
room's own palette, with a text file beside each giving its viewport, tile
and map sizes and its hotspot list — rectangle, verb mask and name — and
the pictures of the strip's files (Dunkle Schatten's `PANELS.DAT`,
Captain Zins's `PANEL0.DAT` and `PANEL1.DAT`) and `CMBMOU.DAT` as indexed
PNGs through the
palette their file carries. `--out` defaults to `out`.

## `picture <gamedata> <file> [n] [--out FILE]`

One resource of one RIB file as an indexed PNG: resource `n`, 0 by default,
written to `picture.png` unless `--out` says otherwise. A room comes out as
its 320×128 playfield in its own palette, anything else as a picture
through the palette its file carries.

## `dialogues <gamedata>`

Every dialogue bank of the game, its places numbered the way a handler
names them: the two global banks of `DIALOG.DAT`, 999 and 2, and each
location's own out of its room slot. The texts are shown with their
controls read rather than raw: a `02 n` substitution prints as `«n»`,
because what it stands for is drawn at run time; the `01` and `03` pairs,
which steer the painter, are dropped; and the page break prints as ` / `.

## `rib <gamedata> <file>`

What each resource of one `DAT` file is, one line per resource: its index,
its size and what the readers make of it — a palette bank, a picture, a
cel, an opcode stream, a font, a room — or its first bytes when no reader
claims it. A RIB is a numbered bag of blobs with no type field, so this is
where reading a file starts; a guess here is a starting point, since two
of the shapes overlap and the program is the authority on which one it
meant.

## `room <gamedata> <id>`

One room in text: its viewport, tile and map sizes and cel count, its
layers, every object record with its id, kind, flags, position, anchor,
bundle, cel, scale and layer, the walk graph's nodes, and the hotspot
table with each hotspot's texts and the verb that reaches each.

## `registers <gamedata> <module> [--ticks N] [--out FILE]`

Module `N` of `PSMP2L.DAT` played endlessly through the game's own
`MUSADL.DRV` out of `PSMDRV.DAT`: every OPL register write, one per line as
`tick register value`, the driver's switch-on on tick 0 and the song from
tick 1. `--ticks` defaults to 6000 driver ticks, `--out` to
`registers-NN.txt`. No engine is involved — the song is started by its
number, as the Ad Lib path hands its bare `PLX` to the driver — and the
file is the side of a comparison with an OPL capture of the original that
this repository can produce ([verification](verification.md)).

## See also

- [Formats](README.md#formats) — what these files are
- [Verification](verification.md) — the comparison `registers` is made for
