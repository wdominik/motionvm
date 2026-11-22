[← Documentation index](../README.md)

# Music

*PARSEC — Dunkle Schatten (1994): which piece the program plays where, and when it fades and stops it. Addresses are linear offsets into the unpacked `XP01.EXE` (150 096 bytes) unless a sentence names `XP00.EXE`. The driver itself is a component and is documented under [PSM 2](../../psm/README.md).*

The music is PSM 2, played by `MUSADL.DRV`, the Ad Lib driver. The program
does not interpret a song: it loads one, hands it to the driver's entries
and decides when. What follows is that deciding.

## The driver and the library

The program installs `MUSADL.DRV` when the sound setup's `PSMCFG4.DAT`
names an Ad Lib card (`0x5B7B`, which sets `[20Ch]`), and a digital driver
beside it when the file names a DAC (`0x5BEC`, `[210h]`). The Ad Lib
driver is not a loose file: it is resource 5 of `RESOURCE\PSMDRV.DAT`
(`DS:0A06h`), itself a RIB holding the one file
([audio](../formats/audio.md)). Each program brings its own copy: the
title program's driver goes with it, and `XP01`'s init loads and installs
its own (`0x5B7B`).

A song is loaded by `1197h:000Dh(n)` (`0x1197D`). Its loader takes, for a
card without a DAC, the bare `PLX` section of module `n` out of
`PSMP2L.DAT` (`0x1435A`), and the far pointer to that block is what the
program keeps and passes on; the logic only ever tests it against null.
The sound effects (`0590h:091Ch`) need the digital driver, `[210h]`, and
play nothing without one.

## The four calls

The library of segment `0590h` is what the handlers and the visit call;
each ends in the driver's own entries — `SetSong` with `Play` or
`PlayFade`, `FadeOut`, `Stop`:

| Routine | What it does |
|---|---|
| `0590h:06A9h(song, loops)` (`0x5FA9`) | `[214h]` := loops; `[216h:218h]` := the song when it loops endlessly, else null; with the switch `[212h]` on, `SetSong` and `Play` |
| `0590h:07A5h(song, loops, ms)` (`0x60A5`) | the same with `PlayFade(ms, loops)` (`0x619D`) |
| `0590h:08A9h(ms)` (`0x61A9`) | with the switch on, `FadeOut(ms)`; `[216h:218h]` := null either way (`0x61EC`) |
| `0590h:08FAh` (`0x61FA`) | `Stop`, whatever the switch; the endless song forgotten |

`[212h]` is the music switch. The game options toggle it (`0x12304`) and
tell the driver: off stops the kept song, on starts it again
([options and saving](options.md)).

## A location's song

Every room slot below 30 has a song in the word table at `DS:13E8h`, −1
for none; `1197h:0035h(slot)` (`0x119A5`) loads it. The room loader puts
the slot's song into `[285Dh]` (`0x7814`) — the Zimmer's is module 1
(`13E8h[2]`), the Küche's module 10 (`13E8h[3]`).

- **On entry**, once the entry effect has played, the room's song is
  started endlessly (`0x0ECA9`–`0x0ECB7`).
- **On leaving**, it is faded over 1 500 ms (`0x0ED39`); the transition
  plays, and since the effect is synchronous the stop (`0x0ED55`) comes a
  transition later — about a second into the fade, so what still sounds
  dies away from the level the fade has reached.
- **As the slot is unloaded**, a loaded room song is stopped and its block
  freed (`0x85F3`), which after a leaving finds nothing playing.

## Between two forest rows

The forest rows — the Wald and the maze — all show slot 20, and between
two of them the room stays loaded ([rooms](../formats/rooms.md)). The
music follows the room:

- The leaving skips the fade and the transition (`0x0ED37`), and the entry
  clears its song (`0x0EC74`): nothing is played.
- The slot unload asks the kept room's object map first: with bit 3 of its
  flags set (`0x8416`, `0x85ED`), nothing is stopped and `[285Dh]` holds.
- A forest row's map keeps the word as it finds it, and with bit 3 clear
  the row's own song would take its place (`0x8339`–`0x834F`).

Every object map Dunkle Schatten ships — 31 of them — carries the flags
`3Fh`, so bit 3 is always set: the forest plays the picture's song from
the first forest row it is entered at, and plays on from row to row
([object maps](../formats/object-map.md)). Entering the maze from a place
of another kind plays it afresh.

## The title program

`XP00` plays module 0 from its menu on, endlessly (XP00 `0x3854`); a line
chosen fades it over 2 000 ms (XP00 `0x3AA2`) and the menu's fade-out
ends with a stop (XP00 `0x3B07`). See
[boot and the title program](boot.md#the-title-programs-music).

## Open questions

What is not yet known about where the leaving's stop falls is recorded in
the [open questions](../open-questions.md).

## See also

- [PSM 2](../../psm/README.md) — the format and the driver's entries
- [Audio](../formats/audio.md) — `PSMP2L.DAT` and `PSMDRV.DAT`
- [Game loop](game-loop.md) — the visit, the leaving and the synchronous effects
- [Object maps](../formats/object-map.md) — the flags a forest row's map carries
