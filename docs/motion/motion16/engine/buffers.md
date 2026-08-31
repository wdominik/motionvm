[← Documentation index](../../README.md)

# Off-Screen Buffers

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

The kernel has a family of buffer words — `BUFON`, `SETBUF`, `SDBUF`,
`RESETBUF`, `KILLNBUF`, and the descriptor mode `SDBLK` beside them — that
the 32-bit game never exercises: in Dunkle Schatten 2 `SETBUF`/`RESETBUF`
are inert bookkeeping. In Die Enviro-Kids greifen ein they carry the intro and
every person
sprite, so they cannot be stubs here. What follows is what the call sites
establish; the handlers are unread.

## The words and how the game calls them

| Word | Sites | Call | Reading |
|---|---:|---|---|
| `BUFON` | 1 | `RUN`, right after `NEWANIM` | `( -- )` — turn buffer compositing on for the session |
| `SETBUF` | 129 | `320 200 1 SETBUF` (intro), `100 140 ?LPB SETBUF` (`NEWPERS`), `0 0 1 SETBUF` (intro teardown) | `( w h id -- )` — allocate or resize buffer `id`; `0 0` frees it |
| `SDBUF` | 108 | `2 SDBUF` … `6 SDBUF` on the intro motifs, `1 SDBUF` on the intro screen, `?LPB SDBUF` on a person | `( id -- )` — the current descriptor draws into buffer `id` |
| `KILLNBUF` | 1 | `?LPB 1 + -1 KILLNBUF` in `INCLLOC` | two arguments; drops the per-person buffers on leaving a location, the same way `?LPD 1 + KILLNDESC` drops their descriptors next to it |
| `SDBLK` | 2 | on descriptors | a descriptor mode; in the 32-bit engine it sets a block-centering bit |
| `RESETBUF` | 0 | — | unused by this game |

`?LPB` is a script word over the variable `_LPB`, a running buffer number
that `NEWPERS` increments (`_LPB ++`) — each person gets a buffer of its
own, 100×140, and draws through it.

## What the intro does with them

`STARTINTRO` builds five motif descriptors (sprites 2482–2486), sets each
inactive and assigns buffers 2–6 to them one each; a logo and a text
descriptor follow; then `320 200 1 SETBUF`, `1 SDBUF` and `ANIMPLAY`. On
the way out it frees buffer 1 with `0 0 1 SETBUF`. The intro therefore needs
the place a buffered descriptor leaves to be put back — which is what the
save-under reading below gives it.

## What the handlers say

Read in `ENVIRO.EXE` (`SETBUF` at file `0xac33`, `SDBUF` `0xabf1`, `BUFON`
`0xaab8`, `KILLNBUF` `0xafab`, and the frame loop's restore and draw steps
at `016a:179e`, `0362:0e5d`, `016a:0a2d`):

- **A buffer is a save-under.** The kernel keeps 80 buffer records of 0x1c
  bytes at `DS:0x1ad4` — an image pointer, the owning screen, the
  descriptor's index, the saved rectangle, and a bit that says the buffer
  currently holds pixels. Each frame, with `BUFON` on and no rebuild
  pending on the screen, the loop pastes back what every dirty buffered
  descriptor had saved under itself, redraws the dirty descriptors in
  draw-list order, and saves under them again. A descriptor without a
  buffer is drawn where it now is; the place it left keeps its old picture.
- **`SETBUF ( w h n -- )`** allocates an empty image of `w × h` — the pitch
  rounded up to eight, a six-byte `u16 w, u16 h, u16 0` header like a
  sprite's — on the far heap, or frees the buffer for `0 × 0`. It copies
  nothing from the screen. `KILLNBUF ( from to -- )` frees `from` through
  `to`, and -1 means the last buffer, 79. `RESETBUF` frees all.
- **`SDBUF ( n -- )`** stores `n + 1` in the current descriptor's +0x16 (0 =
  no buffer) and marks the descriptor dirty, like every `SD*` word.
- **`BUFON`** sets one global (`DS:0x0456`); it is not per screen. Off, the
  loop clears the surface and redraws every active descriptor each frame.

`?LPB` is a script word over the variable `_LPB`, a running buffer number
that `NEWPERS` increments (`_LPB ++`) — each person gets a buffer of its
own, 100×140, and draws through it. The intro's motifs slide across the
screen on buffers 2–6, and the briefing text stands on buffer 1.

The rebuild keeps what `SETBUF` allocates and `SDBUF` attaches and treats a
buffered descriptor as a save-under by recomposing the place it leaves; how
that differs from the original's paste-back, and when it could show, is a
[departure](../../departures.md#the-16-bit-machine).

## Open questions

- **The descriptor drawer**, `016a:0aac`: the save step itself, and how it
  clips and keys a sprite.
- The priority byte a buffer record carries at +0xc against a descriptor's
  +0x29 — read as a tie-break for overlapping saves, not established.
- What `SDBLK` does here; its two sites are in descriptor setup.
- The 32-bit engine has the same names; whether its inert `SETBUF` is the
  same feature left unused by that game or a stub in the engine is open on
  that side too ([departures (MOTION 32-bit)](../../departures.md)).

## See also

- [Boot and frame loop](game-loop.md) — where the calls sit
- [Descriptors and screens](descriptors.md) — the descriptor words around them
- [Kernel words](../vm/kernel-words.md) — ordinals 210–214
