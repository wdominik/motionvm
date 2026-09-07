[← Documentation index](README.md)

# Verification

*What motionvm has been held against, for both generations of the engine and every game it plays. Which comparison covers which game is named in each entry. Deliberate divergences are a different record and live in [departures](departures.md).*

There are three kinds of check in this project, and the distinctions matter
enough to be written down rather than implied.

A check against the original's **own output** — a screen capture, a register
dump, a mixer capture — says the engine *behaves* as the engine did. A check
against the original's **files** — its resources, its bytecode, its driver
binaries — says only that the readers agree with the data; that is the weaker
claim, and the one that covers most of what is here. A check against
**motionvm's own earlier output** says neither, and says the thing the other
two cannot: that nothing has moved since.

How a check of the first kind is made — the emulator's settings, what a
capture is brought to, and the two commands that compare a frame and a
register stream — is on [The verification method](verification-method.md).

## Held against the original's own output

### Dunkle Schatten 2, on the 32-bit engine

- **One rendered frame**, pixel for pixel — the title screen, 307 200 pixels
  over 206 palette indices, held against the original running under DOSBox-X.
  Every index maps to one color and every color back to one index, which is a
  stronger check than comparing RGB ([screens](motion32/engine/screens.md)).
- **Fourteen VM primitives**, against modules the 1996 compiler produced.
- **A song's first 8 000 OPL register writes**, and one mixer envelope
  ([FM driver](motion32/engine/fm-driver.md)).

### Checker 2000, on the earlier 32-bit build

- **Two rendered boards, pixel for pixel**, against lossless recordings of
  the original under DOSBox-X: the registration board as it stands waiting
  for a name, and the main menu after a name and a postcode were typed —
  307 200 pixels each, 0 differ, the pointer included. The pointer is what
  the comparison was made for: the engine's own arrow that `TOGFX` installs,
  in the two indices it resolves against the system palette and keeps
  through every `SETPAL` after, drawn at the corner where the emulator's
  mouse driver parks it
  ([interaction](motion32/engine/interaction.md#the-engines-own-arrow)).
- **Two pages of the information book, pixel for pixel**, against the same
  kind of recording: *Check it!* clicked on the main menu, then the down
  arrow — the white page `FADEIN` mode 2 paints, its frame, the text on it,
  the pointer on the text. 0 of 307 200 differ on each. The recording was
  also the one that timed this build's curtains: 70 frames a second, the
  fade-out gone within one frame, the fade-in open within three
  ([transitions](motion32/engine/transitions.md#timing)).
- **The speech, by the frame**, against the same recording's sound track,
  the lines found in it by their loudness envelopes: the classroom's first
  file begins six game frames after the scene's curtain opens, the scene
  steps on seven frames after the file's last sample, and the schoolyard's
  second file begins on the frame the first ran dry — three intervals the
  scripts derive from `?STIME`, and all three the engine's. The recording's
  seconds are not the game's: under DOSBox-X a frame of `8 DELAY` runs 134
  ms, not 125 — the slide show's sixteen-frame slides come 2.15 s apart —
  which is the emulator's timer batching and is why the intervals are
  counted in frames ([audio](motion32/engine/audio.md#the-speech-system)).
  The same sound track settles the DSP's rate: the speech in it holds the
  6–10 kHz that a 22 050 Hz output passes and an 11 025 Hz one would not,
  which is the rate the layer's init asks for
  ([the digital mixer](motion32/engine/audio.md#the-digital-mixer)).
- **One rendered frame of a story scene**, the beach of location 12, held
  against a capture of the original from the game's own archive: 307 200
  pixels, of which 3 % differ, every one of them inside the two captions and
  the information button the scene shows at that moment and under the
  pointer — the capture was taken between two lines of dialogue. The picture
  underneath, background and both figures, is the same index for index
  ([screens](motion32/engine/screens.md)). The office of location 4 is
  captured scrolled, and the scroll has not been matched yet.
- **The *Futurespiel* menu**, driven headless and digested; no capture
  holds it yet.

### Die Enviro-Kids greifen ein, on the 16-bit engine

Against lossless recordings of the original under DOSBox-X:

- **The intro's title scene**, 99.9 % of sampled pixels identical to a
  frame-rate video capture.
- **A page of the help viewer**, RGB-identical to the pixel — 0 of 51 200
  differ — reached through the game's own menu.
- **The music's whole register stream**: 2 738 OPL writes across two tunes and
  the room change between them, identical to the capture's end
  ([PSM music](motion16/formats/psm-music.md#how-the-rebuild-is-checked)).
- **The box fades**, ring for ring — same widths on all four sides, same
  cadence — and the text drawer's outline, against a played capture.

### Victor Loomes – Das Spiel, on the oldest 16-bit build

- **The whole intro**, picture for picture: the eight pictures it holds — the
  client's logo, the title card, *featuring Victor Loomes* over the office
  vignette, and five more — each RGB-identical to a lossless capture of the
  original, 0 of 64 000 pixels differing in any of them, and held in the same
  order. Neither side is searched for a match: the original plays its intro on
  a timer with no input, and both are reduced by the same rule — a picture is
  one the intro *holds* when it stands unchanged for forty steps. This is what
  covers the earlier container framing, whose unpacking, sprite and font
  decoders, palette, opaque block path and text metrics all stand behind those
  eight pictures.
- **The intro's whole jingle**: 744 OPL register writes, first note to last,
  plus the start of the fade behind it, identical to a capture of the original.
  The older `MUSADL.DRV` build carries byte-identical tables at an offset
  `0xd0` earlier, which is what makes this check worth running — a reader at
  the wrong offsets comes back with plausible values rather than none
  ([PSM music](motion16/formats/psm-music.md#how-the-rebuild-is-checked)).

### Falsches Spiel mit Eddie M., on the second 16-bit build

- **The intro's whole tune**, block 24, against an OPL capture of the original
  standing in its intro under DOSBox-X — the title animation over, the tune
  looping under the poll loop that waits for a key: 54.8 seconds, 3 640
  register writes in the recording, of which the 3 551 past the recording's
  opening snapshot agree with the rebuilt stream write for write, and the
  chip's 89 registers at the first note agree register for register. The
  stream is rendered from the game's own files by `motionvm-motion-tools
  registers`, so this is the check that says the fifth build's `MUSADL.DRV`
  reading and the `MTCVTS` module's section table are right for a game two
  years older than the one the player was read from
  ([PSM music](motion16/formats/psm-music.md#how-the-rebuild-is-checked)).
- **The opening scene's sound effect**, block 17, against a recording of the
  original's rendered audio under DOSBox-X with an Ad Lib and a Sound Blaster
  configured: aligned at its onset, the sample peaks where the rebuild does —
  the DAC at full scale — lasts 3.88 seconds in both, correlates with the
  rebuild at 0.98 over the whole effect, and where it sounds sits 2 to 10 %
  under it in RMS, the emulator interpolating across the DAC's steps the
  rebuild holds. The clock came out of this comparison: with the period read
  as PIT cycles the rebuild fell 2.3 ms behind the recording over 2.3
  seconds, and on the DSP's time constant — the period in whole microseconds,
  as the driver's table has it — it stays within two frames, first byte to
  last
  ([PSM music](motion16/formats/psm-music.md#the-sample--an-sm8-block)).

Those comparisons need recordings of the original engine, and a recording of a
game is no more redistributable than the game. None ships here, so the checks
that consume them are not part of the test suite. What ships is their result,
stated above.

## Held against itself

A third kind, weaker than either but the only one that runs on every machine
and on every change. For every scene the test suites compose and every tune
they play, the suite keeps a **digest** — an FNV-1a 64 of the composed indexed
frame, or of the register stream in the order the chip would have seen it — and
holds each run against the one before. Eighty-three of them, across all
seven games: each game's intro, the room it starts in, four of Dunkle
Schatten 2's densest scenes, Checker 2000's registration in two states, its
main menu and a page of its information book, and every tune the six older
games ship.

A digest says nothing about whether a picture is *right*. What it says is that
nothing moved, which is what the comparisons above cannot say twice: a capture
of the original is consulted once, by hand, on one machine, and a rebuild that
matched it in 2025 has nothing holding it there. The digests are that. They may
live in the repository for the same reason the captures may not — sixteen hex
digits reconstruct no artwork and are not a fixture derived from a game's data.

They rest on the engine being deterministic, which is engineered rather than
hoped for: no clock is read anywhere, `RANDOM` is a seeded generator, no
`HashMap` sits on a drawing path, and the resource directory is walked in
sorted order.

## Held against the original's files

Everything else. Each game is held against its own material: every item in its
containers re-parsed, every script module disassembled with no unknown ordinal,
every location it has entered and drawn, every tune it ships played, and its
savegames written and read back in the engine's own layout. The 32-bit FM
driver and the 16-bit PSM player read their tables out of the shipped driver
binaries on every run rather than carrying transcriptions, so that part of the
check works on any copy of a game.

The suite reaches wider than the games do. It also reads Dunkle Schatten 2's
`TEST.HMI` and `TEST.MID` — one song decoded two ways and cross-checked —
walks `HMIDRV.386` and `HMIDET.386` to their last byte, and reads the loose
`000.PAL` and `011.SCR` for their size, range, number and words. The games
never touch those files; they are evidence about the formats all the same.

Two more passes over the files hold the readers and the engine to them
without comparing anything to the original. Every kernel word a game's
modules reach for is checked to be one the interpreter or the engine
implements — for the five 16-bit games that is every word, for the two
32-bit games every word but a named handful the suite carries with its
reason.
And the shipped files are handed to the readers damaged — cut short at every
length through their headers, single bytes flipped where a seeded generator
says — with the assertion that a reader answers rather than crashes; what a
mutation cannot say is whether the answer is right, which is the digests'
business over the undamaged files.

## What has not been compared

Interaction, dialogue, walking, savegames, the verb menu and most locations of
all seven games have never been differentially compared against a recording —
Checker 2000's story scenes past the schoolyard and its twenty mini-games
among them, its speech only as far as its three first files' frames, and its
sound effects — the samples that sound beside the voice, and the two the
beach and the schoolyard loop under a passage — read out of the sound
layer's slot walk and the mixer's end check and not yet heard against a
recording — and
**nothing of Jeff Jet or Hilfe für Amajambere has been**. Victor Loomes is
covered as far as its intro reaches and no further, and Falsches Spiel mit
Eddie M. as far as its intro's tune: what a played room looks like there is as
unchecked as it is for the other two, and so is the cut `PLAYSAMPLE` makes in
a playing tune, read out of the binary and not yet heard: the recording that
holds the sample it plays after had stopped the intro's tune by key before the
effect, and in shipped play only the score jingle can be under an effect — a
scored action followed within two seconds by a sound — which no recording has
caught.

That is not a gap being hidden. It is the honest edge of what a
reimplementation can claim without the original running beside it, and it is
why the lists above are kept apart.

## See also

- [The verification method](verification-method.md) — how a capture is made and compared
- [Departures](departures.md) — every place motionvm knowingly does something else, and why
- [Open questions](open-questions.md) — what is unknown, unverified or hypothetical about the original
- [Screens](motion32/engine/screens.md) — what the pixel-exact match does and does not settle
- [FM driver](motion32/engine/fm-driver.md) — the 32-bit rebuild and how it is checked
- [PSM music](motion16/formats/psm-music.md) — the 16-bit rebuild and how it is checked
