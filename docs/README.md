# Technical Documentation

The reverse-engineering record, one tree per engine family. Each family's
index fixes that family's vocabulary and conventions once, and every page
below it names the game or the generation it speaks for.

| Family | Tree | What it covers |
|---|---|---|
| MOTION (DigiTales, 1996) | [motion/](motion/README.md) | Both generations of the engine, the four 16-bit builds, the five games, their formats, and the family's ledgers — departures, open questions, verification, savegames, the inspection CLI |

Everything here describes the *original* engines and their shipped files,
without reference to the code in this repository; `ARCHITECTURE.md` at the
root is where the code describes itself. One page stands outside the trees:
[Writing an engine family](writing-a-family.md), the contract read from the
engine's side, for whoever brings a further family.
