[← Documentation index](../../README.md)

# Other Shipped Files

*Dunkle Schatten 2 — this page describes the game's own data and script library. The engine it runs on is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

Thirty files, 20.5 MB, and motionvm opens five of them — eight with sound.

| File | Size | Date | What it is |
|---|---:|---|---|
| `001.RSC` | 3 799 821 | 1996-10-25 | [Resource container](../../motion32/formats/container.md) — scripts, texts, blocks, fonts, palettes |
| `002.RSC` | 12 296 112 | 1996-10-13 | [Resource container](../../motion32/formats/container.md) — nearly all the artwork |
| `003.RSC` | 2 090 509 | 1996-10-13 | [Resource container](../../motion32/formats/container.md) — 57 more sprites |
| `ENGINE.EXE` | 845 467 | 1996-10-24 | [The MOTION 32-bit engine](../../motion32/engine/engine-exe.md) |
| `000.FRT` | 516 | 1996-10-01 | [Font reference table](../../motion32/formats/font-reference-table.md) — character to glyph |
| `HMIMDRV.386` | 117 852 | 1996-09-20 | [Driver archive](../../motion32/formats/driver-archive.md): the OPL3 driver whose tables the rebuilt FM driver reads |
| `MELODIC.BNK` | 5 404 | 1995-09-15 | [Ad Lib bank](../../motion32/formats/adlib-bank.md) — melodic patches |
| `DRUM.BNK` | 5 404 | 1995-09-14 | [Ad Lib bank](../../motion32/formats/adlib-bank.md) — percussion patches |
| `DS2.BAT` | 102 | 1996-10-28 | The launcher, below |
| `_RUNVM.VMC` | 87 | 1996-10-28 | DOS/4GW virtual-memory configuration, below |
| `SYSTEM.RSC` | 16 | 1996-10-13 | The plain-text Forth bootstrap, below |
| `DOS4GW.EXE` | 265 396 | 1996-10-01 | The DOS/4GW 32-bit extender the engine runs on |
| `DS2.ICO` | 766 | 1996-10-11 | Program icon |
| `SNDSETUP.EXE` | 166 649 | 1996-10-16 | Sound-card setup; writes `HMISET.CFG` |
| `SNDSETUP.INI` | 3 129 | 1996-10-14 | Its configuration, below |
| `HMIDRV.386` | 317 317 | 1996-09-20 | [Driver archive](../../motion32/formats/driver-archive.md) — the digital drivers |
| `HMIDET.386` | 83 774 | 1996-09-20 | [Driver archive](../../motion32/formats/driver-archive.md) — card detection |
| `LOADPATS.EXE` | 49 194 | 1995-11-30 | Patch loader for wavetable cards |
| `PATCHES.INI` | 6 988 | 1995-11-30 | Its patch list |
| `TEST.HMI` | 13 125 | 1996-09-20 | Sound-setup test song — [HMI](../../motion32/formats/hmi.md) |
| `TEST.MID` | 14 571 | 1996-09-20 | The same song as standard MIDI |
| `TEST.WAV` | 80 684 | 1996-09-20 | Sound-setup test sample |
| `TEST.RAW` | 40 320 | 1996-09-20 | The same sample, headerless |
| `000.PAL` | 768 | 1996-10-01 | [Palette](../../motion32/formats/palette.md) |
| `000.FNT` | 1 082 | 1996-10-01 | [System font](../../motion32/formats/fonts.md) |
| `002.SCR` | 40 136 | 1996-09-30 | Standalone [script module](../../motion32/formats/script-modules.md) |
| `011.SCR` | 36 908 | 1996-09-25 | Standalone [script module](../../motion32/formats/script-modules.md) |
| `RSC.INF` | 249 592 | 1996-10-25 | Resource-manager configuration, below |
| `LIESMICH.TXT` | 2 434 | 1996-10-17 | German readme, below |
| `LIESMICH.DOK` | 1 975 | 1996-10-17 | The competition reply form the game's ending points at |

The five the player needs are the three containers, `ENGINE.EXE` and
`000.FRT`; `HMIMDRV.386`, `MELODIC.BNK` and `DRUM.BNK` add sound. The rest are
never opened. What each of the interesting ones is follows.

## The launcher

### DS2.BAT

The launcher sets `DOS4GVM=@_RUNVM.VMC`, runs the sound setup **only if no
configuration exists yet** (`if exist HMISET.CFG goto end`), then starts
`ENGINE`:

```bat
@echo off
SET DOS4GVM=@_RUNVM.VMC
if exist HMISET.CFG goto end
sndsetup
:end
engine
SET DOS4GVM=
```

### _RUNVM.VMC

The complete DOS/4GW virtual-memory configuration:

```
MINMEM = 8192
MAXMEM = 16384
VIRTUALSIZE = 16384
SWAPNAME = _RUNVM.SWP
DELETESWAP
```

8–16 MB of memory, a 16 MB virtual arena, swap to `_RUNVM.SWP`, deleted on
exit.

### SYSTEM.RSC

Despite the `.RSC` extension this is not a resource container but **source
text**, interpreted as Forth at startup. The shipped file is exactly two
lines:

```
4 =>GET
START
```

— load script module 4 and run its `START` word. Without a valid `START`
the engine boots into its authoring environment instead (see
[ENGINE.EXE](../../motion32/engine/engine-exe.md)).

## The sound stack

| File | Contents |
|---|---|
| `SNDSETUP.EXE` | Sound card setup utility |
| `SNDSETUP.INI` | Setup configuration — see below |
| `HMIDRV.386`, `HMIDET.386`, `HMIMDRV.386` | HMI (Human Machine Interfaces) sound and MIDI drivers — see [Driver archives](../../motion32/formats/driver-archive.md) |
| `DRUM.BNK`, `MELODIC.BNK` | [Ad Lib instrument banks](../../motion32/formats/adlib-bank.md) — the FM patches |
| `LOADPATS.EXE`, `PATCHES.INI` | Patch loader and patch list for wavetable cards |
| `TEST.HMI`, `TEST.MID`, `TEST.WAV`, `TEST.RAW` | Test files for the sound setup |

### SNDSETUP.INI

Identifies the product and publisher:

```ini
Title       =  Dunkle Schatten 2 Sound-Setup
Copyright   =  1996 - Art Department Werbeagentur GmbH
ConfigFile  =  hmiset.cfg
```

and lists the supported hardware: Sound Blaster (classic, Pro, 16, AWE32),
Ensoniq SoundScape, Microsoft Sound System, Pro Audio Spectrum 16, Gravis
UltraSound (and Max), ESS AudioDrive, Toptek Golden 16, and more, each with
device id, port, IRQ, and DMA defaults. The chosen configuration is written
to `HMISET.CFG` (whose existence is what makes `DS2.BAT` skip the setup on
later launches). The music blocks inside the containers use the
[HMI song format](../../motion32/formats/hmi.md) these drivers play.

## Documentation and metadata

| File | Contents |
|---|---|
| `LIESMICH.DOK`, `LIESMICH.TXT` | The German readme files ("Liesmich" = "read me") — see below |
| `RSC.INF` | Resource-manager configuration — see below |
| `000.PAL` | Default palette — see [Palettes](../../motion32/formats/palette.md) |
| `000.FNT` | System font — see [Fonts](../../motion32/formats/fonts.md) |
| `000.FRT` | Character-to-glyph table — see [Font reference table](../../motion32/formats/font-reference-table.md) |
| `002.SCR`, `011.SCR` | Standalone script modules — see [Script modules](../../motion32/formats/script-modules.md) |

### LIESMICH.TXT

Support notes for the original audience (Windows 95 DOS-mode advice, mouse
driver hints, sound troubleshooting, a support hotline). Two details are
technically relevant:

- **Alt-F10 is an immediate exit hotkey**:

  > "Solltest DU beim Spielstart feststellen, daß die Maus nicht reagiert,
  > kannst Du das Spiel mit der Tastenkombination Alt-F10 direkt wieder
  > verlassen. WICHTIG: verwende diesen Ausstieg NIE im laufenden Spiel."
  >
  > ("If you find at game start that the mouse does not respond, you can
  > quit the game immediately with Alt-F10. IMPORTANT: NEVER use this exit
  > during a running game.")

  The warning implies the hotkey bypasses the save/cleanup path.

- EMS must be disabled (`EMM386 … NOEMS`); the engine wants XMS-style
  memory through DOS/4GW.

### RSC.INF

A 249 592-byte binary configuration/index for the resource manager. The
header begins:

| Offset | Value | Description |
|---|---|---|
| `+0x00` | 31 | Unknown |
| `+0x04` | 4 000 000 | Cache size in bytes (presumed) |
| `+0x08` | 1 323 520 | Second cache/arena size (presumed) |
| `+0x0c` | 400 | Unknown |
| `+0x10`.. | 5000, 300, 900, 20, 900, 200 | The six slot counts of the [RSC containers](../../motion32/formats/container.md) |
| after | pointer-like values | Addresses in the engine's data segment |

The bulk of the file is a run of **22-byte records**, most carrying an
identical default pattern. The per-record layout and the mapping of
records to resource slots have not been worked out.

## Open questions

- The RSC.INF record layout.
- Which engine path reads `LOADPATS.EXE`/`PATCHES.INI` (Gravis wavetable
  patches) — the game itself or only the setup.

## See also

- [Documentation index](../../README.md)
- [ENGINE.EXE](../../motion32/engine/engine-exe.md)
- [HMI songs](../../motion32/formats/hmi.md)
