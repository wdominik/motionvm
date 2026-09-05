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
directory would find each other's slots. The name is put on inside the engine,
so nothing that embeds it can leave it off, and it is in the header as well —
a directory is a convention, and a file can be moved by hand.

## How they are written

Each file goes to `<name>.tmp` first, is flushed to the device, and only then
takes its name. A rename inside a directory is atomic, so a reader sees either
every byte of the new file or every byte of the old one; a crash, a full disk
or a pulled plug partway through a save costs the new slot and not the old.
The original writes straight through and has no such property — a save
interrupted there leaves a file that is neither, and still opens.

The directory itself is deliberately not flushed. Without that a power cut can
leave the older of the two files in place, which is the older of the two
consistent states, and consistent is the whole of what is promised.

## The head

Both `.FRZ` and `.anm` open the same way, twenty bytes:

```text
[8] magic       the engine generation's
u32 version     1
u32 length      of the body
u32 crc32       of the body
```

| Engine | `.FRZ` magic | `.anm` magic |
|---|---|---|
| 32-bit | `DS2FRZ\0\0` | `DS2ANM\0\0` |
| 16-bit | `ENVFRZ\0\0` | `ENVANM\0\0` |

The magic is the **engine generation's**, not the game's: all four 16-bit games
write `ENVFRZ` and `ENVANM`. It is not repeated inside the body — two fields
saying the same thing can disagree, and a reader would then have to decide
which to believe.

The checksum is a CRC-32 over the body, the reflected IEEE 802.3 polynomial
that PNG, zip and gzip use. It is checked before anything in the body is
looked at, so a file damaged after it was written is refused as damaged rather
than half-read.

Everything is little-endian, as everywhere else in these formats. An `i32` and
a `u32` are four bytes; a `u8` is one. An **optional** value is a `u8` flag
and then the `i32` — always five bytes, whether the flag is set or not. A
**string** is a `u32` length and then that many bytes of UTF-8.

## The body

```text
string slug     the game whose slot this is: ds2, enviro, jeffjet, hfa, vloomes
string wrote    the motionvm version that wrote it
sections, to the end of the body:
    [4] tag   u32 length   <length bytes>
```

`slug` is what catches a slot carried into another game's directory, and it is
checked before any section is read:

```text
701.anm: a savegame of enviro, and this is hfa
```

`wrote` is never read back. It is there for the person holding a slot that
will not load, who needs to know which build made it before anything else can
be worked out.

Every section carries its length, so a reader steps over a tag it does not
know. That is what lets a later build **add** a section without moving the
version number. Inside a section the layout belongs to the version that
defines it and is fixed: bytes left over in a section this build knows are a
reader and a writer disagreeing, and are reported as such. Growth is a new
section, never a field appended to an old one.

## `NNN.FRZ` — the resident modules

One section.

```text
MODS   u32 count
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

Four sections, and a fifth on the 16-bit engine.

```text
HEAD   u32 next_descriptor
       optional current_descriptor   optional current_screen
       u8 pointer_visible
       i32 dialogue_offset   i32 dialogue_return
       u8[768] palette

SCRN   u32 count
       per screen:  u32 handle
                    i32 w  i32 h            (size)
                    i32 w  i32 h            (full view)
                    i32 w  i32 h            (view)
                    i32 x  i32 y            (view position)
                    i32 x  i32 y            (position)
                    i32 x  i32 y            (origin)

FLIP   u32 count
       per mirror:  u32 from   u32 to

DESC   u32 count
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

BUFS   u8 on                                     (16-bit only)
       u32 count
       per buffer:  i32 id   i32 width   i32 height
```

`shows_tag` is 0 for nothing, 1 for a sprite, 2 for a picture, and
`shows_value` its id — one field, because a descriptor shows one thing.

The named fields travel as **names** rather than as offsets, so a field this
build does not know is a named error rather than a silent drop.

`BUFS` is absent from a 32-bit file rather than empty: the 32-bit engine has
no off-screen buffers, and an optional section is how that is said.

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

`version` is 1. A bump means the *file* changed shape — a field added to a
section, a section whose meaning moved — and not that the engine did; a new
section beside the known ones needs no bump at all, which is the whole reason
the body is sectioned.

**There is one reader, for this version.** A file of any other version is
refused **by name** and left on disk — nothing here deletes a savegame it
cannot read — and nothing converts one. Until the first stable release a
change to the layout is a bump and a refusal, and the release notes say so.

```text
701.FRZ: savegame version 0, this build writes 1
```

A file whose magic does not match is refused the same way, which is what
catches one engine generation's slot under the other:

```text
701.anm: not a savegame file ([44, 53, 32, 41, 4e, 4d, 00, 00])
```

## What a refusal says

Every message names the file or the word that asked for it, and then what was
wrong:

```text
701.FRZ: damaged after it was written: the header says a checksum of 3735928559, the file has 2596069104
701.anm: a savegame of enviro, and this is hfa
701.anm: no DESC section
=>GETAS 701: slot 701 is incomplete — 701.FRZ is missing, so the save it belongs to was interrupted while it was being written
=>GETAS 701: module 907 appears twice, as record 4 and 11
```

The last two are worth telling apart. A slot is *incomplete* when its `.blk`
is there and one of the other two is not: `EXIST` answers off the `.blk` alone
— as the original's does, at `0x66b0e` — so the game offers a slot whose state
files may never have been written. With each file now written whole or not at
all, that is the one way a save can still be half-made.

## See also

- [Departures](departures.md#savegames) — why these files cannot be the original's
- [Savegames (MOTION 32-bit)](motion32/engine/savegames.md) — what the original writes, and when
- [Boot and frame loop (MOTION 16-bit)](motion16/engine/game-loop.md) — the 16-bit engine's save and load path
