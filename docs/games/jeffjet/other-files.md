[← Documentation index](../../README.md)

# Other Shipped Files

*Jeff Jet - Abenteuer InfoHighway — this page describes the game's own installation. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Thirteen files, and motionvm opens four of them.

| File | Size | Date | What it is |
|---|---:|---|---|
| `DATA.-1-` | 1 404 960 | 1998-04-10 | Volume 1 of the container: the scripts, the texts, the music and 947 sprites |
| `DATA.-2-` | 1 098 474 | 1998-04-10 | Volume 2: 523 sprites, and every palette, both fonts and the font reference table |
| `HPPLAY.EXE` | 165 702 | 1998-04-10 | The player. motionvm reads it for the kernel word table and never runs it ([HPPLAY.EXE](../../motion16/engine/hpplay-exe.md)) |
| `MUSADL.DRV` | 4 480 | 1998-04-10 | The PSM 2 Ad Lib driver, whose tables the rebuilt player reads |
| `DETECTOR.DRV` | 2 640 | 1998-04-10 | Sound-card detection, for `SOUND.EXE` |
| `DMABLAST.DRV` | 9 840 | 1998-04-10 | Digital playback, Sound Blaster |
| `DMASB2P.DRV` | 10 208 | 1998-04-10 | Digital playback, Sound Blaster Pro 2 |
| `DMASB16M.DRV` | 10 128 | 1998-04-10 | Digital playback, SB16 mono |
| `DMASB16S.DRV` | 11 040 | 1998-04-10 | Digital playback, SB16 stereo |
| `SOUND.EXE` | 11 175 | 1998-04-10 | The sound setup: writes `PSMCFG4.DAT` |
| `HP.BAT` | 172 | 1998-04-10 | The launcher |
| `HPLOGO.EXE` | 164 704 | 1998-04-10 | The Hewlett-Packard splash picture, not MOTION |
| `PROMSOFT.EXE` | 164 704 | 1998-04-10 | The Promotion Software splash picture, not MOTION |

The date is a re-stamp. The six `*.DRV` and `SOUND.EXE` are **byte-identical**
to the copies Die Enviro-Kids greifen ein ships with a 1995-07-10 date, and
`HP.BAT` signs itself off with 1995; what carries 1998 is the distribution,
not the build.

## The launcher

```bat
@hplogo /W5 /DIS /FAD
@promsoft /W5 /FAD
@if not exist psmcfg4.dat sound
@hpplay
@echo (c) 1995 Hewlett Packard
@echo Entwickelt von Promotion Software GmbH, TM-übingen
```

Two splash pictures, then the sound setup on first run only, then the game,
then two lines of credit — with the shipped typo `TM-übingen` for *Tübingen*,
where a `ü` in the batch file has swallowed the `T`.

## The sound stack

`SOUND.EXE` asks once which card is in the machine and writes the answer to
`PSMCFG4.DAT`; `HP.BAT` then skips it forever. The player reads five `u16`
fields of that 36-byte file — music on, music port, digital on, digital port,
digital driver index — and loads `MUSADL.DRV` for the Ad Lib music or one of
the four `DMA*.DRV` for digital playback.

The game never reaches a digital sample: `PLAYSAMPLE` and `XGFXSAMPLE` are in
the kernel table and its bytecode calls them zero times, and `?SAMPLE` does not
exist in this build at all. The four `DMA*.DRV` and `DETECTOR.DRV` are shipped
and unused. motionvm reads `MUSADL.DRV` and nothing else.

## The two splash pictures

`HPLOGO.EXE` and `PROMSOFT.EXE` are the same size to the byte and share a
10.5 KB display stub with different image payloads behind it: they are Alchemy
Mindworks **Graphic Workshop 7.0** self-displaying pictures, 640×480, and the
flags `HP.BAT` passes them (`/W5` wait five seconds, `/DIS` dissolve, `/FAD`
fade) are that program's. Nothing in them is MOTION, and motionvm does not
show them — the game begins where `HPPLAY.EXE` begins.

## See also

- [The DATA container](../../motion16/formats/data-container.md) — the two volumes
- [HPPLAY.EXE](../../motion16/engine/hpplay-exe.md) — the player
- [PSM 2 music](../../motion16/formats/psm-music.md) — what `MUSADL.DRV` plays
- [Other files (Die Enviro-Kids greifen ein)](../enviro/other-files.md) — the same stack, one game earlier
