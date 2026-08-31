[← Documentation index](../../../README.md)

# Module 216 — The In-Game BBS

*Dunkle Schatten 2 — this page describes one of the game's own script modules. The engine it runs on is documented under [MOTION 32-bit](../../../README.md#motion-32-bit).*

Module 216 (286 words) implements the bulletin-board terminal — the
"network" of the title — and is the most elaborate location module in the
game. Its `LTMANAGER` alone is 6221 cells.

## Menus

The menu items (`M_FILES`, `M_BOARDS`, `M_SYSOP`, `M_MAIL`, `M_LIST`,
`M_JUMP`, `M_READ`, `M_WRITE`, `M_HELP`, `M_BACK`, `M_LOGOFF`, `M_JA`,
`M_NEIN`) are constants 0–12 indexing five parallel tables: normal
sprite, highlighted sprite, description sprite, pixel width, and hot-key
code (F, B, S, M, L, J, R, W, H, B, O, J, N). A menu bar is built
imperatively — `NEWMEN` resets, one `ADDMEN` per entry advances a
running x offset — and `HOTMEN`/`UNHOTMEN` move the highlight;
`SAFEMEN`/`RESTMEN` push and pop an entire bar so submenus can replace
it and restore it exactly.

## Keys

The terminal is worked from the keyboard, and the manual says so (text bank
006, entry 60): *"danach kannst Du Dich mit den Pfeiltasten durch die Menüs
bewegen und diese mit RETURN anwählen"*. `LTMANAGER` dispatches at `0x0c21c`,
behind `_EDVMODE @ 3 =` and the cursor animation standing still:

| Key | Also | Does |
|---|---|---|
| ← | `4` | `HOTMEN` one entry left, round the end |
| → | `6` | `HOTMEN` one entry right |
| ↑ | `8` | `SELUP`, only while `_SELFLAG` is set |
| ↓ | `2` | `SELDOWN`, same gate |
| Return | Space | take what is highlighted |
| a letter | | jump the highlight to the entry whose hot-key it is, tested against `?MENHOT` and `?MENHOT + 32` so either case does |

The four cursor codes are 331, 333, 328 and 336 — `0x100` over their scan codes,
which is how [`?KEY`](../../../motion32/vm/kernel-words.md) answers a key that carries no
character. The digits are the same four directions a second time — the numeric
keypad with Num Lock on.

## Screens and the modem illusion

Terminal screens are lists of text-row sprites (ids 4018–4209)
registered via `NEWSCR` … `INITFADE` … `ENDSCR` (up to 16 line
descriptors each); `DOFADE` then reveals them **one text row at a
time** and `CLSCR` erases them the same way — faking a slow modem
redraw. About twenty screens exist (login, main, file areas, mail,
search, news, help, sysop, …). Below the menu sits a generic selection
list on an 8-pixel text grid (`NEWSEL`, `SELUP`/`SELDOWN`, a highlight
bar).

### How the wipe is made

Every change of screen runs the same three phases (`0x0a0bc` is one of a
dozen):

```
phase 0:  HIDSCR   \ all sixteen row descriptors off, then 15 ->LTWAIT
phase 1:  CLSCR    \ a black bar over one row every three frames, up to _LASTL
phase 2:  DOFADE   \ the bars off again, one row every five frames
```

It reads as a slow redraw only because **hiding a descriptor does not erase
it** (see [Screens](../../../motion32/engine/screens.md#the-drawn-buffer)). The rows stay on
the screen after `HIDSCR`, and the bars are what takes them away.

The two halves are built for exactly that, in module 316:

| | built at | Level | `SDAUTOBUF` |
|---|---|---|---|
| `_BG`, the monitor (sprite 4009) | `0x00120` | 2 | no |
| `SCRDESC[0..15]`, the text rows | `0x00f60` | 8 | **no** |
| `BDESC[0..42]`, the bars (sprite 4148) | `0x01020` | 12 | **yes** |

A row carries no buffer, so switching it off leaves it standing; a bar carries
one, so taking it down puts back what was under it — which is how `DOFADE`
uncovers the *new* screen a row at a time.

And the bar is not a black rectangle over a picture: sprite 4148 is 509×8
pixels of **index 9**, and index 9 is precisely what the monitor's screen area
in sprite 4009 is painted in. Over the background it is pixel for pixel
invisible. It can only erase text.

## Login as animation

The player never types. Every possible user name and password has a
prerecorded **typing animation**: a frame list plus a matching x-advance
table that walks the blinking cursor along behind the letters. Which
name/password gets "typed" is chosen from clickable dialogue options
(up to three lines, options disappear once used). The login phase
machine gives the player three attempts (failure swaps in a "login
incorrect" line sprite), and the correct choices are gated by story
flags (`_K1LOGIN`, `_KSIEGNEW`, `_KEMAIL`); `_BOXLOCKED` and
`_COMPLOCKED` start at 1 — both systems begin locked. A separate
sequence handles a web-page password later in the same handler.

## Verb lockout

While the terminal is active, `DO_TALK`, `DO_LEAVE`, and `DO_HANDLE`
are empty — the normal verb system is deliberately disabled; all
interaction runs through the terminal's own menus.

## See also

- [Module map](../module-map.md)
- [Dialogue](dialogue.md)
- [Objects and flags](objects-and-flags.md) — the lock flags
