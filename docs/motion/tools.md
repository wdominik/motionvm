[← Documentation index](README.md)

# motionvm-motion-tools

*The command-line inspector, built from a checkout, across both generations of the engine. It reads a game's containers and writes out what it finds; what those containers are is documented under [MOTION 32-bit](README.md#motion-32-bit) and [MOTION 16-bit](README.md#motion-16-bit).*

`motionvm-motion-tools` reads a game's containers and writes what it finds as ordinary
files — a 32-bit game's `NNN.RSC` banks or a 16-bit game's `DATA.-n-` volumes,
told apart by the files in the directory. It asks less of a directory than the
player does: a container and the engine binary beside it are enough, and it
reads a MOTION game whether or not motionvm plays it.

Every command takes the game directory as its first argument; `--help` prints
the same summary this page does. The release archives carry only `motionvm`, so
the tool is run from a checkout:

```sh
cargo run --release -p motionvm-motion-tools -- info /path/to/gamedata
```

The examples below abbreviate that invocation to `motionvm-motion-tools`, and their
ids are Dunkle Schatten 2's.

## `info <gamedata>`

One line per resource bank: how many sprites, texts, songs, fonts, palettes and
script modules it holds, and how much of the file is not accounted for. The
quickest way to tell a complete installation from a partial one.

```sh
motionvm-motion-tools info /path/to/gamedata
```

## `extract <gamedata> [--out DIR] [--pal N]`

Writes everything out at once, each resource both as the raw bytes it is stored
as and as something you can look at: sprites as indexed PNGs, palettes as
`.pal` plus a swatch grid, fonts as `.fnt` plus a contact sheet and a JSON of
their glyph metrics, text tables as JSON, and script modules as `.scr`.
`--out` defaults to `out`.

For a **32-bit** game it also writes the songs as `.hmi`, a `.f` disassembly of
every module with kernel words resolved by name, and `kernel-usage.txt`, which
lists the kernel words the game's own code reaches for and marks which of them
this engine implements.

For a **16-bit** game the blocks come out as `.blk` with an index that marks
the PSM 2 songs; the modules come out additionally as `scripts/modules.txt`,
every module's symbol table, and as `.f` listings read through the kernel table
of that game's own engine binary, with its own `kernel-usage.txt`; and the
sprites — which carry no palette of their own — come out through palette
`--pal N`, palette 0 by default.

```sh
motionvm-motion-tools extract /path/to/gamedata --out out
```

## `sprite <gamedata> <id> [--out FILE] [--pal N]`

One sprite as an indexed PNG, palette index 0 written as transparent; `--pal`
picks the palette for a 16-bit sprite. `--out` defaults to `sprite-NNNN.png`.

```sh
motionvm-motion-tools sprite /path/to/gamedata 1010
```

## `script <gamedata> <id>`

One script module on stdout: its header, its symbol table, and its threaded
code disassembled with kernel words resolved by name — through `ENGINE.EXE`'s
table for a 32-bit module, and through `ENVIRO.EXE`'s, `HPPLAY.EXE`'s,
`BMZ.EXE`'s or `LL.EXE`'s, whichever the directory holds, for a 16-bit one. The
table has to come from the game's own build: ordinals shift between builds, and
a table from the wrong one names every word some distance along without saying
anything is wrong.

```sh
motionvm-motion-tools script /path/to/gamedata 323 | less
```

Reading a compiled module is in scope for this project; writing one is not.

## See also

- [Documentation index](README.md) — the formats these commands write out
- [Script modules](motion32/formats/script-modules.md) — what a `.scr` is
- [Kernel words](motion16/vm/kernel-words.md) — the tables the disassembler resolves against
- [Verification](verification.md) — what the extracted material is held against
