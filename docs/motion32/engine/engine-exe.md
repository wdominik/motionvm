[← Documentation index](../../README.md)

# ENGINE.EXE — The MOTION Engine Binary

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

`ENGINE.EXE` is not game-specific code. It is **MOTION**, a general
adventure authoring system by DigiTales (Stefan Hoffmann), version
V0.06.06/R109, dated 1996-10-22. It contains a complete IDE, a Forth
compiler, and a debugger; the game itself is data — 86 compiled Forth
modules (see [Script modules](../formats/script-modules.md)).

The binary was built with Watcom C/C++32 and ships as a **Linear
Executable (LE)** for the DOS/4GW extender: a real-mode DOS stub followed
by the 32-bit LE image.

## Boot behavior

At startup the engine interprets `SYSTEM.RSC` as Forth source text. The
shipped file contains `4 =>GET` and `START` — load module 4, run its
`START` word, which launches the game. Without a valid `START` the engine
drops into the MOTION IDE instead and creates a project file `NONAME.PRJ`.

The engine requires a VESA 640×480 256-color mode; plain VGA is not
sufficient.

## LE image structure

Fields of the LE header that matter for loading (offsets relative to the LE
header, which the DOS stub locates via the `u32` at file offset `0x3c`):

| Offset | Type | Description |
|---|---|---|
| `+0x00` | `char[2]` | Signature `LE` |
| `+0x14` | `u32` | Page count |
| `+0x28` | `u32` | Page size |
| `+0x40` | `u32` | Object table offset |
| `+0x44` | `u32` | Object count |
| `+0x68` | `u32` | Fixup page table offset |
| `+0x6c` | `u32` | Fixup record table offset |
| `+0x80` | `u32` | File offset of the page data |

Object table entries are 24 bytes:

| Offset | Type | Description |
|---|---|---|
| `+0` | `u32` | Virtual size |
| `+4` | `u32` | Base address |
| `+8` | `u32` | Flags: `0x4` executable, `0x2` writable |
| `+12` | `u32` | First page (1-based) |
| `+16` | `u32` | Page count |

`ENGINE.EXE` has 4 objects (object 1 is the code object) and 170 pages.

### Relocated address to file offset

Everything in this documentation cites the **relocated** image — an address
as the loader lays it out, which is what a running disassembly shows. Reading
the same place out of the file on disk needs the other number:

```text
file offset = relocated address + 0x14E00
```

The constant is the file offset of object 1's page data minus its base
address, so it holds for the code object and only for it.

**Confusing the two disassembles nonsense that looks plausible in places.**
x86 is a variable-length encoding with no alignment, so a stream started at
the wrong offset still decodes — into instructions that are not there. There
is no error, no signature, nothing to notice; the output simply describes a
program that does not exist. Any address quoted without saying which of the
two it is has to be treated as unusable.

## Fixups

The pointers inside the image — including the kernel word tables — **do not
exist in the file**. They materialize only when the loader applies the
fixup (relocation) records; 17,270 32-bit fixups apply to this binary.

Fixup records per page are delimited by consecutive entries of the fixup
page table. A record consists of:

- `u8 src_type`, `u8 flags`
- If `src_type & 0x20`: a source *list* — `u8 count` followed by
  `count × i16` patch offsets; otherwise a single `i16` offset.
- Object index: 16-bit if `flags & 0x40`, else 8-bit.
- Target offset: none if `(src_type & 0x0f) == 2`; else `u32` if
  `flags & 0x10`, else `u16`.

Only fixups with `(src_type & 0x0f) == 7` (32-bit offset) carry pointers;
they patch `object_base + target_offset` into the image. Source offsets can
be **negative**, reaching back into the previous page. This file contains
only internal references.

## What lives in the relocated image

- The three kernel word tables at `0xdb067`, `0xdb397`, `0xdb504` — see
  [Kernel words](../vm/kernel-words.md).
- The module table at `0xEE6C0`/`0xEE6D0` used by address decoding — see
  [Execution model](../vm/execution-model.md).
- Engine globals such as the text-spacing values at `0xd66ec`/`0xd66ee` —
  see [Text rendering](text-rendering.md).
- The font reference table image at `0xE8378` — see
  [Font reference table](../formats/font-reference-table.md).

All addresses in this documentation refer to the relocated image.

## Authoring facilities

Because MOTION is an authoring system, the binary contains many words that
only matter at development time: the module writer `=>PUT`, the image
importers `FULLCUT`/`XYCUT`/`SCANCUT` (see
[Kernel words](../vm/kernel-words.md)), the IDE, and the debugger. The
`Scanning for ...` messages printed at startup while the resource files are
indexed also come from this layer.

## See also

- [Kernel words](../vm/kernel-words.md)
- [Execution model](../vm/execution-model.md)
- [Other files](../../games/ds2/other-files.md) — DOS4GW, SYSTEM.RSC, and the
  rest of the shipped environment
