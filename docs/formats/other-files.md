[← Documentation index](../README.md)

# Other Shipped Files

Everything in the game directory that is not an RSC container or one of the
formats with its own page.

## Boot and engine

| File | Contents |
|---|---|
| `DS2.BAT` | Launcher — see below |
| `ENGINE.EXE` | The MOTION engine — see [ENGINE.EXE](../engine/engine-exe.md) |
| `DOS4GW.EXE` | The DOS/4GW 32-bit DOS extender the engine runs on |
| `_RUNVM.VMC` | DOS/4GW virtual-memory configuration — see below |
| `SYSTEM.RSC` | Plain-text Forth bootstrap — see below |
| `DS2.ICO` | Program icon |

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
[ENGINE.EXE](../engine/engine-exe.md)).

## Sound

| File | Contents |
|---|---|
| `SNDSETUP.EXE` | Sound card setup utility |
| `SNDSETUP.INI` | Setup configuration — see below |
| `HMIDRV.386`, `HMIDET.386`, `HMIMDRV.386` | HMI (Human Machine Interfaces) sound and MIDI drivers — see [Driver archives](driver-archive.md) |
| `DRUM.BNK`, `MELODIC.BNK` | [Ad Lib instrument banks](adlib-bank.md) — the FM patches |
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
[HMI song format](hmi.md) these drivers play.

## Documentation and metadata

| File | Contents |
|---|---|
| `LIESMICH.DOK`, `LIESMICH.TXT` | The German readme files ("Liesmich" = "read me") — see below |
| `RSC.INF` | Resource-manager configuration — see below |
| `000.PAL` | Default palette — see [Palettes](palette.md) |
| `000.FNT` | System font — see [Fonts](fonts.md) |
| `000.FRT` | Character-to-glyph table — see [Font reference table](font-reference-table.md) |
| `002.SCR`, `011.SCR` | Standalone script modules — see [Script modules](script-modules.md) |

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

A 249,592-byte binary configuration/index for the resource manager. The
header begins:

| Offset | Value | Reading |
|---|---|---|
| `+0x00` | 31 | Unknown |
| `+0x04` | 4,000,000 | Cache size in bytes (presumed) |
| `+0x08` | 1,323,520 | Second cache/arena size (presumed) |
| `+0x0c` | 400 | Unknown |
| `+0x10`.. | 5000, 300, 900, 20, 900, 200 | The six slot counts of the [RSC containers](rsc-container.md) |
| after | pointer-like values | Addresses in the engine's data segment |

The bulk of the file is a run of **22-byte records**, most carrying an
identical default pattern. The per-record layout and the mapping of
records to resource slots have not been worked out.

## Open questions

- The RSC.INF record layout.
- Which engine path reads `LOADPATS.EXE`/`PATCHES.INI` (Gravis wavetable
  patches) — the game itself or only the setup.

## See also

- [Documentation index](../README.md)
- [ENGINE.EXE](../engine/engine-exe.md)
- [HMI songs](hmi.md)
