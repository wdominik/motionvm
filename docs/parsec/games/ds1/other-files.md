[← Documentation index](../../README.md)

# Other Files

*Dunkle Schatten — the files of an installation besides the two programs and the resources they read: the batch files, the `.DOK` viewer, and the rest. The programs and their resources are documented under [formats](../../README.md#formats).*

`FILES.DOK` lists what an installation has to hold for the game to start:
ten files in `\SCHATTEN` and ten in `\SCHATTEN\RESOURCE`, every one dated
1994-10-24, 20:00. It adds that `SCHATTEN.BAT` is renewed by the
installation, so its contents may differ from the list's.

## The batch files

**`SCHATTEN.BAT`** is how the game is started, as `INFO.DOK` says
(*"Gehe in das Verzeichnis mit "Dunkle Schatten" und gib "SCHATTEN" ein"*).
It erases a `SCHATTEN.BAT` one directory up, runs `xp00.exe` and,
unless that left errorlevel 1 or more, `xp01.exe`, handing both whatever
was typed after the batch file's name.

**`DS1.BAT`** is the same file with an `attrib -r *.*` in front, which
takes the read-only flag off every file of the directory — what a copy
from a CD-ROM needs before the game can write its saved game. It and the
icon `DS1.ICO` are not in `FILES.DOK`'s list.

**`INFO.BAT`** and **`FILES.BAT`** run `xp02.exe info.dok` and
`xp02.exe FILES.dok`.

## `XP02.EXE` and the two texts

`XP02.EXE` is the third program, packed as the other two are
([the program files](../../formats/executables.md)); the batch files run it
on the two `.DOK` files, and neither of the other two programs names it. **`INFO.DOK`** is the short
guide: the machine the game asks for, the installation, how to start it,
the keys that stand in for a mouse — the cursor
keys move the pointer, Return or Space presses the button — the in-game
menu on Escape or on the small monitor symbol, tips, and the hotline.
**`FILES.DOK`** is the file list above.

## The rest

- **`RESOURCE/EXTROS.DAT`**, an FGROUP of four slots
  ([containers](../../formats/containers.md)). `XP01` opens it on the way
  out: for the end sequence of a game that ends on location −1, and for
  the sequence it plays when the player quits
  ([the way out](../../engine/ending.md)).
- **`RESOURCE/PRGVER.DAT`**, 212 bytes that no reader here takes for a
  format. Both programs name it, `RESOURCE\PRGVER.DAT`.
- **`CONFIG.DAT`**, nine bytes: seven ASCII zeros and a line end. None of
  the three programs names it.
