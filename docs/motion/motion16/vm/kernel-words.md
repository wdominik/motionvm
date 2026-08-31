[← Documentation index](../../README.md)

# Kernel Words

*MOTION 16-bit — the engine as shipped in `ENVIRO.EXE` with Die Enviro-Kids greifen ein, in `HPPLAY.EXE` with Jeff Jet - Abenteuer InfoHighway, in `BMZ.EXE` with Hilfe für Amajambere and in `LL.EXE` with Victor Loomes – Das Spiel, which are older builds of the same player. What is measured here is measured on Die Enviro-Kids greifen ein's files unless a sentence names another game. The 32-bit engine is documented under [MOTION 32-bit](../../README.md#motion-32-bit).*

The 16-bit kernel registers its words in two arrays in `ENVIRO.EXE`'s data,
each a list of 8-byte entries

```c
struct { char far *name; void (far *handler)(); }   /* u16 off, u16 seg, u16 off, u16 seg */
```

terminated by an entry whose name points at an empty string and whose
handler is `0000:0000`:

| File offset | Entries | Ordinals | Contents |
|---|---:|---|---|
| `0x2064e` | 82 | 1–82 | Core Forth, the compiler runtimes (`_PutLit`, `_CheckIf`, …), the module operators, `GET`/`PUT` |
| `0x1f9f6` | 151 | 105–255 | Domain words: video, mouse, screens, descriptors, fonts, buffers, walking, inventory, sound |

233 words. The name strings sit together at file `0x1fec5`–`0x20aae`.
**Ordinals are 1-based and contiguous within a table**: `ordinal = index +
1` for the core table, `ordinal = index + 105` for the domain table; the
cell in threaded code is `0x8000 | ordinal`. Ordinals 83–104 never occur in
a module — the gap where the authoring compiler's words sit in the 32-bit
kernel, absent from this player build. Table order departs from the 32-bit
kernel's from index 11 on, so **no ordinal carries over**; names do.

Handlers are far functions; the `segment:offset` below is of the load
image, which starts at file offset `0x3200`, and the file offset follows in
parentheses (see [ENVIRO.EXE](../engine/enviro-exe.md)).

**Of the 233 words, the script modules of Die Enviro-Kids greifen ein use 151.**
The "Sites" column is
the number of cells in the 65 modules that name the word, counted by walking
every body with the inline operands skipped; a dash is an unused word. A
reimplementation needs the 151; the other 82 can stay stubs for this game.

## The other builds' tables

Four builds ship, and each is a prefix of the next by deletion alone —
nothing is ever added going forward:

| Build | Core | Domain | Total | Domain base | Missing against `ENVIRO.EXE` |
|---|---:|---:|---:|---:|---|
| `LL.EXE` | 80 at `0x14feb` | 124 at `0x146ca` | 204 | **102** | 27 domain words, and the core table's last two |
| `HPPLAY.EXE` | 82 at `0x20236` | 146 at `0x1f42e` | 228 | 105 | `SETMOUSEX/Y/LB/RB` (124–127) and `?SAMPLE` (255) |
| `BMZ.EXE` | 82 at `0x2067a` | 150 at `0x1f7e6` | 232 | 105 | `?SAMPLE` (255) |
| `ENVIRO.EXE` | 82 | 151 | 233 | 105 | — |

The core table is name for name and order for order the same in all four, as
far as each has it: `LL.EXE` ends two words earlier, at `_PutLit`, where the
later builds append `_PutStringAdr` and `$->`.

**Where the domain table starts is the build's, not the format's.** The player
hands ordinals out in the order it registers words, and it registers three
runs: the core table, then a run of `DUMMY#F0R3i` placeholders, then the
domain table. So the first domain word binds at `core + placeholders + 1`.
The three later builds register 22 placeholders behind 82 core words and start
at 105; `LL.EXE` registers 21 behind 80 and starts at **102** (`0afe:009a`
against `140e:002f`, `140a:0039` and `13d9:002f`). The gap between the two
tables is not unused numbering — it is placeholder words. The count is read out
of the binary rather than assumed; see [LL.EXE](../engine/ll-exe.md).

Where the deletions sit is what decides whether ordinals move. `HPPLAY.EXE`
lacks four words *inside* the domain table, so **every domain word from ordinal
124 up is four below its namesake there** — `DOWALK` 239 against 243,
`PLAYSAMPLE` 250 against 254. `BMZ.EXE` lacks only the appended `?SAMPLE`, so
**its ordinals are exactly those of Die Enviro-Kids greifen ein**, 105 to 254, and
only 255 is absent. The ordinals in the tables below are that game's, which
makes them `BMZ.EXE`'s too and
not `HPPLAY.EXE`'s.

A table read from one build and applied to another binds without complaint. The
loud case is `HPPLAY.EXE`, which would then name the wrong handler from ordinal
124 on; the quiet one is `BMZ.EXE`, which would be named correctly throughout
and hold a word at 255 that is not there. Either way the binding is scanned out
of the binary the game ships with ([HPPLAY.EXE](../engine/hpplay-exe.md),
[BMZ.EXE](../engine/bmz-exe.md), [LL.EXE](../engine/ll-exe.md)). `LL.EXE` is
the loudest of the three cases, because its whole domain table sits three
ordinals below the others'.

What each game asks for: the modules of Die Enviro-Kids greifen ein use 151 of its
233, Jeff Jet's 150 of its 228 — the same set less `-FONT`, `SDBLK` and
`SDH%SHR`, plus
`GDOX` and `GDOY` — Hilfe für Amajambere's 143 of its 232, adding `&` and
`GFXVFLIP` to what the other two use between them, and Victor Loomes' 100 of
its 124 domain words, which adds eight the later games never call: `?INSIDE`,
`CROUTE`, `GSCRPOS`, `SETSHADE`, `SETCYCLE`, `SYSFC`, `SYSBC` and `_POOR`,
plus the core table's `I'`.

## Core table — ordinals 1–82

| Ordinal | Name | Sites | Handler |
|---:|---|---:|---|
| 1 | `##` | 673 | `12c8:10e2` (file `0x16f62`) |
| 2 | `+` | 959 | `1977:000c` (file `0x1c97c`) |
| 3 | `-` | 113 | `1977:001d` (file `0x1c98d`) |
| 4 | `*` | 110 | `12c8:000e` (file `0x15e8e`) |
| 5 | `/` | 21 | `12c8:0030` (file `0x15eb0`) |
| 6 | `MOD` | — | `12c8:0072` (file `0x15ef2`) |
| 7 | `SWAP` | 63 | `1977:0055` (file `0x1c9c5`) |
| 8 | `DUP` | 339 | `1977:002e` (file `0x1c99e`) |
| 9 | `OVER` | 12 | `1977:011c` (file `0x1ca8c`) |
| 10 | `ROT` | 7 | `1977:012d` (file `0x1ca9d`) |
| 11 | `DROP` | 603 | `12c8:0092` (file `0x15f12`) |
| 12 | `:` | — | `12c8:009c` (file `0x15f1c`) |
| 13 | `VAR` | — | `12c8:00d4` (file `0x15f54`) |
| 14 | `@` | 2395 | `1977:003f` (file `0x1c9af`) |
| 15 | `!` | 1536 | `12c8:00e3` (file `0x15f63`) |
| 16 | `ALLOT` | — | `12c8:0105` (file `0x15f85`) |
| 17 | `CONST` | — | `12c8:00d9` (file `0x15f59`) |
| 18 | `=` | 1793 | `12c8:03f2` (file `0x16272`) |
| 19 | `<` | 209 | `12c8:0464` (file `0x162e4`) |
| 20 | `>` | 61 | `12c8:043e` (file `0x162be`) |
| 21 | `>=` | 63 | `1977:0098` (file `0x1ca08`) |
| 22 | `<=` | 51 | `1977:00ba` (file `0x1ca2a`) |
| 23 | `0=` | — | `12c8:048a` (file `0x1630a`) |
| 24 | `!=` | 28 | `12c8:0418` (file `0x16298`) |
| 25 | `NOT` | 294 | `12c8:04e1` (file `0x16361`) |
| 26 | `AND` | 289 | `1977:0068` (file `0x1c9d8`) |
| 27 | `OR` | 215 | `12c8:04f5` (file `0x16375`) |
| 28 | `>>` | — | `12c8:0562` (file `0x163e2`) |
| 29 | `<<` | — | `12c8:057d` (file `0x163fd`) |
| 30 | `~` | — | `12c8:051f` (file `0x1639f`) |
| 31 | `&` | — | `12c8:0530` (file `0x163b0`) |
| 32 | `\|` | 10 | `12c8:0549` (file `0x163c9`) |
| 33 | `>R` | 10 | `12c8:0598` (file `0x16418`) |
| 34 | `R>` | 13 | `12c8:05b5` (file `0x16435`) |
| 35 | `I` | 36 | `12c8:05d2` (file `0x16452`) |
| 36 | `I'` | — | `12c8:05ea` (file `0x1646a`) |
| 37 | `_PutAdr` | 792 | `1977:00dc` (file `0x1ca4c`) |
| 38 | `_PutConst` | 310 | `12c8:00a6` (file `0x15f26`) |
| 39 | `_LoopStart` | 6 | `12c8:01b8` (file `0x16038`) |
| 40 | `_LoopEnd` | 6 | `12c8:01f3` (file `0x16073`) |
| 41 | `_CheckIf` | 2896 | `12c8:02b4` (file `0x16134`) |
| 42 | `_CheckEIf` | 298 | `12c8:02e0` (file `0x16160`) |
| 43 | `_ChElseDup` | — | `12c8:0343` (file `0x161c3`) |
| 44 | `_CheckElse` | 2084 | `12c8:032c` (file `0x161ac`) |
| 45 | `KEY` | 3 | `12c8:061c` (file `0x1649c`) |
| 46 | `?KEY` | 7 | `12c8:063c` (file `0x164bc`) |
| 47 | `_AddLoop` | — | `12c8:025c` (file `0x160dc`) |
| 48 | `_ULoopEnd` | — | `12c8:0225` (file `0x160a5`) |
| 49 | `LEAVE` | — | `12c8:029a` (file `0x1611a`) |
| 50 | `_Until` | 2 | `12c8:0379` (file `0x161f9`) |
| 51 | `_LoopBreak` | 9 | `12c8:03c6` (file `0x16246`) |
| 52 | `_Repeat` | 9 | `12c8:03aa` (file `0x1622a`) |
| 53 | `,` | — | `12c8:010a` (file `0x15f8a`) |
| 54 | `CREATE` | — | `12c8:00de` (file `0x15f5e`) |
| 55 | `<N>` | — | `12c8:07d5` (file `0x16655`) |
| 56 | `'` | — | `12c8:07b0` (file `0x16630`) |
| 57 | `EXECUTE` | 13 | `12c8:0fe1` (file `0x16e61`) |
| 58 | `=>START` | — | `12c8:08d7` (file `0x16757`) |
| 59 | `=>END` | — | `12c8:09bf` (file `0x1683f`) |
| 60 | `=>TABLES` | — | `12c8:102d` (file `0x16ead`) |
| 61 | `=>INFO` | — | `12c8:1032` (file `0x16eb2`) |
| 62 | `=>ERASE` | 27 | `12c8:0a8a` (file `0x1690a`) |
| 63 | `=>EXIST` | 2 | `12c8:110c` (file `0x16f8c`) |
| 64 | `=>PUTAS` | 3 | `12c8:1169` (file `0x16fe9`) |
| 65 | `=>GETAS` | 1 | `12c8:124e` (file `0x170ce`) |
| 66 | `=>PUT` | — | `1400:04a7` (file `0x176a7`) |
| 67 | `=>GET` | 25 | `1400:04ac` (file `0x176ac`) |
| 68 | `=>AT` | — | `1400:0815` (file `0x17a15`) |
| 69 | `=>VALID` | — | `12c8:0a24` (file `0x168a4`) |
| 70 | `=>RESET` | — | `12c8:0a68` (file `0x168e8`) |
| 71 | `PUT` | 3 | `1400:0829` (file `0x17a29`) |
| 72 | `GET` | 6 | `12c8:0de3` (file `0x16c63`) |
| 73 | `ENDEBUG` | — | `12c8:1012` (file `0x16e92`) |
| 74 | `DISDEBUG` | — | `12c8:101d` (file `0x16e9d`) |
| 75 | `WORD` | — | `12c8:1028` (file `0x16ea8`) |
| 76 | `EMIT` | 5 | `12c8:1037` (file `0x16eb7`) |
| 77 | `.` | 22 | `12c8:105a` (file `0x16eda`) |
| 78 | `_PutString` | 2 | `12c8:10be` (file `0x16f3e`) |
| 79 | `RANDOM` | 188 | `12c8:10f6` (file `0x16f76`) |
| 80 | `_PutLit` | 5230 | `1977:00fd` (file `0x1ca6d`) |
| 81 | `_PutStringAdr` | 162 | `12c8:134c` (file `0x171cc`) |
| 82 | `$->` | — | `12c8:1382` (file `0x17202`) |

`##` (ordinal 1) is the return. `VAR`, `CONST`, `ALLOT`, `:`, `,`, `CREATE`,
`WORD`, `'` and the `=>START`/`=>END`/`=>TABLES` family are in the table
but never called — compile-time and authoring words left in a player that
compiles nothing. `PUT` and `GET` (71, 72) move blocks between the container
and Forth memory; `=>EXIST` (63) is how the game probes its save slots.

## Domain table — ordinals 105–255

| Ordinal | Name | Sites | Handler |
|---:|---|---:|---|
| 105 | `TOGFX` | 1 | `05f1:0104` (file `0x9214`) |
| 106 | `GFXTO` | 1 | `05f1:0167` (file `0x9277`) |
| 107 | `SETSHADE` | — | `05f1:1fbd` (file `0xb0cd`) |
| 108 | `NEWANIM` | 1 | `05f1:000a` (file `0x911a`) |
| 109 | `DELAY` | 2 | `05f1:17f1` (file `0xa901`) |
| 110 | `QUITANIM` | 4 | `05f1:1808` (file `0xa918`) |
| 111 | `ANIMPLAY` | 2 | `016a:04fd` (file `0x4d9d`) |
| 112 | `SHBLOCK` | — | `05f1:01a8` (file `0x92b8`) |
| 113 | `SETPAL` | 11 | `05f1:01ff` (file `0x930f`) |
| 114 | `SETCYCLE` | — | `08f4:112a` (file `0xd26a`) |
| 115 | `ATMOUSE` | 6 | `05f1:028a` (file `0x939a`) |
| 116 | `XATMOUSE` | 3 | `05f1:0343` (file `0x9453`) |
| 117 | `MOUSEX` | 8 | `05f1:0469` (file `0x9579`) |
| 118 | `MOUSEY` | 7 | `05f1:0484` (file `0x9594`) |
| 119 | `MOUSEXY` | — | `05f1:040f` (file `0x951f`) |
| 120 | `MOUSELK` | 13 | `05f1:0433` (file `0x9543`) |
| 121 | `MOUSERK` | 11 | `05f1:044e` (file `0x955e`) |
| 122 | `SHOWMOUSE` | 1 | `14ee:0874` (file `0x18954`) |
| 123 | `HIDEMOUSE` | 1 | `14ee:094b` (file `0x18a2b`) |
| 124 | `SETMOUSEX` | — | `05f1:2f8d` (file `0xc09d`) |
| 125 | `SETMOUSEY` | — | `05f1:2fbd` (file `0xc0cd`) |
| 126 | `SETMOUSELB` | — | `05f1:2fed` (file `0xc0fd`) |
| 127 | `SETMOUSERB` | — | `05f1:3013` (file `0xc123`) |
| 128 | `GFXSTAT` | — | `05f1:1fd2` (file `0xb0e2`) |
| 129 | `XGFXSTAT` | 1 | `05f1:2033` (file `0xb143`) |
| 130 | `GFXSTAT+` | — | `05f1:1ff6` (file `0xb106`) |
| 131 | `XGFXSTAT+` | 18 | `05f1:211a` (file `0xb22a`) |
| 132 | `GFXVFLIP` | — | `05f1:21f2` (file `0xb302`) |
| 133 | `XGFXVFLIP` | 14 | `05f1:221b` (file `0xb32b`) |
| 134 | `TXTSTAT` | 3 | `05f1:21a0` (file `0xb2b0`) |
| 135 | `XTXTSTAT` | — | `05f1:21bf` (file `0xb2cf`) |
| 136 | `NEWSCREEN` | 3 | `05f1:049f` (file `0x95af`) |
| 137 | `SCRSIZE` | 3 | `05f1:065e` (file `0x976e`) |
| 138 | `SCRPOS` | 4 | `05f1:068b` (file `0x979b`) |
| 139 | `SCRX` | 42 | `05f1:06f9` (file `0x9809`) |
| 140 | `SCRY` | — | `05f1:0715` (file `0x9825`) |
| 141 | `GSCRSIZE` | — | `05f1:07a2` (file `0x98b2`) |
| 142 | `GSCRPOS` | — | `05f1:07cd` (file `0x98dd`) |
| 143 | `GSCRX` | 29 | `05f1:06ca` (file `0x97da`) |
| 144 | `GSCRY` | 20 | `05f1:06e1` (file `0x97f1`) |
| 145 | `SCRVSIZE` | 3 | `05f1:051e` (file `0x962e`) |
| 146 | `SCRFVSIZE` | 3 | `05f1:0557` (file `0x9667`) |
| 147 | `SCRVPOS` | 3 | `05f1:0740` (file `0x9850`) |
| 148 | `GSCRVSIZE` | — | `05f1:05d8` (file `0x96e8`) |
| 149 | `GSCRVPOS` | — | `05f1:0603` (file `0x9713`) |
| 150 | `GSCRVX` | — | `05f1:062e` (file `0x973e`) |
| 151 | `GSCRVY` | — | `05f1:0646` (file `0x9756`) |
| 152 | `SCRSTAT` | — | `05f1:07f7` (file `0x9907`) |
| 153 | `DRAWSCR` | — | `05f1:0822` (file `0x9932`) |
| 154 | `ERASESCR` | 1 | `05f1:0868` (file `0x9978`) |
| 155 | `ACTSCR` | 7 | `05f1:08c7` (file `0x99d7`) |
| 156 | `REMSCR` | 1 | `05f1:08d4` (file `0x99e4`) |
| 157 | `SCRCTRL` | 2 | `05f1:0a8f` (file `0x9b9f`) |
| 158 | `SCRACT` | — | `05f1:094a` (file `0x9a5a`) |
| 159 | `SCRINACT` | — | `05f1:09b9` (file `0x9ac9`) |
| 160 | `GSCRACT` | 1 | `05f1:0a5e` (file `0x9b6e`) |
| 161 | `FADEOUT` | 17 | `05f1:2827` (file `0xb937`) |
| 162 | `FADEIN` | 34 | `05f1:29e4` (file `0xbaf4`) |
| 163 | `->SCRX` | 1 | `08f4:0009` (file `0xc149`) |
| 164 | `->SCRY` | 1 | `08f4:06ad` (file `0xc7ed`) |
| 165 | `FRESHSCREEN` | — | `08f4:09ca` (file `0xcb0a`) |
| 166 | `ACTDESC` | 55 | `05f1:0a82` (file `0x9b92`) |
| 167 | `NEWDESC` | — | `05f1:0aa8` (file `0x9bb8`) |
| 168 | `NEWSETDESC` | 3 | `05f1:0ac8` (file `0x9bd8`) |
| 169 | `KILLNDESC` | 3 | `05f1:0c2a` (file `0x9d3a`) |
| 170 | `SDX` | 52 | `05f1:0dd1` (file `0x9ee1`) |
| 171 | `SDY` | 43 | `05f1:0f34` (file `0xa044`) |
| 172 | `SDPOS` | — | `05f1:105d` (file `0xa16d`) |
| 173 | `SDLEV` | 20 | `05f1:1018` (file `0xa128`) |
| 174 | `GDX` | 11 | `05f1:1638` (file `0xa748`) |
| 175 | `GDY` | 17 | `05f1:164f` (file `0xa75f`) |
| 176 | `GDLEV` | — | `05f1:1667` (file `0xa777`) |
| 177 | `SDACTIVE` | 132 | `05f1:10ea` (file `0xa1fa`) |
| 178 | `SDINACTIVE` | 222 | `05f1:10c9` (file `0xa1d9`) |
| 179 | `GDACTIVE` | 4 | `05f1:10fd` (file `0xa20d`) |
| 180 | `GDNR` | — | `05f1:0c69` (file `0x9d79`) |
| 181 | `SDBL` | 6 | `05f1:11d6` (file `0xa2e6`) |
| 182 | `SDSPR` | 358 | `05f1:129a` (file `0xa3aa`) |
| 183 | `SDWORD` | 78 | `05f1:1335` (file `0xa445`) |
| 184 | `SDWAIT` | 298 | `05f1:135c` (file `0xa46c`) |
| 185 | `GDBL` | 4 | `05f1:1682` (file `0xa792`) |
| 186 | `GDSPR` | 55 | `05f1:16b1` (file `0xa7c1`) |
| 187 | `SDTXT` | 27 | `05f1:0ca2` (file `0x9db2`) |
| 188 | `GDTXT` | 1 | `05f1:0ce0` (file `0x9df0`) |
| 189 | `GDTXTLEN` | 1 | `05f1:0cfc` (file `0x9e0c`) |
| 190 | `SDTB` | 14 | `05f1:0d68` (file `0x9e78`) |
| 191 | `GDTB` | 1 | `05f1:0d50` (file `0x9e60`) |
| 192 | `SDCOL` | 16 | `05f1:0d8f` (file `0x9e9f`) |
| 193 | `SDFNT` | 13 | `05f1:1375` (file `0xa485`) |
| 194 | `SDCEN` | 12 | `05f1:13b7` (file `0xa4c7`) |
| 195 | `SDBLK` | 2 | `05f1:1515` (file `0xa625`) |
| 196 | `SDNORM` | — | `05f1:1616` (file `0xa726`) |
| 197 | `SDVCEN` | 12 | `05f1:1537` (file `0xa647`) |
| 198 | `GDWIDTH` | 4 | `05f1:1705` (file `0xa815`) |
| 199 | `GDHEIGHT` | 4 | `05f1:177b` (file `0xa88b`) |
| 200 | `SDTDT` | 13 | `05f1:0c78` (file `0x9d88`) |
| 201 | `DEFTDT` | 1 | `05f1:2ee7` (file `0xbff7`) |
| 202 | `SD%SHR` | 52 | `05f1:227b` (file `0xb38b`) |
| 203 | `SD%COSHR` | — | `05f1:22a1` (file `0xb3b1`) |
| 204 | `SDH%SHR` | 7 | `05f1:23f2` (file `0xb502`) |
| 205 | `SDV%SHR` | 2 | `05f1:246f` (file `0xb57f`) |
| 206 | `SFT` | 2 | `05f1:1813` (file `0xa923`) |
| 207 | `+FONT` | 3 | `05f1:1860` (file `0xa970`) |
| 208 | `-FONT` | 1 | `05f1:1900` (file `0xaa10`) |
| 209 | `RESETFONT` | — | `05f1:195e` (file `0xaa6e`) |
| 210 | `BUFON` | 1 | `05f1:19a8` (file `0xaab8`) |
| 211 | `SDBUF` | 108 | `05f1:1ae1` (file `0xabf1`) |
| 212 | `SETBUF` | 129 | `05f1:1b23` (file `0xac33`) |
| 213 | `RESETBUF` | — | `05f1:1ce6` (file `0xadf6`) |
| 214 | `KILLNBUF` | 1 | `05f1:1e9b` (file `0xafab`) |
| 215 | `SDCX` | 45 | `05f1:24ec` (file `0xb5fc`) |
| 216 | `SDCY` | 39 | `05f1:255d` (file `0xb66d`) |
| 217 | `SDOX` | 2 | `05f1:2637` (file `0xb747`) |
| 218 | `SDOY` | 12 | `05f1:25ce` (file `0xb6de`) |
| 219 | `GDCX` | 13 | `05f1:26a0` (file `0xb7b0`) |
| 220 | `GDCY` | 1 | `05f1:2711` (file `0xb821`) |
| 221 | `GDOX` | — | `05f1:27eb` (file `0xb8fb`) |
| 222 | `GDOY` | — | `05f1:2782` (file `0xb892`) |
| 223 | `SYSFC` | — | `05f1:2c94` (file `0xbda4`) |
| 224 | `SYSBC` | — | `05f1:2ca1` (file `0xbdb1`) |
| 225 | `REQUEST` | — | `05f1:2cae` (file `0xbdbe`) |
| 226 | `?INSIDE` | — | `0a40:1a3d` (file `0xf03d`) |
| 227 | `?XINSIDE` | 2 | `0a40:1a9b` (file `0xf09b`) |
| 228 | `CROUTE` | — | `0a40:10d0` (file `0xe6d0`) |
| 229 | `STEPMULTI` | 4 | `0a40:18ba` (file `0xeeba`) |
| 230 | `STARTTUNE` | 16 | `05f1:2e79` (file `0xbf89`) |
| 231 | `ENDTUNE` | 1 | `05f1:2edd` (file `0xbfed`) |
| 232 | `PUTANIM` | 3 | `08f4:032d` (file `0xc46d`) |
| 233 | `GETANIM` | 1 | `08f4:09de` (file `0xcb1e`) |
| 234 | `_POOR` | — | `016a:0000` (file `0x48a0`) |
| 235 | `GAMEREQUEST` | — | `05f1:2e22` (file `0xbf32`) |
| 236 | `CHTEXT` | — | `08f4:1239` (file `0xd379`) |
| 237 | `PRINTFORM` | — | `0a29:0030` (file `0xd4c0`) |
| 238 | `SAVEADRESS` | — | `0a29:0008` (file `0xd498`) |
| 239 | `INITPRINT` | — | `0a29:000d` (file `0xd49d`) |
| 240 | `PRINTMANOR` | — | `0a29:002b` (file `0xd4bb`) |
| 241 | `MANORINFO` | — | `0a29:0012` (file `0xd4a2`) |
| 242 | `XGFXSAMPLE` | — | `05f1:216b` (file `0xb27b`) |
| 243 | `DOWALK` | 12 | `0a40:1c5f` (file `0xf25f`) |
| 244 | `MOUSEINFO` | 2 | `0a40:2a15` (file `0x10015`) |
| 245 | `DOORDER` | 2 | `0d34:201f` (file `0x1255f`) |
| 246 | `CCALCINV` | 1 | `0d34:351d` (file `0x13a5d`) |
| 247 | `ADDTOINV` | 1 | `0d34:3774` (file `0x13cb4`) |
| 248 | `SUBFROMINV` | 1 | `0d34:3858` (file `0x13d98`) |
| 249 | `?INVINCL` | 24 | `0d34:37e0` (file `0x13d20`) |
| 250 | `FREEZESCR` | 1 | `0d34:38eb` (file `0x13e2b`) |
| 251 | `UNFREEZESCR` | 2 | `0d34:38f6` (file `0x13e36`) |
| 252 | `ADDMESSPIPE` | 12 | `0d34:3901` (file `0x13e41`) |
| 253 | `GIVEDATE` | — | `0d34:3a01` (file `0x13f41`) |
| 254 | `PLAYSAMPLE` | — | `1696:0357` (file `0x19eb7`) |
| 255 | `?SAMPLE` | — | `1696:03b2` (file `0x19f12`) |

## What the 32-bit kernel does not have

Fourteen of the used names have no counterpart in the 32-bit kernel of
`ENGINE.EXE` V0.06.06/R109: `##`, `.`, `EMIT`, `KEY`, `SFT`, `SCRCTRL`,
`=>EXIST`, `_PutString`, `NEWANIM`, `BUFON`, `->SCRX`, `->SCRY`, `KILLNBUF`,
`-FONT`. The 32-bit engine has `EXIST` where this one has `=>EXIST`, and
installs its frame handler through `CTRL` where this one uses `SCRCTRL`
with a word id. Conversely the 32-bit engine's dialogue machine
(`CALCDIALOG`, `SDIAL`, …), `SCANCUT`, `LOADLBM`, `TURNTO`, `CALCROUTE`,
`SETRES` and the hicolor path have no 16-bit counterpart.

## Words that matter more here than in the 32-bit game

`SETBUF` (129 sites), `SDBUF` (108) and `BUFON` (1) build the intro and
every person sprite: the 16-bit game draws through off-screen buffers,
where the 32-bit game leaves the same words unused or inert. See
[Off-screen buffers](../engine/buffers.md).

## What is read

The stack helpers and the calling convention (data stack pointer at
`DS:0x8160`, `pop` at `12c8:0652`, `push` at `12c8:0663`, the current
descriptor through `016a:04ca`, a script address turned into a pointer by
`1400:0490`, a word run by id through `1400:028f`), the interpreter's arena
and word table ([execution model](execution-model.md)), the frame loop and
`DELAY` ([boot and frame loop](../engine/game-loop.md)), the buffers
([off-screen buffers](../engine/buffers.md)), and the walk, the inventory,
the order machine, `MOUSEINFO`, `?XINSIDE`, `ADDMESSPIPE`, `SCRX`/`SCRPOS`
and `->SCRX`, `NEWDESC`/`NEWSETDESC`/`ACTDESC`/`KILLNDESC` and the
drawer's two paths, the `GFXSTAT`/`TXTSTAT` family, `SD%SHR` and the
conversation machine — `calc_dialog` at `0d34:0ed9`, the `DOORDER` modes
12–18, verbs 5–7 ([interaction](../engine/interaction.md)) —
each compared with its 32-bit namesake. Unread: every handler not named
here and not on the read pages — the text drawer and `FADEIN`/`FADEOUT`
are read ([text rendering](../engine/text-rendering.md),
[transitions](../engine/transitions.md)).

## See also

- [Threaded code](threaded-code.md) — how a cell names a word
- [ENVIRO.EXE](../engine/enviro-exe.md) — where the tables sit in the binary
- [HPPLAY.EXE](../engine/hpplay-exe.md), [BMZ.EXE](../engine/bmz-exe.md) — the earlier builds' tables
- [Kernel words (MOTION 32-bit)](../../motion32/vm/kernel-words.md) — the 356-word kernel for the names that match
