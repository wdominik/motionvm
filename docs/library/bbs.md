[← Documentation index](../README.md)

# Module 216 — The In-Game BBS

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

## Screens and the modem illusion

Terminal screens are lists of text-row sprites (ids 4018–4209)
registered via `NEWSCR` … `INITFADE` … `ENDSCR` (up to 16 line
descriptors each); `DOFADE` then reveals them **one text row at a
time** and `CLSCR` erases them the same way — faking a slow modem
redraw. About twenty screens exist (login, main, file areas, mail,
search, news, help, sysop, …). Below the menu sits a generic selection
list on an 8-pixel text grid (`NEWSEL`, `SELUP`/`SELDOWN`, a highlight
bar).

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

- [Module map](../engine/module-map.md)
- [Dialogue](dialogue.md)
- [Objects and flags](objects-and-flags.md) — the lock flags
