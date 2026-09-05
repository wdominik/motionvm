[← Documentation index](../../README.md)

# Other Shipped Files

*Falsches Spiel mit Eddie M. — this page describes the game's own installation. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Fifteen files, and motionvm opens five of them.

| File | Size | Date | What it is |
|---|---:|---|---|
| `DATA.-1-` | 216 697 | 1994-10-13 | Volume 1 of the container: the scripts, the texts, the per-location tables, the catalogs |
| `DATA.-2-` | 1 265 145 | 1994-10-10 | Volume 2: 995 sprites, every palette, both fonts, the font reference table, the samples and the songs |
| `DATA.-3-` | 1 195 841 | 1994-10-10 | Volume 3: 774 sprites |
| `STERN.EXE` | 167 334 | 1994-10-06 | The player — [read, not run](../../motion16/engine/stern-exe.md) |
| `MUSADL.DRV` | 4 480 | 1994-02-05 | The PSM 2 Ad Lib driver, rebuilt from its tables |
| `SOUND.EXE` | 11 175 | 1994-03-31 | The sound setup, which writes `PSMCFG4.DAT` |
| `DETECTOR.DRV` | 2 640 | 1993-07-25 | Sound-card detection |
| `DMABLAST.DRV` | 9 840 | 1994-02-16 | Sound Blaster digital output |
| `DMASB2P.DRV` | 10 208 | 1994-02-18 | Sound Blaster Pro 2 digital output |
| `DMASB16M.DRV` | 10 128 | 1994-02-18 | Sound Blaster 16 mono digital output |
| `DMASB16S.DRV` | 11 040 | 1994-02-18 | Sound Blaster 16 stereo digital output |
| `MAKEBOOT.EXE` | 22 896 | 1994-11-08 | Makes a boot floppy for a machine short of memory |
| `MOUSE.SYS` | 45 648 | 1991-11-01 | A Microsoft mouse driver, for that floppy |
| `README.BAT` | 39 | 1994-11-09 | `type readme.txt | more` |
| `README.TXT` | 2 631 | 1994-11-10 | German readme |

The dates are coherent and not a re-stamp: the drivers carry their 1993–1994
build dates, the game its October 1994 ones, and the tooling that came with the
release a month later. `MOUSE.SYS` is older than everything else, a driver
bundled and not written.

## No launcher

There is none. The readme's instruction is *"Tippen Sie STERN"* — the player
runs the binary — and the installer, on the *"rote Diskette 1/2"* the readme
names and this copy does not include, left nothing behind but these files.
`PSMCFG4.DAT`, which the player opens by name, is not in this copy either: it
is written by `SOUND.EXE` on the player's machine, and the readme corrects the
box, which named the setup program `PSMCFG` — *"Es handelt sich um einen
Druckfehler"*. motionvm reads neither the configuration nor the setup program.

## The sound stack

`SOUND.EXE`, `MUSADL.DRV`, `DETECTOR.DRV` and the four `DMA*.DRV` are
**byte-identical** to the copies Die Enviro-Kids greifen ein, Jeff Jet and
Hilfe für Amajambere ship: the same PSM 2 license, two years before those
games, and the driver Victor Loomes' older build is the only exception to.
motionvm opens exactly one of them, `MUSADL.DRV`, and rebuilds the Ad Lib
player around its tables; see [PSM 2 music](../../motion16/formats/psm-music.md).

The digital drivers matter more here than in the sibling games. This is the
one game whose scripts call `PLAYSAMPLE` — thirty-four times, for the sound
effects in the thirteen `SM8` blocks — and the player loads one of the four
`DMA*.DRV` by the index `SOUND.EXE` wrote when the setup chose digital sound.
`DMABLAST.DRV` is read for its play path and rebuilt from that reading — the
player opens none of the four; the suite reads the time-constant table out of
each — and the word runs the way the player runs
it with both cards on ([PSM 2 music](../../motion16/formats/psm-music.md#the-sample--an-sm8-block),
[departures](../../departures.md#the-16-bit-machine)).

## The boot-disk tooling

The readme's second section is about memory: *"kein EMS"*, *"zuwenig Base
Memory"*, a mouse that does not work. Its answer is `MAKEBOOT.EXE`, which asks
for a formatted floppy, copies the system to it with `SYS.COM`, adds
`KEYBOARD.SYS` and `MOUSE.SYS`, and writes a `CONFIG.SYS` that loads `HIMEM.SYS`
and `EMM386.EXE` — its own text asks for at least 1024 KB of EMS, 2500
recommended, and 555 KB of base memory, the same figures the player's prompts
name. The machine then boots from the floppy and starts the game itself;
*"Erneut spielen: Tippen Sie STERN."* The banner inside the tool is the title
as the game spells it, *FALSCHES SPIEL MIT EDDIE M.*, where the catalogs write
*Eddy*.

## The readme

`README.TXT` is five numbered points: no Windows while installing or playing;
the boot disk; the setup program's misprinted name; the right mouse button,
pressed *"während der Name oder die Bezeichnung erscheint"*, to talk to people
and take things; and the menu — *"indem Sie mit dem Mauszeiger in das untere
Feld gehen und 'Menü' anklicken"* — for saving and the speed settings.

## See also

- [Resource inventory](inventory.md) — what the three volumes hold
- [STERN.EXE](../../motion16/engine/stern-exe.md) — the player this game reads its kernel table out of
- [PSM 2 music](../../motion16/formats/psm-music.md) — the driver and the songs
- [Other shipped files (Hilfe für Amajambere)](../hfa/other-files.md) — the same sound stack, a launcher and an integrity chain
