[← Documentation index](../../README.md)

# Other Shipped Files

*Checker 2000 — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

Twenty-four entries, 34 MB, and motionvm opens seven of them — ten with
sound. The copy on hand was re-stamped 2010-10-30 wholesale; the two dates
below that are not that one are the two that survived.

| File | Size | Date | What it is |
|---|---:|---|---|
| `001.RSC` | 5 601 580 | — | [Resource container](../../motion32/formats/container.md) — scripts, texts, blocks, palettes, 33 sprites |
| `002.RSC` | 9 768 978 | — | [Resource container](../../motion32/formats/container.md) — nearly all the artwork, and the three fonts |
| `003.RSC` | 13 271 686 | 1996-03-10 | [Resource container](../../motion32/formats/container.md) — 88 sprites with a palette each: the photo story |
| `004.RSC` | 3 687 834 | — | [Resource container](../../motion32/formats/container.md) — 43 blocks: 39 sound effects as WAV, and four songs ([inventory](inventory.md#blocks)) |
| `ENGINE.RSC` | 53 198 | — | [Resource container](../../motion32/formats/container.md) — the system font and the startup palette, below |
| `ENGINE.EXE` | 729 551 | — | [The MOTION 32-bit engine, build R78](../../motion32/engine/engine-r78.md) |
| `000.FRT` | 516 | — | [Font reference table](../../motion32/formats/font-reference-table.md) — character to glyph |
| `HMIMDRV.386` | 117 852 | — | [Driver archive](../../motion32/formats/driver-archive.md): the OPL3 driver whose tables the rebuilt FM driver reads |
| `MELODIC.BNK` | 5 404 | — | [Ad Lib bank](../../motion32/formats/adlib-bank.md) — melodic patches |
| `DRUM.BNK` | 5 404 | — | [Ad Lib bank](../../motion32/formats/adlib-bank.md) — percussion patches |
| `SMPPATH` | 18 | — | Where `->STARTSAMPLE`'s files are, below |
| `WAVS/` | 73 files | — | The speech, one WAV file a line |
| `CHECKER.BAT` | 103 | — | The launcher, below |
| `SYSTEM.RSC` | 43 | — | The plain-text Forth bootstrap, below |
| `PATH1.RSC` | 27 | — | A second bootstrap, below |
| `DOS4GW.EXE` | 265 396 | 1996-10-01 | The DOS/4GW 32-bit extender the engine runs on |
| `SETUP.EXE` | 165 899 | — | HMI's sound-card setup; writes `HMISET.CFG` |
| `SETUP.INI` | 3 445 | — | Its configuration, below |
| `HMIDRV.386` | 317 317 | — | [Driver archive](../../motion32/formats/driver-archive.md) — the digital drivers |
| `HMIDET.386` | 83 774 | — | [Driver archive](../../motion32/formats/driver-archive.md) — card detection |
| `TEST.HMI` | 13 109 | — | Sound-setup test song — [HMI](../../motion32/formats/hmi.md); the same song is block 1 of `001.RSC` |
| `TEST.WAV` | 80 684 | — | Sound-setup test sample; the same file is block 3 of `001.RSC` |
| `RSC.INF` | 249 592 | — | The resource catalog the engine's lookup reads, below |
| `README.TXT` | 2 455 | — | German readme, below |

The seven the player needs are the five containers, `ENGINE.EXE` and
`000.FRT`; `HMIMDRV.386`, `MELODIC.BNK` and `DRUM.BNK` add music, and
`SMPPATH` with `WAVS/` adds the speech. The rest are never opened.

## ENGINE.RSC

What Dunkle Schatten 2 ships as two loose files — `000.FNT`, the system
font, and `000.PAL`, the palette the engine starts with — this game keeps in
a fifth container of two items, font 0 and palette 0. The engine registers
it before the numbered containers, so it is slot 0 of its catalog, and an
id filled here and in a numbered container resolves here; the two shipped
games fill no such id, but nineteen sprites and one palette are filled in two
*numbered* containers of this game, and the same rule — the lowest slot —
decides those ([containers](../../motion32/formats/container.md#multi-file-overlay)).
The system font is twelve pixels tall.

## The launcher

### CHECKER.BAT

```bat
@echo off
SET DOS4GVM=MAXMEM#16384
if exist hmiset.cfg goto Start
Setup
:Start
ENGINE
@echo on
```

The extender is told its memory cap on the command line, 16 MB, where Dunkle
Schatten 2 hands it a configuration file; the sound setup runs **only if no
configuration exists yet**, then `ENGINE`.

### SYSTEM.RSC

The bootstrap the engine interprets as Forth source at start-up:

```
" C:\checker\"
3 ->RSCPATH
4 =>GET
START
```

Two lines more than Dunkle Schatten 2's `4 =>GET` / `START`. `->RSCPATH
( path$ slot -- )` (R78 `0x4cf70`) copies the string into the path field of
container slot `slot` of the engine's container table (`0xc8c3c`, `0x78` a
record, the path at `+0x20`), for a slot of 0 to 5 — so `003.RSC`, slot 3,
is looked for in `C:\checker\` while the other containers are wherever the
engine was started from. That is the shape of a game run from CD with its
largest container installed to the hard disk, and `003.RSC` is the one file
whose original date the copy kept.

### PATH1.RSC

```
3 ->RSCPATH
4 =>GET
START
```

The same bootstrap without the path: `->RSCPATH` then takes whatever the
stack holds as its string. Nothing in the shipped files reads this file
under this name; it is the bootstrap for an installation where the path is
pushed by something else, or a leftover.

### SMPPATH

```
C:\Checker\wavs#
```

The directory `->STARTSAMPLE` prefixes to the file names the scripts hand
it, `#` starting a comment ([audio](../../motion32/engine/audio.md)).
motionvm reads the path but looks under the game's own directory for the
last component, `WAVS/`, so a copy installed anywhere plays its speech.

## RSC.INF

The resource catalog: a header, then a record per resource kind with one
byte per id whose bits say which container slots hold the item. It is the
same size as Dunkle Schatten 2's, and it was held against this game's five
containers id for id: wherever two containers hold one id the catalog names
the lowest slot, and it names no slot without the item — which is where the
lowest-slot rule of [containers](../../motion32/formats/container.md#multi-file-overlay)
was confirmed. motionvm does not read it; the containers say the same thing.

## Sound setup

`SETUP.EXE` and `SETUP.INI` are HMI's *Sound Operating System Setup
Utility* of 1995, the same kit Dunkle Schatten 2 ships as `SNDSETUP.*`:
eighteen digital devices from *No Digital Device* to *Reveal FX/32*, the
test files `TEST.WAV` and `TEST.HMI`, `MELODIC.BNK` and `DRUM.BNK` for the
MIDI test, and `HMISET.CFG` as the file it writes and the launcher looks
for.

## README.TXT

Five numbered points and a paragraph, in German: a VESA-compatible graphics
card is required, and a VESA driver from the card's disks where the card
has none built in; the game is played entirely with the mouse and needs a
DOS mouse driver; 4 MB of memory and 500 KB of conventional memory, with
SMARTDRIVE named as the likely culprit when there is less; a 16 MB swap file
is created in the root of the current partition and removed on exit; and
the game runs under DOS and should be started from *"richtigem" DOS*, not
from Windows or OS/2. The closing paragraph advises a boot floppy and a
backup of every system file changed.

## See also

- [The game](README.md)
- [Game structure](game-structure.md)
- [Module map](module-map.md)
- [ENGINE.EXE V0.04.15/R78](../../motion32/engine/engine-r78.md) — the build
- [Dunkle Schatten 2's other files](../ds2/other-files.md) — the same sound kit under its other name
