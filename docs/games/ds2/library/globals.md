[← Documentation index](../../../README.md)

# Module 2 — Globals and Core Helpers

*Dunkle Schatten 2 — this page describes one of the game's own script modules. The engine it runs on is documented under [MOTION 32-bit](../../../README.md#motion-32-bit-ds2).*

Module 2 is loaded first and stays resident. Of its 263 words, 205 are
plain variables and 29 are constants; the remaining 29 have code bodies.
Everything else in the game refers to this module's storage.

## Helper words with code

| Word | Effect | Body |
|---|---|---|
| `++` | `( addr -- )` | increment the cell (`DUP @ 1 + SWAP !`) |
| `--` | `( addr -- )` | decrement the cell |
| `,,` | `( v addr -- addr+4 )` | store and advance — a build pointer helper |
| `+@` | `( addr off -- v )` | fetch with offset (`+ @`) |
| `+!` | `( v addr off -- )` | store with offset (`+ !`) |
| `0+!` | `( addr off -- )` | zero a field |
| `0!` | `( addr -- )` | zero a cell |
| `1+` | `( n -- n+1 )` | |
| `SMDESC` | `( addr -- )` | "set my descriptor": fetch a handle *from a variable* and `ACTDESC` it — the single most-used helper in the game |
| `0SDWAIT` | `( -- )` | clear the current descriptor's wait |
| `XYLSITEM.` | `( x y lev spr -- h )` | create a sprite descriptor (`NEWSETDESC` + `SDSPR`) |
| `XYLBITEM.` | `( x y lev img -- h )` | create a full-image descriptor (`SDBL`) |
| `XYLTITEM.` | `( x y lev tb -- h )` | create a text descriptor (`SDTB`, entry 1, font `_F1`) |
| `->MOVEMEM` | `( src dst len -- )` | cell-wise copy, length rounded up to whole cells |
| `WALKQUEUE` | `( -- addr )` | read the active walk queue pointer |
| `OUT` | `( n -- )` | debug print sink (shows up on the debug overlay) |
| `JEHOVA` | `( -- )` | empty — a Monty Python joke left in the code |
| `2OVER` | — | **broken**: a nonstandard `( a b c -- a b c a )` with stray cells in its body; the one caller relies on the `a b c a` behavior |

`O.AktOrder`, `O.AktObj1`/`O.NextLoc` (a union — for a "leave" order the
object slot holds the destination location), `O.AktObj2`, `O.MenStatus`,
`O.TakeSt`, `O.TakeEnd`, `O.DoTake` are field-address words into the
`_ORDER` record (offsets 0, 4, 4, 8, 12, 24, 28, 32) — see
[Game library](game-library.md) for that record's role.

## Constants

**Table geometry** (the record sizes behind the per-location data —
see [Blocks](../../../motion32/formats/block.md)):

```
SIZE_SPEAKER=12  A_SPEAKER=10        A_ROUTES=25  S_ROUTES=36  S_XROUTES=24
SIZE_KLICKAREA=20  A_KLICKAREA=10    A_FITEM=80   S_FITEM=20
A_LDITEM=35  S_LDITEM=64  S_ORDER=520
SIZE_PERINFO=184  SIZE_PERSTRUCT=480  SIZE_PERCOORD=1224
```

**Verb-capability bits** (stored in item `ORDER`/`FORDER` fields):
`TAKEABLE=3`, `HANDLEABLE=6`, `USEABLE=10`, `TALKABLE=18`,
`LEAVEONLY=128`. Bit 1 (value 2) is the shared "interactive hotspot" bit
present in all of them.

**Interaction states** (`_ORDER+12`, `O.MenStatus`): `MS_IDLE=0`,
`MS_MEN_INV=2`, `MS_OBJ_AS_M=3`, `MS_MEN_PIC=4`, `MS_LEAVE=9`,
`MS_P1_SPEAK=12`, `MS_P2_SPEAK=13`, `MS_MCM=14`, `MS_MEN_DIALOG=17`.
States 12–18 mean "a dialogue is on screen". State 8 is tested by code
but has no named constant.

**Preset variables** (nonzero initial values): `_STASK=1`,
`_STARTLOC=23` (the title), `_TSPEED=250` (text display time factor),
`_TSMODE=2` (text-speed setting 1–3), `_GSMODE=1` (walk-speed setting),
`_ITEMTEST=600` (first inventory-sprite id).

## Variable catalog

The 205 variables group by subsystem (sizes of the array variables match
the constants above):

- **Screens**: `_SCREEN` (main), `_IS` (interface/status bar),
  `_DSCREEN` (debug); backgrounds `_BG`, `_BG2`, `_BG3`.
- **Fonts and text descriptors**: `_F1 _F2 _F3` (font handles),
  `_TI1 _TI2` (floating captions), `_IINFO` (speech/info line),
  `_MINFO` (mouse-over line), `_TINFO`, `_THANDLE`.
- **Location state**: `_ACTLOC`, `_LASTLOC`, `_NEXTLOC`, `_STARTLOC`,
  `_LOCTABLE` (100 cells).
- **Task machine**: `_LOCTASK`, `_LOCTASKPHA`, `_LOCTASKWAI`,
  `_LTHANDLER`, `_ANHANDLER` (animation handler), `_BUSY`,
  `_BUSYMOUSE`, `_SYS_LEVEL` (0 = free, 1 = busy/cutscene, ≥ 2 =
  menu/system), `_T1KILL`, `_TONORMAL`.
- **Input**: `_MX _MY` / `_LMX _LMY` (current/last mouse), `_MMX _MMY`
  (mouse within the picture area, −1 outside), `_IMX _IMY` (mouse within
  the status bar, y −400), `_MLK _MRK` (buttons), `_MPRESSED`
  (debounce), `_AKTKEY`, `_DOMOUSE`.
- **Inventory**: `_GAMEINV` (100 cells of object ids), `_ACTINV`
  (scroll offset), UI descriptors `_INVUP _INVDOWN _INVICON _INVOV1
  _INVOV2 _INVSEL _INVSEL1 _INVSEL2 _INVSP`, `_INVSTAR` (5 save-slot
  stars), `_INVMODE` (UI mode), `_ITEM`, `_IBNR _IBON`.
- **Records**: `_LDITEM` (35×64 B location items), `_FITEM` (80×20 B
  objects), `_ORDER` (520 B interaction record), `_ROUTE` (904 B),
  `_XROUTE` (600 B), `_KLICKAREA` (200 B), `_DIALFIELD` (8000 B
  dialogue workspace), `_MESSPIPE` (1200 B).
- **Walking**: `_WALKQUEUE`, `_DESTX _DESTY`, `_FORBIDWALK`,
  `_FORBIDTALK`, `_ACTPERSON`.
- **Speech/sound**: `SPEAKER` (10×12 B), `_SPEAKTABLE` (10×40 B),
  `_ACTSPEECH`, `_ACTTABLE`, `_SPEECH`, `_ACTMUSIC`, `_SxCol _SxTDT
  _SxHandle` (current speaker), `_SpCol _SpTDT _SpHandle _SpX _SpY
  _SpZ` (5-slot speaker attributes).
- **Verb dispatch vectors**: `_LC_TAKE … _LC_LEAVE` (per-location,
  cleared on every location change), `_P_CALCTAKE … _P_CALCLEAVE`
  (global fallbacks).
- **Saves**: `_?STARTUP`, `_DOSAVE`, `_LOADTABLE` (5 slots), `SAVEBUF`
  (184 B), `_NAME0…_NAME5`, `_DATE1…_DATE5`, `_SCORE1…_SCORE5`,
  `_GSCORE`.
- **Menu/document viewer**: `_MENCZW` (menu click latch), `_DOC`
  (document page), `_ANL1 _ANL2` (full-screen layers),
  `_ANT1…_ANT10` (text) and `_ANG1…_ANG10` (graphics) descriptors,
  `_GUIMODE`.
- **Debug**: `_DEBUGON` (0–4 state machine), `_?DEBUG`, `_TDEBUG`,
  `_DT1…_DT6`, `_DVAL _DVAL1`, `_DINPUT` (200-byte line buffer) +
  `_DIPT`, `_ZWSTACK` (scratch).
- **Scratch**: `_ZW` ("Zwischenwert" — intermediate value), `_LEN
  _SADR _DADR` (copy helper locals), `_CHKM _CHKMSTAT` (talking-head
  descriptor and state).

## See also

- [Game library](game-library.md) — module 5, which operates on all of
  this state
- [Module map](../module-map.md)
