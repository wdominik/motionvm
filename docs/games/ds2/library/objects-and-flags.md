[← Documentation index](../../../README.md)

# Module 11 — Objects, Story Flags, and Dialogue Data

*Dunkle Schatten 2 — this page describes one of the game's own script modules. The engine it runs on is documented under [MOTION 32-bit](../../../README.md#motion-32-bit-ds2).*

Module 11 (405 words) is the game's world state: the inventory-object
enumeration, the story- and dialogue-flag arrays, ~85 story-progress
variables, and 119 dialogue data records. Module 12 (3 words) performs
the one-time initialization and is unloaded afterwards.

## The object enumeration

Every object is a **constant holding a slot index 1–74** into the
80-entry object table (`_FITEM`, 20 bytes per record; 0 = "no object",
75–79 spare). The names are German; a selection with translations:

| Id | Word | Meaning |
|---|---|---|
| 1, 2 | `EMPTY` | placeholder (defined twice — the second shadows the first, so id 1 is unreachable by name) |
| 3 | `STIFT` | pen |
| 4 | `BROSCHÜRE` | brochure |
| 5 | `ZETTEL` | note |
| 6 | `TELEFONKARTE` | phone card |
| 7 / 8 / 28 | `CD` / `DISK2` / `DISK1` | CD, floppy disks |
| 9 | `FLICKZEUG` | tire repair kit |
| 10–12 | `ZISCHLÜSSEL`, `GESCHLÜSSEL`, `SCHRSCHLÜSSEL` | keys (room, building, locker) |
| 14 / 15 | `LEEREIMER` / `VOLLEIMER` | empty / full bucket |
| 16 | `ZEITUNG` | newspaper |
| 17 | `LUFTPUMPE` | air pump |
| 18 / 19 / 35 / 36 | `RAD`, `1SCHLAUCH`, `2SCHLAUCH`, `3SCHLAUCH` | wheel and the inner tube in its three puzzle states |
| 29 / 33 / 39 | `DIKTIERGERÄT`, `MIKRO`, `DIKTMIKRO` | dictaphone, microphone, and their combination |
| 30 / 38 | `FLUGIS` / `NAZIFLUGI` | leaflets |
| 42 | `GELD` | money |
| 50 | `BEKENNERSCHREIBEN` | claim-of-responsibility letter |
| 51 | `ZIPPO` | lighter |
| 52–54 | `PAPER`, `PAPERSTEIN`, `STEIN` | paper, paper-wrapped stone, stone |
| 57 | `ANRUFBEANTWORTER` | answering machine |
| 69 / 70 | `TASCHENLAMPE` / `1TASCHENLAMPE` | flashlight (without/with batteries) |
| 73 | `EMAIL` | e-mail |
| 74 | `INFOBUCH2` | the info book (the player's starting item) |

**An object's state changes by re-registering its record**: the pumped-up
inner tube keeps id 19 but gets a new name text, sprite, and
description via `->FALL`.

## The object records

`INIT_ITEMS` (module 12) registers all objects with
`name order gfx info dir OBJ ->FALL`:

- **name** — text id of the object's display name (128–310).
- **order** — the verb-capability mask; almost always
  `TAKE_USE_HANDLE` (15).
- **gfx** — the inventory sprite, ids 600–738 **in steps of two** (a
  normal/highlighted pair per object, which is why state variants share
  a base sprite).
- **info** — text id of the examine description; 1 or 2 means "handled
  by code, not by a canned line".
- **dir** — 0 for everything except the school notebook (4).

## Story and dialogue flags

Two arrays of one-cell flags, with query/set/clear triples:

| Array | Size | Words |
|---|---|---|
| `_SLDONE` (storyline done) | 120 flags | `?SLDONE ( n -- f )`, `->SLDONE ( n -- )`, `->!SLDONE ( n -- )` |
| `_DDONE` (dialogue done) | 150 flags | `?DDONE`, `->DDONE`, `->!DONE` (note the asymmetric name) |

The setters double as debug tracers: with debugging enabled they pop a
caption showing the flag number (the dialogue tracer offsets its number
by +115 into the same label table — an authoring inconsistency).
Six separate variables `_?HINT1DONE`…`_?HINT6DONE` track shown hints.

## Story-progress variables

About 85 variables named `_K…` ("Karsten has …") record plot progress
with small multi-valued states (0–5), e.g. `_KGABY`, `_KFEUERZEUG`
(lighter), `_KBRANDART` (arson article), `_KANRUFBA` (answering
machine), `_KVERHAFTUNG` (arrest), `_KENDE` (ending), `_KINTRO`.
Alongside them sit per-object state variables named after the objects
(`_?STIFT`, `_?ZEITUNG`, `_?SCHLAUCH1`, …). Two variables initialize to
1: `_BOXLOCKED` and `_COMPLOCKED` — the BBS and the computer both start
locked.

## Dialogue records (`dial01`–`dial112`)

119 data words, one per conversation. Observed record shape (cells):

| Cell | Content |
|---|---|
| 0 | dialogue id (50–170, allocated in definition order) |
| 2 | count of trailing per-line cells |
| 3, 4 | unknown (3 is always larger than 2 — total lines?) |
| 7 | text-table id (450 + index — one text table per dialogue) |
| 8 | unknown (15 or −1) |
| 9–10 | the record's own ASCII name (`dial03`, `dial38a`, …) |
| 12… | one small value (1/2/3) per line — speaker or flag codes |

`->DIAL` loads the matching dialogue **block** (ids 450–570) into the
`_DIALFIELD` workspace — see [Blocks](../../../motion32/formats/block.md). Field
meanings beyond id/count/text-table are unmapped (open question).

## Module 12 — initialization

- `INIT_PORDER` — installs module 13's `PCALC*` handlers into the
  global verb vectors (see [Dialogue](dialogue.md)).
- `INIT_ITEMS` — the 88 `->FALL` registrations above.
- `DS_INIT` — runs both, gives the player the info book
  (`INFOBUCH2 ADDITEM`), generates **33 mirrored walk sprites** with
  `XGFXVFLIP` (nine ranges, e.g. 800→860×5 — the game stores only one
  facing and mirrors the other), and creates the document-viewer
  descriptors (background sprite 32, page arrows 39/40).

## Open questions

- The full meaning of the dialogue-record fields (cells 1, 3, 4, 8, and
  the per-line codes).
- The complete story-flag → plot-event assignment.

## See also

- [Dialogue](dialogue.md) — the code that consumes these flags
- [Game library](game-library.md) — `->FALL`, `ADDITEM`, the record
  accessors
- [Blocks](../../../motion32/formats/block.md) — the dialogue blocks
