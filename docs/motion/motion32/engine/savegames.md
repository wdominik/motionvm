[← Documentation index](../../README.md)

# Savegames

*MOTION 32-bit — the engine as shipped in `ENGINE.EXE` V0.06.06/R109 with Dunkle Schatten 2; what is measured here is measured on that game's files. The 16-bit engine is documented under [MOTION 16-bit](../../README.md#motion-16-bit).*

Five slots, numbered **701–705**, three files each. The script side —
which button leads where, and in what order the words run — is in
[Shell](../../games/ds2/library/shell.md); this page is the engine side: what each
file contains and how the engine writes it.

## The three files

Names come from the engine's own format templates, which use `#` as the
escape character rather than `%`: `#F0R3i.blk` at `0xd4872` is `%03d.blk`,
and its siblings are `#F0R3i.FRZ` (`0xd480d`) and `#F0R3i.anm`
(`0xd503d`). None carries a path component, so a save lands in the
process's working directory — beside `ENGINE.EXE`, among the shipped
data.

| file | word | handler | contents |
|---|---|---|---|
| `NNN.blk` | `PUT` | `0x668aa` | four bytes: the location number |
| `NNN.FRZ` | `=>PUTAS` | `0x6559e` | every resident module's memory |
| `NNN.anm` | `PUTANIM` | `0x6dd87` | the descriptor tree, five globals and a 180-byte table |

`EXIST` (`0x66ada`) is what finds them: it formats `%03d.blk` and calls
the runtime's file test, pushing **−1** when the file is there and 0 when
it is not (`0x66b18` / `0x66b77`). It is a real file test, unlike
`=>EXIST`, which asks the resource catalog.

## `NNN.FRZ` — the module image

`=>PUTAS` walks descriptor slots 1 to 31 — slot 0 is the kernel's own
`Basismodul` and is skipped — and writes one record per occupied slot:

```
u32       module number
u8[0x30]  the module descriptor, verbatim
u8[DP*4]  module memory
u8[16004] the second region
```

No header, no count, no terminator. `=>GETAS` (`0x657a4`) walks the same
slots and copies the two memory regions back, taking their sizes from the
*running* descriptor: it reads the module number and the descriptor into
the cursor and then ignores both. **The mapping is purely positional**,
and it works only because the loaded set is identical on both sides —
which is what `INCLLOC`, running before `=>GETAS`, guarantees.

Two of the forty-eight descriptor bytes in each record are live DOS4GW
heap addresses. Neither reader looks at them.

### Which modules are resident

Fixed by the bytecode, not by chance. `SYSTEM.RSC` is two lines,
`4 =>GET` and `START`. `4:START` at `0x5420` does `2 5 6 11 13 3 =>GET`,
runs `STARTUP`, `3 =>ERASE`, `12 =>GET`, `DS_INIT`, `12 =>ERASE`. And
`5:INCLLOC` frees the outgoing location's three modules at
`0x1a08`–`0x1a48` *before* it takes the incoming one's at
`0x1b08`–`0x1b48`. So in a running game:

```
slot 1  2  3  4  5   6   7      8      9
     4  2  5  6  11  13  L+100  L+200  L+300
```

Nine records. The freeing-before-taking is what keeps the location's
three modules in slots 7, 8 and 9 across every move, and therefore what
makes the positional format work at all.

Modules 3 and 12 are erased during boot, so no savegame contains them.

## `NNN.anm` — the display state

`PUTANIM` writes the root screen descriptor and payload, then walks the
descriptor tree recursively, writing a type word, the 0x3E-byte
descriptor and a type-sized payload (`0x41E` for a screen, 8 for a
sprite, 4 for an image, `0x5C` for a text) for each node. Sibling chains
end with `-1`; screens carry a list of 0x1C-byte records tagged `0x3EB`
and terminated by `0x3EC`, then `0x3E9` if children follow or `0x3EA` if
not. After the tree come a global 0x1C-byte record list tagged `0x3EF`,
five globals — `0xDB4BC` the active screen, `0xDB4C0` the active
descriptor, then `0xDB4AC`, `0xDB4B4`, `0xDB4B8` — and a 180-byte table
at `0xF2594`.

`GETANIM` (`0x6e355`) tears the tree down with `NEWANIM` and rebuilds it,
allocating fresh nodes and **recomputing everything pointer-shaped**; the
addresses in the file are stale and are never used as addresses. It is
also the only place in any of these words where a file handle is checked
(`0x6e37e`): a missing `.anm` makes it return without touching anything.

Note what this implies about the original's design. `NEWDESC` and
`NEWSCREEN` hand the scripts raw heap pointers, and those pointers are
stored in the module memory a savegame restores — so a load only lines up
because `GETANIM` reallocates the same nodes in the same order and the
allocator is deterministic.

## Side effects

`PUT` and `=>PUT` both rewrite `rsc.inf` (`0x556c1` / `0x558bb` into
`0x5506a`): 0x48 bytes of resource-manager globals followed by every
catalog, pointers and all. It is a memory image of the resource manager,
so saving the game mutates it.

Writing at all is gated on bit 1 of the mode word at `0xd7064`, which the
shipped game sets to `0x1F` — the value the first four bytes of the
shipped `RSC.INF` still show.

Where motionvm departs from all of this — the file layouts, where saves are
written, that they are validated before being applied, and what the display
snapshot deliberately leaves out — is recorded in
[Departures](../../departures.md).

## Open questions

- What three of the five globals `PUTANIM` writes — `0xDB4AC`, `0xDB4B4`,
  `0xDB4B8` — and the 180-byte table at `0xF2594` hold. The layout is read; the
  meaning is not.
- Whether the original ever writes a slot other than 701–705. Nothing in the
  shell's bytecode does, and no other number appears in any handler.

## See also

- [Departures](../../departures.md) — what motionvm does differently here
- [Shell](../../games/ds2/library/shell.md) — the script side: which button leads where
- [Module map](../../games/ds2/module-map.md) — which modules a save carries, and why that set
- [Game structure](../../games/ds2/game-structure.md)
