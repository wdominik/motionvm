[← Documentation index](README.md)

# motionvm's Savegames

*motionvm's own — this page describes files this program writes, not files any original MOTION player wrote. Why they cannot be the original's is in [Departures](departures.md#savegames); what the original writes, and when, is under [MOTION 32-bit](README.md#motion-32-bit) and [MOTION 16-bit](README.md#motion-16-bit).*

Every MOTION game saves through the same three files per slot, named from the
id `701 + slot`:

| File | Written by | Holds |
|---|---|---|
| `NNN.blk` | `PUT` | the location number, straight out of module memory |
| `NNN.FRZ` | `=>PUTAS` | every resident module's memory |
| `NNN.anm` | `PUTANIM` | the screens, the descriptors and a handful of globals |

`NNN.blk` is written and read straight through, so its bytes are the
original's: four on the 32-bit engine, two on the 16-bit one, and no header at
all. The other two are motionvm's own layout, and this page is what they are.

## Where they live

Under the platform's data directory, one directory per game named for it —
`saves/ds2/`, `saves/enviro/`, `saves/jeffjet/`, `saves/hfa/`,
`saves/vloomes/`. Every game names its slots alike, and on the 16-bit engine
the magic below is the engine's rather than the game's, so two games sharing a
directory would find and load each other's slots. The name is put on inside
the engine, so nothing that embeds it can leave it off.

## The header

Both `.FRZ` and `.anm` open the same way: eight bytes of magic, then a `u32`
version.

| Engine | `.FRZ` magic | `.anm` magic |
|---|---|---|
| 32-bit | `DS2FRZ\0\0` | `DS2ANM\0\0` |
| 16-bit | `ENVFRZ\0\0` | `ENVANM\0\0` |

The magic is the **engine generation's**, not the game's: all four 16-bit games
write `ENVFRZ` and `ENVANM`, which is a fact about the format family and the
reason the directory carries the game's name instead.

Everything is little-endian, as everywhere else in these formats. An `i32` and
a `u32` are four bytes; a `u8` is one. An **optional** value is a `u8` flag
and then the `i32` — always four bytes, whether the flag is set or not. A
**string** is a `u32` length and then that many bytes of UTF-8.

## `NNN.FRZ` — the resident modules

```text
magic  u32 version  u32 count
per module:  u32 number   u32 length   <length units>
```

The unit is the machine's cell: `u32` cells on the 32-bit engine, single bytes
on the 16-bit one, and `length` counts whichever it is. Modules are written in
the order the engine holds them.

Records are keyed by module number and matched by it on the way back in, which
is one of the two things this file does that the original's does not — the
original writes the number and then ignores it, relying on `INCLLOC` having
just rebuilt an identical set. The other is that the whole file is parsed and
every record matched to a loaded module of the same size *before the first
byte is written back*, so a damaged savegame stops the load instead of
half-applying itself.

## `NNN.anm` — the display

```text
magic  u32 version
u32 next_descriptor
optional current_descriptor   optional current_screen
u8 pointer_visible
i32 dialogue_offset   i32 dialogue_return
u8[768] palette
u32 screen_count
  per screen:  u32 handle
               i32 w  i32 h            (size)
               i32 w  i32 h            (full view)
               i32 w  i32 h            (view)
               i32 x  i32 y            (view position)
               i32 x  i32 y            (position)
               i32 x  i32 y            (origin)
u32 mirror_count
  per mirror:  u32 from   u32 to
u32 descriptor_count
  per descriptor:  u32 handle   u32 screen
                   i32 x   i32 y   i32 level
                   u8 shows_tag   i32 shows_value
                   optional text   optional font
                   optional color  optional template
                   i32 wait   i32 callback
                   u8 x_mode   u8 y_mode
                   u8 active   u8 auto_buffer
                   u32 field_count
                     per field:  string name   i32 value
                   optional buffer          (16-bit only)
u8 buffers_on                                (16-bit only)
u32 buffer_count                             (16-bit only)
  per buffer:  i32 id   i32 width   i32 height
```

`shows_tag` is 0 for nothing, 1 for a sprite, 2 for a picture, and
`shows_value` its id — one field, because a descriptor shows one thing.

The named fields travel as **names** rather than as offsets, so a field this
build does not know is a named error rather than a silent drop.

Three things here have no counterpart in the original's file, and each is
there because this engine is not the original one. The **palette**, because
`SETPAL` runs in every location macro but two, and a savegame made in one of
those would otherwise come back wearing the menu's colors. The **handle
counter**, because the original hands out heap addresses that a fresh
allocation cannot collide with while these are counted from one. And the
**mirror recipe** — the ids `GFXVFLIP` was asked for, not the pixels — because
mirrored sprites go into the graphics pool under ids no resource file holds,
so a reload could not find them again.

Deliberately absent: whether a screen is frozen or inactive. At the moment of
a save the picture is frozen, because the game's own menu is open over it, and
restoring that would load a game that stands still.

## What a version change does

`version` is 1, and a file whose version is not the one this build writes is
refused by name:

```text
701.FRZ: savegame version 1, this build writes 2
```

The refusal is the whole policy. There is no converter and no fallback: a
savegame is a snapshot of engine state that the engine's own structures
define, and a build that has changed them cannot honestly read an older one
back. A release that changes the number says so in the changelog, and slots
written before it stay on disk and stay unreadable — nothing deletes them.

A file whose magic does not match is refused the same way, which is what
catches one game's slot in another's directory:

```text
701.anm: not a savegame file ([44, 53, 32, 41, 4e, 4d, 00, 00])
```

## See also

- [Departures](departures.md#savegames) — why these files cannot be the original's
- [Savegames (MOTION 32-bit)](motion32/engine/savegames.md) — what the original writes, and when
- [Boot and frame loop (MOTION 16-bit)](motion16/engine/game-loop.md) — the 16-bit engine's save and load path
