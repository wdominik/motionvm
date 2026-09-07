[← Documentation index](../../README.md)

# ENGINE.EXE V0.04.15/R78 — The Earlier Build

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.04.15/R78 with Checker 2000; what is measured here is measured on that game's files and that binary, and every address on this page is R78's. The later build, V0.06.06/R109 under Dunkle Schatten 2, is [ENGINE.EXE](engine-exe.md), and it is the one the other pages under `motion32/` cite unless they say otherwise.*

The same program, half a year younger. Checker 2000's `ENGINE.EXE` is
729 551 bytes to Dunkle Schatten 2's 845 467: the same Watcom LE for DOS/4GW,
the same IDE, compiler, debugger and player, the same three kernel tables
and the same shell registering its words one by one — with 35 words fewer,
and one handler that reads differently. Everything the other pages say
about the 32-bit engine was read on R109 and holds on R78 wherever it was
read again, which is every routine the game reaches; what this page holds is
where R78 is *itself*.

## The image

| | R78 | R109 |
|---|---:|---:|
| Pages | 146 | 170 |
| 32-bit fixups applied | 15 075 | 17 270 |
| Objects | 4 | 4 |
| Code object | `0x10000`, `0x7d381` bytes | `0x10000`, `0x929a1` bytes |
| Data object | `0xb0000`, `0x26650` bytes | `0xd0000`, `0x2dfd0` bytes |
| File offset of a code address | `address + 0x10200` | `address + 0x14e00` |

The data object sits `0x20000` lower, so no data address of one build means
anything in the other; the code is laid out alike enough that a routine of
one is found in the other by its neighbours, never by its number.

## The kernel

Three tables of `(name pointer, handler)` pairs in the data object, and the
shell's words registered singly by the init:

| Group | Where | Words | R109 |
|---|---|---:|---:|
| Table 0, the core | `0xba104` | 91 | 101 |
| Table 1, the compiling words | `0xba3e4` | 27 | 27 |
| Table 2, the domain | `0xba544` | 214 | 228 |
| The shell | registered one by one | 43 | 54 |
| | | **375** | **410** |

The ordinals the compiled modules call by come out of the init the same way
on both builds — every registered word takes five bytes of the dictionary
and its ordinal is the dictionary pointer plus four, so the bases are counts
([threaded code](../vm/threaded-code.md)):

| | R78 | R109 |
|---|---:|---:|
| Table 0 | 104 | 104 |
| `_FNAME`, registered between | 559 | 609 |
| Table 1 | 584 | 634 |
| The shell's words | 719 (`TEST`) … 884 (`->RSCPATH`) | 769 … 989 |
| Table 2 | 934 (`TOGFX`) | 1039 |

So `NEWSCREEN` is 1034 here and 1144 there, `SDINSERT` 1844, `GIVEDATE`
1924. The binding is derived, not tabled: motionvm reads the registration
out of each build's own init, and a third build would bind the same way.

The 35 words R109 has and R78 has not, by family: the savegame pair
`PUTANIM` and `GETANIM`; the string words `$->INT`, `INT->$`, `$COPY`,
`$ERASE`, `$FIND`, `$INS`, `$LEN`, `$NCOPY`, `INTERPRET$`; the file words
`?EXIST`, `EXIST`, `FDIR`, `FSDIR`, `SCANDRIVE`; the click-area test
`?XINSIDE`; the translucency and shading set `SDTRANS`, `SDSHADE`,
`GDTRANS`, `GDSHADE`, `CRTRANS`, `CRSHADE`, `SETTRANS`, `SETSHADE`,
`RESETTRANS`… (`RESETSHADE`); the native dialogue `SDIAL`, `PDIAL`; and
`?HELP`, `MDEBUG`, `PROGINFO`, `RESETTI`, `RSCINCLUDE`, `RSCSTATUS`,
`VIEWG8`. Every word R78 has, R109 has too. What the difference says about
the game is on [its pages](../../games/checker/README.md): no `?XINSIDE`
means every clickable area is a literal `MOUSEX`/`MOUSEY` range in a script,
and no `PUTANIM` means the game saves a task number and a highscore
([game structure](../../games/checker/game-structure.md#saving)).

## The calling convention

The same three helpers under other addresses, and the same globals:

| | R78 | R109 |
|---|---|---|
| `pop`, with the word's name for the underflow message | `0x56120` | `0x66c95` |
| `push` | `0x56190` | `0x66cfa` |
| The stack check | `0x54fe0` | |
| The data stack pointer | `0xc8fcc` | |
| The current screen | `0xba504` | |
| The current descriptor | `0xba508` | `0xf2af0` |
| The module table and its index | `0xc8ffc`, `0xc8fc4` | `0xee6d0`, `0xee6c0` |
| A cell address to a pointer | `0x568f0` | `0x67404` |
| The master tick counter's pointer | `0xc3be0` | `0xe7f38` |
| The `#`-formatter and `strlen` | `0x11bd0`, `0x11b80` | `0x11d66`, `0x11d0e` |

A handler on R78 disassembles as the R109 one does — the six pushes, `mov
%esp,%ebp`, then a `mov $name,%eax; call pop` per argument — which is what
lets the arity of a word be read off either build by the same rule.

## Where R78 differs

Read on both builds, handler against handler:

- **`SDINSERT` takes two arguments, `( value slot -- )`**, at `0x611c0`;
  R109's (`0x75cfe`) takes three, `( value kind slot -- )`, and files the
  kind in a second array of the text record. The record is `0x34` bytes here
  (`SDTXT` `0x5d9a0`, `SDTB` `0x5db20`) against `0x5c` there, with the five
  slots at `+0x20` in both. The layout (`0x5a100`; R109 `0x6c9f8`) hands
  every set slot to the formatter as a pointer and runs **every** text
  through it, where R109 converts each slot by its kind and skips the
  formatter for a text with no slot set. motionvm reads which of the two a
  build is off its `SDINSERT` and carries it as a capability
  ([text rendering](text-rendering.md#the-layout-the-line-window-and-the-inserts)).
- **The video mode is seeded the same way**: the kernel init (`0x56d00`;
  R109 `0x6845a`) stores its second argument as the `SETRES` selection
  (`0xba4ec`), and its one caller (`0x2fd5b`; R109 `0x37297`) passes 2,
  `640x480x256`, before `system.rsc` is read — which is why Checker 2000's
  `STARTUP` can say `TOGFX` without a `SETRES` and get 640×480
  ([screens](screens.md#the-video-mode)). The mode table is at `0x13e00`
  (R109 `0x13fc0`), the entry at `0x14030` (`0x141dc`), `TOGFX` at
  `0x5aec0` (`0x6ee64`), `SETRES` at `0x5b050` (`0x6efe2`), the mode's
  three words at `0xb5790`, `0xb5798` and `0xb579a`. Both `TOGFX`es then
  install the system palette — `000.pal` as the init resolved it, palette 0
  out of `ENGINE.RSC` here — through the handle at `0xb5854` (R109
  `0xd66fc`; the installer `0x14580`, R109 `0x14734`).
- **The mouse layer is the same, byte for byte where it is data**: the
  shape definer at `0x1f500` (R109 `0x24637`), the counted show and hide
  at `0x20340` and `0x20540` (`0x2543e`, `0x2562e`) behind the two words'
  handlers, around the count at `0xb5884` (`0xd672c`), the nearest-color lookup at `0x1b940` (`0x1ee26`)
  over the DAC copy at `0xbad74`, and the engine's two built-in pointer
  pictures at `0xb58ac` (`0xd6754`), 524 bytes the two builds hold
  identically. `TOGFX` ends by installing the arrow and showing it, and
  `NORMMOUSE` (`0x5ed50`; R109 `0x735ac`) installs it again — which is
  what this game points with on every board, since it never says
  `SHOWMOUSE` and reaches `XATMOUSE` only in its story
  ([interaction](interaction.md#the-engines-own-arrow)).
- **The curtains wait for nothing.** `FADEIN` (`0x60360`) and `FADEOUT`
  (`0x60560`) pop their three arguments and never divide the duration by
  the band count or poll the timer, where R109's (`0x74a42`, `0x74c79`)
  do both; each pass marks its band and presents, and the next follows at
  once. Read off the handlers — the division is the one `idivl` R109 has
  and R78 has not — and carried as a capability, so a fade here is over
  within a frame ([transitions](transitions.md#timing)). This game is
  also the one that reaches `FADEIN`'s mode 2, the mode-1 curtain over
  `WHITEBOX`'s box, for its information book
  ([transitions](transitions.md#the-mode-argument)).
- **`GET` of a block no container holds** copies from linear address 0 — the
  interrupt vector table — on both builds, after the resource layer has put
  two error boxes up (messages 24 and 37 of the table at `0xb5ffc`);
  motionvm leaves the memory as it was, shows no box and counts the miss
  ([departures](../../departures.md#the-virtual-machine)).
- **`->RSCPATH ( path$ slot -- )`** (`0x4cf70`) copies a path into the
  container table (`0xc8c3c`, `0x78` a record, the path at `+0x20`) for a
  slot of 0 to 5. Both builds have the word; only this game's bootstrap uses
  it ([other files](../../games/checker/other-files.md#systemrsc)).
- The timer words, the sample words, the scroll slide, `WHITEBOX`,
  `GIVEDATE` and `TEXT->PRINTER` are the words this game reaches and Dunkle
  Schatten 2 does not; they were read on R78 first and found again on R109
  where R109 has them ([audio](audio.md), [screens](screens.md)).

## What was not read

Whatever Checker 2000 does not reach was not read on R78: the dialogue and
interaction machines, walking, the native animation runtime's handlers, the
FM driver's tables — all of which the other pages describe on R109, and
which R78 shares as far as the game's scripts show. Where a page says
"both builds", it was read on both; where it names an R109 address alone,
the R78 counterpart is unread.

## See also

- [ENGINE.EXE](engine-exe.md) — the later build, and the LE format
- [Kernel words](../vm/kernel-words.md), [Threaded code](../vm/threaded-code.md) — the tables and the ordinals
- [Checker 2000](../../games/checker/README.md) — the game that ships this build
- [Verification](../../verification.md) — what of it was held against the original
