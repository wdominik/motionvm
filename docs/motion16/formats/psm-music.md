[← Documentation index](../../README.md)

# PSM 2 Music

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway and in `BMZ.EXE` with Hilfe für Amajambere, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit-ds2).*

Ten blocks of the [DATA container](data-container.md) are music: PSM 2
modules, played on an OPL2 by `MUSADL.DRV`. The driver file is the whole
player — sequencer, fade engine and register back-end in 4480 bytes behind
a `MUS\0` header — and `ENVIRO.EXE` keeps only a loader, a call table and
a timer host around it. File offsets below are `MUSADL.DRV`'s unless
marked otherwise; they are also its code-segment offsets, because the
installer normalizes the loaded image to a paragraph boundary (`ENVIRO.EXE`
`198c:0193`).

## The module

Sixteen bytes of tag, `MTCVTS PSM 2.00\0`, then a table of up to ten
`u32` section offsets at +0x10, ascending, ending at the first zero. The
loader (`1696:017e`) converts them to far pointers in place and hands the
driver the first section — the `PLX\0` song, at offsets 767 to 1640 in
the shipped modules. Every later section is an `SM8\0` sample for the
digital-sample driver the game can install beside the Ad Lib one, and an
`MDH\0` block sits between table and sections at +0x38; the FM path reads
neither.

## The song — the `PLX` section

```
char tag[4]     ; "PLX\0" — the driver compares it (0xe3b, literal 0xe37)
u8   speed      ; ticks to a row
u16  tempo      ; the timer period, in PIT cycles (1 193 182 to a second)
u16  track[9]   ; one stream per OPL channel, PLX-relative; 0 = no stream
```

Across the ten shipped tunes the speed runs 10 to 34 and the tempo 0x15d6
to 0x1734 — 199 to 215 ticks a second, three to twenty rows a second.

A track is a run of events. Each is one flags byte, its operands in bit
order, and a closing delay byte — rows until the channel's next event, 0
chaining several events into one row (the tick drains a channel until its
next event lies ahead, `0xadf`):

| Flags | Operand | What happens (`0x890`) |
|---|---|---|
| `0x00` | — | The stream ends: the channel falls idle and its key goes off. No delay byte follows |
| `0x80` alone | — | Nothing but the delay byte |
| bit 0 | `u16` | Instrument: the operand is a PLX-relative record offset (below) |
| bit 1 | `u8` | Volume: a raw carrier-level byte, kept as loudness, not written |
| bit 2 | — | Key off — the `B0` write happens only where the key is on |
| bit 3 | `u8` | Note: twice the semitone, a byte offset into the driver's note table |
| bit 4 | `u16` | A raw `A0`/`B0` register pair (only read when bit 3 is clear) |
| bit 5 | — | Key on: OR 0x20 into the pair — alone, it retriggers the held pair |
| `0x40` as the rest | `u16` | Tempo: a new period, re-clamped and re-registered |
| anything else left | — | The channel is killed (`0xa6f`); no shipped stream carries such a byte |

With bits 3, 4 or 5 the driver builds one sixteen-bit pair — `A0` low,
`B0` high — stores it, and writes both registers. The key bit is the
flag's **or the register shadow's** (`0x9a5`): a note without bit 5 keeps
a key that is already down sounding, and the stored pair carries the key
bit in its high byte.

An instrument record is twelve bytes at its offset. The driver skips the
first and writes ten of the remaining eleven straight to the chip — `C0+ch`;
`0x20/0x40/0x60/0x80/0xE0` + modulator; `0x20` + carrier; `0x60/0x80/0xE0`
+ carrier. The eighth is not written: it becomes the channel's loudness
word, `((b & 0x3F) ^ 0x3F) | (b & 0xC0) << 8` — level inverted into
loudness, the KSL bits kept beside it — and a volume event's byte goes
through the same transform onto the same word (`0x960`).

The note table (`0x6d6`) is 96 words of `block << 10 | fnum`, C-0 to B-7:
twelve F-numbers 343 363 385 408 432 458 485 514 544 577 611 647, the
block the octave.

## The driver

`MUSADL.DRV` opens with `MUS\0`, version 0x0100, fifteen entry offsets at
+0x18, seven service-pointer cells the installer fills at +0x36, and an
`NS` trailer at the offset +0x0a names. The manager (`198c:0193`) verifies
exactly that, builds fifteen five-byte far-jump stubs, and calls entry 0 —
which flushes the install state to the chip: one default per register
(`0x5c4`), `0xFF` marking the untouched ones, written in ascending
register order.

Every write goes through that 256-byte shadow (`0x7df`/`0x7ec`): a value
the shadow already holds is not sent again, so the register stream carries
**changes only** — across tunes too, because the shadow lives as long as
the driver.

The game calls four entries. `STARTTUNE ( loop n -- )` (`ENVIRO.EXE` file
`0xbf89`) loads block `n` and runs SetSong (`0x2f9`) — tag check, pointer
stored — then Play (`0x470`): notes stopped if something still plays, the
loop count stored, the volume full, the streams primed, the host timer
registered with the period. The loop count is compared unsigned against a
pass counter that starts at 1, so the `-1` every one of the game's sixteen
call sites passes means forever. `ENDTUNE` (`1696:02fd`) is FadeOut
(`0x3e2`) with 2000 ms, a 500 ms wait, then Stop (`0x4c9`).

**The tick** (`0xa71`), on the host timer at the period: every `speed`th
tick is a row. Channels run 8 down to 0, each draining its due events; a
channel that dies under them no longer counts. When no channel is live, or
the sixteen-bit row counter wraps, the song is over — passes exhausted
stops it, otherwise the streams re-prime and the fresh row 0 plays **in
the same tick** (`0xb63`): the loop is gapless.

**Volume.** After every row (`0xc5e`), each live channel's carrier level
register is set from its loudness and its channel master (0x100 = full,
the file's value for all nine): full master passes the loudness through,
anything lower scales it byte-wise; the result is inverted back
(`^ 0x3F`) with the KSL bits on top. A fade (`0xb77`) instead runs **every
tick**: a Q8.8 level moves by `±0x10000 / ticks` for the asked-for
milliseconds, each live carrier follows `master × level × loudness` with
every product truncated to a byte, reaching zero closes all carriers
(`0x3F` + KSL), and reaching 0x100 ends the fade with a full pass.

**Stop** (`0xde2`): channel 8 down to 0 — key off where a key is on, then
the release rate of both operators opened to `0x0F`, so what still sounds
dies away; the timer is unregistered.

The two scale entries — tempo (`0x282`) and pitch (`0x2ba`) — stand at
their neutral 0x100 in the file and the game never calls them: the tempo
word **is** the period (clamped no lower than 0x200), and notes sound
exactly as the table says.

## How the rebuild is checked

Two ways, as with [the 32-bit FM driver](../../motion32/engine/fm-driver.md).
The tables — defaults, operator offsets, notes — are not transcribed but
read out of the shipped `MUSADL.DRV` at the addresses above, on every run;
that check works on any copy of the game. Above it, the whole register
stream has been held write for write against an OPL capture of the
original playing from game start into location 1 — tune 8, the scripted
room change's `ENDTUNE` with its fade and stop, then tune 1 — and the two
streams are identical to the capture's end, 2 738 writes. That needs a
recording of the original, so it is a result reported rather than
something a reader can re-run.

## Open questions

- The `SM8` sample sections are the digital renderer's voices — with the
  configuration set to digital only, the tunes play sampled
  ([other files](../../games/enviro/other-files.md#the-sound-stack)) —
  but the `DMA*.DRV` drivers' internals are unread, and motionvm plays
  the Ad Lib rendition only.
- The driver's callback protocol (`0x27a`) and the Volume entry's negative
  selectors: present, but the game leaves the callback on its stub and
  never calls the entry.

## See also

- [Blocks](blocks.md) — where the modules live, and the other block families
- [The DATA container](data-container.md) — the BLK segment
- [Boot and frame loop](../engine/boot-and-loop.md) — the clock the same driver file feeds
- [Other files](../../games/enviro/other-files.md) — `MUSADL.DRV` among the shipped drivers
- [The FM driver (MOTION 32-bit)](../../motion32/engine/fm-driver.md) — the other game's music path
