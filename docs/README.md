# Technical Documentation

The reverse-engineering record, one tree per engine family. Each family's
index fixes that family's vocabulary and conventions once, and every page
below it names the game or the generation it speaks for.

| Family | Tree | What it covers |
|---|---|---|
| MOTION (DigiTales, 1996) | [motion/](motion/README.md) | Both generations of the engine, the two 32-bit and five 16-bit builds, the seven games, their formats, and the family's ledgers — departures, open questions, verification, savegames, the inspection CLI |

Beside the families stand the components — a piece the games ship that is
no family's engine, rebuilt once and used by every family whose games play
through it. A component whose files are read has a tree of its own; the
OPL3 is the chip those files write to, emulated rather than read out of
anything the games ship, and has none:

| Component | Tree | What it covers |
|---|---|---|
| PSM 2 | [psm/](psm/README.md) | The PSM 2 music format, the Ad Lib driver `MUSADL.DRV` in the two builds that are read, and the sample path of the `DMA*.DRV` digital drivers |

Everything here describes the *original* engines and their shipped files,
without reference to the code in this repository; `ARCHITECTURE.md` at the
root is where the code describes itself. One page stands outside the trees:
[Writing an engine family](writing-a-family.md), the contract read from the
engine's side, for whoever brings a further family.
