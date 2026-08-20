[← Documentation index](../README.md)

# The Game Loop

The game's frame cycle is a collaboration between the engine's native
loop and a script word registered as a callback. The shape is unusual
and worth stating up front:

1. The bootstrap installs the script word `ICTRL` (module 4) as the
   engine's **control callback** via the kernel word `CTRL` — whose
   entire handler is seven instructions: it stores the address at
   `0xdb4a8`. Exactly one word reads that cell: `ANIMPLAY`.
2. It then calls `1 2 3 4 5 6 7 8 9 10 ANIMPLAY` — a blocking word —
   and **the entire game runs inside that one call**: `ANIMPLAY` sets a
   running flag (`0xdb4a4`), and while it stands, invokes the control
   callback once per frame, plus registered idle callbacks (a four-slot
   table that also runs the sample collector, see [Audio](audio.md)).
   `QUITANIM` clears the flag — that is how the game ends. (`ANIMPLAY`
   pops none of the ten values `START` pushes for it; their meaning is
   open.)

So the "native loop" is the engine's frame pump inside the blocking
words, and the game-side loop body is `ICTRL` — an ordinary Forth word,
fully described under [Shell](../library/shell.md).

## The frame rate: 25 frames per second

`DELAY ( n -- )` does not wait — it sets the frame length: its handler
(`0x7065d`) computes `0xdb4b8 = 200 / n` (−1 passes through as "no
limit"). The bootstrap runs `25 DELAY`, so **the game runs at 25 frames
per second**, each frame 8 ticks of the engine's 200 Hz tick base long.

### Where the 200 Hz comes from, and where it stops

Two independent places agree on it. `DELAY` writes `200/n` into
`0xdb4b8`, and the frame limiter inside `ANIMPLAY` (`0x690e0`) polls a
**unit-3 timer** against that very cell — the same `GIVETIMER` spin the
curtain uses:

```
000690e9  call 0x000236FE        ; elapsed, in unit-3 ticks
000690f1  cmp  0xDB4B8,%eax
000690f7  jb   0x000690E9
```

Dividing a constant by `n` yields a period only if the constant is a
rate, so one unit-3 tick is 1/200 s **by the engine's own arithmetic**,
and the frame rate and the fades rest on exactly the same reading. Every
timer the engine opens is unit 3 — all fifteen call sites of `0x235AE`
are preceded by `mov $3,%eax`.

What cannot be closed from `ENGINE.EXE` is the **wall clock** — but it
can be cornered. The raw counter lives behind a pointer at `0xE7F38`,
filled in exactly once: `0x2358F` calls the loader-patched thunk
`0xA28F8` **with no arguments** and stores what comes back. No rate is
requested there, so the rate is entirely the provider's. The engine does
carry its own PIT programmer (`0x95F49`: `out 0x43, 0x36` then a 16-bit
divisor to `0x40`) and an IRQ0 installer (`0x95F8F`, handler `0x8D3DB`,
hooked via DPMI `int 31h 0204/0205`), and `sosTIMERInitSystem`
(`0x8CD7D`) would arm both and compute the divisor as `1193180 / rate` —
but the sound startup at `0x84fc0` calls it with **rate 0, flags 1**,
and flag bit 0 skips the whole install block, so the gate `0xDF330`
stays 0 and **no `out` to port 0x40/0x43 is ever executed by the game
binary**. Nor by the drivers: the only PIT writes in all three
`HMI*.386` files address channel 2, the PC speaker (`HMIMDRV.386`
`0x1cc2c`, `HMIDRV.386` `0x32ab3/0x32acf`). Whoever feeds the counter,
its **intended** rate is written into the engine twice over:
`GIVETIMER`'s conversion for unit 3 is `(raw − stamp) × 10 / 51`
(`0x23761`), which is `/5.1` — exact only at a **1020 Hz** master, where
unit 3 comes out at the same **200 Hz** that `DELAY`'s `200/n` assumes.
Units 1 and 2 (`/20`, `/10`) then land at 51 and 102 Hz. A real PIT can
only approximate that (divisor 1170 → 1019.81 Hz), but 1020/200 is the
author's arithmetic.

Measured against the original running under DOSBox-X, the model holds
where it should and the emulator shows its seams where it must — the
measurement is the engine's own timers read back out of a running game,
not a reading of the code. Driving 930 curtain bands
through `FADEOUT`/`FADEIN` while an `OPENTIMER` unit-3 timer watches:
the engine's own count divided by the captured wall time gives a master
of **~978 Hz** — the design rate, minus emulator scheduling loss — and
loading the HMI drivers (SB16 + MPU-401) changes neither count nor wall
time (R 2920 vs 2922, 15.23 s vs 15.24 s), so the digital path's
1500.86 Hz service master (see [Audio](audio.md)) is a *different* rate
riding the same hardware, not this counter's. What DOSBox does distort
is **delivery**: the ticks arrive in bursts aligned to the emulated
70 Hz refresh, and the burst size moves with the `cycles` setting (the
same 930 bands take 15.2 s at `cycles=max` and 22.7 s at
`cycles=10000`). A wait of 1 tick therefore stretches to a whole burst
(~14–16 ms) under emulation, which is an artifact of the emulator's
interrupt batching, not of the engine — on period hardware the interrupt
fired evenly and a 1-tick wait cost the quantized 6 raw ticks
(≈5.9 ms). See [Transitions](transitions.md#timing) for what that does
to the fades, which is where the difference is actually visible.

## A frame is the unit of script time

The task system counts time in **control-callback invocations**.
`!LTWAIT` decrements the wait counter `_LOCTASKWAI` by one per
invocation; a task "waiting 50" waits 50 frames = 2 seconds. Nothing in
the scripting interface exposes a wall-clock timer to this mechanism
(the audio words have their own timers — see [Audio](audio.md)).

## What one frame does

`ICTRL`, per invocation:

1. Samples input: keyboard (`?KEY` → `_AKTKEY`), mouse position split
   into picture-area and status-bar coordinates, buttons
   (`MOUSELK`/`MOUSERK` → `_MLK`/`_MRK`), with `_MPRESSED` as a
   one-frame click debounce — input handling is edge-triggered by
   construction.
2. Runs UI-mode-specific input handling (free play, menu, save/load,
   document viewer, options, quit, ending sequences).
3. In free play: hit-tests the scene (`SCANITEM` → the native
   `MOUSEINFO`), copies the input snapshot into the `_ORDER` record,
   and dispatches the player's order through the native `DOORDER` — the
   entry into the [interaction machine](interaction.md), which calls
   back into the script verb handlers (see
   [Game library](../library/game-library.md)).
4. Housekeeping: enter `_STARTLOC` if no location is active yet (this
   is how the title screen appears); run `GLOBTASK` (global tasks) and
   the location's `_LTHANDLER`; **consume `_NEXTLOC`** (location
   changes are requested by storing a number there and performed here);
   run `_ANHANDLER` (the animation driver).

After the controller returns, the frame continues natively:

5. The **descriptor walk** (`0x68c64`) counts down every visible
   descriptor's wait and fires expired callbacks — the mechanism behind
   self-hiding captions (see [Descriptors](descriptors.md)).
6. The **drawer** (`0x6915b`) renders the frame. It runs here, once per
   frame — and once more inside `FADEIN` — and nowhere else; see
   [Screens](screens.md) for the consequences.

While a blocking word (`FADEOUT`, `FADEIN`, `ANIMPLAY`, `ANIMSIM`) is
mid-execution, the callback is not re-entered — which is why a task
phase that fades and swaps scenery is atomic from the scripts' point of
view. Native handlers *within* a frame do, however, re-enter the
interpreter freely (see [Execution model](../vm/execution-model.md)).

## The location task machine

Per-location scripted behavior runs as a phase machine on module-2
variables:

| Variable | Role |
|---|---|
| `_LOCTASK` | Current task number (< 1000 location, > 1000 global) |
| `_LOCTASKPHA` | Current phase within the task |
| `_LOCTASKWAI` | Wait counter, decremented by `!LTWAIT` |
| `_LTHANDLER` | Packed address of the location's handler word |

The words operating on it (`SETLOCTASK`, `?LTWAIT`, `!LTWAIT`,
`NEXTLTP`, `FINISH_LT`, …) are in the
[game library](../library/game-library.md). For the title sequence the
handler is `LTMANAGER` in module 223 at byte offset `0x19c8`; eighteen
locations define such a handler (see [Module map](module-map.md)).

**The wait mechanism is never armed.** Across all modules,
`_LOCTASKWAI` is only ever set to zero, read, and decremented — never
set positive. Pauses in the original game's pacing are loading time,
not script intent.

## Open questions

- The meaning of the ten values `START` pushes before `ANIMPLAY`.
- The exact wall-clock frequency of the 200 Hz tick base (inference).

## See also

- [Shell](../library/shell.md) — ICTRL in full detail
- [Interaction machine](interaction.md) — DOORDER and friends
- [Game structure](game-structure.md) — locations and their handlers
- [Execution model](../vm/execution-model.md) — blocking words
- [Audio](audio.md) — the idle-callback table and timers
- [Departures](../departures.md) — which of these rates is rendered, and why
