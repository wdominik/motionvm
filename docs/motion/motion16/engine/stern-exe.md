[← Documentation index](../../README.md)

# STERN.EXE — The Second Build of the MOTION 16-bit Player

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere, in `STERN.EXE` with Falsches Spiel mit Eddie M. and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

`STERN.EXE` (167 334 bytes, dated 1994-10-06) is the same player as
[`ENVIRO.EXE`](enviro-exe.md), built two years earlier — later than
[`LL.EXE`](ll-exe.md) and earlier than [`HPPLAY.EXE`](hpplay-exe.md), which
puts it second of the five. Everything the `ENVIRO.EXE` page says about the
binary holds here — Turbo C real mode, an 800-paragraph header, two kernel
tables of far function pointers, a data stack behind a far pointer in DGROUP —
and this page is only what differs. Where a subsystem page cites an
`ENVIRO.EXE` address, the same code is in this image at its own address; the
five are not interchangeable.

The name is the client's magazine: the game was made for Gruner + Jahr's
*Stern*.

## The binary

| Field | `STERN.EXE` | `LL.EXE` | `HPPLAY.EXE` | `ENVIRO.EXE` |
|---|---|---|---|---|
| Size | 167 334 | 123 222 | 165 702 | 167 430 |
| Header | `0x3200`, load image 154 534 bytes | `0x1e00`, 115 542 | `0x3200`, 152 902 | `0x3200`, 154 630 |
| Relocations | 3123 | 1853 | 3157 | 3160 |
| Entry | `CS:IP = 0000:0000`, `SS:SP = 25ac:00e6` | `SS:SP = 1c27:00e6` | `SS:SP = 2546:00e6` | `SS:SP = 25b2:00e6` |
| Data segment | `0x1bcb` | `0x1271` | `0x1bce` | `0x1c24` |
| Turbo C banner | file `0x1eeb4` | file `0x14514` | file `0x1eee4` | file `0x1f444` |
| Memory asked for | 1 MB EMS (2.5 MB at most), 555 KByte base | — | 2 MB EMS, 590 000 base | 1 MB EMS, 550 000 base |

The dates are the build's own and not a re-stamp: 1994-10-06 for the binary,
1994-10-10 and 1994-10-13 for the three volumes, November for the boot-disk
tooling and the readme. Its prompts are formal *Sie* (*"Bitte Spieldiskette
einlegen!"*, file `0x1fe7b`), as `HPPLAY.EXE`'s and `BMZ.EXE`'s are; only the
oldest and the latest build say *Du*. It carries the fifteen interpreter error
messages `LL.EXE` and `BMZ.EXE` carry and `ENVIRO.EXE` strips, from file
`0x200d4` — *"Line #i: Unterminated Comment!"* at `0x20181`, *"Line #i:
Illegale Opcode ( #i )"* at `0x2028c`, *"Line #i: Fehler diverser Natur
(FDN)"* at `0x202c8`.

## Where this build sits between the other four

Its kernel is **`LL.EXE`'s core table under `HPPLAY.EXE`'s domain table.** The
core table has eighty words, name for name and order for order the oldest
build's, and stops at `_PutLit` where the three later builds append
`_PutStringAdr` and `$->`. The domain table has 146 words, name for name Jeff
Jet's build's: it has `GDNR` and `SDNORM` inside the table and the walk, order,
inventory and sound words appended, none of which `LL.EXE` has, and it lacks
the `SETMOUSE*` four and `?SAMPLE`, which `BMZ.EXE` and `ENVIRO.EXE` add.

| Build | Domain words | Core words | Total | Domain base |
|---|---:|---:|---:|---:|
| `LL.EXE` | 124 | 80 | 204 | **102** |
| `STERN.EXE` | 146 | 80 | 226 | **102** |
| `HPPLAY.EXE` | 146 | 82 | 228 | 105 |
| `BMZ.EXE` | 150 | 82 | 232 | 105 |
| `ENVIRO.EXE` | 151 | 82 | 233 | 105 |

That is the order they were built in, by the argument the other pages make: a
word is appended to a live ordinal space, not inserted into the middle of one.
`GDNR` sits inside the domain table here and not in `LL.EXE`, so this build is
later; `_PutStringAdr` is appended to the core table in `HPPLAY.EXE` and not
here, so that build is later still. The domain table was finished before the
core table grew.

**The base is `LL.EXE`'s, not the later builds'.** Eighty core words and 21
`DUMMY#F0R3i` placeholders (the name at file `0x20728`) put the first domain
word at `80 + 21 + 1 = 102`, so every domain word sits three ordinals below its
namesake in `HPPLAY.EXE` and, from 124 up, seven below `ENVIRO.EXE`'s: `DOWALK`
is 236 here, 239 and 243 there; `PLAYSAMPLE` closes the table at 247. A table
baked from any other build binds this game's bytecode without complaint and
names every word of it wrongly from `TOGFX` on. The base is read out of the
binary's registration loops, as for every build.

## What lives where

| File offset | What |
|---|---|
| `0x1f4fa` | Kernel table 2 — 146 domain words, ordinals 102–247 ([kernel words](../vm/kernel-words.md)) |
| `0x202f0` | Kernel table 1 — 80 core words, ordinals 1–80, `LL.EXE`'s name for name |
| `0x20728` | `DUMMY#F0R3i`, the placeholder name the 21-word run registers |
| `0x200d4` | The fifteen interpreter error messages |
| `0x1f40e`, `0x1f43a` | `data.-1-` and the `DATA.-#i-` name former: the loader opens further volumes, and this game has three |
| `0x1fe36` | `gfx.inf`, named and not shipped — the game's sprites are packed, and the loader unpacks them itself |
| `0x1ff4d`, `0x209ac` | `psmcfg4.dat` and `musadl.drv`: the 1995/96 sound stack, two years early |
| `0x1674a` | `RANDOM`'s handler (`1251:103a`), the generator the [kernel words](../vm/kernel-words.md#random) page reads in `ENVIRO.EXE` |
| `0x19399` | `PLAYSAMPLE` (`15e5:0349`) — the one build whose game calls it; read, see below |
| `0x13709` | `GIVEDATE` (`0cd3:37d9`) — DOS function 2Ah, pushed `dl`, `dh`, `cx`: day, month, year |
| `0x9894` | `REMSCR` (`05e1:0884`), which pops its screen handle — the word whose stack effect this game's `CTRL` checks on |

## What the game asks of it

Falsches Spiel mit Eddie M.'s bytecode uses **153** of the 226 words, and two of
them no other game calls. `GIVEDATE` is called once, in location 1's scene,
to compute the current issue number of the magazine. `PLAYSAMPLE` is called
thirty-four times: for sound effects in the rooms, with a mode of 0, and — under
a variable `MAC` that no module ever stores to — in place of `STARTTUNE` and
`ENDTUNE`, for the digital rendition of the two songs.

**`PLAYSAMPLE ( block mode -- )`**, read at `15e5:0349`. Both cells are popped;
the mode is never read again. If the driver's flag says a tune is playing
(`ds:1af8`, the flag `ENDTUNE`'s stop routine at `1614:0003` tests), the
handler runs that routine without its first step: no `FadeOut(2000)`, but the
tick counter reset, a spin until it reads 100 (`cmp $0x64` — half a second at
200 Hz), the flag cleared, the music driver's Stop entry called, and the
digital driver's two stop calls where one is installed. Then, only if a digital
driver is installed (`ds:108c`) and the configuration's digital flag was read
as 1 (`ds:7770` set to `0x71` at `1058:00ca`, from `PSMCFG4.DAT`'s word at
`+0x1a`), it spells the block number into the `#F0R3i.blk` template, loads the
item, copies it into the driver's buffer at `ds:9700` and hands it to the
driver's play entry — the manager's stub `1933:0271`, entry 17 of the driver,
with a mode of 0 whatever the script passed. Before that it has run the
driver's StopAll (`1933:0276`, entry 18) and cleared the digital music module
(`1933:0285`, entry 21, four zeroes). The manager's stub tables are at
`1933:012c` for the fifteen entries of `MUSADL.DRV` and `1933:021c` for the
twenty-five of the digital driver, five bytes a stub; the sound setup
(`1058:0000`) loads the digital driver named by `PSMCFG4.DAT`'s index when its
digital flag is 1, installs it at the configured port with IRQ 7 and DMA 1 —
and, through the trampoline, with the caller's `DI` as the channel count, which
is 1. `ENVIRO.EXE`'s handler (`1696:0357`) is the first half alone. What
motionvm makes of the two halves is a
[departure](../../departures.md#the-16-bit-machine); the driver's own path is
on the [PSM 2 music](../formats/psm-music.md#the-sample--an-sm8-block) page.

**`GIVEDATE ( -- day month year )`**, read at `0cd3:37d9`: `ah` = `0x2a`,
`int 21h` through the Turbo C `int86` at `1ba8:0004`, and `dl`, `dh` and `cx`
pushed in that order — the day of the month, the month, the year in full.

## Open questions

- **The handlers not read.** Read only where it differs from `ENVIRO.EXE` —
  the tables, the two words above, `REMSCR`, the sound module's stop and play
  routines — so the descriptor drawer, the fades and the rest of the sound
  interface stand on that binary's reading
  ([ENVIRO.EXE](enviro-exe.md#open-questions)).

## See also

- [ENVIRO.EXE](enviro-exe.md) — the latest build, and everything the five share
- [LL.EXE](ll-exe.md) — the oldest build, whose core table and domain base this one keeps
- [HPPLAY.EXE](hpplay-exe.md) — the next build, whose domain table this one already has
- [BMZ.EXE](bmz-exe.md) — the fourth build
- [Kernel words](../vm/kernel-words.md) — the two tables, entry by entry
- [Other files (Falsches Spiel mit Eddie M.)](../../games/eddiem/other-files.md) — what else the installation holds
