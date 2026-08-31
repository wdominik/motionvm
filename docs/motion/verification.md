[← Documentation index](README.md)

# Verification

*What motionvm has been held against, for both generations of the engine and every game it plays. Which comparison covers which game is named in each entry. Deliberate divergences are a different record and live in [departures](departures.md).*

There are two kinds of check in this project, and the distinction matters
enough to be written down rather than implied.

A check against the original's **own output** — a screen capture, a register
dump, a mixer capture — says the engine *behaves* as the engine did. A check
against the original's **files** — its resources, its bytecode, its driver
binaries — says only that the readers agree with the data. The second is the
weaker claim, and it is the one that covers most of what is here.

## Held against the original's own output

### Dunkle Schatten 2, on the 32-bit engine

- **One rendered frame**, pixel for pixel — the title screen, 307 200 pixels
  over 206 palette indices, held against the original running under DOSBox-X.
  Every index maps to one color and every color back to one index, which is a
  stronger check than comparing RGB ([screens](motion32/engine/screens.md)).
- **Fourteen VM primitives**, against modules the 1996 compiler produced.
- **A song's first 8 000 OPL register writes**, and one mixer envelope
  ([FM driver](motion32/engine/fm-driver.md)).

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

Those comparisons need recordings of the original engine, and a recording of a
game is no more redistributable than the game. None ships here, so the checks
that consume them are not part of the test suite. What ships is their result,
stated above.

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

## What has not been compared

Interaction, dialogue, walking, savegames, the verb menu and most locations of
all five games have never been differentially compared against a recording, and
**nothing of Jeff Jet or Hilfe für Amajambere has been**. Victor Loomes is
covered as far as its intro reaches and no further: what a played room looks
like there is as unchecked as it is for the other two.

That is not a gap being hidden. It is the honest edge of what a
reimplementation can claim without the original running beside it, and it is
why the two lists above are kept apart.

## See also

- [Departures](departures.md) — every place motionvm knowingly does something else, and why
- [Open questions](open-questions.md) — what is unknown, unverified or hypothetical about the original
- [Screens](motion32/engine/screens.md) — what the pixel-exact match does and does not settle
- [FM driver](motion32/engine/fm-driver.md) — the 32-bit rebuild and how it is checked
- [PSM music](motion16/formats/psm-music.md) — the 16-bit rebuild and how it is checked
