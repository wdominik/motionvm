[← Documentation index](../../../README.md)

# Text and Speech Words

*Dunkle Schatten 2 — this page describes one of the game's own script modules. The engine it runs on is documented under [MOTION 32-bit](../../../README.md#motion-32-bit).*

The script-side text machinery (module 5, plus `SAYKARSTEN` from module
13). The underlying descriptor mechanics are in
[Text rendering](../../../motion32/engine/text-rendering.md); the sample playback layer
is in [Audio](../../../motion32/engine/audio.md).

## Captions and the info line

Three text descriptors carry almost all running text: `_IINFO` (the
speech/info line), `_TI1`/`_TI2` (floating captions), and `_MINFO` (the
mouse-over line the engine fills natively).

| Word | Effect |
|---|---|
| `NEWTEXT. ( x y lev tb -- h )` | create a text descriptor bound to table `tb` |
| `SETT1 ( col tdt x y txt tb -- )` | show a caption in `_TI1`: table/entry, centered at (x, y), template `tdt`, color `col+256`, display time from `TSX`. Clamps the left edge to stay on screen |
| `SETT2 ( … -- )` | the same for `_TI2`, without the clamp |
| `?READYT1` / `?READYT2 ( -- f )` | true when the caption has expired (descriptor inactive) — task phases wait on this |
| `?TEXTREADY ( -- f )` | the same for `_IINFO` |
| `FT1` / `FINISHTEXT ( -- )` | the expiry callback: if the descriptor shows table 2 entry 1 (the blank caption), hide it; otherwise switch it to that blank entry. Installed via `SDWORD` on every spoken line |
| `SETINFOTEXT ( -- )` | `TSC TS` — re-clamp and re-time the info line; installed in the engine interface |
| `TSC ( -- )` | clamp the current descriptor into the visible area (x within 10…620 of the screen edge, y within 1…400) and re-anchor its center |
| `TS ( -- )` | set the display time: `TSX SDWAIT` |
| `TSX ( -- t )` | display-time heuristic from the measured text length: < 20 → 20; 20–50 → length; 50–100 → max(len/2, 35); > 100 → max(len/3, 60); scaled by `_TSPEED/100` |

`_TSPEED` comes from the options menu (150/250/400 for the three
text-speed settings).

## Character speech

A "say" is a caption positioned above the speaking figure:

| Word | Effect |
|---|---|
| `DEFMC ( col tdt handlevar -- )` | define the **main character** as speaker: stores handle, template, color (+256) in `_Sx…`. Every scene macro calls `50 2 _WALKKARSTE @ DEFMC` |
| `SAYMC ( txt tb -- )` | say a line as the main character: show `_IINFO` with the text, centered 20 px above the figure's sprite, level 127, expiry callback `FINISHTEXT`, time `TSC TS`, styled from `_Sx…` |
| `SAYKARSTEN ( txt -- )` | module 13's shorthand: `txt 3 SAYMC` — **Karsten's lines live in text table 3** |
| `DEFTP ( x y z col tdt handle slot -- )` | define one of five fixed speaker slots (`_Sp…` arrays) |
| `SAYTP ( txt tb slot -- )` | say a line at a fixed slot's position/style |
| `SAY_CHKM ( txt -- )` / `SAY_INSERT ( txt -- )` | captions at fixed positions (320, 120) / (320, 240), table 2, template 9 — the talking-head overlay (unused by the shipped scripts) |
| `TON_CHKM` / `TOFF_CHKM ( -- )` | arm/disarm the talking-head state `_CHKMSTAT` (4/0) and reset its mouth sprite (529); unused |

## Dialogue resources

`->DIAL ( dialrec -- )` prepares a dialogue: marks the dialogue's text
table in the cache and loads its definition block (ids 450–570) into the
8000-byte `_DIALFIELD` workspace. The `dial…` records live in
[module 11](objects-and-flags.md).

## The speaker/cue system (built for voice acting, never enabled)

- `SETSPEAKER ( handle col1 tdt x y z col2 slot -- )` — fill one of ten
  40-byte `_SPEAKTABLE` entries binding a figure to a speaker slot.
  Called by 16 location modules with per-character `Pers…` data words.
- `->SPEAKER ( stopfn startfn slot -- )` — fill a `SPEAKER` slot
  (callbacks + idle marker). Never called.
- `->SPEECHSEQ ( name tbl -- )` — start a speech WAV and set the cue
  table. Never called.
- `SPEECHSEQ-> ( -- 0 | handle )` — poll the running speech sample
  (clearing it when finished); consumed by the
  [speech pump](../../../motion32/engine/audio.md).

The full pump logic and why the system is dead in the shipped game are
described under [Audio](../../../motion32/engine/audio.md).

## See also

- [Text rendering](../../../motion32/engine/text-rendering.md) — measurement, backing,
  outline, and the self-hide mechanism these words rely on
- [Dialogue machine](../../../motion32/engine/dialogue-machine.md)
- [Audio](../../../motion32/engine/audio.md)
- [Dialogue](dialogue.md)
