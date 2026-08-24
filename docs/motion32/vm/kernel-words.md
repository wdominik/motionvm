[← Documentation index](../../README.md)

# Kernel Words

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit-enviro).*

The Forth kernel's built-in words are registered in three arrays inside
`ENGINE.EXE`'s data segment. Each array is a NULL-terminated list of 8-byte
entries:

```c
struct { const char *name; void (*handler)(); }
```

terminated by a `{"None", NULL}` sentinel. The arrays are **not aligned**
(the first starts three bytes off a 4-byte boundary), so locating them
requires a byte-wise scan. Addresses refer to the engine's relocated image
(see [ENGINE.EXE](../engine/engine-exe.md)).

| Address | Entries | Contents |
|---|---|---|
| `0xdb067` | 101 | Core Forth and the compiler runtimes (`_PutLit`, `_CheckIf`, …) |
| `0xdb397` | 27 | Compiling words (`:` `;` `IF` `DO` `BEGIN` …) |
| `0xdb504` | 228 | Domain words (`NEWSCREEN`, `SDIAL`, `ANIMPLAY`, …) |

356 words in total. Table 0's entry 0 is a word named `##`; `VAR`, `CONST`,
and `CREATE` sit at table-0 indices 17, 18, 19. The module-management
family occupies indices 59–64: `=>START`, `=>INIT`, `=>END`, `=>GET`
(ordinal 414), `=>PUT`, `=>ERASE` (424).

Table 1 holds exactly these 27 compiling words, in table order:

```
:  ;  IF  >IF  =IF  =>IF  ELSE  ELSEDUP  ENDIF  DO  LOOP  +LOOP  /LOOP
BEGIN  UNTIL  WHILE  REPEAT  [N]  ."  "  .""  $"  F"  /*  //  COMPILE  ALLOT
```

They run at compile time only and never appear in compiled code (which is
why their ordinal base is unknown — see
[Threaded code](threaded-code.md)). Note the comparison family `IF`/`>IF`/
`=IF`/`=>IF`: two-operand conditionals compiled to `_CheckIf`-style
runtimes, and both comment forms (`/*`, `//`).

**Of the 356 words, the game's script modules use 174.** That set is the
real scope of any reimplementation. Counted by disassembling all 86 script
modules and taking the union of the kernel names their cells name; no module
word shares a name with a kernel word, so the count is exact.

A count taken any other way comes out low. Walking a module only as far as
`0x50 + 16004` misses the words appended past module memory, and a static
count over *decoded instructions* stops wherever the disassembler loses its
footing — see
[Measuring a handler](#measuring-a-handler-count-bytes-not-instructions).

## Calling convention

Every handler fetches its arguments one at a time through a helper function
and stores results through a second one. The word's own name is kept in
`eax` during the fetch so that a stack underflow can report its cause.

| Address | Role |
|---|---|
| `0x66c95` | Pop a value from the data stack |
| `0x66cfa` | Push a value onto the data stack |
| `0x65aaa` | Stack check |
| `0xdb053` | Stack depth |
| `0xee6c4` | Stack pointer |
| `0xdb4bc` | Current screen |
| `0xf2af0` | Current descriptor |

### Determining a word's arity

The reliable signal is the **stack check**: before touching the stack,
every handler calls `0x65aaa` with a count in `eax` — **positive for
arguments it will consume, negative for results it will leave**. This holds
even for handlers that bypass the pop helper entirely: `GET` walks the
stack pointer itself, so counting pop-helper calls yields 0 where the true
arity is 3.

Counting pop/push call sites is still useful as a second signal, but for
handlers with branches it is only an **upper bound**: `?SOUND` contains two
push sites but leaves exactly one value; `DEFTDT` contains 17 pop sites but
consumes 9 (one selector, then eight more in one of two branches). The
extreme case is `DOWALK` (statically 3 pops / 32 pushes, actually 1 → 0):
native words may invoke *other primitives* through the Forth stack, so
their internal pushes are argument passing, not results — see
[Walking](../engine/walking.md).

### Measuring a handler: count bytes, not instructions

Any figure derived from a linear disassembly of `ENGINE.EXE` is a floor, not
a measurement, because the disassembler stops short in two ways:

- **It cannot follow `jmpl *%cs:0x…` jump tables.** It decodes the table
  itself as code and gives up a few bytes in. `0x7bf80` therefore measures
  seventeen instructions and is `EXECORDER` with 783 — the body continues at
  `0x7bfbb`, which has to be supplied by hand as a second root. `GMSHOWMENU`
  is affected the same way. The tables must be read as data.
- **It loses alignment inside a body.** In `DOORDER` the bytes at `0x7dd5e`
  decode as nonsense and the stream only recovers at `0x7dd6f`; an
  instruction count stops at 1026 for a function twice that long.

So measure over **byte ranges** — entry point to the next word's entry point —
rather than over decoded instructions. Every under-count of a handler's size
has had one of these two causes.

## Measured arities (startup set)

The words the game's `STARTUP` word needs, grouped by arity
(arguments → results):

| Arity | Words |
|---|---|
| 0 → 0 | `TOGFX` `RESETANIM` `RESETFONT` `SDINACTIVE` `SDAUTOBUF` |
| 0 → 1 | `NEWSCREEN` `HICOLOR` `?SOUND` |
| 1 → 0 | `SETRES` `ACTDESC` `SDX` `SDSPR` `SDCEN` `SDVCEN` `SDWAIT` `SDWORD` `SDTXT` `SDTB` `SDCOL` `SDFNT` `SDTDT` |
| 1 → 1 | `+FONT` |
| 2 → 0 | `SCRSIZE` `SCRPOS` `SCRFVSIZE` `SCRVSIZE` `SCRVPOS` |
| 3 → 0 | `GET` `XGFXSTAT-` |
| 6 → 1 | `NEWSETDESC` |
| 9 → 0 | `DEFTDT` |

Two of these are traps for anyone reading call sites instead of handlers:

- **`NEWSETDESC` takes six arguments, not three.** Call sites look like
  `0 0 NEWSETDESC` with one value before them because the remaining
  arguments come from the calling word's stack. See
  [Descriptors](../engine/descriptors.md).
- **`DEFTDT` takes nine**: a template number on top (checked against 1–9),
  then eight more in one of two branches.

## Word categories

A rough map of the 228 domain words by prefix convention:

| Prefix / family | Meaning |
|---|---|
| `SCR…` | Screen configuration (`SCRSIZE`, `SCRVPOS`, …) |
| `SD…` | Set a property of the *current descriptor* (`SDX`, `SDSPR`, `SDTXT`, …) |
| `GD…` | Get a property of the current descriptor (`GDWIDTH`, `GDTXTLEN`, …) |
| `GSCR…` | Get a property of the current screen (`GSCRX`, `GSCRACT`, …) |
| `X…` | Extended/graphics utilities (`XGFXSTAT-`, `XGFXVFLIP`, `XATMOUSE`, …) |
| `?…` | Predicates/queries (`?SOUND`, `?SLDONE`, `?LTWAIT`, `?XINSIDE`, …) |
| `=>…` | Module management (`=>GET`, `=>PUT`, `=>ERASE`, `=>INIT`, `=>END`, …) |
| `ANIM…` | Animation (`ANIMPLAY`, `ANIMSIM`, `RESETANIM`, …) |

`=>GET ( n -- )` loads script module `n` (file pattern `%03d.SCR` or the
RSC copy); `=>PUT` writes a module back to disk — the authoring path that
also serves as the save mechanism candidate (see open questions).

## Assorted handler-verified semantics

Small words whose handlers have been read completely:

| Word | Semantics |
|---|---|
| `CTRL ( addr -- )` | Stores the control callback at `0xdb4a8`; read only by `ANIMPLAY` (see [Game loop](../engine/game-loop.md)) |
| `DELAY ( n -- )` | Does not wait: sets the frame length, `0xdb4b8 = 200/n` |
| `QUITANIM ( -- )` | Clears `ANIMPLAY`'s running flag — ends the game |
| `?KEY ( -- k )` | Takes the waiting key, or zero when none waits. **Not a character** and not a flag: the handler at `0x6258c` calls the translator `0x2379b`, which reads INT 16h through the trampoline table at `0x88f9c` — `0x83e78` is AH=01, `0x83e9c` is AH=00 (AL the character, AH the scan code), `0x83eb8` is AH=02 — and answers `flags \| code`. `0x100` says the low byte is a **scan code** (`0x23894`); `0x200`, `0x400` and `0x800` are Shift, Ctrl and Alt, folded out of the shift state by `0x23a03`. So a character key answers its own byte — `ICTRL` opens with `?KEY DUP _AKTKEY !` (module 4, `0x022a0`) and tests 8 backspace (`0x051a0`), 13 Return, 27 Escape (`0x02c40`), 48…57 the teleport digits (`0x04e60`), 103 `g` (`0x04d40`), 105 `i` (`0x05020`) — while a key that carries none answers `0x100` over its scan code: 328, 336, 331 and 333 for the cursor keys, which is what the mailbox dispatches on (module 216, `0x0c21c`), and 315, 316 and 323 for F1, F2 and F9 in the debug layer (`0x027a0`, `0x052e0`). Shift and Ctrl on a function key fold onto the plain one and set their bit (`0x23860`, `0x2387a`); Ctrl and a letter answer the upper-case letter with `0x400` (`0x238b8`); Alt and a letter reads the table at `0xd667c`, whose entry for `Z` names `O`'s scan code and is therefore never reached. The `_AKTKEY @ 0 >` form is the "any key" test and reads correctly for all of it |
| `KEY ( -- k )` | The same value, waited for: `0x62501` loops on `0x2379b` until it is non-zero, running the control callback at `0xdb023` while it waits. No shipped module uses it |

| `GET` size 0 | **"Whole resource"**, not "nothing": a zero size is replaced by the resource's own length before copying |
| `EXIST ( n -- f )` | Tests for the file `NNN.blk` — how save slots 701–705 are probed. Answers **−1** or 0 (`0x66b18`/`0x66b77`), unlike `=>EXIST`, which asks the resource catalog |
| `SPEEDMODE ( n -- )` | Writes a global no word reads back |
| `MOUSEINFO` | Takes **24** arguments (the handler pops exactly that many) |
| `ANIMPLAY` | Pops **none** of the ten values the game pushes for it |
| `640x480x256` etc. | The mode words push small ordinals — 2, 1, 4 — not packed dimensions; `SETRES` consumes them |
| `GFXCRUNCH` / `XGFXCRUNCH` | Toggle one flag bit in a record — no pixel work |
| `RGB->COL ( b g r -- i )` | 6-bit components in, a palette index out |
| `MOUSEXY` | Pushes y first, then x (x ends on top) |
| `?ACTDESC` / `GDNR` | The same two instructions — both return the current descriptor handle |

## Authoring-only words

`FULLCUT`, `XYCUT`, and `SCANCUT` sound like screen capture but are
**import** tools of the authoring environment: they read an image file and
slice it into GFX8 items. The engine's own messages make this explicit:

> `FULLCUT: Bild #s nicht gefunden!`
> ("FULLCUT: image #s not found!")

> `Cut-Funktion:|Bild #s nicht gefunden!`
> ("Cut function:|image #s not found!")

The engine has no word that saves its own screen contents.

## Open questions

- Handler-level semantics of a shrinking set of used words — the audio
  family is fully mapped ([Audio](../engine/audio.md)) and script usage
  pins down most others; what remains (native animation, dialogue
  requesters, `MOUSEINFO`/`DOORDER` internals, module persistence) is
  listed in [Open questions](../../open-questions.md).
- Table 1's ordinal base.

## See also

- [Threaded code](threaded-code.md) — the ordinal system
- [Word semantics](word-semantics.md) — the measured core words
- [ENGINE.EXE](../engine/engine-exe.md) — locating the tables
- [Descriptors](../engine/descriptors.md), [Screens](../engine/screens.md) —
  the domain words in context
