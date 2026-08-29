[← Documentation index](../../README.md)

# Text Tables

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway and in `BMZ.EXE` with Hilfe für Amajambere, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit-ds2).*

A TXT item is a table of NUL-terminated strings with a 16-bit offset per
string:

```
u16 n                 ; number of strings
u16 off[n]            ; offset of each string, relative to the string area
; string area begins at 2 + 2*n
; n NUL-terminated CP437 strings
```

`off[i]` counts from the **start of the string area**, not from the start of
the item; `off[0]` is 0 in every table. The offsets of all 96 tables of
Die Enviro-Kids greifen ein
are monotone, and no string crosses its table's end.

This differs from the 32-bit engine's table in two places: there the count
is a `u32` and the offsets are relative to the item start — see
[Text tables (MOTION 32-bit)](../../motion32/formats/text-tables.md).

## Strings

- Encoding is CP437; the German umlauts and `ß` appear as their CP437 bytes
  (`"Enviro-Kids"`, `Städtchen`, `grüne Lunge`).
- Line breaks inside a string are a single `0x0A`. 2199 of the 3508 strings
  of Die Enviro-Kids greifen ein
  strings contain one; none contains `0x0D`.
- Many tables start with a blank or one-space string: entry 0 of a table
  reads as "no text".

## How the scripts use them

`SDTB ( table -- )` selects the table on the current descriptor, `SDTXT
( n -- )` the string. The intro does `14 SDTB` then `1 SDTXT` and shows
table 14's *second* string — the town briefing *"Viele Leute in Waldbach
leben vom Tourismus …"* — which is consistent with the 32-bit engine's
1-based `SDTXT`. Table 20 carries the character switch lines *"Jetzt bin ich
Eva."* / *"Jetzt bin ich Maik."*; table 11 (235 strings) and tables 20–22
(257, 243 and 160) are the big dialogue tables.

## Open questions

- Whether `SDTXT 0` is legal and what it shows — the scripts never pass 0.

## See also

- [The DATA container](data-container.md) — the TXT segment (ids 0–99)
- [Resource inventory](../../games/enviro/inventory.md) — string counts per table
- [Descriptors and screens](../engine/descriptors.md) — `SDTB`, `SDTXT`, `SDFNT`, `SDCOL`
