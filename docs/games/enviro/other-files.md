[← Documentation index](../../README.md)

# Other Shipped Files

*Die Enviro-Kids greifen ein — this page describes the game's own files. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Everything in the game directory that is not the container or the engine.
Fourteen files ship; the game needs four of them to run and five more for
sound.

| File | Size | Date | What it is |
|---|---:|---|---|
| `ENVIRO.EXE` | 167 430 | 1996-08-27 | [The MOTION 16-bit player](../../motion16/engine/enviro-exe.md) |
| `DATA.-1-` | 7 609 296 | 1996-08-26 | [The container](../../motion16/formats/container.md) — the whole game |
| `KIDS.BAT` | 520 | 1996-08-27 | The launcher, below |
| `README.TXT` | 1 082 | 1996-12-22 | German readme: how to provide EMS (`DEVICE=C:\DOS\EMM386.EXE RAM HIGHSCAN` instead of `NOEMS`) |
| `SOUND.EXE` | 11 175 | 1995-07-10 | The PSM 2 sound setup (*"A PARSEC Production"*); writes `PSMCFG4.DAT` |
| `MUSADL.DRV` | 4 480 | 1995-07-10 | PSM 2 Ad Lib music driver: a `MUS\0` header, a sixteen-entry jump table, then flat 16-bit code — the OPL back-end of the sound code compiled into `ENVIRO.EXE` |
| `DMABLAST.DRV` | 9 840 | 1995-07-10 | PSM 2 digital driver, Sound Blaster (tag `DMA\0`) |
| `DMASB2P.DRV` | 10 208 | 1995-07-10 | PSM 2 digital driver, Sound Blaster Pro |
| `DMASB16M.DRV` | 10 128 | 1995-07-10 | PSM 2 digital driver, Sound Blaster 16 mono |
| `DMASB16S.DRV` | 11 040 | 1995-07-10 | PSM 2 digital driver, Sound Blaster 16 stereo |
| `DETECTOR.DRV` | 2 640 | 1995-07-10 | PSM 2 card detection (tag `DTC\0`) |
| `A.DAT` | 117 192 | 1996-12-22 | Not referenced by the player; entropy 7.998 bits per byte, no known magic |
| `32RTM.EXE` | 152 108 | 1996-05-14 | Borland 32-bit runtime manager — not referenced by the player or the launcher |
| `DPMI32VM.OVL` | 58 376 | 1996-05-14 | Its DPMI server — likewise unreferenced |

## The launcher

```
if "%1" == "SOUND" goto new_sound     (also "sound", "Sound")
if exist PSMCFG4.DAT goto play_game
:new_sound
sound.exe
if exist PSMCFG4.DAT goto play_game
echo Bevor Du spielen kannst, mußt Du eine Soundkarte einstellen!
goto endof_bat
:play_game
enviro.exe
```

So `SOUND.EXE` runs first unless a `PSMCFG4.DAT` already exists; the game
will not start without one, and `KIDS SOUND` re-runs the setup. The player
itself opens `psmcfg4.dat` and the six `.DRV` files by name — their names
are in `ENVIRO.EXE`'s strings. `PSMCFG4.DAT` is 36 bytes when written.

## The sound stack

PSM 2 is Parsec's sound system, not the HMI middleware of the 32-bit
engine: the music blocks in the container are `MTCVTS PSM 2.00` modules
([blocks](../../motion16/formats/blocks.md)), and the game ships **two
renderers for the same music**. `MUSADL.DRV` plays it on an OPL2 —
sequencer, fades and register back-end in one file, read whole and
rebuilt: [PSM 2 music](../../motion16/formats/psm-music.md). The
`DMA*.DRV` drivers play it from the modules' `SM8` sample sections
instead: with the configuration's digital flag set and the music flag
clear, the tunes still sound — sampled — so the samples are the music's
own voices, not effects (`PLAYSAMPLE` and `?SAMPLE` are in the kernel
and unused by every script). The configuration picks the driver by index
(`10d8:0110`: 0 and 5 `DMABLAST.DRV`, 1 and 2 `DMASB2P.DRV`, 3
`DMASB16M.DRV`, 4 `DMASB16S.DRV`) and installs it at the configured port
with IRQ 7 and DMA 1 hardcoded (`10d8:01f5`). motionvm plays the Ad Lib
rendition; the digital drivers' internals stay unread. The intro is
silent under either renderer — the first `STARTTUNE` is location 1's.

## The leftovers

`A.DAT` carries the readme's date, three months after the game's files,
and nothing in `ENVIRO.EXE` names it — the only `.dat` string in the
player is `psmcfg4.dat`. `32RTM.EXE` and `DPMI32VM.OVL` are the Borland
32-bit extender; `ENVIRO.EXE` is a real-mode program and neither it nor
the launcher mentions them. All three read as packaging leftovers and can
be left out.

## See also

- [ENVIRO.EXE](../../motion16/engine/enviro-exe.md) — what the player asks for at run time
- [Other files (Dunkle Schatten 2)](../ds2/other-files.md) — the 32-bit game's directory
