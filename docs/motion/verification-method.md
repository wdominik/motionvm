[← Documentation index](README.md)

# The verification method

*How a comparison against the original engine is made, for both generations: what is captured, with which settings, how a capture is brought to the form the comparison needs, and the two commands that make it. What has been compared, and with what result, is the other page — [Verification](verification.md).*

A check against the original's **own output** is the only kind that says the
engine behaves as the engine did; the other two kinds, against the original's
files and against motionvm's own earlier output, are on the verification
page. This page is about the first kind, and about making it repeatable: a
capture of the original is a file, our side of it is a file, and
`motionvm-motion-tools` compares the two and names the first place they part.
Neither the tool nor the capture knows anything of the engine being checked —
which is the point.

## The emulator

DOSBox-X, on a copy of the game's directory, headless where the run allows
it. The settings that matter, for Dunkle Schatten 2:

```ini
[dosbox]
machine  = svga_s3      # VESA 640x480x256 — a plain VGA machine will not do
memsize  = 32           # DOS/4GW asks for 8 to 16 MB
captures = capture      # where DX-CAPTURE and screenshots land

[render]
aspect = false
scaler = none           # nothing between the DAC and the file

[cpu]
core   = dynamic
cycles = max
```

For a register recording the sound card has to be the one the game's own
setup selects: `sbtype=sb16`, `oplmode=opl3`, `oplemu=nuked`, and an
`HMISET.CFG` naming device `0xA009` at port `0x388` — SNDSETUP's "Sound
Blaster 16" — which picks the OPL3 driver `fmmidi3.com` out of
`HMIMDRV.386`. A 16-bit game wants a `PSMCFG.DAT` with music on at `0x388`
instead; its `MUSADL.DRV` is an OPL2 driver, and its recording is one of an
OPL2.

Two things about the run itself. `-time-limit N` is a *graceful* stop: the
emulator leaves its main loop, tears the devices down, and the capture's
writer finishes the file — the games never exit on their own, so this is what
closes a recording, and killing the process instead loses the writer's buffer
and the header. And `DX-CAPTURE /O <program>` (the hyphen matters) starts a
register recording that opens at the first note and stops with the program;
`/V` records lossless video the same way.

## A frame

Our side is the picture F12 writes: the frame as the engine composed it, one
index a pixel and the palette beside it, without the window's scaling and
without a screen capture's color profile ([debugging](debugging.md)). The
original's side is DOSBox-X's own screenshot of a 256-color mode, which is
the same kind of file at the native size — or a frame out of `DX-CAPTURE
/V`, scaled back to native with nearest-neighbor sampling so that the pixels
are the DAC values themselves, or a screenshot of the window when the window
is a whole multiple of the frame and the scaler is none.

The comparison is made **on colors as the DAC held them**, six bits a
channel. Each picture's pixels are taken through its own palette to those six
bits and compared; a renumbered palette is the same picture, and a capture
that widened six bits to eight comes back to the same six whichever way it
widened. A screenshot in RGB loses its bottom two bits the same way. Indices
are not compared — a scene can hold one color under two of them — but each
side's index is reported beside the color where the file had one, which is
what makes a difference readable.

```sh
motionvm-motion-tools compare-frame shot.png capture.png
motionvm-motion-tools compare-frame shot.png screenshot.png --crop 1920,960,1280,960 --scale 2
```

The report is one of two things: the pictures agree on every color, or how
many pixels differ, the box they fall in, the first of them with both sides'
index and color, and how many of them show a color our palette does not hold
at all — a shift and a wrong picture look different in that line. The
command exits 1 on a difference.

What a match means: the composed picture is the original's, pixel for pixel,
in that scene at that moment. What it does not: the moment has to be one that
stands still on both sides — a title screen, a scene held with F12 — since a
frame a few ticks off shows another line of a caption or another step of an
animation, which reads as a displacement and is none.

## A register stream

Our side is rendered offline out of the game's own files:

```sh
motionvm-motion-tools registers /path/to/gamedata 25 --ticks 6000
```

plays block 25 through the sequencer and the OPL driver the player runs —
the block straight out of the container, the driver's switch-on at tick 0,
the song from tick 1 — and writes every register write as `tick register
value`, one a line. No engine is involved: a song is started by number, the
way `STARTTUNE` starts one, and everything after that is the audio layer's.

The original's side is the DRO version 2 file `DX-CAPTURE /O` writes. Three
things about it a reader has to get right. The two delay codes are in the
header, after the code map — not `0x00` and `0x01`, whatever third-party
descriptions say. The file opens at the first note with a dump of every
register the chip holds, in register order: a state, not a sequence. And
from there DOSBox-X records only the writes that *change* a register; the
original's five identical release writes per note appear once.

So `compare-dro` reduces both sides to changes from a chip that starts at
zero, sets the recording's snapshot apart at the first register it names
twice, lines the two streams up on the write the recording resumes with —
choosing, among the places we make that write, the one that agrees longest,
so a write matching by accident does not decide the comparison — holds the
snapshot against our state at that point register for register, and then
compares the streams write for write:

```sh
motionvm-motion-tools compare-dro registers-025.txt capture.dro
```

```text
recording: 17073 writes over 32.8 s on OPL3
ours:      58345 writes, 19998 of them changes
the recording's snapshot at its first note holds 26 registers; it resumes with our write 31
the chip's state at that note agrees, register for register
the streams agree for all 17047 writes the recording holds past its snapshot
```

A divergence is named at its position with the writes around it on both
sides, and the command exits 1. Two things to know when it does. A recording
that runs past the song — into a room change, say — parts where the game
intervened, at the fade `ENDTUNE` asks for or the next tune's first write;
that is the recording ending, not the driver differing, and the position says
so. And the comparison is of order and values, not of time: the recording's
timestamps are read but not held against the ticks, and the one thing they
would settle — that a 32-bit song runs at 115.45 Hz rather than the 120 its
header names — is written down under the [FM driver](motion32/engine/fm-driver.md).

## What the method does not reach

The mixer's output: DOSBox resamples and mixes its own way, and a WAV of its
OPL emulation is good for catching a stream that runs away from itself, not
for a sample-exact comparison. Interaction, walking, dialogue and savegames
have no capture to be held against at all, and the verification page's
[last section](verification.md#what-has-not-been-compared) keeps the honest
edge of that.

## See also

- [Verification](verification.md) — what has been held against the original, and the result
- [motionvm-motion-tools](tools.md) — the commands, with the rest of the tool
- [Debugging and diagnostics](debugging.md) — F12 and the other keys the window keeps
- [The FM driver](motion32/engine/fm-driver.md) — the 32-bit rebuild the recording is held against
- [PSM music](motion16/formats/psm-music.md) — the 16-bit one
