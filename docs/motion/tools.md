[← Documentation index](README.md)

# motionvm-motion-tools

*The command-line inspector, built from a checkout, across both generations of the engine. It reads a game's containers and writes out what it finds; what those containers are is documented under [MOTION 32-bit](README.md#motion-32-bit) and [MOTION 16-bit](README.md#motion-16-bit).*

`motionvm-motion-tools` reads a game's containers and writes what it finds as ordinary
files — a 32-bit game's `NNN.RSC` banks or a 16-bit game's `DATA.-n-` volumes,
told apart by the files in the directory. It asks less of a directory than the
player does: a container and the engine binary beside it are enough, and it
reads a MOTION game whether or not motionvm plays it.

The five commands that read a game take its directory as their first
argument, and the two comparisons take files; `--help` prints the same
summary this page does. The release archives carry only `motionvm`, so
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
the interpreter itself owns — the rest are the engine's, which these tools
deliberately know nothing of.

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
`BMZ.EXE`'s, `STERN.EXE`'s or `LL.EXE`'s, whichever the directory holds, for a 16-bit one. The
table has to come from the game's own build: ordinals shift between builds, and
a table from the wrong one names every word some distance along without saying
anything is wrong.

```sh
motionvm-motion-tools script /path/to/gamedata 323 | less
```

Reading a compiled module is in scope for this project; writing one is not.

## `registers <gamedata> <block> [--ticks N] [--out FILE]`

A song's OPL register stream, rendered offline: the block straight out of the
container, played through the sequencer and the driver the player runs — the
OPL3 driver of `HMIMDRV.386` with its two instrument banks for a 32-bit game,
`MUSADL.DRV` for a 16-bit one — and written as `tick register value`, one
write a line, the driver's switch-on at tick 0 and the song from tick 1. A
16-bit song is started to loop, as `-1 N STARTTUNE` starts nearly every one;
a 32-bit song loops by its own data. `--ticks` defaults to 6000, `--out` to
`registers-NNN.txt`.

```sh
motionvm-motion-tools registers /path/to/gamedata 25
```

This is our side of `compare-dro`. No engine is involved, which is what
makes the stream a statement about the audio layer alone.

## `compare-frame <ours.png> <theirs.png> [--crop X,Y,W,H] [--scale N]`

The frame F12 wrote against a capture of the original, on the colors the DAC
held — six bits a channel, each picture through its own palette, so a
renumbered palette is the same picture and a screenshot's widening does not
matter. `--crop` takes the picture area out of a screenshot that shows more
than the frame, `--scale N` keeps every Nth pixel of it for a window that
showed the frame at a whole multiple. The report says how many pixels differ,
where, and the first of them with both sides' index and color; the command
exits 1 on a difference.

```sh
motionvm-motion-tools compare-frame shot.png capture.png
```

## `compare-dro <registers.txt> <capture.dro>`

A rendered register stream against a DRO version 2 recording of the original
— DOSBox-X's `DX-CAPTURE /O`. Both sides are reduced to the writes that
change a register, the recording's snapshot at its first note is set apart,
the streams are lined up on the write it resumes with, the chip's state at
that note is held register for register, and then the streams write for
write. The first difference is named with its position and the writes around
it; the command exits 1 on one.

```sh
motionvm-motion-tools compare-dro registers-025.txt capture.dro
```

How the captures on the other side are made, and what a match does and does
not mean, is on [The verification method](verification-method.md).

## See also

- [Documentation index](README.md) — the formats these commands write out
- [Script modules](motion32/formats/script-modules.md) — what a `.scr` is
- [Kernel words](motion16/vm/kernel-words.md) — the tables the disassembler resolves against
- [Verification](verification.md) — what the extracted material is held against
- [The verification method](verification-method.md) — the captures the two comparisons take, and how they are made
