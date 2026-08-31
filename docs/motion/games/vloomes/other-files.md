[← Documentation index](../../README.md)

# Other Shipped Files

*Victor Loomes – Das Spiel — this page describes the game's own installation. The engine it runs on is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Eleven files, 1 194 035 bytes, and motionvm opens three of them.

| File | Size | Date | What it is |
|---|---:|---|---|
| `DATA.-1-` | 1 009 597 | 1993-05-21 | The whole game, on one volume — [the earlier framing](../../motion16/formats/container.md#the-earlier-framing) of the container |
| `LL.EXE` | 123 222 | 1993-05-20 | The player — [read, not run](../../motion16/engine/ll-exe.md) |
| `GFX.INF` | 4 800 | 1993-05-21 | The sprite-dimension side table (below) |
| `MUSADL.DRV` | 3 915 | 1992-06-10 | The PSM 2 Ad Lib driver, in an older build than the later games' |
| `DETECTOR.DRV` | 2 165 | 1992-06-09 | Sound-card detection, for the sound setup |
| `PSMCFG.EXE` | 7 720 | 1992-09-24 | The sound setup, which writes `PSMCFG.DAT` |
| `LBS.BAT` | 38 | 1993-05-13 | The launcher |
| `LQ.EXE` | 4 774 | 1993-05-13 | A memory-preparation stub |
| `LP.EXE` | 13 882 | 1992-03-18 | A third-party graphics driver |
| `LC.EXE` | 22 726 | 1993-05-12 | Plays the logo jingle |
| `LOGOMSFX.CMF` | 1 196 | 1993-05-12 | The jingle it plays |

The dates are coherent and not a re-stamp: `LP.EXE` from 1992-03 is the oldest
of the eleven, the sound drivers carry 1992-06, the sound setup
1992-09, and the game's own files 1993-05-12 to 1993-05-21 — `DATA.-1-` and
`GFX.INF` on the last of them, the mastering date. No file here carries a date
after May 1993, which makes this the oldest MOTION artifact motionvm plays.

`PSMCFG.DAT` is not in this copy. It is written by `PSMCFG.EXE` on the player's
machine and was removed when the corpus was cleaned of run artifacts. It is
**32 bytes** here where the later games' `PSMCFG4.DAT` is 36, and the layout is
read out of the player rather than carried over: `LL.EXE` opens the file,
reads `0x20` bytes at `091e:0093`, and branches on the words at `+0x1A`,
`+0x16` and `+0x18` — the first three of the five fields the later file has.
motionvm never reads it, having no sound card to configure.

## The launcher

`LBS.BAT` is thirty-eight bytes and names four programs:

```bat
echo off
lq
lp >nul
lc
ll
echo on
```

- **`LQ.EXE`** is a Borland C++ memory-preparation stub: the DOS calls `30h`,
  `4Ah`, `67h` and `48h` between `0x208` and `0x2fd`, and no strings beyond the
  runtime's own.
- **`LP.EXE`** is not Promotion Software's. It is PKLITE-packed and signs
  itself `GFX-DRIVER V01.00 - (C) 1991 by LINEL` (offsets `0x2a64`, `0x2a78`,
  `0x2ec1`), with an author fragment `Arndt H…` beside it. The batch file
  silences it with `>nul`.
- **`LC.EXE`** plays the logo jingle: it carries the name `LOGOMSFX.CMF` in
  fragments at `0x6f7` and nothing else of interest.
- **`LL.EXE`** is the game.

**The game runs without the first three.** Started directly under DOSBox-X,
`LL.EXE` plays from its intro into location 1 on its own — which is how the FM
capture the rebuilt sequencer is held against was taken
([PSM 2 music](../../motion16/formats/psm-music.md)). Copies circulate with a
`TROUBLE.BAT` that is this batch file with `lq` commented out, so the chain
tolerates losing at least one more of them.
Started that way it plays its whole intro — the eight pictures it holds are
recorded from exactly such a run and match the rebuilt game's to the pixel
([verification](../../verification.md)) — and its music comes out of
`MUSADL.DRV` unchanged. So `LP.EXE` is not a precondition for the player
starting, drawing or sounding. What it changes past the intro is unread, and
motionvm runs none of the chain in any case.

## The sound stack

Three files, and no digital output at all. `PSMCFG.EXE` — LZEXE-packed, `LZ91` at
`0x1c` — is the older PSM generation's setup, and it writes the 32-byte
`PSMCFG.DAT` the player reads. `DETECTOR.DRV` is version 12 with a smaller card
table than the later games' copy, 2165 bytes against 2640; motionvm never runs
it.

`MUSADL.DRV` is the one file motionvm opens besides the container and the
player. It is an **older build** than the byte-identical 4480-byte copy the
three later games all ship: 3915 bytes, fourteen driver entries rather than
fifteen, and the four data tables `Driver::parse` reads sit `0xd0` earlier —
`0x4f4`, `0x5f4`, `0x5fd` and `0x606`. Their contents are the same to the byte,
which is what makes reading them at the later build's offsets a silent wrong
answer rather than a loud one, so the entry count is what selects the profile.
See [PSM 2 music](../../motion16/formats/psm-music.md).

**No `DMA*.DRV` ships here**, and nothing would use one: no block in the
container carries an `SM8` sample section, and the sample words the later
builds' kernels have — `PLAYSAMPLE`, `XGFXSAMPLE`, `?SAMPLE` — do not exist in
this one's table at all. Where the later games ship four digital drivers and
never reach them, this game has no digital path to leave unused.

## The sprite-dimension table

4800 bytes, and exactly `1200 × (u16 width, u16 height)` — one entry per GFX
slot, `FFFF FFFF` for an empty one. It is the file a player needs when its
sprites are packed and it unpacks them on demand: the dimensions are the first
four bytes of the *unpacked* item, so without this table a size costs a
decompression.

Only the two games with the earlier container framing ship it, both at 4800
bytes. `LL.EXE` names it at file `0x14e86`, and so do all three later builds —
which no longer need it, because their sprites are stored plainly and the
dimensions are the item's own first four bytes.

It agrees with the container in both directions: 721 present entries against
721 occupied sprite slots, no disagreement either way, and every entry equal to
the decoded sprite's own header. motionvm **validates it rather than reading
it** — the container unpacks every item as it opens, so a pre-decode size table
has no consumer, and a second source of truth that nothing consults is worse
than none ([departures](../../departures.md#the-16-bit-machine)). The reader
exists for the tools and the tests.

## The logo jingle

**Not a Creative CMF.** The magic is `A.H.` where a Creative Music File has
`CTMF`, and the embedded credits read *DimosQuest / composed by / T.Detert /
in 1993 / an x.a.p production / … eclipse s.d / soundfx* — an OPL asset from
LINEL's *Dimo's Quest*, the same house `LP.EXE` comes from. It is `LC.EXE`'s
jingle and has nothing to do with the MOTION music stack: the game's own
fourteen tunes are PSM 2 blocks inside the container, and this file is never
opened by `LL.EXE`.

## See also

- [Resource inventory](inventory.md) — what the one volume holds
- [`LL.EXE`](../../motion16/engine/ll-exe.md) — the player this game's kernel table is read out of
- [The DATA container](../../motion16/formats/container.md) — the earlier framing, and where `GFX.INF` fits it
- [PSM 2 music](../../motion16/formats/psm-music.md) — the older driver and the bare `PLX` songs
- [Other shipped files (Hilfe für Amajambere)](../hfa/other-files.md) — the later sound stack, with its four digital drivers
