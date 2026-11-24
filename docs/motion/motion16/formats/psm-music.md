[← Documentation index](../../README.md)

# PSM 2 Music

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere, in `STERN.EXE` with Falsches Spiel mit Eddie M. and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

Ten blocks of the [DATA container](container.md) are music: PSM 2
modules, played on an OPL2 by `MUSADL.DRV`. The driver file is the whole
player, and `ENVIRO.EXE` keeps only a loader, a call table and a timer host
around it. The format and the driver files are a third party's program, not
the engine, and are documented as a component under
[PSM 2](../../../psm/README.md): [the format](../../../psm/format.md) and
[the drivers](../../../psm/drivers.md). This page is what the engine does
with them, and what was measured on this family's games. File offsets below are `MUSADL.DRV`'s unless marked otherwise;
they are also its code-segment offsets, because the installer normalizes
the loaded image to a paragraph boundary (`ENVIRO.EXE` `198c:0193`).

## The module

Sixteen bytes of tag, `MTCVTS PSM 2.00\0`, then a table of up to ten
`u32` section offsets at +0x10, ascending, ending at the first zero. The
loader (`1696:017e`) converts them to far pointers in place and hands the
driver the first section — the `PLX\0` song, at offsets 767 to 1640 in
the shipped modules. Every later section is an `SM8\0` sample for the
digital-sample driver the game can install beside the Ad Lib one, and an
`MDH\0` block sits between table and sections at +0x38; the FM path reads
neither.

**The earliest game has no module at all.** All fourteen of Victor Loomes'
tunes are a bare `PLX` section stored as the block itself — no tag, no section
table, no samples, which fits a game that ships no digital drivers and has no
sample word in its kernel. Both forms reach the same reader, which sniffs the
tag: a block that opens on `MTCVTS` has its section 0 cut out first, a block
that opens on `PLX\0` is the section.

## The song — the `PLX` section

The section's layout, its events and its instrument records are the
format's ([the song](../../../psm/format.md#the-song--the-plx-section)).
Across the ten shipped tunes the speed runs 10 to 34 and the tempo 0x15d6
to 0x1734 — 199 to 215 ticks a second, three to twenty rows a second — and
no shipped stream carries a flags byte the driver would kill its channel
for.

## The driver

What `MUSADL.DRV` does inside — its header, its tables, the register
shadow, the tick, the fade and the stop — is [the drivers
page](../../../psm/drivers.md#musadldrv)'s. Two builds ship: the 1995/96
games all carry the same 4480-byte file with fifteen entries, and Victor
Loomes carries the 3915-byte build with **fourteen**, whose tables sit
`0xd0` earlier; the entry count is what says where to read. The manager
(`198c:0193`) verifies exactly that, builds fifteen five-byte far-jump
stubs, and calls entry 0. The rebuild checks the header the way the
manager's header check does (`198c:0078`): the `MUS\0` tag, version 1.00,
the entry and import counts, and the `NS` trailer at the offset the header
names.

The game calls four entries. `STARTTUNE ( loop n -- )` (`ENVIRO.EXE` file
`0xbf89`) loads block `n` and runs SetSong (`0x2f9`) — tag check, pointer
stored — then Play (`0x470`): notes stopped if something still plays, the
loop count stored, the volume full, the streams primed, the host timer
registered with the period. The loop count is compared unsigned against a
pass counter that starts at 1, so the `-1` every one of the game's sixteen
call sites passes means forever. `ENDTUNE` (`1696:02fd`) is FadeOut
(`0x3e2`) with 2000 ms, a 500 ms wait, then Stop (`0x4c9`) — and the wait
is the script's: the stop routine resets the tick counter and spins until
the 200 Hz reading (`110a:042e`, the driver's millisecond count over five)
comes back 100, and only then stops the driver and returns. It does nothing
at all while its own flag (`ds:18f4`) says no song is playing, and Play
(`1696:02ce`) runs the whole stop routine first when one is — which is how
Victor Loomes changes its music, with no `ENDTUNE` between locations: the
new tune begins half a second after the old one's fade started. The other
three builds' routines are the same shape (`HPPLAY.EXE` `1639:02c0`,
`BMZ.EXE` `166d:02c2`, `LL.EXE` `0e87:01f2`). The flag is the host's, not
the driver's: SetSong's success sets it (`STERN.EXE` `1611:0020`) and only
the two stop routines clear it (`1614:0015`, `161a:0026`), so a tune that
ends on its own — a loop count of 0, as `SET_POINTS`'s jingle passes —
leaves it set, and the room's next `STARTTUNE`, `ENDTUNE` or `PLAYSAMPLE`
pays the half-second wait over silence. motionvm's flag lives the same way.

The two scale entries stand at their neutral 0x100 in the file, and the
game never calls them.

## The sample — an `SM8` block

Falsches Spiel mit Eddie M. ships thirteen blocks that are a sample on their
own — blocks 1, 9, 10, 12–15, 17, 18 and 20–23, 2305 to 31 960 bytes, the
sound effects `PLAYSAMPLE` names by block number — and the header is the
section header the modules carry ([the sample](../../../psm/format.md#the-sample--an-sm8-block)):
version `0x0100` in every one, the length the block's less ten, and a period
of 56 (21.3 kHz) to 179 (6.7 kHz) PIT cycles.
How the digital driver turns the period into the DSP's clock, and the two
paths its play entry can take, are [the drivers
page](../../../psm/drivers.md#the-sample-path-of-the-dma-drivers)'s.

**Which path plays.** `STERN.EXE` installs the driver with one channel — the
sixth install argument is what the sound manager's trampoline leaves in that
slot, the caller's `DI`, which is 1 at the call (`1058:01bd`) — so every
sample the game plays is the direct path: once, on the DSP's clock, at full
scale, no volume. The host's `PLAYSAMPLE` runs the driver's StopAll (entry
18) before every play, so one sample sounds at a time whatever the channel
count. What motionvm makes of it is a
[departure](../../departures.md#the-16-bit-machine). Nothing in shipped play
reaches the mixer, so none of it is rebuilt.

## How the rebuild is checked

Two ways, as with [the 32-bit FM driver](../../motion32/engine/fm-driver.md).
The tables — defaults, operator offsets, notes — are not transcribed but
read out of the shipped `MUSADL.DRV` at the addresses [the drivers
page](../../../psm/drivers.md#musadldrv) gives, on every run;
that check works on any copy of the game. Above it, the whole register
stream has been held write for write against an OPL capture of the
original playing from game start into location 1 — tune 8, the scripted
room change's `ENDTUNE` with its fade and stop, then tune 1 — and the two
streams are identical to the capture's end, 2 738 writes. That needs a
recording of the original, so it is a result reported rather than
something a reader can re-run.

The same driver two years earlier is held against a recording of its own:
Falsches Spiel mit Eddie M.'s intro runs `-1 24 STARTTUNE` under a poll loop
that waits for a key, and a capture of the original standing there holds the
tune for 54.8 seconds — 3 640 writes, of which the 3 551 past the recording's
opening snapshot agree with the rebuilt stream write for write, and each
register the snapshot holds at the first note has the rebuilt stream's value
— compared in that one direction only, so a register the rebuilt stream sets
and the snapshot leaves at zero is not covered. That is the fifth
build's `MUSADL.DRV`, byte-identical to the 1995/96 games', playing a module
whose `PLX` section sits 1688 bytes in.

The sample path is held against a recording of the same original's rendered
audio — DOSBox-X with an Ad Lib and a Sound Blaster on, `PSMCFG4.DAT` naming
`DMABLAST.DRV` — through the opening scene's effect, block 17: 31 275 bytes at
a period of 149. Aligned at its onset, the recording peaks where the rebuild
does, the DAC at full scale; it lasts 3.88 seconds in both; the two waveforms
correlate at 0.98 over the whole effect; and where the effect sounds its RMS
is 2 to 10 % under the rebuild's, the emulator's resampler interpolating
across the DAC's steps that the rebuild holds. The clock came out of this
comparison: with the period read as PIT cycles the rebuild fell 2.3 ms behind
the recording over 2.3 seconds, and on the DSP's constant it stays within two
frames of it, first byte to last. The table itself needs no recording — the suite reads
the 256 bytes at `0x60` out of every `DMA*.DRV` the game ships and holds the
formula to them, on any copy of the game.

The older driver is held against a recording of its own, and against a
different question. Victor Loomes' intro runs `0 7 STARTTUNE ANIMPLAY
ENDTUNE` — block 7, a 552-byte jingle six seconds long, started with a loop
count of zero — and the two streams agree for 744 writes: the whole tune, first
note to last, and the start of the fade behind it. Since the two builds' tables
are byte-identical, that is the check which says the v14 profile reads the
right bytes rather than plausible ones. What the recording holds past the fade
is not covered ([open questions](../../open-questions.md#motion-16-bit)).

## Open questions

- The `SM8` sample sections are the digital renderer's voices — with the
  configuration set to digital only, the tunes play sampled
  ([other files](../../games/enviro/other-files.md#the-sound-stack)) —
  and motionvm plays the Ad Lib rendition only. What of the drivers is
  unread is [the component's](../../../psm/drivers.md#open-questions); the
  game leaves the driver's callback on its stub and never calls its Volume
  entry.

## See also

- [Blocks](blocks.md) — where the modules live, and the other block families
- [The DATA container](container.md) — the BLK segment
- [Boot and frame loop](../engine/game-loop.md) — the clock the same driver file feeds
- [Other files](../../games/enviro/other-files.md) — `MUSADL.DRV` among the shipped drivers
- [The FM driver (MOTION 32-bit)](../../motion32/engine/fm-driver.md) — the other generation's music path
- [PSM 2](../../../psm/README.md) — the format and the driver files, as a component
