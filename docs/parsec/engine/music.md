[← Documentation index](../README.md)

# Music

*PARSEC — Dunkle Schatten (1994): which piece the program plays where, which sound effect when, and when it fades and stops them. Addresses are linear offsets into the unpacked `XP01.EXE` (150 096 bytes) unless a sentence names `XP00.EXE`. The drivers themselves are a component and are documented under [PSM 2](../../psm/README.md).*

The sound is PSM 2, played by two drivers out of `RESOURCE\PSMDRV.DAT`
(`DS:0A06h`), each a RIB holding one file ([audio](../formats/audio.md)):
`MUSADL.DRV`, the Ad Lib driver, on the OPL, and `DMABLAST.DRV`, the
digital driver, whose software mixer plays the sound effects and a song's
digital voices on the DAC. The program does not interpret a song or a
sample: it loads one, hands it to the drivers' entries and decides when.
What follows is that deciding.

## The drivers and the library

`XP01`'s init installs the two (`0590h:000Ah`):

- `MUSADL.DRV`, resource 5 (`0x5B7B`), which sets `[20Ch]`;
- `DMABLAST.DRV`, resource 1 (`0x5BB3`, installed at `0x5BEC`) with **six
  effect channels** (`push 6`, `0x5BD3`), which sets `[210h]` (`0x5C1B`).
  It plays at its file's time constant `D0h`, a byte every 48 µs — 20 833
  bytes a second, mono, 8-bit. Its default volume is set to `50h` right
  after the install (entry 22, `0x5C15`) and to **`3Ch`** at the start of
  `_main` (`0x4E85`); every effect plays at `3Ch`.

The title program installs its own copies the same way, the digital driver
with three channels (XP00 `0x9A2`) and the volume at `50h` (XP00
`0x9E4`); its drivers go with it, and `XP01` installs its own.

A song is loaded by `1197h:000Dh(n)` (`0x1197D`). With the digital driver
installed, its loader assembles module `n` out of `PSMP2L.DAT` (`0x14186`):
the module's head, the song's `PLX` behind it and the song's samples behind
that, the head's eight sample words turned into their offsets. The far
pointer to that block is what the program keeps and passes on; the logic
only ever tests it against null. The play routine hands the module's
digital part — the `MDH` chunk at `+38h` and the sample words at `+14h` —
to the digital driver's entry 21 (`0x6080`, `0x6180`), and the `PLX` at the
head's `+10h` to the Ad Lib driver's `SetSong`.

## The four calls

The library of segment `0590h` is what the handlers and the visit call;
each ends in the drivers' own entries — `SetSong` with `Play` or
`PlayFade`, `FadeOut`, `Stop` of the Ad Lib driver, play, `StopAll` and the
song's digital part of the digital one:

| Routine | What it does |
|---|---|
| `0590h:06A9h(song, loops)` (`0x5FA9`) | `[214h]` := loops; `[216h:218h]` := the song when it loops endlessly, else null; with the switch `[212h]` on, `SetSong` and `Play` |
| `0590h:07A5h(song, loops, ms)` (`0x60A5`) | the same with `PlayFade(ms, loops)` (`0x619D`) |
| `0590h:08A9h(ms)` (`0x61A9`) | with the switch on, `FadeOut(ms)`; `[216h:218h]` := null either way (`0x61EC`) |
| `0590h:08FAh` (`0x61FA`) | `Stop`, whatever the switch, then entry 21 with nothing (`0x6207`): the digital voices let go; the endless song forgotten |
| `0590h:091Ch(sample, 0)` (`0x621C`) | with the switch on and the digital driver, entry 17: the effect played |
| `0590h:0948h` (`0x6248`) | with the switch on and the digital driver, entry 18 (`StopAll`): every effect and every digital voice cut |

`[212h]` is the music switch. The game options toggle it (`0x12304`) and
tell the drivers: off stops the kept song and lets its digital voices go
(`0x12321`, `0x12338`), on starts it again ([options and
saving](options.md)). The switch silences the effects as well.

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
- **As the slot is unloaded**, after the effects' `StopAll`, a loaded room
  song is stopped and its block freed (`0x85F3`), which after a leaving
  finds nothing playing.

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

## A song's digital voices

The digital part plays beside the FM part, in the same mixer as the
effects: five to eight voices a song, each a sample of the song played
from its start whenever the voice's event comes, at one of four levels,
with no pitch and no note-off. `MUSADL.DRV` steps the voices once a row
through its callback cell, in the tick before the row; a fade scales them
through the digital driver's volume curve as it scales the FM channels;
`Stop` cuts them. The driver side is the component's
([PSM 2 drivers](../../psm/drivers.md)).

## The sound effects

`0EB9h:1949h(k)` (`0x104D9`) plays the effect in slot `k` of the 21 far
pointers at `DS:1EF4h` through `0590h:091Ch` — only with the switch on,
the digital driver installed and the slot filled (`0x621F`–`0x6233`).
Slot `k` holds the row of the library's table A that the byte at
`2140h:0034h + k` names — row `k` for every slot in the shipped image —
fetched as one sample (`13ECh:0210h`, held to the table's bound of 21).

- **Slots 0 to 11** are fetched at the start (`1458h:01C1h`, `0x14741`,
  11 down to 0) and stay.
- **Slots 12 to 20** are the room's. The room loader fetches the ones the
  room slot's mask at `2140h:0000h + 2·slot` names, bit `k − 12` for slot
  `k`, into slots still empty (`1458h:00F7h`, `0x14677`, called at
  `0x781C`; again for a forest row whose map has bit 2 clear, `0x8361`).
  The Zimmer's mask names 12 and 13, the Wohnzimmer's 14 and 19, the
  Straße's and the Bogenstraße's 18, the Laden's 20, the forest picture's
  15 to 17.
- **As the slot is unloaded**, the digital driver's `StopAll` cuts what
  plays and slots 12 to 20 are freed (`0x85DA`, `0x85DF`) — unless the
  kept room's map has bit 2 set (`0x85D4`): between two forest rows every
  shipped map has, so the effects and the voices play on.

Who asks for which:

| Effect | Asked for by |
|---|---|
| 0, 1 | the figure's and a mover's footsteps (the walk's phases 1 and 4) |
| 2 | a turn of the figure (`0x0B619`, `0x0BB39`) |
| 3, 4 | an animation's `17` opcode with code 8 (`0x10961`) and 3 (`0x10975`) |
| 5 to 11, 18 | a dialogue's `03 n` control through the hook (`0x10691`): 50 → 5, 51 → 6, 52 → 7, 55 → 8, 56 → 10, 57 → 9, 59 → 11, 60 to 62 → 18 |
| 8 | the hook's events 105 (with the figure linked, `[16BBh]` not −1), 106 and 107 (`0x107A6`–`0x10807`) |
| 12 | the seventh verb's rule on an object that is not a door (`0x15C0D`) |
| 13 | the eighth verb's rule on an object that is not a door (`0x15CF6`) |
| others | the location logic of the Wohnzimmer, the Laden, the Schuppen and the maze |

### When an effect is heard

The mixer mixes 128 bytes a block into one half of a two-block buffer
while the other plays; the interrupt at a block's end starts the next,
already mixed, and mixes the one after. A play into a running mixer is
therefore heard from the first byte of the block after the next — 128 to
256 bytes, 6.1 to 12.3 ms, later — and a play into a standing mixer from
the call, which mixes its first block and starts it at once. `StopAll`
ends the output with the block playing; the voices of a song that plays
on stay silent until an effect or a song's start runs the mixer again.

## The title program

`XP00` plays module 0 from its menu on, endlessly (XP00 `0x3854`), its
digital part with it; a line chosen fades it over 2 000 ms (XP00
`0x3AA2`) and the menu's fade-out ends with a stop (XP00 `0x3B07`) and the
digital driver's `StopAll` (XP00 `0x3B0C`). The program plays no effect. See
[boot and the title program](boot.md#the-title-programs-music).

## Open questions

What is not yet known about where the leaving's stop falls is recorded in
the [open questions](../open-questions.md).

## See also

- [PSM 2](../../psm/README.md) — the format and the drivers' entries
- [Audio](../formats/audio.md) — `PSMP2L.DAT` and `PSMDRV.DAT`
- [Game loop](game-loop.md) — the visit, the leaving and the synchronous effects
- [Object maps](../formats/object-map.md) — the flags a forest row's map carries
