[← Documentation index](../README.md)

# Script-Side Animation

Two animation systems live in script code: module 6's general **sprite
animation runtime** (the largest coherent subsystem outside the game
library) and module 5's small **sequence animator** (`GT`/`DODIR`).
Both ultimately just call `SDSPR` on descriptors — the kernel's own
animation words (`ANIMPLAY`, `ANIMSIM`, `GETANIM`, `PUTANIM`) are a
separate, still unmapped native layer.

## Module 6 — the animation runtime

Animations are allocated from a pool at run time; nothing is static.
`INITAHANDL` sets up the pool and installs the per-frame driver
`ANIHANDLE` into the global `_ANHANDLER`, which the
[control handler](shell.md) executes every frame.

### The animation record (128 bytes)

`SADESC ( n -- )` selects animation *n* as current (`_AKTANI`);
`?AN ( off -- v )` / `->AN ( v off -- )` access its fields:

| Offset | Meaning |
|---|---|
| +0 | descriptor handle |
| +4 | pointer to the frame array (a −1-terminated list of sprite ids) |
| +8 | run state: 0 off, 1 frozen, 2 forward, 3 ping-pong |
| +12 | current frame index |
| +16 | completion callback (executed at each cycle boundary) |
| +20 / +24 / +28 | x / y / level |
| +32 / +88 | frame delay / delay counter |
| +36 | kind: 1 one-shot, 2 loop, 3 suspended, 4 ping-pong |
| +40 / +44 | group-relative offset x / y |
| +48 / +96 / +100 / +104 | update frame / stop frame / last frame / start frame |
| +52 | direction flag |
| +56…+84 | movement: speed, increments, accumulators, destination x/y, axis selector |
| +80 | move state (0 off, 1 running, −1 finished) |
| +92 | "exclude from group" flag |
| +108 / +112 | repeat counter / total |
| +116 / +120 | tween-list cursor / base |

A **group** record (52 bytes, `SGADESC`/`?GAN`/`->GAN`) holds group
x/y, a scale percentage, a −1-terminated member list, and a group
callback.

### Word families

- **Creation**: `INITANI` (ten stack arguments; creates the descriptor,
  scans the frame array for its end), `CLONEANI`/`CLONEGANI`,
  `INITGANI`.
- **Control**: `STARTANI`, `STOPANI`, `CONTANI`, `FREEZEANI`,
  `SHOWANI`, `HIDEANI`, `UPDATEANI`, plus setters/getters
  (`SDANISTAT`, `SDANIFRM`, `SDANIKND`, `SDANIDIR`, `SDANIRANGE`,
  `GDANIAFRM`, `GDANSPR`, …).
- **Position/scale**: `SDANIXY`, `SDANIX/Y`, `SDANILEV`, `SDANI%SHR`,
  `GRABFIG` (adopt a figure record's center/offset/scale/level).
- **Movement**: `DESTMOVE` (destination move; `MCALCINC` computes a
  slope in 1/100ths and picks the dominant axis), `TWEENMOVE` (walks a
  16-byte-per-node tween list `{dx dy x y}`, terminated by the sentinel
  9999), `STARTMOVE`/`STOPMOVE`/`KILLMOVE`, `DOMOVANI` (one step, in
  1/1000ths). The sentinels `TW_END`/`TW_L` (9999) and `TW_R` (9998)
  mark list ends and mirroring.
- **Groups**: every verb has a `G…` counterpart iterating the member
  list and honoring the exclude flag; `SDGANI%SHR` rescales member
  offsets.
- **The driver**: `DOANI` advances one animation (delay counter, frame
  window, kind and repeat handling) and touches the descriptor only when
  the frame actually changes; `ANIHANDLE` runs `DOANI` over every
  allocated animation, fires completion callbacks at cycle boundaries,
  then runs the group callbacks.

## Module 5 — the sequence animator (`GT`)

A lighter mechanism for "reach out, act, pull back" object animations,
driven from task phases:

- A **GT record** (124 bytes): descriptor, current direction, frame
  counter, active sprite range (start/count/end), frame delay, and an
  **inline table** of four direction entries, each holding two
  `(start, count, end)` sprite triples.
- `DODIR ( gt dir -- f )` — arm and step a sequence. Directions 1–4
  select a table entry's first triple, 11–14 the second; adding 1000
  plays the sequence **in reverse**. Returns 1 = idle, 0 = running,
  −1 = just finished (reverse ends on the resting frame).
- `CALLDIR ( x xt gt dir -- r )` — the three-phase pattern: play the
  sequence forward, `EXECUTE` the callback (the actual game effect),
  play it backward; returns 3 when complete. `CALLDIRWAIT` is the same
  but repeats the callback until it reports done.

Scene code uses it as
`0 <callback> _GRABTABLE <item> LDDIR-> CALLDIR 3 = IF NEXTLTP` — the
grab animation plays, the effect fires at full extension, the arm
retracts, the task advances.

The GT direction tables are authored data stored past the location
modules' dictionaries; no script writes them.

## See also

- [Descriptors](../engine/descriptors.md) — what `SDSPR` acts on
- [Walking](../engine/walking.md) — the figure walk animation
- [Shell](shell.md) — where `_ANHANDLER` runs
