[← Documentation index](../../README.md)

# Other Shipped Files

*Hilfe für Amajambere — this page describes the game's own installation. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Nineteen files, and motionvm opens four of them.

| File | Size | Date | What it is |
|---|---:|---|---|
| `DATA.-1-` | 506 066 | 1995-06-06 | Volume 1 of the container: the scripts, the texts and the music |
| `DATA.-2-` | 4 827 543 | 1995-06-06 | Volume 2: every sprite, every palette, all seven fonts and the font reference table |
| `BMZ.EXE` | 166 806 | 1995-06-05 | The player — [read, not run](../../motion16/engine/bmz-exe.md) |
| `MUSADL.DRV` | 4 480 | 1994-02-05 | The PSM 2 Ad Lib driver, rebuilt from its tables |
| `SOUND.EXE` | 11 175 | 1994-03-31 | The sound setup, which writes `PSMCFG4.DAT` |
| `DETECTOR.DRV` | 2 640 | 1993-07-25 | Sound-card detection |
| `DMABLAST.DRV` | 9 840 | 1994-02-16 | Sound Blaster digital output |
| `DMASB2P.DRV` | 10 208 | 1994-02-18 | Sound Blaster Pro 2 digital output |
| `DMASB16M.DRV` | 10 128 | 1994-02-18 | Sound Blaster 16 mono digital output |
| `DMASB16S.DRV` | 11 040 | 1994-02-18 | Sound Blaster 16 stereo digital output |
| `AFRIKA.BAT` | 785 | 1995-07-05 | The launcher |
| `VRCHKSUM.EXE` | 9 271 | 1994-10-21 | The integrity checker |
| `ORIGINAL.BIN` | 1 924 | 1995-06-07 | Its reference data |
| `ORIGINAL.REP` | 621 | 1995-06-07 | Its readable report |
| `ORIGINAL.SCR` | 173 | 1995-06-07 | The list of files it checks |
| `CONFIG.DAT` | 9 | 1995-07-05 | Installer output |
| `INFO.TXT` | 3 212 | 1995-06-06 | German readme (requirements, setup) |
| `FREEWARE.TXT` | 1 263 | 1995-05-26 | The licence |
| `LIESMICH.DOK` | 3 827 | 1995-06-01 | A printable reply card to the ministry |

The dates are coherent and not a re-stamp, unlike Jeff Jet's: the drivers carry
their 1993–1994 build dates, the game its 1995-06 ones, and the two files the
installer writes — `AFRIKA.BAT` and `CONFIG.DAT` — a month later. This is the
oldest-dated of the games motionvm plays.

## The launcher

```bat
@echo off
cls
if exist ..\afrika.bat erase ..\afrika.bat
vrchksum /cmd:testbinary /bin:original.bin
pause
…
if "%1" == "SOUND" goto new_sound
if exist PSMCFG4.DAT goto play_game
:new_sound
sound.exe
if exist PSMCFG4.DAT goto play_game
… Bevor Sie spielen können, müssen Sie eine Soundkarte einstellen!
:play_game
bmz.exe
```

The integrity check first, then the sound setup on first run or on
`AFRIKA SOUND`, then the game — and the game only if `PSMCFG4.DAT` exists, so
an unconfigured copy never starts. The `erase ..\afrika.bat` on the third line
is the installer's mark: the batch file that ran the installation deletes
itself from the parent directory.

`PSMCFG4.DAT` is not in this copy. It is written by `SOUND.EXE` on the player's
machine and was removed when the corpus was cleaned of run artifacts; motionvm
never reads it, having no sound card to configure.

## The sound stack

`SOUND.EXE` and all six `*.DRV` are **byte-identical** to the copies both
sibling games ship — the same files, from the same PSM 2 licence, with their
own 1993–1994 dates. motionvm opens exactly one of them, `MUSADL.DRV`, and
rebuilds the Ad Lib player around its tables; see
[PSM 2 music](../../motion16/formats/psm-music.md). The five digital-output
drivers are never opened, because the game never plays a sample: `PLAYSAMPLE`
and `XGFXSAMPLE` are in the kernel and called zero times.

## The integrity chain

No other game in the corpus ships one. `VRCHKSUM.EXE` — a PRSC-compressed MZ
binary that signs itself *VRCHKSUM 1.00, (C) … The Art Department* — is run by
the launcher as `vrchksum /cmd:testbinary /bin:original.bin` and reports on the
fourteen files named in `ORIGINAL.SCR`: everything shipped except the three
files the check itself consists of and the installer's `CONFIG.DAT`.

`ORIGINAL.REP` is the readable form, one line per file:

```
[CRC 40D3B874] [  166806 Bytes] BMZ.EXE
[CRC 4DB0F60B] [  506066 Bytes] DATA.-1-
[CRC E038AD7A] [ 4827543 Bytes] DATA.-2-
```

All fourteen sizes match the files on disc exactly. The checksum column does
not match any standard CRC-32 variant — zlib, BZIP2, JAMCRC, MPEG-2, POSIX and
XFER were all tried against `SOUND.EXE`, whose zlib CRC-32 is `C00814B6` where
the report claims `31B54817` — so the algorithm is `VRCHKSUM.EXE`'s own and
sits inside the compressed image, unread.
`ORIGINAL.BIN` is the same report in a form the checker reads back: a
`VRCHKSUM_BINARY\0` magic and 1908 bytes of high-entropy data with no readable
file names.

None of this runs under motionvm, which opens the container and the player
binary directly. The chain is documented because it is part of what the game
shipped, not because anything reads it.

## The readmes

`INFO.TXT` states the machine the game was written for: an AT with an 80286,
MS-DOS 3.30 or later, VGA or MCGA, a mouse, about 550 KB of free base memory,
about 1 MB of EMS and some 5 MB of disc — with an AdLib card as the minimum for
music and a 386/33 with a Sound Blaster as what it runs best on. The same
figures the binary's own prompts name.

`FREEWARE.TXT` is the licence: the game is a copyright of the **ART DEPARTMENT
WA GmbH** of Bochum, made for the Bundesministerium für wirtschaftliche
Zusammenarbeit und Entwicklung, and given away as freeware — copyable
unmodified, at most 5 DM for the disc, and not to be put on CD-ROM
compilations or disc magazines without consent.

`LIESMICH.DOK` is not a technical file at all. It is a reply card to be printed
and posted to the ministry in Bonn, with a competition — six trips to Bonn and
sixty information packages, closing 1995-12-31 — and a form for ordering
brochures.

## See also

- [Resource inventory](inventory.md) — what the two volumes hold
- [BMZ.EXE](../../motion16/engine/bmz-exe.md) — the player this game reads its kernel table out of
- [PSM 2 music](../../motion16/formats/psm-music.md) — the driver and the songs
- [Other shipped files (Jeff Jet - Abenteuer InfoHighway)](../jeffjet/other-files.md) — the same sound stack, a different launcher
